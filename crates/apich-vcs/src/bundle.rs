//! Project bundle export and import implementation.

use crate::api::ProjectVcs;
use crate::error::Result;
use crate::error::VcsError;
use crate::model::Branch;
use chrono::DateTime;
use chrono::Utc;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::Deserialize;
use serde::Serialize;
use std::fs::File;
use std::fs::{
    self,
};
use std::io::Read;
use std::io::Write;
use std::path::Path;
use tar::Archive;
use tar::Builder;
use tar::Header;
use uuid::Uuid;

/// Result of accepting an uploaded bundle.
///
/// Indicates which branches advanced, and which were left untouched
/// because they were not a fast-forward of the receiving side's current history.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PushOutcome {
    /// Branches that were successfully accepted and updated.
    pub accepted_branches: Vec<String>,
    /// Branches that were rejected alongside the reason for rejection.
    pub rejected_branches: Vec<(String, String)>,
}

/// Metadata manifest stored within every APICH project bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleManifest {
    /// VCS format version used to create the bundle.
    pub vcs_version: String,
    /// Timestamp when the bundle was created.
    pub created_at: DateTime<Utc>,
    /// Name of the active branch at export time.
    pub head_branch: Option<String>,
    /// Snapshot ID of HEAD at export time.
    pub head_snapshot_id: Option<Uuid>,
    /// Total number of snapshots contained in the bundle.
    pub total_snapshots: usize,
}

