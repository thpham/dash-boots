//! The 13 core SFX recipes, ported 1:1 from gen_audio.py (Python f64 ->
//! f64). Python's `random.Random(seed)` noise is replaced by a deterministic
//! xorshift (see synth::Noise), so the golden tests compare RMS within a
//! tolerance instead of sample-exact — the ear stays the final judge.
//! The newer ids are Rust-native (bell, wheel_slide, crust, moo).

use super::synth::{mix_into, note_freq, notes, pulse, sweep, triangle, Noise, SR};
use crate::audio::SFX_NAMES;

fn jump() -> Vec<f64> {
    sweep(0.16, 260.0, 640.0, |p| pulse(p, 0.25), 0.5, 6.0)
}

fn dash() -> Vec<f64> {
    let mut nz = Noise::new(7);
    let n = (0.28 * SR) as usize;
    let mut lp = 0.0;
    let mut out: Vec<f64> = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f64 / n as f64;
        lp += (nz.sample(16000.0) - lp) * (0.6 - 0.5 * t);
        out.push(lp * 0.7 * (1.0 - t).powf(1.5));
    }
    mix_into(out, &sweep(0.2, 900.0, 180.0, |p| pulse(p, 0.5), 0.25, 6.0), 0)
}

fn coin() -> Vec<f64> {
    notes(&[("B5", 0.07), ("E6", 0.38)], |p| pulse(p, 0.5), 0.35, 0.0)
}

fn stomp() -> Vec<f64> {
    let out = sweep(0.14, 520.0, 110.0, |p| pulse(p, 0.5), 0.45, 8.0);
    let mut nz = Noise::new(3);
    let thump: Vec<f64> = (0..(0.08 * SR) as usize)
        .map(|i| nz.sample(4000.0) * 0.4 * (-(i as f64 / SR) * 40.0).exp())
        .collect();
    mix_into(out, &thump, 0)
}

fn hurt() -> Vec<f64> {
    let n = (0.4 * SR) as usize;
    let mut out = Vec::with_capacity(n);
    let mut phase = 0.0;
    for i in 0..n {
        let t = i as f64 / SR;
        let f = 520.0 * (1.0 - 0.5 * t / 0.4) * (1.0 + 0.08 * std::f64::consts::TAU * 28.0 * t).sin();
        phase += f / SR;
        out.push(pulse(phase, 0.5) * 0.4 * (1.0 - t / 0.4));
    }
    out
}

fn death() -> Vec<f64> {
    notes(
        &[("B4", 0.11), ("F5", 0.11), ("F5", 0.11), ("F5", 0.15), ("E5", 0.15), ("D5", 0.15), ("C5", 0.5)],
        |p| pulse(p, 0.5),
        0.35,
        0.0,
    )
}

fn powerup() -> Vec<f64> {
    let seq = ["C5", "E5", "G5", "C6", "D5", "F#5", "A5", "D6", "E5", "G#5", "B5", "E6"];
    let mut s: Vec<(&str, f64)> = seq.iter().map(|n| (*n, 0.045)).collect();
    s.push(("E6", 0.25));
    notes(&s, |p| pulse(p, 0.25), 0.35, 0.0)
}

fn checkpoint() -> Vec<f64> {
    let a = notes(&[("E6", 0.1), ("B6", 0.45)], triangle, 0.5, 0.0);
    let b = notes(&[("E5", 0.1), ("B5", 0.45)], |p| pulse(p, 0.125), 0.18, 0.0);
    mix_into(a, &b, 0)
}

fn clear() -> Vec<f64> {
    let lead = notes(
        &[("A5", 0.13), ("D6", 0.13), ("F#6", 0.13), ("A6", 0.4), ("F#6", 0.13), ("A6", 0.9)],
        |p| pulse(p, 0.25),
        0.3,
        0.0,
    );
    let harm = notes(
        &[("F#5", 0.13), ("A5", 0.13), ("D6", 0.13), ("F#6", 0.4), ("D6", 0.13), ("F#6", 0.9)],
        |p| pulse(p, 0.5),
        0.13,
        0.0,
    );
    let bass = notes(&[("D3", 0.39), ("A2", 0.4), ("D3", 0.13), ("D3", 0.9)], triangle, 0.45, 0.0);
    mix_into(mix_into(lead, &harm, 0), &bass, 0)
}

fn bump() -> Vec<f64> {
    sweep(0.07, 180.0, 90.0, |p| pulse(p, 0.5), 0.45, 4.0)
}

fn pause() -> Vec<f64> {
    notes(&[("E6", 0.06), ("B5", 0.06), ("E6", 0.12)], |p| pulse(p, 0.5), 0.3, 0.0)
}

/// Tiny LCG replacing the rift's `random.Random(seed).choice(...)`.
fn rift_pick<'a>(state: &mut u64, items: &[&'a str]) -> &'a str {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    items[(*state >> 33) as usize % items.len()]
}

