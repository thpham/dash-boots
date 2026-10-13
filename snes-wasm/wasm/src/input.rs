//! Platform-agnostic input: the host packs held buttons into a bitmask.

pub const BTN_LEFT: u32 = 1 << 0;
pub const BTN_RIGHT: u32 = 1 << 1;
pub const BTN_UP: u32 = 1 << 2;
pub const BTN_DOWN: u32 = 1 << 3;
pub const BTN_JUMP: u32 = 1 << 4;
pub const BTN_DASH: u32 = 1 << 5;
pub const BTN_START: u32 = 1 << 6;

/// Buttons currently held.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct InputState {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub jump: bool,
    pub dash: bool,
    pub start: bool,
}

impl InputState {
    pub fn from_bits(bits: u32) -> Self {
        Self {
            left: bits & BTN_LEFT != 0,
            right: bits & BTN_RIGHT != 0,
            up: bits & BTN_UP != 0,
            down: bits & BTN_DOWN != 0,
            jump: bits & BTN_JUMP != 0,
            dash: bits & BTN_DASH != 0,
            start: bits & BTN_START != 0,
        }
    }
}

/// Held state plus press edges for one simulation step.
#[derive(Clone, Copy, Default, Debug)]
pub struct Buttons {
    pub held: InputState,
    pub jump_pressed: bool,
    pub dash_pressed: bool,
    pub start_pressed: bool,
    pub up_pressed: bool,
    pub down_pressed: bool,
    pub left_pressed: bool,
    pub right_pressed: bool,
}

/// Turns host frames into per-step `Buttons`.
///
/// Press edges are detected once per host frame and kept pending until a
/// simulation step consumes them, so a tap is never lost when a frame runs
/// zero steps (high refresh-rate displays) and never doubled when it runs several.
#[derive(Clone, Copy, Default, Debug)]
pub struct InputTracker {
    last: InputState,
    pending_jump: bool,
    pending_dash: bool,
    pending_start: bool,
    pending_up: bool,
    pending_down: bool,
    pending_left: bool,
    pending_right: bool,
}

impl InputTracker {
    pub fn feed(&mut self, input: InputState) {
        self.pending_jump |= input.jump && !self.last.jump;
        self.pending_dash |= input.dash && !self.last.dash;
        self.pending_start |= input.start && !self.last.start;
        self.pending_up |= input.up && !self.last.up;
        self.pending_down |= input.down && !self.last.down;
        self.pending_left |= input.left && !self.last.left;
        self.pending_right |= input.right && !self.last.right;
        self.last = input;
    }

    /// Start from buttons already held (no press edge until released).
    pub fn prime(&mut self, held: InputState) {
        self.last = held;
    }

    pub fn consume(&mut self) -> Buttons {
        let b = Buttons {
            held: self.last,
            jump_pressed: self.pending_jump,
            dash_pressed: self.pending_dash,
            start_pressed: self.pending_start,
            up_pressed: self.pending_up,
            down_pressed: self.pending_down,
            left_pressed: self.pending_left,
            right_pressed: self.pending_right,
        };
        self.pending_jump = false;
        self.pending_dash = false;
        self.pending_start = false;
        self.pending_up = false;
        self.pending_down = false;
        self.pending_left = false;
        self.pending_right = false;
        b
    }
}
