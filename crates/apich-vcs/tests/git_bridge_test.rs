mod common;
use apich_vcs::LfsPolicy;
use apich_vcs::ProjectVcs;
use common::test_temp_dir;
use git2::Repository;
use std::fs;

#[test]
fn test_git_init_and_snapshot_export() {
    let temp = test_temp_dir();
    let vcs = ProjectVcs::open_or_init(temp.path()).unwrap();

    vcs.git_init().unwrap();

    // Create research project files
    fs::write(temp.path().join("README.md"), "# Quantum Superposition\n").unwrap();
    fs::create_dir_all(temp.path().join("docs")).unwrap();
    fs::write(
        temp.path().join("docs").join("spec.txt"),
        "Protocol specifications\n",
    )
    .unwrap();

    let commit_oid = vcs
        .git_export_commit(
            "main",
            "Initial publication commit",
            "Alice Researcher",
            "alice@institute.edu",
        )
        .unwrap();

    assert_eq!(commit_oid.len(), 40);

    // Open Git repository directly and verify its objects
    let git_repo = Repository::open(temp.path()).unwrap();
    let oid = git2::Oid::from_str(&commit_oid).unwrap();
    let commit = git_repo.find_commit(oid).unwrap();

    assert_eq!(commit.author().name().unwrap(), "Alice Researcher");
    assert_eq!(commit.author().email().unwrap(), "alice@institute.edu");

    let msg = commit.message().unwrap();
    assert!(msg.contains("Initial publication commit"));
    assert!(msg.contains("Apich-Change-Id:"));
    assert!(msg.contains("Apich-Snapshot-Id:"));

    // Verify Git Tree
    let tree = commit.tree().unwrap();
    assert!(tree.get_name("README.md").is_some());
    assert!(tree.get_name("docs").is_some());

    let docs_entry = tree.get_name("docs").unwrap();
    let docs_tree = git_repo.find_tree(docs_entry.id()).unwrap();
    assert!(docs_tree.get_name("spec.txt").is_some());
}

#[test]
fn test_git_lfs_pointer_synthesis() {
    let temp = test_temp_dir();
    let vcs = ProjectVcs::open_or_init(temp.path()).unwrap();

    vcs.git_init().unwrap();

    // Create a regular file and a binary database file (*.db / *.sqlite triggers LFS)
    fs::write(temp.path().join("notes.txt"), "Regular text file").unwrap();

    let db_content = b"SQLite format 3\0\x04\0\x01\x01\0@  \0\0\0\x01\0\0\0\0";
    fs::write(temp.path().join("experiment.db"), db_content).unwrap();

    let commit_oid = vcs
        .git_export_commit(
            "main",
            "Added experiment database",
            "Bob",
            "bob@research.org",
        )
        .unwrap();

    let git_repo = Repository::open(temp.path()).unwrap();
    let oid = git2::Oid::from_str(&commit_oid).unwrap();
    let commit = git_repo.find_commit(oid).unwrap();
    let tree = commit.tree().unwrap();

    // Verify regular file is stored as normal blob
    let notes_entry = tree.get_name("notes.txt").unwrap();
    let notes_blob = git_repo.find_blob(notes_entry.id()).unwrap();
    assert_eq!(notes_blob.content(), b"Regular text file");

    // Verify .db file is synthesized as Git LFS pointer
    let db_entry = tree.get_name("experiment.db").unwrap();
    let db_blob = git_repo.find_blob(db_entry.id()).unwrap();
    let pointer_text = std::str::from_utf8(db_blob.content()).unwrap();

    assert!(pointer_text.starts_with("version https://git-lfs.github.com/spec/v1"));
    assert!(pointer_text.contains("oid sha256:"));
    assert!(pointer_text.contains(&format!("size {}", db_content.len())));

    // Parse with LfsPolicy
    let parsed = LfsPolicy::parse_lfs_pointer(pointer_text).unwrap();
    assert_eq!(parsed.1, db_content.len() as u64);
}

#[test]
fn test_git_subfolder_material_cloning() {
    let temp = test_temp_dir();
    let origin_dir = temp.path().join("origin_upstream");
    let vcs_dir = temp.path().join("workspace_project");

    fs::create_dir_all(&origin_dir).unwrap();
    fs::create_dir_all(&vcs_dir).unwrap();

    // Create a local git repo as mock upstream
    let upstream_repo = Repository::init(&origin_dir).unwrap();
    fs::write(origin_dir.join("library.py"), "def helper(): return 42\n").unwrap();
    let mut index = upstream_repo.index().unwrap();
    index.add_path(std::path::Path::new("library.py")).unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = upstream_repo.find_tree(tree_id).unwrap();
    let sig = git2::Signature::now("Upstream Author", "upstream@lib.org").unwrap();
    upstream_repo
        .commit(
            Some("HEAD"),
            &sig,
            &sig,
            "Initial upstream commit",
            &tree,
            &[],
        )
        .unwrap();

    // Now in VCS project, clone upstream repo as material
    let vcs = ProjectVcs::open_or_init(&vcs_dir).unwrap();
    let origin_path_str = origin_dir.to_str().unwrap();

    let record = vcs
        .clone_material(origin_path_str, "materials/upstream_lib")
        .unwrap();

    assert_eq!(record.name, "upstream_lib");
    assert_eq!(record.rel_path, "materials/upstream_lib");
    assert!(vcs_dir.join("materials/upstream_lib/library.py").exists());
    assert_eq!(
        fs::read_to_string(vcs_dir.join("materials/upstream_lib/library.py")).unwrap(),
        "def helper(): return 42\n"
    );
}
