//! Debounced autosave engine implementation.

use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::Mutex;

/// Autosave configuration
#[derive(Debug, Clone)]
pub struct AutosaveConfig {
    /// Whether continuous autosave is enabled.
    pub enabled: bool,
    /// Duration to wait after the last change before triggering an autosave snapshot.
    pub debounce_duration: Duration,
}

impl Default for AutosaveConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            debounce_duration: Duration::from_secs(2),
        }
    }
}

/// Thread-safe autosave debouncer
#[derive(Clone)]
pub struct AutosaveEngine {
    config: AutosaveConfig,
    last_change: Arc<Mutex<Option<Instant>>>,
    has_pending: Arc<AtomicBool>,
}

impl AutosaveEngine {
    /// Creates a new `AutosaveEngine` with the given configuration.
    #[must_use]
    pub fn new(config: AutosaveConfig) -> Self {
        Self {
            config,
            last_change: Arc::new(Mutex::new(None)),
            has_pending: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Record a change notification
    pub async fn notify_change(&self) {
        if !self.config.enabled {
            return;
        }
        {
            let mut last = self.last_change.lock().await;
            *last = Some(Instant::now());
        }
        self.has_pending.store(true, Ordering::SeqCst);
    }

    /// Check if debounced snapshot should be taken now
    pub async fn should_trigger_snapshot(&self) -> bool {
        if !self.config.enabled || !self.has_pending.load(Ordering::SeqCst) {
            return false;
        }

        let elapsed = {
            let last = self.last_change.lock().await;
            last.map(|instant| instant.elapsed())
        };
        if let Some(elapsed) = elapsed {
            if elapsed >= self.config.debounce_duration {
                return true;
            }
        }
        false
    }

    /// Reset pending change flag after snapshot has been committed
    pub fn reset_pending(&self) {
        self.has_pending.store(false, Ordering::SeqCst);
    }
}
