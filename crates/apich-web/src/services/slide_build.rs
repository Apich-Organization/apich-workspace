//! Tracks in-flight `cargo slide build --target <triple>` cross-compilation jobs so their real
//! progress can be polled and, once finished, the resulting binary downloaded -- across several
//! separate HTTP requests, since a cross-compiled build (especially the musl and Windows-arm64
//! targets, which compile a genuinely large dependency graph from scratch) can run for minutes,
//! far longer than a single request should block for. Mirrors `AgentLoginRegistry`'s own shape
//! (a registry of ids -> background tasks pumping a real `ExecStream` into a pollable record).
//!
//! Progress itself is real, not simulated: the patched `cargo-slide` binary this project vendors
//! (see `vendor/cargo-slide`) emits one JSON object per line when run with `--log-format json`,
//! including a `"percent"` field derived from actual `cargo build --message-format=json`
//! compiler-artifact events as they complete (see that crate's `run_cargo_build_with_progress`).
//! This module just parses that stream and stores the latest values.

use apich_sandbox::ExecStream;
use apich_sandbox::OutputChunk;
use apich_sandbox::UserContainer;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum SlideBuildStatus {
    Running { percent: u8, message: String },
    Done { filename: String, size_bytes: u64 },
    Failed { message: String },
}

pub struct SlideBuildJob {
    pub owner_user_id: Uuid,
    status: Mutex<SlideBuildStatus>,
    binary: Mutex<Option<Vec<u8>>>,
}

impl SlideBuildJob {
    pub async fn snapshot(&self) -> SlideBuildStatus {
        self.status.lock().await.clone()
    }
}

#[derive(Default, Clone)]
pub struct SlideBuildRegistry {
    jobs: Arc<Mutex<HashMap<Uuid, Arc<SlideBuildJob>>>>,
}

impl SlideBuildRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts tracking a build already running as `stream` inside `container`; spawns a
    /// background task that parses cargo-slide's `--log-format json` progress events out of the
    /// stream as they arrive, then -- once the process exits successfully -- reads the finished
    /// binary out of the container at `out_rel_path` (and deletes it there) before making it
    /// available via `binary()`. Returns the new job id immediately; the caller does not wait
    /// for the build itself.
    pub async fn start(
        &self,
        owner_user_id: Uuid,
        container: UserContainer,
        mut stream: ExecStream,
        out_rel_path: String,
        filename: String,
    ) -> Uuid {
        let id = Uuid::new_v4();
        let job = Arc::new(SlideBuildJob {
            owner_user_id,
            status: Mutex::new(SlideBuildStatus::Running {
                percent: 0,
                message: "Queued".to_string(),
            }),
            binary: Mutex::new(None),
        });
        self.jobs.lock().await.insert(id, job.clone());

        let registry = self.clone();
        tokio::spawn(async move {
            let mut buf: Vec<u8> = Vec::new();
            let mut exit_code = 0;
            let mut last_error: Option<String> = None;

            loop {
                match stream.next_chunk().await {
                    | Some(OutputChunk::Stdout(bytes) | OutputChunk::Stderr(bytes)) => {
                        buf.extend_from_slice(&bytes);
                        while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                            let line_bytes: Vec<u8> = buf.drain(..=pos).collect();
                            let line = String::from_utf8_lossy(&line_bytes);
                            let Ok(value) = serde_json::from_str::<serde_json::Value>(line.trim())
                            else {
                                continue;
                            };
                            let message = value
                                .get("message")
                                .and_then(|m| m.as_str())
                                .map(str::to_string);
                            let level = value
                                .get("level")
                                .and_then(|l| l.as_str())
                                .unwrap_or("info");
                            if let Some(percent) =
                                value.get("percent").and_then(serde_json::Value::as_u64)
                            {
                                *job.status.lock().await = SlideBuildStatus::Running {
                                    percent: percent.min(100) as u8,
                                    message: message.unwrap_or_else(|| "Building...".to_string()),
                                };
                            } else if level == "error" {
                                if let Some(msg) = message {
                                    last_error = Some(msg);
                                }
                            }
                        }
                    },
                    | Some(OutputChunk::Exit(code)) => {
                        exit_code = code;
                        break;
                    },
                    | None => break,
                }
            }

            if exit_code != 0 {
                let message = last_error
                    .unwrap_or_else(|| format!("Build process exited with code {exit_code}"));
                *job.status.lock().await = SlideBuildStatus::Failed { message };
                tokio::time::sleep(std::time::Duration::from_mins(10)).await;
                registry.jobs.lock().await.remove(&id);
                return;
            }

            match container.read_file(&out_rel_path).await {
                | Ok(bytes) => {
                    let size_bytes = bytes.len() as u64;
                    *job.binary.lock().await = Some(bytes);
                    *job.status.lock().await = SlideBuildStatus::Done { filename, size_bytes };
                },
                | Err(e) => {
                    *job.status.lock().await = SlideBuildStatus::Failed {
                        message: format!("Build reported success but no binary was found: {e}"),
                    };
                },
            }
            let _ = container.exec(["rm", "-f", &out_rel_path]).await;

            // Keep the finished record around briefly so a client mid-poll (or about to click
            // download) still sees it, then drop it -- these are low-volume but must not
            // accumulate forever in a long-running server.
            tokio::time::sleep(std::time::Duration::from_mins(10)).await;
            registry.jobs.lock().await.remove(&id);
        });

        id
    }

    pub async fn get(
        &self,
        id: Uuid,
    ) -> Option<Arc<SlideBuildJob>> {
        self.jobs.lock().await.get(&id).cloned()
    }

    /// The finished binary's bytes, if the job has reached `Done` -- does not remove the job
    /// record, so a status poll after downloading still reports `Done` rather than "not found".
    pub async fn binary(
        &self,
        id: Uuid,
    ) -> Option<Vec<u8>> {
        let job = self.get(id).await?;
        let bytes = job.binary.lock().await.clone();
        bytes
    }
}
