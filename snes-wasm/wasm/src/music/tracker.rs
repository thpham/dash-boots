//! libxm render -> bounded, deterministic stereo interleaved f32 buffer.
//!
//! The loop point is detected as a backward order jump (`position().
//! pattern_index`): song loops jump via pattern jumps and never wrap the
//! order list, so `loop_count()` alone is not enough. Two passes (libxm is
//! deterministic):
//! 1. coarse, chunk-stepped — module structure, approximate jump offsets;
//! 2. re-simulated with per-frame polling around each jump — exact offsets,
//!    buffer = the looped region only (the module intro never enters RAM;
//!    in game you hear the loop, not the intro).
//!
//! `MusicRenderTask` wraps this pipeline incrementally: each `step` costs a
//! few milliseconds of simulation at most, so the host can pump it without
//! freezing the render loop (review Critical 1: measured 0.5-7.7 s freeze
//! with a synchronous render). `render_music` (one-shot, for tools and
//! tests) is the same task driven to completion — there is one code path.

use super::tracks::TRACKS;

pub const MUSIC_SR: usize = 22050;
/// Anti-pathology cap for the coarse pass: leaves room for the longest loop
/// measured by the probe (249 s) plus one more full cycle.
const MAX_FRAMES: usize = MUSIC_SR * 600;
/// Non-looping fallback (coda with no return jump, pathological module):
/// a render capped at 60 s, looped as-is by the host.
const FALLBACK_FRAMES: usize = MUSIC_SR * 60;
const CHUNK: usize = 4096;

enum Phase {
    /// Coarse pass: find the offsets of the first two backward jumps.
    Coarse,
    /// Refine pass: re-simulation with per-frame polling around each jump,
    /// collecting the `[f1, f2)` region.
    Refine,
    /// Fallback: truncated render on a FRESH context (the coarse-pass context
    /// already carries up to 600 s of simulation state — continuing on it
    /// would render the tail of the module, not the start of the song).
    Fallback,
    Done,
}

pub struct MusicRenderTask {
    data: Vec<u8>,
    ctx: libxm::XMContext,
    prev_idx: u8,
    played: usize,
    jumps: Vec<usize>,
    /// 0 -> approaching q1, 1 -> approaching q2, 2 -> done (Refine phase).
    jump: usize,
    collecting: bool,
    phase: Phase,
    out: Vec<f32>,
}

impl MusicRenderTask {
    pub fn new(id: u8) -> Self {
        assert!((id as usize) < TRACKS.len(), "unknown music track {id}");
        Self::from_data(TRACKS[id as usize].data.to_vec())
    }

    /// Same pipeline on arbitrary module bytes (auditioning tools).
    pub fn from_data(data: Vec<u8>) -> Self {
        let ctx = libxm::XMContext::new(&data, MUSIC_SR as u32).expect("module must parse");
        Self {
            prev_idx: ctx.position().pattern_index,
            played: 0,
            jumps: Vec::with_capacity(2),
            jump: 0,
            collecting: false,
            phase: Phase::Coarse,
            out: Vec::new(),
            data,
            ctx,
        }
    }

    /// Advance by up to `chunks` steps (a chunk, or a frame near a jump).
    /// Returns `true` when the render is done (`take` is then valid).
    pub fn step(&mut self, chunks: usize) -> bool {
        for _ in 0..chunks {
            self.advance_one();
            if matches!(self.phase, Phase::Done) {
                return true;
            }
        }
        false
    }

    /// Final buffer, normalized to 0.95 peak. Only call after `step` returned
    /// true; empty otherwise (the host must never see it through this path).
    pub fn take(self) -> Vec<f32> {
        self.out
    }

    fn fresh_ctx(&self) -> libxm::XMContext {
        libxm::XMContext::new(&self.data, MUSIC_SR as u32).expect("module must parse")
    }

    fn advance_one(&mut self) {
        match &mut self.phase {
            Phase::Coarse => {
                let mut frames = [0f32; 2 * CHUNK];
                self.ctx.generate_samples(&mut frames);
                self.played += CHUNK;
                let idx = self.ctx.position().pattern_index;
                if idx < self.prev_idx {
                    self.jumps.push(self.played);
                }
                self.prev_idx = idx;
                if self.jumps.len() >= 2 {
                    // refine pass: fresh re-simulation, counters reset
                    self.ctx = self.fresh_ctx();
                    self.prev_idx = self.ctx.position().pattern_index;
                    self.played = 0;
                    self.phase = Phase::Refine;
                } else if self.played >= MAX_FRAMES {
                    // no loop detected: fallback on a fresh context
                    self.ctx = self.fresh_ctx();
                    self.played = 0;
                    self.phase = Phase::Fallback;
                }
            }
            Phase::Refine => {
                let q = self.jumps[self.jump];
                if self.played + CHUNK < q {
                    let mut frames = [0f32; 2 * CHUNK];
                    self.ctx.generate_samples(&mut frames);
                    if self.collecting {
                        self.out.extend_from_slice(&frames);
                    }
                    self.played += CHUNK;
                    self.prev_idx = self.ctx.position().pattern_index;
                } else {
                    // approaching a jump: per-frame polling for the exact offset
                    let mut frame = [0f32; 2];
                    self.ctx.generate_samples(&mut frame);
                    self.played += 1;
                    let idx = self.ctx.position().pattern_index;
                    if idx < self.prev_idx {
                        self.jump += 1;
                        if self.jump == 1 {
                            // this frame starts the looped cycle
                            self.collecting = true;
                            self.out.extend_from_slice(&frame);
                        }
                        // for the closing jump: this frame belongs to the next
                        // cycle, so drop it — the buffer ends just before.
                    } else if self.collecting {
                        self.out.extend_from_slice(&frame);
                    }
                    if self.jump == 2 {
                        self.finish();
                    }
                    self.prev_idx = idx;
                }
            }
            Phase::Fallback => {
                let mut frames = [0f32; 2 * CHUNK];
                self.ctx.generate_samples(&mut frames);
                self.out.extend_from_slice(&frames);
                self.played += CHUNK;
                if self.played >= FALLBACK_FRAMES || self.ctx.loop_count() > 0 {
                    self.finish();
                }
            }
            Phase::Done => {}
        }
    }

    fn finish(&mut self) {
        let peak = self.out.iter().fold(0.0f32, |a, &s| a.max(s.abs()));
        let g = (0.95 / peak.max(1e-6)).min(1.0);
        for s in &mut self.out {
            *s = (*s * g).clamp(-1.0, 1.0);
        }
        self.phase = Phase::Done;
    }
}

/// One-shot render (tools: examples/dump_wav; tests).
pub fn render_music(id: u8) -> Vec<f32> {
    let mut task = MusicRenderTask::new(id);
    while !task.step(1) {}
    task.take()
}

/// One-shot render of arbitrary module bytes (same pipeline, same output as
/// `render_music` on the same data).
pub fn render_module(data: Vec<u8>) -> Vec<f32> {
    let mut task = MusicRenderTask::from_data(data);
    while !task.step(1) {}
    task.take()
}
