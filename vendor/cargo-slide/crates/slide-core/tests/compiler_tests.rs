use slide_core::compiler::SlideCompiler;
use std::fs::write;
use tempfile::tempdir;

#[test]
fn test_compile_typst_deck() {
    let compiler = match SlideCompiler::new() {
        | Ok(c) => c,
        | Err(_) => {
            eprintln!("Typst not available, skipping test");
            return;
        },
    };

    let dir = tempdir().expect("tempdir");
    let typst_file = dir.path().join("test.typ");
    let content = r#"
#set page(width: 16cm, height: 9cm)
= Slide 1
Welcome to testing!
Visit #link("https://typst.app")[Typst].

#pagebreak()

= Slide 2
Second slide with math: $E = m c^2$.
"#;
    write(&typst_file, content).expect("write typst file");

    let deck = compiler
        .compile_file(&typst_file)
        .expect("Failed to compile typst deck");
    assert_eq!(deck.total_slides(), 2);
    assert_eq!(deck.slides[0].page_number, 1);
    assert_eq!(deck.slides[1].page_number, 2);
    assert!(!deck.slides[0].hotspots.is_empty());
}

#[test]
fn test_detect_slide_overflow() {
    use slide_core::error::SlideError;

    let compiler = match SlideCompiler::new() {
        | Ok(c) => c,
        | Err(_) => {
            eprintln!("Typst not available, skipping test");
            return;
        },
    };

    let dir = tempdir().expect("tempdir");
    let typst_file = dir.path().join("overflow.typ");
    // Define a 1-slide deck where huge content overflows the 5cm page
    let content = r#"
#set page(width: 10cm, height: 5cm)
#let title-slide(title: "") = {
  heading(title)
}
#title-slide(title: "Overflowing Slide")
#v(10cm)
This content definitely overflows the 5cm page bounds!
"#;
    write(&typst_file, content).expect("write overflow typst file");

    let res = compiler.compile_file(&typst_file);
    assert!(
        res.is_err(),
        "Expected compilation to fail with SlideError::Overflow"
    );
    match res.err().unwrap() {
        | SlideError::Overflow(msg) => {
            assert!(msg.contains("Typst slide content overflow detected"));
            assert!(msg.contains("Overflowing Slide"));
        },
        | other => panic!("Expected SlideError::Overflow, got {:?}", other),
    }
}

#[test]
fn test_multilingual_cjk_and_unicode_rendering() {
    let compiler = match SlideCompiler::new() {
        | Ok(c) => c,
        | Err(_) => {
            eprintln!("Typst not available, skipping test");
            return;
        },
    };

    let dir = tempdir().expect("tempdir");
    let typst_file = dir.path().join("cjk.typ");
    let content = r#"
#set page(width: 16cm, height: 9cm)
= 多语言与国际化排版测试
这是一个完全支持中文、日本語、한국어 的原生测试。
- 中文测试：你好，世界！向量图形渲染流畅无畸变。
- 日本語テスト：こんにちは世界！ルビ表記や約物。
- 한국어 테스트: 안녕하세요 세계! 유니코드 서체.
- Unicode & Emojis: 🚀 📊 🎨 ⚛️ ∑ ∇ ∫
- Latin Accents: Café, Über, façade, Español.
"#;
    write(&typst_file, content).expect("write typst file");

    let deck = compiler
        .compile_file(&typst_file)
        .expect("Failed to compile CJK typst deck");
    assert_eq!(deck.total_slides(), 1);
    assert!(!deck.slides[0].svg_data.is_empty());
    assert!(deck.slides[0].svg_data.contains("<svg"));

    let info = slide_core::svg::parse_svg_slide(&deck.slides[0].svg_data).expect("parse svg slide");
    assert!(info.view_box.width > 0.0);
    assert!(info.view_box.height > 0.0);
}

