//! Content-addressable storage (CAS) and garbage collection subsystem.

/// Content-addressable storage for chunks, trees, and snapshots.
pub mod cas;
/// Mark-and-sweep garbage collection engine.
pub mod gc;

pub use cas::ContentAddressableStorage;
pub use gc::GarbageCollector;
pub use gc::GcStats;
