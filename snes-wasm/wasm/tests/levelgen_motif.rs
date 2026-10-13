use snes_wasm::levelgen::motif::{vocab, Category, Motif, Theme, ALL};

#[test]
fn vocabulary_fits_a_u64_mask_and_round_trips() {
    assert!(vocab::size() <= 64, "grammar masks are u64: {} tokens", vocab::size());
    for t in 0..vocab::size() as u32 {
        let name = vocab::token_name(t);
        assert_eq!(vocab::parse_token(&name), Some(t), "{name}");
    }
    assert_eq!(vocab::token_name(vocab::theme_token(Theme::Cave)), "<cave>");
    assert_eq!(vocab::token_name(vocab::diff_token(5)), "<d5>");
}

#[test]
fn motif_names_are_unique_and_parse_back() {
    for &m in ALL {
        assert_eq!(Motif::parse(&m.name()), Some(m));
    }
    let mut names: Vec<String> = ALL.iter().map(|m| m.name()).collect();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), ALL.len());
}

#[test]
fn categories_drive_the_intro_and_hazard_rules() {
    assert!(!Motif::Walkers(1).intro_ok() && Motif::Walkers(1).has_enemy());
    assert!(Motif::Gap(3).intro_ok() && !Motif::Gap(4).intro_ok());
    assert!(!Motif::Tunnel(5).intro_ok() && !Motif::RiftOut.intro_ok());
    assert!(Motif::Flat(3).intro_ok() && !Motif::Flat(3).is_hazard());
    assert_eq!(Motif::Checkpoint.category(), Category::Fixed);
    assert!(Motif::Marsh.is_hazard() && Motif::Climb.is_hazard());
    assert!(!Motif::RiftOut.is_hazard(), "rifts are calm (runway)");
}

#[test]
fn themes() {
    assert_eq!(Theme::Cave.roof_rows(), 5);
    assert_eq!(Theme::Meadow.roof_rows(), 0);
    for t in Theme::ALL {
        assert_eq!(Theme::parse(t.name()), Some(t));
    }
}
