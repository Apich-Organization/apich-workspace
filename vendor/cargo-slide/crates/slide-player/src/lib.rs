//! # `slide-player`
//!
//! Native 60 FPS presentation player engine, tiny-skia 32-bit ARGB software blitter,
//! borderless fullscreen & windowed modes, presenter interaction tools (laser trail, 7-color palette pen),
//! Rodio audio mixer, and live HUD chart data inspector.

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
    clippy::too_many_arguments,
    clippy::mixed_case_hex_literals,
    clippy::field_reassign_with_default,
    clippy::manual_c_str_literals,
    missing_docs,
    clippy::pedantic,
    clippy::nursery,
    clippy::single_call_fn,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::unwrap_used,
    clippy::expect_used
)]

pub mod audio;
pub mod hud;
pub mod media;
pub mod renderer;
pub mod transition;
pub mod window;

pub use audio::AudioEngine;
pub use media::MediaPlayer;
pub use renderer::RenderMetrics;
pub use renderer::SvgRenderer;
pub use transition::TransitionManager;
pub use window::PlayerConfig;
pub use window::SlideApp;
pub use window::SlidePlayer;

pub mod prelude {
    pub use crate::AudioEngine;
    pub use crate::PlayerConfig;
    pub use crate::SlideApp;
    pub use crate::SlidePlayer;
    pub use crate::TransitionManager;
    pub use slide_core::animation::AnimationRegistry;
    pub use slide_core::animation::ComponentAnimation;
    pub use slide_core::animation::RenderContext;
    pub use slide_core::animation::SlideAnimation;
    pub use slide_core::animation::SlideSurface;
    pub use slide_core::animation::SlideTransition;
    pub use slide_core::animation::blend_pixel;
    pub use slide_core::animation::dim_pixel;
    pub use slide_core::animation::ease_in_out;
    pub use slide_core::animation::ease_out_back;
    pub use slide_core::animation::ease_out_cubic;
    pub use slide_core::animation::smooth_step;
    pub use slide_core::chart::ChartData;
    pub use slide_core::chart::ChartType;
    pub use slide_core::chart::SeriesData;
    pub use slide_core::compiler::SlideCompiler;
    pub use slide_core::error::Result as SlideResult;
    pub use slide_core::error::SlideError;
    pub use slide_core::model::Hotspot;
    pub use slide_core::model::Rect;
    pub use slide_core::model::RenderMetrics;
    pub use slide_core::model::Slide;
    pub use slide_core::model::SlideDeck;
    pub use slide_core::model::StepFragment;
}
