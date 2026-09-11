use crate::error::Result;
use crate::error::VcsError;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

/// Record of an external Git repository cloned into the project as research material
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterialRecord {
    pub name: String,
    pub url: String,
    pub rel_path: String,
    pub commit_oid: String,
    pub cloned_at: DateTime<Utc>,
}

pub struct MaterialManager {
    project_root: PathBuf,
    manifest_file: PathBuf,
}

impl MaterialManager {
    pub fn new(project_root: impl AsRef<Path>) -> Self {
        let root = project_root.as_ref().to_path_buf();
        let manifest = root.join(".apich").join("materials.json");
        Self {
            project_root: root,
            manifest_file: manifest,
        }
    }

    /// List all registered materials
    pub fn list(&self) -> Result<Vec<MaterialRecord>> {
        if !self.manifest_file.exists() {
            return Ok(Vec::new());
        }
        let data = fs::read(&self.manifest_file)?;
        let list: Vec<MaterialRecord> = serde_json::from_slice(&data)?;
        Ok(list)
    }

    /// Clone an external Git repository into a project subfolder as reference material
    pub fn clone_material(
        &self,
        url: &str,
        rel_target_dir: &str,
    ) -> Result<MaterialRecord> {
        let clean_rel = rel_target_dir.trim_start_matches('/');
        let target_path = self.project_root.join(clean_rel);

        if target_path.exists() {
            return Err(VcsError::InvalidPath(format!(
                "Target material path already exists: {}",
                clean_rel
            )));
        }

        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Execute system git clone
        let mut cmd = Command::new("git");
        cmd.arg("clone").arg("--depth=1").arg(url).arg(&target_path);

        let output = cmd.output().map_err(VcsError::Io)?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(VcsError::Internal(format!(
                "Failed to clone external material '{}': {}",
                url, stderr
            )));
        }

        // Get latest commit OID
        let mut rev_cmd = Command::new("git");
        rev_cmd
            .arg("-C")
            .arg(&target_path)
            .arg("rev-parse")
            .arg("HEAD");
        let rev_out = rev_cmd.output().map_err(VcsError::Io)?;
        let commit_oid = String::from_utf8_lossy(&rev_out.stdout).trim().to_string();

        let name = Path::new(clean_rel)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("material")
            .to_string();

        let record = MaterialRecord {
            name,
            url: url.to_string(),
            rel_path: clean_rel.to_string(),
            commit_oid,
            cloned_at: Utc::now(),
        };

        let mut all = self.list()?;
        all.push(record.clone());

        if let Some(parent) = self.manifest_file.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&self.manifest_file, serde_json::to_vec_pretty(&all)?)?;

        Ok(record)
    }
}
