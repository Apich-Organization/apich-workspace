use crate::container::UserContainer;
use crate::error::Result;
use crate::exec::ExecOptions;
use crate::exec::ExecResult;
use crate::exec::ExecStream;
use crate::exec::InteractiveExec;
use crate::exec::OutputChunk;
use std::time::Duration;
use tokio::sync::mpsc;

/// A hung or slow-to-fail agent CLI (observed live: `claude --print` with an invalid API key
/// can hang well past 100s instead of failing fast) must not block the HTTP request that
/// triggered it indefinitely -- cap every agent invocation.
const AGENT_EXEC_TIMEOUT: Duration = Duration::from_secs(120);

/// A logged-in-via-account flow can sit waiting on a human to finish something in their own
/// browser (visit a URL, authorize, come back) -- OAuth codes are typically valid for up to
/// ~15 minutes (verified live: codex's device code says "expires in 15 minutes"), so the exec
/// itself must not be killed by the much shorter `AGENT_EXEC_TIMEOUT` used for normal agent runs.
const AGENT_LOGIN_TIMEOUT: Duration = Duration::from_secs(15 * 60);

/// How a given agent CLI's real account-login flow behaves, checked live against the actual
/// built image (`apich-sandbox:latest`), not assumed from documentation:
/// - `claude auth login` prints an OAuth URL, then blocks on stdin for the user to paste back a
///   code copied from the browser callback page.
/// - `codex login --device-auth` prints a URL + one-time code and polls automatically -- no
///   stdin needed at all, the process itself exits once the browser step completes.
/// - opencode's `auth login` is a full interactive TUI (arrow-key provider picker, not a single
///   linear prompt) and goose's `configure` is a similar wizard; aider has no account-login
///   concept, it's API-key-only. None of these get a fake "login" button here -- BYOK via
///   `AgentToolchain::run`'s `api_key` is still how they're used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginSupport {
    /// No real account-login flow this session can drive; use the BYOK `api_key` param instead.
    None,
    /// Prints a URL + code and polls on its own; just stream output until it exits.
    DeviceCode,
    /// Prints a URL, then needs one line of pasted-back input on stdin before it can finish.
    PasteCodeBack,
}

/// A CLI coding agent installed in the sandbox image (see `docker/Containerfile.sandbox`).
/// Per plan.md, these run directly inside the project's container against the real workspace
/// files -- this is not a hosted chat API call, it's a real agent process with a real shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentKind {
    ClaudeCode,
    Codex,
    OpenCode,
    Aider,
    Goose,
    /// Google's Antigravity CLI (`agy`), the successor to Gemini CLI. Previously left out of
    /// this list entirely -- plan.md names it, but no publicly verifiable install method could
    /// be found for it at the time. It's since been confirmed real and installed on the dev
    /// qube (`~/.local/bin/agy`, `agy --version` -> 1.1.27, real `--help`/flag behavior checked
    /// live rather than assumed -- see `non_interactive_args`'s doc comment for a real flag
    /// syntax mistake this caught before it shipped).
    Agy,
}

impl AgentKind {
    pub const ALL: [AgentKind; 6] = [
        AgentKind::ClaudeCode,
        AgentKind::Codex,
        AgentKind::OpenCode,
        AgentKind::Aider,
        AgentKind::Goose,
        AgentKind::Agy,
    ];

