use apich_sandbox::SandboxManager;
use apich_vcs::ProjectVcs;
use apich_vcs::RepositoryLock;
use std::fs;
use tempfile::tempdir;

#[tokio::test]
async fn test_container_and_git_e2e_integration() {
    let temp = tempdir().expect("Failed to create tempdir");
    let workspace_root = temp.path().to_path_buf();

    // 1. Start sandbox container with bind-mounted workspace
    let manager =
        SandboxManager::new(&workspace_root, "localhost/apich-sandbox:test").with_selinux(true);

    let unique_suffix = &uuid::Uuid::now_v7().to_string()[..8];
    let user_id = format!("e2e_{}", unique_suffix);
    let _ = manager.destroy_user_sandbox(&user_id).await;
    let container = manager
        .ensure_running(&user_id)
        .await
        .expect("Failed to start sandbox container");

    assert!(container.is_running().await.unwrap());

    // 2. Initialize ProjectVcs on host side pointing to the workspace
    let user_workspace = workspace_root.join(&user_id);
    fs::create_dir_all(&user_workspace).unwrap();
    let vcs = ProjectVcs::open_or_init(&user_workspace).expect("Failed to open_or_init ProjectVcs");

    assert_eq!(vcs.current_branch().unwrap(), Some("main".to_string()));
    assert!(!vcs.has_changes().unwrap());

    // 3. Inside container: create a Python script and data file
    container
        .save_file_str(
            "simulate.py",
            r#"import json
results = {'iterations': 100, 'accuracy': 0.985}
with open('metrics.json', 'w') as f:
    json.dump(results, f)
print('Simulation complete')
"#,
        )
        .await
        .expect("Failed to save script in container");

    // 4. Verify host VCS immediately sees the file via Linux Inode bind-mount
    assert!(vcs.has_changes().unwrap());
    let status_before_exec = vcs.status().unwrap();
    assert!(status_before_exec
        .added
        .contains(&"simulate.py".to_string()));

    // 5. In container: execute Python script via container toolchain
    let exec_res = container
        .exec(&["python3", "simulate.py"])
        .await
        .expect("Failed to run python in container");
    assert!(
        exec_res.stdout_lossy().contains("Simulation complete"),
        "Failed output: stdout='{}', stderr='{}'",
        exec_res.stdout_lossy(),
        exec_res.stderr_lossy()
    );

    // 6. Host VCS sees generated 'metrics.json'
    let status_after_exec = vcs.status().unwrap();
    assert!(status_after_exec
        .added
        .contains(&"metrics.json".to_string()));

    // 7. Commit snapshot from host
    let s1 = vcs
        .snapshot("Initial simulation code and metrics")
        .expect("Failed to create snapshot");
    assert_eq!(s1.message, "Initial simulation code and metrics");
    assert!(!vcs.has_changes().unwrap());

    // 8. Verify conditional snapshotting: no changes -> snapshot_if_changed returns None!
    let redundant_snap = vcs
        .snapshot_if_changed("Redundant attempt")
        .expect("Failed to check snapshot_if_changed");
    assert!(redundant_snap.is_none());
    assert_eq!(vcs.list_snapshots().unwrap().len(), 1);

    // 9. Git Bridge Integration: Export VCS snapshot to standard Git commit
    vcs.git_init().expect("Failed to init git bridge");
    let commit_oid = vcs
        .git_export_commit(
            "main",
            "Export simulation results to Git",
            "Alice Scientist",
            "alice@laboratory.org",
        )
        .expect("Failed to export snapshot to git commit");
    assert!(!commit_oid.is_empty());

    // 10. Container Git toolchain inspects the exported Git commit
    let git_log_out = container
        .exec(&["git", "log", "-1", "--pretty=format:%s|%an"])
        .await
        .expect("Container git log failed");
    let git_log_str = git_log_out.stdout_lossy();
    assert!(git_log_str.contains("Export simulation results to Git"));
    assert!(git_log_str.contains("Alice Scientist"));

    // 11. In container: modify a file and commit using container's native Git CLI
    container
        .save_file_str(
            "notes.md",
            "# Lab Experiment Notes\nConfirmed high precision.\n",
        )
        .await
        .unwrap();

    let git_tool = container.git();
    git_tool.add(None, "notes.md").await.unwrap();
    git_tool
        .commit(
            None,
            "Add experiment notes from container git",
            Some(("Alice Scientist", "alice@laboratory.org")),
        )
        .await
        .unwrap();

    // 12. Host VCS detects the container Git modification
    assert!(vcs.has_changes().unwrap());
    let s2 = vcs
        .snapshot("Synchronized container git commit")
        .expect("Failed to snapshot container changes");
    assert_eq!(s2.parent_snapshot_id, Some(s1.id));
    assert_eq!(vcs.list_snapshots().unwrap().len(), 2);

    // 13. Test cross-process / cross-boundary advisory locking
    {
        let _lock = RepositoryLock::acquire(&user_workspace).expect("Should acquire lock");
        assert!(user_workspace.join(".apich").join("lock").exists());
        // Nested acquire in same thread succeeds due to reentrant design
        let _nested =
            RepositoryLock::acquire(&user_workspace).expect("Reentrant lock should succeed");
    }

    // 14. Test apich CLI commands directly against the container workspace
    let apich_bin = env!("CARGO_BIN_EXE_apich");

    // CLI status
    let cli_status = std::process::Command::new(apich_bin)
        .arg("-p")
        .arg(&user_workspace)
        .arg("status")
        .output()
        .expect("CLI status failed");
    assert!(cli_status.status.success());
    let cli_out = String::from_utf8_lossy(&cli_status.stdout);
    assert!(cli_out.contains("On branch main"));

    // Modify file inside container
    container
        .save_file_str(
            "notes.md",
            "# Lab Experiment Notes\nConfirmed high precision.\nAdded via CLI test.\n",
        )
        .await
        .unwrap();

    // CLI diff
    let cli_diff = std::process::Command::new(apich_bin)
        .arg("-p")
        .arg(&user_workspace)
        .arg("diff")
        .output()
        .expect("CLI diff failed");
    assert!(cli_diff.status.success());
    let diff_out = String::from_utf8_lossy(&cli_diff.stdout);
    assert!(diff_out.contains("M notes.md"));

    // CLI snapshot
    let cli_snap = std::process::Command::new(apich_bin)
        .arg("-p")
        .arg(&user_workspace)
        .arg("snapshot")
        .arg("-m")
        .arg("CLI committed changes")
        .output()
        .expect("CLI snapshot failed");
    assert!(cli_snap.status.success());

    // CLI undo
    let cli_undo = std::process::Command::new(apich_bin)
        .arg("-p")
        .arg(&user_workspace)
        .arg("undo")
        .output()
        .expect("CLI undo failed");
    assert!(cli_undo.status.success());

    // CLI redo
    let cli_redo = std::process::Command::new(apich_bin)
        .arg("-p")
        .arg(&user_workspace)
        .arg("redo")
        .output()
        .expect("CLI redo failed");
    assert!(cli_redo.status.success());

    // CLI oplog
    let cli_oplog = std::process::Command::new(apich_bin)
        .arg("-p")
        .arg(&user_workspace)
        .arg("oplog")
        .output()
        .expect("CLI oplog failed");
    assert!(cli_oplog.status.success());
    let oplog_out = String::from_utf8_lossy(&cli_oplog.stdout);
    assert!(oplog_out.contains("Snapshot"));
    assert!(oplog_out.contains("Undo"));
    assert!(oplog_out.contains("Redo"));

    // 15. Stop and destroy container
    container
        .destroy()
        .await
        .expect("Failed to destroy container");
}
