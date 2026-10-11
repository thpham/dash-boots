//! Synthesis primitives for the SFX (exact port of the SFX subset of
//! gen_audio.py, f64 like Python; music goes through libxm, see tracker.rs).

pub const SR: f64 = 22050.0;

pub fn note_freq(name: &str) -> f64 {
    let (pitch, octave) = name.split_at(name.len() - 1);
    let idx = match pitch {
        "C" => 0,
        "C#" => 1,
        "D" => 2,
        "D#" => 3,
        "E" => 4,
        "F" => 5,
        "F#" => 6,
        "G" => 7,
        "G#" => 8,
        "A" => 9,
        "A#" => 10,
        "B" => 11,
        _ => panic!("unknown note {pitch}"),
    };
    let midi = 12 * (octave.parse::<i32>().expect("note octave") + 1) + idx;
    440.0 * 2f64.powf((midi - 69) as f64 / 12.0)
}

pub fn pulse(phase: f64, duty: f64) -> f64 {
    if phase % 1.0 < duty {
        1.0
    } else {
        -1.0
    }
}

pub fn triangle(phase: f64) -> f64 {
    let p = phase % 1.0;
    if p < 0.5 {
        4.0 * p - 1.0
    } else {
        3.0 - 4.0 * p
    }
}

/// Replaces Python's `random.Random(seed)` with a deterministic xorshift.
pub struct Noise {
    state: u64,
    value: f64,
    counter: f64,
}

impl Noise {
    pub fn new(seed: u64) -> Self {
        let state = if seed == 0 { 0x9E3779B97F4A7C15 } else { seed.wrapping_mul(0x9E3779B97F4A7C15) };
        Noise { state, value: 0.0, counter: 0.0 }
    }

    /// Sample-and-hold: a lower rate means darker noise.
    pub fn sample(&mut self, rate_hz: f64) -> f64 {
        self.counter += rate_hz / SR;
        if self.counter >= 1.0 {
            self.counter %= 1.0;
            let mut x = self.state;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.state = x;
            self.value = (x & 0xFFFFFF) as f64 / 0x7FFFFF as f64 - 1.0; // [-1, 1]
        }
        self.value
    }
}

pub fn sweep(dur: f64, f0: f64, f1: f64, wave: impl Fn(f64) -> f64, vol: f64, decay: f64) -> Vec<f64> {
    let n = (dur * SR) as usize;
    let mut out = Vec::with_capacity(n);
    let mut phase = 0.0;
    for i in 0..n {
        let t = i as f64 / n as f64;
        let f = f0 * (f1 / f0).powf(t); // curve='exp' (every Python call site)
        phase += f / SR;
        out.push(wave(phase) * vol * (-t * decay / 3.0).exp() * ((n - i) as f64 / (0.01 * SR)).min(1.0));
    }
    out
}

pub fn notes(seq: &[(&str, f64)], wave: impl Fn(f64) -> f64, vol: f64, gap: f64) -> Vec<f64> {
    let mut out = Vec::new();
    for &(name, dur) in seq {
        let n = (dur * SR) as usize;
        let f = note_freq(name);
        let mut phase = 0.0;
        for i in 0..n {
            let t = i as f64 / SR;
            phase += f / SR;
            out.push(wave(phase) * vol * (-t * 4.0).exp() * ((n - i) as f64 / (0.008 * SR)).min(1.0));
        }
        out.extend(std::iter::repeat_n(0.0, (gap * SR) as usize));
    }
    out
}

pub fn mix_into(mut base: Vec<f64>, other: &[f64], offset: usize) -> Vec<f64> {
    let need = offset + other.len();
    if base.len() < need {
        base.resize(need, 0.0);
    }
    for (i, &v) in other.iter().enumerate() {
        base[offset + i] += v;
    }
    base
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pulse_fills_duty() {
        // duty 0.25 : +1 sur le premier quart de cycle, -1 ensuite
        assert_eq!(pulse(0.0, 0.25), 1.0);
        assert_eq!(pulse(0.25, 0.25), -1.0);
        assert_eq!(pulse(0.249_99, 0.25), 1.0);
    }

    #[test]
    fn triangle_goes_up_then_down() {
        assert_eq!(triangle(0.0), -1.0);
        assert_eq!(triangle(0.25), 0.0);
        assert_eq!(triangle(0.5), 1.0);
        assert_eq!(triangle(0.75), 0.0);
    }

    #[test]
    fn note_freq_matches_a4() {
        assert!((note_freq("A4") - 440.0).abs() < 1e-9);
        assert!((note_freq("C5") - 523.251130601).abs() < 1e-6);
        assert!((note_freq("E6") - 1318.51022765).abs() < 1e-6);
    }

    #[test]
    fn noise_holds_between_retriggers() {
        let mut nz = Noise::new(42);
        // at 4000 Hz, ~5.5 held samples per noise sample (22050/4000)
        let a = nz.sample(4000.0);
        let b = nz.sample(4000.0);
        assert!((a - b).abs() < 1e-9, "sample-and-hold");
        assert!(a.abs() <= 1.0);
    }

    #[test]
    fn sweep_has_python_length_and_decay() {
        let out = sweep(0.16, 260.0, 640.0, |p| pulse(p, 0.25), 0.5, 6.0);
        assert_eq!(out.len(), (0.16 * SR) as usize, "int(dur*SR)");
        assert!(out.iter().all(|s| s.is_finite()));
        assert!(out[out.len() - 1].abs() < out[0].abs() / 2.0, "exponential decay");
    }

    #[test]
    fn notes_sums_lengths_and_mix_into_resizes() {
        let out = notes(&[("B5", 0.07), ("E6", 0.38)], |p| pulse(p, 0.5), 0.35, 0.02);
        // Python's notes() adds the gap after EACH note, port is faithful.
        let per_note =
            (0.07 * SR) as usize + (0.02 * SR) as usize + (0.38 * SR) as usize + (0.02 * SR) as usize;
        assert_eq!(out.len(), per_note);
        let base = vec![0.5; 10];
        let base = mix_into(base, &[1.0, 2.0], 20);
        assert_eq!(base.len(), 22, "resized to offset+len");
        assert_eq!(base[20], 1.0);
        assert_eq!(base[9], 0.5, "zone avant l'offset intacte");
    }
}
