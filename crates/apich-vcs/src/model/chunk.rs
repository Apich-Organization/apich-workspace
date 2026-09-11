use serde::Deserialize;
use serde::Serialize;

/// Reference to a content-defined chunk stored in CAS
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkRef {
    /// BLAKE3 hash of the chunk data
    pub hash: String,
    /// Byte offset in the original file
    pub offset: u64,
    /// Length of the chunk in bytes
    pub length: usize,
}