/// Configuration options for project bundle export
#[derive(Debug, Clone, Copy)]
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
    ///
    /// # Errors
    /// Returns an error if the bundle export, import, or file operation fails.
    pub fn export<W: Write>(
        vcs: &ProjectVcs,
        writer: W,
        options: BundleOptions,
    ) -> Result<()> {
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
    ///
    /// # Errors
    /// Returns an error if the bundle export, import, or file operation fails.
    pub fn export_to_file(
        vcs: &ProjectVcs,
        dest_path: impl AsRef<Path>,
        options: BundleOptions,
    ) -> Result<()> {
        let file = File::create(dest_path)?;
        Self::export(vcs, file, options)
    }

    /// Import a project bundle from a stream into a destination folder.
    ///
    /// Fully materializes all historical snapshots, branches, `OpLog`, and working copy files.
    ///
    /// # Errors
    /// Returns an error if the bundle export, import, or file operation fails.
    pub fn import<R: Read>(
        reader: R,
        target_dir: impl AsRef<Path>,
    ) -> Result<ProjectVcs> {
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
    ///
    /// # Errors
    /// Returns an error if the bundle export, import, or file operation fails.
    pub fn import_from_file(
        bundle_path: impl AsRef<Path>,
        target_dir: impl AsRef<Path>,
    ) -> Result<ProjectVcs> {
        let file = File::open(bundle_path)?;
        Self::import(file, target_dir)
    }

    /// Accept an uploaded or downloaded bundle into an existing project.
    ///
    /// Acts as a "push" (or symmetrically, a "pull" -- the same
    /// merge logic is safe in both directions): unlike `import`, this
    /// never replaces or wipes anything already there.
    ///
    /// CAS objects (chunks/trees/snapshots) are merged in unconditionally -- they're
    /// content-addressed, so a file with the same relative path always has identical content, and
    /// copying it in is always safe. Branch refs are the one part of `.apich` that *isn't*
    /// append-only, so each incoming branch is only advanced if the receiving side's current HEAD
    /// for that branch is an ancestor of the incoming HEAD (a fast-forward) or the branch doesn't
    /// exist here yet; anything else is left untouched and reported back, mirroring how a real
    /// Git server rejects a non-fast-forward push rather than silently discarding history.
    ///
    /// # Errors
    /// Returns an error if the bundle export, import, or file operation fails.
    pub fn accept_push<R: Read>(
        vcs: &ProjectVcs,
        reader: R,
    ) -> Result<PushOutcome> {
        let tmp = tempfile::tempdir().map_err(VcsError::Io)?;
        let decoder = GzDecoder::new(reader);
        let mut archive = Archive::new(decoder);
        archive.unpack(tmp.path())?;

        let incoming_apich = tmp.path().join(".apich");
        if !incoming_apich.exists() {
            return Err(VcsError::Internal(
                "Uploaded bundle has no .apich directory".to_string(),
            ));
        }

        let local_apich = vcs.project_root().join(".apich");
        for sub in ["cas/chunks", "cas/trees", "cas/snapshots"] {
            let src = incoming_apich.join(sub);
            if src.exists() {
                Self::merge_copy_dir(&src, &local_apich.join(sub))?;
            }
        }

        let mut outcome = PushOutcome::default();
        let incoming_branches_dir = incoming_apich.join("branches");
        if incoming_branches_dir.exists() {
            for entry in fs::read_dir(&incoming_branches_dir)? {
                let path = entry?.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                let incoming_branch: Branch = serde_json::from_slice(&fs::read(&path)?)?;

                let is_current_branch =
                    vcs.current_branch()?.as_deref() == Some(incoming_branch.name.as_str());

                match vcs.get_branch(&incoming_branch.name)? {
                    | None => {
                        let new_head = incoming_branch.head_snapshot_id;
                        vcs.save_branch(&incoming_branch)?;
                        outcome.accepted_branches.push(incoming_branch.name);
                        if is_current_branch {
                            vcs.checkout_tree(new_head)?;
                        }
                    },
                    | Some(local_branch)
                        if local_branch.head_snapshot_id == incoming_branch.head_snapshot_id =>
                    {
                        outcome.accepted_branches.push(incoming_branch.name);
                    },
                    | Some(local_branch) => {
                        if Self::is_ancestor(
                            vcs,
                            local_branch.head_snapshot_id,
                            incoming_branch.head_snapshot_id,
                        ) {
                            let new_head = incoming_branch.head_snapshot_id;
                            vcs.save_branch(&incoming_branch)?;
                            outcome.accepted_branches.push(incoming_branch.name);
                            // Materialize the new content into the working directory -- without
                            // this, a subsequent local snapshot would scan the still-stale working
                            // tree and silently create a new snapshot that reverts the branch ref
                            // right back to the old (pre-push) content.
                            if is_current_branch {
                                vcs.checkout_tree(new_head)?;
                            }
                        } else {
                            outcome.rejected_branches.push((
                                incoming_branch.name,
                                "not a fast-forward of the current history -- pull before pushing"
                                    .to_string(),
                            ));
                        }
                    },
                }
            }
        }

        Ok(outcome)
    }

    /// Walk `descendant_id`'s parent chain looking for `ancestor_id`.
    fn is_ancestor(
        vcs: &ProjectVcs,
        ancestor_id: Uuid,
        descendant_id: Uuid,
    ) -> bool {
        let mut curr = Some(descendant_id);
        while let Some(id) = curr {
            if id == ancestor_id {
                return true;
            }
            curr = vcs
                .cas()
                .get_snapshot(id)
                .ok()
                .and_then(|s| s.parent_snapshot_id);
        }
        false
    }

    /// Recursively copy files from `src` into `dest`, skipping any existing paths.
    ///
    /// Correct here specifically because every file under `.apich/cas/` is
    /// named by the content hash of its own bytes, so "already exists" means "already identical".
    fn merge_copy_dir(
        src: &Path,
        dest: &Path,
    ) -> Result<()> {
        fs::create_dir_all(dest)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let path = entry.path();
            let dest_path = dest.join(entry.file_name());
            if path.is_dir() {
                Self::merge_copy_dir(&path, &dest_path)?;
            } else if !dest_path.exists() {
                fs::copy(&path, &dest_path)?;
            }
        }
        Ok(())
    }

    /// Export a clean archive (tar.gz) of a specific snapshot without .apich metadata
    /// Ideal for journal submissions, grading downloads, and distribution
    ///
    /// # Errors
    /// Returns an error if the bundle export, import, or file operation fails.
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
            header.set_mode(if entry.is_executable {
                0o755
            } else {
                0o644
            });
            header.set_cksum();
            tar_builder.append(&header, &data[..])?;
        }

        let encoder = tar_builder.into_inner()?;
        encoder.finish()?;
        Ok(())
    }

    /// Helper to export clean snapshot archive to a file
    ///
    /// # Errors
    /// Returns an error if the bundle export, import, or file operation fails.
    pub fn export_snapshot_archive_to_file(
        vcs: &ProjectVcs,
        snapshot_id: Uuid,
        dest_path: impl AsRef<Path>,
    ) -> Result<()> {
        let file = File::create(dest_path)?;
        Self::export_snapshot_archive(vcs, snapshot_id, file)
    }
}
