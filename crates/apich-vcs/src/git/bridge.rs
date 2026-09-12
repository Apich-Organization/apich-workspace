//! Git compatibility bridge implementation.

use super::lfs::LfsPolicy;
use crate::error::Result;
use crate::error::VcsError;
use crate::model::Snapshot;
use crate::model::VcsTree;
use crate::storage::ContentAddressableStorage;
use git2::Oid;
use git2::Repository;
use git2::Signature;
use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

/// Git Compatibility Bridge for commit and tree translation.
///
/// Allows bidirectional translation between apich-vcs snapshots/CAS
/// and standard Git commit graphs.
pub struct GitBridge {
    project_root: PathBuf,
    lfs_policy: LfsPolicy,
}

impl GitBridge {
    /// Creates a new `GitBridge` for the repository located at `project_root`.
    pub fn new(project_root: impl AsRef<Path>) -> Self {
        Self {
            project_root: project_root.as_ref().to_path_buf(),
            lfs_policy: LfsPolicy::default(),
        }
    }

    /// Sets the LFS policy using a builder pattern.
    #[must_use]
    pub fn with_lfs_policy(
        mut self,
        policy: LfsPolicy,
    ) -> Self {
        self.lfs_policy = policy;
        self
    }

    /// Updates the active LFS policy.
    pub fn set_lfs_policy(
        &mut self,
        policy: LfsPolicy,
    ) {
        self.lfs_policy = policy;
    }

    /// Returns a reference to the active LFS policy.
    #[must_use]
    pub const fn lfs_policy(&self) -> &LfsPolicy {
        &self.lfs_policy
    }

    /// Returns a mutable reference to the active LFS policy.
    pub const fn lfs_policy_mut(&mut self) -> &mut LfsPolicy {
        &mut self.lfs_policy
    }

    /// Open existing Git repository or initialize a new one in the project directory
    ///
    /// # Errors
    /// Returns an error if the Git operation or repository conversion fails.
    pub fn open_or_init(&self) -> Result<Repository> {
        match Repository::open(&self.project_root) {
            | Ok(repo) => Ok(repo),
            | Err(_) => {
                let repo = Repository::init(&self.project_root)?;
                Ok(repo)
            },
        }
    }

    /// Export an apich-vcs snapshot and tree into a standard Git commit
    ///
    /// # Errors
    /// Returns an error if the Git operation or repository conversion fails.
    pub fn export_snapshot(
        &self,
        snapshot: &Snapshot,
        tree: &VcsTree,
        cas: &ContentAddressableStorage,
        branch_name: &str,
        author_name: &str,
        author_email: &str,
    ) -> Result<String> {
        let repo = self.open_or_init()?;

        // Build Git Tree recursively
        // Group entries into paths
        let git_tree_oid = self.build_git_tree(&repo, tree, cas)?;
        let git_tree = repo.find_tree(git_tree_oid)?;

        // Prepare Commit
        let sig = Signature::now(author_name, author_email)?;
        let commit_message = format!(
            "{}\n\nApich-Change-Id: {}\nApich-Snapshot-Id: {}\n",
            snapshot.message, snapshot.change_id, snapshot.id
        );

        // Check if branch exists to set parent
        let branch_ref_name = format!("refs/heads/{branch_name}");
        let parent_commit = match repo.find_reference(&branch_ref_name) {
            | Ok(r) => r.peel_to_commit().ok(),
            | Err(_) => None,
        };

        let parents = match &parent_commit {
            | Some(p) => vec![p],
            | None => vec![],
        };

        let commit_oid = repo.commit(
            Some(&branch_ref_name),
            &sig,
            &sig,
            &commit_message,
            &git_tree,
            &parents,
        )?;

        let _ = repo.set_head(&branch_ref_name);

        Ok(commit_oid.to_string())
    }

    /// Internal recursive tree builder
    fn build_git_tree(
        &self,
        repo: &Repository,
        tree: &VcsTree,
        cas: &ContentAddressableStorage,
    ) -> Result<Oid> {
        let mut builder = repo.treebuilder(None)?;

        // Partition files by top-level component
        let mut subdirs: BTreeMap<String, VcsTree> = BTreeMap::new();

        for (path, entry) in &tree.entries {
            if let Some((first, rest)) = path.split_once('/') {
                let sub = subdirs.entry(first.to_string()).or_default();
                let mut sub_entry = entry.clone();
                sub_entry.path = rest.to_string();
                sub.insert(sub_entry);
            } else {
                // Leaf file in current directory
                let file_bytes = cas.read_file_data(&entry.chunks)?;

                let (blob_bytes, file_mode) = if self.lfs_policy.is_lfs_file(path, entry.size) {
                    let (pointer, _) = LfsPolicy::create_lfs_pointer(&file_bytes);
                    (pointer.into_bytes(), 0o100_644)
                } else {
                    let mode = if entry.is_executable {
                        0o100_755
                    } else {
                        0o100_644
                    };
                    (file_bytes, mode)
                };

                let blob_oid = repo.blob(&blob_bytes)?;
                builder.insert(path, blob_oid, file_mode)?;
            }
        }

        // Build subdirectories recursively
        for (subdir_name, sub_tree) in subdirs {
            let sub_oid = self.build_git_tree(repo, &sub_tree, cas)?;
            builder.insert(&subdir_name, sub_oid, 0o040_000)?;
        }

        let tree_oid = builder.write()?;
        Ok(tree_oid)
    }

    // --- Branch Operations ---

