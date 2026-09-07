use apich_sandbox::SandboxManager;
use tempfile::tempdir;

#[tokio::test]
async fn test_toolchain_git_python_and_r() {
    let temp = tempdir().unwrap();
    // Use pre-built local test image containing bash, git, python3, pip, R
    let manager =
        SandboxManager::new(temp.path(), "localhost/apich-sandbox:test").with_selinux(true);

    let user_id = "test_user_tools_all";
    let container = manager
        .ensure_running(user_id)
        .await
        .expect("Failed to ensure running");

    // 1. Test Python Toolchain
    let py = container.python();
    let py_ver = py
        .python_version()
        .await
        .expect("Failed to get python version");
    assert!(py_ver.starts_with("Python 3."));

    let code_res = py
        .run_code("print(100 * 25)")
        .await
        .expect("Failed to run code");
    assert_eq!(code_res.stdout_lossy().trim(), "2500");

    // Save python script and run file
    container
        .save_file_str(
            "calc.py",
            "import sys\nprint('Total:', sum(int(x) for x in sys.argv[1:]))\n",
        )
        .await
        .unwrap();
    let script_res = py
        .run_file("calc.py", &["10", "20", "30"])
        .await
        .expect("Failed to run script");
    assert_eq!(script_res.stdout_lossy().trim(), "Total: 60");

    // 2. Test Git Toolchain
    let git = container.git();
    let git_ver = git.git_version().await.expect("Failed to get git version");
    assert!(git_ver.starts_with("git version"));

    // Init repository
    git.init(None).await.expect("Failed to git init");

    // Create a file and add it
    container
        .save_file_str("README.md", "# APICH Project\n")
        .await
        .unwrap();
    git.add(None, "README.md").await.expect("Failed to git add");

    // Commit
    git.commit(None, "Initial workspace commit", None)
        .await
        .expect("Failed to git commit");

    // Check log
    let log_res = git.log(None, Some(5)).await.expect("Failed to git log");
    assert!(log_res.stdout_lossy().contains("Initial workspace commit"));

    // Status check
    let status_res = git.status(None).await.expect("Failed to git status");
    assert!(status_res.stdout_lossy().contains("?? calc.py"));

    // 3. Test R Toolchain
    let r = container.r();
    let r_ver = r.r_version().await.expect("Failed to get R version");
    assert!(r_ver.contains("R version"));

    let r_res = r.run_code("cat(2^8)").await.expect("Failed to run R code");
    assert_eq!(r_res.stdout_lossy().trim(), "256");

    // Cleanup
    container
        .destroy()
        .await
        .expect("Failed to destroy container");
}

#[tokio::test]
async fn test_toolchain_helpers_structure() {
    let temp = tempdir().unwrap();
    let manager =
        SandboxManager::new(temp.path(), "docker.io/library/alpine:latest").with_selinux(true);

    let user_id = "test_user_toolchains_struct";
    let container = manager
        .ensure_running(user_id)
        .await
        .expect("Failed to ensure running");

    // Rust toolchain command structure
    let rust = container.rust();
    let rust_res = rust.cargo_version().await;
    assert!(
        rust_res.is_err(),
        "Cargo should not be installed yet on bare alpine"
    );

    // Typst toolchain
    let typst = container.typst();
    let typst_res = typst.typst_version().await;
    assert!(
        typst_res.is_err(),
        "Typst should not be installed yet on bare alpine"
    );

    // LaTeX toolchain
    let latex = container.latex();
    let latex_res = latex
        .clean_aux_files(None)
        .await
        .expect("rm command should succeed");
    assert!(latex_res.success());

    // Cleanup
    container
        .destroy()
        .await
        .expect("Failed to destroy container");
}
