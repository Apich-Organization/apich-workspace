//! High-level Unified Version Control facade for workspace projects.

use crate::autosave::AutosaveEngine;
use crate::chunking::FastCdcConfig;
use crate::error::Result;
use crate::error::VcsError;
use crate::git::GitBridge;
use crate::git::MaterialManager;
use crate::git::MaterialRecord;
use crate::history::ReconcileResult;
use crate::history::Reconciler;
use crate::history::RetentionPolicy;
use crate::ignore::IgnoreFilter;
use crate::model::Branch;
use crate::model::FileEntry;
use crate::model::Snapshot;
use crate::model::VcsTree;
use crate::oplog::OpAction;
use crate::oplog::OpLog;
use crate::oplog::VcsOperation;
use crate::storage::ContentAddressableStorage;
use crate::storage::GarbageCollector;
use crate::storage::GcStats;
use chrono::Utc;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use uuid::Uuid;

use crate::config::VcsConfig;

/// High-level Unified Version Control facade for a Workspace Project
pub struct ProjectVcs {
    project_root: PathBuf,
    cas: ContentAddressableStorage,
    oplog: OpLog,
    config: VcsConfig,
    ignore_filter: IgnoreFilter,
    git_bridge: GitBridge,
    material_mgr: MaterialManager,
    autosave: AutosaveEngine,
    cdc_config: FastCdcConfig,
    retention_policy: RetentionPolicy,
}

impl ProjectVcs {
    /// Open existing project or initialize a new apich-vcs repository, loading project configuration
    ///
    /// # Errors
    /// Returns an error if initializing or opening the repository fails.
    pub fn open_or_init(project_root: impl AsRef<Path>) -> Result<Self> {
        let root = project_root.as_ref().to_path_buf();
        let apich_dir = root.join(".apich");
        fs::create_dir_all(apich_dir.join("branches"))?;

        let cas = ContentAddressableStorage::new(apich_dir.join("cas"))?;
        let oplog = OpLog::new(&root)?;
        let config = VcsConfig::load_from_project(&root)?;
        let ignore_filter = config.to_ignore_filter(&root)?;
        let lfs_policy = config.to_lfs_policy(&root);
        let git_bridge = GitBridge::new(&root).with_lfs_policy(lfs_policy);
        let material_mgr = MaterialManager::new(&root);
        let autosave = AutosaveEngine::new(config.to_autosave_config());
        let cdc_config = config.to_cdc_config();
        let retention_policy = config.to_retention_policy();

        let vcs = Self {
            project_root: root,
            cas,
            oplog,
            config,
            ignore_filter,
            git_bridge,
            material_mgr,
            autosave,
            cdc_config,
            retention_policy,
        };

        // Initialize default main branch if not exists
        if vcs.current_branch()?.is_none() {
            vcs.set_current_branch("main")?;
        }

        Ok(vcs)
    }

    /// Returns the root path of the project workspace.
    #[must_use]
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    /// Returns a reference to the content-addressable storage engine.
    #[must_use]
    pub const fn cas(&self) -> &ContentAddressableStorage {
        &self.cas
    }

    /// Returns a reference to the Git bridge integration.
    #[must_use]
    pub const fn git_bridge(&self) -> &GitBridge {
        &self.git_bridge
    }

    /// Returns a reference to the continuous autosave engine.
    #[must_use]
    pub const fn autosave(&self) -> &AutosaveEngine {
        &self.autosave
    }

    // --- Branch & HEAD Management ---

    fn head_file(&self) -> PathBuf {
        self.project_root.join(".apich").join("HEAD")
    }

    /// Gets the name of the currently active branch, if any.
    ///
    /// # Errors
    /// Returns an error if branch operations or reference updates fail.
    pub fn current_branch(&self) -> Result<Option<String>> {
        let file = self.head_file();
        if file.exists() {
            let name = fs::read_to_string(file)?.trim().to_string();
            Ok(Some(name))
        } else {
            Ok(None)
        }
    }

    /// Sets the currently active branch name in HEAD.
    ///
    /// # Errors
    /// Returns an error if branch operations or reference updates fail.
    pub fn set_current_branch(
        &self,
        name: &str,
    ) -> Result<()> {
        fs::write(self.head_file(), name)?;
        Ok(())
    }

    fn branch_file(
        &self,
        name: &str,
    ) -> PathBuf {
        self.project_root
            .join(".apich")
            .join("branches")
            .join(format!("{name}.json"))
    }

    /// Retrieves branch metadata by branch name.
    ///
    /// # Errors
    /// Returns an error if branch operations or reference updates fail.
    pub fn get_branch(
        &self,
        name: &str,
    ) -> Result<Option<Branch>> {
        let path = self.branch_file(name);
        if path.exists() {
            let json = fs::read(path)?;
            let branch: Branch = serde_json::from_slice(&json)?;
            Ok(Some(branch))
        } else {
            Ok(None)
        }
    }

