//! The playthrough bots: the critic of generate → play → accept, and the
//! difficulty meter. One controller (the shipped-level bot, unchanged for the
//! expert) degraded by a skill profile: reaction delay, seeded timing noise,
//! and whether it holds the run button. The player of difficulty d is
//! `PROFILES[..d]`: it finishes a level if any of those strategies does.

use super::rng::Rng;
use crate::game::{Game, Phase};
use crate::input::InputState;
use crate::tuning::DT;
use crate::world::level::LevelData;
use crate::world::physics::overlaps_solid;
use crate::world::player;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkillProfile {
    pub name: &'static str,
    /// Steps between seeing a hazard and pressing jump.
    pub reaction: u32,
    /// Extra random delay, uniform in `0..=jitter` steps (seeded).
    pub jitter: u32,
    /// Holds the run button (else walks everywhere).
    pub run: bool,
}

/// Novice → expert, two walkers then three runners. Tuned so the shipped
/// campaign orders sensibly (tests/levelgen_handmade.rs) while spreading the
/// measured difficulties of generated levels over d1..d5.
pub const PROFILES: [SkillProfile; 5] = [
    SkillProfile { name: "novice", reaction: 4, jitter: 3, run: false },
    SkillProfile { name: "casual", reaction: 2, jitter: 2, run: false },
    SkillProfile { name: "average", reaction: 3, jitter: 2, run: true },
    SkillProfile { name: "skilled", reaction: 1, jitter: 1, run: true },
    SkillProfile { name: "expert", reaction: 0, jitter: 0, run: true },
];
pub const EXPERT: SkillProfile = PROFILES[4];
/// Deaths a profile may spend and still count as finishing.
pub const MAX_DEATHS: u32 = 2;

pub struct Bot {
    profile: SkillProfile,
    rng: Rng,
    jump_hold: u32,
    /// Pending jump: (steps to wait, hold length).
    armed: Option<(u32, u32)>,
}

impl Bot {
    pub fn new(profile: SkillProfile, seed: u64) -> Self {
        Bot { profile, rng: Rng::new(seed), jump_hold: 0, armed: None }
    }

    /// Forget any jump in progress (after a death).
    pub fn reset(&mut self) {
        self.jump_hold = 0;
        self.armed = None;
    }

    pub fn decide(&mut self, g: &Game) -> InputState {
        let p = &g.player;
        let lvl = &g.level;
        let mut inp = InputState { right: true, dash: self.profile.run, ..Default::default() };
        let front = p.pos.x + player::WIDTH;
        let feet = p.feet().y;

        // Low ceiling ahead (crawl space): stand-height blocked, crouch-height free.
        let stand_blocked =
            overlaps_solid(lvl, front + 2.0, feet - player::HEIGHT_STAND, 12.0, player::HEIGHT_STAND - 1.0);
        let crouch_free = !overlaps_solid(
            lvl,
            front + 2.0,
            feet - player::HEIGHT_CROUCH,
            12.0,
            player::HEIGHT_CROUCH - 1.0,
        );
        let tunnel = (stand_blocked && crouch_free)
            || (p.crouching && overlaps_solid(lvl, p.pos.x, feet - player::HEIGHT_STAND, player::WIDTH, 8.0));
        if tunnel && p.on_ground {
            inp.down = true;
            return inp;
        }

        // Gap planning: if a short gap starts soon, walk instead of run so the
        // jump lands on the next (possibly small) platform.
        let has_floor = |x: f32| overlaps_solid(lvl, x, feet - 40.0, 1.0, lvl.height_px() - feet + 41.0);
        if let Some(start) = (0..6).map(|i| front + 2.0 + i as f32 * 8.0).find(|x| !has_floor(*x)) {
            let width =
                (0..16).map(|i| start + i as f32 * 8.0).take_while(|x| !has_floor(*x)).count() as f32 * 8.0;
            let land = start + width;
            let landing =
                (0..8).map(|i| land + i as f32 * 8.0).take_while(|x| has_floor(*x)).count() as f32 * 8.0;
            if width <= 56.0 && landing <= 48.0 {
                inp.dash = false;
            }
        }

        let wall_ahead = overlaps_solid(lvl, front, feet - 20.0, 20.0, 18.0);
        let gap_ahead = (0..=3).any(|i| {
            let x = front + 2.0 + i as f32 * 2.0;
            !overlaps_solid(lvl, x, feet, 1.0, lvl.height_px() - feet + 1.0)
        });
        let enemy_ahead = g.enemies.iter().any(|e| {
            e.is_dangerous()
                && e.pos.x - front > -4.0
                && e.pos.x - front < 40.0
                && (e.pos.y + 14.0 - feet).abs() < 20.0
        });

        // Release the button once landed so the next jump is a fresh press.
        if p.on_ground && self.jump_hold > 0 && self.jump_hold < 20 {
            self.jump_hold = 0;
            return inp;
        }
        // Air control: when descending over a landing spot with a pit right
        // after it, brake (like a human aiming for a small platform).
        if !p.on_ground && p.vel.y > 0.0 {
            let probe = |x: f32| overlaps_solid(lvl, x, feet, 1.0, 64.0);
            if probe(p.pos.x + player::WIDTH * 0.5) && !probe(front + 10.0) {
                inp.right = false;
                inp.left = true;
            }
        }
        if self.jump_hold > 0 {
            self.jump_hold -= 1;
            inp.jump = true;
            return inp;
        }
        if p.on_ground && (wall_ahead || gap_ahead || enemy_ahead) && self.armed.is_none() {
            // Small hop for low steps, full jump otherwise.
            let ts = lvl.tile_size;
            let wall_h = [2.0, 8.0, 14.0, 20.0]
                .iter()
                .map(|dx| {
                    (1..=5)
                        .take_while(|i| lvl.is_solid_at_pixel(front + dx, feet - *i as f32 * ts + 1.0))
                        .count()
                })
                .max()
                .unwrap_or(0);
            let hold = if wall_ahead && !gap_ahead && !enemy_ahead && wall_h <= 1 { 6 } else { 24 };
            let delay = self.profile.reaction + self.rng.below(self.profile.jitter as usize + 1) as u32;
            self.armed = Some((delay, hold));
        }
        if let Some((delay, hold)) = self.armed {
            if !p.on_ground {
                self.armed = None;
            } else if delay == 0 {
                self.armed = None;
                self.jump_hold = hold;
                inp.jump = true;
            } else {
                self.armed = Some((delay - 1, hold));
            }
        }
        inp
    }
}

