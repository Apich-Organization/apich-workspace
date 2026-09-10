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

pub use config::{MountSpec, ResourceLimits, SandboxConfig, SandboxConfigBuilder};
pub use container::UserContainer;
pub use driver::{ContainerInspectInfo, ContainerStatus, PodmanDriver};
pub use error::{Result, SandboxError};
pub use exec::{ExecOptions, ExecResult, ExecStream, InteractiveExec, OutputChunk};
pub use fs::{
    create_dir_all_safe, file_exists_safe, list_dir_safe, read_file_safe, remove_file_safe,
    resolve_safe_path, write_file_safe, FileEntry,
};
pub use manager::SandboxManager;
pub use tools::{
    AgentKind, AgentToolchain, GitToolchain, LatexEngine, LatexToolchain, LoginSupport,
    PythonToolchain, RToolchain, RustToolchain, TypstToolchain,
};
