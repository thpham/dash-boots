/// Fixed-timestep accumulator stepping.
///
/// Minimal implementation for MVP: clamp dt to avoid spiral-of-death.
#[derive(Debug, Clone, Copy)]
pub struct FixedTimestep {
    pub step_dt: f32,
    pub max_frame_dt: f32,
    pub accumulator: f32,
}

impl FixedTimestep {
    pub fn new(step_hz: f32) -> Self {
        Self {
            step_dt: 1.0 / step_hz,
            max_frame_dt: 0.25,
            accumulator: 0.0,
        }
    }

    pub fn push_frame_dt(&mut self, dt: f32) {
        let dt = dt.clamp(0.0, self.max_frame_dt);
        self.accumulator += dt;
    }

    pub fn pop_step(&mut self) -> bool {
        if self.accumulator >= self.step_dt {
            self.accumulator -= self.step_dt;
            true
        } else {
            false
        }
    }
}
