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
    /// Build presentation into a standalone executable, a .slide package, or a WASM CSR web bundle
    Build {
        /// Path to .typ slide file (default: slides.typ)
        #[arg(default_value = "slides.typ")]
        file: PathBuf,

        /// Output path (default: `<filename>-presentation`, `<filename>.slide`, or `dist/`)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output format: binary (default standalone executable), slide (.slide package), wasm (Leptos CSR web bundle)
        #[arg(short, long, default_value = "binary")]
        format: String,

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
    /// Pack presentation into a standalone compressed .slide package (LZMA2)
    Pack {
        /// Path to .typ slide file (default: slides.typ)
        #[arg(default_value = "slides.typ")]
        file: PathBuf,

        /// Output .slide package file path (default: `<filename>.slide`)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Default transition animation
        #[arg(short, long, default_value = "fade")]
        animation: String,
    },
    /// Serve presentation via built-in static web server
    Serve {
        /// Path to .typ slide file, .slide package, or CSR directory (default: slides.typ)
        #[arg(default_value = "slides.typ")]
        file: PathBuf,

        /// Port to bind the static server to (default: 8080)
        #[arg(short, long, default_value_t = 8080)]
        port: u16,

        /// IP address to bind the static server to (default: 127.0.0.1)
        #[arg(long, default_value = "127.0.0.1")]
        ip: String,

        /// Optional directory of an already built Leptos CSR package to serve directly
        #[arg(short, long)]
        dir: Option<PathBuf>,

        /// Open presentation in default web browser
        #[arg(long)]
        open: bool,
    },
    /// Export presentation to PDF, SVG, .slide, or WASM CSR
    Export {
        /// Path to .typ slide file (default: slides.typ)
        #[arg(default_value = "slides.typ")]
        file: PathBuf,

        /// Export format (pdf, svg, slide, wasm)
        #[arg(short, long, default_value = "pdf")]
        format: String,

        /// Output file or directory path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Install universal slide-viewer player and desktop integration to system
    InstallViewer,
}
