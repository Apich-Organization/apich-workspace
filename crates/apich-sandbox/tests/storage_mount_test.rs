use apich_sandbox::SandboxManager;
use tempfile::tempdir;

#[tokio::test]
async fn test_storage_mount_and_file_persistence() {
    let temp = tempdir().unwrap();
    let manager =
        SandboxManager::new(temp.path(), "docker.io/library/alpine:latest").with_selinux(true);

    let user_id = "test_user_storage";
    let container = manager
        .ensure_running(user_id)
        .await
        .expect("Failed to ensure running");

    // 1. Host writes a file to workspace
    let file_content = "Hello from Host File System!";
    container
        .save_file_str("notes/test.txt", file_content)
        .await
        .expect("Failed to save file from host");

    // Verify file_exists on host
    assert!(container.file_exists("notes/test.txt"));

    // 2. Container reads the file from inside /workspace
    let read_res = container
        .exec(&["cat", "notes/test.txt"])
        .await
        .expect("Failed to exec cat in container");
    read_res.ensure_success(container.container_name()).unwrap();
    assert_eq!(read_res.stdout_lossy().trim(), file_content);

    // 3. Container writes a file inside /workspace
    let container_msg = "Created inside container via shell!";
    let write_res = container
        .exec(&[
            "sh",
            "-c",
            &format!("echo '{}' > /workspace/from_container.txt", container_msg),
        ])
        .await
        .expect("Failed to exec echo in container");
    write_res
        .ensure_success(container.container_name())
        .unwrap();

    // 4. Host reads the file written by container
    let read_from_host = container
        .read_file_str("from_container.txt")
        .await
        .expect("Failed to read file from host");
    assert_eq!(read_from_host.trim(), container_msg);

    // 5. List files in workspace
    let entries = container
        .list_files(".")
        .await
        .expect("Failed to list files");
    let names: Vec<String> = entries.into_iter().map(|e| e.name).collect();
    assert!(names.contains(&"notes".to_string()));
    assert!(names.contains(&"from_container.txt".to_string()));

    // 6. Delete file
    container
        .remove_file("from_container.txt")
        .await
        .expect("Failed to delete file");
    assert!(!container.file_exists("from_container.txt"));

    // Cleanup
    container
        .destroy()
        .await
        .expect("Failed to destroy container");
}
