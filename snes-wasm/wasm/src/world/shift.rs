//! Past ↔ future shift timeline ("Mémoire vive" spec §5). Pure and
//! deterministic: stepped once per simulation step, it only produces numbers
//! that the renderer consumes. Gameplay never depends on it.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum World {
    Past,
    Future,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShiftState {
    Past,
    ToFuture(u32),
    Future,
    ToPast(u32),
}

pub const TO_FUTURE_STEPS: u32 = 90;
pub const TO_PAST_STEPS: u32 = 108;
pub const NOSTALGIA_STEPS: u32 = 120;
pub const REDUCED_FADE_STEPS: u32 = 48;
pub const CRT_BASE: f32 = 0.25;
pub const CRT_NOSTALGIA: f32 = 0.6;
pub const FINAL_YAW_DEG: f32 = 75.0;
pub const FINAL_PITCH_DEG: f32 = -15.0;

const STEP_SECONDS: f32 = 1.0 / 60.0;
/// Length of the to-future curve in seconds; the way back replays it reversed.
const CURVE_SECONDS: f32 = TO_FUTURE_STEPS as f32 * STEP_SECONDS;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShiftParams {
    /// 0 = show the 2D past image, 1 = show the 3D scene.
    pub world_mix: f32,
    pub glitch: f32,
    /// Extrusion wave progress.
    pub extrude: f32,
    /// Camera pivot progress (yaw, pitch, distance, fov).
    pub pivot: f32,
    /// Resolution focus: 0 = 320×180, 1 = max future resolution.
    pub focus: f32,
    pub crt: f32,
    pub warmth: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Shift {
    pub state: ShiftState,
    /// Remaining afterglow steps after coming back to the past.
    nostalgia: u32,
}

fn clamp01(v: f32) -> f32 {
    v.clamp(0.0, 1.0)
}

fn ramp(t: f32, start: f32, len: f32) -> f32 {
    clamp01((t - start) / len)
}

pub fn ease_in_out(x: f32) -> f32 {
    let x = clamp01(x);
    x * x * (3.0 - 2.0 * x)
}

/// To-future curve at `t` seconds after the rift.
fn future_curve(t: f32) -> ShiftParams {
    ShiftParams {
        world_mix: if t >= 0.05 - 1e-4 { 1.0 } else { 0.0 },
        glitch: 1.0 - ramp(t, 0.0, 0.1),
        extrude: ramp(t, 0.05, 0.55),
        pivot: ease_in_out(ramp(t, 0.2, 1.3)),
        focus: ease_in_out(ramp(t, 0.3, 0.9)),
        crt: 0.0,
        warmth: 0.0,
    }
}

const PAST: ShiftParams = ShiftParams {
    world_mix: 0.0,
    glitch: 0.0,
    extrude: 0.0,
    pivot: 0.0,
    focus: 0.0,
    crt: CRT_BASE,
    warmth: 0.0,
};
const FUTURE: ShiftParams =
    ShiftParams { world_mix: 1.0, glitch: 0.0, extrude: 1.0, pivot: 1.0, focus: 1.0, crt: 0.0, warmth: 0.0 };

impl Shift {
    pub fn new(world: World) -> Self {
        let state = match world {
            World::Past => ShiftState::Past,
            World::Future => ShiftState::Future,
        };
        Self { state, nostalgia: 0 }
    }

    /// World the player is in, or heading to during a shift.
    pub fn world(&self) -> World {
        match self.state {
            ShiftState::Past | ShiftState::ToPast(_) => World::Past,
            ShiftState::Future | ShiftState::ToFuture(_) => World::Future,
        }
    }

    pub fn is_shifting(&self) -> bool {
        matches!(self.state, ShiftState::ToFuture(_) | ShiftState::ToPast(_))
    }

    /// Start a shift toward the other world. Returns false while already shifting.
    pub fn trigger(&mut self) -> bool {
        self.state = match self.state {
            ShiftState::Past => ShiftState::ToFuture(0),
            ShiftState::Future => ShiftState::ToPast(0),
            _ => return false,
        };
        self.nostalgia = 0;
        true
    }

    pub fn step(&mut self) {
        self.state = match self.state {
            ShiftState::ToFuture(s) if s + 1 >= TO_FUTURE_STEPS => ShiftState::Future,
            ShiftState::ToFuture(s) => ShiftState::ToFuture(s + 1),
            ShiftState::ToPast(s) if s + 1 >= TO_PAST_STEPS => {
                self.nostalgia = NOSTALGIA_STEPS;
                ShiftState::Past
            }
            ShiftState::ToPast(s) => ShiftState::ToPast(s + 1),
            ShiftState::Past => {
                self.nostalgia = self.nostalgia.saturating_sub(1);
                ShiftState::Past
            }
            ShiftState::Future => ShiftState::Future,
        };
    }

    pub fn params(&self, reduced_motion: bool) -> ShiftParams {
        match self.state {
            ShiftState::Past => {
                let k = self.nostalgia as f32 / NOSTALGIA_STEPS as f32;
                ShiftParams { crt: CRT_BASE + (CRT_NOSTALGIA - CRT_BASE) * k, warmth: k, ..PAST }
            }
            ShiftState::Future => FUTURE,
            ShiftState::ToFuture(s) if reduced_motion => {
                ShiftParams { world_mix: ease_in_out(s as f32 / REDUCED_FADE_STEPS as f32), ..FUTURE }
            }
            ShiftState::ToFuture(s) => future_curve(s as f32 * STEP_SECONDS),
            ShiftState::ToPast(s) => {
                // The past is about to show: it already glows with nostalgia.
                let nostalgic = |p: ShiftParams| ShiftParams { crt: CRT_NOSTALGIA, warmth: 1.0, ..p };
                if reduced_motion {
                    let mix = 1.0 - ease_in_out(s as f32 / REDUCED_FADE_STEPS as f32);
                    return nostalgic(ShiftParams { world_mix: mix, ..FUTURE });
                }
                // Replay the to-future curve backwards, stretched over 1.8 s.
                let remaining = (TO_PAST_STEPS - 1 - s) as f32 / (TO_PAST_STEPS - 1) as f32;
                nostalgic(future_curve(remaining * CURVE_SECONDS))
            }
        }
    }
}
