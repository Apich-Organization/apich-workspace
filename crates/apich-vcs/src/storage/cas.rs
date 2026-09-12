//! Content-Addressable Storage (CAS) engine.

use crate::chunking::FastCdc;
use crate::chunking::FastCdcConfig;

use crate::error::Result;
use crate::error::VcsError;
use crate::model::ChunkRef;
use crate::model::Snapshot;
use crate::model::VcsTree;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use uuid::Uuid;

/// Content-Addressable Storage (CAS) for FastCDC-chunked file blocks, trees, and snapshots
#[derive(Debug, Clone)]
pub struct ContentAddressableStorage {
    root_dir: PathBuf,
}

impl ContentAddressableStorage {
    /// Creates or opens a `ContentAddressableStorage` repository at `root_dir`.
    ///
    /// # Errors
    /// Returns an error if reading, writing, hashing, or accessing content-addressed storage fails.
    pub fn new(root_dir: impl AsRef<Path>) -> Result<Self> {
        let root = root_dir.as_ref().to_path_buf();
        fs::create_dir_all(root.join("chunks"))?;
        fs::create_dir_all(root.join("trees"))?;
        fs::create_dir_all(root.join("snapshots"))?;
        Ok(Self { root_dir: root })
    }

    /// Root directory of the CAS store (typically `<project>/.apich/cas`)
    #[must_use]
    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    // --- Chunk Storage ---

    fn chunk_path(
        &self,
        hash: &str,
    ) -> PathBuf {
        let prefix1 = if hash.len() >= 2 {
            &hash[0..2]
        } else {
            "xx"
        };
        let prefix2 = if hash.len() >= 4 {
            &hash[2..4]
        } else {
            "yy"
        };
        self.root_dir
            .join("chunks")
            .join(prefix1)
            .join(prefix2)
            .join(format!("{hash}.chunk"))
    }

    /// Checks whether a chunk exists in the store for the given BLAKE3 hash.
    #[must_use]
    pub fn has_chunk(
        &self,
        hash: &str,
    ) -> bool {
        self.chunk_path(hash).exists()
    }

    /// Put raw chunk bytes into CAS (compressed via Gzip). Returns BLAKE3 hex hash.
    ///
    /// # Errors
    /// Returns an error if reading, writing, hashing, or accessing content-addressed storage fails.
    pub fn put_chunk(
        &self,
        data: &[u8],
    ) -> Result<String> {
        let hash = blake3::hash(data).to_hex().to_string();
        let path = self.chunk_path(&hash);

        if !path.exists() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
            encoder.write_all(data)?;
            let compressed = encoder.finish()?;

            // Write atomically via temp file
            let tmp_path = path.with_extension("tmp");
            fs::write(&tmp_path, compressed)?;
            fs::rename(tmp_path, &path)?;
        }

