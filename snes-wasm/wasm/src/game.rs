use crate::audio::{AudioEvent, Sfx};
use crate::input::{Buttons, InputState, InputTracker};
use crate::math::Vec2;
use crate::sim::fixed_timestep::FixedTimestep;
use crate::tuning::{DT, TUNING as T};
use crate::world::camera::Camera;
use crate::world::enemy::{Enemy, EnemyState};
use crate::world::entities::{Checkpoint, Coin, Goal, Pickup, Rift};
use crate::world::level::{Level, LevelData, SpawnPoint};
use crate::world::particles::{Particle, ParticleKind, Particles};
use crate::world::player::Player;
use crate::world::shift::{Shift, World};

/// Music track ids (index into `audio::MUSIC_NAMES`).
pub const MUSIC_LEVEL1: u8 = 0;

const DEATH_DURATION: f32 = 1.6;
/// Delay before the clear screen accepts input.
const CLEAR_INPUT_DELAY: f32 = 1.2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    Title,
    Playing,
    Dying,
    Won,
}

fn feet(p: SpawnPoint) -> Vec2 {
    Vec2::new(p.x, p.y)
}

pub struct Game {
    pub data: LevelData,
    pub level: Level,
    pub player: Player,
    pub enemies: Vec<Enemy>,
    pub coins: Vec<Coin>,
    pub pickups: Vec<Pickup>,
    pub checkpoints: Vec<Checkpoint>,
    pub rifts: Vec<Rift>,
    pub goal: Goal,
    pub particles: Particles,
    pub camera: Camera,
    pub phase: Phase,
    pub shift: Shift,
    /// Seconds spent in the current phase.
    pub phase_time: f32,
    pub paused: bool,
    pub respawn: Vec2,
    pub respawn_world: World,
    pub coins_collected: u32,
    pub coins_total: u32,
    /// Play time in seconds (stops on clear).
    pub time: f32,
    pub deaths: u32,
    /// Simulation steps since boot (animation clock).
    pub frame: u64,
    input: InputTracker,
    timestep: FixedTimestep,
    audio: Vec<AudioEvent>,
}

impl Game {
    pub fn new(data: LevelData, view_w: f32, view_h: f32) -> Self {
        let level = Level::from_data(&data);
        let mut coins = Vec::new();
        for ty in 0..data.height {
            for tx in 0..data.width {
                if data.coins[(ty * data.width + tx) as usize] != 0 {
                    coins.push(Coin {
                        tile_pos: Vec2::new(tx as f32 * data.tile_size, ty as f32 * data.tile_size),
                        collected: false,
                    });
                }
            }
        }
        let respawn = feet(data.spawns.player);
        let mut game = Self {
            level,
            player: Player::spawn_at(respawn),
            enemies: Vec::new(),
            coins_total: coins.len() as u32,
            coins,
            pickups: Vec::new(),
            checkpoints: data
                .spawns
                .checkpoints
                .iter()
                .map(|c| Checkpoint { feet: feet(*c), active: false, world: World::Past })
                .collect(),
            rifts: data.spawns.rifts.iter().map(|r| Rift { feet: feet(*r), active: true }).collect(),
            respawn_world: World::Past,
            shift: Shift::new(World::Past),
            goal: Goal { feet: feet(data.spawns.goal) },
            particles: Particles::default(),
            camera: Camera::new(view_w, view_h),
            phase: Phase::Title,
            phase_time: 0.0,
            paused: false,
            respawn,
            coins_collected: 0,
            time: 0.0,
            deaths: 0,
            frame: 0,
            input: InputTracker::default(),
            timestep: FixedTimestep::new(60.0),
            audio: Vec::new(),
            data,
        };
        game.reset_actors();
        game
    }

    /// Start playing immediately (skips the title screen).
    pub fn start(&mut self) {
        self.set_phase(Phase::Playing);
        self.audio.push(AudioEvent::PlayMusic(MUSIC_LEVEL1));
        self.audio.push(AudioEvent::MusicFuture(self.shift.world() == World::Future));
    }

