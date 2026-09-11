use super::gear::GEAR_TABLE;

/// Configuration parameters for Content-Defined Chunking
#[derive(Debug, Clone, Copy)]
pub struct FastCdcConfig {
    pub min_size: usize,
    pub avg_size: usize,
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
    pub fn document() -> Self {
        Self {
            min_size: 4 * 1024,  // 4 KB
            avg_size: 16 * 1024, // 16 KB
            max_size: 64 * 1024, // 64 KB
        }
    }

    /// Configuration optimized for large datasets / SQLite tables / PDFs
    pub fn large_file() -> Self {
        Self {
            min_size: 64 * 1024,   // 64 KB
            avg_size: 256 * 1024,  // 256 KB
            max_size: 1024 * 1024, // 1 MB
        }
    }
}

/// A chunk cut emitted by the FastCDC algorithm
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk<'a> {
    pub offset: usize,
    pub length: usize,
    pub data: &'a [u8],
}

/// FastCDC chunk iterator over a byte buffer
pub struct FastCdc<'a> {
    data: &'a [u8],
    config: FastCdcConfig,
    cursor: usize,
    mask_s: u64,
    mask_l: u64,
}

impl<'a> FastCdc<'a> {
    pub fn new(
        data: &'a [u8],
        config: FastCdcConfig,
    ) -> Self {
        // Calculate logarithmic power for masks
        let bits = (config.avg_size as f64).log2().round() as u32;
        let mask_s = (1u64 << (bits + 1)) - 1;
        let mask_l = (1u64 << (bits - 1)) - 1;

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
        let remaining = self.data.len() - self.cursor;
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
                data: &self.data[chunk_start..self.cursor],
            });
        }

        // Sub-minimum skipping: advance by min_size without hash evaluation
        let mut offset = self.config.min_size;
        let max_chunk = remaining.min(self.config.max_size);
        let normal_size = remaining.min(self.config.avg_size);
        let mut fingerprint: u64 = 0;

        // Region 1: between min_size and avg_size, use mask_s
        while offset < normal_size {
            let byte = self.data[self.cursor + offset];
            fingerprint = (fingerprint << 1).wrapping_add(GEAR_TABLE[byte as usize]);
            if (fingerprint & self.mask_s) == 0 {
                offset += 1;
                self.cursor += offset;
                return Some(Chunk {
                    offset: chunk_start,
                    length: offset,
                    data: &self.data[chunk_start..self.cursor],
                });
            }
            offset += 1;
        }

        // Region 2: between avg_size and max_size, use mask_l
        while offset < max_chunk {
            let byte = self.data[self.cursor + offset];
            fingerprint = (fingerprint << 1).wrapping_add(GEAR_TABLE[byte as usize]);
            if (fingerprint & self.mask_l) == 0 {
                offset += 1;
                self.cursor += offset;
                return Some(Chunk {
                    offset: chunk_start,
                    length: offset,
                    data: &self.data[chunk_start..self.cursor],
                });
            }
            offset += 1;
        }

        // Hit max_size or EOF boundary
        self.cursor += offset;
        Some(Chunk {
            offset: chunk_start,
            length: offset,
            data: &self.data[chunk_start..self.cursor],
        })
    }
}
