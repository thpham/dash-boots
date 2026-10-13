//! Evaluate a level model: per (theme, difficulty) cell, sample N levels with
//! fixed seeds; accepted = the target-skill player finishes in 2 of 3 attempts; also measured
//! difficulty, style, distinct levels and native latency.
//! cargo run --release --example levelgen_eval -- [--weights PATH] [--n N] [--seed0 S]

use std::collections::HashSet;
use std::time::Instant;

use snes_wasm::levelgen::bot;
use snes_wasm::levelgen::handmade::style_score;
use snes_wasm::levelgen::model::{sample, Model, WEIGHTS};
use snes_wasm::levelgen::motif::Theme;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let get = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    let bytes = get("--weights")
        .map(|p| std::fs::read(&p).expect("read weights"))
        .unwrap_or_else(|| WEIGHTS.to_vec());
    let n: u64 = get("--n").and_then(|s| s.parse().ok()).unwrap_or(20);
    let seed0: u64 = get("--seed0").and_then(|s| s.parse().ok()).unwrap_or(10_000);
    let model = Model::from_bytes(&bytes).unwrap_or_else(|e| panic!("{e}"));
    println!("model: {} params, {} KiB", model.n_params(), bytes.len() / 1024);

    let (mut accepted_all, mut total_all, mut min_cell, mut sample_s) = (0u64, 0u64, 100.0f32, 0.0f64);
    let mut mean_measured = [0.0f32; 5];
    println!("{:8} {:>3} {:>8} {:>9} {:>6} {:>8}", "theme", "d", "accept%", "measured", "style", "distinct");
    for theme in Theme::ALL {
        for d in 1..=5u8 {
            let (mut ok, mut meas, mut style) = (0u64, 0.0f32, 0.0f32);
            let mut distinct = HashSet::new();
            for i in 0..n {
                let seed = seed0 + i;
                let t = Instant::now();
                let tr = sample(&model, theme, d, seed, 1.0).expect("grammar never dead-ends");
                sample_s += t.elapsed().as_secs_f64();
                let data = tr.level.level_data();
                ok += bot::finishes_reliably(&data, d, seed) as u64;
                meas += bot::measure(&data, seed).unwrap_or(6) as f32;
                style += style_score(&tr.level.motifs);
                distinct.insert(tr.tokens.clone());
            }
            let pct = 100.0 * ok as f32 / n as f32;
            min_cell = min_cell.min(pct);
            accepted_all += ok;
            total_all += n;
            mean_measured[d as usize - 1] += meas / n as f32 / 4.0;
            println!(
                "{:8} {:>3} {:>7.1}% {:>9.2} {:>6.2} {:>8}",
                theme.name(),
                d,
                pct,
                meas / n as f32,
                style / n as f32,
                distinct.len()
            );
        }
    }
    let monotonic = mean_measured.windows(2).all(|w| w[1] > w[0]);
    println!("mean measured per tag: {mean_measured:?}");
    println!("ACCEPT {:.1}", 100.0 * accepted_all as f32 / total_all as f32);
    println!("MIN_CELL {min_cell:.1}");
    println!("MONOTONIC {}", if monotonic { "yes" } else { "no" });
    println!("MS_PER_LEVEL {:.2}", sample_s * 1e3 / total_all as f64);
}
