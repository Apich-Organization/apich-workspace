//! # `slide-core`
//!
//! Core data models, Typst compiler bridge, SVG parser, chart & SQL processing,
//! animation traits, and structured logging for `cargo-slide`.
//!
//! ## Modules
//! - [`model`]: Presentation models (`Slide`, `SlideDeck`, `Hotspot`, `StepFragment`).
//! - [`compiler`]: Typst CLI invocation, slide overflow detection, and PDF compilation.
//! - [`svg`]: SVG DOM parsing, link hotspot extraction, and vector normalization.
//! - [`chart`]: CSV/JSON/SQLite data querying, in-memory SQL execution, and chart models.
//! - [`animation`]: `SlideAnimation` and `ComponentAnimation` traits and easing functions.
//! - [`logger`]: Human-readable and structured JSON event logging.
//! - [`error`]: Unified error types and result aliases.

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

#[allow(
    missing_docs,
    clippy::pedantic,
    clippy::nursery,
    clippy::single_call_fn,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::unwrap_used,
    clippy::expect_used
)]
pub mod animation;
#[allow(
    missing_docs,
    clippy::pedantic,
    clippy::nursery,
    clippy::single_call_fn,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::unwrap_used,
    clippy::expect_used
)]
pub mod chart;
#[allow(
    missing_docs,
    clippy::pedantic,
    clippy::nursery,
    clippy::single_call_fn,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::unwrap_used,
    clippy::expect_used
)]
pub mod compiler;
#[allow(
    missing_docs,
    clippy::pedantic,
    clippy::nursery,
    clippy::single_call_fn,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::unwrap_used,
    clippy::expect_used
)]
pub mod error;
#[allow(
    missing_docs,
    clippy::pedantic,
    clippy::nursery,
    clippy::single_call_fn,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::unwrap_used,
    clippy::expect_used
)]
pub mod logger;
#[allow(
    missing_docs,
    clippy::pedantic,
    clippy::nursery,
    clippy::single_call_fn,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::unwrap_used,
    clippy::expect_used
)]
pub mod model;
#[allow(
    missing_docs,
    clippy::pedantic,
    clippy::nursery,
    clippy::single_call_fn,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::unwrap_used,
    clippy::expect_used
)]
pub mod svg;
#[allow(
    missing_docs,
    clippy::pedantic,
    clippy::nursery,
    clippy::single_call_fn,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::unwrap_used,
    clippy::expect_used
)]
pub mod watcher;
