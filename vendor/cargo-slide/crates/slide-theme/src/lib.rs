//! Built-in Typst themes, templates, and slide macros

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
    clippy::empty_line_after_doc_comments
)]

/// Default Typst presentation theme source code
pub const DEFAULT_THEME: &str = include_str!("../typst/theme.typ");

/// Built-in slide component macros (#slide, #step, #chart, etc.)
pub const SLIDE_MACROS: &str = include_str!("../typst/slide.typ");

/// Default starter presentation template
pub const DEFAULT_PRESENTATION: &str = include_str!("../typst/template.typ");
