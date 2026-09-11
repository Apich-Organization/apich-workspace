mod common;
use apich_vcs::ProjectVcs;
use common::test_temp_dir;
use std::fs;

#[test]
fn test_vcs_init_and_empty_state() {
    let temp = test_temp_dir();
    let vcs = ProjectVcs::open_or_init(temp.path()).unwrap();

    assert_eq!(vcs.current_branch().unwrap(), Some("main".to_string()));
    assert!(vcs.head_snapshot().unwrap().is_none());
    assert!(vcs.list_snapshots().unwrap().is_empty());
    assert!(vcs.list_milestones().unwrap().is_empty());
}

#[test]
fn test_vcs_snapshot_and_scan() {
    let temp = test_temp_dir();
    let vcs = ProjectVcs::open_or_init(temp.path()).unwrap();

    // Create research files
    fs::write(
        temp.path().join("paper.typ"),
        "= Quantum Decoherence in Superconductors\nBy Alice Researcher\n",
    )
    .unwrap();

    fs::create_dir_all(temp.path().join("data")).unwrap();
    fs::write(
        temp.path().join("data").join("measurements.csv"),
        "timestamp,temp_kelvin,voltage_mv\n1700000000,0.015,1.24\n1700000001,0.015,1.25\n",
    )
    .unwrap();

    let s1 = vcs.snapshot("Initial draft and raw data").unwrap();
    assert_eq!(s1.message, "Initial draft and raw data");
    assert!(s1.parent_snapshot_id.is_none());
    assert!(!s1.is_milestone);

    // Verify HEAD
    let head = vcs.head_snapshot().unwrap().unwrap();
    assert_eq!(head.id, s1.id);
    assert_eq!(head.tree_hash, s1.tree_hash);

    // Verify snapshots list
    let snapshots = vcs.list_snapshots().unwrap();
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].id, s1.id);

    // Verify tree inspection
    let tree = vcs.cas().get_tree(&s1.tree_hash).unwrap();
    assert_eq!(tree.entries.len(), 2);
    assert!(tree.entries.contains_key("paper.typ"));
    assert!(tree.entries.contains_key("data/measurements.csv"));
}

#[test]
fn test_vcs_milestones() {
    let temp = test_temp_dir();
    let vcs = ProjectVcs::open_or_init(temp.path()).unwrap();

    fs::write(
        temp.path().join("thesis.typ"),
        "= Master Thesis\nChapter 1\n",
    )
    .unwrap();
    vcs.snapshot("Draft chapter 1").unwrap();

    fs::write(
        temp.path().join("thesis.typ"),
        "= Master Thesis\nChapter 1\nChapter 2\n",
    )
    .unwrap();

    let milestone = vcs
        .create_milestone("v1.0-submission", "Submitted to Committee")
        .unwrap();

    assert!(milestone.is_milestone);
    assert_eq!(milestone.milestone_name.as_deref(), Some("v1.0-submission"));
    assert_eq!(milestone.message, "Submitted to Committee");

    let milestones = vcs.list_milestones().unwrap();
    assert_eq!(milestones.len(), 1);
    assert_eq!(milestones[0].id, milestone.id);
    assert_eq!(
        milestones[0].milestone_name.as_deref(),
        Some("v1.0-submission")
    );
}

#[test]
fn test_vcs_revert_to() {
    let temp = test_temp_dir();
    let vcs = ProjectVcs::open_or_init(temp.path()).unwrap();

    // State 1: two files
    fs::write(temp.path().join("alpha.txt"), "Version 1 of Alpha").unwrap();
    fs::write(temp.path().join("beta.txt"), "Version 1 of Beta").unwrap();
    let s1 = vcs.snapshot("State 1").unwrap();

    // State 2: modify alpha, remove beta, add gamma
    fs::write(temp.path().join("alpha.txt"), "Version 2 of Alpha (edited)").unwrap();
    fs::remove_file(temp.path().join("beta.txt")).unwrap();
    fs::write(temp.path().join("gamma.txt"), "Version 1 of Gamma").unwrap();
    let s2 = vcs.snapshot("State 2").unwrap();

    assert_eq!(
        fs::read_to_string(temp.path().join("alpha.txt")).unwrap(),
        "Version 2 of Alpha (edited)"
    );
    assert!(!temp.path().join("beta.txt").exists());
    assert!(temp.path().join("gamma.txt").exists());

    // Revert to State 1
    vcs.revert_to(s1.id).unwrap();

    // Verify working copy matches State 1
    assert_eq!(
        fs::read_to_string(temp.path().join("alpha.txt")).unwrap(),
        "Version 1 of Alpha"
    );
    assert_eq!(
        fs::read_to_string(temp.path().join("beta.txt")).unwrap(),
        "Version 1 of Beta"
    );
    assert!(!temp.path().join("gamma.txt").exists());

    // Head is now at s1
    assert_eq!(vcs.head_snapshot().unwrap().unwrap().id, s1.id);

    // Revert forward to State 2
    vcs.revert_to(s2.id).unwrap();
    assert_eq!(
        fs::read_to_string(temp.path().join("alpha.txt")).unwrap(),
        "Version 2 of Alpha (edited)"
    );
    assert!(!temp.path().join("beta.txt").exists());
    assert!(temp.path().join("gamma.txt").exists());
}

