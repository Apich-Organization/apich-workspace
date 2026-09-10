mod common;
use apich_vcs::{BundleOptions, ProjectVcs};
use common::test_temp_dir;
use std::fs;

/// Real "clone" + "push" + "pull" over apich-vcs's own bundle-based remote protocol, using two
/// independent on-disk repositories to stand in for a server and a client (the HTTP layer in
/// apich-web is a thin wrapper around exactly this same `export_bundle`/`accept_push_bundle`
/// pair, so this exercises the actual merge logic end to end without needing a live server).
#[test]
fn test_push_pull_clone_over_bundle_protocol() {
    let server_temp = test_temp_dir();
    let server_root = server_temp.path().join("server_repo");
    fs::create_dir_all(&server_root).unwrap();
    let server_vcs = ProjectVcs::open_or_init(&server_root).unwrap();
    fs::write(server_root.join("a.txt"), "v1\n").unwrap();
    let s1 = server_vcs.snapshot("first commit").unwrap();

    // "Clone": download the server's bundle and import it fresh.
    let client_temp = test_temp_dir();
    let client_root = client_temp.path().join("client_repo");
    let mut bundle_bytes = Vec::new();
    server_vcs.export_bundle(&mut bundle_bytes, BundleOptions::default()).unwrap();
    let client_vcs = ProjectVcs::import_bundle(&bundle_bytes[..], &client_root).unwrap();
    assert_eq!(
        fs::read_to_string(client_root.join("a.txt")).unwrap(),
        "v1\n"
    );

    // Client makes local progress and "pushes": server accepts it (fast-forward, new to server
    // only in the sense that the client is ahead -- same branch, server's HEAD is an ancestor).
    fs::write(client_root.join("a.txt"), "v2\n").unwrap();
    let s2 = client_vcs.snapshot("second commit").unwrap();
    let mut push_bytes = Vec::new();
    client_vcs.export_bundle(&mut push_bytes, BundleOptions::default()).unwrap();
    let outcome = server_vcs.accept_push_bundle(&push_bytes[..]).unwrap();
    assert_eq!(outcome.accepted_branches, vec!["main".to_string()]);
    assert!(outcome.rejected_branches.is_empty());
    assert_eq!(
        server_vcs.get_branch("main").unwrap().unwrap().head_snapshot_id,
        s2.id
    );
    // The pushed snapshot's tree must be a real, independently-readable object on the server now.
    let s2_from_server = server_vcs.cas().get_snapshot(s2.id).unwrap();
    assert_eq!(s2_from_server.parent_snapshot_id, Some(s1.id));

    // A third, independent repo (never touched by the push above) diverges from the same base,
    // then tries to push: the server must reject it rather than silently discarding history.
    let rogue_temp = test_temp_dir();
    let rogue_root = rogue_temp.path().join("rogue_repo");
    let mut base_bytes = Vec::new();
    // Re-export from a bundle taken before the client's second commit, to simulate a clone that
    // is now behind the server.
    let stale_client_temp = test_temp_dir();
    let stale_root = stale_client_temp.path().join("stale_repo");
    let _stale_vcs = ProjectVcs::import_bundle(&bundle_bytes[..], &stale_root).unwrap();
    let stale_vcs = ProjectVcs::open_or_init(&stale_root).unwrap();
    fs::write(stale_root.join("a.txt"), "conflicting-v2\n").unwrap();
    let _s2_rogue = stale_vcs.snapshot("conflicting second commit").unwrap();
    stale_vcs.export_bundle(&mut base_bytes, BundleOptions::default()).unwrap();
    let _ = fs::create_dir_all(&rogue_root);

    let rejected_outcome = server_vcs.accept_push_bundle(&base_bytes[..]).unwrap();
    assert!(rejected_outcome.accepted_branches.is_empty(), "diverged push must not silently win: {:?}", rejected_outcome);
    assert_eq!(rejected_outcome.rejected_branches.len(), 1);
    assert_eq!(rejected_outcome.rejected_branches[0].0, "main");
    // Server's branch ref must be unchanged -- still pointing at the legitimately-pushed s2.
    assert_eq!(
        server_vcs.get_branch("main").unwrap().unwrap().head_snapshot_id,
        s2.id
    );

    // "Pull": a fresh clone of the server now sees the client's pushed second commit.
    let puller_temp = test_temp_dir();
    let puller_root = puller_temp.path().join("puller_repo");
    let mut server_bytes_now = Vec::new();
    server_vcs.export_bundle(&mut server_bytes_now, BundleOptions::default()).unwrap();
    let puller_vcs = ProjectVcs::import_bundle(&server_bytes_now[..], &puller_root).unwrap();
    assert_eq!(
        fs::read_to_string(puller_root.join("a.txt")).unwrap(),
        "v2\n"
    );
    assert_eq!(
        puller_vcs.head_snapshot().unwrap().unwrap().id,
        s2.id
    );
}

