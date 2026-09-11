use slide_player::prelude::*;
use std::time::Duration;

/// A custom external slide transition: "curtain-split"
/// Splits the screen in the middle and slides both left and right curtains outward.
pub struct CurtainSplitTransition {
    duration_ms: u64,
}

impl CurtainSplitTransition {
    pub fn new(duration_ms: u64) -> Self {
        Self { duration_ms }
    }
}

impl SlideAnimation for CurtainSplitTransition {
    fn name(&self) -> &str {
        "curtain-split"
    }

    fn duration(&self) -> Duration {
        Duration::from_millis(self.duration_ms)
    }

    fn render(
        &self,
        ctx: &mut RenderContext,
        from: Option<&SlideSurface>,
        to: &SlideSurface,
        progress: f32,
    ) {
        let t = smooth_step(progress);
        let mid_x = ctx.width / 2;
        let curtain_shift = (t * mid_x as f32).round() as usize;

        // Background is target slide
        ctx.copy_from(to);

        // Draw left curtain from old slide shifting left
        if let Some(old_slide) = from {
            for y in 0..ctx.height {
                // Left half
                for x in 0..mid_x {
                    if x >= curtain_shift {
                        let src_x = x - curtain_shift;
                        let pixel = old_slide.get_pixel(src_x, y);
                        ctx.set_pixel(x, y, pixel);
                    }
                }
                // Right half
                for x in mid_x..ctx.width {
                    let shifted_x = x + curtain_shift;
                    if shifted_x < ctx.width {
                        let pixel = old_slide.get_pixel(shifted_x, y);
                        ctx.set_pixel(x, y, pixel);
                    }
                }
            }
        }
    }
}

/// A custom external in-slide component reveal animation: "typewriter-block"
/// Reveals a component horizontally like a typewriter, wiping left to right.
pub struct TypewriterRevealAnimation;

impl ComponentAnimation for TypewriterRevealAnimation {
    fn name(&self) -> &str {
        "typewriter-block"
    }

    fn duration(&self) -> Duration {
        Duration::from_millis(350)
    }

    fn render(
        &self,
        progress: f32,
        rect: Rect,
        full_slide: &SlideSurface,
        output: &mut [u32],
        width: usize,
        height: usize,
        bg_color: u32,
    ) {
        let t = progress.clamp(0.0, 1.0);
        let x1 = (rect.x.max(0.0) as usize).min(width);
        let y1 = (rect.y.max(0.0) as usize).min(height);
        let x2 = ((rect.x + rect.width).max(0.0) as usize).min(width);
        let y2 = ((rect.y + rect.height).max(0.0) as usize).min(height);

        let reveal_cutoff = x1 + ((x2 - x1) as f32 * t).round() as usize;

        for y in y1..y2 {
            let row = y * width;
            for x in x1..x2 {
                if x <= reveal_cutoff {
                    output[row + x] = full_slide.get_pixel(x, y);
                } else {
                    output[row + x] = bg_color;
                }
            }
        }
    }
}

#[test]
fn test_custom_slide_animation_registration_and_rendering() {
    let mut registry = AnimationRegistry::default();

    // Verify built-in animations exist
    assert!(registry.get("fade").is_some());
    assert!(registry.get("cube").is_some());

    // Register external custom transition
    let custom_trans = CurtainSplitTransition::new(450);
    registry.register(custom_trans);

    let retrieved = registry.get("curtain-split");
    assert!(retrieved.is_some());
    let anim = retrieved.unwrap();
    assert_eq!(anim.name(), "curtain-split");
    assert_eq!(anim.duration(), Duration::from_millis(450));

    // Test frame rendering with surfaces
    let w = 100usize;
    let h = 50usize;
    let surf_from = SlideSurface::blank(w, h, 0xFF112233);
    let surf_to = SlideSurface::blank(w, h, 0xFFAABBCC);
    let mut buffer = vec![0u32; w * h];

    // Frame at progress 0.0 (old slide visible)
    {
        let mut ctx = RenderContext::new(w, h, &mut buffer);
        anim.render(&mut ctx, Some(&surf_from), &surf_to, 0.0);
    }
    assert_eq!(buffer[0], 0xFF112233);

    // Frame at progress 1.0 (new slide fully revealed)
    {
        let mut ctx = RenderContext::new(w, h, &mut buffer);
        anim.render(&mut ctx, Some(&surf_from), &surf_to, 1.0);
    }
    assert_eq!(buffer[0], 0xFFAABBCC);
}

#[test]
fn test_custom_component_animation_registration_and_rendering() {
    let mut registry = AnimationRegistry::default();

    // Register external custom component animation
    registry.register_component(TypewriterRevealAnimation);

    let retrieved = registry.get_component("typewriter-block");
    assert!(retrieved.is_some());
    let comp_anim = retrieved.unwrap();
    assert_eq!(comp_anim.name(), "typewriter-block");
    assert_eq!(comp_anim.duration(), Duration::from_millis(350));

    let w = 80usize;
    let h = 40usize;
    let full_slide = SlideSurface::blank(w, h, 0xFF00FF00); // Green text/component
    let mut buffer = vec![0xFF000000u32; w * h]; // Black background
    let rect = Rect::new(10.0, 10.0, 40.0, 20.0);
    let bg_color = 0xFF000000u32;

    // Render at progress 0.5 (half revealed horizontally)
    comp_anim.render(0.5, rect, &full_slide, &mut buffer, w, h, bg_color);

    // Coordinate (15, 15) is within the first half: should be revealed (green)
    assert_eq!(buffer[15 * w + 15], 0xFF00FF00);

    // Coordinate (45, 15) is in the second half: should be masked (black)
    assert_eq!(buffer[15 * w + 45], 0xFF000000);
}

