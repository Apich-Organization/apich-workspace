//! Slide file and project directory watcher for live hot reloading.

use crate::error::Result;
use notify::Event;
use notify::EventKind;
use notify::RecommendedWatcher;
use notify::RecursiveMode;
use notify::Watcher;
use std::path::Path;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::channel;
use std::time::Duration;
use std::time::Instant;

/// Watches slides and related presentation assets for file changes to enable live hot reloading.
pub struct SlideWatcher {
    _watcher: RecommendedWatcher,
    rx: Receiver<notify::Result<Event>>,
    watch_dir: PathBuf,
    target_file: Option<PathBuf>,
    pending_change: Option<Instant>,
    last_changed_path: Option<PathBuf>,
    debounce_duration: Duration,
}

impl SlideWatcher {
    /// Create a new `SlideWatcher` watching the directory containing `target_file` (or `target_file` itself).
    /// Default debounce duration is 150ms.
    pub fn new(target_file: impl AsRef<Path>) -> Result<Self> {
        Self::with_debounce(target_file, Duration::from_millis(150))
    }

    /// Create a new `SlideWatcher` with custom debounce duration.
    pub fn with_debounce(
        target_file: impl AsRef<Path>,
        debounce_duration: Duration,
    ) -> Result<Self> {
        let input_path = target_file.as_ref();
        let abs_path = if input_path.is_absolute() {
            input_path.to_path_buf()
        } else {
            std::env::current_dir()
                .map(|cwd| cwd.join(input_path))
                .unwrap_or_else(|_| input_path.to_path_buf())
        };

        let file_path = abs_path.canonicalize().unwrap_or(abs_path);
        let watch_dir = if file_path.is_dir() {
            file_path.clone()
        } else {
            file_path
                .parent()
                .map(std::path::Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."))
        };
        let watch_dir = watch_dir.canonicalize().unwrap_or(watch_dir);

        let (tx, rx) = channel();
        let mut watcher = notify::recommended_watcher(move |res| {
            let _ = tx.send(res);
        })?;

        watcher.watch(&watch_dir, RecursiveMode::Recursive)?;

        Ok(Self {
            _watcher: watcher,
            rx,
            watch_dir,
            target_file: Some(file_path),
            pending_change: None,
            last_changed_path: None,
            debounce_duration,
        })
    }

    /// Returns the root directory being watched.
    #[must_use]
    pub fn watch_dir(&self) -> &Path {
        &self.watch_dir
    }

    /// Non-blocking check for debounced file changes.
    ///
    /// Call this inside an event loop (e.g. 60 FPS frame update).
    /// Returns `Some(PathBuf)` of the changed file if a relevant file modification was settled.
    pub fn poll_change(&mut self) -> Option<PathBuf> {
        // Drain all pending filesystem events from the channel
        while let Ok(res) = self.rx.try_recv() {
            if let Ok(ref event) = res
                && Self::is_relevant_event(
                    event,
                    self.target_file.as_deref(),
                    Some(&self.watch_dir),
                )
            {
                self.pending_change = Some(Instant::now());
                if let Some(first_path) = event.paths.first() {
                    self.last_changed_path = Some(first_path.clone());
                }
            }
        }

        // Check if debounce window has settled
        if let Some(since) = self.pending_change
            && since.elapsed() >= self.debounce_duration
        {
            self.pending_change = None;
            let path = self
                .last_changed_path
                .take()
                .or_else(|| self.target_file.clone());
            return path;
        }

        None
    }

    /// Blocking wait for a debounced file change up to the given timeout.
    /// Useful for headless tests or CLI wait loops.
    pub fn wait_for_change(
        &mut self,
        timeout: Duration,
    ) -> Option<PathBuf> {
        let start = Instant::now();
        while start.elapsed() < timeout {
            if let Some(path) = self.poll_change() {
                return Some(path);
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        None
    }

    /// Drain all queued events and reset any pending debounce timer.
    ///
    /// Call this after compilation finishes so that events generated during compilation
    /// (e.g. file reads, OS timestamp updates, compiler caches) do not trigger a subsequent redundant reload.
    pub fn drain(&mut self) {
        while self.rx.try_recv().is_ok() {}
        self.pending_change = None;
        self.last_changed_path = None;
    }

    /// Determine if an event from `notify` represents a relevant file modification.
    #[must_use]
    pub fn is_relevant_event(
        event: &Event,
        target_file: Option<&Path>,
        watch_dir: Option<&Path>,
    ) -> bool {
        // Only react to events that indicate actual data modifications, creation, or deletion.
        // Explicitly ignore pure read/access events and metadata changes (atime, chmod, xattr)
        // which can be triggered when Typst or media engines read presentation files.
        match event.kind {
            | EventKind::Modify(notify::event::ModifyKind::Data(_))
            | EventKind::Modify(notify::event::ModifyKind::Name(_))
            | EventKind::Modify(notify::event::ModifyKind::Any)
            | EventKind::Create(_)
            | EventKind::Remove(_)
            | EventKind::Access(notify::event::AccessKind::Close(
                notify::event::AccessMode::Write,
            ))
            | EventKind::Any => {},
            | EventKind::Modify(notify::event::ModifyKind::Metadata(_))
            | EventKind::Modify(notify::event::ModifyKind::Other)
            | EventKind::Access(_)
            | EventKind::Other => return false,
        }

        // Check if any path in the event is a relevant presentation source/asset
        event
            .paths
            .iter()
            .any(|p| Self::is_relevant_path(p, target_file, watch_dir))
    }

    /// Determine whether a specific file path should trigger a presentation reload.
    #[must_use]
    pub fn is_relevant_path(
        path: &Path,
        target_file: Option<&Path>,
        watch_dir: Option<&Path>,
    ) -> bool {
        // 1. Relativize path against watch_dir to inspect sub-components
        let rel_path = if let Some(w_dir) = watch_dir
            && let Ok(stripped) = path.strip_prefix(w_dir)
        {
            stripped
        } else {
            path
        };

        // Ignore hidden directories and files relative to watch_dir (.git, .build_tmp, .cargo, .vscode, etc.)
        for component in rel_path.components() {
            let comp_str = component.as_os_str().to_string_lossy();
            if comp_str.starts_with('.') && comp_str != "." && comp_str != ".." {
                return false;
            }
            if comp_str == "target" || comp_str == "node_modules" {
                return false;
            }
        }

        // Get file name string
        let file_name = match path.file_name().and_then(|f| f.to_str()) {
            | Some(n) => n,
            | None => return false,
        };

        // Ignore hidden files
        if file_name.starts_with('.') {
            return false;
        }

        // Ignore compiler cache files (*.cache.csv, *.cache.json, *.cache, *.db.cache.csv)
        // to prevent compiler-generated cache writes from creating infinite recompile loops
        if file_name.contains(".cache") {
            return false;
        }

        // Ignore interactive chart data exports and individual slide SVG renders
        if file_name == "chart_export.csv"
            || (file_name.starts_with("slide-") && file_name.ends_with(".svg"))
        {
            return false;
        }

        // Ignore editor temporary, swap, and backup files
        if file_name.starts_with(".#")
            || file_name.starts_with('~')
            || file_name.ends_with('~')
            || (file_name.starts_with('#') && file_name.ends_with('#'))
            || file_name.ends_with(".swp")
            || file_name.ends_with(".swo")
            || file_name.ends_with(".swn")
            || file_name.ends_with(".tmp")
            || file_name.ends_with(".temp")
            || file_name.ends_with(".bak")
            || file_name.ends_with(".kate-swp")
            || file_name.ends_with(".crdownload")
            || file_name.ends_with(".part")
            || file_name.contains("___jb_tmp___")
            || file_name.contains("___jb_old___")
            || file_name == "4913"
        {
            return false;
        }

        // Direct or filename match to target slide file
        if let Some(target) = target_file {
            if path == target {
                return true;
            }
            if let Some(tn) = target.file_name().and_then(|t| t.to_str())
                && file_name == tn
            {
                return true;
            }
        }

        // Ignore generated export PDFs to prevent infinite recompile loop
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let ext_lower = ext.to_ascii_lowercase();
            if ext_lower == "pdf" {
                return false;
            }

            // Allowed extensions for presentation assets and sources
            matches!(
                ext_lower.as_str(),
                "typ"
                    | "typst"
                    | "csv"
                    | "tsv"
                    | "json"
                    | "jsonl"
                    | "sql"
                    | "sqlite"
                    | "db"
                    | "png"
                    | "jpg"
                    | "jpeg"
                    | "webp"
                    | "svg"
                    | "gif"
                    | "toml"
                    | "yaml"
                    | "yml"
                    | "mp3"
                    | "wav"
                    | "ogg"
                    | "mp4"
                    | "webm"
            )
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_relevant_path_filtering() {
        assert!(SlideWatcher::is_relevant_path(
            Path::new("slides.typ"),
            None,
            None
        ));
        assert!(SlideWatcher::is_relevant_path(
            Path::new("template.typ"),
            None,
            None
        ));
        assert!(SlideWatcher::is_relevant_path(
            Path::new("data/chart.csv"),
            None,
            None
        ));
        assert!(SlideWatcher::is_relevant_path(
            Path::new("images/logo.png"),
            None,
            None
        ));
        assert!(SlideWatcher::is_relevant_path(
            Path::new("config.toml"),
            None,
            None
        ));

        // Hidden files
        assert!(!SlideWatcher::is_relevant_path(
            Path::new(".git/index"),
            None,
            None
        ));
        assert!(!SlideWatcher::is_relevant_path(
            Path::new(".build_tmp/cache.typ"),
            None,
            None
        ));

        // Editor temp files
        assert!(!SlideWatcher::is_relevant_path(
            Path::new("slides.typ~"),
            None,
            None
        ));
        assert!(!SlideWatcher::is_relevant_path(
            Path::new("slides.typ.swp"),
            None,
            None
        ));
        assert!(!SlideWatcher::is_relevant_path(
            Path::new(".#slides.typ"),
            None,
            None
        ));
        assert!(!SlideWatcher::is_relevant_path(
            Path::new("#slides.typ#"),
            None,
            None
        ));
        assert!(!SlideWatcher::is_relevant_path(
            Path::new("temp.tmp"),
            None,
            None
        ));
        assert!(!SlideWatcher::is_relevant_path(
            Path::new("slides.typ___jb_tmp___"),
            None,
            None
        ));
        assert!(!SlideWatcher::is_relevant_path(
            Path::new("4913"),
            None,
            None
        ));

        // Compiler cache files - MUST be rejected to avoid infinite loop
        assert!(!SlideWatcher::is_relevant_path(
            Path::new("assets/benchmarks.csv.cache.csv"),
            None,
            None
        ));
        assert!(!SlideWatcher::is_relevant_path(
            Path::new("assets/telemetry.db.cache.csv"),
            None,
            None
        ));
        assert!(!SlideWatcher::is_relevant_path(
            Path::new("assets/metrics.json.cache.csv"),
            None,
            None
        ));
        assert!(!SlideWatcher::is_relevant_path(
            Path::new("chart_export.csv"),
            None,
            None
        ));

        // Target directory and node_modules
        assert!(!SlideWatcher::is_relevant_path(
            Path::new("target/debug/build.typ"),
            None,
            None
        ));
        assert!(!SlideWatcher::is_relevant_path(
            Path::new("node_modules/pkg/index.typ"),
            None,
            None
        ));

        // Exported PDF and slide SVGs
        assert!(!SlideWatcher::is_relevant_path(
            Path::new("slides.pdf"),
            None,
            None
        ));
        assert!(!SlideWatcher::is_relevant_path(
            Path::new("slide-1.svg"),
            None,
            None
        ));
    }

    #[test]
    fn test_relevant_event_filtering() {
        use notify::event::*;

        let path = PathBuf::from("slides.typ");

        // Data modification -> relevant
        let data_event = Event {
            kind: EventKind::Modify(ModifyKind::Data(DataChange::Content)),
            paths: vec![path.clone()],
            attrs: Default::default(),
        };
        assert!(SlideWatcher::is_relevant_event(
            &data_event,
            Some(&path),
            None
        ));

        // Rename -> relevant (atomic save)
        let rename_event = Event {
            kind: EventKind::Modify(ModifyKind::Name(RenameMode::To)),
            paths: vec![path.clone()],
            attrs: Default::default(),
        };
        assert!(SlideWatcher::is_relevant_event(
            &rename_event,
            Some(&path),
            None
        ));

        // Metadata change (e.g. atime update during read) -> NOT relevant
        let meta_event = Event {
            kind: EventKind::Modify(ModifyKind::Metadata(MetadataKind::AccessTime)),
            paths: vec![path.clone()],
            attrs: Default::default(),
        };
        assert!(!SlideWatcher::is_relevant_event(
            &meta_event,
            Some(&path),
            None
        ));

        // Pure read access -> NOT relevant
        let access_event = Event {
            kind: EventKind::Access(AccessKind::Read),
            paths: vec![path.clone()],
            attrs: Default::default(),
        };
        assert!(!SlideWatcher::is_relevant_event(
            &access_event,
            Some(&path),
            None
        ));

        // Event for cache file -> NOT relevant
        let cache_event = Event {
            kind: EventKind::Modify(ModifyKind::Data(DataChange::Content)),
            paths: vec![PathBuf::from("assets/benchmarks.csv.cache.csv")],
            attrs: Default::default(),
        };
        assert!(!SlideWatcher::is_relevant_event(
            &cache_event,
            Some(&path),
            None
        ));
    }

    #[test]
    fn test_watcher_drain() {
        let dir = tempdir().expect("Failed to create tempdir");
        let slide_path = dir.path().join("slides.typ");
        std::fs::write(&slide_path, "= Slide 1").unwrap();

        let mut watcher = SlideWatcher::with_debounce(&slide_path, Duration::from_millis(50))
            .expect("Failed to start watcher");
        std::thread::sleep(Duration::from_millis(100));

        // Write to slide
        std::fs::write(&slide_path, "= Slide 2").unwrap();
        std::thread::sleep(Duration::from_millis(10));

        // Drain should clear everything
        watcher.drain();
        assert!(watcher.poll_change().is_none());
        assert!(
            watcher
                .wait_for_change(Duration::from_millis(100))
                .is_none()
        );
    }

    #[test]
    fn test_watcher_detects_file_modification() {
        let dir = tempdir().expect("Failed to create tempdir");
        let slide_path = dir.path().join("slides.typ");
        {
            let mut f = File::create(&slide_path).expect("Failed to create slide file");
            writeln!(f, "= Slide 1").expect("Failed to write initial content");
        }

        let mut watcher = SlideWatcher::with_debounce(&slide_path, Duration::from_millis(50))
            .expect("Failed to start watcher");

        // Small wait to ensure watcher is registered
        std::thread::sleep(Duration::from_millis(100));

        // Initially no change
        assert!(watcher.poll_change().is_none());

        // Modify file
        {
            let mut f = File::create(&slide_path).expect("Failed to append to slide file");
            writeln!(f, "= Slide 2").expect("Failed to write update");
        }

        // Wait for debounced change
        let changed = watcher.wait_for_change(Duration::from_secs(3));
        assert!(
            changed.is_some(),
            "Watcher should have detected slide file change"
        );
    }

    #[test]
    fn test_watcher_ignores_irrelevant_files() {
        let dir = tempdir().expect("Failed to create tempdir");
        let slide_path = dir.path().join("slides.typ");
        {
            let mut f = File::create(&slide_path).expect("Failed to create slide file");
            writeln!(f, "= Slide 1").expect("Failed to write initial content");
        }

        let mut watcher = SlideWatcher::with_debounce(&slide_path, Duration::from_millis(50))
            .expect("Failed to start watcher");

        std::thread::sleep(Duration::from_millis(100));

        // Modify irrelevant swap file
        let swap_path = dir.path().join("slides.typ.swp");
        {
            let mut f = File::create(&swap_path).expect("Failed to create swap file");
            writeln!(f, "swap data").expect("Failed to write swap data");
        }

        // Wait a bit
        let changed = watcher.wait_for_change(Duration::from_millis(200));
        assert!(
            changed.is_none(),
            "Watcher should ignore .swp file modifications"
        );
    }

    #[test]
    fn test_watcher_with_relative_path() {
        let dir = tempdir().expect("Failed to create tempdir");
        let orig_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        std::fs::write("slides.typ", "= Slide 1").unwrap();
        let mut watcher =
            SlideWatcher::with_debounce(Path::new("slides.typ"), Duration::from_millis(50))
                .expect("Failed to start watcher");
        std::thread::sleep(Duration::from_millis(100));
        std::fs::write("slides.typ", "= Slide 2").unwrap();
        let changed = watcher.wait_for_change(Duration::from_secs(3));
        std::env::set_current_dir(orig_cwd).unwrap();
        assert!(
            changed.is_some(),
            "Relative path watcher should detect change: {:?}",
            changed
        );
    }

    #[test]
    fn test_watcher_detects_atomic_rename() {
        let dir = tempdir().expect("Failed to create tempdir");
        let slide_path = dir.path().join("slides.typ");
        std::fs::write(&slide_path, "= Slide 1").unwrap();

        let mut watcher = SlideWatcher::with_debounce(&slide_path, Duration::from_millis(50))
            .expect("Failed to start watcher");
        std::thread::sleep(Duration::from_millis(100));

        // Simulate atomic save (write to temp file then rename)
        let tmp_path = dir.path().join("slides.typ.tmp");
        std::fs::write(&tmp_path, "= Slide 2 Updated via atomic rename").unwrap();
        std::fs::rename(&tmp_path, &slide_path).unwrap();

        let changed = watcher.wait_for_change(Duration::from_secs(3));
        assert!(changed.is_some(), "Atomic rename should trigger watcher");
    }
}
