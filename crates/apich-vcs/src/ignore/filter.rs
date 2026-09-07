use super::profile::IgnoreProfile;
use crate::error::Result;
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Smart synthetic ignore filter combining profiles, .gitignore, and .apichignore
#[derive(Clone)]
pub struct IgnoreFilter {
    glob_set: GlobSet,
    whitelist_set: GlobSet,
    enabled_profiles: HashSet<IgnoreProfile>,
    custom_rules: Vec<String>,
}

impl IgnoreFilter {
    /// Create a new filter with academic, data science, and development defaults
    pub fn new_with_defaults(project_root: impl AsRef<Path>) -> Result<Self> {
        let mut filter = Self::empty();
        filter.enabled_profiles.insert(IgnoreProfile::Academic);
        filter.enabled_profiles.insert(IgnoreProfile::Python);
        filter.enabled_profiles.insert(IgnoreProfile::R);
        filter.enabled_profiles.insert(IgnoreProfile::Development);

        // Load project-specific ignore files if present
        let gitignore = project_root.as_ref().join(".gitignore");
        if gitignore.exists() {
            filter.load_file(&gitignore)?;
        }
        let apichignore = project_root.as_ref().join(".apichignore");
        if apichignore.exists() {
            filter.load_file(&apichignore)?;
        }

        filter.recompile()?;
        Ok(filter)
    }

    /// Create an empty ignore filter
    pub fn empty() -> Self {
        Self {
            glob_set: GlobSet::empty(),
            whitelist_set: GlobSet::empty(),
            enabled_profiles: HashSet::new(),
            custom_rules: Vec::new(),
        }
    }

    pub fn builder() -> IgnoreFilterBuilder {
        IgnoreFilterBuilder::new()
    }

    /// Test if a relative path matches any ignore rule (taking into account whitelist exceptions)
    pub fn is_ignored(&self, rel_path: &str) -> bool {
        // Always ignore internal directories
        if rel_path.starts_with(".apich") || rel_path.starts_with(".git") {
            return true;
        }

        let clean_path = rel_path.trim_start_matches("./");

        // Whitelist / negative glob check (e.g. "!important.aux")
        if self.whitelist_set.is_match(clean_path) {
            return false;
        }

        self.glob_set.is_match(clean_path)
    }

    pub fn is_profile_enabled(&self, profile: IgnoreProfile) -> bool {
        self.enabled_profiles.contains(&profile)
    }

    pub fn enabled_profiles(&self) -> &HashSet<IgnoreProfile> {
        &self.enabled_profiles
    }

    pub fn custom_rules(&self) -> &[String] {
        &self.custom_rules
    }

    /// Dynamically enable a profile and recompile
    pub fn enable_profile(&mut self, profile: IgnoreProfile) -> Result<()> {
        self.enabled_profiles.insert(profile);
        self.recompile()
    }

    /// Dynamically disable a profile and recompile
    pub fn disable_profile(&mut self, profile: IgnoreProfile) -> Result<()> {
        self.enabled_profiles.remove(&profile);
        self.recompile()
    }

    /// Add a custom ignore or whitelist rule (e.g. "*.tmp" or "!data/sample.csv")
    pub fn add_rule(&mut self, rule: impl Into<String>) -> Result<()> {
        let r = rule.into();
        let trimmed = r.trim();
        if !trimmed.is_empty() && !trimmed.starts_with('#') {
            self.custom_rules.push(trimmed.to_string());
            self.recompile()?;
        }
        Ok(())
    }

    /// Remove a custom rule
    pub fn remove_rule(&mut self, rule: &str) -> Result<()> {
        self.custom_rules.retain(|r| r != rule);
        self.recompile()
    }

    /// Load lines from an ignore file into custom rules
    pub fn load_file(&mut self, path: impl AsRef<Path>) -> Result<()> {
        if let Ok(content) = fs::read_to_string(path.as_ref()) {
            for line in content.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    self.custom_rules.push(trimmed.to_string());
                }
            }
            self.recompile()?;
        }
        Ok(())
    }

    /// Recompile all profile patterns and custom rules into GlobSets
    pub fn recompile(&mut self) -> Result<()> {
        let mut ignore_builder = GlobSetBuilder::new();
        let mut whitelist_builder = GlobSetBuilder::new();

        // 1. Add active profile patterns
        for profile in &self.enabled_profiles {
            for pat in profile.patterns() {
                Self::add_pattern_to_builder(&mut ignore_builder, pat);
            }
        }

        // 2. Add custom rules
        for rule in &self.custom_rules {
            if let Some(positive) = rule.strip_prefix('!') {
                Self::add_pattern_to_builder(&mut whitelist_builder, positive);
            } else {
                Self::add_pattern_to_builder(&mut ignore_builder, rule);
            }
        }

        self.glob_set = ignore_builder.build().map_err(|e| {
            crate::error::VcsError::Internal(format!("Failed to compile ignore globs: {}", e))
        })?;

        self.whitelist_set = whitelist_builder.build().map_err(|e| {
            crate::error::VcsError::Internal(format!("Failed to compile whitelist globs: {}", e))
        })?;

        Ok(())
    }

    fn add_pattern_to_builder(builder: &mut GlobSetBuilder, pat: &str) {
        let trimmed = pat.trim();
        if let Ok(glob) = Glob::new(trimmed) {
            builder.add(glob);
        }
        if trimmed.ends_with('/') {
            if let Ok(glob) = Glob::new(&format!("{}**", trimmed)) {
                builder.add(glob);
            }
            if let Ok(glob) = Glob::new(&format!("**/{}**", trimmed)) {
                builder.add(glob);
            }
        } else if !trimmed.contains('/') && !trimmed.contains('*') {
            if let Ok(glob) = Glob::new(&format!("{}/**", trimmed)) {
                builder.add(glob);
            }
            if let Ok(glob) = Glob::new(&format!("**/{}/**", trimmed)) {
                builder.add(glob);
            }
            if let Ok(glob) = Glob::new(&format!("**/{}", trimmed)) {
                builder.add(glob);
            }
        } else if !trimmed.starts_with("**/") && !trimmed.starts_with('*') {
            if let Ok(glob) = Glob::new(&format!("**/{}", trimmed)) {
                builder.add(glob);
            }
        }
    }
}

pub struct IgnoreFilterBuilder {
    filter: IgnoreFilter,
}

impl Default for IgnoreFilterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl IgnoreFilterBuilder {
    pub fn new() -> Self {
        Self {
            filter: IgnoreFilter::empty(),
        }
    }

    pub fn add_profile(mut self, profile: IgnoreProfile) -> Self {
        self.filter.enabled_profiles.insert(profile);
        self
    }

    pub fn add_rule(mut self, rule: impl Into<String>) -> Self {
        let r = rule.into();
        let trimmed = r.trim();
        if !trimmed.is_empty() && !trimmed.starts_with('#') {
            self.filter.custom_rules.push(trimmed.to_string());
        }
        self
    }

    pub fn load_file(mut self, path: impl AsRef<Path>) -> Result<Self> {
        let _ = self.filter.load_file(path);
        Ok(self)
    }

    pub fn build(mut self) -> Result<IgnoreFilter> {
        self.filter.recompile()?;
        Ok(self.filter)
    }
}
