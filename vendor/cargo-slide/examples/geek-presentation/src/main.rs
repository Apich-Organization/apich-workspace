//! # cargo-slide Custom Rust Extensions Showcase
//!
//! > **NOTE**: Writing Rust code is **100% OPTIONAL** when authoring presentations!
//! > Normal presentations only need a `slides.typ` file and can be run via:
//! > `cargo slide run slides.typ` or built via `cargo slide build slides.typ`.
//!
//! This file demonstrates how advanced Rust developers can embed `cargo-slide`
//! as a library and register **custom Rust traits**:
//! 1. Custom Page Transitions implementing `SlideAnimation` (`curtain-split`, `spiral`)
//! 2. Custom In-Slide Component Animations implementing `ComponentAnimation` (`typewriter-block`)

// -------------------------------------------------------------------------
// LEVEL 1: CRITICAL ERRORS (Deny)
// -------------------------------------------------------------------------
#![deny(
    // Rust Compiler Errors
    unreachable_code,
    improper_ctypes_definitions,
    future_incompatible,
    nonstandard_style,
    rust_2018_idioms,
    clippy::perf,
    clippy::correctness,
    clippy::suspicious,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::missing_safety_doc,
    clippy::same_item_push,
    clippy::implicit_clone,
    clippy::all,
    clippy::pedantic,
    missing_docs,
    clippy::nursery,
    clippy::single_call_fn,
)]
// -------------------------------------------------------------------------
// LEVEL 2: STYLE WARNINGS (Warn)
// -------------------------------------------------------------------------
#![warn(
    // For `no-std` Situation Issues
    dead_code,
    warnings,
    unsafe_code,
    clippy::dbg_macro,
    clippy::todo,
    clippy::unnecessary_safety_comment
)]
// -------------------------------------------------------------------------
// LEVEL 3: ALLOW/IGNORABLE (Allow)
// -------------------------------------------------------------------------
#![allow(
    clippy::restriction,
    clippy::inline_always,
    unused_doc_comments,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::empty_line_after_doc_comments,
    clippy::cast_precision_loss,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::suboptimal_flops
)]

use slide_player::prelude::*;
use std::time::Duration;

/// Custom Page Transition: "curtain-split"
/// Splits the outgoing slide vertically and slides both halves outward to reveal the new slide.
pub struct CurtainSplitTransition {
    duration_ms: u64,
}

impl CurtainSplitTransition {
    /// Create a new curtain split transition
    #[must_use]
    pub const fn new(duration_ms: u64) -> Self {
        Self { duration_ms }
    }
}

impl SlideAnimation for CurtainSplitTransition {
    fn name(&self) -> &'static str {
        "curtain-split"
    }

    fn duration(&self) -> Duration {
        Duration::from_millis(self.duration_ms)
    }

    fn render(
        &self,
        ctx: &mut RenderContext<'_>,
        from: Option<&SlideSurface>,
        to: &SlideSurface,
        progress: f32,
    ) {
        let t = smooth_step(progress);
        let mid_x = ctx.width / 2;
        let curtain_shift = (t * mid_x as f32).round() as usize;

        // Base target slide is revealed underneath
        ctx.copy_from(to);

        if let Some(old_slide) = from {
            for y in 0..ctx.height {
                // Left curtain half sliding left
                for x in 0..mid_x {
                    if x >= curtain_shift {
                        let src_x = x - curtain_shift;
                        let pixel = old_slide.get_pixel(src_x, y);
                        ctx.set_pixel(x, y, pixel);
                    }
                }
                // Right curtain half sliding right
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

/// Custom Page Transition: "spiral"
/// Rotational angular wave reveal around screen center
pub struct CustomSpiralAnimation;

impl SlideAnimation for CustomSpiralAnimation {
    fn name(&self) -> &'static str {
        "spiral"
    }

    fn duration(&self) -> Duration {
        Duration::from_millis(600)
    }

    fn render(
        &self,
        ctx: &mut RenderContext<'_>,
        from: Option<&SlideSurface>,
        to: &SlideSurface,
        progress: f32,
    ) {
        let t = progress.clamp(0.0, 1.0);
        let width = ctx.width;
        let height = ctx.height;
        let cx = width as f32 / 2.0;
        let cy = height as f32 / 2.0;
        let max_r = cx.max(cy);

        for y in 0..height {
            let dy = y as f32 - cy;
            let row_start = y * width;
            for x in 0..width {
                let dx = x as f32 - cx;
                let angle = (dy.atan2(dx) + std::f32::consts::PI) / (2.0 * std::f32::consts::PI);
                let radius = dx.hypot(dy) / max_r;
                let spiral_val = (angle + radius * 2.0).fract();

                let pixel = if t >= spiral_val {
                    to.get_pixel(x, y)
                } else if let Some(from_surf) = from {
                    from_surf.get_pixel(x, y)
                } else {
                    0
                };
                ctx.buffer[row_start + x] = pixel;
            }
        }
    }
}

/// Custom In-Slide Component Animation: "typewriter-block"
/// Smoothly reveals an in-slide fragment component from left to right like a typewriter wipe.
pub struct TypewriterRevealAnimation;

impl ComponentAnimation for TypewriterRevealAnimation {
    fn name(&self) -> &'static str {
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Launching presentation with custom Rust animation extensions...");

    SlideApp::new("slides.typ")
        .default_animation("curtain-split")
        .register_transition(CurtainSplitTransition::new(550))
        .register_transition(CustomSpiralAnimation)
        .register_component_animation(TypewriterRevealAnimation)
        .run()?;

    Ok(())
}
