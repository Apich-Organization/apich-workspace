//! Smart synthetic ignore subsystem.

/// Ignore filtering engine and builder.
pub mod filter;
/// Built-in ignore profile definitions.
pub mod profile;

pub use filter::IgnoreFilter;
pub use filter::IgnoreFilterBuilder;
pub use profile::IgnoreProfile;
