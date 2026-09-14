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
    assert!(stdout.contains("cargo-slide"));

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

#[test]
fn test_cli_pack_slide_archive() {
    let bin = env!("CARGO_BIN_EXE_cargo-slide");
    let dir = tempdir().expect("tempdir");

    // Initialize presentation
    let init_res = Command::new(bin)
        .arg("init")
        .current_dir(dir.path())
        .output()
        .expect("Execute cargo-slide init");
    assert!(init_res.status.success());

    // Pack presentation to .slide (LZMA2 extreme)
    let out_slide = dir.path().join("presentation.slide");
    let pack_res = Command::new(bin)
        .arg("pack")
        .arg("slides.typ")
        .arg("-o")
        .arg(&out_slide)
        .current_dir(dir.path())
        .output()
        .expect("Execute cargo-slide pack");

    let stdout = String::from_utf8_lossy(&pack_res.stdout);
    let stderr = String::from_utf8_lossy(&pack_res.stderr);
    assert!(
        pack_res.status.success(),
        "Pack command failed! stdout:\n{}\nstderr:\n{}",
        stdout,
        stderr
    );
    assert!(out_slide.exists(), "Output .slide archive must exist");
    assert!(out_slide.metadata().unwrap().len() > 100);

    // Verify metadata and unpacking
    let meta = slide_core::package::read_package_metadata(&out_slide)
        .expect("Read metadata from packed .slide file");
    assert_eq!(meta.compression, "lzma2-max");
    assert!(meta.total_slides > 0);

    let deck = slide_core::package::unpack_deck_from_file(&out_slide)
        .expect("Unpack deck from packed .slide file");
    assert_eq!(deck.total_slides(), meta.total_slides);
}

#[test]
fn test_cli_build_wasm_csr_no_inline_js() {
    let bin = env!("CARGO_BIN_EXE_cargo-slide");
    let dir = tempdir().expect("tempdir");

    // Initialize presentation
    let init_res = Command::new(bin)
        .arg("init")
        .current_dir(dir.path())
        .output()
        .expect("Execute cargo-slide init");
    assert!(init_res.status.success());

    // Build WASM CSR bundle
    let out_web = dir.path().join("web_dist");
    let build_res = Command::new(bin)
        .arg("build")
        .arg("slides.typ")
        .arg("--format")
        .arg("wasm")
        .arg("-o")
        .arg(&out_web)
        .current_dir(dir.path())
        .output()
        .expect("Execute cargo-slide build --format wasm");

    let stdout = String::from_utf8_lossy(&build_res.stdout);
    let stderr = String::from_utf8_lossy(&build_res.stderr);
    assert!(
        build_res.status.success(),
        "WASM CSR build failed! stdout:\n{}\nstderr:\n{}",
        stdout,
        stderr
    );

    // Verify all required static CSR assets are generated
    assert!(out_web.join("index.html").exists());
    assert!(out_web.join("bootstrap.js").exists());
    assert!(out_web.join("style.css").exists());
    assert!(out_web.join("slide_web.js").exists());
    assert!(out_web.join("slide_web_bg.wasm").exists());
    assert!(out_web.join("deck.json").exists());

    // Strict requirement check: zero inline JavaScript in index.html!
    let index_html = std::fs::read_to_string(out_web.join("index.html")).unwrap();
    // Verify script tags ONLY reference external src
    assert!(index_html.contains("<script type=\"module\" src=\"./bootstrap.js\"></script>"));
    assert!(!index_html.contains("<script>"));
    assert!(!index_html.contains("javascript:"));
}

#[test]
fn test_cli_export_slide_and_wasm() {
    let bin = env!("CARGO_BIN_EXE_cargo-slide");
    let dir = tempdir().expect("tempdir");

    // Initialize presentation
    let init_res = Command::new(bin)
        .arg("init")
        .current_dir(dir.path())
        .output()
        .expect("Execute cargo-slide init");
    assert!(init_res.status.success());

    // Export .slide
    let export_slide = dir.path().join("exported.slide");
    let res1 = Command::new(bin)
        .arg("export")
        .arg("slides.typ")
        .arg("--format")
        .arg("slide")
        .arg("-o")
        .arg(&export_slide)
        .current_dir(dir.path())
        .output()
        .expect("Execute export slide");
    assert!(res1.status.success());
    assert!(export_slide.exists());

    // Export wasm
    let export_wasm_dir = dir.path().join("exported_wasm");
    let res2 = Command::new(bin)
        .arg("export")
        .arg("slides.typ")
        .arg("--format")
        .arg("wasm")
        .arg("-o")
        .arg(&export_wasm_dir)
        .current_dir(dir.path())
        .output()
        .expect("Execute export wasm");
    assert!(res2.status.success());
    assert!(export_wasm_dir.join("index.html").exists());
    assert!(export_wasm_dir.join("slide_web_bg.wasm").exists());
}

#[test]
fn test_cli_serve_web_server() {
    use std::io::Read;
    use std::io::Write;
    use std::net::TcpStream;

    let bin = env!("CARGO_BIN_EXE_cargo-slide");
    let dir = tempdir().expect("tempdir");

    // Initialize presentation
    let init_res = Command::new(bin)
        .arg("init")
        .current_dir(dir.path())
        .output()
        .expect("Execute cargo-slide init");
    assert!(init_res.status.success());

    let port = 19842;
    let mut child = Command::new(bin)
        .arg("serve")
        .arg("slides.typ")
        .arg("--ip")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(port.to_string())
        .current_dir(dir.path())
        .spawn()
        .expect("Spawn cargo-slide serve");

    // Wait for server to bind and start accepting connections
    let mut connected = false;
    for _ in 0..30 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if TcpStream::connect(format!("127.0.0.1:{port}")).is_ok() {
            connected = true;
            break;
        }
    }
    assert!(connected, "Server failed to bind to 127.0.0.1:{port}");

    // Test GET /
    let mut stream = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
    stream
        .write_all(b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
        .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    assert!(response.contains("200 OK"));
    assert!(response.contains("text/html"));
    assert!(response.contains("bootstrap.js"));

    // Test GET /deck.json
    let mut stream = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
    stream
        .write_all(b"GET /deck.json HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
        .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    assert!(response.contains("200 OK"));
    assert!(response.contains("application/json"));
    assert!(response.contains("\"slides\""));

    // Test GET /slide_web_bg.wasm
    let mut stream = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
    stream
        .write_all(
            b"GET /slide_web_bg.wasm HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
        )
        .unwrap();
    let mut header_buf = [0u8; 512];
    let n = stream.read(&mut header_buf).unwrap();
    let header_str = String::from_utf8_lossy(&header_buf[..n]);
    assert!(header_str.contains("200 OK"));
    assert!(header_str.contains("application/wasm"));

    // Clean up server process
    let _ = child.kill();
    let _ = child.wait();
}
