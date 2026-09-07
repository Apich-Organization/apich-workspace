use crate::error::{WebError, WebResult};
use apich_db::{CreateProjectDto, Database, Project, ProjectSandbox};
use apich_sandbox::{SandboxConfig, SandboxManager};
use apich_vcs::{api::ProjectVcs, Snapshot};
use std::{path::PathBuf, sync::Arc};
use tracing::info;
use uuid::Uuid;

#[derive(Clone)]
pub struct ProjectManager {
    db: Arc<Database>,
    sandbox_manager: Arc<SandboxManager>,
    base_storage_dir: PathBuf,
}

impl ProjectManager {
    pub fn new(
        db: Arc<Database>,
        sandbox_manager: Arc<SandboxManager>,
        base_storage_dir: PathBuf,
    ) -> Self {
        Self {
            db,
            sandbox_manager,
            base_storage_dir,
        }
    }

    /// Create a new project, provision storage workspace directory, and initialize FastCDC VCS
    pub async fn create_project(&self, mut dto: CreateProjectDto) -> WebResult<Project> {
        // Ensure storage path is set within base directory if not absolute
        let project_dir = if dto.storage_path.is_empty() {
            self.base_storage_dir
                .join(dto.org_id.to_string())
                .join(&dto.slug)
        } else {
            PathBuf::from(&dto.storage_path)
        };

        tokio::fs::create_dir_all(&project_dir)
            .await
            .map_err(|e| WebError::Internal(format!("Failed to create project workspace directory: {}", e)))?;

        // Initialize FastCDC Version Control in the project directory
        let _ = ProjectVcs::open_or_init(&project_dir)?;

        dto.storage_path = project_dir.to_string_lossy().to_string();

        let repo = self.db.repository();
        let mut proj = repo.create_project(dto).await?;
        repo.set_project_vcs_initialized(proj.id, true).await?;
        proj.vcs_initialized = true;

        info!(project_id = %proj.id, name = %proj.name, path = %proj.storage_path, "Created project with initialized VCS");
        Ok(proj)
    }

    /// Launch a dedicated container sandbox for a specific (Project + User) pair
    pub async fn launch_sandbox(
        &self,
        project_id: Uuid,
        user_id: Uuid,
    ) -> WebResult<ProjectSandbox> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        if proj.status == "archived" || proj.status == "deleted" {
            return Err(WebError::BadRequest(format!(
                "Cannot launch sandbox for {} project",
                proj.status
            )));
        }

        let user_hex = user_id.simple().to_string();
        let user_suffix = &user_hex[24..];
        let container_name = format!(
            "apich-proj-{}-{}",
            proj.slug,
            user_suffix
        );

        let image = std::env::var("APICH_SANDBOX_IMAGE").unwrap_or_else(|_| "alpine:latest".to_string());
        let sandbox_config = SandboxConfig::builder(user_id.to_string(), &proj.storage_path)
            .container_name(&container_name)
            .image(image)
            .memory_limit("2048m")
            .cpu_limit(2.0)
            .build();

        info!(
            project_id = %project_id,
            user_id = %user_id,
            container = %container_name,
            "Launching project sandbox container"
        );

        self.sandbox_manager
            .ensure_running_with_config(sandbox_config)
            .await?;

        let sandbox = repo
            .upsert_project_sandbox(project_id, user_id, &container_name, "running")
            .await?;

