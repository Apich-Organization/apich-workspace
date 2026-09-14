use slide_core::model::Slide;
use slide_core::model::SlideDeck;
use slide_core::package::pack_deck_to_file;
use slide_core::package::read_package_metadata;
use slide_core::package::unpack_deck_from_file;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_viewer_help_and_version() {
    let bin = env!("CARGO_BIN_EXE_slide-viewer");

    let output = Command::new(bin)
        .arg("--help")
        .output()
        .expect("slide-viewer --help");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Universal standalone presentation player"));
    assert!(stdout.contains("--animation"));
    assert!(stdout.contains("--fullscreen"));

    let ver_output = Command::new(bin)
        .arg("--version")
        .output()
        .expect("slide-viewer --version");
    assert!(ver_output.status.success());
}

#[test]
fn test_viewer_lzma2_package_roundtrip() {
    let dir = tempdir().expect("tempdir");
    let package_path = dir.path().join("presentation.slide");

    let mut deck = SlideDeck::new("Viewer Test Deck");
    deck.slides.push(Slide {
        page_number: 1,
        svg_data:
            r##"<svg viewBox="0 0 100 100"><circle cx="50" cy="50" r="40" fill="cyan"/></svg>"##
                .to_string(),
        view_box: slide_core::model::Rect::new(0.0, 0.0, 100.0, 100.0),
        hotspots: vec![],
        animation: Some("fade".to_string()),
        steps: vec![],
    });

    // Pack using LZMA2 extreme
    pack_deck_to_file(&deck, &package_path).expect("Pack deck");
    assert!(package_path.exists());

    // Read metadata
    let meta = read_package_metadata(&package_path).expect("Read metadata");
    assert_eq!(meta.title, "Viewer Test Deck");
    assert_eq!(meta.total_slides, 1);
    assert_eq!(meta.compression, "lzma2-max");

    // Unpack deck
    let unpacked = unpack_deck_from_file(&package_path).expect("Unpack deck");
    assert_eq!(unpacked.title, "Viewer Test Deck");
    assert_eq!(unpacked.slides.len(), 1);
    assert!(unpacked.slides[0].svg_data.contains("cyan"));
}
