//! Motif sequence → tile grid → `LevelData` / Tiled JSON.
//!
//! Each motif knows its footprint and keeps the ground continuous, so a
//! sequence is locally playable by construction. `Compiler::allowed` and
//! `can_end` are the grammar shared by the model's sampling mask, the teacher
//! and the mutations. It guarantees a *legal and fair* level — ground height
//! bounds, cave ceiling, intro zone, checkpoint spacing, 1–2 paired rift
//! excursions with their runways, one pair of boots, body length — and that
//! a legal ending always stays reachable:
//!
//! `allowed(m)` requires `x + width(m) + reserve ≤ MAX_BODY`, the reserve
//! being what closing legally still needs (38 columns before any excursion,
//! `wait + 2 + 8` while one is open and must last `wait` more columns, 8 when
//! it may close now, 0 afterwards). `flat3` is calm, intro-safe, fits under
//! any ceiling (a cave keeps its ground top ≥ 9: every rise needs jump
//! clearance, `stand + 4` rows, below the roof) and preserves that invariant, so some of `flat3`, `rift_out`,
//! `rift_back` or "end" is always legal: no sequence of allowed motifs can
//! dead-end (fuzzed in tests/levelgen_compile.rs).

use serde_json::{json, Value};

use super::motif::{Motif, Theme, ALL};
use super::rng::Rng;
use crate::world::level::{LevelData, SpawnPoint, Spawns};

pub const H: usize = 15;
const TS: f32 = 16.0;
/// Default ground top row.
pub const BASE: i32 = 12;
/// Highest allowed ground top row.
pub const MIN_TOP: i32 = 8;
const SPAWN_RUN: i32 = 6;
/// Goal run appended by `finish`.
pub const FINALE_RUN: i32 = 12;
/// Absolute columns (spawn run included) the body may reach / must reach.
pub const MAX_BODY: i32 = 210;
pub const MIN_BODY: i32 = 140;
/// The intro zone: 20 columns after the spawn run.
pub const INTRO_END: i32 = SPAWN_RUN + 20;
pub const CHECKPOINT_MIN: i32 = 30;
/// Past this many columns without a checkpoint, only calm motifs or a checkpoint.
pub const CHECKPOINT_MAX: i32 = 70;
/// Columns an excursion lasts at least, after its runway.
pub const EXCURSION_MIN: i32 = 20;
pub const MAX_EXCURSIONS: u32 = 2;
pub const BOOTS_BEFORE: i32 = 100;
const RIFT_W: i32 = 8;
/// Worst overshoot of the 3-column filler while an excursion must go on.
const FILL_SLACK: i32 = 2;
/// Free rows a jump needs above its take-off surface (full jump ≈ 4.3 tiles).
const JUMP_CLEARANCE: i32 = 4;
/// Ground columns before every pit: the run-up a wide jump needs.
const RUN_UP: i32 = 2;

// Tileset ids (1-based, local), mirroring tools/gen_level.py.
const GROUND_TL: u16 = 1;
const GROUND_T: u16 = 2;
const GROUND_TR: u16 = 3;
const GROUND_L: u16 = 4;
const GROUND_F: u16 = 5;
const GROUND_R: u16 = 6;
const BRICK: u16 = 7;
const STONE: u16 = 8;
const PLANK_L: u16 = 9;
const PLANK_M: u16 = 10;
const PLANK_R: u16 = 11;
const GEM: u16 = 12;
const BUSH_L: u16 = 13;
const BUSH_R: u16 = 14;
const FLOWER: u16 = 15;
const TUFT: u16 = 16;
const FENCE: u16 = 17;
const SIGN: u16 = 18;
const ROCK: u16 = 19;
const CAVE: u16 = 20;
const GROUND_PILLAR_TOP: u16 = 21;
const GROUND_PILLAR: u16 = 22;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Cell {
    Empty,
    Ground,
    Brick,
    Stone,
    Plank,
    Gem,
}

#[derive(Clone, Debug)]
struct Obj {
    kind: &'static str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

/// Columns a motif occupies (`x1` exclusive).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Span {
    pub motif: Motif,
    pub x0: i32,
    pub x1: i32,
}

