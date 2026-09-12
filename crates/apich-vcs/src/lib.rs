//! # APICH VCS
//!
//! Next-Generation Version Control System for Technical & Academic Workspaces.
//!
//! Features:
//! - **`FastCDC` Variable-Length Chunking**: Content-defined chunking for deduplicated storage of documents, SQLite databases, and large research datasets without Git LFS.
//! - **Weave-Free History Model**: No tangled criss-cross parent pointer graphs; clean 3-way structural reconcile with first-class, non-blocking conflicts.
//! - **Continuous Auto-save & Linear `OpLog`**: 100% reversible history with atomic undo-tree and zero staging-area (no index) cognitive friction.
//! - **Grandfather-Father-Son Retention**: Multi-tier snapshot compaction keeping active working states while preserving macro-timeline and all milestones indefinitely.
//! - **Bidirectional Git Bridge**: Full export/import mapping between internal `FastCDC` snapshots and standard Git commit/tree objects.
//! - **Synthetic Smart Ignore**: Built-in academic, LaTeX, Typst, Python, R, and technical build artifact filtering.

/// High-level VCS API and repository management.
pub mod api;
/// Continuous background autosave engine and scheduler.
pub mod autosave;
/// Bundle archive format and bundle export/import operations.
pub mod bundle;
/// FastCDC content-defined chunking engine.
pub mod chunking;
/// Configuration schemas and persistence for VCS repositories.
pub mod config;
/// Error types and result aliases for the VCS subsystem.
pub mod error;
/// Bidirectional Git bridge, credentials, and Git LFS material management.
pub mod git;
/// GPG signature signing and verification.
pub mod gpg;
/// History reconciliation and retention policy enforcement.
pub mod history;
/// Synthetic smart ignore profiles and ignore file filtering.
pub mod ignore;
/// Process-safe repository locking.
pub mod lock;
/// Core domain models: trees, branches, snapshots, and conflicts.
pub mod model;
/// Operation log (OpLog) for atomic actions and reversibility.
pub mod oplog;
/// Content-addressable storage (CAS) and garbage collection.
pub mod storage;

pub use api::ProjectVcs;
pub use api::RepoStatus;
pub use autosave::AutosaveConfig;
pub use autosave::AutosaveEngine;
pub use bundle::BundleManifest;
pub use bundle::BundleOptions;
pub use bundle::ProjectBundle;
pub use bundle::PushOutcome;
pub use chunking::Chunk;
pub use chunking::FastCdc;
pub use chunking::FastCdcConfig;
pub use config::ChunkingConfig;
pub use config::IgnoreConfig;
pub use config::LfsConfig;
pub use config::ProfileToggle;
pub use config::RetentionConfig;
pub use config::VcsConfig;
pub use error::Result;
pub use error::VcsError;
pub use git::GitAuth;
pub use git::GitBridge;
pub use git::GitRemoteConfig;
pub use git::LfsPolicy;
pub use git::MaterialManager;
pub use git::MaterialRecord;
pub use gpg::SignatureStatus;
pub use history::ReconcileResult;
pub use history::Reconciler;
pub use history::RetentionPolicy;
pub use ignore::IgnoreFilter;
pub use ignore::IgnoreFilterBuilder;
pub use ignore::IgnoreProfile;
pub use lock::RepositoryLock;
pub use model::Branch;
pub use model::ChunkRef;
pub use model::Conflict;
pub use model::FileEntry;
pub use model::Snapshot;
pub use model::TreeDiff;
pub use model::VcsTree;
pub use oplog::OpAction;
pub use oplog::OpLog;
pub use oplog::VcsOperation;
pub use storage::ContentAddressableStorage;
pub use storage::GarbageCollector;
pub use storage::GcStats;