#[test]
fn test_slide_app_builder_registration() {
    let mut deck = SlideDeck::new("Custom Animation Test Deck");
    deck.slides.push(Slide {
        page_number: 1,
        svg_data: "<svg></svg>".to_string(),
        view_box: Rect::new(0.0, 0.0, 100.0, 100.0),
        hotspots: Vec::new(),
        animation: Some("curtain-split".to_string()),
        steps: Vec::new(),
    });

    // Verify builder chaining with external animations
    let _app = SlideApp::from_deck(deck)
        .title("Test App")
        .register_transition(CurtainSplitTransition::new(500))
        .register_component_animation(TypewriterRevealAnimation);
}

#[test]
fn test_cjk_and_accessibility_svg_rasterization() {
    use slide_player::renderer::SvgRenderer;

    // Typst converts all typography and CJK glyphs into vector <path> curves in <defs>
    let vector_svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 200" width="400" height="200">
        <rect width="400" height="200" fill="#0f111a"/>
        <path d="M 50,50 L 150,50 L 150,150 L 50,150 Z" fill="#39d353"/>
        <circle cx="250" cy="100" r="40" fill="#58a6ff"/>
    </svg>"##;

    let renderer = SvgRenderer::new();
    let (surface, metrics) = renderer
        .render_svg(vector_svg, 800, 400)
        .expect("render vector svg");
    assert_eq!(surface.width, 800);
    assert_eq!(surface.height, 400);
    assert!(metrics.scale > 0.0);

    let has_non_bg = surface
        .pixels
        .iter()
        .any(|&p| p != 0xFF000000 && p != 0xFF0F111A);
    assert!(
        has_non_bg,
        "Expected vector curves to render visible pixels"
    );
}

#[test]
fn test_svg_renderer_viewbox_metrics() {
    use slide_core::model::Rect;
    use slide_player::renderer::SvgRenderer;

    let svg_str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="50 20 500 300" width="500" height="300">
        <rect x="50" y="20" width="500" height="300" fill="#112233"/>
        <rect x="100" y="60" width="200" height="100" fill="#ff0000"/>
    </svg>"##;

    let renderer = SvgRenderer::new();
    let (surface, metrics) = renderer.render_svg(svg_str, 1000, 600).expect("render svg");
    assert_eq!(metrics.view_box_x, 50.0);
    assert_eq!(metrics.view_box_y, 20.0);
    assert_eq!(metrics.scale, 2.0);
    assert_eq!(metrics.offset_x, 0.0);
    assert_eq!(metrics.offset_y, 0.0);

    let red_rect = Rect::new(100.0, 60.0, 200.0, 100.0);
    let screen_rect = metrics.svg_to_screen_rect(&red_rect);
    // x: 0.0 + (100.0 - 50.0) * 2.0 = 100.0
    // y: 0.0 + (60.0 - 20.0) * 2.0 = 80.0
    // w: 400.0, h: 200.0
    assert_eq!(screen_rect.x, 100.0);
    assert_eq!(screen_rect.y, 80.0);
    assert_eq!(screen_rect.width, 400.0);
    assert_eq!(screen_rect.height, 200.0);

    // Verify pixel at center of red rect is indeed red (0xFFFF0000)
    let p_center = surface.get_pixel(300, 180);
    assert_eq!(p_center, 0xFFFF0000);
}

#[test]
fn test_inspector_layout_clamping() {
    use slide_player::hud::get_inspector_layout;

    // Constrained small display (800x480)
    let (
        modal_rect,
        _title_rect,
        _toolbar,
        _type_bar,
        _trans_bar,
        _series_bar,
        left_pane,
        right_pane,
    ) = get_inspector_layout(800, 480);
    assert!(modal_rect.x >= 10.0);
    assert!(modal_rect.y >= 10.0);
    assert!(modal_rect.x + modal_rect.width <= 800.0);
    assert!(modal_rect.y + modal_rect.height <= 480.0);
    assert!(left_pane.x >= modal_rect.x);
    assert!(right_pane.x + right_pane.width <= modal_rect.x + modal_rect.width);
    assert!(right_pane.y + right_pane.height <= modal_rect.y + modal_rect.height);

    // Standard 1080p display (1920x1080)
    let (m1080, _, _, _, _, _, l1080, r1080) = get_inspector_layout(1920, 1080);
    assert!(m1080.x >= 10.0);
    assert!(m1080.y >= 10.0);
    assert!(m1080.x + m1080.width <= 1920.0);
    assert!(m1080.y + m1080.height <= 1080.0);
    assert!(l1080.x >= m1080.x);
    assert!(r1080.x + r1080.width <= m1080.x + m1080.width);
}

#[test]
fn test_slide_app_watch_configuration() {
    let app = SlideApp::new("slides.typ")
        .watch(true)
        .fullscreen(false)
        .default_animation("fade");
    let _ = app;
}
