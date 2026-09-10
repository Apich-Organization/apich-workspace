//! Real Rust replacement for the terminal page's hand-written JS (`terminal_page.rs`): submits
//! commands to the project's real sandbox container and appends real output to the screen.

use leptos::prelude::*;

#[island]
pub fn TerminalIsland(#[prop(into)] project_id: String, #[prop(into)] initial_screen: String) -> impl IntoView {
    let screen = RwSignal::new(initial_screen);
    let input = RwSignal::new(String::new());
    let busy = RwSignal::new(false);

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
            <div style="display:flex; gap:0.4rem; flex-wrap:wrap;">
                <span style="font-size:0.75rem; color:#94a3b8; margin-right:0.25rem; display:flex; align-items:center;">"Quick Commands:"</span>
                {quick_commands.into_iter().map(|cmd| {
                    view! {
                        <button
                            type="button"
                            class="btn btn-secondary btn-sm"
                            style="background:#1e293b; color:#cbd5e1; border-color:#334155; font-family:var(--font-mono); font-size:0.75rem;"
                            on:click=move |_| input.set(cmd.to_string())
                        >
                            {cmd}
                        </button>
                    }
                }).collect::<Vec<_>>()}
            </div>
        </div>

        <div id="term-screen" class="terminal-screen">{move || screen.get()}</div>
        <form on:submit=move |ev| { ev.prevent_default(); submit(); }>
            <div class="terminal-bar">
                <span style="color:#38bdf8; font-family:var(--font-mono); display:flex; align-items:center; font-weight:700;">"$"</span>
                <input
                    type="text"
                    class="terminal-input"
                    placeholder="e.g., typst compile main.typ paper.pdf"
                    autocomplete="off"
                    autofocus=true
                    prop:value=move || input.get()
                    on:input=move |ev| input.set(event_target_value(&ev))
                />
                <button type="submit" class="btn btn-primary btn-sm" disabled=move || busy.get()>"Run"</button>
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
