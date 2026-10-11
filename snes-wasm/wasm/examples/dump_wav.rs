//! Dev tool: renders .xm modules to WAV in /tmp for auditioning, through the
//! SAME pipeline the game delivers (looped region, fresh-context fallback) —
//! not a raw libxm dump (review Important 3).
//! Usage with no argument: renders the embedded campaign tracks (tracks.rs).
//! Usage: cargo run --example dump_wav -- /tmp/cand1.xm /tmp/cand2.xm ...

use snes_wasm::music::tracker::{render_module, MUSIC_SR};

fn write_wav(path: &str, stereo: &[f32]) {
    // stereo interleaved i16 PCM; samples arrive already peak-normalized to
    // 0.95 by the tracker — no extra gain here, so the file matches the game.
    let mut data: Vec<u8> = Vec::with_capacity(stereo.len() * 2);
    for &s in stereo {
        let v = (s.clamp(-1.0, 1.0) as f64 * 32767.0) as i16;
        data.extend_from_slice(&v.to_le_bytes());
    }
    let mut wav = Vec::new();
    wav.extend_from_slice(b"RIFF");
    let riff_len = (36 + data.len()) as u32;
    wav.extend_from_slice(&riff_len.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    // "fmt " chunk: id, size 16, body
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend(1u16.to_le_bytes()); // PCM
    wav.extend(2u16.to_le_bytes()); // stereo
    wav.extend_from_slice(&(MUSIC_SR as u32).to_le_bytes());
    wav.extend_from_slice(&((MUSIC_SR * 4) as u32).to_le_bytes());
    wav.extend(4u16.to_le_bytes());
    wav.extend(16u16.to_le_bytes());
    // "data" chunk
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(data.len() as u32).to_le_bytes());
    wav.extend_from_slice(&data);
    std::fs::write(path, wav).unwrap();
    println!("wrote {path} ({:.1}s)", stereo.len() as f64 / 2.0 / MUSIC_SR as f64);
}

fn main() {
    if std::env::args().len() > 1 {
        for p in std::env::args().skip(1) {
            let data = std::fs::read(&p).unwrap_or_else(|e| panic!("read {p}: {e}"));
            let stem = std::path::Path::new(&p).file_stem().unwrap().to_string_lossy().into_owned();
            let stereo = render_module(data);
            write_wav(&format!("/tmp/{stem}.wav"), &stereo);
        }
        return;
    }
    // No argument: the embedded campaign tracks, with the game's track ids.
    for (name, id) in [
        ("SMB3 / jellydog", 0u8),
        ("Summer Memories / Arachno & Dualtrax", 1),
        ("Shock Therapy / Chromag", 2),
        ("Bar Rocks / Cerror", 3),
    ] {
        let stereo = snes_wasm::music::render_music(id);
        write_wav(&format!("/tmp/level{}.wav", id + 1), &stereo);
        println!("  {name}");
    }
}