    /// Saves branch metadata to disk.
    ///
    /// # Errors
    /// Returns an error if branch operations or reference updates fail.
    pub fn save_branch(
        &self,
        branch: &Branch,
    ) -> Result<()> {
        let path = self.branch_file(&branch.name);
        let json = serde_json::to_vec_pretty(branch)?;
        fs::write(path, json)?;
        Ok(())
    }

    /// List names of all branches in the repository
    ///
    /// # Errors
    /// Returns an error if branch operations or reference updates fail.
    pub fn list_branches(&self) -> Result<Vec<String>> {
        let branches_dir = self.project_root.join(".apich").join("branches");
        if !branches_dir.exists() {
            return Ok(Vec::new());
        }
        let mut branches = Vec::new();
        for entry in fs::read_dir(branches_dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(stripped) = name.strip_suffix(".json") {
                branches.push(stripped.to_string());
            }
        }
        branches.sort();
        Ok(branches)
    }

    /// Returns the current HEAD snapshot, if any.
    ///
    /// # Errors
    /// Returns an error if creating or retrieving the snapshot fails.
    pub fn head_snapshot(&self) -> Result<Option<Snapshot>> {
        if let Some(branch_name) = self.current_branch()? {
            if let Some(branch) = self.get_branch(&branch_name)? {
                let s = self.cas.get_snapshot(branch.head_snapshot_id)?;
                return Ok(Some(s));
            }
        }
        Ok(None)
    }

    // --- Working Copy Scanning & Snapshotting ---

    /// Scan working directory on disk and construct a new `VcsTree` using `FastCDC`
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn scan_working_tree(&self) -> Result<VcsTree> {
        let mut tree = VcsTree::new();
        self.visit_dir_collect(&self.project_root, "", &mut tree)?;
        tree.recompute_hash();
        Ok(tree)
    }

