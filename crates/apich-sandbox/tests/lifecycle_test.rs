use apich_sandbox::{ContainerStatus, PodmanDriver, SandboxManager};
use tempfile::tempdir;

#[tokio::test]
async fn test_podman_available() {
    let driver = PodmanDriver::default();
    let version = driver
        .check_available()
        .await
        .expect("Podman should be available");
    println!("Detected Podman version: {}", version);
    assert!(!version.is_empty());
}

/// Regression test for a real, previously-invisible permission bug: rootless Podman's default
/// user-namespace mapping puts the *host* UID that owns the bind-mounted workspace at UID 0
/// (root) inside the container's namespace -- but `apich-sandbox:latest` deliberately runs its
/// commands as a non-root user (`apich`, UID 1000, see `docker/Containerfile.sandbox`), which
/// doesn't map to that, so every file in the mount shows as `root root` from inside the
/// container, and any write from `apich` fails with a real "Permission denied". Found live while
/// building the LaTeX-preview feature (`pdflatex` couldn't write its own `.log` file) -- a bare
/// `alpine` image wouldn't reproduce this, since its default user *is* root, which the default
/// mapping already lets through; the bug specifically needs a non-root container user, which is
/// exactly what the real image (and every real project sandbox) uses. Without
/// `--userns=keep-id` this reproduces even for something as simple as `touch`; with it, the
/// container's UID maps 1:1 to the host UID that owns the directory, and writes succeed.
#[tokio::test]
async fn test_keep_id_allows_writes_to_mounted_workspace() {
    let temp = tempdir().unwrap();
    let manager = SandboxManager::new(temp.path(), "localhost/apich-sandbox:latest")
        .with_selinux(true)
        .with_keep_id(true);

    let user_id = "test_user_keep_id_write";
    let container = manager.ensure_running(user_id).await.expect("Failed to ensure running");

    let result = container
        .exec(["sh", "-c", "id; touch /workspace/keep-id-write-check.txt && echo WRITE_OK || echo WRITE_FAIL"])
        .await
        .expect("exec should succeed");
    let combined = format!("{}{}", result.stdout_lossy(), result.stderr_lossy());
    assert!(
        combined.contains("WRITE_OK"),
        "write into the mounted workspace should succeed with --userns=keep-id: {combined}"
    );

    container.destroy().await.expect("Failed to destroy container");
}

/// The other half of the regression pair above: proves the test actually detects the bug (and
/// that it isn't just passing for an unrelated reason) by confirming the write genuinely fails
/// *without* `--userns=keep-id` -- the exact behavior `ProjectManagerService::project_sandbox_config`
/// used to have before this fix, and would silently regress back to if `.keep_id(true)` were
/// ever removed there.
#[tokio::test]
async fn test_without_keep_id_write_to_mounted_workspace_fails() {
    let temp = tempdir().unwrap();
    let manager = SandboxManager::new(temp.path(), "localhost/apich-sandbox:latest").with_selinux(true);

    let user_id = "test_user_no_keep_id_write";
    let container = manager.ensure_running(user_id).await.expect("Failed to ensure running");

    let result = container
        .exec(["sh", "-c", "id; touch /workspace/no-keep-id-write-check.txt && echo WRITE_OK || echo WRITE_FAIL"])
        .await
        .expect("exec should succeed");
    let combined = format!("{}{}", result.stdout_lossy(), result.stderr_lossy());
    assert!(
        combined.contains("WRITE_FAIL"),
        "this test documents the bug this session found and fixed -- if this now passes, either \
         the image or Podman's defaults changed underneath it and `project_sandbox_config`'s \
         `.keep_id(true)` may no longer be necessary (worth re-checking, not just relaxing this \
         assertion): {combined}"
    );

    container.destroy().await.expect("Failed to destroy container");
}

#[tokio::test]
async fn test_user_container_lifecycle() {
    let temp = tempdir().unwrap();
    let manager =
        SandboxManager::new(temp.path(), "docker.io/library/alpine:latest").with_selinux(true);

    let user_id = "test_user_lifecycle";

    // 1. Ensure running (should spawn container)
    let container = manager
        .ensure_running(user_id)
        .await
        .expect("Failed to ensure running");
    assert_eq!(container.user_id(), user_id);
    assert!(container.is_running().await.unwrap());
    assert_eq!(container.status().await.unwrap(), ContainerStatus::Running);

    // 2. Ensure running again (should be idempotent)
    let container2 = manager
        .ensure_running(user_id)
        .await
        .expect("Idempotent ensure running failed");
    assert!(container2.is_running().await.unwrap());

    // 3. Pause and unpause
    container.pause().await.expect("Failed to pause");
    assert_eq!(container.status().await.unwrap(), ContainerStatus::Paused);

    container.unpause().await.expect("Failed to unpause");
    assert_eq!(container.status().await.unwrap(), ContainerStatus::Running);

    // 4. Stop container
    container.stop(2).await.expect("Failed to stop container");
    let status = container.status().await.unwrap();
    assert!(status == ContainerStatus::Exited || status == ContainerStatus::Stopped);

    // 5. Restart container
    container
        .start()
        .await
        .expect("Failed to start stopped container");
    assert!(container.is_running().await.unwrap());

    // 6. Destroy container
    container
        .destroy()
        .await
        .expect("Failed to destroy container");
    let inspect_res = container.inspect().await.unwrap();
    assert!(inspect_res.is_none());
}
