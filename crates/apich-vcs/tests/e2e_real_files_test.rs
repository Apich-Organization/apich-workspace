mod common;
use apich_vcs::BundleOptions;
use apich_vcs::LfsPolicy;
use apich_vcs::ProjectVcs;
use common::test_temp_dir;
use git2::Repository;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Helper to create a valid SQLite3 database with realistic tables and data
fn create_sqlite_db(
    path: &Path,
    initial_rows: usize,
) {
    let script = format!(
        "import sqlite3\n\
        conn = sqlite3.connect('{}')\n\
        c = conn.cursor()\n\
        c.execute('CREATE TABLE benchmarks (id INTEGER PRIMARY KEY, runtime_ms REAL, algorithm TEXT)')\n\
        for i in range({}):\n    \
            c.execute('INSERT INTO benchmarks (runtime_ms, algorithm) VALUES (?, ?)', (i * 1.25, f'algo_{{i % 4}}'))\n\
        conn.commit()\n\
        conn.close()\n",
        path.to_str().unwrap(),
        initial_rows
    );

    let output = Command::new("python3")
        .args(["-c", &script])
        .output()
        .expect("Failed to execute python sqlite3 script");
    assert!(output.status.success());
}

/// Helper to append new records to an existing SQLite database
fn append_sqlite_db(
    path: &Path,
    additional_rows: usize,
) {
    let script = format!(
        "import sqlite3\n\
        conn = sqlite3.connect('{}')\n\
        c = conn.cursor()\n\
        for i in range({}):\n    \
            c.execute('INSERT INTO benchmarks (runtime_ms, algorithm) VALUES (?, ?)', (100.0 + i * 0.5, f'algo_new_{{i}}'))\n\
        conn.commit()\n\
        conn.close()\n",
        path.to_str().unwrap(),
        additional_rows
    );

    let output = Command::new("python3")
        .args(["-c", &script])
        .output()
        .expect("Failed to execute python sqlite3 script");
    assert!(output.status.success());
}

/// Helper to query row count from an SQLite database
fn query_sqlite_count(path: &Path) -> usize {
    let script = format!(
        "import sqlite3\n\
        conn = sqlite3.connect('{}')\n\
        c = conn.cursor()\n\
        c.execute('SELECT COUNT(*) FROM benchmarks')\n\
        count = c.fetchone()[0]\n\
        conn.close()\n\
        print(count)\n",
        path.to_str().unwrap()
    );

    let output = Command::new("python3")
        .args(["-c", &script])
        .output()
        .expect("Failed to execute python sqlite3 script");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.trim().parse::<usize>().unwrap()
}

/// Helper to generate a genuine, valid 1x1 PNG binary file
fn sample_png_bytes() -> Vec<u8> {
    vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR header
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1x1 dimensions
        0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, // 8-bit RGBA
        0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, // IDAT header
        0x54, 0x78, 0x9C, 0x63, 0xF8, 0xCF, 0xC0, 0x00, // compressed pixel data
        0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB0, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
        0x4E, // IEND header
        0x44, 0xAE, 0x42, 0x60, 0x82,
    ]
}

