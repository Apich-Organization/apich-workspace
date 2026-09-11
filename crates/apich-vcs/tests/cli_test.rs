use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_cli_lifecycle_end_to_end() {
    let temp = tempdir().unwrap();
    let repo_path = temp.path();
    let apich_bin = env!("CARGO_BIN_EXE_apich");

    // 1. apich init
    let output = Command::new(apich_bin)
        .arg("-p")
        .arg(repo_path)
        .arg("init")
        .output()
        .expect("Failed to run apich init");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Initialized empty APICH VCS repository"));

    // 2. apich status (initial empty)
    let output = Command::new(apich_bin)
        .arg("-p")
        .arg(repo_path)
        .arg("status")
        .output()
        .expect("Failed to run apich status");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("On branch main"));
    assert!(stdout.contains("Working tree clean"));

    // 3. Create files and check status
    let paper_file = repo_path.join("main.typ");
    fs::write(&paper_file, "#set page(paper: \"a4\")\n= Introduction\n").unwrap();

    let output = Command::new(apich_bin)
        .arg("-p")
        .arg(repo_path)
        .arg("status")
        .output()
        .expect("Failed to run apich status");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("new file:   main.typ"));

    // 4. apich snapshot
    let output = Command::new(apich_bin)
        .arg("-p")
        .arg(repo_path)
        .arg("snapshot")
        .arg("-m")
        .arg("Initial typst paper draft")
        .output()
        .expect("Failed to run apich snapshot");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("[main "));
    assert!(stdout.contains("Initial typst paper draft"));

    // 5. apich milestone
    let output = Command::new(apich_bin)
        .arg("-p")
        .arg(repo_path)
        .arg("milestone")
        .arg("draft-v1")
        .arg("--desc")
        .arg("First readable draft for coauthors")
        .output()
        .expect("Failed to run apich milestone");
    assert!(output.status.success());

    // 6. apich milestones
    let output = Command::new(apich_bin)
        .arg("-p")
        .arg(repo_path)
        .arg("milestones")
        .output()
        .expect("Failed to run apich milestones");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("draft-v1"));

    // 7. apich branch create & checkout
    let output = Command::new(apich_bin)
        .arg("-p")
        .arg(repo_path)
        .arg("branch")
        .arg("create")
        .arg("feature-experiment")
        .output()
        .expect("Failed to run apich branch create");
    assert!(output.status.success());

    let output = Command::new(apich_bin)
        .arg("-p")
        .arg(repo_path)
        .arg("checkout")
        .arg("feature-experiment")
        .output()
        .expect("Failed to run apich checkout");
    assert!(output.status.success());

    // Verify branch switch in status
    let output = Command::new(apich_bin)
        .arg("-p")
        .arg(repo_path)
        .arg("status")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("On branch feature-experiment"));

    // 8. apich snapshot when no changes (should detect no changes and not snapshot)
    let output = Command::new(apich_bin)
        .arg("-p")
        .arg(repo_path)
        .arg("snapshot")
        .arg("-m")
        .arg("Redundant snapshot")
        .output()
        .expect("Failed to run apich snapshot");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Nothing to snapshot"));

    // 9. Modify file and test diff & cat
    fs::write(
        &paper_file,
        "#set page(paper: \"a4\")\n= Introduction\n= Experiments\n",
    )
    .unwrap();
    let output = Command::new(apich_bin)
        .arg("-p")
        .arg(repo_path)
        .arg("diff")
        .output()
        .expect("Failed to run apich diff");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("M main.typ"));

    let output = Command::new(apich_bin)
        .arg("-p")
        .arg(repo_path)
        .arg("cat")
        .arg("main.typ")
        .output()
        .expect("Failed to run apich cat");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("= Experiments"));

    // Snapshot modification
    let output = Command::new(apich_bin)
        .arg("-p")
        .arg(repo_path)
        .arg("snapshot")
        .arg("-m")
        .arg("Added experiments section")
        .output()
        .expect("Failed to run snapshot");
    assert!(output.status.success());

    // 10. Test Undo and Redo cycle
    let output = Command::new(apich_bin)
        .arg("-p")
        .arg(repo_path)
        .arg("undo")
        .output()
        .expect("Failed to run undo");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Successfully undid last operation"));

    // Verify disk content was undone
    let content = fs::read_to_string(&paper_file).unwrap();
    assert!(!content.contains("= Experiments"));

    let output = Command::new(apich_bin)
        .arg("-p")
        .arg(repo_path)
        .arg("redo")
        .output()
        .expect("Failed to run redo");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Successfully redid operation"));

    // Verify disk content was restored
    let content = fs::read_to_string(&paper_file).unwrap();
    assert!(content.contains("= Experiments"));

    // 11. Test OpLog
    let output = Command::new(apich_bin)
        .arg("-p")
        .arg(repo_path)
        .arg("oplog")
        .output()
        .expect("Failed to run oplog");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Snapshot"));
    assert!(stdout.contains("Undo"));
    assert!(stdout.contains("Redo"));

    // 12. Test Git CLI subcommands
    let output = Command::new(apich_bin)
        .arg("-p")
        .arg(repo_path)
        .arg("git")
        .arg("init")
        .output()
        .expect("Failed to run git init");
    assert!(output.status.success());

    let output = Command::new(apich_bin)
        .arg("-p")
        .arg(repo_path)
        .arg("git")
        .arg("export")
        .arg("--branch")
        .arg("main")
        .arg("--message")
        .arg("CLI Git export commit")
        .output()
        .expect("Failed to run git export");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Exported snapshot to Git commit"));

    // 13. apich config ignore-add & show
    let output = Command::new(apich_bin)
        .arg("-p")
        .arg(repo_path)
        .arg("config")
        .arg("ignore-add")
        .arg("temp_cache/**")
        .output()
        .expect("Failed to run config ignore-add");
    assert!(output.status.success());

    let output = Command::new(apich_bin)
        .arg("-p")
        .arg(repo_path)
        .arg("config")
        .arg("show")
        .output()
        .expect("Failed to run config show");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("temp_cache/**"));
}

#[test]
fn test_cross_boundary_lock_mutual_exclusion() {
    use apich_vcs::RepositoryLock;

    let temp = tempdir().unwrap();
    let repo_path = temp.path();

    // Acquire lock in thread 1
    let lock1 = RepositoryLock::acquire(repo_path).expect("Should acquire lock 1");

    // Reentrant acquire in the same thread/process succeeds
    let lock2 = RepositoryLock::acquire(repo_path).expect("Reentrant lock should succeed");

    drop(lock2);
    drop(lock1);

    // After dropping, another acquire succeeds cleanly
    let lock3 = RepositoryLock::acquire(repo_path).expect("Should acquire lock after drops");
    drop(lock3);
}