#[test]
fn test_geek_presentation_hotspots() {
    let compiler = match SlideCompiler::new() {
        | Ok(c) => c,
        | Err(_) => return,
    };
    let path = std::path::Path::new("../../examples/geek-presentation/slides.typ");
    if !path.exists() {
        return;
    }
    let deck = compiler
        .compile_file(path)
        .expect("compile geek-presentation");
    assert!(deck.total_slides() > 0);
    for slide in &deck.slides {
        assert!(slide.view_box.width > 0.0);
        assert!(slide.view_box.height > 0.0);
        for hs in &slide.hotspots {
            let r = hs.rect();
            assert!(r.width > 0.0);
            assert!(r.height > 0.0);
        }
    }
}

#[test]
fn test_render_metrics_viewbox_coordinate_mapping() {
    use slide_core::model::Rect;
    use slide_core::model::RenderMetrics;

    // Test zero offset viewBox
    let metrics_zero = RenderMetrics {
        scale: 2.0,
        offset_x: 100.0,
        offset_y: 50.0,
        content_width: 800.0,
        content_height: 600.0,
        view_box_x: 0.0,
        view_box_y: 0.0,
    };

    let svg_rect = Rect::new(10.0, 20.0, 30.0, 40.0);
    let screen_rect = metrics_zero.svg_to_screen_rect(&svg_rect);
    assert_eq!(screen_rect.x, 120.0);
    assert_eq!(screen_rect.y, 90.0);
    assert_eq!(screen_rect.width, 60.0);
    assert_eq!(screen_rect.height, 80.0);

    let (round_x, round_y) = metrics_zero
        .screen_to_svg(120.0, 90.0)
        .expect("screen to svg");
    assert!((round_x - 10.0).abs() < 1e-4);
    assert!((round_y - 20.0).abs() < 1e-4);

    // Test non-zero viewBox offset (e.g. SVG viewBox="50 30 400 300")
    let metrics_shifted = RenderMetrics {
        scale: 1.5,
        offset_x: 40.0,
        offset_y: 20.0,
        content_width: 600.0,
        content_height: 450.0,
        view_box_x: 50.0,
        view_box_y: 30.0,
    };

    let rect2 = Rect::new(70.0, 50.0, 100.0, 80.0);
    let screen2 = metrics_shifted.svg_to_screen_rect(&rect2);
    assert!((screen2.x - 70.0).abs() < 1e-4);
    assert!((screen2.y - 50.0).abs() < 1e-4);
    assert!((screen2.width - 150.0).abs() < 1e-4);
    assert!((screen2.height - 120.0).abs() < 1e-4);

    // Round trip back to SVG
    let (back_x, back_y) = metrics_shifted
        .screen_to_svg(70.0, 50.0)
        .expect("screen to svg shifted");
    assert!((back_x - 70.0).abs() < 1e-4);
    assert!((back_y - 50.0).abs() < 1e-4);
}

#[test]
fn test_typst_warning_formatting() {
    let mock_stderr = "warning: unknown font family: segoe ui\n   ┌─ theme.typ:67:10\n   │\n67 │     font: active-fonts,\n   │           ^^^^^^^^^^^^\n\nwarning: unknown font family: sf pro display\n   ┌─ theme.typ:67:10\n   │\n67 │     font: active-fonts,\n   │           ^^^^^^^^^^^^\n\nwarning: heading is too long\n   ┌─ slides.typ:15:1\n   │\n15 │ = A very long heading\n   │ ^^^^^^^^^^^^^^^^^^^^^";

    let (formatted, has_font, missing) = slide_core::compiler::format_typst_warnings(mock_stderr);
    assert!(has_font);
    assert_eq!(missing, vec!["segoe ui", "sf pro display"]);
    assert!(formatted.contains("heading is too long"));
    assert!(formatted.contains("segoe ui, sf pro display"));
    assert!(formatted.contains("Typst automatically falls back to available system fonts"));
}
