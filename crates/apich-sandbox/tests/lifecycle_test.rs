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
