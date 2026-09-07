use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Autosave configuration
#[derive(Debug, Clone)]
pub struct AutosaveConfig {
    pub enabled: bool,
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
        let mut last = self.last_change.lock().await;
        *last = Some(Instant::now());
        self.has_pending.store(true, Ordering::SeqCst);
    }

    /// Check if debounced snapshot should be taken now
    pub async fn should_trigger_snapshot(&self) -> bool {
        if !self.config.enabled || !self.has_pending.load(Ordering::SeqCst) {
            return false;
        }

        let last = self.last_change.lock().await;
        if let Some(instant) = *last {
            if instant.elapsed() >= self.config.debounce_duration {
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
