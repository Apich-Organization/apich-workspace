use slide_core::compiler::SlideCompiler;
use slide_core::watcher::SlideWatcher;
use std::fs;
use std::time::Duration;
use tempfile::tempdir;

#[test]
fn test_hot_reload_lifecycle_with_compiler() {
    let dir = tempdir().expect("Failed to create tempdir");
    let slide_file = dir.path().join("slides.typ");

    // 1. Initial slide deck with 1 slide
    fs::write(
        &slide_file,
        "#set page(paper: \"presentation-16-9\")\n= Slide 1\nHello World\n",
    )
    .expect("Failed to write initial slide");

    let compiler = SlideCompiler::new().expect("Typst compiler should be available");
    let initial_deck = compiler
        .compile_file(&slide_file)
        .expect("Initial deck compilation failed");
    assert_eq!(initial_deck.total_slides(), 1);

    // 2. Start watcher on slide_file
    let mut watcher = SlideWatcher::with_debounce(&slide_file, Duration::from_millis(50))
        .expect("Watcher initialization failed");
    std::thread::sleep(Duration::from_millis(100));

    // 3. Update slide file to add a second slide
    fs::write(
        &slide_file,
        "#set page(paper: \"presentation-16-9\")\n= Slide 1\n#pagebreak()\n= Slide 2\nNew slide added!\n",
    )
    .expect("Failed to write updated slide");

    let changed = watcher.wait_for_change(Duration::from_secs(3));
    assert!(changed.is_some(), "Watcher must detect slide file update");

    let reloaded_deck = compiler
        .compile_file(&slide_file)
        .expect("Recompilation should succeed");
    assert_eq!(
        reloaded_deck.total_slides(),
        2,
        "Deck should now have 2 slides after hot reload"
    );

    // 4. Test error resilience: write invalid Typst syntax
    fs::write(&slide_file, "= Broken Slide\n#{ let x = {{{{ ;;\n")
        .expect("Failed to write syntax error");

    let changed_err = watcher.wait_for_change(Duration::from_secs(3));
    assert!(
        changed_err.is_some(),
        "Watcher must detect change even when syntax is invalid"
    );

    let err_result = compiler.compile_file(&slide_file);
    assert!(
        err_result.is_err(),
        "Compiler should report syntax error without crashing"
    );

    // 5. Test recovery: fix syntax and add a third slide
    fs::write(
        &slide_file,
        "#set page(paper: \"presentation-16-9\")\n= Slide 1\n#pagebreak()\n= Slide 2\n#pagebreak()\n= Slide 3\nAll recovered!\n",
    )
    .expect("Failed to write recovered slides");

    let changed_recovery = watcher.wait_for_change(Duration::from_secs(3));
    assert!(
        changed_recovery.is_some(),
        "Watcher must detect recovery save"
    );

    let recovered_deck = compiler
        .compile_file(&slide_file)
        .expect("Compiler should cleanly recompile after error fixed");
    assert_eq!(
        recovered_deck.total_slides(),
        3,
        "Deck should now have 3 slides after recovery"
    );
}

#[test]
fn test_hot_reload_detects_imported_asset_changes() {
    let dir = tempdir().expect("Failed to create tempdir");
    let template_file = dir.path().join("template.typ");
    let slide_file = dir.path().join("slides.typ");

    fs::write(&template_file, "#let brand-color = rgb(\"102030\")\n").unwrap();
    fs::write(
        &slide_file,
        "#import \"template.typ\": brand-color\n#set page(fill: brand-color)\n= Branded Slide\n",
    )
    .unwrap();

    let compiler = SlideCompiler::new().unwrap();
    let deck1 = compiler.compile_file(&slide_file).unwrap();
    assert_eq!(deck1.total_slides(), 1);

    let mut watcher = SlideWatcher::with_debounce(&slide_file, Duration::from_millis(50)).unwrap();
    std::thread::sleep(Duration::from_millis(100));

    // Modify imported template.typ instead of slides.typ directly
    fs::write(&template_file, "#let brand-color = rgb(\"FF0000\")\n").unwrap();

    let changed = watcher.wait_for_change(Duration::from_secs(3));
    assert!(
        changed.is_some(),
        "Watcher must detect changes to imported template.typ in project directory"
    );

    let deck2 = compiler.compile_file(&slide_file).unwrap();
    assert_eq!(deck2.total_slides(), 1);
}

#[test]
fn test_events_during_compilation() {
    let dir = tempdir().unwrap();
    let slide_file = dir.path().join("slides.typ");
    std::fs::write(&slide_file, "= Slide 1\n").unwrap();
    let compiler = SlideCompiler::new().unwrap();
    let mut watcher = SlideWatcher::with_debounce(&slide_file, Duration::from_millis(50)).unwrap();
    std::thread::sleep(Duration::from_millis(100));

    // Drain initial
    watcher.drain();

    // Now compile!
    let _ = compiler.compile_file(&slide_file);

    // Sleep a bit and check if watcher sees any change
    std::thread::sleep(Duration::from_millis(200));
    let changed = watcher.poll_change();
    assert!(
        changed.is_none(),
        "Compiling the file should NOT trigger a file change event! But got: {:?}",
        changed
    );
}

#[test]
fn test_hot_reload_does_not_loop_on_charts() {
    let dir = tempdir().unwrap();
    let slide_file = dir.path().join("slides.typ");
    let csv_file = dir.path().join("data.csv");

    // Write CSV data
    std::fs::write(&csv_file, "Category,Value\nA,10\nB,20\nC,30\n").unwrap();

    // Write slides.typ referencing chart
    std::fs::write(
        &slide_file,
        "#set page(paper: \"presentation-16-9\")\n= Chart Slide\n#image(\"data.csv.cache.csv\")\n",
    )
    .unwrap();

    let compiler = SlideCompiler::new().unwrap();

    let mut watcher = SlideWatcher::with_debounce(&slide_file, Duration::from_millis(50)).unwrap();
    std::thread::sleep(Duration::from_millis(100));
    watcher.drain();

    // Edit slides.typ once
    std::fs::write(
        &slide_file,
        "#set page(paper: \"presentation-16-9\")\n= Updated Chart Slide\n",
    )
    .unwrap();

    // Watcher should detect the user save
    let changed = watcher.wait_for_change(Duration::from_secs(3));
    assert!(
        changed.is_some(),
        "Watcher must detect user save: {:?}",
        changed
    );

    // Simulate window.rs: compile and then drain
    let _ = compiler.compile_file(&slide_file);
    watcher.drain();

    // Wait longer than debounce duration (200ms > 50ms)
    std::thread::sleep(Duration::from_millis(200));

    // Assert that NO further reload event was generated
    let stray_event = watcher.poll_change();
    assert!(
        stray_event.is_none(),
        "Hot reload must NOT loop after first reload! But received stray event: {:?}",
        stray_event
    );
}
