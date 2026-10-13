//! GRPO rollouts: sample groups with exactly the game's inference and grammar
//! mask, score them with the bots, print JSON lines for tools/levelgen/grpo.py.
//! Group g uses theme ALL[(seed + g) % 4] and difficulty 1 + (seed / 4 + g) % 5.

use serde_json::json;
use snes_wasm::levelgen::model::{sample, Model};
use snes_wasm::levelgen::motif::Theme;
use snes_wasm::levelgen::reward::score;

fn arg<T: std::str::FromStr>(args: &[String], name: &str, default: T) -> T {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let weights: String = arg(&args, "--weights", String::new());
    let groups: u64 = arg(&args, "--groups", 8);
    let size: u64 = arg(&args, "--size", 16);
    let seed: u64 = arg(&args, "--seed", 0);
    let temperature: f32 = arg(&args, "--temperature", 1.0);
    let threads: u64 = arg(&args, "--threads", 4);
    let bytes = std::fs::read(&weights).unwrap_or_else(|e| panic!("read {weights}: {e}"));
    let model = Model::from_bytes(&bytes).unwrap_or_else(|e| panic!("{e}"));

    let jobs: Vec<(u64, u64)> = (0..groups).flat_map(|g| (0..size).map(move |k| (g, k))).collect();
    let lines: Vec<String> = std::thread::scope(|scope| {
        let chunks: Vec<_> = jobs
            .chunks(jobs.len().div_ceil(threads as usize).max(1))
            .map(|chunk| {
                let model = &model;
                scope.spawn(move || {
                    chunk
                        .iter()
                        .map(|&(g, k)| {
                            let theme = Theme::ALL[((seed + g) % 4) as usize];
                            let difficulty = 1 + ((seed / 4 + g) % 5) as u8;
                            let s = seed.wrapping_mul(1_000_003).wrapping_add(g * size + k);
                            match sample(model, theme, difficulty, s, temperature) {
                                None => json!({"group": g, "theme": theme.index(), "difficulty": difficulty,
                                               "failed": true}),
                                Some(tr) => {
                                    let r = score(&tr.level, difficulty, s);
                                    json!({"group": g, "theme": theme.index(), "difficulty": difficulty,
                                           "tokens": tr.tokens, "masks": tr.masks, "reward": r.total,
                                           "measured": r.measured, "style": r.style, "repetition": r.repetition})
                                }
                            }
                            .to_string()
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        chunks.into_iter().flat_map(|h| h.join().expect("rollout thread")).collect()
    });
    for l in lines {
        println!("{l}");
    }
}
