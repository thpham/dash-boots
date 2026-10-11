use snes_wasm::TUNING;

#[test]
fn jump_velocity_is_negative() {
    assert!(TUNING.jump_v < 0.0);
}
