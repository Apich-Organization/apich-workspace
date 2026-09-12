//! Academic and technical language toolchains available inside the sandbox.

/// AI coding agent CLI driver and interactive sessions.
pub mod agent;
/// Git CLI operations within the sandbox container.
pub mod git;
/// LaTeX compilation engines and toolchain.
pub mod latex;
/// Python runtime execution and package manager helpers.
pub mod python;
/// R language interpreter and script execution.
pub mod r;
/// Rust toolchain: rustc, cargo, and formatting.
pub mod rust;
/// Typst document compiler toolchain.
pub mod typst;

pub use agent::AgentKind;
pub use agent::AgentToolchain;
pub use agent::LoginSupport;
pub use git::GitToolchain;
pub use latex::LatexEngine;
pub use latex::LatexToolchain;
pub use python::PythonToolchain;
pub use r::RToolchain;
pub use rust::RustToolchain;
pub use typst::TypstToolchain;