/// Columns a motif adds.
pub fn width(m: Motif) -> i32 {
    match m {
        Motif::Flat(n) => n as i32,
        Motif::Rest => 5,
        Motif::Gap(n) | Motif::GapPlank(n) => RUN_UP + n as i32 + 3,
        Motif::GapPillar => RUN_UP + 11,
        Motif::Marsh => RUN_UP + 15,
        Motif::Climb => 13,
        Motif::StepUp(_) | Motif::StepDown(_) => 4,
        Motif::Pyramid(n) => 2 * n as i32 + 5,
        Motif::StairsUp => 8,
        Motif::Wall => 7,
        Motif::Walkers(k) => 6 + 3 * k as i32,
        Motif::Goat | Motif::GemEnemy | Motif::CrawlBricks => 9,
        Motif::Stash | Motif::PlankCoins | Motif::Cows => 8,
        Motif::CoinRow => 10,
        Motif::Tunnel(n) => n as i32 + 5,
        Motif::Alcove => 7,
        Motif::Checkpoint | Motif::Boots => 5,
        Motif::RiftOut | Motif::RiftBack => RIFT_W,
    }
}

/// Rows above the current ground top a motif uses (solids, objects, coins):
/// it must stay below the cave ceiling.
pub fn headroom(m: Motif) -> i32 {
    match m {
        Motif::Flat(_) | Motif::Rest | Motif::Checkpoint | Motif::Boots | Motif::Cows | Motif::CoinRow => 2,
        Motif::Goat | Motif::Tunnel(_) => 2,
        Motif::StepDown(_) => 1,
        Motif::StepUp(n) => n as i32 + 1,
        Motif::Gap(_) | Motif::Wall | Motif::Stash => 4,
        Motif::GapPlank(_)
        | Motif::Walkers(_)
        | Motif::CrawlBricks
        | Motif::Alcove
        | Motif::RiftOut
        | Motif::RiftBack => 3,
        Motif::GapPillar | Motif::Marsh | Motif::Climb | Motif::GemEnemy => 6,
        Motif::Pyramid(n) => n as i32 + 3,
        Motif::StairsUp | Motif::PlankCoins => 5,
    }
}

/// Rows above the ground top of the highest surface a motif makes the player
/// jump from or over (cave jump clearance).
pub fn stand(m: Motif) -> i32 {
    match m {
        Motif::Pyramid(n) | Motif::StepUp(n) => n as i32,
        Motif::StairsUp => 5,
        Motif::Climb => 4,
        Motif::Alcove => 3,
        Motif::Wall | Motif::Stash | Motif::Marsh | Motif::GapPlank(_) | Motif::GapPillar => 2,
        // A long jump needs its whole arc: wider gaps, more clearance.
        Motif::Gap(n) => (n as i32 - 2).max(0),
        _ => 0,
    }
}

/// Incremental compiler: feed motifs left to right, then `finish`.
#[derive(Clone, Debug)]
pub struct Compiler {
    theme: Theme,
    cols: Vec<[Cell; H]>,
    deco: Vec<[u16; H]>,
    coins: Vec<[bool; H]>,
    objects: Vec<Obj>,
    /// Ground top row at the cursor.
    pub top: i32,
    boots_used: bool,
    last_checkpoint: i32,
    excursions: u32,
    /// Column where the open excursion's runway ends.
    open_rift: Option<i32>,
    pub motifs: Vec<Motif>,
    spans: Vec<Span>,
}

impl Compiler {
    pub fn new(theme: Theme) -> Self {
        let mut c = Compiler {
            theme,
            cols: Vec::new(),
            deco: Vec::new(),
            coins: Vec::new(),
            objects: Vec::new(),
            top: BASE,
            boots_used: false,
            last_checkpoint: 0,
            excursions: 0,
            open_rift: None,
            motifs: Vec::new(),
            spans: Vec::new(),
        };
        c.ground(SPAWN_RUN);
        c.bushes(0);
        c.obj("player_spawn", 3, c.top, 16.0, 16.0);
        c
    }

    pub fn theme(&self) -> Theme {
        self.theme
    }

    /// Cursor column (absolute, spawn run included).
    pub fn x(&self) -> i32 {
        self.cols.len() as i32
    }

    pub fn excursions(&self) -> u32 {
        self.excursions
    }

    pub fn rift_open(&self) -> bool {
        self.open_rift.is_some()
    }

