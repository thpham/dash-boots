use snes_wasm::world::shift::*;

fn run(s: &mut Shift, n: u32) {
    for _ in 0..n {
        s.step();
    }
}

#[test]
fn starts_settled_in_the_past() {
    let s = Shift::new(World::Past);
    assert_eq!(s.world(), World::Past);
    assert!(!s.is_shifting());
    let p = s.params(false);
    assert_eq!((p.world_mix, p.glitch, p.pivot, p.focus), (0.0, 0.0, 0.0, 0.0));
    assert_eq!(p.crt, CRT_BASE);
}

#[test]
fn to_future_timeline() {
    let mut s = Shift::new(World::Past);
    assert!(s.trigger());
    assert_eq!(s.world(), World::Future, "world() reports the destination");
    let p = s.params(false);
    assert_eq!(p.glitch, 1.0, "spark on the first frame");
    assert_eq!(p.world_mix, 0.0, "2D image still shown");
    run(&mut s, 3); // t = 0.05 s
    assert_eq!(s.params(false).world_mix, 1.0, "swap under the glitch");
    run(&mut s, 3); // t = 0.1 s
    assert_eq!(s.params(false).glitch, 0.0, "single short burst");
    run(&mut s, 30); // t = 0.6 s
    let p = s.params(false);
    assert!((p.extrude - 1.0).abs() < 1e-6, "wave done at 0.6 s");
    assert!(p.pivot > 0.0 && p.pivot < 1.0);
    run(&mut s, TO_FUTURE_STEPS - 36);
    assert_eq!(s.state, ShiftState::Future);
    let p = s.params(false);
    assert_eq!((p.world_mix, p.pivot, p.focus, p.crt), (1.0, 1.0, 1.0, 0.0));
}

#[test]
fn glitch_is_a_single_burst() {
    let mut s = Shift::new(World::Past);
    s.trigger();
    let mut lit = 0;
    for _ in 0..TO_FUTURE_STEPS {
        if s.params(false).glitch > 0.0 {
            lit += 1;
        }
        s.step();
    }
    assert!((5..=7).contains(&lit), "glitch frames: {lit}");
}

#[test]
fn back_to_past_then_nostalgia_decays() {
    let mut s = Shift::new(World::Future);
    assert!(s.trigger());
    assert_eq!(s.world(), World::Past);
    assert_eq!(s.params(false).world_mix, 1.0, "starts from the 3D view");
    run(&mut s, TO_PAST_STEPS - 1);
    let p = s.params(false);
    assert_eq!(p.world_mix, 0.0, "2D image back at the end");
    assert_eq!(p.glitch, 1.0, "spark at the swap");
    s.step();
    assert_eq!(s.state, ShiftState::Past);
    let p = s.params(false);
    assert!((p.crt - CRT_NOSTALGIA).abs() < 0.01 && (p.warmth - 1.0).abs() < 0.01);
    run(&mut s, NOSTALGIA_STEPS / 2);
    let p = s.params(false);
    assert!(p.crt > CRT_BASE && p.crt < CRT_NOSTALGIA && p.warmth > 0.0 && p.warmth < 1.0);
    run(&mut s, NOSTALGIA_STEPS);
    let p = s.params(false);
    assert_eq!((p.crt, p.warmth), (CRT_BASE, 0.0));
}

#[test]
fn trigger_is_ignored_while_shifting() {
    let mut s = Shift::new(World::Past);
    assert!(s.trigger());
    run(&mut s, 10);
    assert!(!s.trigger());
    assert_eq!(s.state, ShiftState::ToFuture(10));
}

#[test]
fn reduced_motion_is_a_plain_crossfade() {
    let mut s = Shift::new(World::Past);
    s.trigger();
    for i in 0..TO_FUTURE_STEPS {
        let p = s.params(true);
        assert_eq!(p.glitch, 0.0);
        assert_eq!((p.pivot, p.focus, p.extrude), (1.0, 1.0, 1.0), "already at the final angle");
        if i >= REDUCED_FADE_STEPS {
            assert_eq!(p.world_mix, 1.0);
        }
        s.step();
    }
    s.trigger();
    run(&mut s, REDUCED_FADE_STEPS);
    assert_eq!(s.params(true).world_mix, 0.0, "fade back done after 0.8 s");
}

#[test]
fn params_always_in_bounds() {
    for reduced in [false, true] {
        let mut s = Shift::new(World::Past);
        for round in 0..2 {
            s.trigger();
            for _ in 0..(TO_PAST_STEPS + NOSTALGIA_STEPS + 5) {
                let p = s.params(reduced);
                for v in [p.world_mix, p.glitch, p.extrude, p.pivot, p.focus, p.crt, p.warmth] {
                    assert!(v.is_finite() && (0.0..=1.0).contains(&v), "round {round}: {p:?}");
                }
                s.step();
            }
        }
    }
}
