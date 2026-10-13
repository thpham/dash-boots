//! The campaign, transcribed into the motif language (v2): the style anchor
//! of the corpus (mutations), of the GRPO reward (style score) and the
//! last-resort fallback of an endless run. Each transcription follows its
//! level's sections (comments) as closely as the motifs and the 210-column
//! body allow.

use std::collections::HashSet;
use std::sync::OnceLock;

use super::compile::{compile_strict, CompiledLevel};
use super::motif::{Category, Motif, Theme, ALL};
use super::rng::Rng;

pub struct Transcription {
    pub name: &'static str,
    pub theme: Theme,
    pub text: &'static str,
}

pub const CAMPAIGN: [Transcription; 4] = [
    Transcription {
        name: "level1",
        theme: Theme::Meadow,
        // A tutorial meadow · B planks, plateau, boots, crawl · C pits, rift,
        // stairs, gem, rift back · D pillar gap, marmot parade, slide tunnel.
        text: "flat3 wall flat5 flat5 walk1 gap3 plank up2 walk2 down2 checkpoint gap4 boots crawl walk1 \
               gapplank7 rift_out pyramid4 checkpoint gem rift_back gappillar walk3 tunnel7",
    },
    Transcription {
        name: "level2",
        theme: Theme::Sunset,
        // A gentle start · B plank terrace climb, plateau goat · C stairs,
        // rift, gem · D plank valley, coin-shower corridor, crawl tunnel.
        text: "flat5 wall flat3 flat5 gap3 plank walk1 up3 walk2 checkpoint goat down3 gap2 boots cows pyramid4 \
               rift_out walk1 gem flat5 rift_back checkpoint gapplank7 coinrow walk2 tunnel5",
    },
    Transcription {
        name: "level3",
        theme: Theme::Cave,
        // A entry corridor with cows · B gem terraces · C crawl tunnel, gem
        // shelf · D flooded marsh, rift, the goat pen and stash, stairs.
        text: "cows flat5 wall walk1 gap3 walk1 gapplank7 up2 walk1 checkpoint down2 boots walk1 tunnel7 walk1 \
               gem marsh checkpoint rift_out stash goat pyramid3 rift_back walk1",
    },
    Transcription {
        name: "level4",
        theme: Theme::Night,
        // A moonlit meadow · B terrace with notches · C double crawl guarded by
        // goats, rift, gem, stairs · D stepping-stone marsh, last crawl.
        text: "flat5 wall flat5 flat3 walk1 gap3 gap2 boots up3 gap2 walk2 down3 checkpoint goat tunnel7 goat \
               tunnel7 checkpoint rift_out gem pyramid4 rift_back marsh checkpoint tunnel5",
    },
];

impl Transcription {
    pub fn motifs(&self) -> Vec<Motif> {
        self.text
            .split_whitespace()
            .map(|w| Motif::parse(w).unwrap_or_else(|| panic!("{}: unknown motif `{w}`", self.name)))
            .collect()
    }

    pub fn compile(&self) -> CompiledLevel {
        compile_strict(self.theme, &self.motifs())
            .unwrap_or_else(|| panic!("{}: illegal transcription", self.name))
    }
}

/// The campaign level of a theme (an endless run's last-resort level).
pub fn for_theme(theme: Theme) -> &'static Transcription {
    &CAMPAIGN[theme.index()]
}

/// 1–4 random edits, each one changing the sequence: swap a motif for
/// another of its category, swap two neighbours, or swap two sections between
/// checkpoints that hold no rift. The result may be illegal; callers keep it
/// only if `compile_strict` accepts it.
pub fn mutate(seq: &[Motif], rng: &mut Rng) -> Vec<Motif> {
    let mut s = seq.to_vec();
    let ops = 1 + rng.below(4);
    for _ in 0..ops {
        for _attempt in 0..10 {
            let before = s.clone();
            match rng.below(3) {
                0 => replace_in_category(&mut s, rng),
                1 => swap_neighbours(&mut s, rng),
                _ => swap_sections(&mut s, rng),
            }
            if s != before {
                break;
            }
        }
    }
    s
}

fn replace_in_category(s: &mut [Motif], rng: &mut Rng) {
    let i = rng.below(s.len());
    let cat = s[i].category();
    if cat == Category::Fixed {
        return;
    }
    let pool: Vec<Motif> = ALL.iter().copied().filter(|m| m.category() == cat && *m != s[i]).collect();
    if !pool.is_empty() {
        s[i] = pool[rng.below(pool.len())];
    }
}

fn swap_neighbours(s: &mut [Motif], rng: &mut Rng) {
    let i = rng.below(s.len() - 1);
    if s[i] != s[i + 1] && s[i].category() != Category::Fixed && s[i + 1].category() != Category::Fixed {
        s.swap(i, i + 1);
    }
}

fn swap_sections(s: &mut Vec<Motif>, rng: &mut Rng) {
    let mut sections: Vec<Vec<Motif>> = vec![Vec::new()];
    for &m in s.iter() {
        if m == Motif::Checkpoint {
            sections.push(Vec::new());
        } else {
            sections.last_mut().unwrap().push(m);
        }
    }
    let movable: Vec<usize> = (1..sections.len())
        .filter(|&k| !sections[k].iter().any(|m| matches!(m, Motif::RiftOut | Motif::RiftBack)))
        .collect();
    if movable.len() >= 2 {
        let a = movable[rng.below(movable.len())];
        let b = movable[rng.below(movable.len())];
        sections.swap(a, b);
        *s = sections.join(&Motif::Checkpoint);
    }
}

fn campaign_bigrams() -> &'static HashSet<(Motif, Motif)> {
    static SET: OnceLock<HashSet<(Motif, Motif)>> = OnceLock::new();
    SET.get_or_init(|| {
        CAMPAIGN
            .iter()
            .flat_map(|t| t.motifs().windows(2).map(|w| (w[0], w[1])).collect::<Vec<_>>())
            .collect()
    })
}

/// Share of the sequence's motif bigrams that occur in the campaign (0..=1).
pub fn style_score(seq: &[Motif]) -> f32 {
    if seq.len() < 2 {
        return 0.0;
    }
    let set = campaign_bigrams();
    let hits = seq.windows(2).filter(|w| set.contains(&(w[0], w[1]))).count();
    hits as f32 / (seq.len() - 1) as f32
}
