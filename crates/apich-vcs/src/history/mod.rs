//! History reconciliation and snapshot retention subsystem.

/// Three-way structural reconciliation and merge engine.
pub mod reconcile;
/// Exponential decay (Grandfather-Father-Son) retention policies.
pub mod retention;

pub use reconcile::ReconcileResult;
pub use reconcile::Reconciler;
pub use retention::RetentionPolicy;
