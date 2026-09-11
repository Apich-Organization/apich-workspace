//! # APICH Sandbox
//!
//! A high-performance, asynchronous Podman containerization and sandbox orchestration engine
//! designed for the APICH Technical & Academic Workspace.
//!
//! ## Core Features:
//! - **Per-user Container Sandboxing**: Dynamically creates, binds, limits, and manages user workspace containers.
//! - **Storage Mount & Persistence**: Mounts host workspace folders with automatic SELinux label handling.
//! - **Full Lifecycle Management**: Initialize, start, pause, unpause, restart, inspect, gracefully stop, and destroy containers.
//! - **Command Execution & Real-time Output Streaming**: Execute commands with stdout/stderr multiplexed async streams.
//! - **File System Operations**: Fast, safe workspace read/write, file entry listing, path traversal defenses.
//! - **Academic & Technical Toolchain Integration**: Ready-to-use helpers for Rust, Python, Git, R, LaTeX, and Typst.

pub mod config;
pub mod container;
pub mod driver;
pub mod error;
pub mod exec;
pub mod fs;
pub mod manager;
pub mod tools;

pub use config::MountSpec;
pub use config::ResourceLimits;
pub use config::SandboxConfig;
pub use config::SandboxConfigBuilder;
pub use container::UserContainer;
pub use driver::ContainerInspectInfo;
pub use driver::ContainerStatus;
pub use driver::PodmanDriver;
pub use error::Result;
pub use error::SandboxError;
pub use exec::ExecOptions;
pub use exec::ExecResult;
pub use exec::ExecStream;
pub use exec::InteractiveExec;
pub use exec::OutputChunk;
pub use fs::create_dir_all_safe;
pub use fs::file_exists_safe;
pub use fs::list_dir_safe;
pub use fs::read_file_safe;
pub use fs::remove_file_safe;
pub use fs::resolve_safe_path;
pub use fs::write_file_safe;
pub use fs::FileEntry;
pub use manager::SandboxManager;
pub use tools::AgentKind;
pub use tools::AgentToolchain;
pub use tools::GitToolchain;
pub use tools::LatexEngine;
pub use tools::LatexToolchain;
pub use tools::LoginSupport;
pub use tools::PythonToolchain;
pub use tools::RToolchain;
pub use tools::RustToolchain;
pub use tools::TypstToolchain;
