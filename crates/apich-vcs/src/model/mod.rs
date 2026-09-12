//! Core domain models for the VCS subsystem.

/// Branch pointer models.
pub mod branch;
/// Content-defined chunk reference models.
pub mod chunk;
/// Non-blocking conflict models.
pub mod conflict;
/// Snapshot metadata models.
pub mod snapshot;
/// Merkle tree and structural diff models.
pub mod tree;

pub use branch::Branch;
pub use chunk::ChunkRef;
pub use conflict::Conflict;
pub use snapshot::Snapshot;
pub use tree::FileEntry;
pub use tree::TreeDiff;
pub use tree::VcsTree;