    /// Grammar: may this motif come next?
    pub fn allowed(&self, m: Motif) -> bool {
        let x = self.x();
        let new_x = x + width(m);
        if new_x + self.reserve_after(m, new_x) > MAX_BODY {
            return false;
        }
        let roof = self.theme.roof_rows();
        if self.top - headroom(m) < roof {
            return false;
        }
        if roof > 0 && self.top - stand(m) - JUMP_CLEARANCE < roof {
            return false;
        }
        if x < INTRO_END && !m.intro_ok() {
            return false;
        }
        let since_cp = x - self.last_checkpoint;
        if since_cp > CHECKPOINT_MAX && m.is_hazard() {
            return false;
        }
        match m {
            Motif::StepUp(n) => self.top - n as i32 >= MIN_TOP,
            Motif::Climb => self.top - 3 >= MIN_TOP,
            Motif::StepDown(n) => self.top + n as i32 <= BASE,
            Motif::Boots => !self.boots_used && x < BOOTS_BEFORE,
            Motif::Checkpoint => since_cp >= CHECKPOINT_MIN,
            Motif::RiftOut => self.open_rift.is_none() && self.excursions < MAX_EXCURSIONS,
            Motif::RiftBack => matches!(self.open_rift, Some(end) if x - end >= EXCURSION_MIN),
            _ => true,
        }
    }

    /// Columns that must stay free after placing `m` (ending at `new_x`) so
    /// the level can still close legally (see the module docs).
    fn reserve_after(&self, m: Motif, new_x: i32) -> i32 {
        let (open, done) = match m {
            Motif::RiftOut => (Some(new_x), self.excursions),
            Motif::RiftBack => (None, self.excursions + 1),
            _ => (self.open_rift, self.excursions),
        };
        match open {
            Some(end) => {
                let wait = (EXCURSION_MIN - (new_x - end)).max(0);
                if wait > 0 {
                    wait + FILL_SLACK + RIFT_W
                } else {
                    RIFT_W
                }
            }
            None if done == 0 => RIFT_W + EXCURSION_MIN + FILL_SLACK + RIFT_W,
            None => 0,
        }
    }

    /// Grammar: may the level end here?
    pub fn can_end(&self) -> bool {
        self.x() >= MIN_BODY && self.open_rift.is_none() && self.excursions >= 1
    }