    /// Pause if currently playing (host lost focus). No-op otherwise.
    pub fn pause(&mut self) {
        if !self.paused && matches!(self.phase, Phase::Playing | Phase::Dying) {
            self.paused = true;
            self.audio.push(AudioEvent::PauseMusic);
        }
    }

    /// Restart the level from scratch, straight into play.
    pub fn restart(&mut self) {
        let data = self.data.clone();
        let (vw, vh) = (self.camera.view_w, self.camera.view_h);
        let fresh = Game::new(data, vw, vh);
        // Keep host-facing state: the input tracker (a held Enter must not
        // count as a new press), the timestep accumulator and the anim clock.
        let (input, timestep, frame) = (self.input, self.timestep, self.frame);
        let audio = std::mem::take(&mut self.audio);
        *self = Game { input, timestep, frame, audio, ..fresh };
        self.start();
    }

    /// Enemies, power-ups and the player back to their spawn state (used on
    /// boot and after each death). Coins and checkpoints persist.
    fn reset_actors(&mut self) {
        self.enemies = self.data.spawns.enemies.iter().map(|e| Enemy::spawn_at(feet(*e))).collect();
        // Dying always costs the boots, so the pickups come back too.
        self.pickups =
            self.data.spawns.dash_boots.iter().map(|b| Pickup { feet: feet(*b), taken: false }).collect();
        self.player = Player::spawn_at(self.respawn);
        self.particles.clear();
        self.camera.snap_to(self.player.feet(), self.level.width_px(), self.level.height_px());
        // Respawn settled in the checkpoint's world; rifts ahead are usable again.
        self.shift = Shift::new(self.respawn_world);
        let rx = self.respawn.x;
        for r in &mut self.rifts {
            if r.feet.x > rx {
                r.active = true;
            }
        }
    }

    fn set_phase(&mut self, phase: Phase) {
        self.phase = phase;
        self.phase_time = 0.0;
    }

    fn sfx(&mut self, s: Sfx) {
        self.audio.push(AudioEvent::Sfx(s));
    }

    pub fn drain_audio(&mut self) -> Vec<AudioEvent> {
        std::mem::take(&mut self.audio)
    }

    /// Feed one host frame: accumulate `frame_dt` seconds and run as many fixed
    /// steps as needed. Returns the number of steps run.
    pub fn advance(&mut self, input: InputState, frame_dt: f32) -> u32 {
        self.input.feed(input);
        self.timestep.push_frame_dt(frame_dt);
        let mut steps = 0;
        while self.timestep.pop_step() {
            let b = self.input.consume();
            self.update(&b);
            steps += 1;
        }
        steps
    }

    /// One fixed simulation step.
    pub fn update(&mut self, b: &Buttons) {
        self.frame += 1;

        if self.paused {
            if b.start_pressed {
                self.paused = false;
                self.audio.push(AudioEvent::ResumeMusic);
            }
            return;
        }
        self.phase_time += DT;
        if self.phase != Phase::Title {
            self.shift.step();
        }

        match self.phase {
            Phase::Title => {
                if b.start_pressed || b.jump_pressed {
                    self.start();
                }
                self.particles.update(DT);
            }
            Phase::Playing => {
                if b.start_pressed {
                    self.paused = true;
                    self.sfx(Sfx::Pause);
                    self.audio.push(AudioEvent::PauseMusic);
                    return;
                }
                self.time += DT;
                self.update_playing(b);
            }
            Phase::Dying => {
                self.player.vel.y = (self.player.vel.y + T.gravity_fall * 0.5 * DT).min(T.max_fall);
                self.player.pos = self.player.pos + self.player.vel * DT;
                self.particles.update(DT);
                if self.phase_time >= DEATH_DURATION {
                    self.reset_actors();
                    self.set_phase(Phase::Playing);
                    self.audio.push(AudioEvent::PlayMusic(MUSIC_LEVEL1));
                    self.audio.push(AudioEvent::MusicFuture(self.shift.world() == World::Future));
                }
            }
            Phase::Won => {
                // Let the player land, no control.
                let idle = Buttons::default();
                self.player.step(&idle, &self.level, DT);
                if self.frame.is_multiple_of(10) && self.phase_time < 2.5 {
                    let x = self.goal.feet.x + ((self.frame / 10) % 5) as f32 * 9.0 - 18.0;
                    let at = Vec2::new(x, self.goal.feet.y - 60.0 - ((self.frame / 10) % 3) as f32 * 14.0);
                    self.particles.burst(ParticleKind::Sparkle, at, 8, 50.0, 0.6);
                }
                self.particles.update(DT);
                if self.phase_time > CLEAR_INPUT_DELAY && (b.start_pressed || b.jump_pressed) {
                    self.restart();
                }
            }
        }
    }

