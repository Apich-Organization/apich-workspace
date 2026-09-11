mod common;
use apich_vcs::ProjectVcs;
use common::test_temp_dir;
use std::fs;

#[test]
fn test_branch_creation_and_switch() {
    let temp = test_temp_dir();
    let vcs = ProjectVcs::open_or_init(temp.path()).unwrap();

    fs::write(temp.path().join("index.md"), "# Title\n").unwrap();
    let s1 = vcs.snapshot("Initial commit").unwrap();

    // Create experiment branch
    vcs.branch_create("experiment").unwrap();

    let main_b = vcs.get_branch("main").unwrap().unwrap();
    let exp_b = vcs.get_branch("experiment").unwrap().unwrap();
    assert_eq!(main_b.head_snapshot_id, s1.id);
    assert_eq!(exp_b.head_snapshot_id, s1.id);

    // Switch branch
    vcs.branch_switch("experiment").unwrap();
    assert_eq!(
        vcs.current_branch().unwrap(),
        Some("experiment".to_string())
    );
}

#[test]
fn test_clean_3way_reconcile_without_conflicts() {
    let temp = test_temp_dir();
    let vcs = ProjectVcs::open_or_init(temp.path()).unwrap();

    // Ancestor state
    fs::write(temp.path().join("intro.txt"), "Introduction\n").unwrap();
    fs::write(temp.path().join("methods.txt"), "Methods\n").unwrap();
    let ancestor = vcs.snapshot("Ancestor commit").unwrap();

    // Create feature branch
    vcs.branch_create("feature").unwrap();
    vcs.branch_switch("feature").unwrap();

    // On feature: add results.txt and edit methods.txt
    fs::write(temp.path().join("methods.txt"), "Methods\nStep 1\n").unwrap();
    fs::write(temp.path().join("results.txt"), "Results 42\n").unwrap();
    let s_feature = vcs.snapshot("Feature work").unwrap();
    assert_ne!(ancestor.id, s_feature.id);

    // Switch back to main
    vcs.branch_switch("main").unwrap();
    assert_eq!(
        fs::read_to_string(temp.path().join("methods.txt")).unwrap(),
        "Methods\n"
    );
    assert!(!temp.path().join("results.txt").exists());

    // On main: edit intro.txt
    fs::write(
        temp.path().join("intro.txt"),
        "Introduction\nBackground info\n",
    )
    .unwrap();
    let s_main = vcs.snapshot("Main work").unwrap();

    // Merge feature into main (3-way weave-free reconcile)
    let reconcile = vcs.merge("feature").unwrap();
    assert!(
        reconcile.conflicts.is_empty(),
        "Merge should have 0 conflicts"
    );

    // Verify all merged files on disk
    assert_eq!(
        fs::read_to_string(temp.path().join("intro.txt")).unwrap(),
        "Introduction\nBackground info\n"
    );
    assert_eq!(
        fs::read_to_string(temp.path().join("methods.txt")).unwrap(),
        "Methods\nStep 1\n"
    );
    assert_eq!(
        fs::read_to_string(temp.path().join("results.txt")).unwrap(),
        "Results 42\n"
    );

    // Check resulting merge snapshot
    let head = vcs.head_snapshot().unwrap().unwrap();
    assert_eq!(head.parent_snapshot_id, Some(s_main.id));
    assert!(head.message.contains("Weave-free"));
}

#[test]
fn test_conflicting_merge_with_inline_markers() {
    let temp = test_temp_dir();
    let vcs = ProjectVcs::open_or_init(temp.path()).unwrap();

    // Ancestor state
    fs::write(temp.path().join("abstract.txt"), "Draft abstract\n").unwrap();
    vcs.snapshot("Initial abstract").unwrap();

    // Create branch reviewer-a
    vcs.branch_create("reviewer-a").unwrap();
    vcs.branch_switch("reviewer-a").unwrap();
    fs::write(temp.path().join("abstract.txt"), "Reviewer A abstract\n").unwrap();
    vcs.snapshot("Reviewer A update").unwrap();

    // Switch to main and do conflicting edit
    vcs.branch_switch("main").unwrap();
    fs::write(temp.path().join("abstract.txt"), "Reviewer B abstract\n").unwrap();
    vcs.snapshot("Reviewer B update").unwrap();

    // Merge reviewer-a into main
    let result = vcs.merge("reviewer-a").unwrap();
    assert_eq!(result.conflicts.len(), 1, "Must produce 1 conflict");
    assert_eq!(result.conflicts[0].path, "abstract.txt");

    // File on disk should contain standard inline conflict markers
    let merged_content = fs::read_to_string(temp.path().join("abstract.txt")).unwrap();
    assert!(merged_content.contains("<<<<<<< ours"));
    assert!(merged_content.contains("Reviewer B abstract"));
    assert!(merged_content.contains("======="));
    assert!(merged_content.contains("Reviewer A abstract"));
    assert!(merged_content.contains(">>>>>>> theirs"));

    // User can resolve conflict seamlessly without repository lockout
    fs::write(
        temp.path().join("abstract.txt"),
        "Joint Consensus Abstract by Reviewers A & B\n",
    )
    .unwrap();
    let resolved_snapshot = vcs.snapshot("Resolved merge conflict").unwrap();

    assert_eq!(
        fs::read_to_string(temp.path().join("abstract.txt")).unwrap(),
        "Joint Consensus Abstract by Reviewers A & B\n"
    );
    assert_eq!(
        vcs.head_snapshot().unwrap().unwrap().id,
        resolved_snapshot.id
    );
}