    /// The binary name on PATH inside the sandbox image, matching
    /// `apich-agent-status` in `docker/Containerfile.sandbox`.
    pub fn binary(&self) -> &'static str {
        match self {
            | AgentKind::ClaudeCode => "claude",
            | AgentKind::Codex => "codex",
            | AgentKind::OpenCode => "opencode",
            | AgentKind::Aider => "aider",
            | AgentKind::Goose => "goose",
            | AgentKind::Agy => "agy",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            | AgentKind::ClaudeCode => "Claude Code",
            | AgentKind::Codex => "Codex CLI",
            | AgentKind::OpenCode => "opencode",
            | AgentKind::Aider => "Aider",
            | AgentKind::Goose => "goose",
            | AgentKind::Agy => "Antigravity CLI",
        }
    }

    /// The env var this agent reads its BYOK API key from. The caller is responsible for
    /// actually holding the user's key (never persisted server-side) and passing it into
    /// `run`/`run_stream` -- this only tells the caller which env var name the CLI expects.
    pub fn credential_env_var(&self) -> &'static str {
        match self {
            | AgentKind::ClaudeCode => "ANTHROPIC_API_KEY",
            | AgentKind::Codex => "OPENAI_API_KEY",
            | AgentKind::OpenCode => "ANTHROPIC_API_KEY",
            | AgentKind::Aider => "OPENAI_API_KEY",
            | AgentKind::Goose => "ANTHROPIC_API_KEY",
            // Confirmed by a real string baked into the `agy` binary itself ("You are using the
            // Gemini API directly with GEMINI_API_KEY..."), not assumed from agy being framed as
            // a Gemini CLI successor.
            | AgentKind::Agy => "GEMINI_API_KEY",
        }
    }

    pub fn login_support(&self) -> LoginSupport {
        match self {
            | AgentKind::ClaudeCode => LoginSupport::PasteCodeBack,
            | AgentKind::Codex => LoginSupport::DeviceCode,
            // `agy --help` has no `login`/`auth` subcommand -- but running the bare `agy`
            // command with no stored credentials *does* trigger a real login flow of its own:
            // an interactive menu ("Select login method: 1. Google OAuth / 2. Use a Google
            // Cloud project"), then (after selecting OAuth) a real
            // `https://accounts.google.com/o/oauth2/auth?...` URL and a prompt to paste back
            // the authorization code -- confirmed live via a PTY-driven probe against the real
            // `agy` binary (temporarily moving its stored token aside to force the unauthenticated
            // path, then restoring it). Same PasteCodeBack shape as `claude auth login`; see
            // `AgentToolchain::login`'s Agy branch for the extra menu-selection keystroke this
            // one needs before it reaches that stage.
            | AgentKind::Agy => LoginSupport::PasteCodeBack,
            // opencode's `auth login` is a full interactive TUI (arrow-key provider picker, not
            // a single linear prompt) and goose's `configure` is a similar wizard; aider has no
            // account-login concept, it's API-key-only. None of these get a fake "login" button
            // here -- BYOK via `AgentToolchain::run`'s `api_key` is still how they're used.
            | AgentKind::OpenCode | AgentKind::Aider | AgentKind::Goose => LoginSupport::None,
        }
    }

    /// Real, verified invocation for each CLI's account-login flow (checked against the actual
    /// built image, see `LoginSupport`'s doc comment). `None` for kinds with no real login flow
    /// this session can drive.
    fn login_args(&self) -> Option<Vec<String>> {
        match self {
            | AgentKind::ClaudeCode => Some(vec!["auth".to_string(), "login".to_string()]),
            | AgentKind::Codex => Some(vec!["login".to_string(), "--device-auth".to_string()]),
            // No subcommand at all -- the bare binary itself is what shows the login menu.
            | AgentKind::Agy => Some(vec![]),
            | AgentKind::OpenCode | AgentKind::Aider | AgentKind::Goose => None,
        }
    }

    pub fn parse(name: &str) -> Option<AgentKind> {
        match name {
            | "claude" | "claude-code" => Some(AgentKind::ClaudeCode),
            | "codex" => Some(AgentKind::Codex),
            | "opencode" => Some(AgentKind::OpenCode),
            | "aider" => Some(AgentKind::Aider),
            | "goose" => Some(AgentKind::Goose),
            | "agy" | "antigravity" => Some(AgentKind::Agy),
            | _ => None,
        }
    }

    /// Real, verified non-interactive one-shot invocation for each CLI (checked against each
    /// tool's actual `--help`/`<subcommand> --help` output inside a built `apich-sandbox:latest`
    /// image, not guessed from memory -- see docker/Containerfile.sandbox).
    ///
    /// `agy`'s own `--help` describes `--print` as "Run a single prompt non-interactively and
    /// print the response", which reads like a boolean flag followed by a positional prompt --
    /// it isn't. Tried live: `agy --print --mode accept-edits ... "say hi"` fails with `--print
    /// took "--mode" as its prompt, so the intended prompt was left as an argument and ignored`
    /// -- `--print` (like its `--prompt` alias) takes the prompt as its own next token, so any
    /// flag placed directly after it gets swallowed as the prompt text instead. Fixed by putting
    /// `--print <prompt>` last; confirmed working afterward with a real (if throwaway) API key.
    fn non_interactive_args(
        &self,
        prompt: &str,
    ) -> Vec<String> {
        match self {
            | AgentKind::ClaudeCode => {
                vec![
                    "--print".to_string(),
                    "--permission-mode".to_string(),
                    "acceptEdits".to_string(),
                    "--output-format".to_string(),
                    "text".to_string(),
                    prompt.to_string(),
                ]
            },
            | AgentKind::Codex => {
                vec![
                    "exec".to_string(),
                    "--sandbox".to_string(),
                    "workspace-write".to_string(),
                    "--skip-git-repo-check".to_string(),
                    prompt.to_string(),
                ]
            },
            | AgentKind::OpenCode => {
                vec!["run".to_string(), prompt.to_string(), "--auto".to_string()]
            },
            | AgentKind::Aider => {
                vec![
                    "--yes-always".to_string(),
                    "--message".to_string(),
                    prompt.to_string(),
                ]
            },
            | AgentKind::Goose => {
                vec![
                    "run".to_string(),
                    "--text".to_string(),
                    prompt.to_string(),
                    "--no-session".to_string(),
                    "-q".to_string(),
                ]
            },
            | AgentKind::Agy => {
                vec![
                    "--mode".to_string(),
                    "accept-edits".to_string(),
                    "--output-format".to_string(),
                    "text".to_string(),
                    "--print".to_string(),
                    prompt.to_string(),
                ]
            },
        }
    }
}

