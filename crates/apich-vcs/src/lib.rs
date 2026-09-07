//! # APICH VCS
//!
//! Next-Generation Version Control System for Technical & Academic Workspaces.
//!
//! Features:
//! - **FastCDC Variable-Length Chunking**: Content-defined chunking for deduplicated storage of documents, SQLite databases, and large research datasets without Git LFS.
//! - **Weave-Free History Model**: No tangled criss-cross parent pointer graphs; clean 3-way structural reconcile with first-class, non-blocking conflicts.
//! - **Continuous Auto-save & Linear OpLog**: 100% reversible history with atomic undo-tree and zero staging-area (no index) cognitive friction.
//! - **Grandfather-Father-Son Retention**: Multi-tier snapshot compaction keeping active working states while preserving macro-timeline and all milestones indefinitely.
//! - **Bidirectional Git Bridge**: Full export/import mapping between internal FastCDC snapshots and standard Git commit/tree objects.
//! - **Synthetic Smart Ignore**: Built-in academic, LaTeX, Typst, Python, R, and technical build artifact filtering.

pub mod api;
pub mod autosave;
pub mod bundle;
pub mod chunking;
pub mod config;
pub mod error;
pub mod git;
pub mod history;
pub mod ignore;
pub mod lock;
pub mod model;
pub mod oplog;
pub mod storage;

pub use api::{ProjectVcs, RepoStatus};
pub use autosave::{AutosaveConfig, AutosaveEngine};
pub use bundle::{BundleManifest, BundleOptions, ProjectBundle};
pub use chunking::{Chunk, FastCdc, FastCdcConfig};
pub use config::{ChunkingConfig, IgnoreConfig, LfsConfig, RetentionConfig, VcsConfig};
pub use error::{Result, VcsError};
pub use git::{GitAuth, GitBridge, GitRemoteConfig, LfsPolicy, MaterialManager, MaterialRecord};
pub use history::{ReconcileResult, Reconciler, RetentionPolicy};
pub use ignore::{IgnoreFilter, IgnoreFilterBuilder, IgnoreProfile};
pub use lock::RepositoryLock;
pub use model::{Branch, ChunkRef, Conflict, FileEntry, Snapshot, TreeDiff, VcsTree};
pub use oplog::{OpAction, OpLog, VcsOperation};
pub use storage::{ContentAddressableStorage, GarbageCollector, GcStats};