#[derive(Clone, Debug)]
pub struct Report {
    pub won: bool,
    pub seconds: f32,
    pub deaths: u32,
    /// Player x (px) at each death.
    pub death_x: Vec<f32>,
    /// Player x (px) where no progress was made for 8 s.
    pub stuck_x: Option<f32>,
    pub coins: u32,
    pub coins_total: u32,
    /// Furthest tile column reached.
    pub best_tile: u32,
}

/// Play a level with one profile until the goal, `max_deaths + 1` deaths, 8 s
/// without progress, or 4 minutes. `won` means reached with ≤ `max_deaths`.
pub fn play(data: LevelData, profile: SkillProfile, seed: u64, max_deaths: u32) -> Report {
    let mut g = Game::new(data, 320.0, 180.0);
    g.start();
    let mut bot = Bot::new(profile, seed);
    let (mut best_x, mut furthest, mut last_progress) = (0.0f32, 0.0f32, 0u32);
    let (mut death_x, mut stuck_x) = (Vec::new(), None);
    let (mut won, mut steps, mut was_dying) = (false, 0u32, false);
    for step in 0..60 * 240 {
        steps = step;
        let inp = if g.phase == Phase::Playing {
            bot.decide(&g)
        } else {
            bot.reset();
            InputState::default()
        };
        g.advance(inp, DT);
        let dying = g.phase == Phase::Dying;
        if dying && !was_dying {
            death_x.push(g.player.pos.x);
            if g.deaths > max_deaths {
                break;
            }
        }
        if was_dying && !dying {
            // Respawned: progress counts again from the checkpoint.
            best_x = g.player.pos.x;
            last_progress = step;
        }
        was_dying = dying;
        if g.phase == Phase::Won {
            won = true;
            break;
        }
        if g.player.pos.x > best_x + 8.0 {
            best_x = g.player.pos.x;
            furthest = furthest.max(best_x);
            last_progress = step;
        }
        if step - last_progress >= 60 * 8 {
            stuck_x = Some(g.player.pos.x);
            break;
        }
    }
    Report {
        won: won && g.deaths <= max_deaths,
        seconds: steps as f32 / 60.0,
        deaths: g.deaths,
        death_x,
        stuck_x,
        coins: g.coins_collected,
        coins_total: g.coins_total,
        best_tile: (furthest / 16.0) as u32,
    }
}

/// Does the player of difficulty `d` finish it? That player has every
/// strategy up to its skill (an expert can also walk carefully), so this is
/// monotonic in `d` by construction.
pub fn finishes(data: &LevelData, difficulty: u8, seed: u64) -> bool {
    measure_up_to(data, difficulty.clamp(1, 5), seed).is_some()
}

/// `finishes` in at least 2 of 3 attempts (derived seeds): filters the bots'
/// own timing noise out of quality measurements (a novice repeats a won d1
/// level only ~92 % of the time).
pub fn finishes_reliably(data: &LevelData, difficulty: u8, seed: u64) -> bool {
    (0..3u64).filter(|k| finishes(data, difficulty, seed.wrapping_add(k * 0x9E37))).count() >= 2
}

/// Weakest profile among the first `max_difficulty` that finishes, as a
/// difficulty (1-based); `None` if none of them does.
pub fn measure_up_to(data: &LevelData, max_difficulty: u8, seed: u64) -> Option<u8> {
    (0..max_difficulty.min(5) as usize)
        .find(|&i| play(data.clone(), PROFILES[i], seed ^ i as u64, MAX_DEATHS).won)
        .map(|i| i as u8 + 1)
}

pub fn measure(data: &LevelData, seed: u64) -> Option<u8> {
    measure_up_to(data, 5, seed)
}
