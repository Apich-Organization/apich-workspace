use clap::Parser;
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "cargo-slide",
    bin_name = "cargo-slide",
    version,
    about = "Modern code-driven presentation system in Rust + Typst"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Path to typst file when run directly (default: slides.typ)
    #[arg(global = true)]
    pub file: Option<PathBuf>,

    /// Log format: human (default) or json
    #[arg(long, global = true, default_value = "human")]
    pub log_format: String,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize a new presentation project in current or specified directory
    Init {
        /// Target directory path (default: current directory ".")
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Include Rust Cargo package with main.rs for custom trait extensions
        #[arg(long)]
        rust: bool,
    },
    /// Create a new presentation project directory
    New {
        /// Name of the presentation project
        name: String,

        /// Include Rust Cargo package with main.rs for custom trait extensions
        #[arg(long)]
        rust: bool,
    },
    /// Run presentation with interactive GUI player
    Run {
        /// Path to .typ slide file (default: slides.typ)
        #[arg(default_value = "slides.typ")]
        file: PathBuf,

        /// Default transition animation (fade, cut, slide-left, slide-right, particles, zoom)
        #[arg(short, long, default_value = "fade")]
        animation: String,

        /// Watch file for changes and re-render
        #[arg(short, long)]
        watch: bool,

        /// Start directly in fullscreen mode
        #[arg(long)]
        fullscreen: bool,
    },
    /// Alias for run --watch
    Dev {
        /// Path to .typ slide file (default: slides.typ)
        #[arg(default_value = "slides.typ")]
        file: PathBuf,

        /// Default transition animation
        #[arg(short, long, default_value = "fade")]
        animation: String,

        /// Start directly in fullscreen mode
        #[arg(long)]
        fullscreen: bool,
    },
    /// Build a standalone self-contained single binary (.exe / ELF)
    Build {
        /// Path to .typ slide file (default: slides.typ)
        #[arg(default_value = "slides.typ")]
        file: PathBuf,

        /// Output binary path (default: `<filename>-presentation`)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Default transition animation
        #[arg(short, long, default_value = "fade")]
        animation: String,

        /// Cross-compile for a different Rust target triple (e.g.
        /// `x86_64-pc-windows-msvc`, `aarch64-unknown-linux-musl`) instead of the host's own
        /// platform. The target must already be installed (`rustup target add <triple>`) and,
        /// for a non-host target, have a working linker configured (see `.cargo/config.toml`).
        #[arg(long)]
        target: Option<String>,
    },
    /// Export presentation to PDF or SVGs
    Export {
        /// Path to .typ slide file (default: slides.typ)
        #[arg(default_value = "slides.typ")]
        file: PathBuf,

        /// Export format (pdf, svg)
        #[arg(short, long, default_value = "pdf")]
        format: String,

        /// Output file or directory path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}
