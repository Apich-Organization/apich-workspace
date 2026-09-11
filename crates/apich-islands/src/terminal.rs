//! Real Rust replacement for the terminal page's hand-written JS (`terminal_page.rs`): submits
//! commands to the project's real sandbox container and appends real output to the screen.

use leptos::prelude::*;

#[island]
pub fn TerminalIsland(
    #[prop(into)] project_id: String,
    #[prop(into)] initial_screen: String,
    is_zh: bool,
) -> impl IntoView {
    let screen = RwSignal::new(initial_screen);
    let input = RwSignal::new(String::new());
    let busy = RwSignal::new(false);

    // Real bug, confirmed live: before this, every control here rendered as if already
    // interactive (server-rendered HTML has no way to know the ~4MB wasm bundle hasn't finished
    // downloading/instantiating in this browser yet). Click a quick-command button or press
    // Enter in that window and nothing visibly happens -- no listener is attached yet, so a
    // button click is silently swallowed and the input's own `<form>` (no `action`, relying
    // entirely on JS to `prevent_default()`) falls back to a native GET-to-self submission,
    // which just reloads the page with the screen reset and no error. On a slower connection
    // (confirmed with Chrome DevTools network throttling at 300kbps/150ms latency, plausible for
    // this being served across a VM boundary) that window is easily long enough for an
    // impatient real user to hit. `hydrated` starts `false` in the server-rendered HTML (so
    // controls render visibly disabled from the first paint) and flips to `true` from an
    // `Effect`, which -- unlike the rest of this function's body -- only ever runs on the client
    // after hydration actually completes, never during SSR.
    let hydrated = RwSignal::new(false);
    Effect::new(move |_| {
        hydrated.set(true);
    });

    let submit = move || {
        let cmd = input.get_untracked().trim().to_string();
        if cmd.is_empty() || busy.get_untracked() {
            return;
        }
        screen.update(|s| {
            s.push_str("\n$ ");
            s.push_str(&cmd);
            s.push('\n');
        });
        input.set(String::new());
        busy.set(true);
        run_command(project_id.clone(), cmd, screen, busy);
    };

    let quick_commands = ["typst --version", "python3 --version", "git status", "ls -la"];

    view! {
        <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:0.75rem; flex-wrap:wrap; gap:0.5rem;">
            <div style="display:flex; gap:0.4rem; flex-wrap:wrap; align-items:center;">
                <span style="font-size:0.75rem; color:#94a3b8; margin-right:0.25rem; display:flex; align-items:center;">{crate::t(is_zh, "Quick Commands:", "快捷命令：")}</span>
                {quick_commands.into_iter().map(|cmd| {
                    view! {
                        <button
                            type="button"
                            class="btn btn-secondary btn-sm"
                            style="background:#1e293b; color:#cbd5e1; border-color:#334155; font-family:var(--font-mono); font-size:0.75rem;"
                            disabled=move || !hydrated.get()
                            on:click=move |_| input.set(cmd.to_string())
                        >
                            {cmd}
                        </button>
                    }
                }).collect::<Vec<_>>()}
                <span
                    style=move || format!(
                        "font-size:0.72rem; color:#f59e0b; align-items:center; gap:0.3rem; display:{};",
                        if hydrated.get() { "none" } else { "flex" }
                    )
                >
                    "⏳ "{crate::t(is_zh, "Loading terminal...", "终端加载中…")}
                </span>
            </div>
        </div>

        <div id="term-screen" class="terminal-screen">{move || screen.get()}</div>
        <form on:submit=move |ev| { ev.prevent_default(); submit(); }>
            <div class="terminal-bar">
                <span style="color:#38bdf8; font-family:var(--font-mono); display:flex; align-items:center; font-weight:700;">"$"</span>
                <input
                    type="text"
                    class="terminal-input"
                    placeholder=move || if hydrated.get() {
                        crate::t(is_zh, "e.g., typst compile main.typ paper.pdf", "例如：typst compile main.typ paper.pdf")
                    } else {
                        crate::t(is_zh, "Loading terminal, please wait...", "终端加载中，请稍候…")
                    }
                    autocomplete="off"
                    autofocus=true
                    disabled=move || !hydrated.get()
                    prop:value=move || input.get()
                    on:input=move |ev| input.set(event_target_value(&ev))
                />
                <button type="submit" class="btn btn-primary btn-sm" disabled=move || busy.get() || !hydrated.get()>{crate::t(is_zh, "Run", "运行")}</button>
            </div>
        </form>
    }
}

#[cfg(feature = "hydrate")]
fn run_command(project_id: String, cmd: String, screen: RwSignal<String>, busy: RwSignal<bool>) {
    wasm_bindgen_futures::spawn_local(async move {
        let body = format!("command={}", urlencode(&cmd));
        let result = gloo_net::http::Request::post(&format!("/projects/{}/terminal/exec", project_id))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .expect("valid form body")
            .send()
            .await;

        match result {
            Ok(resp) => match resp.json::<serde_json::Value>().await {
                Ok(data) => {
                    if let Some(out) = data.get("output").and_then(|v| v.as_str()) {
                        screen.update(|s| s.push_str(out));
                    } else if let Some(err) = data.get("error").and_then(|v| v.as_str()) {
                        screen.update(|s| {
                            s.push_str("[Error]: ");
                            s.push_str(err);
                            s.push('\n');
                        });
                    }
                }
                Err(e) => screen.update(|s| s.push_str(&format!("[Response Error]: {e}\n"))),
            },
            Err(e) => screen.update(|s| s.push_str(&format!("[Network Error]: {e}\n"))),
        }
        busy.set(false);
    });
}

#[cfg(not(feature = "hydrate"))]
fn run_command(_project_id: String, _cmd: String, _screen: RwSignal<String>, busy: RwSignal<bool>) {
    busy.set(false);
}

#[cfg(feature = "hydrate")]
fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}
