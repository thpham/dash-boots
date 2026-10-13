//! Print the model vocabulary in token-id order (read by tools/levelgen/lgm.py).
use snes_wasm::levelgen::motif::vocab;

fn main() {
    for t in 0..vocab::size() as u32 {
        println!("{}", vocab::token_name(t));
    }
}
