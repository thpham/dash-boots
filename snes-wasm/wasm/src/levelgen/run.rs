//! An endless run: level n of a run (seed, start difficulty) has difficulty
//! `min(5, start + n/2)`, the theme rotation of the campaign, and a seed
//! derived from (run seed, n). Each level is the model's first sample that
//! the player of that difficulty (bot profile) finishes, within 8 tries; then
//! the teacher (4 tries); then the theme's campaign transcription — there is
//! always a level, in bounded time (~300 ms worst case).

use super::bot;
use super::compile::{compile_strict, CompiledLevel};
use super::handmade;
use super::model::{sample, Model};
use super::motif::{vocab::MAX_DIFFICULTY, Theme};
use super::rng::splitmix64;
use super::teacher;

pub const MODEL_TRIES: u32 = 8;
pub const TEACHER_TRIES: u32 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Model(u32),
    Teacher,
    Handmade,
}

pub struct RunLevel {
    pub level: CompiledLevel,
    pub source: Source,
    pub theme: Theme,
    pub difficulty: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct Run {
    pub seed: u32,
    pub start: u8,
}

impl Run {
    pub fn difficulty(&self, n: u32) -> u8 {
        (self.start.clamp(1, MAX_DIFFICULTY) as u32 + n / 2).min(MAX_DIFFICULTY as u32) as u8
    }

    pub fn theme(&self, n: u32) -> Theme {
        Theme::ALL[(n % 4) as usize]
    }

    pub fn level_seed(&self, n: u32) -> u64 {
        splitmix64(((self.seed as u64) << 32) | n as u64)
    }

    pub fn level(&self, n: u32, model: Option<&Model>) -> RunLevel {
        self.level_with(n, |theme, d, seed| {
            model.and_then(|m| sample(m, theme, d, seed, 1.0)).map(|t| t.level)
        })
    }

    /// `level` with an injectable sampler (tests).
    pub fn level_with(
        &self,
        n: u32,
        mut sampler: impl FnMut(Theme, u8, u64) -> Option<CompiledLevel>,
    ) -> RunLevel {
        let (theme, d, base) = (self.theme(n), self.difficulty(n), self.level_seed(n));
        let done = |level: CompiledLevel, source| RunLevel { level, source, theme, difficulty: d };
        for i in 0..MODEL_TRIES {
            let seed = base.wrapping_add(i as u64);
            if let Some(level) = sampler(theme, d, seed) {
                if bot::finishes(&level.level_data(), d, seed) {
                    return done(level, Source::Model(i + 1));
                }
            }
        }
        for i in 0..TEACHER_TRIES {
            let seed = base.wrapping_add(100 + i as u64);
            if let Some(level) = compile_strict(theme, &teacher::generate(theme, d, seed)) {
                if bot::finishes(&level.level_data(), d, seed) {
                    return done(level, Source::Teacher);
                }
            }
        }
        done(handmade::for_theme(theme).compile(), Source::Handmade)
    }
}
