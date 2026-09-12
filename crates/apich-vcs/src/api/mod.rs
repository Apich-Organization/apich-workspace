//! High-level VCS API module.

/// High-level Unified Version Control facade for workspace projects.
pub mod project_vcs;

pub use project_vcs::ProjectVcs;
pub use project_vcs::RepoStatus;
