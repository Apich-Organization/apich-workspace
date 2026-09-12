//! FastCDC content-defined chunking subsystem.

/// FastCDC chunking algorithm and iterator.
pub mod fastcdc;
/// Gear hash lookup table and utilities.
pub mod gear;

pub use fastcdc::Chunk;
pub use fastcdc::FastCdc;
pub use fastcdc::FastCdcConfig;
