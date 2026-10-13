//! The level description language (v2). A level is `<theme> <difficulty>`
//! followed by design motifs (Launchpad rhythm groups / Dahlskog–Togelius
//! patterns), one token each. Parameters are baked into the variant so the
//! vocabulary stays small and flat. Geometry lives in `compile.rs`.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Theme {
    Meadow,
    Sunset,
    Cave,
    Night,
}

impl Theme {
    /// Order = campaign order = music track id = host theme index.
    pub const ALL: [Theme; 4] = [Theme::Meadow, Theme::Sunset, Theme::Cave, Theme::Night];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn name(self) -> &'static str {
        ["meadow", "sunset", "cave", "night"][self as usize]
    }

    pub fn parse(s: &str) -> Option<Theme> {
        Theme::ALL.into_iter().find(|t| t.name() == s)
    }

    /// Tiled background colour, as in tools/gen_level.py.
    pub fn background(self) -> &'static str {
        ["#5aa0f0", "#e8734a", "#101433", "#0a1030"][self as usize]
    }

    /// Rows `0..roof_rows()` are solid rock (cave ceiling); 0 = open sky.
    pub fn roof_rows(self) -> i32 {
        if self == Theme::Cave {
            5
        } else {
            0
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Motif {
    /// Plain ground run (n columns).
    Flat(u8),
    /// Calm stretch with bushes: a rhythm-group separator.
    Rest,
    /// Pit of n columns with a coin arc.
    Gap(u8),
    /// Wide pit (n columns) with a 3-wide stepping plank.
    GapPlank(u8),
    /// 8-wide pit with a stone pillar.
    GapPillar,
    /// 12-wide valley: pillar, higher pillar, plank (levels 3/4).
    Marsh,
    /// 10-wide pit, rising planks onto ground 3 rows higher (level 1).
    Climb,
    /// Raise the ground by n rows.
    StepUp(u8),
    /// Lower the ground by n rows.
    StepDown(u8),
    /// Stone stairs up (n), 2-wide pit, stairs down.
    Pyramid(u8),
    /// Final-style 5-step staircase.
    StairsUp,
    /// 2-high wall with coins on top.
    Wall,
    /// n marmots patrolling a flat run.
    Walkers(u8),
    /// A cheese-wheel goat.
    Goat,
    /// Floating gem platform with a marmot on it.
    GemEnemy,
    /// Hop-wall with a coin stash behind, guarded by a marmot (level 3).
    Stash,
    /// Floating plank with coins.
    PlankCoins,
    /// Coin shower over a corridor (level 2).
    CoinRow,
    /// Mandatory crawl under a wall to the sky / cave roof (n columns).
    Tunnel(u8),
    /// Brick block with an optional coin crawl space under it.
    CrawlBricks,
    /// Coin hideout between two pillars.
    Alcove,
    Checkpoint,
    Boots,
    Cows,
    /// Into the future (with its flat, enemy-free runway).
    RiftOut,
    /// Back to the past (with its runway).
    RiftBack,
}

/// Every motif, in token order (the model's vocabulary depends on this order).
pub const ALL: &[Motif] = &[
    Motif::Flat(3),
    Motif::Flat(5),
    Motif::Flat(8),
    Motif::Rest,
    Motif::Gap(2),
    Motif::Gap(3),
    Motif::Gap(4),
    Motif::Gap(5),
    Motif::GapPlank(7),
    Motif::GapPlank(9),
    Motif::GapPillar,
    Motif::Marsh,
    Motif::Climb,
    Motif::StepUp(1),
    Motif::StepUp(2),
    Motif::StepUp(3),
    Motif::StepDown(1),
    Motif::StepDown(2),
    Motif::StepDown(3),
    Motif::Pyramid(3),
    Motif::Pyramid(4),
    Motif::StairsUp,
    Motif::Wall,
    Motif::Walkers(1),
    Motif::Walkers(2),
    Motif::Walkers(3),
    Motif::Goat,
    Motif::GemEnemy,
    Motif::Stash,
    Motif::PlankCoins,
    Motif::CoinRow,
    Motif::Tunnel(5),
    Motif::Tunnel(7),
    Motif::CrawlBricks,
    Motif::Alcove,
    Motif::Checkpoint,
    Motif::Boots,
    Motif::Cows,
    Motif::RiftOut,
    Motif::RiftBack,
];

/// Mutation classes: a motif may be swapped for another of its category.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Calm,
    Gap,
    WideGap,
    Enemy,
    Rise,
    Fall,
    Stairs,
    Tunnel,
    /// Structure the mutations never touch.
    Fixed,
}

impl Motif {
    pub fn name(self) -> String {
        match self {
            Motif::Flat(n) => format!("flat{n}"),
            Motif::Rest => "rest".into(),
            Motif::Gap(n) => format!("gap{n}"),
            Motif::GapPlank(n) => format!("gapplank{n}"),
            Motif::GapPillar => "gappillar".into(),
            Motif::Marsh => "marsh".into(),
            Motif::Climb => "climb".into(),
            Motif::StepUp(n) => format!("up{n}"),
            Motif::StepDown(n) => format!("down{n}"),
            Motif::Pyramid(n) => format!("pyramid{n}"),
            Motif::StairsUp => "stairs".into(),
            Motif::Wall => "wall".into(),
            Motif::Walkers(n) => format!("walk{n}"),
            Motif::Goat => "goat".into(),
            Motif::GemEnemy => "gem".into(),
            Motif::Stash => "stash".into(),
            Motif::PlankCoins => "plank".into(),
            Motif::CoinRow => "coinrow".into(),
            Motif::Tunnel(n) => format!("tunnel{n}"),
            Motif::CrawlBricks => "crawl".into(),
            Motif::Alcove => "alcove".into(),
            Motif::Checkpoint => "checkpoint".into(),
            Motif::Boots => "boots".into(),
            Motif::Cows => "cows".into(),
            Motif::RiftOut => "rift_out".into(),
            Motif::RiftBack => "rift_back".into(),
        }
    }

    pub fn parse(s: &str) -> Option<Motif> {
        ALL.iter().copied().find(|m| m.name() == s)
    }

    pub fn index(self) -> usize {
        ALL.iter().position(|m| *m == self).expect("motif in ALL")
    }

    pub fn category(self) -> Category {
        match self {
            Motif::Flat(_)
            | Motif::Rest
            | Motif::Cows
            | Motif::CoinRow
            | Motif::PlankCoins
            | Motif::Alcove
            | Motif::CrawlBricks
            | Motif::Wall => Category::Calm,
            Motif::Gap(_) => Category::Gap,
            Motif::GapPlank(_) | Motif::GapPillar | Motif::Marsh | Motif::Climb => Category::WideGap,
            Motif::Walkers(_) | Motif::Goat | Motif::GemEnemy | Motif::Stash => Category::Enemy,
            Motif::StepUp(_) => Category::Rise,
            Motif::StepDown(_) => Category::Fall,
            Motif::Pyramid(_) | Motif::StairsUp => Category::Stairs,
            Motif::Tunnel(_) => Category::Tunnel,
            Motif::Checkpoint | Motif::Boots | Motif::RiftOut | Motif::RiftBack => Category::Fixed,
        }
    }

    pub fn has_enemy(self) -> bool {
        self.category() == Category::Enemy
    }

    /// Something the player has to act on (jump, time, crawl).
    pub fn is_hazard(self) -> bool {
        matches!(self.category(), Category::Gap | Category::WideGap | Category::Enemy | Category::Tunnel)
            || matches!(self, Motif::Pyramid(_))
    }

    /// Allowed in the intro zone: no enemy, no gap wider than 3, no tunnel, no rift.
    pub fn intro_ok(self) -> bool {
        !self.has_enemy()
            && !matches!(
                self,
                Motif::Gap(4 | 5)
                    | Motif::GapPlank(_)
                    | Motif::GapPillar
                    | Motif::Marsh
                    | Motif::Climb
                    | Motif::Tunnel(_)
                    | Motif::RiftOut
                    | Motif::RiftBack
            )
    }
}

/// Token vocabulary shared with the model: specials, theme tags, difficulty
/// tags, motifs. Retrain the model whenever this changes.
pub mod vocab {
    use super::{Motif, Theme, ALL};

    pub const PAD: u32 = 0;
    pub const EOS: u32 = 1;
    pub const THEME0: u32 = 2;
    pub const DIFF0: u32 = THEME0 + 4;
    pub const MAX_DIFFICULTY: u8 = 5;
    pub const MOTIF0: u32 = DIFF0 + MAX_DIFFICULTY as u32;

    pub fn size() -> usize {
        MOTIF0 as usize + ALL.len()
    }

    pub fn theme_token(t: Theme) -> u32 {
        THEME0 + t.index() as u32
    }

    pub fn diff_token(d: u8) -> u32 {
        DIFF0 + (d.clamp(1, MAX_DIFFICULTY) - 1) as u32
    }

    pub fn motif_token(m: Motif) -> u32 {
        MOTIF0 + m.index() as u32
    }

    pub fn token_motif(t: u32) -> Option<Motif> {
        t.checked_sub(MOTIF0).and_then(|i| ALL.get(i as usize).copied())
    }

    pub fn token_name(t: u32) -> String {
        match t {
            PAD => "<pad>".into(),
            EOS => "<eos>".into(),
            t if (THEME0..DIFF0).contains(&t) => format!("<{}>", Theme::ALL[(t - THEME0) as usize].name()),
            t if (DIFF0..MOTIF0).contains(&t) => format!("<d{}>", t - DIFF0 + 1),
            t => token_motif(t).map(Motif::name).unwrap_or_else(|| "<unk>".into()),
        }
    }

    pub fn parse_token(s: &str) -> Option<u32> {
        (0..size() as u32).find(|&t| token_name(t) == s)
    }
}
