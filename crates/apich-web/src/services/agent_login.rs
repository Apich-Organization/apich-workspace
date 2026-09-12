//! Tracks in-flight CLI agent account-login sessions.
//!
//! Sessions (`claude auth login`, `codex login --device-auth`, ...) can be polled
//! and have pasted-back codes submitted across several separate HTTP requests.
//!
//! This is real account login, not another shape of API-key entry: once a session in here
//! succeeds, the agent's credentials live in the container's home directory and subsequent
//! `AgentToolchain::run` calls need no `api_key` at all.

use apich_sandbox::tools::AgentKind;
use apich_sandbox::tools::LoginSupport;
use apich_sandbox::InteractiveExec;
use apich_sandbox::OutputChunk;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentLoginStatus {
    Running,
    Succeeded,
    Failed { exit_code: i32 },
}

pub struct AgentLoginSession {
    pub agent: AgentKind,
    pub login_support: LoginSupport,
    pub owner_user_id: Uuid,
    pub project_id: Uuid,
    output: Mutex<Vec<u8>>,
    status: Mutex<AgentLoginStatus>,
    stdin_tx: Mutex<Option<mpsc::Sender<Vec<u8>>>>,
}

impl AgentLoginSession {
    pub async fn snapshot(&self) -> (String, AgentLoginStatus) {
        let output = self.output.lock().await;
        let status = self.status.lock().await;
        (String::from_utf8_lossy(&output).to_string(), status.clone())
    }
}

#[derive(Default, Clone)]
pub struct AgentLoginRegistry {
    sessions: Arc<Mutex<HashMap<Uuid, Arc<AgentLoginSession>>>>,
}

impl AgentLoginRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts the agent's real login flow inside the given interactive exec session, registers
    /// it, and spawns a background task pumping its output/exit status into the record so
    /// separate poll/submit-code requests can observe progress. Returns the new session id.
    pub async fn start(
        &self,
        agent: AgentKind,
        owner_user_id: Uuid,
        project_id: Uuid,
        exec: InteractiveExec,
    ) -> Uuid {
        let id = Uuid::new_v4();
        let session = Arc::new(AgentLoginSession {
            agent,
            login_support: agent.login_support(),
            owner_user_id,
            project_id,
            output: Mutex::new(Vec::new()),
            status: Mutex::new(AgentLoginStatus::Running),
            stdin_tx: Mutex::new(Some(exec.stdin_tx)),
        });

        self.sessions.lock().await.insert(id, session.clone());

        let mut stream = exec.stream;
        let registry = self.clone();
        tokio::spawn(async move {
            loop {
                match stream.next_chunk().await {
                    | Some(OutputChunk::Stdout(bytes) | OutputChunk::Stderr(bytes)) => {
                        session.output.lock().await.extend_from_slice(&bytes);
                    },
                    | Some(OutputChunk::Exit(code)) => {
                        *session.status.lock().await = if code == 0 {
                            AgentLoginStatus::Succeeded
                        } else {
                            AgentLoginStatus::Failed { exit_code: code }
                        };
                        *session.stdin_tx.lock().await = None;
                        break;
                    },
                    | None => {
                        *session.stdin_tx.lock().await = None;
                        break;
                    },
                }
            }
            // Keep the finished record around briefly so a client mid-poll still sees the final
            // status, then drop it -- login sessions are low-volume but must not accumulate
            // forever in a long-running server.
            tokio::time::sleep(std::time::Duration::from_mins(5)).await;
            registry.sessions.lock().await.remove(&id);
        });

        id
    }

    pub async fn get(
        &self,
        id: Uuid,
    ) -> Option<Arc<AgentLoginSession>> {
        self.sessions.lock().await.get(&id).cloned()
    }

    /// Writes one line of pasted-back input (e.g. an OAuth code) to the session's stdin.
    /// Returns `Err` if the session has already finished or never accepted input in the first
    /// place (a `DeviceCode` flow like Codex's polls on its own and needs no pasted code).
    pub async fn submit_input(
        &self,
        id: Uuid,
        line: &str,
    ) -> Result<(), &'static str> {
        let session = self.get(id).await.ok_or("Login session not found")?;
        let tx = session
            .stdin_tx
            .lock()
            .await
            .as_ref()
            .cloned()
            .ok_or("This login session is no longer accepting input")?;
        let mut bytes = line.as_bytes().to_vec();
        bytes.push(b'\n');
        tx.send(bytes)
            .await
            .map_err(|_| "This login session is no longer accepting input")
    }
}
