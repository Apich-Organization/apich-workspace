//! FastCDC (Fast Content-Defined Chunking) implementation.

use super::gear::GEAR_TABLE;

/// Configuration parameters for Content-Defined Chunking
#[derive(Debug, Clone, Copy)]
pub struct FastCdcConfig {
    /// Minimum allowable chunk size in bytes.
    pub min_size: usize,
    /// Target average chunk size in bytes.
    pub avg_size: usize,
    /// Hard upper bound chunk size in bytes.
    pub max_size: usize,
}

impl Default for FastCdcConfig {
    fn default() -> Self {
        Self {
            min_size: 16 * 1024,  // 16 KB
            avg_size: 64 * 1024,  // 64 KB
            max_size: 256 * 1024, // 256 KB
        }
    }
}

impl FastCdcConfig {
    /// Configuration optimized for smaller documents (Markdown, Typst, LaTeX)
    #[must_use]
    pub const fn document() -> Self {
        Self {
            min_size: 4 * 1024,  // 4 KB
            avg_size: 16 * 1024, // 16 KB
            max_size: 64 * 1024, // 64 KB
        }
    }

    /// Configuration optimized for large datasets / SQLite tables / PDFs
    #[must_use]
    pub const fn large_file() -> Self {
        Self {
            min_size: 64 * 1024,   // 64 KB
            avg_size: 256 * 1024,  // 256 KB
            max_size: 1024 * 1024, // 1 MB
        }
    }
}

/// A chunk cut emitted by the `FastCDC` algorithm
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk<'a> {
    /// Byte offset within the source buffer where this chunk begins.
    pub offset: usize,
    /// Size of the chunk in bytes.
    pub length: usize,
    /// Borrowed byte slice containing chunk data.
    pub data: &'a [u8],
}

/// `FastCDC` chunk iterator over a byte buffer
pub struct FastCdc<'a> {
    data: &'a [u8],
    config: FastCdcConfig,
    cursor: usize,
    mask_s: u64,
    mask_l: u64,
}

impl<'a> FastCdc<'a> {
    /// Creates a new `FastCdc` iterator over the provided data buffer.
    #[must_use]
    pub fn new(
        data: &'a [u8],
        config: FastCdcConfig,
    ) -> Self {
        // Calculate logarithmic power for masks
        let bits = config.avg_size.checked_ilog2().unwrap_or(18);
        let mask_s = (1u64.checked_shl(bits.saturating_add(1)).unwrap_or(0)).saturating_sub(1);
        let mask_l = (1u64.checked_shl(bits.saturating_sub(1)).unwrap_or(0)).saturating_sub(1);

        Self {
            data,
            config,
            cursor: 0,
            mask_s,
            mask_l,
        }
    }
}

impl<'a> Iterator for FastCdc<'a> {
    type Item = Chunk<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let remaining = self.data.len().saturating_sub(self.cursor);
        if remaining == 0 {
            return None;
        }

        let chunk_start = self.cursor;

        // If remaining bytes is less than min_size, the whole tail is the final chunk
        if remaining <= self.config.min_size {
            self.cursor = self.data.len();
            return Some(Chunk {
                offset: chunk_start,
                length: remaining,
                data: self.data.get(chunk_start..self.cursor).unwrap_or_default(),
            });
        }

        // Sub-minimum skipping: advance by min_size without hash evaluation
        let mut offset = self.config.min_size;
        let max_chunk = remaining.min(self.config.max_size);
        let normal_size = remaining.min(self.config.avg_size);
        let mut fingerprint: u64 = 0;

        // Region 1: between min_size and avg_size, use mask_s
        while offset < normal_size {
            let idx = self.cursor.saturating_add(offset);
            let byte = self.data.get(idx).copied().unwrap_or_default();
            let gear = GEAR_TABLE
                .get(usize::from(byte))
                .copied()
                .unwrap_or_default();
            fingerprint = (fingerprint << 1).wrapping_add(gear);
            if (fingerprint & self.mask_s) == 0 {
                offset = offset.saturating_add(1);
                self.cursor = self.cursor.saturating_add(offset);
                return Some(Chunk {
                    offset: chunk_start,
                    length: offset,
                    data: self.data.get(chunk_start..self.cursor).unwrap_or_default(),
                });
            }
            offset = offset.saturating_add(1);
        }

        // Region 2: between avg_size and max_size, use mask_l
        while offset < max_chunk {
            let idx = self.cursor.saturating_add(offset);
            let byte = self.data.get(idx).copied().unwrap_or_default();
            let gear = GEAR_TABLE
                .get(usize::from(byte))
                .copied()
                .unwrap_or_default();
            fingerprint = (fingerprint << 1).wrapping_add(gear);
            if (fingerprint & self.mask_l) == 0 {
                offset = offset.saturating_add(1);
                self.cursor = self.cursor.saturating_add(offset);
                return Some(Chunk {
                    offset: chunk_start,
                    length: offset,
                    data: self.data.get(chunk_start..self.cursor).unwrap_or_default(),
                });
            }
            offset = offset.saturating_add(1);
        }

        // Hit max_size or EOF boundary
        self.cursor = self.cursor.saturating_add(offset);
        Some(Chunk {
            offset: chunk_start,
            length: offset,
            data: self.data.get(chunk_start..self.cursor).unwrap_or_default(),
        })
    }
}