pub struct AgentToolchain<'a> {
    container: &'a UserContainer,
}

impl<'a> AgentToolchain<'a> {
    pub fn new(container: &'a UserContainer) -> Self {
        Self { container }
    }

    /// Which agent CLIs actually made it into this container's image. Image builds without
    /// network access to a given tool's installer silently skip that one tool (see
    /// `docker/Containerfile.sandbox`'s per-tool `|| echo ... failed` guards), so this checks
    /// real `command -v` results via the image's own `apich-agent-status` script rather than
    /// assuming every kind is present.
    pub async fn availability(&self) -> Result<Vec<(AgentKind, bool)>> {
        let res = self.container.exec(&["apich-agent-status"]).await?;
        let out = res.stdout_lossy();
        Ok(AgentKind::ALL
            .into_iter()
            .map(|kind| {
                let available = out
                    .lines()
                    .any(|l| l.trim() == format!("{}: available", kind.binary()));
                (kind, available)
            })
            .collect())
    }

    fn build_options(
        &self,
        kind: AgentKind,
        prompt: &str,
        api_key: Option<&str>,
    ) -> ExecOptions {
        let mut cmd = vec![kind.binary().to_string()];
        cmd.extend(kind.non_interactive_args(prompt));
        let mut opts = ExecOptions::new(cmd).timeout(AGENT_EXEC_TIMEOUT);
        if let Some(key) = api_key {
            opts = opts.env(kind.credential_env_var(), key);
        }
        opts
    }

    /// Run the agent non-interactively against the container's real project workspace,
    /// buffering all output until the process exits. The user's BYOK API key, if given, is
    /// injected as an env var scoped to this single exec call only -- never written to disk
    /// inside the container, never logged.
    pub async fn run(
        &self,
        kind: AgentKind,
        prompt: &str,
        api_key: Option<&str>,
    ) -> Result<ExecResult> {
        let opts = self.build_options(kind, prompt, api_key);
        self.container.exec_with_options(opts).await
    }

    /// Same as `run`, but streams stdout/stderr chunks as the agent produces them instead of
    /// buffering until exit -- for live-updating UI (e.g. a terminal-style island) instead of a
    /// single blocking request.
    pub async fn run_stream(
        &self,
        kind: AgentKind,
        prompt: &str,
        api_key: Option<&str>,
    ) -> Result<ExecStream> {
        let opts = self.build_options(kind, prompt, api_key);
        self.container.exec_stream(opts).await
    }

