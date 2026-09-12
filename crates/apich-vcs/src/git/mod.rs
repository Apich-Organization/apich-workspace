//! Bidirectional Git bridge and interoperability subsystem.

/// Git compatibility bridge for commit export and branch operations.
pub mod bridge;
/// Git remote authentication and credential models.
pub mod credentials;
/// Git Large File Storage (LFS) policies and pointer generators.
pub mod lfs;
/// External Git research material manager.
pub mod material;

pub use bridge::GitBridge;
pub use credentials::GitAuth;
pub use credentials::GitRemoteConfig;
pub use lfs::LfsPolicy;
pub use material::MaterialManager;
pub use material::MaterialRecord;
