use crate::autosave::AutosaveConfig as RuntimeAutosaveConfig;
use crate::chunking::FastCdcConfig;
use crate::error::Result;
use crate::error::VcsError;
use crate::git::LfsPolicy;
use crate::history::RetentionPolicy;
use crate::ignore::IgnoreFilter;
use crate::ignore::IgnoreProfile;
use chrono::Duration;
use serde::Deserialize;
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

fn default_true() -> bool {
    true
}

fn default_lfs_threshold() -> u64 {
    50 * 1024 * 1024 // 50 MB
}

fn default_lfs_patterns() -> Vec<String> {
    vec![
        "*.sqlite".to_string(),
        "*.db".to_string(),
        "*.parquet".to_string(),
        "*.bin".to_string(),
        "*.onnx".to_string(),
        "*.pt".to_string(),
        "*.tar.gz".to_string(),
        "*.zip".to_string(),
    ]
}

fn default_chunk_profile() -> String {
    "document".to_string()
}

fn default_keep_all_hours() -> i64 {
    24
}

fn default_hourly_days() -> i64 {
    7
}

fn default_daily_days() -> i64 {
    30
}

fn default_weekly_days() -> i64 {
    365
}

fn default_debounce_ms() -> u64 {
    1500
}

/// Project-level configuration for APICH Version Control System
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VcsConfig {
    #[serde(default)]
    pub ignore: IgnoreConfig,
    #[serde(default)]
    pub lfs: LfsConfig,
    #[serde(default)]
    pub chunking: ChunkingConfig,
    #[serde(default)]
    pub retention: RetentionConfig,
    #[serde(default)]
    pub autosave: AutosaveConfigDto,
}

impl VcsConfig {
    /// Load configuration from project root or .apich directory, falling back to defaults
    pub fn load_from_project(project_root: impl AsRef<Path>) -> Result<Self> {
        let root = project_root.as_ref();
        let candidate_root = root.join("apich.toml");
        let candidate_apich = root.join(".apich").join("config.toml");

        let target_file = if candidate_root.exists() {
            Some(candidate_root)
        } else if candidate_apich.exists() {
            Some(candidate_apich)
        } else {
            None
        };

        if let Some(path) = target_file {
            let content = fs::read_to_string(&path).map_err(VcsError::Io)?;
            let cfg: VcsConfig = toml::from_str(&content).map_err(|e| {
                VcsError::Internal(format!("Failed to parse config {}: {}", path.display(), e))
            })?;
            Ok(cfg)
        } else {
            Ok(Self::default())
        }
    }

    /// Save configuration to project directory (.apich/config.toml or apich.toml if present)
    pub fn save_to_project(
        &self,
        project_root: impl AsRef<Path>,
    ) -> Result<PathBuf> {
        let root = project_root.as_ref();
        let target_path = if root.join("apich.toml").exists() {
            root.join("apich.toml")
        } else {
            let apich_dir = root.join(".apich");
            fs::create_dir_all(&apich_dir)?;
            apich_dir.join("config.toml")
        };

        let toml_str = toml::to_string_pretty(self)
            .map_err(|e| VcsError::Internal(format!("Failed to serialize config: {}", e)))?;
        fs::write(&target_path, toml_str)?;
        Ok(target_path)
    }

    /// Convert ignore config into a live IgnoreFilter
    pub fn to_ignore_filter(
        &self,
        project_root: impl AsRef<Path>,
    ) -> Result<IgnoreFilter> {
        let mut filter = IgnoreFilter::empty();

        if self.ignore.academic_profile {
            filter.enable_profile(IgnoreProfile::Academic)?;
        }
        if self.ignore.python_profile {
            filter.enable_profile(IgnoreProfile::Python)?;
        }
        if self.ignore.r_profile {
            filter.enable_profile(IgnoreProfile::R)?;
        }
        if self.ignore.development_profile {
            filter.enable_profile(IgnoreProfile::Development)?;
        }
        if self.ignore.rust_profile {
            filter.enable_profile(IgnoreProfile::Rust)?;
        }
        if self.ignore.javascript_profile {
            filter.enable_profile(IgnoreProfile::JavaScript)?;
        }
        if self.ignore.java_profile {
            filter.enable_profile(IgnoreProfile::Java)?;
        }
        if self.ignore.ccpp_profile {
            filter.enable_profile(IgnoreProfile::CCpp)?;
        }
        if self.ignore.editor_profile {
            filter.enable_profile(IgnoreProfile::Editor)?;
        }

        // Add custom rules from config
        for rule in &self.ignore.custom_rules {
            filter.add_rule(rule)?;
        }

        // Also load .gitignore and .apichignore if present
        let root = project_root.as_ref();
        let gitignore = root.join(".gitignore");
        if gitignore.exists() {
            let _ = filter.load_file(&gitignore);
        }
        let apichignore = root.join(".apichignore");
        if apichignore.exists() {
            let _ = filter.load_file(&apichignore);
        }

        filter.recompile()?;
        Ok(filter)
    }

    /// Convert LFS config into a live LfsPolicy
    pub fn to_lfs_policy(
        &self,
        project_root: impl AsRef<Path>,
    ) -> LfsPolicy {
        let mut policy = LfsPolicy::new(self.lfs.size_threshold_bytes, self.lfs.patterns.clone());

        if self.lfs.load_gitattributes {
            let root = project_root.as_ref();
            let gitattrs = root.join(".gitattributes");
            if gitattrs.exists() {
                let _ = policy.load_gitattributes(&gitattrs);
            }
        }

        policy
    }