        Ok(sandbox)
    }

    /// Stop a dedicated container sandbox for a (Project + User) pair
    pub async fn stop_sandbox(
        &self,
        project_id: Uuid,
        user_id: Uuid,
    ) -> WebResult<ProjectSandbox> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let user_hex = user_id.simple().to_string();
        let user_suffix = &user_hex[24..];
        let container_name = format!(
            "apich-proj-{}-{}",
            proj.slug,
            user_suffix
        );

        info!(
            project_id = %project_id,
            user_id = %user_id,
            container = %container_name,
            "Stopping project sandbox container"
        );

        // Stop container via container manager (ignore error if already stopped)
        let _ = self.sandbox_manager.stop_container(&container_name, 10).await;

        let sandbox = repo
            .upsert_project_sandbox(project_id, user_id, &container_name, "stopped")
            .await?;

        Ok(sandbox)
    }

    /// Take a VCS snapshot if there are working copy modifications
    pub async fn snapshot_project(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        message: &str,
    ) -> WebResult<Option<Snapshot>> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let user_hex = user_id.simple().to_string();
        let user_suffix = &user_hex[24..];
        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        let commit_msg = format!("{} (by user {})", message, user_suffix);
        let summary = vcs.snapshot_if_changed(&commit_msg)?;

        Ok(summary)
    }

    /// Retrieve the VCS snapshot timeline for a project
    pub async fn get_project_timeline(&self, project_id: Uuid) -> WebResult<Vec<Snapshot>> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        let mut snapshots = vcs.list_snapshots()?;
        snapshots.sort_by_key(|a| std::cmp::Reverse(a.created_at));
        Ok(snapshots)
    }

    /// Retrieve branches (current active branch and all branches)
    pub async fn get_branches(&self, project_id: Uuid) -> WebResult<(Option<String>, Vec<String>)> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        let current = vcs.current_branch()?;
        let branches = vcs.list_branches()?;
        Ok((current, branches))
    }

    /// Execute a weave-free 3-way merge from another branch
    pub async fn merge_branch(&self, project_id: Uuid, other_branch: &str) -> WebResult<MergeSummary> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        let reconcile = vcs.merge(other_branch)?;
        let conflicts = self.detect_conflicts(project_id).await?;

        Ok(MergeSummary {
            auto_merged_count: reconcile.auto_merged_count,
            conflicts_count: conflicts.len(),
            conflicts,
        })
    }

    /// Detect files with active conflict markers in the workspace
    pub async fn detect_conflicts(&self, project_id: Uuid) -> WebResult<Vec<ConflictFileView>> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let proj_root = PathBuf::from(&proj.storage_path);
        let mut conflicts = Vec::new();

        let mut dirs = vec![proj_root.clone()];
        while let Some(dir) = dirs.pop() {
            let mut entries = match tokio::fs::read_dir(&dir).await {
                Ok(e) => e,
                Err(_) => continue,
            };

            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                let file_name = entry.file_name();
                let name = file_name.to_string_lossy();

                if name.starts_with('.') || name == "target" || name == "node_modules" {
                    continue;
                }

                if let Ok(ft) = entry.file_type().await {
                    if ft.is_dir() {
                        dirs.push(path);
                    } else if ft.is_file() {
                        if let Ok(content) = tokio::fs::read_to_string(&path).await {
                            if content.contains("<<<<<<<") && content.contains(">>>>>>>") {
                                let rel = path
                                    .strip_prefix(&proj_root)
                                    .unwrap_or(&path)
                                    .to_string_lossy()
                                    .to_string();

                                let (ours, theirs) = parse_conflict_snippets(&content);
                                conflicts.push(ConflictFileView {
                                    path: rel,
                                    full_content: content,
                                    ours_snippet: ours,
                                    theirs_snippet: theirs,
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(conflicts)
    }

    /// Resolve a conflicted file using ours, theirs, or custom resolved content
    pub async fn resolve_conflict(
        &self,
        project_id: Uuid,
        rel_path: &str,
        choice: &str,
        custom_content: Option<&str>,
    ) -> WebResult<()> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let full_path = PathBuf::from(&proj.storage_path).join(rel_path);
        if !full_path.exists() {
            return Err(WebError::NotFound(format!("File '{}' not found", rel_path)));
        }

        let new_content = if let Some(custom) = custom_content {
            custom.to_string()
        } else {
            let raw = tokio::fs::read_to_string(&full_path)
                .await
                .map_err(|e| WebError::Internal(format!("Failed to read file: {}", e)))?;
            resolve_conflict_content(&raw, choice)
        };

        tokio::fs::write(&full_path, new_content.as_bytes())
            .await
            .map_err(|e| WebError::Internal(format!("Failed to write resolved file: {}", e)))?;

        // Create snapshot recording the resolution
        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        let _ = vcs.snapshot_if_changed(format!("Resolved merge conflict in {}", rel_path))?;

        Ok(())
    }

    /// Synchronize project history with Git bridge
    pub async fn git_sync(&self, project_id: Uuid, commit_message: &str) -> WebResult<Option<String>> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        vcs.git_init()?;

        let current_branch = vcs.current_branch()?.unwrap_or_else(|| "main".to_string());
        if vcs.head_snapshot()?.is_some() {
            let commit_oid = vcs.git_export_commit(
                &current_branch,
                commit_message,
                "APICH System",
                "dev@apich.org",
            )?;
            Ok(Some(commit_oid))
        } else {
            Ok(None)
        }
    }

    /// Retrieve Git bridge status
    pub async fn get_git_status(&self, project_id: Uuid) -> WebResult<GitStatusView> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        let git_dir = PathBuf::from(&proj.storage_path).join(".git");
        let initialized = git_dir.exists();

        let remote_url = vcs
            .git_remotes()
            .ok()
            .and_then(|r| r.first().map(|(_, u)| u.clone()));

        let branches = vcs.list_branches().unwrap_or_default();
        let current_branch = vcs.current_branch().ok().flatten();

        Ok(GitStatusView {
            initialized,
            remote_url,
            branches,
            current_branch,
        })
    }

    /// Archive a project and cleanly terminate any active containers
    pub async fn archive_project(&self, project_id: Uuid) -> WebResult<()> {
        let repo = self.db.repository();
        repo.update_project_status(project_id, "archived").await?;
        info!(project_id = %project_id, "Project archived");
        Ok(())
    }

    /// Delete a project (soft delete)
    pub async fn delete_project(&self, project_id: Uuid) -> WebResult<()> {
        let repo = self.db.repository();
        repo.update_project_status(project_id, "deleted").await?;
        info!(project_id = %project_id, "Project marked deleted");
        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConflictFileView {
    pub path: String,
    pub full_content: String,
    pub ours_snippet: String,
    pub theirs_snippet: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MergeSummary {
    pub auto_merged_count: usize,
    pub conflicts_count: usize,
    pub conflicts: Vec<ConflictFileView>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GitStatusView {
    pub initialized: bool,
    pub remote_url: Option<String>,
    pub branches: Vec<String>,
    pub current_branch: Option<String>,
}

pub fn resolve_conflict_content(raw: &str, choice: &str) -> String {
    let mut result = String::new();
    let mut in_conflict = false;
    let mut in_ours = false;
    let mut in_theirs = false;
    let mut ours_buf = String::new();
    let mut theirs_buf = String::new();

    for line in raw.lines() {
        if line.starts_with("<<<<<<<") {
            in_conflict = true;
            in_ours = true;
            in_theirs = false;
            ours_buf.clear();
            theirs_buf.clear();
        } else if in_conflict && line.starts_with("=======") {
            in_ours = false;
            in_theirs = true;
        } else if in_conflict && line.starts_with(">>>>>>>") {
            in_conflict = false;
            in_ours = false;
            in_theirs = false;
            if choice == "theirs" {
                result.push_str(&theirs_buf);
            } else {
                result.push_str(&ours_buf);
            }
        } else if in_ours {
            ours_buf.push_str(line);
            ours_buf.push('\n');
        } else if in_theirs {
            theirs_buf.push_str(line);
            theirs_buf.push('\n');
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }
    result
}

fn parse_conflict_snippets(content: &str) -> (String, String) {
    let mut ours = String::new();
    let mut theirs = String::new();
    let mut in_ours = false;
    let mut in_theirs = false;

    for line in content.lines() {
        if line.starts_with("<<<<<<<") {
            in_ours = true;
            in_theirs = false;
        } else if in_ours && line.starts_with("=======") {
            in_ours = false;
            in_theirs = true;
        } else if in_theirs && line.starts_with(">>>>>>>") {
            in_ours = false;
            in_theirs = false;
        } else if in_ours {
            ours.push_str(line);
            ours.push('\n');
        } else if in_theirs {
            theirs.push_str(line);
            theirs.push('\n');
        }
    }

    (ours.trim().to_string(), theirs.trim().to_string())
}
