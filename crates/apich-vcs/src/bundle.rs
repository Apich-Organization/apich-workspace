use crate::api::ProjectVcs;
use crate::error::{Result, VcsError};
use chrono::{DateTime, Utc};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use tar::{Archive, Builder, Header};
use uuid::Uuid;

/// Metadata manifest stored within every APICH project bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleManifest {
    pub vcs_version: String,
    pub created_at: DateTime<Utc>,
    pub head_branch: Option<String>,
    pub head_snapshot_id: Option<Uuid>,
    pub total_snapshots: usize,
}

/// Configuration options for project bundle export
#[derive(Debug, Clone)]
pub struct BundleOptions {
    /// Flate2 gzip compression level (default: 6)
    pub compression_level: u32,
    /// Automatically create a snapshot of uncommitted working directory changes before exporting
    pub auto_snapshot_before_export: bool,
}

impl Default for BundleOptions {
    fn default() -> Self {
        Self {
            compression_level: 6,
            auto_snapshot_before_export: true,
        }
    }
}

/// Project Packaging and Bundle Management for local synchronization and gateway downloads
pub struct ProjectBundle;

impl ProjectBundle {
    /// Export the entire VCS repository (.apich directory and metadata) to a compressed stream
    pub fn export<W: Write>(vcs: &ProjectVcs, writer: W, options: BundleOptions) -> Result<()> {
        if options.auto_snapshot_before_export {
            let _ = vcs.snapshot("Pre-export bundle auto-save");
        }

        let encoder = GzEncoder::new(writer, Compression::new(options.compression_level));
        let mut tar_builder = Builder::new(encoder);

        let apich_dir = vcs.project_root().join(".apich");
        if !apich_dir.exists() {
            return Err(VcsError::Internal(
                "Cannot bundle non-existent .apich directory".to_string(),
            ));
        }

        // 1. Pack manifest
        let manifest = BundleManifest {
            vcs_version: env!("CARGO_PKG_VERSION").to_string(),
            created_at: Utc::now(),
            head_branch: vcs.current_branch()?,
            head_snapshot_id: vcs.head_snapshot()?.map(|s| s.id),
            total_snapshots: vcs.list_snapshots()?.len(),
        };
        let manifest_bytes = serde_json::to_vec_pretty(&manifest)?;
        let mut header = Header::new_gnu();
        header.set_path("bundle.json")?;
        header.set_size(manifest_bytes.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar_builder.append(&header, &manifest_bytes[..])?;

        // 2. Recursively append .apich directory contents
        tar_builder.append_dir_all(".apich", &apich_dir)?;

        let encoder = tar_builder.into_inner()?;
        encoder.finish()?;
        Ok(())
    }

    /// Helper to export bundle directly to a filesystem path
    pub fn export_to_file(
        vcs: &ProjectVcs,
        dest_path: impl AsRef<Path>,
        options: BundleOptions,
    ) -> Result<()> {
        let file = File::create(dest_path)?;
        Self::export(vcs, file, options)
    }

    /// Import a project bundle from a stream into a destination folder, fully materializing
    /// all historical snapshots, branches, OpLog, and working copy files
    pub fn import<R: Read>(reader: R, target_dir: impl AsRef<Path>) -> Result<ProjectVcs> {
        let target = target_dir.as_ref();
        fs::create_dir_all(target)?;

        let decoder = GzDecoder::new(reader);
        let mut archive = Archive::new(decoder);
        archive.unpack(target)?;

        // Open the unpacked project
        let vcs = ProjectVcs::open_or_init(target)?;

        // Materialize the HEAD snapshot into the working directory
        if let Some(head) = vcs.head_snapshot()? {
            vcs.checkout_tree(head.id)?;
        }

        Ok(vcs)
    }

    /// Helper to import a bundle file from disk into a target directory
    pub fn import_from_file(
        bundle_path: impl AsRef<Path>,
        target_dir: impl AsRef<Path>,
    ) -> Result<ProjectVcs> {
        let file = File::open(bundle_path)?;
        Self::import(file, target_dir)
    }

    /// Export a clean archive (tar.gz) of a specific snapshot without .apich metadata
    /// Ideal for journal submissions, grading downloads, and distribution
    pub fn export_snapshot_archive<W: Write>(
        vcs: &ProjectVcs,
        snapshot_id: Uuid,
        writer: W,
    ) -> Result<()> {
        let snapshot = vcs.cas().get_snapshot(snapshot_id)?;
        let tree = vcs.cas().get_tree(&snapshot.tree_hash)?;

        let encoder = GzEncoder::new(writer, Compression::default());
        let mut tar_builder = Builder::new(encoder);

        for (rel_path, entry) in &tree.entries {
            let data = vcs.cas().read_file_data(&entry.chunks)?;
            let mut header = Header::new_gnu();
            header.set_path(rel_path)?;
            header.set_size(data.len() as u64);
            header.set_mode(if entry.is_executable { 0o755 } else { 0o644 });
            header.set_cksum();
            tar_builder.append(&header, &data[..])?;
        }

        let encoder = tar_builder.into_inner()?;
        encoder.finish()?;
        Ok(())
    }

    /// Helper to export clean snapshot archive to a file
    pub fn export_snapshot_archive_to_file(
        vcs: &ProjectVcs,
        snapshot_id: Uuid,
        dest_path: impl AsRef<Path>,
    ) -> Result<()> {
        let file = File::create(dest_path)?;
        Self::export_snapshot_archive(vcs, snapshot_id, file)
    }
}
