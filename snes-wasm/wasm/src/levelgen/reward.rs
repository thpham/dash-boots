//! GRPO reward for one sampled level at a target difficulty (spec §3):
//! −1 if the target-skill bot cannot finish it; otherwise closeness of the
//! measured difficulty to the target, plus campaign style, minus repetition.

use super::bot;
use super::compile::CompiledLevel;
use super::handmade::style_score;
use super::motif::Motif;

/// Weight of the campaign style. Kept low: at 0.3 GRPO collapsed onto campaign
/// bigrams (16/40 distinct levels per cell).
pub const STYLE_WEIGHT: f32 = 0.1;

#[derive(Clone, Debug, PartialEq)]
pub struct Reward {
    pub total: f32,
    pub measured: Option<u8>,
    pub style: f32,
    pub repetition: f32,
}

/// Share of positions repeating the previous motif.
pub fn repetition(seq: &[Motif]) -> f32 {
    if seq.len() < 2 {
        return 0.0;
    }
    seq.windows(2).filter(|w| w[0] == w[1]).count() as f32 / (seq.len() - 1) as f32
}

pub fn score(level: &CompiledLevel, target: u8, seed: u64) -> Reward {
    let target = target.clamp(1, 5);
    let measured = bot::measure_up_to(&level.level_data(), target, seed);
    let style = style_score(&level.motifs);
    let repetition = repetition(&level.motifs);
    let total = match measured {
        None => -1.0,
        Some(m) => 1.0 - (target - m) as f32 / 4.0 + STYLE_WEIGHT * style - 0.2 * repetition,
    };
    Reward { total, measured, style, repetition }
}
