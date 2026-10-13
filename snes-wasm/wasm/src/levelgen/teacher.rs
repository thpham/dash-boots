//! The section teacher: a hand-written generator that structures a level like
//! the campaign — a calm intro, 3–5 sections each closed by a checkpoint with
//! a tension-then-rest curve, one or two excursions into the future, an
//! ending — with motif weights driven by the target difficulty and the theme.
//! It writes ~70 % of the corpus (diversity) and is the runtime fallback. Its
//! `target` only biases the content; corpus labels are measured by the bots.

use super::compile::{Compiler, INTRO_END, MAX_BODY};
use super::motif::{Motif, Theme, ALL};
use super::rng::Rng;

/// Base weight of a motif at difficulty `d` (1..=5).
fn weight(m: Motif, d: u8) -> f32 {
    let f = (d.clamp(1, 5) - 1) as f32 / 4.0;
    match m {
        Motif::Flat(_) => 1.0 - 0.5 * f,
        Motif::Rest => 0.8 - 0.4 * f,
        Motif::Cows => 0.4,
        Motif::CoinRow | Motif::PlankCoins => 0.5,
        Motif::Alcove => 0.3,
        Motif::CrawlBricks => 0.4,
        Motif::Wall => 0.6,
        Motif::StairsUp => 0.2,
        Motif::StepUp(1) | Motif::StepDown(1) => 0.6,
        Motif::StepUp(2) | Motif::StepDown(2) => 0.4,
        Motif::StepUp(_) | Motif::StepDown(_) => 0.2 + 0.2 * f,
        Motif::Boots => 0.5,
        Motif::Gap(2) => 1.0 - 0.6 * f,
        Motif::Gap(3) => 1.0,
        // Walking players cannot clear 4+ columns: none at the lowest targets.
        Motif::Gap(4) => (1.4 * (f - 0.2)).max(0.0),
        Motif::Gap(_) => (1.8 * (f - 0.4)).max(0.0),
        Motif::GapPlank(_) => 0.1 + 0.5 * f,
        Motif::GapPillar => (0.8 * (f - 0.2)).max(0.0),
        Motif::Marsh => (0.8 * (f - 0.3)).max(0.0),
        Motif::Climb => (0.6 * (f - 0.2)).max(0.0),
        // The 2-wide pit at the top of the stairs stops walking players.
        Motif::Pyramid(3) => 0.5 * f,
        Motif::Pyramid(_) => (0.6 * (f - 0.25)).max(0.0),
        Motif::Walkers(k) => {
            ((0.6 - 0.15 * k as f32) + 0.4 * f * k as f32 / 3.0) * if k == 3 { f } else { 1.0 }
        }
        Motif::Goat => 0.1 + 0.4 * f,
        Motif::GemEnemy => 0.3,
        Motif::Stash => 0.2 + 0.2 * f,
        Motif::Tunnel(_) => 0.1 + 0.3 * f,
        // Structure is placed by the section logic, never by weight.
        Motif::Checkpoint | Motif::RiftOut | Motif::RiftBack => 0.0,
    }
}

fn theme_weight(m: Motif, theme: Theme) -> f32 {
    match (theme, m) {
        (Theme::Cave, Motif::Tunnel(_)) => 2.0,
        (Theme::Cave, Motif::CrawlBricks | Motif::GemEnemy | Motif::Marsh) => 1.5,
        (Theme::Cave, Motif::Cows) => 0.3,
        (Theme::Cave, Motif::Climb) => 0.5,
        (Theme::Meadow, Motif::Cows) => 1.5,
        (Theme::Meadow, Motif::Gap(4 | 5)) => 0.8,
        (Theme::Sunset, Motif::Goat | Motif::StepUp(_) | Motif::StepDown(_) | Motif::CoinRow) => 1.5,
        (Theme::Night, Motif::Gap(4 | 5) | Motif::Tunnel(_)) => 1.3,
        (Theme::Night, Motif::Walkers(_)) => 1.2,
        _ => 1.0,
    }
}

/// Pick an allowed, weighted motif of the wanted kind (hazard or calm), never
/// repeating the previous one; falls back to calm, then to anything allowed.
fn pick(c: &Compiler, rng: &mut Rng, theme: Theme, target: u8, hazard: bool) -> Option<Motif> {
    let prev = c.motifs.last().copied();
    let draw = |rng: &mut Rng, keep: &dyn Fn(Motif) -> bool| -> Option<Motif> {
        let w: Vec<f32> = ALL
            .iter()
            .map(|&m| {
                if !c.allowed(m) || Some(m) == prev || !keep(m) {
                    0.0
                } else {
                    (weight(m, target) * theme_weight(m, theme)).max(0.0)
                }
            })
            .collect();
        rng.weighted(&w).map(|i| ALL[i])
    };
    draw(rng, &|m| m.is_hazard() == hazard)
        .or_else(|| draw(rng, &|m| !m.is_hazard()))
        .or_else(|| draw(rng, &|_| true))
}

pub fn generate(theme: Theme, target: u8, seed: u64) -> Vec<Motif> {
    let target = target.clamp(1, 5);
    let mut rng = Rng::new(seed ^ ((theme.index() as u64) << 40) ^ ((target as u64) << 48));
    let mut c = Compiler::new(theme);

    // Intro: calm motifs and small gaps (the grammar enforces the zone).
    while c.x() < INTRO_END {
        let hazard = rng.f32() < 0.25;
        match pick(&c, &mut rng, theme, target, hazard) {
            Some(m) => {
                c.push(m);
            }
            None => break,
        }
    }

    let sections = 3 + rng.below(3) as i32;
    let two_excursions = rng.f32() < 0.35 && sections >= 4;
    let span = (MAX_BODY - 20 - INTRO_END) / sections;
    let max_run = 1 + target as u32 / 2;
    for s in 0..sections {
        let start = c.x();
        let end = INTRO_END + span * (s + 1);
        let opens_rift = s == 1 || (two_excursions && s == 3);
        if opens_rift && c.allowed(Motif::RiftOut) {
            c.push(Motif::RiftOut);
        }
        let mut hazard_run = 0;
        while c.x() < end {
            if c.allowed(Motif::RiftBack) && rng.f32() < 0.25 {
                c.push(Motif::RiftBack);
                continue;
            }
            let p = (c.x() - start) as f32 / (end - start).max(1) as f32;
            let tension = (0.35 + 0.1 * target as f32) * (1.0 - 0.6 * p);
            let hazard = hazard_run < max_run && rng.f32() < tension;
            let Some(m) = pick(&c, &mut rng, theme, target, hazard) else { break };
            c.push(m);
            hazard_run = if m.is_hazard() { hazard_run + 1 } else { 0 };
        }
        if c.allowed(Motif::Checkpoint) {
            c.push(Motif::Checkpoint);
        }
    }

    // Ending: close the excursion, make sure there was one, reach the length.
    while !c.can_end() {
        if c.allowed(Motif::RiftBack) {
            c.push(Motif::RiftBack);
        } else if c.excursions() == 0 && !c.rift_open() && c.allowed(Motif::RiftOut) {
            c.push(Motif::RiftOut);
        } else if !c.push(Motif::Flat(3)) {
            break; // unreachable: the grammar always keeps flat3 or a rift legal
        }
    }
    if c.allowed(Motif::StairsUp) && rng.f32() < 0.3 {
        c.push(Motif::StairsUp);
    }
    c.motifs
}
