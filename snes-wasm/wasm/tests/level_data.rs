mod common;

use common::*;

#[test]
fn rift_with_flat_runway_is_valid() {
    let d = level(&["..........", ".PR.......", "##########"]);
    assert_eq!(d.spawns.rifts.len(), 1);
    d.validate().unwrap();
}

#[test]
fn rift_before_a_pit_is_rejected() {
    let d = level(&["..........", ".PR.......", "######.###"]);
    let e = d.validate().unwrap_err();
    assert!(e.contains("rift") && e.contains("flat"), "{e}");
}

#[test]
fn rift_before_a_step_is_rejected() {
    let d = level(&["..........", ".PR...#...", "##########"]);
    let e = d.validate().unwrap_err();
    assert!(e.contains("rift") && e.contains("flat"), "{e}");
}

#[test]
fn rift_with_enemy_in_runway_is_rejected() {
    let d = level(&["..........", ".PR..E....", "##########"]);
    let e = d.validate().unwrap_err();
    assert!(e.contains("rift") && e.contains("enemy"), "{e}");
}

#[test]
fn enemy_just_after_runway_is_fine() {
    let d = level(&["...........", ".PR......E.", "###########"]);
    d.validate().unwrap();
}
