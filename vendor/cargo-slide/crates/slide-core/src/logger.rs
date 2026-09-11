use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::SystemTime;

static JSON_MODE: AtomicBool = AtomicBool::new(false);

/// Log output format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Human,
    Json,
}

impl LogFormat {
    #[must_use]
    pub const fn from_str(s: &str) -> Self {
        if s.eq_ignore_ascii_case("json") {
            Self::Json
        } else {
            Self::Human
        }
    }
}

/// Initialize the global logging format
pub fn init_logger(format: LogFormat) {
    JSON_MODE.store(format == LogFormat::Json, Ordering::SeqCst);
}

/// Check if currently in JSON logging mode
pub fn is_json_mode() -> bool {
    if let Ok(val) = std::env::var("CARGO_SLIDE_LOG_FORMAT")
        && val.eq_ignore_ascii_case("json")
    {
        return true;
    }
    JSON_MODE.load(Ordering::SeqCst)
}

/// Emit an info log
pub fn log_info(msg: &str) {
    if is_json_mode() {
        emit_json("info", msg, None);
    } else {
        println!("{msg}");
    }
}

/// Emit a success log
pub fn log_success(msg: &str) {
    if is_json_mode() {
        emit_json("success", msg, None);
    } else {
        println!("{msg}");
    }
}

/// Emit a warning log
pub fn log_warn(msg: &str) {
    if is_json_mode() {
        emit_json("warn", msg, None);
    } else {
        eprintln!("{msg}");
    }
}

/// Emit an error log
pub fn log_error(msg: &str) {
    if is_json_mode() {
        emit_json("error", msg, None);
    } else {
        eprintln!("{msg}");
    }
}

/// Emit a structured log event with optional JSON data
pub fn log_event(
    level: &str,
    msg: &str,
    data: Option<serde_json::Value>,
) {
    if is_json_mode() {
        emit_json(level, msg, data);
    } else if level == "error" || level == "warn" {
        eprintln!("{msg}");
    } else {
        println!("{msg}");
    }
}

fn emit_json(
    level: &str,
    msg: &str,
    data: Option<serde_json::Value>,
) {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64());

    let mut obj = serde_json::json!({
        "level": level,
        "timestamp": now,
        "message": msg,
    });

    if let Some(d) = data
        && let Some(map) = obj.as_object_mut()
    {
        if let Some(d_map) = d.as_object() {
            for (k, v) in d_map {
                map.insert(k.clone(), v.clone());
            }
        } else {
            map.insert("data".to_string(), d);
        }
    }

    if level == "error" || level == "warn" {
        eprintln!("{obj}");
    } else {
        println!("{obj}");
    }
}

#[macro_export]
macro_rules! slide_info {
    ($($arg:tt)*) => {
        $crate::logger::log_info(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! slide_success {
    ($($arg:tt)*) => {
        $crate::logger::log_success(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! slide_warn {
    ($($arg:tt)*) => {
        $crate::logger::log_warn(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! slide_error {
    ($($arg:tt)*) => {
        $crate::logger::log_error(&format!($($arg)*))
    };
}
