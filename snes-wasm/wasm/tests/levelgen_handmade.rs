use snes_wasm::levelgen::bot::{play, EXPERT, PROFILES};
use snes_wasm::levelgen::compile::{compile_strict, random_legal, Compiler};
use snes_wasm::levelgen::handmade::{mutate, style_score, CAMPAIGN};
use snes_wasm::levelgen::rng::Rng;

#[test]
fn every_transcription_is_legal() {
    for t in &CAMPAIGN {
        let mut c = Compiler::new(t.theme);
        for m in t.motifs() {
            let (x, top) = (c.x(), c.top);
            assert!(c.push(m), "[{}] {m:?} refused at x={x} top={top}", t.name);
        }
        assert!(c.can_end(), "[{}] cannot end at x={} (excursions {})", t.name, c.x(), c.excursions());
    }
}

#[test]
fn the_expert_finishes_every_transcription() {
    for t in &CAMPAIGN {
        let r = play(t.compile().level_data(), EXPERT, 0, 2);
        assert!(r.won, "[{}] {r:?}\n{}", t.name, t.compile().ascii());
    }
}

/// How hard the graded bots find a level: deaths (≤ 3) plus a 2-point
/// penalty per failed profile, summed over profiles, averaged over seeds.
fn struggle(data: &snes_wasm::world::level::LevelData) -> f32 {
    let mut s = 0.0;
    for seed in 0..3 {
        for p in PROFILES {
            let r = play(data.clone(), p, seed, 2);
            s += r.deaths.min(3) as f32 + if r.won { 0.0 } else { 2.0 };
        }
    }
    s / 3.0
}

#[test]
fn the_bots_order_the_shipped_campaign() {
    // Calibration: the shipped levels were designed easy → hard; the graded
    // bots must agree (absolute buckets cannot be calibrated: the campaign was
    // only ever tuned for the expert bot).
    let s: Vec<f32> = ["level1", "level2", "level3", "level4"]
        .iter()
        .map(|n| {
            let path = format!("{}/../public/levels/{n}.json", env!("CARGO_MANIFEST_DIR"));
            struggle(&snes_wasm::tiled::parse_tiled_json(&std::fs::read_to_string(path).unwrap()).unwrap())
        })
        .collect();
    println!("campaign struggle: {s:?}");
    assert!(s.windows(2).all(|w| w[1] >= w[0]), "struggle must not decrease along the campaign: {s:?}");
    assert!(s[3] > s[0], "level 4 harder than level 1: {s:?}");
}

#[test]
fn mutations_are_mostly_legal_and_new() {
    for t in &CAMPAIGN {
        let src = t.motifs();
        let mut rng = Rng::new(9);
        let (mut legal, mut changed) = (0, 0);
        for _ in 0..200 {
            let m = mutate(&src, &mut rng);
            if m != src {
                changed += 1;
            }
            if compile_strict(t.theme, &m).is_some() {
                legal += 1;
            }
        }
        assert!(legal >= 40, "[{}] only {legal}/200 legal mutations", t.name);
        assert!(changed >= 180, "[{}] only {changed}/200 mutations change something", t.name);
    }
}

#[test]
fn style_score_rewards_the_campaign_idiom() {
    for t in &CAMPAIGN {
        assert_eq!(style_score(&t.motifs()), 1.0, "[{}]", t.name);
    }
    let random: f32 =
        (0..50).map(|s| style_score(&random_legal(CAMPAIGN[s % 4].theme, s as u64).motifs)).sum::<f32>()
            / 50.0;
    assert!(random < 0.6, "random walks look too much like the campaign: {random}");
}
