//! Configuration models and persistence for APICH VCS.

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

const fn default_true() -> bool {
    true
}

const fn default_lfs_threshold() -> u64 {
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

const fn default_keep_all_hours() -> i64 {
    24
}

const fn default_hourly_days() -> i64 {
    7
}

const fn default_daily_days() -> i64 {
    30
}

const fn default_weekly_days() -> i64 {
    365
}

const fn default_debounce_ms() -> u64 {
    1500
}

/// Project-level configuration for APICH Version Control System
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VcsConfig {
    /// File ignore rules and profile configurations.
    #[serde(default)]
    pub ignore: IgnoreConfig,
    /// Git LFS interoperability and pointer generation policies.
    #[serde(default)]
    pub lfs: LfsConfig,
    /// `FastCDC` content-defined chunking parameters.
    #[serde(default)]
    pub chunking: ChunkingConfig,
    /// Snapshot retention and compaction policies.
    #[serde(default)]
    pub retention: RetentionConfig,
    /// Continuous autosave settings.
    #[serde(default)]
    pub autosave: AutosaveConfigDto,
}

impl VcsConfig {
    /// Load configuration from project root or .apich directory, falling back to defaults
    ///
    /// # Errors
    /// Returns an error if reading, parsing, or writing the configuration fails.
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
            let cfg: Self = toml::from_str(&content).map_err(|e| {
                VcsError::Internal(format!("Failed to parse config {}: {}", path.display(), e))
            })?;
            Ok(cfg)
        } else {
            Ok(Self::default())
        }
    }

    /// Save configuration to project directory (.apich/config.toml or apich.toml if present)
    ///
    /// # Errors
    /// Returns an error if reading, parsing, or writing the configuration fails.
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
            .map_err(|e| VcsError::Internal(format!("Failed to serialize config: {e}")))?;
        fs::write(&target_path, toml_str)?;
        Ok(target_path)
    }

    /// Convert ignore config into a live `IgnoreFilter`
    ///
    /// # Errors
    /// Returns an error if reading, parsing, or writing the configuration fails.
    pub fn to_ignore_filter(
        &self,
        project_root: impl AsRef<Path>,
    ) -> Result<IgnoreFilter> {
        let mut filter = IgnoreFilter::empty();

        if *self.ignore.academic_profile {
            filter.enable_profile(IgnoreProfile::Academic)?;
        }
        if *self.ignore.python_profile {
            filter.enable_profile(IgnoreProfile::Python)?;
        }
        if *self.ignore.r_profile {
            filter.enable_profile(IgnoreProfile::R)?;
        }
        if *self.ignore.development_profile {
            filter.enable_profile(IgnoreProfile::Development)?;
        }
        if *self.ignore.rust_profile {
            filter.enable_profile(IgnoreProfile::Rust)?;
        }
        if *self.ignore.javascript_profile {
            filter.enable_profile(IgnoreProfile::JavaScript)?;
        }
        if *self.ignore.java_profile {
            filter.enable_profile(IgnoreProfile::Java)?;
        }
        if *self.ignore.ccpp_profile {
            filter.enable_profile(IgnoreProfile::CCpp)?;
        }
        if *self.ignore.editor_profile {
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

    /// Convert LFS config into a live `LfsPolicy`
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

    /// Convert `FastCDC` chunking config into runtime parameters
    #[must_use]
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

    /// Convert retention config into runtime `RetentionPolicy`
    #[must_use]
    pub const fn to_retention_policy(&self) -> RetentionPolicy {
        RetentionPolicy {
            keep_all_duration: Duration::hours(self.retention.keep_all_hours),
            hourly_duration: Duration::days(self.retention.hourly_days),
            daily_duration: Duration::days(self.retention.daily_days),
            weekly_duration: Duration::days(self.retention.weekly_days),
            monthly_beyond: self.retention.monthly_beyond,
        }
    }

    /// Convert autosave config into runtime `AutosaveConfig`
    #[must_use]
    pub const fn to_autosave_config(&self) -> RuntimeAutosaveConfig {
        RuntimeAutosaveConfig {
            enabled: self.autosave.enabled,
            debounce_duration: std::time::Duration::from_millis(self.autosave.debounce_ms),
        }
    }
}

/// Wrapper around a boolean flag representing whether an ignore profile is enabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct ProfileToggle(pub bool);

impl ProfileToggle {
    /// Creates a new `ProfileToggle` with the given enabled state.
    #[must_use]
    pub const fn new(enabled: bool) -> Self {
        Self(enabled)
    }

    /// Returns the boolean state of the toggle.
    #[must_use]
    pub const fn is_enabled(self) -> bool {
        self.0
    }
}

impl std::ops::Not for ProfileToggle {
    type Output = bool;

    fn not(self) -> Self::Output {
        !self.0
    }
}

impl std::ops::Deref for ProfileToggle {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for ProfileToggle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<bool> for ProfileToggle {
    fn from(value: bool) -> Self {
        Self(value)
    }
}

impl From<ProfileToggle> for bool {
    fn from(value: ProfileToggle) -> Self {
        value.0
    }
}

impl PartialEq<bool> for ProfileToggle {
    fn eq(
        &self,
        other: &bool,
    ) -> bool {
        self.0 == *other
    }
}

const fn default_profile_toggle_true() -> ProfileToggle {
    ProfileToggle(true)
}

/// Configuration for ignore profiles and custom pattern exclusions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IgnoreConfig {
    /// Enable academic output file ignoring (LaTeX, Typst, BibTeX, PDFs).
    #[serde(default = "default_profile_toggle_true")]
    pub academic_profile: ProfileToggle,
    /// Enable Python cache and virtual environment ignoring.
    #[serde(default = "default_profile_toggle_true")]
    pub python_profile: ProfileToggle,
    /// Enable R history and workspace image ignoring.
    #[serde(default = "default_profile_toggle_true")]
    pub r_profile: ProfileToggle,
    /// Enable general development artifact ignoring.
    #[serde(default = "default_profile_toggle_true")]
    pub development_profile: ProfileToggle,
    /// Enable Rust target directory ignoring.
    #[serde(default = "default_profile_toggle_true")]
    pub rust_profile: ProfileToggle,
    /// Enable Node.js / JavaScript `node_modules` ignoring.
    #[serde(default = "default_profile_toggle_true")]
    pub javascript_profile: ProfileToggle,
    /// Enable Java build artifacts ignoring.
    #[serde(default = "default_profile_toggle_true")]
    pub java_profile: ProfileToggle,
    /// Enable C/C++ compiled binaries and object file ignoring.
    #[serde(default = "default_profile_toggle_true")]
    pub ccpp_profile: ProfileToggle,
    /// Enable editor temporary files and workspace settings ignoring.
    #[serde(default = "default_profile_toggle_true")]
    pub editor_profile: ProfileToggle,
    /// Additional custom glob patterns to ignore.
    #[serde(default)]
    pub custom_rules: Vec<String>,
}

impl IgnoreConfig {
    /// Get/set profile toggles by `IgnoreProfile` id, for generic UI wiring
    #[must_use]
    pub const fn profile_enabled(
        &self,
        profile: IgnoreProfile,
    ) -> bool {
        match profile {
            | IgnoreProfile::Academic => self.academic_profile.0,
            | IgnoreProfile::Python => self.python_profile.0,
            | IgnoreProfile::R => self.r_profile.0,
            | IgnoreProfile::Development => self.development_profile.0,
            | IgnoreProfile::Rust => self.rust_profile.0,
            | IgnoreProfile::JavaScript => self.javascript_profile.0,
            | IgnoreProfile::Java => self.java_profile.0,
            | IgnoreProfile::CCpp => self.ccpp_profile.0,
            | IgnoreProfile::Editor => self.editor_profile.0,
        }
    }

    /// Enables or disables a specific ignore profile.
    pub const fn set_profile_enabled(
        &mut self,
        profile: IgnoreProfile,
        enabled: bool,
    ) {
        let toggle = ProfileToggle(enabled);
        match profile {
            | IgnoreProfile::Academic => self.academic_profile = toggle,
            | IgnoreProfile::Python => self.python_profile = toggle,
            | IgnoreProfile::R => self.r_profile = toggle,
            | IgnoreProfile::Development => self.development_profile = toggle,
            | IgnoreProfile::Rust => self.rust_profile = toggle,
            | IgnoreProfile::JavaScript => self.javascript_profile = toggle,
            | IgnoreProfile::Java => self.java_profile = toggle,
            | IgnoreProfile::CCpp => self.ccpp_profile = toggle,
            | IgnoreProfile::Editor => self.editor_profile = toggle,
        }
    }
}

impl Default for IgnoreConfig {
    fn default() -> Self {
        Self {
            academic_profile: ProfileToggle(true),
            python_profile: ProfileToggle(true),
            r_profile: ProfileToggle(true),
            development_profile: ProfileToggle(true),
            rust_profile: ProfileToggle(true),
            javascript_profile: ProfileToggle(true),
            java_profile: ProfileToggle(true),
            ccpp_profile: ProfileToggle(true),
            editor_profile: ProfileToggle(true),
            custom_rules: Vec::new(),
        }
    }
}

/// Configuration for Git Large File Storage (LFS) interoperability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LfsConfig {
    /// Minimum file size in bytes to treat as an LFS pointer.
    #[serde(default = "default_lfs_threshold")]
    pub size_threshold_bytes: u64,
    /// Path glob patterns that should always be tracked via Git LFS.
    #[serde(default = "default_lfs_patterns")]
    pub patterns: Vec<String>,
    /// Whether to load and respect patterns defined in `.gitattributes`.
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

/// Configuration for `FastCDC` chunking parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkingConfig {
    /// Target chunking profile preset (e.g., "default", "document", "`large_file`").
    #[serde(default = "default_chunk_profile")]
    pub profile: String,
    /// Explicit minimum chunk size override in bytes.
    pub min_size: Option<usize>,
    /// Explicit average chunk size override in bytes.
    pub avg_size: Option<usize>,
    /// Explicit maximum chunk size override in bytes.
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

/// Configuration for Grandfather-Father-Son snapshot retention policies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionConfig {
    /// Number of hours to keep all snapshots without compaction.
    #[serde(default = "default_keep_all_hours")]
    pub keep_all_hours: i64,
    /// Number of days to retain hourly snapshots.
    #[serde(default = "default_hourly_days")]
    pub hourly_days: i64,
    /// Number of days to retain daily snapshots.
    #[serde(default = "default_daily_days")]
    pub daily_days: i64,
    /// Number of days to retain weekly snapshots.
    #[serde(default = "default_weekly_days")]
    pub weekly_days: i64,
    /// Whether to retain one monthly snapshot indefinitely beyond the weekly window.
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

/// Configuration data transfer object for autosave preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutosaveConfigDto {
    /// Whether background autosave is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Debounce interval in milliseconds before an autosave is performed.
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
