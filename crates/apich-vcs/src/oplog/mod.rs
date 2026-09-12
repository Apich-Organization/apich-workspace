//! Operation log (OpLog) subsystem for linear history and undo-tree.

/// Append-only operation log manager.
pub mod log;
/// Operation log records and action types.
pub mod op;

pub use log::OpLog;
pub use op::OpAction;
pub use op::VcsOperation;
