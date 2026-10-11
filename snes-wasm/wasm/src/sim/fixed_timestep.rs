/// Fixed-timestep accumulator.
///
/// Frame deltas are clamped to `max_frame_dt` so a long stall (tab in background,
/// debugger pause) never triggers a spiral of death: at most
/// `max_frame_dt / step_dt` steps run for a single frame.
#[derive(Debug, Clone, Copy)]
pub struct FixedTimestep {
    pub step_dt: f32,
    pub max_frame_dt: f32,
    pub accumulator: f32,
}

impl FixedTimestep {
    pub fn new(step_hz: f32) -> Self {
        Self { step_dt: 1.0 / step_hz, max_frame_dt: 0.1, accumulator: 0.0 }
    }

    pub fn push_frame_dt(&mut self, dt: f32) {
        let dt = if dt.is_finite() { dt.clamp(0.0, self.max_frame_dt) } else { 0.0 };
        self.accumulator += dt;
    }

    pub fn pop_step(&mut self) -> bool {
        // Small tolerance so a 60 Hz display with jittery timestamps still gets
        // exactly one step per frame on average.
        if self.accumulator >= self.step_dt - 1e-4 {
            self.accumulator = (self.accumulator - self.step_dt).max(0.0);
            true
        } else {
            false
        }
    }
}
