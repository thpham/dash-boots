//! Golden tests: every SFX rendered in Rust is checked against the Python WAV
//! (fixtures copied from public/audio/sfx). Python's noise (random.Random)
//! differs from Rust's deterministic xorshift, so noise-heavy effects get a
//! looser RMS tolerance.

use snes_wasm::audio::SFX_NAMES;
use snes_wasm::music::sfx::sfx_samples;

fn read_wav_i16(path: &std::path::Path) -> Vec<f32> {
    let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    assert!(&bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE", "not a wav: {path:?}");
    // Walk RIFF chunks to find `data` (Python's `wave` writes a standard layout).
    let mut off = 12;
    let mut data_off = None;
    while off + 8 <= bytes.len() {
        let id = &bytes[off..off + 4];
        let len = u32::from_le_bytes(bytes[off + 4..off + 8].try_into().unwrap()) as usize;
        if id == b"data" {
            data_off = Some(off + 8);
            break;
        }
        off += 8 + len + (len & 1);
    }
    let start = data_off.unwrap_or_else(|| panic!("no data chunk in {path:?}"));
    bytes[start..].as_chunks::<2>().0.iter().map(|c| i16::from_le_bytes(*c) as f32 / 32767.0).collect()
}

fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|s| s * s).sum::<f32>() / x.len() as f32).sqrt()
}

/// (name, relative RMS tolerance); noise-heavy: dash, stomp, rift.
const TOLERANCES: [(&str, f32); 13] = [
    ("jump", 0.15),
    ("dash", 0.45),
    ("coin", 0.10),
    ("stomp", 0.45),
    ("hurt", 0.15),
    ("death", 0.10),
    ("powerup", 0.10),
    ("checkpoint", 0.15),
    ("clear", 0.15),
    ("bump", 0.15),
    ("pause", 0.10),
    ("rift", 0.50),
    ("return", 0.15),
];

#[test]
fn every_sfx_matches_its_python_wav_within_tolerance() {
    for (id, (name, tol)) in TOLERANCES.iter().enumerate() {
        assert_eq!(SFX_NAMES[id], *name, "fixture order must match SFX_NAMES");
        let wav = read_wav_i16(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/fixtures/sfx/{name}.wav")),
        );
        let out = sfx_samples(id as u8);
        assert!(out.iter().all(|s| s.is_finite()), "[{name}] non-finite sample");
        assert!(
            (out.len() as i64 - wav.len() as i64).abs() <= 2,
            "[{name}] length: rust {} vs wav {}",
            out.len(),
            wav.len()
        );
        let n = out.len().min(wav.len());
        let rms_rust = rms(&out[..n].iter().map(|&s| s as f32).collect::<Vec<_>>());
        let rms_wav = rms(&wav[..n]);
        let diff = (rms_rust - rms_wav) / rms_wav.max(1e-9);
        assert!(
            diff.abs() <= *tol,
            "[{name}] rms diff {:.1}% > {:.0}% (rust {:.4} wav {:.4})",
            diff.abs() * 100.0,
            tol * 100.0,
            rms_rust,
            rms_wav
        );
    }
}