        Ok(hash)
    }

    /// Read raw chunk bytes from CAS (decompressed)
    ///
    /// # Errors
    /// Returns an error if reading, writing, hashing, or accessing content-addressed storage fails.
    pub fn get_chunk(
        &self,
        hash: &str,
    ) -> Result<Vec<u8>> {
        let path = self.chunk_path(hash);
        if !path.exists() {
            return Err(VcsError::ChunkNotFound(hash.to_string()));
        }

        let compressed = fs::read(&path)?;
        let mut decoder = GzDecoder::new(&compressed[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        Ok(decompressed)
    }

    /// Process a whole file buffer through `FastCDC` and store all chunks into CAS.
    ///
    /// Computes and returns the full-file BLAKE3 hash and Git SHA-1 hash alongside chunk references.
    ///
    /// # Errors
    /// Returns an error if reading, writing, hashing, or accessing content-addressed storage fails.
    pub fn put_file_data(
        &self,
        data: &[u8],
        config: FastCdcConfig,
    ) -> Result<(Vec<ChunkRef>, String, String)> {
        let mut chunk_refs = Vec::new();

        // 1. FastCDC Chunking
        let chunker = FastCdc::new(data, config);
        for chunk in chunker {
            let hash = self.put_chunk(chunk.data)?;
            chunk_refs.push(ChunkRef {
                hash,
                offset: chunk.offset as u64,
                length: chunk.length,
            });
        }

        // 2. Full-file BLAKE3
        let blake3_hash = blake3::hash(data).to_hex().to_string();

        // 3. Native Git Blob SHA-1 (format: "blob <size>\0<content>")
        let git_sha1 = git2::Oid::hash_object(git2::ObjectType::Blob, data)
            .map(|oid| oid.to_string())
            .unwrap_or_default();

        Ok((chunk_refs, blake3_hash, git_sha1))
    }

    /// Reconstruct whole file content from chunk references
    ///
    /// # Errors
    /// Returns an error if reading, writing, hashing, or accessing content-addressed storage fails.
    pub fn read_file_data(
        &self,
        chunks: &[ChunkRef],
    ) -> Result<Vec<u8>> {
        let mut buffer = Vec::new();
        for chunk_ref in chunks {
            let chunk_data = self.get_chunk(&chunk_ref.hash)?;
            buffer.extend_from_slice(&chunk_data);
        }
        Ok(buffer)
    }

    // --- Tree Storage ---

    fn tree_path(
        &self,
        tree_hash: &str,
    ) -> PathBuf {
        self.root_dir
            .join("trees")
            .join(format!("{tree_hash}.json"))
    }

    /// Stores a serialized `VcsTree` to disk if not already present.
    ///
    /// # Errors
    /// Returns an error if reading, writing, hashing, or accessing content-addressed storage fails.
    pub fn put_tree(
        &self,
        tree: &VcsTree,
    ) -> Result<()> {
        let path = self.tree_path(&tree.tree_hash);
        if !path.exists() {
            let json = serde_json::to_vec(tree)?;
            fs::write(path, json)?;
        }
        Ok(())
    }

    /// Loads a `VcsTree` by its canonical tree hash.
    ///
    /// # Errors
    /// Returns an error if reading, writing, hashing, or accessing content-addressed storage fails.
    pub fn get_tree(
        &self,
        tree_hash: &str,
    ) -> Result<VcsTree> {
        let path = self.tree_path(tree_hash);
        if !path.exists() {
            return Err(VcsError::Internal(format!("Tree not found: {tree_hash}")));
        }
        let json = fs::read(path)?;
        let tree: VcsTree = serde_json::from_slice(&json)?;
        Ok(tree)
    }

    // --- Snapshot Storage ---

    fn snapshot_path(
        &self,
        id: Uuid,
    ) -> PathBuf {
        self.root_dir.join("snapshots").join(format!("{id}.json"))
    }

    /// Persists a `Snapshot` to disk.
    ///
    /// # Errors
    /// Returns an error if reading, writing, hashing, or accessing content-addressed storage fails.
    pub fn put_snapshot(
        &self,
        snapshot: &Snapshot,
    ) -> Result<()> {
        let path = self.snapshot_path(snapshot.id);
        let json = serde_json::to_vec_pretty(snapshot)?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Loads a `Snapshot` by its unique UUID.
    ///
    /// # Errors
    /// Returns an error if reading, writing, hashing, or accessing content-addressed storage fails.
    pub fn get_snapshot(
        &self,
        id: Uuid,
    ) -> Result<Snapshot> {
        let path = self.snapshot_path(id);
        if !path.exists() {
            return Err(VcsError::SnapshotNotFound(id.to_string()));
        }
        let json = fs::read(path)?;
        let snapshot: Snapshot = serde_json::from_slice(&json)?;
        Ok(snapshot)
    }

    /// Lists all snapshots in the repository sorted chronologically ascending.
    ///
    /// # Errors
    /// Returns an error if reading, writing, hashing, or accessing content-addressed storage fails.
    pub fn list_snapshots(&self) -> Result<Vec<Snapshot>> {
        let mut list = Vec::new();
        let snapshots_dir = self.root_dir.join("snapshots");
        if !snapshots_dir.exists() {
            return Ok(list);
        }

        for entry in fs::read_dir(snapshots_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(json) = fs::read(&path) {
                    if let Ok(s) = serde_json::from_slice::<Snapshot>(&json) {
                        list.push(s);
                    }
                }
            }
        }

        // Sort chronologically ascending (oldest to newest)
        list.sort_by_key(|s| s.created_at);
        Ok(list)
    }

    /// Deletes a snapshot file by its UUID.
    ///
    /// # Errors
    /// Returns an error if reading, writing, hashing, or accessing content-addressed storage fails.
    pub fn remove_snapshot(
        &self,
        id: Uuid,
    ) -> Result<()> {
        let path = self.snapshot_path(id);
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }
}
