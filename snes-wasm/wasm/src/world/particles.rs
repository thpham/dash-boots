use crate::math::Vec2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParticleKind {
    Dust,
    Sparkle,
    Star,
}

#[derive(Clone, Copy, Debug)]
pub struct Particle {
    /// Center position.
    pub pos: Vec2,
    pub vel: Vec2,
    pub age: f32,
    pub life: f32,
    pub gravity: f32,
    pub kind: ParticleKind,
}

impl Particle {
    pub fn new(kind: ParticleKind, pos: Vec2, vel: Vec2, life: f32) -> Self {
        Self { pos, vel, age: 0.0, life, gravity: 0.0, kind }
    }

    /// Normalized age in [0, 1].
    pub fn t(&self) -> f32 {
        (self.age / self.life).clamp(0.0, 1.0)
    }
}

#[derive(Default, Debug)]
pub struct Particles {
    pub list: Vec<Particle>,
}

impl Particles {
    pub fn spawn(&mut self, p: Particle) {
        // Hard cap: particles are cosmetic.
        if self.list.len() < 256 {
            self.list.push(p);
        }
    }

    pub fn dust(&mut self, at: Vec2, vx: f32) {
        self.spawn(Particle::new(ParticleKind::Dust, at, Vec2::new(vx, -12.0), 0.3));
    }

    pub fn burst(&mut self, kind: ParticleKind, at: Vec2, count: u32, speed: f32, life: f32) {
        for i in 0..count {
            let a = i as f32 / count as f32 * std::f32::consts::TAU;
            self.spawn(Particle::new(kind, at, Vec2::new(a.cos() * speed, a.sin() * speed), life));
        }
    }

    pub fn update(&mut self, dt: f32) {
        for p in &mut self.list {
            p.age += dt;
            p.vel.y += p.gravity * dt;
            p.pos = p.pos + p.vel * dt;
        }
        self.list.retain(|p| p.age < p.life);
    }

    pub fn clear(&mut self) {
        self.list.clear();
    }
}
