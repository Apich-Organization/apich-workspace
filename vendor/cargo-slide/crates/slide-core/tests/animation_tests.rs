use slide_core::animation::AnimationRegistry;
use slide_core::animation::RenderContext;
use slide_core::animation::SlideAnimation;
use slide_core::animation::SlideSurface;
use std::time::Duration;

struct CustomTestTransition;
impl SlideAnimation for CustomTestTransition {
    fn name(&self) -> &str {
        "test-custom"
    }

    fn duration(&self) -> Duration {
        Duration::from_millis(300)
    }

    fn render(
        &self,
        ctx: &mut RenderContext,
        _from: Option<&SlideSurface>,
        to: &SlideSurface,
        progress: f32,
    ) {
        if progress >= 0.5 {
            ctx.buffer.copy_from_slice(&to.pixels);
        } else {
            ctx.clear(0xFF112233);
        }
    }
}

#[test]
fn test_animation_registry() {
    let mut registry = AnimationRegistry::default();

    // Check built-in page animations
    assert!(registry.get("fade").is_some());
    assert!(registry.get("cut").is_some());
    assert!(registry.get("slide-left").is_some());
    assert!(registry.get("slide-right").is_some());
    assert!(registry.get("slide-up").is_some());
    assert!(registry.get("slide-down").is_some());
    assert!(registry.get("wipe-left").is_some());
    assert!(registry.get("wipe-right").is_some());
    assert!(registry.get("iris").is_some());
    assert!(registry.get("glitch").is_some());
    assert!(registry.get("cube").is_some());
    assert!(registry.get("particles").is_some());
    assert!(registry.get("zoom").is_some());

    // Check built-in component animations
    assert!(registry.get_component("fade-in").is_some());
    assert!(registry.get_component("slide-up").is_some());
    assert!(registry.get_component("zoom-in").is_some());
    assert!(registry.get_component("glitch").is_some());
    assert!(registry.get_component("wipe").is_some());

    // Register and test custom animation
    registry.register(CustomTestTransition);
    let custom = registry
        .get("test-custom")
        .expect("Custom animation should be registered");
    assert_eq!(custom.name(), "test-custom");
    assert_eq!(custom.duration(), Duration::from_millis(300));
}

#[test]
fn test_transition_rendering() {
    let width = 10;
    let height = 10;

    let from = SlideSurface::blank(width, height, 0xFF000000);
    let to = SlideSurface::blank(width, height, 0xFFFFFFFF);

    let mut output = vec![0u32; width * height];
    let registry = AnimationRegistry::default();
    let fade = registry.get("fade").unwrap();

    // Progress 0.0 -> mostly black
    {
        let mut ctx = RenderContext::new(width, height, &mut output);
        fade.render(&mut ctx, Some(&from), &to, 0.0);
    }
    assert_eq!(output[0], 0xFF000000);

    // Progress 1.0 -> full white
    {
        let mut ctx = RenderContext::new(width, height, &mut output);
        fade.render(&mut ctx, Some(&from), &to, 1.0);
    }
    assert_eq!(output[0], 0xFFFFFFFF);
}
