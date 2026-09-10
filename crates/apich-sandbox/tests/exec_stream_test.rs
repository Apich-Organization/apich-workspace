use apich_sandbox::{ExecOptions, OutputChunk, SandboxError, SandboxManager};
use std::time::Duration;
use tempfile::tempdir;

#[tokio::test]
async fn test_exec_stream_and_options() {
    let temp = tempdir().unwrap();
    let manager =
        SandboxManager::new(temp.path(), "docker.io/library/alpine:latest").with_selinux(true);

    let user_id = "test_user_exec";
    let container = manager
        .ensure_running(user_id)
        .await
        .expect("Failed to ensure running");

    // 1. Basic command execution
    let res = container.exec(&["echo", "hello world"]).await.unwrap();
    assert!(res.success());
    assert_eq!(res.stdout_lossy().trim(), "hello world");
    assert!(res.stderr.is_empty());

    // 2. Command failure handling
    let fail_res = container
        .exec(&["sh", "-c", "echo 'something broke' >&2; exit 42"])
        .await
        .unwrap();
    assert_eq!(fail_res.exit_code, 42);
    assert!(fail_res.stderr_lossy().contains("something broke"));
    assert!(!fail_res.success());

    // 3. Environment variables and working directory
    let opts = ExecOptions::new(["sh", "-c", "echo WORKDIR=$(pwd); echo ENV=$CUSTOM_ENV"])
        .working_dir("/tmp")
        .env("CUSTOM_ENV", "APICH_TEST_123");
    let env_res = container.exec_with_options(opts).await.unwrap();
    assert!(env_res.success());
    let out = env_res.stdout_lossy();
    assert!(out.contains("WORKDIR=/tmp"));
    assert!(out.contains("ENV=APICH_TEST_123"));

    // 4. Real-time streaming execution
    let stream_opts = ExecOptions::new([
        "sh",
        "-c",
        "echo chunk1; echo chunk2; echo error_log >&2; echo chunk3",
    ]);
    let mut stream = container.exec_stream(stream_opts).await.unwrap();
    let mut stdout_chunks = Vec::new();
    let mut stderr_chunks = Vec::new();
    let mut exit_received = None;

    while let Some(chunk) = stream.next_chunk().await {
        match chunk {
            OutputChunk::Stdout(bytes) => stdout_chunks.extend_from_slice(&bytes),
            OutputChunk::Stderr(bytes) => stderr_chunks.extend_from_slice(&bytes),
            OutputChunk::Exit(code) => exit_received = Some(code),
        }
    }

    let all_stdout = String::from_utf8_lossy(&stdout_chunks);
    let all_stderr = String::from_utf8_lossy(&stderr_chunks);

    assert!(all_stdout.contains("chunk1"));
    assert!(all_stdout.contains("chunk2"));
    assert!(all_stdout.contains("chunk3"));
    assert!(all_stderr.contains("error_log"));
    assert_eq!(exit_received, Some(0));

    // 5. Execution timeout test
    let timeout_opts = ExecOptions::new(["sleep", "3"]).timeout(Duration::from_millis(300));
    let timeout_res = container.exec_with_options(timeout_opts).await;
    match timeout_res {
        Err(SandboxError::ExecutionTimeout(dur)) => {
            assert_eq!(dur, Duration::from_millis(300));
        }
        other => panic!("Expected ExecutionTimeout error, got {:?}", other),
    }

    // Cleanup
    container
        .destroy()
        .await
        .expect("Failed to destroy container");
}

/// Proves the interactive-exec stdin plumbing actually reaches the container process, not just
/// that the API compiles -- this is the mechanism the real agent-CLI account-login flow
/// (`claude auth login`, which waits for a pasted-back code) depends on.
#[tokio::test]
async fn test_exec_interactive_stdin_round_trip() {
    let temp = tempdir().unwrap();
    let manager =
        SandboxManager::new(temp.path(), "docker.io/library/alpine:latest").with_selinux(true);

    let container = manager
        .ensure_running("test_user_exec_interactive")
        .await
        .expect("Failed to ensure running");

    // A script that prompts, reads one line from stdin, and echoes it back -- mirrors the real
    // shape of `claude auth login`'s "print a prompt, wait for pasted input" behavior closely
    // enough to prove the plumbing without depending on a real OAuth flow in a test.
    let opts = ExecOptions::new([
        "sh",
        "-c",
        "echo READY; read -r line; echo GOT:$line",
    ]);
    let mut session = container.exec_interactive(opts).await.unwrap();

    // Wait for the "READY" prompt before writing, same as a real UI would wait for the login
    // URL to appear before showing the user a code-input box.
    let mut buf = Vec::new();
    loop {
        match session.stream.next_chunk().await {
            Some(OutputChunk::Stdout(bytes)) => {
                buf.extend_from_slice(&bytes);
                if String::from_utf8_lossy(&buf).contains("READY") {
                    break;
                }
            }
            Some(OutputChunk::Exit(_)) | None => panic!("process exited before printing READY"),
            _ => {}
        }
    }

    session
        .stdin_tx
        .send(b"pasted-code-123\n".to_vec())
        .await
        .expect("stdin channel should still be open");

    let mut exit_code = None;
    loop {
        match session.stream.next_chunk().await {
            Some(OutputChunk::Stdout(bytes)) => buf.extend_from_slice(&bytes),
            Some(OutputChunk::Exit(code)) => {
                exit_code = Some(code);
                break;
            }
            Some(OutputChunk::Stderr(_)) => {}
            None => break,
        }
    }

    let all = String::from_utf8_lossy(&buf);
    assert!(all.contains("GOT:pasted-code-123"), "stdin write should have reached the process: {all}");
    assert_eq!(exit_code, Some(0));

    container.destroy().await.expect("Failed to destroy container");
}