    fn visit_dir_collect(
        &self,
        current_dir: &Path,
        rel_prefix: &str,
        tree: &mut VcsTree,
    ) -> Result<()> {
        for entry in fs::read_dir(current_dir)? {
            let entry = entry?;
            let file_name = entry.file_name().to_string_lossy().to_string();
            let rel_path = if rel_prefix.is_empty() {
                file_name.clone()
            } else {
                format!("{rel_prefix}/{file_name}")
            };

            if self.ignore_filter.is_ignored(&rel_path) {
                continue;
            }

            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                self.visit_dir_collect(&entry.path(), &rel_path, tree)?;
            } else if file_type.is_file() {
                let metadata = entry.metadata()?;
                let is_executable = {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        metadata.permissions().mode() & 0o111 != 0
                    }
                    #[cfg(not(unix))]
                    {
                        false
                    }
                };

                let data = fs::read(entry.path())?;
                let (chunks, blake3_hash, git_sha1) =
                    self.cas.put_file_data(&data, self.cdc_config)?;

                let file_entry = FileEntry {
                    path: rel_path,
                    size: metadata.len(),
                    is_executable,
                    chunks,
                    blake3_hash,
                    git_sha1,
                    modified_at: Utc::now(),
                };

                tree.insert(file_entry);
            }
        }
        Ok(())
    }

    /// Explicitly acquire the cross-boundary advisory lock on `.apich/lock`
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn acquire_lock(&self) -> Result<crate::lock::RepositoryLock> {
        crate::lock::RepositoryLock::acquire(&self.project_root)
    }

    /// Commit the current working copy into an immutable Snapshot
    ///
    /// # Errors
    /// Returns an error if creating or retrieving the snapshot fails.
    pub fn snapshot(
        &self,
        message: impl Into<String>,
    ) -> Result<Snapshot> {
        let _lock = crate::lock::RepositoryLock::acquire(&self.project_root)?;
        let tree = self.scan_working_tree()?;
        self.cas.put_tree(&tree)?;

        let current_head = self.head_snapshot()?;
        let parent_id = current_head.as_ref().map(|s| s.id);

        // Deduplication: if directory content is completely identical, return existing snapshot
        if let Some(ref head) = current_head {
            if head.tree_hash == tree.tree_hash {
                return Ok(head.clone());
            }
        }

        let snapshot = Snapshot::new(tree.tree_hash, message, parent_id, "User");
        self.cas.put_snapshot(&snapshot)?;

        // Update current branch pointer
        let branch_name = self.current_branch()?.unwrap_or_else(|| "main".to_string());
        let branch = Branch::new(&branch_name, snapshot.id);
        self.save_branch(&branch)?;

        // Record operation in OpLog
        let op = VcsOperation::new(
            OpAction::Snapshot,
            parent_id,
            Some(snapshot.id),
            snapshot.message.clone(),
        );
        self.oplog.append(&op)?;

        Ok(snapshot)
    }

    /// Commit the working copy and detach-sign with the caller's local GPG keyring.
    ///
    /// Acts exactly like `snapshot()`, then detach-signs the resulting
    /// snapshot's canonical payload with the caller's local GPG keyring and stores the signature
    /// on the snapshot record. `key_id` selects which local secret key to sign with (fingerprint,
    /// key ID, or email `gpg` can resolve); `None` uses `gpg`'s own configured default key.
    ///
    /// Signing is always local: the private key never leaves the caller's machine or this
    /// process's `gpg` invocation -- only the resulting signature is persisted.
    ///
    /// # Errors
    /// Returns an error if creating or retrieving the snapshot fails.
    pub fn snapshot_signed(
        &self,
        message: impl Into<String>,
        key_id: Option<&str>,
    ) -> Result<Snapshot> {
        let mut snapshot = self.snapshot(message)?;
        let signature = crate::gpg::sign_payload(&snapshot.signing_payload(), key_id)?;
        snapshot.gpg_signature = Some(signature);
        snapshot.gpg_key_id = key_id.map(std::string::ToString::to_string);
        self.cas.put_snapshot(&snapshot)?;
        Ok(snapshot)
    }

    /// Verify a snapshot's GPG signature against a specific registered public key.
    ///
    /// Returns `Ok(None)` if the snapshot carries no signature at all (distinct from a present-but-invalid
    /// signature, which is `Ok(Some(SignatureStatus::Invalid(..)))`).
    ///
    /// # Errors
    /// Returns an error if creating or retrieving the snapshot fails.
    pub fn verify_snapshot_signature(
        &self,
        snapshot_id: Uuid,
        public_key_armored: &str,
    ) -> Result<Option<crate::gpg::SignatureStatus>> {
        let snapshot = self.cas.get_snapshot(snapshot_id)?;
        let Some(ref sig) = snapshot.gpg_signature else {
            return Ok(None);
        };
        let status =
            crate::gpg::verify_signature(&snapshot.signing_payload(), sig, public_key_armored)?;
        Ok(Some(status))
    }

    /// Check whether there are uncommitted changes in the working directory compared to HEAD
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn has_changes(&self) -> Result<bool> {
        let tree = self.scan_working_tree()?;
        if let Some(head) = self.head_snapshot()? {
            Ok(head.tree_hash != tree.tree_hash)
        } else {
            Ok(!tree.entries.is_empty())
        }
    }

    /// Commit a snapshot only if there are actual modifications compared to HEAD.
    /// Returns `Ok(Some(snapshot))` if a new snapshot was committed, or `Ok(None)` if no changes were detected.
    ///
    /// # Errors
    /// Returns an error if creating or retrieving the snapshot fails.
    pub fn snapshot_if_changed(
        &self,
        message: impl Into<String>,
    ) -> Result<Option<Snapshot>> {
        let _lock = crate::lock::RepositoryLock::acquire(&self.project_root)?;
        let tree = self.scan_working_tree()?;
        let current_head = self.head_snapshot()?;

        if let Some(ref head) = current_head {
            if head.tree_hash == tree.tree_hash {
                return Ok(None);
            }
        } else if tree.entries.is_empty() {
            return Ok(None);
        }

        let snap = self.snapshot(message)?;
        Ok(Some(snap))
    }

    /// Check if debounced autosave trigger is pending, and if so, commit an autosave snapshot
    /// ONLY IF there are actual changes in the working directory.
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub async fn trigger_autosave_if_needed(&self) -> Result<Option<Snapshot>> {
        if !self.autosave.should_trigger_snapshot().await {
            return Ok(None);
        }

        let snap = self.snapshot_if_changed("Continuous auto-save")?;
        self.autosave.reset_pending();
        Ok(snap)
    }

    /// Mark an explicit Milestone checkpoint
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn create_milestone(
        &self,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Result<Snapshot> {
        let name_str = name.into();
        let desc_str = description.into();

        // Ensure current state is snapshot
        let mut snapshot = self.snapshot(format!("Milestone: {name_str}"))?;
        snapshot.mark_milestone(&name_str, &desc_str);
        self.cas.put_snapshot(&snapshot)?;

        // Record milestone in OpLog
        let op = VcsOperation::new(
            OpAction::CreateMilestone,
            None,
            Some(snapshot.id),
            format!("Created milestone '{name_str}'"),
        );
        self.oplog.append(&op)?;

        Ok(snapshot)
    }

    /// List all snapshots in the project timeline
    ///
    /// # Errors
    /// Returns an error if creating or retrieving the snapshot fails.
    pub fn list_snapshots(&self) -> Result<Vec<Snapshot>> {
        self.cas.list_snapshots()
    }

    /// Retrieve a snapshot by UUID from CAS
    ///
    /// # Errors
    /// Returns an error if creating or retrieving the snapshot fails.
    pub fn get_snapshot(
        &self,
        id: Uuid,
    ) -> Result<Snapshot> {
        self.cas.get_snapshot(id)
    }

    /// Retrieve a directory tree by hash from CAS
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn get_tree(
        &self,
        hash: &str,
    ) -> Result<VcsTree> {
        self.cas.get_tree(hash)
    }

    /// List all milestones
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn list_milestones(&self) -> Result<Vec<Snapshot>> {
        let all = self.list_snapshots()?;
        Ok(all.into_iter().filter(|s| s.is_milestone).collect())
    }

    /// Inspect the working directory status compared to HEAD snapshot
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn status(&self) -> Result<RepoStatus> {
        let branch = self.current_branch()?.unwrap_or_else(|| "main".to_string());
        let head = self.head_snapshot()?;
        let working_tree = self.scan_working_tree()?;

        let (added, modified, removed) = if let Some(ref h) = head {
            let head_tree = self.cas.get_tree(&h.tree_hash)?;
            let diff = working_tree.diff(&head_tree);
            let added = diff.added.into_iter().map(|e| e.path.clone()).collect();
            let modified = diff
                .modified
                .into_iter()
                .map(|(cur, _)| cur.path.clone())
                .collect();
            let removed = diff.removed.into_iter().map(|e| e.path.clone()).collect();
            (added, modified, removed)
        } else {
            let added = working_tree.entries.keys().cloned().collect();
            (added, Vec::new(), Vec::new())
        };

        Ok(RepoStatus {
            branch,
            head_snapshot: head,
            added,
            modified,
            removed,
            total_files: working_tree.entries.len(),
        })
    }

    // --- Revert, Undo, and Redo ---

    /// Materialize a snapshot's tree onto the disk working directory without modifying branch pointers
    ///
    /// # Errors
    /// Returns an error if restoring or checking out the tree fails.
    pub fn checkout_tree(
        &self,
        snapshot_id: Uuid,
    ) -> Result<()> {
        let snapshot = self.cas.get_snapshot(snapshot_id)?;
        let target_tree = self.cas.get_tree(&snapshot.tree_hash)?;
        let current_head = self.head_snapshot()?;

        // Write all target files to disk
        for (rel_path, entry) in &target_tree.entries {
            let dest_path = self.project_root.join(rel_path);
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let data = self.cas.read_file_data(&entry.chunks)?;
            fs::write(dest_path, data)?;
        }

        // Clean up files in current tree that don't exist in target tree
        if let Some(head) = current_head {
            if let Ok(head_tree) = self.cas.get_tree(&head.tree_hash) {
                for rel_path in head_tree.entries.keys() {
                    if !target_tree.entries.contains_key(rel_path) {
                        let path = self.project_root.join(rel_path);
                        if path.exists() {
                            let _ = fs::remove_file(path);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Revert working copy and branch HEAD to a specific snapshot
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn revert_to(
        &self,
        snapshot_id: Uuid,
    ) -> Result<()> {
        let _lock = crate::lock::RepositoryLock::acquire(&self.project_root)?;
        let current_head = self.head_snapshot()?;
        let before_id = current_head.as_ref().map(|s| s.id);

        self.checkout_tree(snapshot_id)?;

        // Update branch HEAD
        let branch_name = self.current_branch()?.unwrap_or_else(|| "main".to_string());
        let branch = Branch::new(&branch_name, snapshot_id);
        self.save_branch(&branch)?;

        let op = VcsOperation::new(
            OpAction::Revert,
            before_id,
            Some(snapshot_id),
            format!("Reverted working copy to snapshot {snapshot_id}"),
        );
        self.oplog.append(&op)?;

        Ok(())
    }

    /// Undo last operation in `OpLog`
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn undo(&self) -> Result<Option<Uuid>> {
        let _lock = crate::lock::RepositoryLock::acquire(&self.project_root)?;
        let current_id = self.head_snapshot()?.map(|s| s.id);
        if let Some(target_id) = self.oplog.undo(current_id)? {
            self.checkout_tree(target_id)?;
            let branch_name = self.current_branch()?.unwrap_or_else(|| "main".to_string());
            let branch = Branch::new(&branch_name, target_id);
            self.save_branch(&branch)?;
            return Ok(Some(target_id));
        }
        Ok(None)
    }

    /// Redo last undone operation in `OpLog`
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn redo(&self) -> Result<Option<Uuid>> {
        let _lock = crate::lock::RepositoryLock::acquire(&self.project_root)?;
        let current_id = self.head_snapshot()?.map(|s| s.id);
        if let Some(target_id) = self.oplog.redo(current_id)? {
            self.checkout_tree(target_id)?;
            let branch_name = self.current_branch()?.unwrap_or_else(|| "main".to_string());
            let branch = Branch::new(&branch_name, target_id);
            self.save_branch(&branch)?;
            return Ok(Some(target_id));
        }
        Ok(None)
    }

    // --- Branch Operations ---

    /// Creates a new branch pointing to the current HEAD snapshot.
    ///
    /// # Errors
    /// Returns an error if branch operations or reference updates fail.
    pub fn branch_create(
        &self,
        name: &str,
    ) -> Result<()> {
        let _lock = crate::lock::RepositoryLock::acquire(&self.project_root)?;
        let current_head = self
            .head_snapshot()?
            .ok_or_else(|| VcsError::Internal("No snapshots exist to branch from".to_string()))?;

        let branch = Branch::new(name, current_head.id);
        self.save_branch(&branch)?;
        let _ = self.git_bridge.branch_create(name);
        Ok(())
    }

    /// Switches the working copy to the specified branch.
    ///
    /// # Errors
    /// Returns an error if branch operations or reference updates fail.
    pub fn branch_switch(
        &self,
        name: &str,
    ) -> Result<()> {
        let _lock = crate::lock::RepositoryLock::acquire(&self.project_root)?;
        let branch = self
            .get_branch(name)?
            .ok_or_else(|| VcsError::BranchNotFound(name.to_string()))?;

        self.checkout_tree(branch.head_snapshot_id)?;
        self.set_current_branch(name)?;
        let _ = self.git_bridge.branch_switch(name);
        Ok(())
    }

    // --- Weave-Free Merge ---

    /// Reconcile and merge another branch into the current branch without pointer weaving
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn merge(
        &self,
        other_branch_name: &str,
    ) -> Result<ReconcileResult> {
        let _lock = crate::lock::RepositoryLock::acquire(&self.project_root)?;
        let other_branch = self
            .get_branch(other_branch_name)?
            .ok_or_else(|| VcsError::BranchNotFound(other_branch_name.to_string()))?;

        let current_head = self
            .head_snapshot()?
            .ok_or_else(|| VcsError::Internal("Current branch has no snapshots".to_string()))?;

        let other_snapshot = self.cas.get_snapshot(other_branch.head_snapshot_id)?;

        let ours_tree = self.cas.get_tree(&current_head.tree_hash)?;
        let theirs_tree = self.cas.get_tree(&other_snapshot.tree_hash)?;

        // Find common ancestor
        let mut ours_ancestors = HashSet::new();
        let mut curr = Some(current_head.id);
        while let Some(id) = curr {
            ours_ancestors.insert(id);
            curr = self
                .cas
                .get_snapshot(id)
                .ok()
                .and_then(|s| s.parent_snapshot_id);
        }

        let mut common_ancestor_id = None;
        let mut curr = Some(other_snapshot.id);
        while let Some(id) = curr {
            if ours_ancestors.contains(&id) {
                common_ancestor_id = Some(id);
                break;
            }
            curr = self
                .cas
                .get_snapshot(id)
                .ok()
                .and_then(|s| s.parent_snapshot_id);
        }

        let base_tree = common_ancestor_id
            .and_then(|anc_id| self.cas.get_snapshot(anc_id).ok())
            .and_then(|anc_snap| self.cas.get_tree(&anc_snap.tree_hash).ok());

        // Reconcile
        let reconciler = Reconciler::new(&self.cas, self.cdc_config);
        let result = reconciler.reconcile(base_tree.as_ref(), &ours_tree, &theirs_tree)?;

        // Materialize merged tree on disk
        for (rel_path, entry) in &result.merged_tree.entries {
            let dest = self.project_root.join(rel_path);
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            let data = self.cas.read_file_data(&entry.chunks)?;
            fs::write(dest, data)?;
        }

        // Remove files that existed in ours but were deleted in the merge
        for rel_path in ours_tree.entries.keys() {
            if !result.merged_tree.entries.contains_key(rel_path) {
                let p = self.project_root.join(rel_path);
                if p.exists() {
                    let _ = fs::remove_file(p);
                }
            }
        }

        // Create unified merge snapshot (single parent, no woven criss-cross parent pointers!)
        let merge_snapshot = self.snapshot(format!(
            "Merge branch '{}' into '{}' (Weave-free)",
            other_branch_name,
            self.current_branch()?.unwrap_or_default()
        ))?;

        let op = VcsOperation::new(
            OpAction::Merge,
            Some(current_head.id),
            Some(merge_snapshot.id),
            format!("Merged branch '{other_branch_name}'"),
        );
        self.oplog.append(&op)?;

        Ok(result)
    }

    // --- Garbage Collection & Retention ---

    /// Run timeline exponential decay pruning and CAS unreferenced chunk cleanup
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn run_gc(
        &self,
        policy: Option<&RetentionPolicy>,
    ) -> Result<GcStats> {
        let _lock = crate::lock::RepositoryLock::acquire(&self.project_root)?;
        let pol = policy.unwrap_or(&self.retention_policy);

        let snapshots = self.list_snapshots()?;
        let prune_ids = pol.select_snapshots_to_prune(&snapshots, Utc::now());

        // Delete pruned snapshots
        for id in &prune_ids {
            self.cas.remove_snapshot(*id)?;
        }

        // Collect all currently referenced chunks from remaining snapshots
        let remaining_snapshots = self.list_snapshots()?;
        let mut referenced_chunks = HashSet::new();

        for s in remaining_snapshots {
            if let Ok(tree) = self.cas.get_tree(&s.tree_hash) {
                for entry in tree.entries.values() {
                    for chunk in &entry.chunks {
                        referenced_chunks.insert(chunk.hash.clone());
                    }
                }
            }
        }

        let gc = GarbageCollector::new(&self.cas);
        let stats = gc.sweep_unreferenced(&referenced_chunks)?;
        Ok(stats)
    }

    // --- Git Integration ---

    /// Initializes an underlying Git repository if not already present.
    ///
    /// # Errors
    /// Returns an error if initializing or opening the repository fails.
    pub fn git_init(&self) -> Result<()> {
        self.git_bridge.open_or_init()?;
        Ok(())
    }

    /// Exports an apich snapshot as a standard Git commit on the specified branch.
    ///
    /// # Errors
    /// Returns an error if creating or retrieving the snapshot fails.
    pub fn git_export_commit(
        &self,
        branch_name: &str,
        message: &str,
        author_name: &str,
        author_email: &str,
    ) -> Result<String> {
        let mut snapshot = self.snapshot(message)?;
        let tree = self.cas.get_tree(&snapshot.tree_hash)?;

        let mut export_snap = snapshot.clone();
        export_snap.message = message.to_string();

        let commit_oid = self.git_bridge.export_snapshot(
            &export_snap,
            &tree,
            &self.cas,
            branch_name,
            author_name,
            author_email,
        )?;

        // Attach Git OID to snapshot
        snapshot.git_commit_oid = Some(commit_oid.clone());
        self.cas.put_snapshot(&snapshot)?;

        Ok(commit_oid)
    }

    /// Configures a remote Git repository URL.
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn git_setup_remote(
        &self,
        name: &str,
        url: &str,
    ) -> Result<()> {
        self.git_bridge.remote_add(name, url)
    }

    /// Pushes the specified branch to a Git remote with optional auth token.
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn git_push_with_auth(
        &self,
        remote: &str,
        branch: &str,
        auth_token: Option<&str>,
    ) -> Result<()> {
        self.git_bridge.push_with_auth(remote, branch, auth_token)
    }

    /// Pushes the specified branch to a Git remote.
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn git_push(
        &self,
        remote: &str,
        branch: &str,
    ) -> Result<()> {
        self.git_bridge.push(remote, branch)
    }

    /// Pulls changes from a Git remote branch into the current branch with optional auth token.
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn git_pull_with_auth(
        &self,
        remote: &str,
        branch: &str,
        auth_token: Option<&str>,
    ) -> Result<()> {
        self.git_bridge.pull_with_auth(remote, branch, auth_token)
    }

    /// Pulls changes from a Git remote branch into the current branch.
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn git_pull(
        &self,
        remote: &str,
        branch: &str,
    ) -> Result<()> {
        self.git_bridge.pull(remote, branch)
    }

    /// Lists configured Git remotes as name-URL pairs.
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn git_remotes(&self) -> Result<Vec<(String, String)>> {
        self.git_bridge.remote_list()
    }

    /// Fetches changes from the specified Git remote with optional auth token.
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn git_fetch_with_auth(
        &self,
        remote: &str,
        auth_token: Option<&str>,
    ) -> Result<()> {
        self.git_bridge.fetch_with_auth(remote, auth_token)
    }

    /// Fetches changes from the specified Git remote.
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn git_fetch(
        &self,
        remote: &str,
    ) -> Result<()> {
        self.git_bridge.fetch(remote)
    }

    /// Rebases current branch on top of upstream.
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn git_rebase(
        &self,
        upstream: &str,
    ) -> Result<()> {
        self.git_bridge.rebase(upstream)
    }

    /// Clone a remote Git repository into `dest`, then initialize apich-vcs tracking on the
    /// result and take an initial snapshot -- so a cloned project is immediately usable both as a
    /// Git working copy and as an apich-vcs history, not just a bare checkout.
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn git_clone(
        url: &str,
        dest: impl AsRef<Path>,
    ) -> Result<Self> {
        let dest = dest.as_ref();
        GitBridge::clone_repo(url, dest)?;
        let vcs = Self::open_or_init(dest)?;
        let _ = vcs.snapshot("Initial import from git clone");
        Ok(vcs)
    }

    /// Clones an external Git repository as read-only reference material.
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn clone_material(
        &self,
        url: &str,
        rel_dir: &str,
    ) -> Result<MaterialRecord> {
        self.material_mgr.clone_material(url, rel_dir)
    }

    /// Lists all cloned reference materials in the repository.
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn list_materials(&self) -> Result<Vec<MaterialRecord>> {
        self.material_mgr.list()
    }

    /// Read file content from a specific snapshot, or from the current working copy if `snapshot_id` is None
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn read_file_content(
        &self,
        snapshot_id: Option<Uuid>,
        rel_path: &str,
    ) -> Result<Vec<u8>> {
        if let Some(id) = snapshot_id {
            let snap = self.cas.get_snapshot(id)?;
            let tree = self.cas.get_tree(&snap.tree_hash)?;
            let entry = tree.entries.get(rel_path).ok_or_else(|| {
                VcsError::InvalidPath(format!("File '{rel_path}' not found in snapshot {id}"))
            })?;
            self.cas.read_file_data(&entry.chunks)
        } else {
            let disk_path = self.project_root.join(rel_path);
            if !disk_path.exists() {
                return Err(VcsError::InvalidPath(format!(
                    "File '{rel_path}' does not exist on disk"
                )));
            }
            Ok(fs::read(disk_path)?)
        }
    }

    /// List all operations in the `OpLog` (snapshots, undos, redos, merges, reverts)
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn oplog_list(&self) -> Result<Vec<VcsOperation>> {
        self.oplog.list()
    }

    /// Compute file-level differences between two snapshots: (added, modified, removed)
    ///
    /// # Errors
    /// Returns an error if creating or retrieving the snapshot fails.
    pub fn diff_snapshots(
        &self,
        from_id: Uuid,
        to_id: Uuid,
    ) -> Result<(Vec<String>, Vec<String>, Vec<String>)> {
        let from_snap = self.cas.get_snapshot(from_id)?;
        let to_snap = self.cas.get_snapshot(to_id)?;
        let from_tree = self.cas.get_tree(&from_snap.tree_hash)?;
        let to_tree = self.cas.get_tree(&to_snap.tree_hash)?;

        let diff = to_tree.diff(&from_tree);
        let added = diff.added.into_iter().map(|e| e.path.clone()).collect();
        let modified = diff
            .modified
            .into_iter()
            .map(|(cur, _)| cur.path.clone())
            .collect();
        let removed = diff.removed.into_iter().map(|e| e.path.clone()).collect();
        Ok((added, modified, removed))
    }

    /// Compute file-level differences between HEAD snapshot and current working copy
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn diff_working(&self) -> Result<(Vec<String>, Vec<String>, Vec<String>)> {
        let status = self.status()?;
        Ok((status.added, status.modified, status.removed))
    }

    // --- Packaging & Bundle Operations for Gateway / Local Work ---

    /// Export the full project repository and state to a compressed stream (e.g. for gateway HTTP download)
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn export_bundle<W: std::io::Write>(
        &self,
        writer: W,
        options: crate::bundle::BundleOptions,
    ) -> Result<()> {
        crate::bundle::ProjectBundle::export(self, writer, options)
    }

    /// Export the full project repository to a local bundle file
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn export_bundle_to_file(
        &self,
        dest_path: impl AsRef<Path>,
        options: crate::bundle::BundleOptions,
    ) -> Result<()> {
        crate::bundle::ProjectBundle::export_to_file(self, dest_path, options)
    }

    /// Import a project bundle from a stream into a local directory and checkout the working copy
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn import_bundle<R: std::io::Read>(
        reader: R,
        target_dir: impl AsRef<Path>,
    ) -> Result<Self> {
        crate::bundle::ProjectBundle::import(reader, target_dir)
    }

    /// Import a project bundle file from disk into a target directory
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn import_bundle_from_file(
        bundle_path: impl AsRef<Path>,
        target_dir: impl AsRef<Path>,
    ) -> Result<Self> {
        crate::bundle::ProjectBundle::import_from_file(bundle_path, target_dir)
    }

    /// Accept a bundle (uploaded as a push, or downloaded as a pull -- the merge is safe either
    /// direction) into this *already-initialized* project. See
    /// `crate::bundle::ProjectBundle::accept_push` for the fast-forward safety rules.
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn accept_push_bundle<R: std::io::Read>(
        &self,
        reader: R,
    ) -> Result<crate::bundle::PushOutcome> {
        crate::bundle::ProjectBundle::accept_push(self, reader)
    }

    /// Export a clean archive (e.g. tar.gz) of a specific snapshot without .apich metadata (ideal for arXiv / IEEE / zip download)
    ///
    /// # Errors
    /// Returns an error if creating or retrieving the snapshot fails.
    pub fn export_snapshot_archive<W: std::io::Write>(
        &self,
        snapshot_id: Uuid,
        writer: W,
    ) -> Result<()> {
        crate::bundle::ProjectBundle::export_snapshot_archive(self, snapshot_id, writer)
    }

    /// Export a clean snapshot archive directly to a file
    ///
    /// # Errors
    /// Returns an error if creating or retrieving the snapshot fails.
    pub fn export_snapshot_archive_to_file(
        &self,
        snapshot_id: Uuid,
        dest_path: impl AsRef<Path>,
    ) -> Result<()> {
        crate::bundle::ProjectBundle::export_snapshot_archive_to_file(self, snapshot_id, dest_path)
    }

    // --- Configuration & Customization APIs for Upper Layer ---

    /// Access the active project configuration
    #[must_use]
    pub const fn config(&self) -> &VcsConfig {
        &self.config
    }

    /// Access the live ignore filter
    #[must_use]
    pub const fn ignore_filter(&self) -> &IgnoreFilter {
        &self.ignore_filter
    }

    /// Apply a new configuration to the running VCS instance
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn apply_config(
        &mut self,
        config: VcsConfig,
    ) -> Result<()> {
        self.ignore_filter = config.to_ignore_filter(&self.project_root)?;
        let lfs_policy = config.to_lfs_policy(&self.project_root);
        self.git_bridge.set_lfs_policy(lfs_policy);
        self.cdc_config = config.to_cdc_config();
        self.retention_policy = config.to_retention_policy();
        self.config = config;
        Ok(())
    }

    /// Save the current configuration to disk (.apich/config.toml or apich.toml)
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn save_config(&self) -> Result<PathBuf> {
        let _lock = crate::lock::RepositoryLock::acquire(&self.project_root)?;
        self.config.save_to_project(&self.project_root)
    }

    /// Reload configuration from project files (.apich/config.toml or apich.toml)
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn reload_config(&mut self) -> Result<()> {
        let cfg = VcsConfig::load_from_project(&self.project_root)?;
        self.apply_config(cfg)
    }

    /// Add a custom ignore rule programmatically (supports globs and negative '!path' rules)
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn add_ignore_rule(
        &mut self,
        rule: impl Into<String>,
    ) -> Result<()> {
        let r = rule.into();
        self.config.ignore.custom_rules.push(r.clone());
        self.ignore_filter.add_rule(r)
    }

    /// Remove a custom ignore rule programmatically
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn remove_ignore_rule(
        &mut self,
        rule: &str,
    ) -> Result<()> {
        self.config.ignore.custom_rules.retain(|r| r != rule);
        self.ignore_filter.remove_rule(rule)
    }

    /// Enable an ignore profile programmatically
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn enable_ignore_profile(
        &mut self,
        profile: crate::ignore::IgnoreProfile,
    ) -> Result<()> {
        self.config.ignore.set_profile_enabled(profile, true);
        self.ignore_filter.enable_profile(profile)
    }

    /// Disable an ignore profile programmatically
    ///
    /// # Errors
    /// Returns an error if the VCS operation fails.
    pub fn disable_ignore_profile(
        &mut self,
        profile: crate::ignore::IgnoreProfile,
    ) -> Result<()> {
        self.config.ignore.set_profile_enabled(profile, false);
        self.ignore_filter.disable_profile(profile)
    }

    /// Add a wildcard or glob pattern candidate for automatic Git LFS pointer synthesis
    pub fn add_lfs_pattern(
        &mut self,
        pattern: impl Into<String>,
    ) {
        let p = pattern.into();
        self.config.lfs.patterns.push(p.clone());
        self.git_bridge.lfs_policy_mut().add_pattern(p);
    }

    /// Set file size threshold in bytes to trigger automatic Git LFS pointer generation
    pub const fn set_lfs_size_threshold(
        &mut self,
        threshold_bytes: u64,
    ) {
        self.config.lfs.size_threshold_bytes = threshold_bytes;
        self.git_bridge
            .lfs_policy_mut()
            .set_size_threshold(threshold_bytes);
    }

    /// Access the Git LFS policy
    #[must_use]
    pub const fn lfs_policy(&self) -> &crate::git::LfsPolicy {
        self.git_bridge.lfs_policy()
    }

    /// Set the retention policy programmatically
    pub const fn set_retention_policy(
        &mut self,
        policy: RetentionPolicy,
    ) {
        self.retention_policy = policy;
    }

    /// Access the active retention policy
    #[must_use]
    pub const fn retention_policy(&self) -> &RetentionPolicy {
        &self.retention_policy
    }

    /// Set the `FastCDC` chunking configuration programmatically
    pub const fn set_cdc_config(
        &mut self,
        config: FastCdcConfig,
    ) {
        self.cdc_config = config;
    }

    /// Access the active `FastCDC` chunking configuration
    #[must_use]
    pub const fn cdc_config(&self) -> FastCdcConfig {
        self.cdc_config
    }
}

/// Status report of a repository working copy relative to HEAD
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RepoStatus {
    /// Active branch name.
    pub branch: String,
    /// HEAD snapshot if one exists.
    pub head_snapshot: Option<Snapshot>,
    /// Relative paths of newly added files.
    pub added: Vec<String>,
    /// Relative paths of modified files.
    pub modified: Vec<String>,
    /// Relative paths of deleted files.
    pub removed: Vec<String>,
    /// Total number of tracked files in the workspace.
    pub total_files: usize,
}

impl RepoStatus {
    /// Returns true if there are no uncommitted additions, modifications, or deletions.
    #[must_use]
    pub const fn is_clean(&self) -> bool {
        self.added.is_empty() && self.modified.is_empty() && self.removed.is_empty()
    }
}
