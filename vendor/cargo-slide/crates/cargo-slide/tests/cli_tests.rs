use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_cli_version_and_help() {
    let bin = env!("CARGO_BIN_EXE_cargo-slide");

    let output = Command::new(bin)
        .arg("--version")
        .output()
        .expect("Failed to execute cargo-slide --version");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("cargo-slide 0.1.1"));

    let help_output = Command::new(bin)
        .arg("--help")
        .output()
        .expect("Failed to execute cargo-slide --help");
    assert!(help_output.status.success());
    let help_stdout = String::from_utf8_lossy(&help_output.stdout);
    assert!(help_stdout.contains("Modern code-driven presentation system"));
    assert!(help_stdout.contains("init"));
    assert!(help_stdout.contains("build"));
    assert!(help_stdout.contains("export"));
}

#[test]
fn test_cli_init_and_new() {
    let bin = env!("CARGO_BIN_EXE_cargo-slide");
    let dir = tempdir().expect("tempdir");

    // Test new
    let new_res = Command::new(bin)
        .arg("new")
        .arg("my_presentation")
        .current_dir(dir.path())
        .output()
        .expect("Execute cargo-slide new");
    assert!(new_res.status.success());

    let proj_dir = dir.path().join("my_presentation");
    assert!(proj_dir.join("slides.typ").exists());
    assert!(proj_dir.join("theme.typ").exists());
    assert!(proj_dir.join("slide.typ").exists());
    assert!(proj_dir.join("assets/data.csv").exists());
    assert!(proj_dir.join(".gitignore").exists());

    // Test init in existing dir
    let init_dir = dir.path().join("init_project");
    std::fs::create_dir_all(&init_dir).unwrap();
    let init_res = Command::new(bin)
        .arg("init")
        .current_dir(&init_dir)
        .output()
        .expect("Execute cargo-slide init");
    assert!(init_res.status.success());
    assert!(init_dir.join("slides.typ").exists());
}

#[test]
fn test_cli_export_pdf_and_svg() {
    let bin = env!("CARGO_BIN_EXE_cargo-slide");
    let dir = tempdir().expect("tempdir");

    // Create presentation
    let init_res = Command::new(bin)
        .arg("init")
        .current_dir(dir.path())
        .output()
        .expect("Execute cargo-slide init");
    assert!(init_res.status.success());

    // Export PDF
    let out_pdf = dir.path().join("output.pdf");
    let pdf_res = Command::new(bin)
        .arg("export")
        .arg("slides.typ")
        .arg("--format")
        .arg("pdf")
        .arg("-o")
        .arg(&out_pdf)
        .current_dir(dir.path())
        .output()
        .expect("Execute cargo-slide export pdf");
    assert!(pdf_res.status.success(), "PDF export failed: {:?}", pdf_res);
    assert!(out_pdf.exists());
    assert!(out_pdf.metadata().unwrap().len() > 0);

    // Export SVG
    let out_svgs = dir.path().join("output_svgs");
    let svg_res = Command::new(bin)
        .arg("export")
        .arg("slides.typ")
        .arg("--format")
        .arg("svg")
        .arg("-o")
        .arg(&out_svgs)
        .current_dir(dir.path())
        .output()
        .expect("Execute cargo-slide export svg");
    assert!(svg_res.status.success(), "SVG export failed: {:?}", svg_res);
    assert!(out_svgs.exists());
    assert!(out_svgs.join("page-1.svg").exists());
}

#[test]
fn test_cli_build_standalone_binary() {
    let bin = env!("CARGO_BIN_EXE_cargo-slide");
    let dir = tempdir().expect("tempdir");

    // Initialize presentation
    let init_res = Command::new(bin)
        .arg("init")
        .current_dir(dir.path())
        .output()
        .expect("Execute cargo-slide init");
    assert!(init_res.status.success());

    let out_bin = dir.path().join("standalone_app");
    let build_res = Command::new(bin)
        .arg("build")
        .arg("slides.typ")
        .arg("-o")
        .arg(&out_bin)
        .current_dir(dir.path())
        .output()
        .expect("Execute cargo-slide build");

    let stdout = String::from_utf8_lossy(&build_res.stdout);
    let stderr = String::from_utf8_lossy(&build_res.stderr);
    assert!(
        build_res.status.success(),
        "Standalone build failed! stdout:\n{}\nstderr:\n{}",
        stdout,
        stderr
    );
    assert!(
        out_bin.exists(),
        "Output binary must exist at {:?}",
        out_bin
    );
    assert!(out_bin.metadata().unwrap().len() > 1_000_000);
}
