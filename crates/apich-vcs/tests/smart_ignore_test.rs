mod common;
use apich_vcs::IgnoreFilter;
use apich_vcs::IgnoreProfile;
use apich_vcs::ProjectVcs;
use common::test_temp_dir;
use std::fs;

#[test]
fn test_academic_profile_filtering() {
    let filter = IgnoreFilter::builder()
        .add_profile(IgnoreProfile::Academic)
        .build()
        .unwrap();

    // LaTeX build artifacts
    assert!(filter.is_ignored("paper.aux"));
    assert!(filter.is_ignored("sections/chapter1.aux"));
    assert!(filter.is_ignored("thesis.synctex.gz"));
    assert!(filter.is_ignored("slides.nav"));
    assert!(filter.is_ignored("slides.snm"));
    assert!(filter.is_ignored("paper.fls"));
    assert!(filter.is_ignored("paper.fdb_latexmk"));
    assert!(filter.is_ignored(".typst-cache/obj_123"));

    // Valid academic documents
    assert!(!filter.is_ignored("paper.tex"));
    assert!(!filter.is_ignored("thesis.typ"));
    assert!(!filter.is_ignored("figures/diagram.svg"));
    assert!(!filter.is_ignored("references.bib"));
}

#[test]
fn test_python_and_r_filtering() {
    let filter = IgnoreFilter::builder()
        .add_profile(IgnoreProfile::Python)
        .add_profile(IgnoreProfile::R)
        .build()
        .unwrap();

    // Python bytecode and environments
    assert!(filter.is_ignored("__pycache__/model.cpython-312.pyc"));
    assert!(filter.is_ignored("module/__pycache__/utils.cpython-312.pyc"));
    assert!(filter.is_ignored(".venv/bin/activate"));
    assert!(filter.is_ignored("venv/lib/python3.12/site-packages/pkg"));
    assert!(filter.is_ignored(".pytest_cache/README.md"));

    // R session dumps
    assert!(filter.is_ignored(".Rhistory"));
    assert!(filter.is_ignored(".RData"));
    assert!(filter.is_ignored("subfolder/.Rhistory"));

    // Valid source files
    assert!(!filter.is_ignored("pipeline.py"));
    assert!(!filter.is_ignored("analysis.R"));
    assert!(!filter.is_ignored("pyproject.toml"));
}

#[test]
fn test_custom_apichignore_and_gitignore() {
    let temp = test_temp_dir();
    let root = temp.path();

    fs::write(root.join(".gitignore"), "*.scratch\nbuild/\n").unwrap();
    fs::write(root.join(".apichignore"), "*.secret\nprivate_data/\n").unwrap();

    let filter = IgnoreFilter::new_with_defaults(root).unwrap();

    // Gitignore rules
    assert!(filter.is_ignored("notes.scratch"));
    assert!(filter.is_ignored("build/out.bin"));

    // Apichignore rules
    assert!(filter.is_ignored("keys.secret"));
    assert!(filter.is_ignored("private_data/users.csv"));

    // Default academic rules still active
    assert!(filter.is_ignored("doc.aux"));

    // Valid file
    assert!(!filter.is_ignored("src/main.rs"));
}

#[test]
fn test_scan_working_tree_filters_ignored_files() {
    let temp = test_temp_dir();
    let vcs = ProjectVcs::open_or_init(temp.path()).unwrap();

    // Write source files
    fs::write(temp.path().join("main.typ"), "= Title\nContent\n").unwrap();
    fs::create_dir_all(temp.path().join("chapters")).unwrap();
    fs::write(
        temp.path().join("chapters").join("intro.typ"),
        "= Chapter 1\n",
    )
    .unwrap();

    // Write junk that should be ignored by default academic/python profiles
    fs::write(temp.path().join("main.aux"), "LaTeX junk").unwrap();
    fs::write(temp.path().join("main.synctex.gz"), "SyncTeX junk").unwrap();
    fs::create_dir_all(temp.path().join("__pycache__")).unwrap();
    fs::write(
        temp.path().join("__pycache__").join("cache.pyc"),
        "Compiled Python",
    )
    .unwrap();
    fs::write(temp.path().join(".DS_Store"), "macOS metadata").unwrap();

    let tree = vcs.scan_working_tree().unwrap();

    // Only valid files must be in the tree
    assert_eq!(tree.entries.len(), 2);
    assert!(tree.entries.contains_key("main.typ"));
    assert!(tree.entries.contains_key("chapters/intro.typ"));

    // Ignored files must not appear
    assert!(!tree.entries.contains_key("main.aux"));
    assert!(!tree.entries.contains_key("main.synctex.gz"));
    assert!(!tree.entries.contains_key("__pycache__/cache.pyc"));
    assert!(!tree.entries.contains_key(".DS_Store"));
}
