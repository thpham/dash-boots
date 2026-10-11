use crate::math::Vec2;

/// Side-scrolling camera with a horizontal deadzone, smoothed vertical follow
/// and hard clamping to the level bounds.
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub x: f32,
    pub y: f32,
    pub view_w: f32,
    pub view_h: f32,
}

impl Camera {
    pub fn new(view_w: f32, view_h: f32) -> Self {
        Self { x: 0.0, y: 0.0, view_w, view_h }
    }

    /// Feet should sit at this fraction of the screen height.
    const FEET_Y: f32 = 0.72;
    const DEADZONE_LEFT: f32 = 0.38;
    const DEADZONE_RIGHT: f32 = 0.48;

    pub fn snap_to(&mut self, feet: Vec2, level_w: f32, level_h: f32) {
        self.x = feet.x - self.view_w * 0.4;
        self.y = feet.y - self.view_h * Self::FEET_Y;
        self.clamp(level_w, level_h);
    }

    pub fn follow(&mut self, feet: Vec2, on_ground: bool, level_w: f32, level_h: f32, dt: f32) {
        let left = self.x + self.view_w * Self::DEADZONE_LEFT;
        let right = self.x + self.view_w * Self::DEADZONE_RIGHT;
        if feet.x > right {
            self.x += feet.x - right;
        } else if feet.x < left {
            self.x -= left - feet.x;
        }

        // Vertical: re-center when landing (platformer-style "platform snap"),
        // but always keep the player on screen while airborne.
        let target = feet.y - self.view_h * Self::FEET_Y;
        if on_ground {
            self.y += (target - self.y) * (1.0 - (-6.0 * dt).exp());
        }
        let min_y = feet.y - self.view_h + 12.0; // feet not below the bottom edge
        let max_y = feet.y - 48.0; // head not above the top edge
        self.y = self.y.max(min_y).min(max_y);
        self.clamp(level_w, level_h);
    }

    pub fn clamp(&mut self, level_w: f32, level_h: f32) {
        self.x = self.x.min(level_w - self.view_w).max(0.0);
        self.y = self.y.min(level_h - self.view_h).max(0.0);
    }

    /// Integer camera position for pixel-perfect rendering.
    pub fn pixel_pos(&self) -> Vec2 {
        Vec2::new(self.x.round(), self.y.round())
    }
}