#[test]
fn test_project_bundle_export_and_import_roundtrip() {
    let source_temp = test_temp_dir();
    let source_root = source_temp.path().join("server_repo");
    fs::create_dir_all(&source_root).unwrap();

    let vcs = ProjectVcs::open_or_init(&source_root).unwrap();

    // 1. Create a project with code, documentation and data
    fs::write(
        source_root.join("paper.typ"),
        "= Academic Paper on Distributed VCS\nAuthor: APICH\n",
    )
    .unwrap();

    fs::create_dir_all(source_root.join("assets")).unwrap();
    let binary_data = vec![0xCA, 0xFE, 0xBA, 0xBE, 0x01, 0x02, 0x03, 0x04];
    fs::write(source_root.join("assets").join("logo.bin"), &binary_data).unwrap();

    let s1 = vcs.snapshot("Initial project setup").unwrap();
    vcs.create_milestone("v1.0-draft", "First complete draft").unwrap();

    // Create a branch
    vcs.branch_create("experiment").unwrap();
    vcs.branch_switch("experiment").unwrap();
    fs::write(
        source_root.join("experiment.txt"),
        "Experimental results: accuracy 98.4%\n",
    )
    .unwrap();
    let s_exp = vcs.snapshot("Experiment findings").unwrap();

    // Switch back to main
    vcs.branch_switch("main").unwrap();

    // 2. Export project bundle to a package file (simulating gateway packaging API)
    let bundle_file = source_temp.path().join("project_export.apich-bundle");
    vcs.export_bundle_to_file(&bundle_file, BundleOptions::default())
        .unwrap();

    assert!(bundle_file.exists());
    assert!(fs::metadata(&bundle_file).unwrap().len() > 0);

    // 3. Import bundle into an entirely new destination directory (simulating local client import)
    let client_temp = test_temp_dir();
    let client_root = client_temp.path().join("local_working_copy");

    let imported_vcs = ProjectVcs::import_bundle_from_file(&bundle_file, &client_root).unwrap();

    // Verify imported state
    assert_eq!(
        imported_vcs.current_branch().unwrap(),
        Some("main".to_string())
    );

    // Working copy files must be physically materialized
    assert_eq!(
        fs::read_to_string(client_root.join("paper.typ")).unwrap(),
        "= Academic Paper on Distributed VCS\nAuthor: APICH\n"
    );
    assert_eq!(
        fs::read(client_root.join("assets").join("logo.bin")).unwrap(),
        binary_data
    );

    // Milestones must be preserved
    let milestones = imported_vcs.list_milestones().unwrap();
    assert_eq!(milestones.len(), 1);
    assert_eq!(milestones[0].milestone_name.as_deref(), Some("v1.0-draft"));

    // Branches must be preserved
    let exp_branch = imported_vcs.get_branch("experiment").unwrap().unwrap();
    assert_eq!(exp_branch.head_snapshot_id, s_exp.id);

    // Can switch to experiment branch in the imported repository
    imported_vcs.branch_switch("experiment").unwrap();
    assert_eq!(
        fs::read_to_string(client_root.join("experiment.txt")).unwrap(),
        "Experimental results: accuracy 98.4%\n"
    );

    // Can continue making new snapshots in the imported repository
    fs::write(
        client_root.join("paper.typ"),
        "= Academic Paper on Distributed VCS\nAuthor: APICH\n\nLocal edits continue!\n",
    )
    .unwrap();
    let s_local = imported_vcs.snapshot("Local continuation").unwrap();
    assert_ne!(s1.id, s_local.id);
}

#[test]
fn test_clean_snapshot_archive_export() {
    let temp = test_temp_dir();
    let vcs = ProjectVcs::open_or_init(temp.path()).unwrap();

    fs::write(temp.path().join("main.typ"), "= Thesis\n").unwrap();
    fs::create_dir_all(temp.path().join("figures")).unwrap();
    fs::write(temp.path().join("figures").join("fig1.svg"), "<svg></svg>").unwrap();

    let s1 = vcs.snapshot("Ready for submission").unwrap();

    // Export clean archive
    let archive_path = temp.path().join("submission.tar.gz");
    vcs.export_snapshot_archive_to_file(s1.id, &archive_path)
        .unwrap();

    assert!(archive_path.exists());

    // Unpack archive into a check directory
    let unpack_dir = temp.path().join("unpacked_submission");
    fs::create_dir_all(&unpack_dir).unwrap();

    let file = fs::File::open(&archive_path).unwrap();
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    archive.unpack(&unpack_dir).unwrap();

    // Clean archive must contain project files, and NOT contain .apich
    assert!(unpack_dir.join("main.typ").exists());
    assert!(unpack_dir.join("figures/fig1.svg").exists());
    assert!(!unpack_dir.join(".apich").exists());
    assert!(!unpack_dir.join("bundle.json").exists());
}