    /// Convert FastCDC chunking config into runtime parameters
    pub fn to_cdc_config(&self) -> FastCdcConfig {
        match self.chunking.profile.as_str() {
            | "large_file" => FastCdcConfig::large_file(),
            | "default" => FastCdcConfig::default(),
            | _ => {
                // If custom bounds provided
                if let (Some(min), Some(avg), Some(max)) = (
                    self.chunking.min_size,
                    self.chunking.avg_size,
                    self.chunking.max_size,
                ) {
                    FastCdcConfig {
                        min_size: min,
                        avg_size: avg,
                        max_size: max,
                    }
                } else {
                    FastCdcConfig::document()
                }
            },
        }
    }

    /// Convert retention config into runtime RetentionPolicy
    pub fn to_retention_policy(&self) -> RetentionPolicy {
        RetentionPolicy {
            keep_all_duration: Duration::hours(self.retention.keep_all_hours),
            hourly_duration: Duration::days(self.retention.hourly_days),
            daily_duration: Duration::days(self.retention.daily_days),
            weekly_duration: Duration::days(self.retention.weekly_days),
            monthly_beyond: self.retention.monthly_beyond,
        }
    }

    /// Convert autosave config into runtime AutosaveConfig
    pub fn to_autosave_config(&self) -> RuntimeAutosaveConfig {
        RuntimeAutosaveConfig {
            enabled: self.autosave.enabled,
            debounce_duration: std::time::Duration::from_millis(self.autosave.debounce_ms),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IgnoreConfig {
    #[serde(default = "default_true")]
    pub academic_profile: bool,
    #[serde(default = "default_true")]
    pub python_profile: bool,
    #[serde(default = "default_true")]
    pub r_profile: bool,
    #[serde(default = "default_true")]
    pub development_profile: bool,
    #[serde(default = "default_true")]
    pub rust_profile: bool,
    #[serde(default = "default_true")]
    pub javascript_profile: bool,
    #[serde(default = "default_true")]
    pub java_profile: bool,
    #[serde(default = "default_true")]
    pub ccpp_profile: bool,
    #[serde(default = "default_true")]
    pub editor_profile: bool,
    #[serde(default)]
    pub custom_rules: Vec<String>,
}

impl IgnoreConfig {
    /// Get/set profile toggles by `IgnoreProfile` id, for generic UI wiring
    pub fn profile_enabled(
        &self,
        profile: IgnoreProfile,
    ) -> bool {
        match profile {
            | IgnoreProfile::Academic => self.academic_profile,
            | IgnoreProfile::Python => self.python_profile,
            | IgnoreProfile::R => self.r_profile,
            | IgnoreProfile::Development => self.development_profile,
            | IgnoreProfile::Rust => self.rust_profile,
            | IgnoreProfile::JavaScript => self.javascript_profile,
            | IgnoreProfile::Java => self.java_profile,
            | IgnoreProfile::CCpp => self.ccpp_profile,
            | IgnoreProfile::Editor => self.editor_profile,
        }
    }

    pub fn set_profile_enabled(
        &mut self,
        profile: IgnoreProfile,
        enabled: bool,
    ) {
        match profile {
            | IgnoreProfile::Academic => self.academic_profile = enabled,
            | IgnoreProfile::Python => self.python_profile = enabled,
            | IgnoreProfile::R => self.r_profile = enabled,
            | IgnoreProfile::Development => self.development_profile = enabled,
            | IgnoreProfile::Rust => self.rust_profile = enabled,
            | IgnoreProfile::JavaScript => self.javascript_profile = enabled,
            | IgnoreProfile::Java => self.java_profile = enabled,
            | IgnoreProfile::CCpp => self.ccpp_profile = enabled,
            | IgnoreProfile::Editor => self.editor_profile = enabled,
        }
    }
}

impl Default for IgnoreConfig {
    fn default() -> Self {
        Self {
            academic_profile: true,
            python_profile: true,
            r_profile: true,
            development_profile: true,
            rust_profile: true,
            javascript_profile: true,
            java_profile: true,
            ccpp_profile: true,
            editor_profile: true,
            custom_rules: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LfsConfig {
    #[serde(default = "default_lfs_threshold")]
    pub size_threshold_bytes: u64,
    #[serde(default = "default_lfs_patterns")]
    pub patterns: Vec<String>,
    #[serde(default = "default_true")]
    pub load_gitattributes: bool,
}

impl Default for LfsConfig {
    fn default() -> Self {
        Self {
            size_threshold_bytes: default_lfs_threshold(),
            patterns: default_lfs_patterns(),
            load_gitattributes: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkingConfig {
    #[serde(default = "default_chunk_profile")]
    pub profile: String,
    pub min_size: Option<usize>,
    pub avg_size: Option<usize>,
    pub max_size: Option<usize>,
}

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            profile: default_chunk_profile(),
            min_size: None,
            avg_size: None,
            max_size: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionConfig {
    #[serde(default = "default_keep_all_hours")]
    pub keep_all_hours: i64,
    #[serde(default = "default_hourly_days")]
    pub hourly_days: i64,
    #[serde(default = "default_daily_days")]
    pub daily_days: i64,
    #[serde(default = "default_weekly_days")]
    pub weekly_days: i64,
    #[serde(default = "default_true")]
    pub monthly_beyond: bool,
}

impl Default for RetentionConfig {
    fn default() -> Self {
        Self {
            keep_all_hours: default_keep_all_hours(),
            hourly_days: default_hourly_days(),
            daily_days: default_daily_days(),
            weekly_days: default_weekly_days(),
            monthly_beyond: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutosaveConfigDto {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_debounce_ms")]
    pub debounce_ms: u64,
}

impl Default for AutosaveConfigDto {
    fn default() -> Self {
        Self {
            enabled: true,
            debounce_ms: default_debounce_ms(),
        }
    }
}