    /// Append a motif. A disallowed motif is refused (returns false) so no
    /// caller can corrupt the level.
    pub fn push(&mut self, m: Motif) -> bool {
        if !self.allowed(m) {
            return false;
        }
        let x = self.x();
        let t = self.top;
        let bottom = H as i32 - 1;
        match m {
            Motif::Flat(n) => {
                self.ground(n as i32);
                self.small_deco(x + 1);
            }
            Motif::Rest => {
                self.ground(5);
                self.bushes(x + 1);
                self.small_deco(x + 4);
            }
            Motif::Gap(n) => {
                let n = n as i32;
                self.ground(RUN_UP);
                let x = x + RUN_UP;
                self.pit(n);
                self.coin_arc(x, x + n - 1, t - 4, 2);
                self.ground(3);
            }
            Motif::GapPlank(n) => {
                let n = n as i32;
                self.ground(RUN_UP);
                let x = x + RUN_UP;
                self.pit(n);
                let p0 = x + (n - 3) / 2;
                for px in p0..p0 + 3 {
                    self.set(px, t - 1, Cell::Plank);
                    self.coin(px, t - 3);
                }
                self.ground(3);
            }
            Motif::GapPillar => {
                self.ground(RUN_UP);
                let x = x + RUN_UP;
                self.pit(8);
                self.column(x + 3, t, bottom, Cell::Stone);
                self.column(x + 4, t, bottom, Cell::Stone);
                self.coin_arc(x, x + 7, t - 6, 2);
                self.ground(3);
            }
            Motif::Marsh => {
                self.ground(RUN_UP);
                let x = x + RUN_UP;
                self.pit(12);
                for px in [x + 2, x + 3] {
                    self.column(px, t, bottom, Cell::Stone);
                }
                for px in [x + 6, x + 7] {
                    self.column(px, t - 1, bottom, Cell::Stone);
                }
                for px in [x + 9, x + 10] {
                    self.set(px, t - 2, Cell::Plank);
                }
                self.coin_arc(x, x + 11, t - 6, 2);
                self.ground(3);
            }
            Motif::Climb => {
                self.pit(10);
                for px in x + 1..x + 4 {
                    self.set(px, t - 2, Cell::Plank);
                }
                for px in x + 5..x + 8 {
                    self.set(px, t - 4, Cell::Plank);
                    self.coin(px, t - 6);
                }
                self.top -= 3;
                self.ground(3);
            }
            Motif::StepUp(n) => {
                self.ground(1);
                self.top -= n as i32;
                self.ground(3);
            }
            Motif::StepDown(n) => {
                self.ground(1);
                self.top += n as i32;
                self.ground(3);
            }
            Motif::Pyramid(n) => {
                let n = n as i32;
                self.ground(1);
                let s0 = self.x();
                self.ground(2 * n + 2);
                for i in 0..n {
                    self.column(s0 + i, t - (i + 1), t - 1, Cell::Stone);
                    self.column(s0 + n + 2 + i, t - (n - i), t - 1, Cell::Stone);
                }
                for px in s0 + n..s0 + n + 2 {
                    for y in self.theme.roof_rows()..H as i32 {
                        self.set(px, y, Cell::Empty);
                    }
                }
                self.coin_arc(s0 + n - 1, s0 + n + 2, t - n - 3, 1);
                self.ground(2);
            }
            Motif::StairsUp => {
                self.ground(8);
                for i in 0..5 {
                    self.column(x + 1 + i, t - (i + 1), t - 1, Cell::Stone);
                }
            }
            Motif::Wall => {
                self.ground(7);
                let k = self.wall_cell();
                self.column(x + 2, t - 2, t - 1, k);
                self.column(x + 3, t - 2, t - 1, k);
                self.coin(x + 2, t - 4);
                self.coin(x + 3, t - 4);
            }
            Motif::Walkers(k) => {
                let k = k as i32;
                self.ground(6 + 3 * k);
                // First marmot 5 columns in: room to land after a step or a gap.
                for i in 0..k {
                    self.obj("enemy_spawn", x + 5 + 3 * i, t, 16.0, 16.0);
                }
                for px in x + 2..x + 4 + 3 * k {
                    self.coin(px, t - 3);
                }
            }
            Motif::Goat => {
                self.ground(9);
                self.obj("goat_spawn", x + 6, t, 16.0, 16.0);
                self.decorate(FENCE, x + 1);
            }
            Motif::GemEnemy => {
                self.ground(9);
                for px in x + 3..x + 7 {
                    self.set(px, t - 4, Cell::Gem);
                    self.coin(px, t - 6);
                }
                self.obj("enemy_spawn", x + 5, t - 4, 16.0, 16.0);
            }
            Motif::Stash => {
                self.ground(8);
                let k = self.wall_cell();
                self.column(x + 2, t - 2, t - 1, k);
                self.column(x + 3, t - 2, t - 1, k);
                for px in [x + 4, x + 5] {
                    self.coin(px, t - 4);
                    self.coin(px, t - 3);
                }
                self.obj("enemy_spawn", x + 6, t, 16.0, 16.0);
            }
            Motif::PlankCoins => {
                self.ground(8);
                for px in x + 2..x + 6 {
                    self.set(px, t - 3, Cell::Plank);
                    self.coin(px, t - 5);
                }
            }
            Motif::CoinRow => {
                self.ground(10);
                for px in x + 1..x + 9 {
                    self.coin(px, t - 2);
                }
            }
            Motif::Tunnel(n) => {
                let n = n as i32;
                self.ground(n + 5);
                self.decorate(SIGN, x + 1);
                let roof = self.theme.roof_rows();
                for px in x + 3..x + 3 + n {
                    self.column(px, roof, t - 2, Cell::Brick);
                    self.deco_at(px, t - 1, CAVE);
                    if px > x + 3 && px < x + 2 + n {
                        self.coin(px, t - 1);
                    }
                }
            }
            Motif::CrawlBricks => {
                self.ground(9);
                for px in x + 2..x + 7 {
                    self.column(px, t - 3, t - 2, Cell::Brick);
                    self.deco_at(px, t - 1, CAVE);
                    self.coin(px, t - 1);
                }
            }
            Motif::Alcove => {
                self.ground(7);
                self.column(x + 1, t - 3, t - 1, Cell::Ground);
                self.column(x + 4, t - 3, t - 1, Cell::Ground);
                for px in x + 2..x + 4 {
                    for y in t - 3..t {
                        self.coin(px, y);
                    }
                }
            }
            Motif::Checkpoint => {
                self.ground(5);
                self.obj("checkpoint", x + 2, t, 16.0, 16.0);
                self.last_checkpoint = x;
            }
            Motif::Boots => {
                self.ground(5);
                self.decorate(SIGN, x + 1);
                self.obj("dash_boots", x + 3, t, 16.0, 16.0);
                self.boots_used = true;
            }
            Motif::Cows => {
                self.ground(8);
                self.obj("cow", x + 2, t, 16.0, 16.0);
                self.obj("cow", x + 6, t, 16.0, 16.0);
                self.decorate(ROCK, x + 4);
            }
            Motif::RiftOut | Motif::RiftBack => {
                self.ground(RIFT_W);
                self.obj("rift", x, t, 16.0, 16.0);
                if m == Motif::RiftOut {
                    self.open_rift = Some(x + RIFT_W);
                } else {
                    self.open_rift = None;
                    self.excursions += 1;
                }
            }
        }
        debug_assert_eq!(self.x() - x, width(m), "{m:?}: footprint != width");
        self.motifs.push(m);
        self.spans.push(Span { motif: m, x0: x, x1: self.x() });
        true
    }

