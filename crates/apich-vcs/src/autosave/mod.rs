//! Continuous background autosave module.

/// Thread-safe autosave debouncing engine.
pub mod engine;

pub use engine::AutosaveConfig;
pub use engine::AutosaveEngine;