fn rift() -> Vec<f64> {
    let mut state = 5 ^ 0x9E3779B97F4A7C15; // nonzero seed
    let seq: Vec<(&str, f64)> = (0..10)
        .map(|_| rift_pick(&mut state, &["E6", "B6", "F#6", "C#7", "G#6", "D#7"]))
        .map(|n| (n, 0.03))
        .collect();
    let out = notes(&seq, |p| pulse(p, 0.125), 0.28, 0.0);
    let out = mix_into(out, &sweep(0.5, 400.0, 2400.0, triangle, 0.3, 3.0), 0);
    let mut nz = Noise::new(11);
    let burst: Vec<f64> = (0..(0.25 * SR) as usize)
        .map(|i| nz.sample(18000.0) * 0.25 * (-(i as f64 / SR) * 18.0).exp())
        .collect();
    mix_into(out, &burst, 0)
}

fn return_warm() -> Vec<f64> {
    let n = (1.6 * SR) as usize;
    let freqs: Vec<f64> = ["D4", "F#4", "A4", "D5"].iter().map(|name| note_freq(name)).collect();
    let mut phases = vec![0.0; freqs.len() * 2];
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f64 / SR;
        let wow = 1.0 + 0.004 * (std::f64::consts::TAU * 0.8 * t).sin();
        let env = (t / 0.08).min(1.0) * (-t * 1.6).exp();
        let mut s = 0.0;
        for (k, &f) in freqs.iter().enumerate() {
            for (j, &det) in [0.997, 1.003].iter().enumerate() {
                phases[k * 2 + j] += f * det * wow / SR;
                s += triangle(phases[k * 2 + j]);
            }
        }
        out.push(s / phases.len() as f64 * 0.8 * env);
    }
    out
}

/// The alpine livestock chime: two struck dings an octave apart.
fn bell() -> Vec<f64> {
    notes(&[("E6", 0.5), ("B5", 0.7)], triangle, 0.4, 0.02)
}

/// "Meuh": a low glide with a slow chesty attack and a slight wobble — the
/// cows' one greeting, never a chime.
fn moo() -> Vec<f64> {
    let dur = 0.65;
    let n = (dur * SR) as usize;
    let mut out = Vec::with_capacity(n);
    let mut phase = 0.0;
    let mut sub_phase = 0.0;
    for i in 0..n {
        let t = i as f64 / SR;
        // Sliding down from a moo's "m-o" into the long "uh", with a soft
        // 5 Hz wobble (like the jaw letting go).
        let f = 170.0 - 55.0 * (t / dur).powf(0.8) + 6.0 * (std::f64::consts::TAU * 5.0 * t).sin();
        phase += f / SR;
        // A sub-octave rumble under the glide, for chestiness.
        sub_phase += (85.0 - 27.5 * (t / dur).powf(0.8)) / SR;
        let env = (t / 0.12).min(1.0) * (1.0 - (t / dur).powf(2.0));
        out.push(triangle(phase) * 0.65 * env + triangle(sub_phase) * 0.35 * env);
    }
    out
}

/// A rolling cheese wheel: wooden rattle — LP-smoothed noise taps at ~11 Hz.
fn wheel_slide() -> Vec<f64> {
    let mut nz = Noise::new(11);
    let n = (0.35 * SR) as usize;
    let mut lp = 0.0;
    let mut out: Vec<f64> = Vec::with_capacity(n);
    for i in 0..n {
        let rattle = 0.5 + 0.5 * (std::f64::consts::TAU * 11.0 * i as f64 / SR).sin();
        lp += (nz.sample(12000.0) - lp) * (0.6 - 0.5 * i as f64 / n as f64);
        out.push(lp * 0.65 * (1.0 - i as f64 / n as f64) * rattle);
    }
    out
}

/// The crust settles: a 60 ms noise crunch over a low thud sweep.
fn crust() -> Vec<f64> {
    let mut nz = Noise::new(3);
    let n = (0.06 * SR) as usize;
    let crunch: Vec<f64> =
        (0..n).map(|i| nz.sample(9000.0) * 0.9 * (1.0 - i as f64 / n as f64).powi(2)).collect();
    mix_into(sweep(0.13, 190.0, 55.0, triangle, 0.5, 10.0), &crunch, 0)
}

/// Mono raw (Python parity before normalization), in `audio::SFX_NAMES` order.
pub fn sfx_samples(id: u8) -> Vec<f64> {
    assert!((id as usize) < SFX_NAMES.len(), "unknown sfx {id}");
    match id {
        0 => jump(),
        1 => dash(),
        2 => coin(),
        3 => stomp(),
        4 => hurt(),
        5 => death(),
        6 => powerup(),
        7 => checkpoint(),
        8 => clear(),
        9 => bump(),
        10 => pause(),
        11 => rift(),
        12 => return_warm(),
        13 => bell(),
        14 => wheel_slide(),
        15 => crust(),
        16 => moo(),
        _ => unreachable!(),
    }
}

/// Mono -> stereo-duplicated, peak-normalized to 0.95 (parity with Python's write_wav).
pub fn render_sfx(id: u8) -> Vec<f32> {
    let mono = sfx_samples(id);
    let peak = mono.iter().fold(0.0f64, |a, &s| a.max(s.abs()));
    let g = (0.95 / peak.max(1e-9)).min(1.0);
    mono.iter()
        .flat_map(|&s| {
            let v = s * g;
            [v as f32, v as f32]
        })
        .collect()
}
