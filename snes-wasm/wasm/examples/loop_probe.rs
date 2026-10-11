//! Dev tool: measures where a .xm module actually loops. Tracks the play
//! position and reports the first two backward pattern-order jumps (song-loop
//! modules jump to a loop order; end-of-orders modules wrap and loop_count()
//! increments — both show up as pattern_index going backward). The distance
//! between the two events is one seamless loop cycle.
//! Usage: cargo run --example loop_probe -- f.xm ...

use libxm::XMContext;

const SR: u32 = 22050;
const MAX_SECONDS: f64 = 600.0;
const MAX_FRAMES: usize = (SR as f64 * MAX_SECONDS) as usize;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    for src in &args {
        let data = std::fs::read(src).unwrap_or_else(|e| panic!("read {src}: {e}"));
        let mut ctx = XMContext::new(&data, SR).unwrap_or_else(|e| panic!("{src}: {e:?}"));
        let mut prev = ctx.position().pattern_index;
        let mut jumps: Vec<usize> = Vec::new(); // frame offsets of backward jumps
        let mut played = 0usize;
        let mut frames = [0f32; 2 * 4096];
        while played < MAX_FRAMES && jumps.len() < 2 {
            ctx.generate_samples(&mut frames);
            played += 4096;
            let idx = ctx.position().pattern_index;
            if idx < prev {
                jumps.push(played);
            }
            prev = idx;
        }
        if jumps.len() >= 2 {
            let len = jumps[1] - jumps[0];
            println!(
                "{src}: LOOP {:.1}s (jump1 {:.1}s, jump2 {:.1}s, loop_count {})",
                len as f64 / SR as f64,
                jumps[0] as f64 / SR as f64,
                jumps[1] as f64 / SR as f64,
                ctx.loop_count()
            );
        } else if !jumps.is_empty() {
            println!(
                "{src}: one jump at {:.1}s, no second in {MAX_SECONDS:.0}s ({jumps:?})",
                jumps[0] as f64 / SR as f64
            );
        } else {
            println!("{src}: no backward jump in {MAX_SECONDS:.0}s");
        }
    }
}
