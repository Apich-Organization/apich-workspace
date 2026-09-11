use slide_core::model::Hotspot;
use slide_core::model::Rect;
use slide_core::svg::parse_svg_slide;

#[test]
fn test_parse_svg_view_box() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1600 900" width="1600" height="900"></svg>"#;
    let info = parse_svg_slide(svg).expect("Failed to parse svg");
    assert_eq!(info.view_box, Rect::new(0.0, 0.0, 1600.0, 900.0));
}

#[test]
fn test_parse_hyperlinks_and_video() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 800 600">
        <g>
            <a href="https://rust-lang.org" transform="translate(100, 50)">
                <rect x="0" y="0" width="200" height="40" fill="none" />
            </a>
            <a href="video:assets/demo.mp4" transform="translate(300, 200)">
                <rect x="10" y="20" width="400" height="250" fill="none" />
            </a>
            <a xlink:href="#page=3" transform="translate(50, 500)">
                <rect x="0" y="0" width="80" height="30" fill="none" />
            </a>
        </g>
    </svg>"##;

    let info = parse_svg_slide(svg).expect("Failed to parse svg");
    assert_eq!(info.hotspots.len(), 3);

    // 1. Web link
    match &info.hotspots[0] {
        | Hotspot::Link { target, rect } => {
            assert_eq!(target, "https://rust-lang.org");
            assert_eq!(rect.x, 100.0);
            assert_eq!(rect.y, 50.0);
            assert_eq!(rect.width, 200.0);
            assert_eq!(rect.height, 40.0);
            assert!(rect.contains(150.0, 70.0));
            assert!(!rect.contains(50.0, 50.0));
        },
        | _ => panic!("Expected Hotspot::Link"),
    }

    // 2. Video placeholder
    match &info.hotspots[1] {
        | Hotspot::Video { source, rect, .. } => {
            assert_eq!(source, "assets/demo.mp4");
            assert_eq!(rect.x, 310.0);
            assert_eq!(rect.y, 220.0);
            assert_eq!(rect.width, 400.0);
            assert_eq!(rect.height, 250.0);
            assert!(rect.contains(400.0, 300.0));
        },
        | _ => panic!("Expected Hotspot::Video"),
    }

    // 3. Internal slide jump
    match &info.hotspots[2] {
        | Hotspot::Link { target, rect } => {
            assert_eq!(target, "#page=3");
            assert_eq!(rect.x, 50.0);
            assert_eq!(rect.y, 500.0);
        },
        | _ => panic!("Expected Hotspot::Link"),
    }
}

#[test]
fn test_parse_audio_hotspot() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 800 600">
        <g>
            <a href="audio:ambient.mp3?autoplay=true&amp;loop=true&amp;vol=0.75" transform="translate(50, 60)">
                <rect x="0" y="0" width="300" height="80" fill="none" />
            </a>
        </g>
    </svg>"##;

    let info = parse_svg_slide(svg).expect("Failed to parse svg");
    assert_eq!(info.hotspots.len(), 1);

    match &info.hotspots[0] {
        | Hotspot::Audio {
            source,
            rect,
            autoplay,
            loop_audio,
            volume,
            ..
        } => {
            assert_eq!(source, "ambient.mp3");
            assert_eq!(rect.x, 50.0);
            assert_eq!(rect.y, 60.0);
            assert_eq!(rect.width, 300.0);
            assert_eq!(rect.height, 80.0);
            assert!(*autoplay);
            assert!(*loop_audio);
            assert!((volume - 0.75).abs() < 0.001);
        },
        | _ => panic!("Expected Hotspot::Audio"),
    }
}

#[test]
fn test_nested_ancestor_transforms_and_step_fragments() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 800 600">
        <g transform="translate(100, 50)">
            <g transform="scale(2)">
                <a href="step:1?effect=slide-up" transform="translate(10, 10)">
                    <rect x="0" y="0" width="50" height="20" fill="none" />
                </a>
                <a href="transition:glitch">
                    <rect x="0" y="0" width="0" height="0" />
                </a>
            </g>
        </g>
    </svg>"##;

    let info = parse_svg_slide(svg).expect("Failed to parse svg");
    assert_eq!(info.transition.as_deref(), Some("glitch"));
    assert_eq!(info.steps.len(), 1);

    let step = &info.steps[0];
    assert_eq!(step.order, 1);
    assert_eq!(step.effect, "slide-up");
    // (100 + 2 * 10) = 120, (50 + 2 * 10) = 70, width = 50 * 2 = 100, height = 20 * 2 = 40
    assert!((step.rect.x - 120.0).abs() < 0.01);
    assert!((step.rect.y - 70.0).abs() < 0.01);
    assert!((step.rect.width - 100.0).abs() < 0.01);
    assert!((step.rect.height - 40.0).abs() < 0.01);
}

#[test]
fn test_parse_chart_hotspot_with_anchor_and_escaped_entities() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 800 600">
        <a href="#chart:type=bar&amp;title=Speed+%26+Efficiency&amp;categories=Cat1,Cat2&amp;series=Series+A:10,20;Series+B:30,40">
            <rect x="50" y="50" width="500" height="300" fill="none" />
        </a>
    </svg>"##;

    let info = parse_svg_slide(svg).expect("Failed to parse svg");
    assert_eq!(info.hotspots.len(), 1);

    match &info.hotspots[0] {
        | slide_core::model::Hotspot::Chart { rect, data } => {
            assert!((rect.x - 50.0).abs() < 0.01);
            assert!((rect.y - 50.0).abs() < 0.01);
            assert_eq!(data.title.as_deref(), Some("Speed & Efficiency"));
            assert_eq!(data.categories, vec!["Cat1", "Cat2"]);
            assert_eq!(data.series.len(), 2);
            assert_eq!(data.series[0].name, "Series A");
            assert_eq!(data.series[0].values, vec![10.0, 20.0]);
            assert_eq!(data.series[1].name, "Series B");
            assert_eq!(data.series[1].values, vec![30.0, 40.0]);
        },
        | _ => panic!("Expected Hotspot::Chart"),
    }
}