#[test]
fn test_e2e_real_files_typst_sqlite_png_lifecycle() {
    let temp = test_temp_dir();
    let project_root = temp.path().join("quantum_paper");
    fs::create_dir_all(&project_root).unwrap();

    let vcs = ProjectVcs::open_or_init(&project_root).unwrap();

    // 1. Setup realistic technical paper structure
    fs::write(
        project_root.join("typst.toml"),
        "[package]\nname = \"quantum-optics\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();

    fs::write(
        project_root.join("main.typ"),
        "#set text(font: \"New Computer Modern\", size: 10pt)\n\
        #show heading: it => [ #it.body ]\n\n\
        = Scalable Fault-Tolerant Quantum Decoherence Simulation\n\
        #include \"chapters/01_introduction.typ\"\n\
        #include \"chapters/02_methods.typ\"\n",
    )
    .unwrap();

    fs::create_dir_all(project_root.join("chapters")).unwrap();
    fs::write(
        project_root.join("chapters/01_introduction.typ"),
        "= Introduction\nQuantum noise poses significant hurdles in NISQ hardware.\n",
    )
    .unwrap();
    fs::write(
        project_root.join("chapters/02_methods.typ"),
        "= Numerical Methods\nWe model the Lindblad master equation.\n",
    )
    .unwrap();

    // 2. Add realistic binary assets: PNG figure and SQLite database
    fs::create_dir_all(project_root.join("figures")).unwrap();
    let png_data = sample_png_bytes();
    fs::write(project_root.join("figures/qubit_decay.png"), &png_data).unwrap();

    fs::create_dir_all(project_root.join("data")).unwrap();
    let db_path = project_root.join("data/measurements.db");
    create_sqlite_db(&db_path, 80);
    assert_eq!(query_sqlite_count(&db_path), 80);

    // 3. Take initial snapshot (Zero-staging: no `git add` required)
    let s1 = vcs
        .snapshot("Initial manuscript draft with dataset and figures")
        .unwrap();
    assert_eq!(vcs.list_snapshots().unwrap().len(), 1);

    // Verify tree integrity
    let tree1 = vcs.cas().get_tree(&s1.tree_hash).unwrap();
    assert!(tree1.entries.contains_key("main.typ"));
    assert!(tree1.entries.contains_key("figures/qubit_decay.png"));
    assert!(tree1.entries.contains_key("data/measurements.db"));

    // 4. Perform continuous working copy modifications
    // Append text to chapters
    fs::write(
        project_root.join("chapters/02_methods.typ"),
        "= Numerical Methods\nWe model the Lindblad master equation.\nRunge-Kutta 4th order integrator applied.\n",
    )
    .unwrap();

    // Insert 40 more records into SQLite database
    append_sqlite_db(&db_path, 40);
    assert_eq!(query_sqlite_count(&db_path), 120);

    // Take second snapshot
    let s2 = vcs
        .snapshot("Added 40 measurement rows and specified RK4 integrator")
        .unwrap();
    assert_eq!(s2.parent_snapshot_id, Some(s1.id));

    // 5. Test Git Bridge compatibility with automatic LFS synthesis for SQLite database
    vcs.git_init().unwrap();
    let commit_oid = vcs
        .git_export_commit(
            "main",
            "Export to external Git repository",
            "Prof. Quantum",
            "quantum@lab.org",
        )
        .unwrap();

    let git_repo = Repository::open(&project_root).unwrap();
    let git_commit = git_repo
        .find_commit(git2::Oid::from_str(&commit_oid).unwrap())
        .unwrap();
    let git_tree = git_commit.tree().unwrap();

    // Verify .typst files were exported as normal Git blobs
    let main_typ_blob = git_repo
        .find_blob(git_tree.get_name("main.typ").unwrap().id())
        .unwrap();
    assert!(std::str::from_utf8(main_typ_blob.content())
        .unwrap()
        .contains("Scalable Fault-Tolerant"));

    // Verify .db file was exported as an automated Git LFS pointer
    let data_tree_oid = git_tree.get_name("data").unwrap().id();
    let data_tree = git_repo.find_tree(data_tree_oid).unwrap();
    let db_blob = git_repo
        .find_blob(data_tree.get_name("measurements.db").unwrap().id())
        .unwrap();

    let pointer_str = std::str::from_utf8(db_blob.content()).unwrap();
    assert!(pointer_str.starts_with("version https://git-lfs.github.com/spec/v1\n"));
    let (sha256, db_size) = LfsPolicy::parse_lfs_pointer(pointer_str).unwrap();
    assert!(!sha256.is_empty());
    assert_eq!(db_size, fs::metadata(&db_path).unwrap().len());
}

#[test]
fn test_e2e_weave_free_merge_and_conflict_resolution() {
    let temp = test_temp_dir();
    let project_root = temp.path().join("collaborative_project");
    fs::create_dir_all(&project_root).unwrap();

    let vcs = ProjectVcs::open_or_init(&project_root).unwrap();

    // Common ancestor
    fs::write(
        project_root.join("abstract.typ"),
        "= Abstract\nThis paper introduces an accelerated quantum kernel algorithm.\n",
    )
    .unwrap();
    fs::write(
        project_root.join("references.bib"),
        "@article{shor1994,\n  title = {Polynomial-Time Algorithms for Prime Factorization},\n  author = {Shor, Peter W.},\n  year = {1994}\n}\n",
    )
    .unwrap();
    let s_base = vcs.snapshot("Base submission").unwrap();

    // Collaborator Alice creates branch
    vcs.branch_create("collab-alice").unwrap();
    vcs.branch_switch("collab-alice").unwrap();

    // Alice modifies abstract and references
    fs::write(
        project_root.join("abstract.typ"),
        "= Abstract\nThis paper introduces an accelerated, provably convergent quantum kernel algorithm.\n",
    )
    .unwrap();
    fs::write(
        project_root.join("references.bib"),
        "@article{shor1994,\n  title = {Polynomial-Time Algorithms for Prime Factorization},\n  author = {Shor, Peter W.},\n  year = {1994}\n}\n\n@article{grover1996,\n  title = {A Fast Quantum Mechanical Algorithm for Database Search},\n  author = {Grover, Lov K.},\n  year = {1996}\n}\n",
    )
    .unwrap();
    vcs.snapshot("Alice: added provable convergence note and Grover citation")
        .unwrap();

    // Switch back to main
    vcs.branch_switch("main").unwrap();

    // Collaborator Bob modifies references independently
    fs::write(
        project_root.join("references.bib"),
        "@article{shor1994,\n  title = {Polynomial-Time Algorithms for Prime Factorization},\n  author = {Shor, Peter W.},\n  year = {1994}\n}\n\n@article{feynman1982,\n  title = {Simulating Physics with Computers},\n  author = {Feynman, Richard P.},\n  year = {1982}\n}\n",
    )
    .unwrap();
    let s_bob = vcs
        .snapshot("Bob: added Feynman 1982 foundational citation")
        .unwrap();

    // Reconcile and Merge (Weave-Free)
    let reconcile_res = vcs.merge("collab-alice").unwrap();

    // abstract.typ was only modified by Alice -> auto-accepted
    assert_eq!(
        fs::read_to_string(project_root.join("abstract.typ")).unwrap(),
        "= Abstract\nThis paper introduces an accelerated, provably convergent quantum kernel algorithm.\n"
    );

    // references.bib was modified concurrently on both branches -> non-blocking conflict markers
    assert_eq!(reconcile_res.conflicts.len(), 1);
    assert_eq!(reconcile_res.conflicts[0].path, "references.bib");

    let conflicted_bib = fs::read_to_string(project_root.join("references.bib")).unwrap();
    assert!(conflicted_bib.contains("<<<<<<< ours"));
    assert!(conflicted_bib.contains("feynman1982"));
    assert!(conflicted_bib.contains("======="));
    assert!(conflicted_bib.contains("grover1996"));
    assert!(conflicted_bib.contains(">>>>>>> theirs"));

    // User seamlessly resolves the conflict directly in the file
    let resolved_bib = "@article{shor1994,\n  title = {Polynomial-Time Algorithms for Prime Factorization},\n  author = {Shor, Peter W.},\n  year = {1994}\n}\n\n@article{feynman1982,\n  title = {Simulating Physics with Computers},\n  author = {Feynman, Richard P.},\n  year = {1982}\n}\n\n@article{grover1996,\n  title = {A Fast Quantum Mechanical Algorithm for Database Search},\n  author = {Grover, Lov K.},\n  year = {1996}\n}\n";
    fs::write(project_root.join("references.bib"), resolved_bib).unwrap();

    let s_resolved = vcs
        .snapshot("Resolved references.bib conflict by including both citations")
        .unwrap();
    assert_ne!(s_resolved.id, s_bob.id);
    assert_ne!(s_resolved.id, s_base.id);

    // Final clean state has both citations
    let final_bib = fs::read_to_string(project_root.join("references.bib")).unwrap();
    assert!(final_bib.contains("feynman1982"));
    assert!(final_bib.contains("grover1996"));
}

#[test]
fn test_e2e_gateway_bundle_and_local_work_restoration() {
    let server_temp = test_temp_dir();
    let server_root = server_temp.path().join("cloud_workspace");
    fs::create_dir_all(&server_root).unwrap();

    let vcs = ProjectVcs::open_or_init(&server_root).unwrap();

    // 1. Setup multi-asset project
    fs::write(server_root.join("doc.typ"), "= Document\n").unwrap();
    fs::create_dir_all(server_root.join("data")).unwrap();
    let db_path = server_root.join("data/local.db");
    create_sqlite_db(&db_path, 25);

    let png_path = server_root.join("data/plot.png");
    fs::write(&png_path, sample_png_bytes()).unwrap();

    vcs.snapshot("Version 1.0 in cloud").unwrap();
    vcs.create_milestone("release-1.0", "Cloud release")
        .unwrap();

    // 2. Gateway exports bundle for local work
    let bundle_path = server_temp.path().join("cloud_project.apich-bundle");
    vcs.export_bundle_to_file(&bundle_path, BundleOptions::default())
        .unwrap();

    // 3. User on local computer imports the bundle to start local work
    let local_temp = test_temp_dir();
    let local_root = local_temp.path().join("my_local_workspace");

    let local_vcs = ProjectVcs::import_bundle_from_file(&bundle_path, &local_root).unwrap();

    // Verify all files on local machine
    assert!(local_root.join("doc.typ").exists());
    assert!(local_root.join("data/local.db").exists());
    assert!(local_root.join("data/plot.png").exists());

    // Verify SQLite database on local machine is completely functional
    let local_db_path = local_root.join("data/local.db");
    assert_eq!(query_sqlite_count(&local_db_path), 25);

    // User does local work (e.g. offline research)
    append_sqlite_db(&local_db_path, 15);
    assert_eq!(query_sqlite_count(&local_db_path), 40);

    fs::write(
        local_root.join("doc.typ"),
        "= Document\nOffline local edits added successfully.\n",
    )
    .unwrap();

    // Local user creates new local snapshot
    let s_local = local_vcs.snapshot("Local offline changes").unwrap();
    assert!(s_local.parent_snapshot_id.is_some());
    assert_eq!(
        fs::read_to_string(local_root.join("doc.typ")).unwrap(),
        "= Document\nOffline local edits added successfully.\n"
    );

    // OpLog preserves undo functionality locally
    let undone = local_vcs.undo().unwrap();
    assert!(undone.is_some());
    assert_eq!(
        fs::read_to_string(local_root.join("doc.typ")).unwrap(),
        "= Document\n"
    );
}