    /// Start the agent's real account-login flow (see `AgentKind::login_support`), returning an
    /// interactive exec session: the caller streams output to show the user the OAuth URL (and,
    /// for `PasteCodeBack` kinds, later writes one line to `stdin_tx` once the user pastes back
    /// the code from their browser). Once logged in this way, credentials persist in the
    /// container's home directory for as long as the container itself isn't destroyed, so
    /// subsequent `run`/`run_stream` calls need no `api_key` at all.
    ///
    /// Returns `None` if this agent has no real login flow (`LoginSupport::None`) -- the caller
    /// should fall back to BYOK (`run`'s `api_key` param) instead of fabricating a login button
    /// for a tool that doesn't have one.
    pub async fn login(
        &self,
        kind: AgentKind,
    ) -> Result<Option<InteractiveExec>> {
        let Some(args) = kind.login_args() else {
            return Ok(None);
        };
        let mut cmd = vec![kind.binary().to_string()];
        cmd.extend(args);
        if kind == AgentKind::Agy {
            // `podman exec -t` allocates a real PTY inside the container, but never sets a size
            // on it -- confirmed live via `stty -F <the actual pts agy was running on> size`,
            // which reported "0 0". There's no real size for podman to propagate in the first
            // place: our own process's stdin/stdout here are plain pipes (`Stdio::piped()` in
            // `exec_interactive`), not a real terminal. A Go TUI framework (agy's included) that
            // waits for a valid `WindowSize` event before rendering anything past its initial
            // "clear screen, hide cursor, enter alternate buffer" setup sequence never gets one on
            // a 0x0 pty -- it just never draws again, forever, while the process itself stays
            // alive and burning real CPU the whole time (confirmed live via `ps aux` inside the
            // container: 0.7% CPU accumulating minutes after the "stall"). This exactly matches
            // the observed symptom, and explains why a real terminal (a genuine local shell, with
            // a real, valid size from the start) never has this problem. Fix: explicitly `stty` a
            // real size onto the exec's own controlling terminal (the pty that was just allocated
            // for it) before actually starting agy, by wrapping the command in a shell.
            cmd = vec![
                "sh".to_string(),
                "-c".to_string(),
                "stty rows 40 cols 120 2>/dev/null; exec agy".to_string(),
            ];
        }
        let mut opts = ExecOptions::new(cmd).timeout(AGENT_LOGIN_TIMEOUT);
        if kind == AgentKind::Agy {
            // agy's login menu is a real TUI (alternate screen buffer, cursor hiding -- confirmed
            // live via a PTY probe) that checks `isatty()` and takes a completely different,
            // broken code path without one: observed live, it silently read the first stdin
            // write as a one-line `--print` prompt and exited immediately ("Print mode: empty
            // prompt, exiting" in its own log) instead of ever showing the menu. `-i` alone
            // (no `-t`) is enough for claude/codex's plain-text login output, but not for this.
            opts = opts.tty(true);
            // agy's own log (`browser_context.go`) shows its OAuth step tries to download a
            // Playwright browser driver from three CDN mirrors, all 404ing in this container,
            // then waits further before it ever gets to the manual-paste-URL flow it already
            // supports -- confirmed live, this genuinely costs on the order of a minute or more
            // of real wall-clock time (`PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1` was tried here and
            // did not change this, so it isn't what agy's bundled Playwright checks). On the dev
            // host, where Playwright happens to already be installed from earlier browser-testing
            // work, this same step succeeds immediately -- which is why an earlier host-level
            // probe reached the URL prompt in seconds while a fresh container takes much longer.
            // Nothing to work around here beyond patience: the caller (and this login flow's own
            // timeout) just needs to allow for it.
        }
        let mut exec = self.container.exec_interactive(opts).await?;

        if kind == AgentKind::Agy {
            // Unlike claude/codex, agy's login isn't a dedicated subcommand: the bare binary
            // shows an interactive menu first ("1. Google OAuth" / "2. Use a Google Cloud
            // project", OAuth pre-highlighted) before it ever prints the URL + paste-back-code
            // prompt that the rest of this flow (and the web UI) expects. Auto-select OAuth by
            // sending Enter so the user-facing flow looks the same as every other PasteCodeBack
            // agent: URL shown, one code pasted back, done -- confirmed live via a PTY probe
            // that this keystroke is exactly what advances past that menu.
            //
            // How long agy's own background language-server takes to finish initializing before
            // it starts reading keystrokes is NOT stable -- live-observed at both 6.5s and 12.5s
            // across two otherwise-identical runs on the same machine (load-dependent). A fixed
            // delay that covers one run's startup time doesn't necessarily cover the next, so
            // instead of guessing a timeout, this watches the real output for the menu text
            // itself and sends Enter right after it actually appears -- with a generous 60s
            // fallback that sends it anyway in case the wording ever changes. The watcher runs in
            // a spawned task that re-emits every chunk it reads into a fresh channel, so the
            // caller (which needs to see this output too, to show the user the OAuth URL) isn't
            // missing anything -- it just receives it through `exec.stream`, transparently
            // relayed rather than read directly.
            let mut real_stream = exec.stream;
            let stdin_tx = exec.stdin_tx.clone();
            let (tx, rx) = mpsc::channel::<OutputChunk>(64);
            tokio::spawn(async move {
                let mut seen = String::new();
                let mut sent_enter = false;
                let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
                loop {
                    let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
                    if remaining.is_zero() {
                        if !sent_enter {
                            let _ = stdin_tx.send(b"\r".to_vec()).await;
                        }
                        break;
                    }
                    let next = tokio::time::timeout(remaining, real_stream.next_chunk()).await;
                    let chunk = match next {
                        | Ok(Some(c)) => c,
                        | Ok(None) => break,
                        | Err(_) => {
                            if !sent_enter {
                                let _ = stdin_tx.send(b"\r".to_vec()).await;
                            }
                            break;
                        },
                    };
                    if let OutputChunk::Stdout(bytes) | OutputChunk::Stderr(bytes) = &chunk {
                        // The raw bytes are a real terminal-UI frame (SGR color codes, cursor
                        // moves, alternate-screen control) -- searching them unstripped for plain
                        // English text reliably never matches, live-confirmed as the actual bug
                        // behind an earlier version of this that fell through to the 30s fallback
                        // on every single run despite the menu genuinely being on screen the
                        // whole time.
                        seen.push_str(&strip_ansi(&String::from_utf8_lossy(bytes)));
                    }
                    let is_exit = matches!(chunk, OutputChunk::Exit(_));
                    if tx.send(chunk).await.is_err() {
                        break;
                    }
                    if is_exit {
                        break;
                    }
                    if !sent_enter
                        && (seen.contains("Select login method") || seen.contains("Google OAuth"))
                    {
                        sent_enter = true;
                        let _ = stdin_tx.send(b"\r".to_vec()).await;
                    }
                }
                // Keep relaying anything that arrives after the menu decision (the OAuth URL
                // itself, the paste-code prompt) until the process exits or the channel closes.
                while let Some(chunk) = real_stream.next_chunk().await {
                    let is_exit = matches!(chunk, OutputChunk::Exit(_));
                    if tx.send(chunk).await.is_err() || is_exit {
                        break;
                    }
                }
            });
            exec.stream = ExecStream::new(rx);
        }

        Ok(Some(exec))
    }
}