    fn update_playing(&mut self, b: &Buttons) {
        let prev_bottom = self.player.aabb().bottom();
        let ev = self.player.step(b, &self.level, DT);
        let pf = self.player.feet();

        if ev.jumped {
            self.sfx(Sfx::Jump);
            self.particles.dust(pf, 0.0);
        }
        if ev.dashed {
            self.sfx(Sfx::Dash);
            for i in 0..3 {
                self.particles.dust(
                    pf + Vec2::new(-self.player.facing * i as f32 * 4.0, -4.0),
                    -self.player.facing * 30.0,
                );
            }
        }
        if ev.landed {
            self.particles.dust(pf + Vec2::new(-4.0, 0.0), -20.0);
            self.particles.dust(pf + Vec2::new(4.0, 0.0), 20.0);
        }
        if ev.bumped {
            self.sfx(Sfx::Bump);
        }
        if ev.slide_started {
            self.particles.dust(pf, 0.0);
        }
        if self.player.is_dashing() && self.frame.is_multiple_of(3) {
            self.particles.dust(pf + Vec2::new(0.0, -8.0), -self.player.facing * 20.0);
        }
        if self.player.sliding && self.player.on_ground && self.frame.is_multiple_of(6) {
            self.particles.dust(pf, -self.player.vel.x.signum() * 15.0);
        }

        // Fell into a pit.
        if self.player.pos.y > self.level.height_px() + 8.0 {
            self.kill_player();
            return;
        }

        self.camera.follow(
            self.player.feet(),
            self.player.on_ground,
            self.level.width_px(),
            self.level.height_px(),
            DT,
        );

        // Wake enemies just before they scroll into view.
        let wake_x = self.camera.x + self.camera.view_w + 8.0;
        for e in &mut self.enemies {
            e.step(&self.level, DT, wake_x);
        }

        self.player_vs_enemies(b, prev_bottom);
        if self.phase != Phase::Playing {
            return;
        }
        self.collect_items();
        self.particles.update(DT);
    }

    fn player_vs_enemies(&mut self, b: &Buttons, prev_bottom: f32) {
        let mut stomped = false;
        let mut hurt = false;
        for i in 0..self.enemies.len() {
            let e = self.enemies[i];
            if !e.is_dangerous() || !self.player.aabb().overlaps(&e.aabb()) {
                continue;
            }
            let center = Vec2::new(e.aabb().center_x(), e.pos.y + 7.0);
            let p = &self.player;
            if p.is_dashing() || (p.sliding && p.vel.x.abs() > 60.0) {
                let dir = p.vel.x.signum();
                self.enemies[i].knock(dir);
                self.sfx(Sfx::Stomp);
                self.particles.burst(ParticleKind::Star, center, 4, 60.0, 0.3);
            } else if p.vel.y > 0.0 && prev_bottom <= e.aabb().top() + 6.0 {
                self.enemies[i].squash();
                stomped = true;
                self.sfx(Sfx::Stomp);
                self.particles.burst(ParticleKind::Star, center, 4, 60.0, 0.3);
            } else if p.invuln <= 0.0 && !stomped {
                hurt = true;
            }
        }
        if stomped {
            // Hold jump for a higher bounce.
            let held = b.held.jump;
            self.player.vel.y = if held { T.stomp_bounce_held } else { T.stomp_bounce };
            self.player.jumping = held;
            self.player.on_ground = false;
        } else if hurt {
            self.hurt_player();
        }
    }

