//! # `cargo-slide`
//!
//! CLI tool for initializing, running, developing, building, and exporting
//! presentations powered by Typst and native Rust player engine.

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

mod cli;
mod commands;

use clap::Parser;
use cli::Cli;
use cli::Commands;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Vec<String> = std::env::args().collect();
    // Handle Cargo subcommand invocation (when called via `cargo slide ...`)
    if args.len() > 1 && args[1] == "slide" {
        args.remove(1);
    }

    let cli = Cli::parse_from(args);

    // Initialize global log format
    let log_format = slide_core::logger::LogFormat::from_str(&cli.log_format);
    slide_core::logger::init_logger(log_format);

    match cli.command {
        | Some(Commands::Init { path, rust }) => {
            commands::init::execute(&path, rust)?;
        },
        | Some(Commands::New { name, rust }) => {
            commands::new::execute(&name, rust)?;
        },
        | Some(Commands::Run {
            file,
            animation,
            watch,
            fullscreen,
        }) => {
            commands::run::execute(&file, &animation, watch, fullscreen)?;
        },
        | Some(Commands::Dev {
            file,
            animation,
            fullscreen,
        }) => {
            commands::run::execute(&file, &animation, true, fullscreen)?;
        },
        | Some(Commands::Build {
            file,
            output,
            animation,
            target,
        }) => {
            commands::build::execute(&file, output, &animation, target.as_deref())?;
        },
        | Some(Commands::Export { file, format, output }) => {
            commands::export::execute(&file, &format, output)?;
        },
        | None => {
            let file = cli.file.unwrap_or_else(|| PathBuf::from("slides.typ"));
            commands::run::execute(&file, "fade", false, false)?;
        },
    }

    Ok(())
}
