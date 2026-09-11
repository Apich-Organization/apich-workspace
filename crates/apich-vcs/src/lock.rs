use crate::error::Result;
use crate::error::VcsError;
use rustix::fs::flock;
use rustix::fs::FlockOperation;
use std::collections::HashMap;
use std::fs::File;
use std::fs::OpenOptions;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::OnceLock;

struct LockEntry {
    file: File,
    count: usize,
}

static LOCK_REGISTRY: OnceLock<Mutex<HashMap<PathBuf, LockEntry>>> = OnceLock::new();

fn get_registry() -> &'static Mutex<HashMap<PathBuf, LockEntry>> {
    LOCK_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Cross-boundary advisory lock on `.apich/lock`.
/// Guarantees that in-container CLI processes and host gateway processes
/// never corrupt repository metadata during concurrent write operations.
/// Supports process-local re-entrancy so nested operations within the same process do not deadlock.
pub struct RepositoryLock {
    lock_path: PathBuf,
}

impl RepositoryLock {
    /// Acquire an exclusive lock on `.apich/lock`
    pub fn acquire(project_root: &Path) -> Result<Self> {
        let apich_dir = project_root.join(".apich");
        std::fs::create_dir_all(&apich_dir)?;

        let canonical_root = project_root
            .canonicalize()
            .unwrap_or_else(|_| project_root.to_path_buf());
        let mut lock_path = canonical_root.join(".apich").join("lock");

        if !lock_path.exists() {
            let _ = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(&lock_path);
        }
        if let Ok(canon) = lock_path.canonicalize() {
            lock_path = canon;
        }

        let registry = get_registry();
        let mut map = registry
            .lock()
            .map_err(|e| VcsError::Internal(format!("Failed to lock lock registry: {}", e)))?;

        if let Some(entry) = map.get_mut(&lock_path) {
            entry.count += 1;
            return Ok(Self { lock_path });
        }

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)?;

        flock(&file, FlockOperation::LockExclusive)
            .map_err(|e| VcsError::Internal(format!("Failed to acquire repository lock: {}", e)))?;

        map.insert(lock_path.clone(), LockEntry { file, count: 1 });

        Ok(Self { lock_path })
    }
}

impl Drop for RepositoryLock {
    fn drop(&mut self) {
        let registry = get_registry();
        if let Ok(mut map) = registry.lock() {
            if let Some(entry) = map.get_mut(&self.lock_path) {
                entry.count = entry.count.saturating_sub(1);
                if entry.count == 0 {
                    let _ = flock(&entry.file, FlockOperation::Unlock);
                    map.remove(&self.lock_path);
                }
            }
        }
    }
}