    /// Close the level with the goal run.
    pub fn finish(mut self) -> CompiledLevel {
        let x = self.x();
        self.ground(FINALE_RUN);
        self.obj("goal", x + 6, self.top, 16.0, 48.0);
        self.bushes(x + 9);
        CompiledLevel::from_compiler(self)
    }

    // --- primitives --------------------------------------------------------

    /// An empty column: open sky, or the cave roof.
    fn blank(&self) -> [Cell; H] {
        let mut col = [Cell::Empty; H];
        for c in col.iter_mut().take(self.theme.roof_rows() as usize) {
            *c = Cell::Ground;
        }
        col
    }

    fn ground(&mut self, n: i32) {
        for _ in 0..n {
            let mut col = self.blank();
            for c in col.iter_mut().skip(self.top as usize) {
                *c = Cell::Ground;
            }
            self.cols.push(col);
            self.deco.push([0; H]);
            self.coins.push([false; H]);
        }
    }

    fn pit(&mut self, n: i32) {
        for _ in 0..n {
            let col = self.blank();
            self.cols.push(col);
            self.deco.push([0; H]);
            self.coins.push([false; H]);
        }
    }

    fn wall_cell(&self) -> Cell {
        if self.theme == Theme::Cave {
            Cell::Stone
        } else {
            Cell::Brick
        }
    }

    fn set(&mut self, x: i32, y: i32, c: Cell) {
        if let Some(col) = self.cols.get_mut(x as usize) {
            if (0..H as i32).contains(&y) {
                col[y as usize] = c;
            }
        }
    }

    fn column(&mut self, x: i32, y0: i32, y1: i32, c: Cell) {
        for y in y0..=y1 {
            self.set(x, y, c);
        }
    }

    fn coin(&mut self, x: i32, y: i32) {
        if let Some(col) = self.coins.get_mut(x as usize) {
            if (0..H as i32).contains(&y) {
                col[y as usize] = true;
            }
        }
    }

    fn coin_arc(&mut self, x0: i32, x1: i32, y_top: i32, height: i32) {
        let n = (x1 - x0).max(1) as f32;
        for x in x0..=x1 {
            let t = ((x - x0) as f32 / n) * 2.0 - 1.0;
            self.coin(x, y_top + (height as f32 * t * t).round() as i32);
        }
    }

    fn deco_at(&mut self, x: i32, y: i32, tile: u16) {
        if let Some(col) = self.deco.get_mut(x as usize) {
            if (0..H as i32).contains(&y) {
                col[y as usize] = tile;
            }
        }
    }

    /// Decoration standing on the surface of column x.
    fn decorate(&mut self, tile: u16, x: i32) {
        if let Some(y) = self.surface(x) {
            self.deco_at(x, y - 1, tile);
        }
    }

    fn surface(&self, x: i32) -> Option<i32> {
        let col = self.cols.get(x as usize)?;
        let roof = self.theme.roof_rows() as usize;
        col.iter().skip(roof).position(|c| *c != Cell::Empty).map(|y| (y + roof) as i32)
    }

    fn bushes(&mut self, x: i32) {
        if self.theme == Theme::Cave {
            self.decorate(ROCK, x + 1);
        } else {
            self.decorate(BUSH_L, x);
            self.decorate(BUSH_R, x + 1);
        }
    }