#[test]
fn test_vcs_oplog_undo() {
    let temp = test_temp_dir();
    let vcs = ProjectVcs::open_or_init(temp.path()).unwrap();

    fs::write(temp.path().join("notes.md"), "# Initial Note\n").unwrap();
    let s1 = vcs.snapshot("First note").unwrap();

    fs::write(
        temp.path().join("notes.md"),
        "# Initial Note\n## Added thoughts\n",
    )
    .unwrap();
    let s2 = vcs.snapshot("Second note").unwrap();

    assert_eq!(vcs.head_snapshot().unwrap().unwrap().id, s2.id);

    // Undo should bring us back to s1
    let undone_to = vcs.undo().unwrap();
    assert_eq!(undone_to, Some(s1.id));
    assert_eq!(vcs.head_snapshot().unwrap().unwrap().id, s1.id);
    assert_eq!(
        fs::read_to_string(temp.path().join("notes.md")).unwrap(),
        "# Initial Note\n"
    );
}

#[test]
fn test_vcs_deduplication_on_identical_content() {
    let temp = test_temp_dir();
    let vcs = ProjectVcs::open_or_init(temp.path()).unwrap();

    fs::write(temp.path().join("static.txt"), "Constant content").unwrap();
    let s1 = vcs.snapshot("First snapshot").unwrap();

    // Calling snapshot again without changing files should return identical snapshot
    let s2 = vcs.snapshot("No change snapshot").unwrap();
    assert_eq!(s1.id, s2.id);
    assert_eq!(vcs.list_snapshots().unwrap().len(), 1);

    // snapshot_if_changed should return None
    assert_eq!(vcs.snapshot_if_changed("Another no change").unwrap(), None);
    assert!(!vcs.has_changes().unwrap());

    // Modify file -> has_changes is true -> snapshot_if_changed succeeds
    fs::write(temp.path().join("static.txt"), "Changed content").unwrap();
    assert!(vcs.has_changes().unwrap());
    let s3 = vcs.snapshot_if_changed("Legit change").unwrap();
    assert!(s3.is_some());
    assert_eq!(vcs.list_snapshots().unwrap().len(), 2);
}

#[test]
fn test_vcs_redo_cycle() {
    let temp = test_temp_dir();
    let vcs = ProjectVcs::open_or_init(temp.path()).unwrap();

    fs::write(temp.path().join("code.py"), "x = 10\n").unwrap();
    let s1 = vcs.snapshot("Version 1").unwrap();

    fs::write(temp.path().join("code.py"), "x = 20\n").unwrap();
    let s2 = vcs.snapshot("Version 2").unwrap();

    fs::write(temp.path().join("code.py"), "x = 30\n").unwrap();
    let s3 = vcs.snapshot("Version 3").unwrap();

    // Undo to s2
    let u1 = vcs.undo().unwrap();
    assert_eq!(u1, Some(s2.id));
    assert_eq!(vcs.head_snapshot().unwrap().unwrap().id, s2.id);

    // Undo to s1
    let u2 = vcs.undo().unwrap();
    assert_eq!(u2, Some(s1.id));
    assert_eq!(vcs.head_snapshot().unwrap().unwrap().id, s1.id);

    // Redo to s2
    let r1 = vcs.redo().unwrap();
    assert_eq!(r1, Some(s2.id));
    assert_eq!(vcs.head_snapshot().unwrap().unwrap().id, s2.id);
    assert_eq!(
        fs::read_to_string(temp.path().join("code.py")).unwrap(),
        "x = 20\n"
    );

    // Redo to s3
    let r2 = vcs.redo().unwrap();
    assert_eq!(r2, Some(s3.id));
    assert_eq!(vcs.head_snapshot().unwrap().unwrap().id, s3.id);
    assert_eq!(
        fs::read_to_string(temp.path().join("code.py")).unwrap(),
        "x = 30\n"
    );

    // Redo again when nothing to redo
    let r3 = vcs.redo().unwrap();
    assert_eq!(r3, None);
}
