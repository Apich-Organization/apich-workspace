use globset::Glob;
use globset::GlobSet;
use globset::GlobSetBuilder;
use sha2::Digest;
use std::fs;
use std::path::Path;

/// Git Large File Storage (LFS) policy and automatic pointer generator
#[derive(Debug, Clone)]
pub struct LfsPolicy {
    /// File size threshold in bytes to trigger automatic Git LFS pointer generation (default: 50MB)
    pub size_threshold: u64,
    /// Glob patterns candidate for LFS (e.g. SQLite databases, models)
    pub patterns: Vec<String>,
    glob_set: GlobSet,
}

impl Default for LfsPolicy {
    fn default() -> Self {
        let default_patterns = vec![
            "*.sqlite".to_string(),
            "*.db".to_string(),
            "*.parquet".to_string(),
            "*.bin".to_string(),
            "*.onnx".to_string(),
            "*.pt".to_string(),
            "*.tar.gz".to_string(),
            "*.zip".to_string(),
        ];
        Self::new(50 * 1024 * 1024, default_patterns)
    }
}

impl LfsPolicy {
    /// Create a new LfsPolicy with a size threshold and glob patterns
    pub fn new(
        size_threshold: u64,
        patterns: Vec<String>,
    ) -> Self {
        let glob_set = Self::compile_globs(&patterns);
        Self {
            size_threshold,
            patterns,
            glob_set,
        }
    }

    fn compile_globs(patterns: &[String]) -> GlobSet {
        let mut builder = GlobSetBuilder::new();
        for pat in patterns {
            if let Ok(glob) = Glob::new(pat) {
                builder.add(glob);
            }
            if !pat.starts_with("**/") && !pat.starts_with('*') {
                if let Ok(glob) = Glob::new(&format!("**/{}", pat)) {
                    builder.add(glob);
                }
            }
        }
        builder.build().unwrap_or_else(|_| GlobSet::empty())
    }

    /// Add a wildcard or glob pattern candidate for LFS (e.g. "models/**/*.onnx" or "data/*.db")
    pub fn add_pattern(
        &mut self,
        pattern: impl Into<String>,
    ) {
        let p = pattern.into();
        let trimmed = p.trim();
        if !trimmed.is_empty() && !trimmed.starts_with('#') {
            self.patterns.push(trimmed.to_string());
            self.glob_set = Self::compile_globs(&self.patterns);
        }
    }

    /// Configure the size threshold in bytes for automatic LFS pointer generation
    pub fn set_size_threshold(
        &mut self,
        size_threshold: u64,
    ) {
        self.size_threshold = size_threshold;
    }

    /// Load LFS rules from a `.gitattributes` file
    /// Parses lines containing `filter=lfs`
    pub fn load_gitattributes(
        &mut self,
        path: impl AsRef<Path>,
    ) -> std::io::Result<()> {
        if let Ok(content) = fs::read_to_string(path.as_ref()) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                if trimmed.contains("filter=lfs") {
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if let Some(pattern) = parts.first() {
                        self.add_pattern(*pattern);
                    }
                }
            }
        }
        Ok(())
    }

    /// Check if a file should be stored via Git LFS pointers when exporting to Git
    pub fn is_lfs_file(
        &self,
        path: &str,
        size: u64,
    ) -> bool {
        if size >= self.size_threshold {
            return true;
        }

        let clean_path = path.trim_start_matches("./");
        self.glob_set.is_match(clean_path)
    }

    /// Compute SHA-256 and generate the standard Git LFS pointer text
    pub fn create_lfs_pointer(data: &[u8]) -> (String, String) {
        let mut hasher = sha2::Sha256::new();
        hasher.update(data);
        let sha256_hex = hex::encode(hasher.finalize());

        let pointer = format!(
            "version https://git-lfs.github.com/spec/v1\noid sha256:{}\nsize {}\n",
            sha256_hex,
            data.len()
        );

        (pointer, sha256_hex)
    }

    /// Parse an LFS pointer if file content matches spec
    pub fn parse_lfs_pointer(content: &str) -> Option<(String, u64)> {
        let lines: Vec<&str> = content.lines().collect();
        if lines.len() < 3 || lines[0] != "version https://git-lfs.github.com/spec/v1" {
            return None;
        }

        let mut oid = None;
        let mut size = None;

        for line in &lines[1..] {
            if let Some(rest) = line.strip_prefix("oid sha256:") {
                oid = Some(rest.trim().to_string());
            } else if let Some(rest) = line.strip_prefix("size ") {
                size = rest.trim().parse::<u64>().ok();
            }
        }

        match (oid, size) {
            | (Some(o), Some(s)) => Some((o, s)),
            | _ => None,
        }
    }
}

mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes
            .as_ref()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }
}