    fn small_deco(&mut self, x: i32) {
        let tile = match self.theme {
            Theme::Cave if x % 4 == 0 => ROCK,
            Theme::Cave => return,
            Theme::Night => TUFT,
            _ if x % 3 == 0 => TUFT,
            _ => FLOWER,
        };
        self.decorate(tile, x);
    }

    /// Object whose bottom-center sits on top of row `feet_row` at column x.
    fn obj(&mut self, kind: &'static str, x: i32, feet_row: i32, w: f32, h: f32) {
        self.objects.push(Obj {
            kind,
            x: x as f32 * TS + TS / 2.0 - w / 2.0,
            y: feet_row as f32 * TS - h,
            w,
            h,
        });
    }
}

/// Compile a sequence only if every motif is legal where it stands and the
/// level may end there (transcriptions, mutations, validation).
pub fn compile_strict(theme: Theme, motifs: &[Motif]) -> Option<CompiledLevel> {
    let mut c = Compiler::new(theme);
    for &m in motifs {
        if !c.push(m) {
            return None;
        }
    }
    c.can_end().then(|| c.finish())
}

/// A uniform random walk over legal motifs, stopping at a legal ending: the
/// widest possible exploration of the grammar (tests, diagnostics).
pub fn random_legal(theme: Theme, seed: u64) -> Compiler {
    let mut rng = Rng::new(seed);
    let mut c = Compiler::new(theme);
    loop {
        let legal: Vec<Motif> = ALL.iter().copied().filter(|&m| c.allowed(m)).collect();
        if legal.is_empty() || (c.can_end() && rng.f32() < 0.15) {
            return c;
        }
        c.push(legal[rng.below(legal.len())]);
    }
}

/// A compiled level: what the bots play and what the host loads.
#[derive(Clone, Debug)]
pub struct CompiledLevel {
    pub width: u32,
    pub theme: Theme,
    pub terrain: Vec<u16>,
    pub collision: Vec<u8>,
    pub coins: Vec<u8>,
    objects: Vec<Obj>,
    pub motifs: Vec<Motif>,
    pub spans: Vec<Span>,
}

impl CompiledLevel {
    fn from_compiler(c: Compiler) -> Self {
        let w = c.cols.len();
        let kind = |x: i32, y: i32| -> Cell {
            if x < 0 || x >= w as i32 || y >= H as i32 {
                return Cell::Ground; // map edges continue the ground
            }
            if y < 0 {
                return Cell::Empty;
            }
            c.cols[x as usize][y as usize]
        };
        let mut terrain = vec![0u16; w * H];
        let mut collision = vec![0u8; w * H];
        let mut coins = vec![0u8; w * H];
        for y in 0..H as i32 {
            for x in 0..w as i32 {
                let i = y as usize * w + x as usize;
                let k = kind(x, y);
                if k != Cell::Empty {
                    collision[i] = 1;
                    terrain[i] = autotile(k, |dx, dy| kind(x + dx, y + dy));
                } else {
                    terrain[i] = c.deco[x as usize][y as usize];
                    coins[i] = c.coins[x as usize][y as usize] as u8;
                }
            }
        }
        CompiledLevel {
            width: w as u32,
            theme: c.theme,
            terrain,
            collision,
            coins,
            objects: c.objects,
            motifs: c.motifs,
            spans: c.spans,
        }
    }

    /// The motif span covering tile column `tile` (diagnostics).
    pub fn span_at_tile(&self, tile: i32) -> Option<&Span> {
        self.spans.iter().find(|s| (s.x0..s.x1).contains(&tile))
    }

    fn feet_of(&self, kind: &str) -> Vec<SpawnPoint> {
        self.objects
            .iter()
            .filter(|o| o.kind == kind)
            .map(|o| SpawnPoint { x: o.x + o.w * 0.5, y: o.y + o.h })
            .collect()
    }

    pub fn level_data(&self) -> LevelData {
        LevelData {
            width: self.width,
            height: H as u32,
            tile_size: TS,
            collision: self.collision.clone(),
            coins: self.coins.clone(),
            terrain: self.terrain.clone(),
            tileset_columns: 8,
            spawns: Spawns {
                player: self.feet_of("player_spawn")[0],
                goal: self.feet_of("goal")[0],
                enemies: self.feet_of("enemy_spawn"),
                goats: self.feet_of("goat_spawn"),
                cows: self.feet_of("cow"),
                checkpoints: self.feet_of("checkpoint"),
                dash_boots: self.feet_of("dash_boots"),
                rifts: self.feet_of("rift"),
            },
        }
    }

