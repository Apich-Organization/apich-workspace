mod common;
use apich_vcs::{FastCdcConfig, IgnoreProfile, ProjectVcs};
use chrono::Duration;
use common::test_temp_dir;
use std::fs;

#[test]
fn test_project_config_file_loading_apich_toml() {
    let temp = test_temp_dir();
    let root = temp.path();

    // 1. User/Team creates apich.toml in project root
    let toml_content = r#"
[ignore]
academic_profile = true
python_profile = false
r_profile = true
development_profile = true
custom_rules = [
    "*.custom_cache",
    "secret_models/**",
    "!secret_models/allowed_demo.bin"
]

[lfs]
size_threshold_bytes = 10485760
patterns = [
    "models/**/*.onnx",
    "data/raw_*.bin",
    "*.sqlite"
]
load_gitattributes = true

[chunking]
profile = "large_file"

[retention]
keep_all_hours = 48
hourly_days = 14
daily_days = 60
weekly_days = 730
monthly_beyond = true

[autosave]
enabled = false
debounce_ms = 3000
"#;
    fs::write(root.join("apich.toml"), toml_content).unwrap();

    // 2. Open project: VCS automatically discovers and applies apich.toml
    let vcs = ProjectVcs::open_or_init(root).unwrap();

    // Verify config values
    assert!(!vcs.config().ignore.python_profile);
    assert_eq!(vcs.config().lfs.size_threshold_bytes, 10 * 1024 * 1024);
    assert!(!vcs.config().autosave.enabled);

    // Verify FastCDC chunking adopted the "large_file" profile
    assert_eq!(vcs.cdc_config().min_size, 64 * 1024);
    assert_eq!(vcs.cdc_config().max_size, 1024 * 1024);

    // Verify Retention policy adopted the custom durations
    assert_eq!(vcs.retention_policy().keep_all_duration, Duration::hours(48));
    assert_eq!(vcs.retention_policy().hourly_duration, Duration::days(14));

    // Verify Smart Ignore:
    // Python profile is disabled -> python files are NOT ignored
    assert!(!vcs.ignore_filter().is_ignored("__pycache__/test.pyc"));
    assert!(!vcs.ignore_filter().is_ignored(".venv/bin/python"));

    // Academic profile is enabled -> LaTeX build files are ignored
    assert!(vcs.ignore_filter().is_ignored("paper.aux"));
    assert!(vcs.ignore_filter().is_ignored("thesis.synctex.gz"));

    // Custom rules and wildcard support
    assert!(vcs.ignore_filter().is_ignored("temp.custom_cache"));
    assert!(vcs.ignore_filter().is_ignored("secret_models/weights.bin"));
    assert!(vcs.ignore_filter().is_ignored("secret_models/subfolder/weights.bin"));

    // Negative exception rule (!secret_models/allowed_demo.bin)
    assert!(!vcs.ignore_filter().is_ignored("secret_models/allowed_demo.bin"));
}

#[test]
fn test_wildcard_lfs_and_gitattributes_loading() {
    let temp = test_temp_dir();
    let root = temp.path();

    // Write a .gitattributes file in the project
    fs::write(
        root.join(".gitattributes"),
        "# Git LFS attributes\n\
        weights/**/*.pt filter=lfs diff=lfs merge=lfs -text\n\
        checkpoints/*.ckpt filter=lfs\n\
        data/embeddings/*.parquet filter=lfs\n",
    )
    .unwrap();

    let mut vcs = ProjectVcs::open_or_init(root).unwrap();

    // Programmatically set size threshold to 20 MB and add a wildcard
    vcs.set_lfs_size_threshold(20 * 1024 * 1024);
    vcs.add_lfs_pattern("deep_models/**/*.onnx");

    // Test wildcard matches:
    // 1. From .gitattributes
    assert!(vcs.lfs_policy().is_lfs_file("weights/transformer/base.pt", 500));
    assert!(vcs.lfs_policy().is_lfs_file("checkpoints/epoch_10.ckpt", 500));
    assert!(vcs.lfs_policy().is_lfs_file("data/embeddings/train.parquet", 500));

    // 2. From programmatic pattern
    assert!(vcs.lfs_policy().is_lfs_file("deep_models/vision/resnet.onnx", 100));

    // 3. Default patterns
    assert!(vcs.lfs_policy().is_lfs_file("measurements.sqlite", 100));
    assert!(vcs.lfs_policy().is_lfs_file("results.db", 100));

    // 4. Regular non-LFS file
    assert!(!vcs.lfs_policy().is_lfs_file("src/main.rs", 1000));
    assert!(!vcs.lfs_policy().is_lfs_file("paper.typ", 5000));

    // 5. File exceeding size threshold (25 MB > 20 MB)
    assert!(vcs.lfs_policy().is_lfs_file("huge_dump.txt", 25 * 1024 * 1024));
}

#[test]
fn test_programmatic_configuration_and_dynamic_customization() {
    let temp = test_temp_dir();
    let root = temp.path();

    let mut vcs = ProjectVcs::open_or_init(root).unwrap();

    // Initially academic files are ignored
    assert!(vcs.ignore_filter().is_ignored("paper.aux"));

    // 1. Dynamically disable Academic profile
    vcs.disable_ignore_profile(IgnoreProfile::Academic).unwrap();
    assert!(!vcs.ignore_filter().is_ignored("paper.aux"));

    // 2. Dynamically add custom ignore rule with wildcards
    vcs.add_ignore_rule("generated_assets/**/*.tmp").unwrap();
    assert!(vcs.ignore_filter().is_ignored("generated_assets/svg/chart.tmp"));

    // 3. Dynamically set custom FastCDC chunking bounds
    vcs.set_cdc_config(FastCdcConfig {
        min_size: 2048,
        avg_size: 8192,
        max_size: 32768,
    });
    assert_eq!(vcs.cdc_config().min_size, 2048);

    // 4. Save configuration to .apich/config.toml
    let saved_path = vcs.save_config().unwrap();
    assert!(saved_path.exists());

    // 5. Reload from disk and verify persistence
    vcs.reload_config().unwrap();
    assert!(!vcs.ignore_filter().is_ignored("paper.aux"));
    assert!(vcs.ignore_filter().is_ignored("generated_assets/svg/chart.tmp"));
}
