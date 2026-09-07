mod common;
use apich_vcs::{BundleOptions, ProjectVcs};
use common::test_temp_dir;
use std::fs;

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