    /// Creates a Git branch with the given name pointing to the current HEAD commit.
    ///
    /// # Errors
    /// Returns an error if the Git operation or repository conversion fails.
    pub fn branch_create(
        &self,
        name: &str,
    ) -> Result<()> {
        let repo = self.open_or_init()?;
        if let Ok(head) = repo.head().and_then(|h| h.peel_to_commit()) {
            let _ = repo.branch(name, &head, false);
        }
        Ok(())
    }

    /// Switches the repository HEAD to point to the named Git branch.
    ///
    /// # Errors
    /// Returns an error if the Git operation or repository conversion fails.
    pub fn branch_switch(
        &self,
        name: &str,
    ) -> Result<()> {
        let repo = self.open_or_init()?;
        let ref_name = format!("refs/heads/{name}");
        let _ = repo.set_head(&ref_name);
        Ok(())
    }

    /// Lists all local Git branches.
    ///
    /// # Errors
    /// Returns an error if the Git operation or repository conversion fails.
    pub fn branch_list(&self) -> Result<Vec<String>> {
        let repo = self.open_or_init()?;
        let mut list = Vec::new();
        for branch in repo.branches(Some(git2::BranchType::Local))? {
            let (b, _) = branch?;
            if let Some(name) = b.name()? {
                list.push(name.to_string());
            }
        }
        Ok(list)
    }

    // --- Remote Operations ---

    /// Adds a named remote repository URL.
    ///
    /// # Errors
    /// Returns an error if the Git operation or repository conversion fails.
    pub fn remote_add(
        &self,
        name: &str,
        url: &str,
    ) -> Result<()> {
        let repo = self.open_or_init()?;
        repo.remote(name, url)?;
        Ok(())
    }

    /// Lists all configured remotes as pairs of name and remote URL.
    ///
    /// # Errors
    /// Returns an error if the Git operation or repository conversion fails.
    pub fn remote_list(&self) -> Result<Vec<(String, String)>> {
        let repo = self.open_or_init()?;
        let mut list = Vec::new();
        for name in repo.remotes()?.iter().flatten() {
            if let Ok(r) = repo.find_remote(name) {
                if let Some(url) = r.url() {
                    list.push((name.to_string(), url.to_string()));
                }
            }
        }
        Ok(list)
    }

    // --- Clone / Fetch / Rebase / Push / Pull via Git Subprocess (Robust and Secure) ---
    //
    // All of these shell out to the real `git` binary rather than using git2, matching `push`/
    // `pull` below: credential handling (SSH agent, stored HTTPS credentials, credential helpers)
    // is delegated entirely to the user's own real git installation, instead of reimplementing
    // auth flows here.

    /// Clone a remote Git repository directly into `dest` (which must not already exist).
    ///
    /// # Errors
    /// Returns an error if the Git operation or repository conversion fails.
    pub fn clone_repo(
        url: &str,
        dest: &Path,
    ) -> Result<()> {
        if dest.exists() {
            return Err(VcsError::Internal(format!(
                "Clone destination already exists: {}",
                dest.display()
            )));
        }
        let output = Command::new("git")
            .arg("clone")
            .arg(url)
            .arg(dest)
            .output()
            .map_err(VcsError::Io)?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(VcsError::Internal(format!("git clone failed: {stderr}")));
        }
        Ok(())
    }

    /// Fetches changes from the specified remote repository.
    ///
    /// # Errors
    /// Returns an error if the Git operation or repository conversion fails.
    pub fn fetch(
        &self,
        remote: &str,
    ) -> Result<()> {
        let output = Command::new("git")
            .arg("-C")
            .arg(&self.project_root)
            .arg("fetch")
            .arg(remote)
            .output()
            .map_err(VcsError::Io)?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(VcsError::Internal(format!("git fetch failed: {stderr}")));
        }
        Ok(())
    }

    /// Rebases current branch commits onto the specified upstream branch.
    ///
    /// # Errors
    /// Returns an error if the Git operation or repository conversion fails.
    pub fn rebase(
        &self,
        upstream: &str,
    ) -> Result<()> {
        let output = Command::new("git")
            .arg("-C")
            .arg(&self.project_root)
            .arg("rebase")
            .arg(upstream)
            .output()
            .map_err(VcsError::Io)?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(VcsError::Internal(format!("git rebase failed: {stderr}")));
        }
        Ok(())
    }

    /// Pushes local commits on the specified branch to the remote repository.
    ///
    /// # Errors
    /// Returns an error if the Git operation or repository conversion fails.
    pub fn push(
        &self,
        remote: &str,
        branch: &str,
    ) -> Result<()> {
        let mut cmd = Command::new("git");
        cmd.arg("-C")
            .arg(&self.project_root)
            .arg("push")
            .arg(remote)
            .arg(branch);

        let output = cmd.output().map_err(VcsError::Io)?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(VcsError::Internal(format!("git push failed: {stderr}")));
        }
        Ok(())
    }

    /// Pulls changes from the remote repository branch into the current working copy.
    ///
    /// # Errors
    /// Returns an error if the Git operation or repository conversion fails.
    pub fn pull(
        &self,
        remote: &str,
        branch: &str,
    ) -> Result<()> {
        let mut cmd = Command::new("git");
        cmd.arg("-C")
            .arg(&self.project_root)
            .arg("pull")
            .arg(remote)
            .arg(branch);

        let output = cmd.output().map_err(VcsError::Io)?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(VcsError::Internal(format!("git pull failed: {stderr}")));
        }
        Ok(())
    }
}
