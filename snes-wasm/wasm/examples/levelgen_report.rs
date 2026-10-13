//! Where do the bots fail? Teacher levels per (theme, target), every profile
//! plays each one; failures (deaths, stuck) are attributed to the motif span
//! under them. Prints failure rate per motif and per motif bigram, per profile,
//! plus the measured-difficulty histogram per target. Use it to tune the
//! teacher weights and the grammar.
//! cargo run --release --example levelgen_report -- [levels per cell]

use std::collections::HashMap;

use snes_wasm::levelgen::bot::{self, play, PROFILES};
use snes_wasm::levelgen::compile::compile_strict;
use snes_wasm::levelgen::motif::{Motif, Theme};
use snes_wasm::levelgen::teacher;

fn main() {
    let n: u64 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(20);
    // (profile, motif) -> (occurrences, failures); same for bigrams.
    let mut per_motif: HashMap<(usize, Motif), (u32, u32)> = HashMap::new();
    let mut per_bigram: HashMap<(usize, Motif, Motif), (u32, u32)> = HashMap::new();
    let mut hist: HashMap<(u8, Option<u8>), u32> = HashMap::new();
    for theme in Theme::ALL {
        for target in 1..=5u8 {
            for seed in 0..n {
                let seq = teacher::generate(theme, target, seed);
                let lvl = compile_strict(theme, &seq).expect("teacher is legal");
                let data = lvl.level_data();
                *hist.entry((target, bot::measure(&data, seed))).or_default() += 1;
                for (pi, p) in PROFILES.iter().enumerate() {
                    let r = play(data.clone(), *p, seed, 2);
                    let mut failed_spans = Vec::new();
                    for x in r.death_x.iter().chain(r.stuck_x.iter()) {
                        if let Some(s) = lvl.span_at_tile((*x / 16.0) as i32 + 1) {
                            failed_spans.push(s.x0);
                        }
                    }
                    for (i, s) in lvl.spans.iter().enumerate() {
                        let failed = failed_spans.contains(&s.x0) as u32;
                        let e = per_motif.entry((pi, s.motif)).or_default();
                        e.0 += 1;
                        e.1 += failed;
                        if i > 0 {
                            let e = per_bigram.entry((pi, lvl.spans[i - 1].motif, s.motif)).or_default();
                            e.0 += 1;
                            e.1 += failed;
                        }
                    }
                }
            }
        }
    }
    for (pi, p) in PROFILES.iter().enumerate() {
        let mut rows: Vec<(f32, u32, String)> = per_motif
            .iter()
            .filter(|((i, _), (occ, _))| *i == pi && *occ >= 10)
            .map(|((_, m), (occ, f))| (*f as f32 / *occ as f32, *occ, m.name()))
            .collect();
        rows.sort_by(|a, b| b.0.total_cmp(&a.0));
        println!("== {} — worst motifs (failure rate, occurrences)", p.name);
        for (rate, occ, name) in rows.iter().take(12) {
            println!("  {name:12} {:5.1}%  ({occ})", rate * 100.0);
        }
        let mut rows: Vec<(f32, u32, String)> = per_bigram
            .iter()
            .filter(|((i, _, _), (occ, _))| *i == pi && *occ >= 8)
            .map(|((_, a, b), (occ, f))| {
                (*f as f32 / *occ as f32, *occ, format!("{} {}", a.name(), b.name()))
            })
            .collect();
        rows.sort_by(|a, b| b.0.total_cmp(&a.0));
        println!("   worst bigrams");
        for (rate, occ, name) in rows.iter().take(8) {
            println!("  {name:24} {:5.1}%  ({occ})", rate * 100.0);
        }
    }
    println!("== measured difficulty per teacher target (None = nobody finishes)");
    for target in 1..=5u8 {
        let row: Vec<String> = [None, Some(1), Some(2), Some(3), Some(4), Some(5)]
            .iter()
            .map(|m| format!("{:4}", hist.get(&(target, *m)).copied().unwrap_or(0)))
            .collect();
        println!("  target d{target}: none,d1..d5 = {}", row.join(" "));
    }
}
