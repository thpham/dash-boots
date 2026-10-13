use snes_wasm::levelgen::model::{Model, Session};
use snes_wasm::levelgen::motif::vocab;

/// A tiny all-zero model in either format (1 layer, d = 4).
fn tiny(int8: bool) -> Vec<u8> {
    let (v, ctx, d, nl, nh, ff) = (vocab::size() as u32, 4u32, 4u32, 1u32, 1u32, 4u32);
    let mut b = Vec::new();
    b.extend_from_slice(if int8 { b"LGM2" } else { b"LGM1" });
    for n in [v, ctx, d, nl, nh, ff] {
        b.extend_from_slice(&n.to_le_bytes());
    }
    let mat = |b: &mut Vec<u8>, rows: u32, cols: u32| {
        if int8 {
            for _ in 0..rows {
                b.extend_from_slice(&1.0f32.to_le_bytes());
            }
            b.extend(std::iter::repeat_n(0u8, (rows * cols) as usize));
        } else {
            b.extend(std::iter::repeat_n(0u8, (rows * cols * 4) as usize));
        }
    };
    let vec = |b: &mut Vec<u8>, n: u32| b.extend(std::iter::repeat_n(0u8, (n * 4) as usize));
    mat(&mut b, v, d);
    mat(&mut b, ctx, d);
    vec(&mut b, d); // ln1 w
    vec(&mut b, d); // ln1 b
    mat(&mut b, 3 * d, d);
    vec(&mut b, 3 * d);
    mat(&mut b, d, d);
    vec(&mut b, d);
    vec(&mut b, d); // ln2 w
    vec(&mut b, d); // ln2 b
    mat(&mut b, ff, d);
    vec(&mut b, ff);
    mat(&mut b, d, ff);
    vec(&mut b, d);
    vec(&mut b, d); // lnf w
    vec(&mut b, d); // lnf b
    b
}

#[test]
fn both_weight_formats_load_and_run() {
    for int8 in [false, true] {
        let m = Model::from_bytes(&tiny(int8)).expect("loads");
        let logits = Session::new(&m).step(vocab::EOS);
        assert_eq!(logits.len(), vocab::size());
    }
}

#[test]
fn bad_weights_are_errors_not_panics() {
    assert!(Model::from_bytes(b"nope").is_err());
    let mut b = tiny(true);
    b.truncate(b.len() - 3);
    assert!(Model::from_bytes(&b).is_err());
    let mut b = tiny(false);
    b[4] ^= 1; // vocab size drift
    assert!(Model::from_bytes(&b).is_err());
}
