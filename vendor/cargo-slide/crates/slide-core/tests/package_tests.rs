use slide_core::model::Slide;
use slide_core::model::SlideDeck;
use slide_core::package::pack_deck_to_file;
use slide_core::package::read_package_metadata;
use slide_core::package::unpack_deck_from_file;
use tempfile::tempdir;

#[test]
fn test_pack_and_unpack_lzma2() {
    let dir = tempdir().unwrap();
    let pkg_path = dir.path().join("presentation.slide");

    let mut deck = SlideDeck::new("LZMA2 Presentation Showcase");
    deck.default_animation = "zoom".to_string();
    let slide1 = Slide {
        page_number: 1,
        svg_data: r#"<svg viewBox="0 0 1920 1080"><text x="100" y="200">Slide 1</text></svg>"#
            .to_string(),
        view_box: slide_core::model::Rect::new(0.0, 0.0, 1920.0, 1080.0),
        hotspots: Vec::new(),
        animation: Some("zoom".to_string()),
        steps: Vec::new(),
    };
    let slide2 = Slide {
        page_number: 2,
        svg_data: r#"<svg viewBox="0 0 1920 1080"><text x="100" y="200">Slide 2 with large payload data</text></svg>"#.to_string(),
        view_box: slide_core::model::Rect::new(0.0, 0.0, 1920.0, 1080.0),
        hotspots: Vec::new(),
        animation: None,
        steps: Vec::new(),
    };
    deck.slides.push(slide1);
    deck.slides.push(slide2);

    // Pack to file using LZMA2 maximum compression
    pack_deck_to_file(&deck, &pkg_path).expect("Failed to pack deck with LZMA2");
    assert!(pkg_path.exists());
    assert!(pkg_path.metadata().unwrap().len() > 0);

    // Read metadata
    let meta = read_package_metadata(&pkg_path).expect("Failed to read metadata");
    assert_eq!(meta.title, "LZMA2 Presentation Showcase");
    assert_eq!(meta.total_slides, 2);
    assert_eq!(meta.default_animation, "zoom");

    // Unpack deck and verify contents
    let unpacked = unpack_deck_from_file(&pkg_path).expect("Failed to unpack deck");
    assert_eq!(unpacked.title, "LZMA2 Presentation Showcase");
    assert_eq!(unpacked.total_slides(), 2);
    assert_eq!(unpacked.slides[0].page_number, 1);
    assert_eq!(unpacked.slides[1].page_number, 2);
}