    fn collect_items(&mut self) {
        let pa = self.player.aabb();
        for i in 0..self.coins.len() {
            if !self.coins[i].collected && pa.overlaps(&self.coins[i].aabb()) {
                self.coins[i].collected = true;
                self.coins_collected += 1;
                self.sfx(Sfx::Coin);
                let c = self.coins[i].tile_pos + Vec2::new(8.0, 8.0);
                self.particles.burst(ParticleKind::Sparkle, c, 4, 40.0, 0.3);
            }
        }
        for i in 0..self.pickups.len() {
            if !self.pickups[i].taken && pa.overlaps(&self.pickups[i].aabb()) {
                self.pickups[i].taken = true;
                self.player.dash_boots = true;
                self.sfx(Sfx::Powerup);
                let c = self.pickups[i].feet - Vec2::new(0.0, 8.0);
                self.particles.burst(ParticleKind::Sparkle, c, 10, 70.0, 0.5);
            }
        }
        for i in 0..self.checkpoints.len() {
            if !self.checkpoints[i].active && pa.overlaps(&self.checkpoints[i].aabb()) {
                self.checkpoints[i].active = true;
                self.respawn = self.checkpoints[i].feet;
                self.checkpoints[i].world = self.shift.world();
                self.respawn_world = self.checkpoints[i].world;
                self.sfx(Sfx::Checkpoint);
                let c = self.checkpoints[i].feet - Vec2::new(-4.0, 26.0);
                self.particles.burst(ParticleKind::Sparkle, c, 8, 50.0, 0.4);
            }
        }
        for i in 0..self.rifts.len() {
            if self.rifts[i].active && !self.shift.is_shifting() && pa.overlaps(&self.rifts[i].aabb()) {
                self.rifts[i].active = false;
                self.shift.trigger();
                let future = self.shift.world() == World::Future;
                self.sfx(if future { Sfx::Rift } else { Sfx::Return });
                self.audio.push(AudioEvent::MusicFuture(future));
            }
        }
        if pa.overlaps(&self.goal.aabb()) {
            self.set_phase(Phase::Won);
            self.player.dash_timer = 0.0;
            self.player.vel.x = 0.0;
            self.audio.push(AudioEvent::StopMusic);
            self.sfx(Sfx::Clear);
        }
    }

    /// Enemy contact: the dash boots absorb one hit, otherwise the player dies.
    fn hurt_player(&mut self) {
        if self.player.dash_boots {
            self.player.dash_boots = false;
            self.player.invuln = T.invuln_time;
            self.player.dash_timer = 0.0;
            self.player.vel = Vec2::new(-self.player.facing * 120.0, -220.0);
            self.player.jumping = false;
            self.sfx(Sfx::Hurt);
        } else {
            self.kill_player();
        }
    }

    fn kill_player(&mut self) {
        self.deaths += 1;
        self.player.vel = Vec2::new(0.0, -330.0);
        self.player.dash_boots = false;
        self.player.crouching = false;
        self.set_phase(Phase::Dying);
        self.audio.push(AudioEvent::StopMusic);
        self.sfx(Sfx::Death);
    }

    /// Enemies still alive (not squashed/knocked/dead).
    pub fn enemies_alive(&self) -> usize {
        self.enemies.iter().filter(|e| matches!(e.state, EnemyState::Dormant | EnemyState::Walking)).count()
    }

    pub fn particles_list(&self) -> &[Particle] {
        &self.particles.list
    }
}