    /// Regular Tiled map, same conventions as tools/gen_level.py, so the host
    /// loads a generated level exactly like a shipped one.
    pub fn to_tiled_json(&self) -> String {
        let layer = |id: u32, name: &str, data: Vec<u32>, visible: bool| -> Value {
            json!({"id": id, "name": name, "type": "tilelayer", "x": 0, "y": 0, "width": self.width,
                   "height": H, "opacity": 1.0, "visible": visible, "data": data})
        };
        let objects: Vec<Value> = self
            .objects
            .iter()
            .enumerate()
            .map(|(i, o)| {
                json!({"id": i + 1, "name": o.kind, "type": o.kind, "x": o.x, "y": o.y,
                       "width": o.w, "height": o.h, "rotation": 0, "visible": true})
            })
            .collect();
        json!({
            "type": "map", "orientation": "orthogonal", "renderorder": "right-down", "infinite": false,
            "width": self.width, "height": H, "tilewidth": 16, "tileheight": 16,
            "backgroundcolor": self.theme.background(),
            "tilesets": [{"firstgid": 1, "name": "tiles", "image": "../assets/tiles.png",
                          "imagewidth": 128, "imageheight": 48, "tilewidth": 16, "tileheight": 16,
                          "tilecount": 24, "columns": 8, "margin": 0, "spacing": 0}],
            "layers": [
                layer(1, "terrain", self.terrain.iter().map(|&t| t as u32).collect(), true),
                layer(2, "collision", self.collision.iter().map(|&c| if c > 0 { 24 } else { 0 }).collect(), false),
                layer(3, "coins", self.coins.iter().map(|&c| if c > 0 { 23 } else { 0 }).collect(), true),
                {"id": 4, "name": "objects", "type": "objectgroup", "objects": objects},
            ],
        })
        .to_string()
    }

    /// ASCII view for reports and debugging.
    pub fn ascii(&self) -> String {
        let w = self.width as usize;
        let mut rows: Vec<Vec<char>> = (0..H)
            .map(|y| {
                (0..w)
                    .map(|x| {
                        let i = y * w + x;
                        match (self.collision[i], self.terrain[i], self.coins[i]) {
                            (1, BRICK, _) => 'b',
                            (1, STONE, _) => 's',
                            (1, PLANK_L | PLANK_M | PLANK_R, _) => '=',
                            (1, GEM, _) => 'g',
                            (1, _, _) => '#',
                            (_, _, 1) => 'o',
                            _ => '.',
                        }
                    })
                    .collect()
            })
            .collect();
        for o in &self.objects {
            let cx = ((o.x + o.w / 2.0) / TS) as usize;
            let cy = ((o.y + o.h - 1.0) / TS) as usize;
            let ch = match o.kind {
                "player_spawn" => 'P',
                "enemy_spawn" => 'E',
                "goat_spawn" => 'G',
                "goal" => 'F',
                "checkpoint" => 'C',
                "dash_boots" => 'B',
                "cow" => 'w',
                "rift" => 'R',
                _ => '?',
            };
            if cy < H && cx < w {
                rows[cy][cx] = ch;
            }
        }
        rows.into_iter().map(|r| r.into_iter().collect::<String>()).collect::<Vec<_>>().join("\n")
    }
}

fn autotile(k: Cell, at: impl Fn(i32, i32) -> Cell) -> u16 {
    match k {
        Cell::Brick => return BRICK,
        Cell::Stone => return STONE,
        Cell::Gem => return GEM,
        Cell::Plank => {
            let (l, r) = (at(-1, 0) == Cell::Plank, at(1, 0) == Cell::Plank);
            return if l && r {
                PLANK_M
            } else if l {
                PLANK_R
            } else {
                PLANK_L
            };
        }
        _ => {}
    }
    let g = |dx, dy| at(dx, dy) == Cell::Ground;
    match (!g(0, -1), !g(-1, 0), !g(1, 0)) {
        (true, true, true) => GROUND_PILLAR_TOP,
        (true, true, false) => GROUND_TL,
        (true, false, true) => GROUND_TR,
        (true, false, false) => GROUND_T,
        (false, true, true) => GROUND_PILLAR,
        (false, true, false) => GROUND_L,
        (false, false, true) => GROUND_R,
        (false, false, false) => GROUND_F,
    }
}
