//! Generated levels (ENDLESS mode). A level is `<theme> <difficulty>` plus a
//! sequence of design motifs (`motif`), compiled to tiles under a grammar that
//! guarantees legal, fair, closable levels (`compile`), validated and rated by
//! graded bots (`bot`). Sequences come from a tiny transformer embedded in the
//! wasm (`model`), trained offline on campaign transcriptions (`handmade`) and
//! a section teacher (`teacher`), then improved by GRPO (`reward`). `run`
//! turns a run seed into an endless, progressively harder sequence of levels.

pub mod bot;
pub mod compile;
pub mod handmade;
pub mod model;
pub mod motif;
pub mod reward;
pub mod rng;
pub mod run;
pub mod teacher;