/// Strip ANSI/CSI/OSC terminal control sequences, leaving only the visible text -- real TUI
/// output (like agy's login menu) is otherwise unsearchable for plain English substrings, since
/// every styled span is wrapped in escape codes.
fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\u{1b}' {
            out.push(c);
            continue;
        }
        match chars.peek() {
            | Some('[') => {
                // CSI: ESC [ ... <final byte in 0x40..=0x7E>
                chars.next();
                for c in chars.by_ref() {
                    if ('\x40'..='\x7e').contains(&c) {
                        break;
                    }
                }
            },
            | Some(']') => {
                // OSC: ESC ] ... (BEL or ESC \)
                chars.next();
                loop {
                    match chars.next() {
                        | Some('\u{7}') | None => break,
                        | Some('\u{1b}') => {
                            if chars.peek() == Some(&'\\') {
                                chars.next();
                            }
                            break;
                        },
                        | _ => {},
                    }
                }
            },
            | Some(_) => {
                // Two-byte escape (e.g. ESC = , ESC > ) -- just consume the next char.
                chars.next();
            },
            | None => {},
        }
    }
    out
}

#[cfg(test)]
mod strip_ansi_tests {
    use super::strip_ansi;

    #[test]
    fn test_strip_ansi_removes_csi_and_leaves_text() {
        let raw = "\u{1b}[?1049h\u{1b}[?25l Welcome to the Antigravity CLI. \u{1b}[1m> 1. Google OAuth\u{1b}[0m";
        let cleaned = strip_ansi(raw);
        assert_eq!(
            cleaned,
            " Welcome to the Antigravity CLI. > 1. Google OAuth"
        );
    }

    #[test]
    fn test_strip_ansi_handles_osc_and_two_byte_escapes() {
        let raw = "\u{1b}]0;title\u{7}\u{1b}=plain text\u{1b}>";
        let cleaned = strip_ansi(raw);
        assert_eq!(cleaned, "plain text");
    }
}
