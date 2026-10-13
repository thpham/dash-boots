//! The training corpus: balanced over 4 themes × 5 measured difficulties.
//! ~30 % mutations of the campaign transcriptions (each in its own theme),
//! the rest from the section teacher; every label is *measured* by the graded
//! bots; unfinishable levels are dropped. Multi-threaded.
//! cargo run --release --example levelgen_corpus -- <total> <out.txt>

use std::collections::HashSet;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

use snes_wasm::levelgen::bot;
use snes_wasm::levelgen::compile::compile_strict;
use snes_wasm::levelgen::handmade::{mutate, CAMPAIGN};
use snes_wasm::levelgen::motif::{Motif, Theme};
use snes_wasm::levelgen::rng::Rng;
use snes_wasm::levelgen::teacher;

const HANDMADE_SHARE: f32 = 0.3;
const THREADS: u64 = 8;

struct Sample {
    theme: Theme,
    measured: u8,
    from_handmade: bool,
    line: String,
}

fn label(theme: Theme, seq: &[Motif], seed: u64, from_handmade: bool) -> Option<Sample> {
    let lvl = compile_strict(theme, seq)?;
    let measured = bot::measure(&lvl.level_data(), seed)?;
    let words: Vec<String> = seq.iter().map(|m| m.name()).collect();
    let line = format!("<{}> <d{measured}> {}", theme.name(), words.join(" "));
    Some(Sample { theme, measured, from_handmade, line })
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let total: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(20_000);
    let out = args.get(2).cloned().unwrap_or_else(|| "corpus.txt".into());
    let quota = total / 20;
    let handmade_quota = (quota as f32 * HANDMADE_SHARE) as usize;
    let stop = AtomicBool::new(false);
    let (tx, rx) = mpsc::channel::<Sample>();

    let mut fill = [[0usize; 5]; 4];
    let mut fill_handmade = [[0usize; 5]; 4];
    let mut seen = HashSet::new();
    let mut lines = Vec::new();
    std::thread::scope(|scope| {
        for w in 0..THREADS {
            let tx = tx.clone();
            let stop = &stop;
            scope.spawn(move || {
                let mut rng = Rng::new(0xC0_4F05 + w);
                let mut seed = w;
                while !stop.load(Ordering::Relaxed) {
                    seed += THREADS;
                    let sample = if rng.f32() < 0.45 {
                        let t = &CAMPAIGN[rng.below(4)];
                        label(t.theme, &mutate(&t.motifs(), &mut rng), seed, true)
                    } else {
                        let theme = Theme::ALL[rng.below(4)];
                        let target = 1 + rng.below(5) as u8;
                        label(theme, &teacher::generate(theme, target, seed), seed, false)
                    };
                    if let Some(s) = sample {
                        if tx.send(s).is_err() {
                            return;
                        }
                    }
                }
            });
        }
        drop(tx);
        let started = std::time::Instant::now();
        for s in rx.iter() {
            let (ti, di) = (s.theme.index(), s.measured as usize - 1);
            let room = if s.from_handmade {
                fill_handmade[ti][di] < handmade_quota && fill[ti][di] < quota
            } else {
                fill[ti][di] - fill_handmade[ti][di] < quota - handmade_quota
            };
            if room && seen.insert(s.line.clone()) {
                fill[ti][di] += 1;
                fill_handmade[ti][di] += s.from_handmade as usize;
                lines.push(s.line);
            }
            let full = fill.iter().flatten().all(|n| *n >= quota);
            if full || started.elapsed().as_secs() > 900 {
                stop.store(true, Ordering::Relaxed);
                break;
            }
        }
        stop.store(true, Ordering::Relaxed);
    });

    let mut f = std::fs::File::create(&out).expect("create corpus");
    for l in &lines {
        writeln!(f, "{l}").unwrap();
    }
    println!("wrote {} levels to {out} (quota {quota}/cell, handmade ≤ {handmade_quota}/cell)", lines.len());
    for theme in Theme::ALL {
        let row: Vec<String> = (0..5)
            .map(|d| format!("{:5}({:4})", fill[theme.index()][d], fill_handmade[theme.index()][d]))
            .collect();
        println!("  {:7} d1..d5 total(handmade): {}", theme.name(), row.join(" "));
    }
}
