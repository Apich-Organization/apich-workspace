//! Real Rust replacement for the AI Copilot drawer's hand-written JS (`components/mod.rs`'s
//! `AiDrawer`): BYOK chat, real in-container agent runs, and real agent account login are all
//! implemented here. The drawer's open/close/backdrop-dismiss is a plain HTML/CSS "checkbox hack"
//! (see the hidden `#ai-drawer-toggle-cb` input in the view below and its sibling selectors in
//! `styles.rs`) -- no JavaScript, no WASM, and so no dependency on this island's hydration having
//! completed before the very first "open the drawer" click works. External "🤖 AI Copilot"
//! buttons scattered across several pages (`project_detail.rs` etc.) are plain
//! `<label for="ai-drawer-toggle-cb">` elements -- a `<label>` can target a checkbox anywhere in
//! the document by id, so it doesn't need to be anywhere near this island's own output.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
struct ChatMessage {
    is_user: bool,
    text: String,
    provider_label: Option<String>,
    suggested_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct AgentInfo {
    id: String,
    name: String,
    available: bool,
    login_support: String,
}

#[derive(Debug, Clone, Serialize)]
struct AgentRunRequest<'a> {
    agent: &'a str,
    prompt: &'a str,
    api_key: Option<&'a str>,
}

#[island]
pub fn AiDrawerIsland(#[prop(into)] project_id: String, file_path: Option<String>) -> impl IntoView {
    let mode = RwSignal::new("chat".to_string());

    // --- Chat state ---
    let provider = RwSignal::new("builtin".to_string());
    let api_key = RwSignal::new(String::new());
    let welcome_text = if file_path.is_some() {
        "Ask about this file -- formulas, analysis, drafting, or scripts.".to_string()
    } else {
        "Ask about this project, or open a file to ask about it specifically.".to_string()
    };
    let messages = RwSignal::new(vec![ChatMessage {
        is_user: false,
        text: welcome_text,
        provider_label: None,
        suggested_code: None,
    }]);
    let prompt = RwSignal::new(String::new());
    let chat_busy = RwSignal::new(false);

    // --- Agent-run state ---
    let agents = RwSignal::new(Vec::<AgentInfo>::new());
    let agents_loaded = RwSignal::new(false);
    let selected_agent = RwSignal::new(String::new());
    let agent_api_key = RwSignal::new(String::new());
    let agent_prompt = RwSignal::new(String::new());
    let agent_output = RwSignal::new("Pick an agent and describe what you want done in this project.".to_string());
    let agent_busy = RwSignal::new(false);

    // --- Agent-login state ---
    let login_visible = RwSignal::new(false);
    let login_output = RwSignal::new(String::new());
    let login_session = RwSignal::new(None::<String>);
    let login_code_visible = RwSignal::new(false);
    let login_code = RwSignal::new(String::new());

    wire_load_saved_key(api_key);

    let selected_login_support = move || {
        let sel = selected_agent.get();
        agents.with(|list| list.iter().find(|a| a.id == sel).map(|a| a.login_support.clone())).unwrap_or_else(|| "none".to_string())
    };

    let load_agents = {
        let project_id = project_id.clone();
        move || {
            if agents_loaded.get_untracked() {
                return;
            }
            agents_loaded.set(true);
            fetch_agent_list(project_id.clone(), agents, selected_agent);
        }
    };

    let on_tab_click = {
        let load_agents = load_agents.clone();
        move |m: &'static str| {
            mode.set(m.to_string());
            if m == "agent" {
                load_agents();
            }
        }
    };

    let send_chat = {
        let project_id = project_id.clone();
        let file_path = file_path.clone();
        move || {
            let text = prompt.get_untracked().trim().to_string();
            if text.is_empty() || chat_busy.get_untracked() {
                return;
            }
            messages.update(|m| m.push(ChatMessage { is_user: true, text: text.clone(), provider_label: None, suggested_code: None }));
            prompt.set(String::new());
            chat_busy.set(true);
            send_chat_request(project_id.clone(), file_path.clone(), text, provider.get_untracked(), api_key.get_untracked(), messages, chat_busy);
        }
    };

    let run_agent = {
        let project_id = project_id.clone();
        move || {
            let agent = selected_agent.get_untracked();
            let text = agent_prompt.get_untracked().trim().to_string();
            if agent.is_empty() || text.is_empty() || agent_busy.get_untracked() {
                return;
            }
            agent_busy.set(true);
            agent_output.set(format!("Running {agent} inside the project's sandbox container..."));
            let key = agent_api_key.get_untracked();
            run_agent_request(project_id.clone(), agent, text, if key.trim().is_empty() { None } else { Some(key) }, agent_output, agent_busy);
        }
    };

    let start_login = {
        let project_id = project_id.clone();
        move || {
            let agent = selected_agent.get_untracked();
            if agent.is_empty() {
                return;
            }
            login_visible.set(true);
            login_code_visible.set(false);
            login_output.set("Starting login...".to_string());
            start_agent_login_request(project_id.clone(), agent, login_session, login_output, login_code_visible);
        }
    };

    let submit_code = {
        let project_id = project_id.clone();
        move || {
            let Some(session_id) = login_session.get_untracked() else { return };
            let code = login_code.get_untracked().trim().to_string();
            if code.is_empty() {
                return;
            }
            login_code.set(String::new());
            login_code_visible.set(false);
            submit_login_code_request(project_id.clone(), session_id, code, login_output);
        }
    };

    let insert_at_cursor = move |code: String| insert_at_editor_cursor(&code);
    let copy_code = move |code: String| copy_to_clipboard(&code);

    view! {
        // The panel's open/close is NOT driven by this Rust signal -- it needs to work the
        // instant the SSR'd page renders, before this island's WASM has necessarily hydrated
        // (live-verified failure mode: a `window` CustomEvent listener registered only once
        // hydrated left the trigger button doing nothing for as long as hydration was still in
        // flight, which on a page with many other islands to hydrate first was a visible, several-
        // second gap). Rather than reach for hydration-independent JS to patch around that, this
        // is a real HTML/CSS "checkbox hack": a hidden checkbox plus CSS sibling selectors
        // (`.ai-drawer-toggle-cb:checked ~ .ai-drawer-panel`, see `styles.rs`) do the whole
        // open/close/backdrop-dismiss job natively, with zero JavaScript and zero WASM involved
        // at any point -- not a hydration-independent workaround, but no dependency on hydration
        // (or on JS at all) to begin with. The trigger buttons elsewhere on the page
        // (`project_detail.rs` etc.) are plain `<label for="ai-drawer-toggle-cb">` elements; any
        // `<label>` anywhere in the document can toggle this checkbox regardless of DOM position.
        <input type="checkbox" id="ai-drawer-toggle-cb" class="ai-drawer-toggle-cb" />
        <label for="ai-drawer-toggle-cb" class="ai-drawer-backdrop"></label>
        <div class="ai-drawer-panel">
            <div class="ai-drawer-header">
                <div style="display:flex; align-items:center; gap:0.5rem;">
                    <span style="font-size:1.2rem;">"🤖"</span>
                    <div>
                        <strong style="font-size:0.95rem; color:var(--text-main);">"APICH Copilot"</strong>
                        <div style="font-size:0.75rem; color:var(--text-sub);">"Bring-your-own-key assistant"</div>
                    </div>
                </div>
                <label for="ai-drawer-toggle-cb" class="btn btn-ghost btn-sm" style="font-size:1.25rem; line-height:1;">"×"</label>
            </div>

            <div style="padding:0.5rem 1.25rem 0; display:flex; gap:0.4rem; border-bottom:1px solid var(--border-subtle);">
                <button type="button" class="btn btn-sm" class:btn-primary=move || mode.get() == "chat" class:btn-secondary=move || mode.get() != "chat" style="font-size:0.75rem;" on:click={let f = on_tab_click.clone(); move |_| f("chat")}>"💬 Chat"</button>
                <button type="button" class="btn btn-sm" class:btn-primary=move || mode.get() == "agent" class:btn-secondary=move || mode.get() != "agent" style="font-size:0.75rem;" on:click={let f = on_tab_click.clone(); move |_| f("agent")}>"🧑‍💻 Run Agent"</button>
            </div>

            <div style:display=move || if mode.get() == "chat" { "block" } else { "none" }>
                <div style="background:var(--bg-muted); padding:0.6rem 1.25rem; border-bottom:1px solid var(--border-subtle); display:flex; flex-direction:column; gap:0.4rem; font-size:0.75rem;">
                    <div style="display:flex; justify-content:space-between; align-items:center;">
                        <span style="font-weight:600; color:var(--text-sub);">"Provider:"</span>
                        <select class="form-control" style="width:auto; height:26px; padding:0 6px; font-size:0.75rem;" prop:value=move || provider.get() on:change=move |ev| provider.set(event_target_value(&ev))>
                            <option value="builtin">"Built-in"</option>
                            <option value="gemini">"Google Gemini (API key)"</option>
                            <option value="openai">"OpenAI (API key)"</option>
                            <option value="ollama">"Ollama (local)"</option>
                        </select>
                    </div>
                    <div style:display=move || if provider.get() == "gemini" || provider.get() == "openai" { "block" } else { "none" }>
                        <input
                            type="password"
                            placeholder="Paste your API key (stored in local storage)"
                            class="form-control"
                            style="font-size:0.75rem; height:28px;"
                            prop:value=move || api_key.get()
                            on:change=move |ev| { let v = event_target_value(&ev); api_key.set(v.clone()); save_key_to_storage(&v); }
                        />
                    </div>
                </div>

                <div class="ai-drawer-body">
                    {move || messages.get().into_iter().map(|m| {
                        let code = m.suggested_code.clone();
                        let insert = insert_at_cursor;
                        let copy = copy_code;
                        view! {
                            <div class="ai-msg" class:ai-msg-user=m.is_user class:ai-msg-assistant=!m.is_user>
                                {m.provider_label.map(|p| view! { <div style="font-size:0.7rem; color:var(--text-sub); margin-bottom:4px;">{p}</div> })}
                                <div style="white-space:pre-wrap;">{m.text}</div>
                                {code.map(|c| {
                                    let c1 = c.clone();
                                    let c2 = c.clone();
                                    view! {
                                        <div style="margin-top:0.75rem; display:flex; gap:0.5rem;">
                                            <button type="button" class="btn btn-primary btn-sm" style="font-size:0.75rem;" on:click=move |_| insert(c1.clone())>"✍️ Insert at Cursor"</button>
                                            <button type="button" class="btn btn-secondary btn-sm" style="font-size:0.75rem;" on:click=move |_| copy(c2.clone())>"📋 Copy Code"</button>
                                        </div>
                                    }
                                })}
                            </div>
                        }
                    }).collect::<Vec<_>>()}
                </div>

                <div style="padding:0.5rem 1.25rem; background:var(--bg-surface); border-top:1px solid var(--border-subtle); display:flex; flex-wrap:wrap; gap:0.35rem;">
                    <button type="button" class="btn btn-secondary btn-sm" style="font-size:0.7rem; padding:2px 7px;" on:click=move |_| prompt.set("Add a formula in this document format".to_string())>"🔬 Formula"</button>
                    <button type="button" class="btn btn-secondary btn-sm" style="font-size:0.7rem; padding:2px 7px;" on:click=move |_| prompt.set("Write a python script to analyze and plot this data".to_string())>"🐍 Python plot"</button>
                    <button type="button" class="btn btn-secondary btn-sm" style="font-size:0.7rem; padding:2px 7px;" on:click=move |_| prompt.set("Suggest next tasks for this file".to_string())>"📋 Suggest tasks"</button>
                </div>

                <div class="ai-drawer-footer">
                    <form on:submit={let f = send_chat.clone(); move |ev| { ev.prevent_default(); f(); }} style="display:flex; gap:0.5rem;">
                        <textarea
                            rows="2"
                            class="form-control"
                            style="font-size:0.85rem; resize:none;"
                            placeholder="Ask APICH Copilot..."
                            prop:value=move || prompt.get()
                            on:input=move |ev| prompt.set(event_target_value(&ev))
                        ></textarea>
                        <button type="submit" class="btn btn-primary" style="padding:0 1rem;" disabled=move || chat_busy.get()>"Send"</button>
                    </form>
                </div>
            </div>

            <div style:display=move || if mode.get() == "agent" { "flex" } else { "none" } style="flex-direction:column; flex:1; min-height:0;">
                <div style="background:var(--bg-muted); padding:0.6rem 1.25rem; border-bottom:1px solid var(--border-subtle); display:flex; flex-direction:column; gap:0.4rem; font-size:0.75rem;">
                    <div style="font-size:0.7rem; color:var(--text-sub);">
                        "Runs a real CLI agent inside this project's sandbox container, with access to the actual files -- not a hosted chat call."
                    </div>
                    <div style="display:flex; justify-content:space-between; align-items:center; gap:0.5rem;">
                        <span style="font-weight:600; color:var(--text-sub);">"Agent:"</span>
                        <select class="form-control" style="flex:1; height:26px; padding:0 6px; font-size:0.75rem;" prop:value=move || selected_agent.get() on:change=move |ev| selected_agent.set(event_target_value(&ev))>
                            {move || {
                                let list = agents.get();
                                if list.is_empty() {
                                    view! { <option value="">{if agents_loaded.get() { "Failed to check sandbox agents" } else { "Checking sandbox..." }}</option> }.into_any()
                                } else {
                                    list.into_iter().map(|a| {
                                        let label = if a.available { a.name.clone() } else { format!("{} (not installed in this sandbox image)", a.name) };
                                        view! { <option value=a.id.clone() disabled=!a.available>{label}</option> }
                                    }).collect::<Vec<_>>().into_any()
                                }
                            }}
                        </select>
                        <button
                            type="button"
                            class="btn btn-secondary btn-sm"
                            style:display=move || if selected_login_support() == "none" { "none" } else { "" }
                            style="font-size:0.7rem; white-space:nowrap;"
                            on:click={let f = start_login.clone(); move |_| f()}
                        >
                            "🔐 Login"
                        </button>
                    </div>
                    <input
                        type="password"
                        placeholder="Or paste an API key (sent only for this run, never stored server-side)"
                        class="form-control"
                        style="font-size:0.75rem; height:28px;"
                        prop:value=move || agent_api_key.get()
                        on:input=move |ev| agent_api_key.set(event_target_value(&ev))
                    />
                    <div style:display=move || if login_visible.get() { "block" } else { "none" } style="background:var(--bg-surface); border:1px solid var(--border-subtle); border-radius:6px; padding:0.5rem;">
                        <div style="font-size:0.7rem; font-weight:600; color:var(--text-sub); margin-bottom:0.3rem;">"Account login"</div>
                        <pre style="margin:0; font-size:0.7rem; white-space:pre-wrap; max-height:140px; overflow-y:auto;" inner_html=move || login_output.get()></pre>
                        <div style:display=move || if login_code_visible.get() { "flex" } else { "none" } style="gap:0.4rem; margin-top:0.4rem;">
                            <input
                                type="text"
                                placeholder="Paste the code from your browser"
                                class="form-control"
                                style="flex:1; font-size:0.75rem; height:26px;"
                                prop:value=move || login_code.get()
                                on:input=move |ev| login_code.set(event_target_value(&ev))
                            />
                            <button type="button" class="btn btn-primary btn-sm" style="font-size:0.7rem;" on:click={let f = submit_code.clone(); move |_| f()}>"Submit"</button>
                        </div>
                    </div>
                </div>
                <pre class="agent-output" style="flex:1; margin:0; padding:0.75rem 1.25rem; overflow-y:auto; font-size:0.8rem; white-space:pre-wrap; background:var(--bg-surface); color:var(--text-main);">{move || agent_output.get()}</pre>
                <div class="ai-drawer-footer">
                    <form on:submit={let f = run_agent.clone(); move |ev| { ev.prevent_default(); f(); }} style="display:flex; gap:0.5rem;">
                        <textarea
                            rows="2"
                            class="form-control"
                            style="font-size:0.85rem; resize:none;"
                            placeholder="e.g. Summarize the data in results.csv and suggest a plot"
                            prop:value=move || agent_prompt.get()
                            on:input=move |ev| agent_prompt.set(event_target_value(&ev))
                        ></textarea>
                        <button type="submit" class="btn btn-primary" style="padding:0 1rem;" disabled=move || agent_busy.get()>"Run"</button>
                    </form>
                </div>
            </div>
        </div>
    }
}

#[cfg(feature = "hydrate")]
fn wire_load_saved_key(api_key: RwSignal<String>) {
    if let Some(Ok(Some(storage))) = web_sys::window().map(|w| w.local_storage()) {
        if let Ok(Some(saved)) = storage.get_item("apich_ai_key") {
            api_key.set(saved);
        }
    }
}
#[cfg(not(feature = "hydrate"))]
fn wire_load_saved_key(_api_key: RwSignal<String>) {}

#[cfg(feature = "hydrate")]
fn save_key_to_storage(val: &str) {
    if let Some(Ok(Some(storage))) = web_sys::window().map(|w| w.local_storage()) {
        let _ = storage.set_item("apich_ai_key", val);
    }
}
#[cfg(not(feature = "hydrate"))]
fn save_key_to_storage(_val: &str) {}

#[cfg(feature = "hydrate")]
fn insert_at_editor_cursor(text: &str) {
    use wasm_bindgen::JsCast;
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else { return };
    let el = doc.get_element_by_id("code-editor-input").or_else(|| doc.get_element_by_id("note-body-editor"));
    let Some(el) = el else { return };
    let Ok(ta) = el.dyn_into::<web_sys::HtmlTextAreaElement>() else { return };
    let start = ta.selection_start().ok().flatten().unwrap_or(0);
    let end = ta.selection_end().ok().flatten().unwrap_or(0);
    let val = ta.value();
    let chars: Vec<char> = val.chars().collect();
    let start = (start as usize).min(chars.len());
    let end = (end as usize).min(chars.len());
    let new_val: String = chars[..start].iter().collect::<String>() + text + &chars[end..].iter().collect::<String>();
    ta.set_value(&new_val);
    let _ = ta.focus();
    let new_pos = (start + text.chars().count()) as u32;
    let _ = ta.set_selection_range(new_pos, new_pos);
}
#[cfg(not(feature = "hydrate"))]
fn insert_at_editor_cursor(_text: &str) {}

#[cfg(feature = "hydrate")]
fn copy_to_clipboard(text: &str) {
    if let Some(win) = web_sys::window() {
        let _ = win.navigator().clipboard().write_text(text);
    }
}
#[cfg(not(feature = "hydrate"))]
fn copy_to_clipboard(_text: &str) {}

#[cfg(feature = "hydrate")]
fn sanitize_term_output(text: &str) -> String {
    // Real CLI output carries ANSI escape sequences (colors, OSC-8 hyperlinks like the ones
    // `claude auth login` prints around its URL) that render as garbage in plain text.
    let mut out = String::with_capacity(text.len());
    let bytes: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == '\u{1b}' {
            if i + 1 < bytes.len() && bytes[i + 1] == ']' {
                // OSC: ESC ] ... (BEL | ESC \)
                let mut j = i + 2;
                while j < bytes.len() && bytes[j] != '\u{7}' && !(bytes[j] == '\u{1b}' && j + 1 < bytes.len() && bytes[j + 1] == '\\') {
                    j += 1;
                }
                i = if j < bytes.len() && bytes[j] == '\u{7}' { j + 1 } else { j + 2 };
                continue;
            } else if i + 1 < bytes.len() && bytes[i + 1] == '[' {
                // CSI: ESC [ ... letter
                let mut j = i + 2;
                while j < bytes.len() && !bytes[j].is_ascii_alphabetic() {
                    j += 1;
                }
                i = j + 1;
                continue;
            } else {
                i += 2;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    out
}

#[cfg(feature = "hydrate")]
fn linkify(text: &str) -> String {
    let escaped = html_escape(text);
    let mut out = String::new();
    let mut rest = escaped.as_str();
    while let Some(pos) = rest.find("http") {
        let (before, from_http) = rest.split_at(pos);
        out.push_str(before);
        let is_url = from_http.starts_with("https://") || from_http.starts_with("http://");
        if !is_url {
            out.push_str("http");
            rest = &from_http[4..];
            continue;
        }
        let end = from_http.find(|c: char| c.is_whitespace() || c == '<').unwrap_or(from_http.len());
        let (url, remainder) = from_http.split_at(end);
        out.push_str(&format!(r#"<a href="{url}" target="_blank" rel="noopener noreferrer" style="color:var(--primary); text-decoration:underline;">{url}</a>"#));
        rest = remainder;
    }
    out.push_str(rest);
    out
}

/// The real login URL a CLI agent prints is already made clickable by `linkify` -- but it's
/// buried inline in a wall of dense, monospace terminal output the user has to actually read to
/// notice, right after a line like Claude's own "Opening browser to sign in..." (an attempt that
/// happens *inside the sandbox container*, which has no real browser -- it always silently does
/// nothing, by design, not a bug in this app). A user skimming that first line and seeing nothing
/// happen on their own screen has no obvious reason to keep reading for the fallback URL.
/// `poll_login_status` uses this to also surface the same URL as a prominent, un-missable button
/// above the raw output, for every agent that ever prints one -- not a fix for any specific
/// agent's login flow, just making the "this link is for YOUR browser" affordance impossible to
/// miss regardless of which agent's output it came from.
#[cfg(feature = "hydrate")]
fn first_url(text: &str) -> Option<String> {
    let mut idx = 0;
    while let Some(pos) = text[idx..].find("http") {
        let start = idx + pos;
        let candidate = &text[start..];
        if candidate.starts_with("https://") || candidate.starts_with("http://") {
            let end = candidate.find(|c: char| c.is_whitespace() || c == '<').unwrap_or(candidate.len());
            return Some(candidate[..end].to_string());
        }
        idx = start + 4;
        if idx >= text.len() {
            break;
        }
    }
    None
}

#[cfg(feature = "hydrate")]
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

#[cfg(feature = "hydrate")]
fn fetch_agent_list(project_id: String, agents: RwSignal<Vec<AgentInfo>>, selected_agent: RwSignal<String>) {
    wasm_bindgen_futures::spawn_local(async move {
        let result = gloo_net::http::Request::get(&format!("/projects/{}/agent/status", project_id)).send().await;
        if let Ok(resp) = result {
            if let Ok(data) = resp.json::<serde_json::Value>().await {
                if data.get("success").and_then(|v| v.as_bool()) == Some(true) {
                    if let Some(list) = data.get("agents").and_then(|v| v.as_array()) {
                        let parsed: Vec<AgentInfo> = list
                            .iter()
                            .filter_map(|a| {
                                Some(AgentInfo {
                                    id: a.get("id")?.as_str()?.to_string(),
                                    name: a.get("name")?.as_str()?.to_string(),
                                    available: a.get("available")?.as_bool()?,
                                    login_support: a.get("login_support").and_then(|v| v.as_str()).unwrap_or("none").to_string(),
                                })
                            })
                            .collect();
                        if let Some(first) = parsed.iter().find(|a| a.available) {
                            selected_agent.set(first.id.clone());
                        }
                        agents.set(parsed);
                        return;
                    }
                }
            }
        }
        agents.set(Vec::new());
    });
}
#[cfg(not(feature = "hydrate"))]
fn fetch_agent_list(_project_id: String, _agents: RwSignal<Vec<AgentInfo>>, _selected_agent: RwSignal<String>) {}

#[cfg(feature = "hydrate")]
fn send_chat_request(
    project_id: String,
    file_path: Option<String>,
    prompt: String,
    provider: String,
    api_key: String,
    messages: RwSignal<Vec<ChatMessage>>,
    busy: RwSignal<bool>,
) {
    wasm_bindgen_futures::spawn_local(async move {
        let file_content = current_editor_content();
        let body = serde_json::json!({
            "prompt": prompt,
            "context_file": file_path,
            "file_content": file_content,
            "provider": provider,
            "api_key": api_key,
        });
        let result = gloo_net::http::Request::post(&format!("/projects/{}/ai/chat", project_id))
            .json(&body)
            .expect("valid json body")
            .send()
            .await;
        match result {
            Ok(resp) => match resp.json::<serde_json::Value>().await {
                Ok(data) => {
                    let reply = data.get("reply").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                    let provider_label = data.get("provider").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let suggested_code = data.get("suggested_code").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(|s| s.to_string());
                    messages.update(|m| m.push(ChatMessage { is_user: false, text: reply, provider_label, suggested_code }));
                }
                Err(e) => messages.update(|m| m.push(ChatMessage { is_user: false, text: format!("Error parsing response: {e}"), provider_label: None, suggested_code: None })),
            },
            Err(e) => messages.update(|m| m.push(ChatMessage { is_user: false, text: format!("Error calling AI Assistant: {e}"), provider_label: None, suggested_code: None })),
        }
        busy.set(false);
    });
}
#[cfg(not(feature = "hydrate"))]
fn send_chat_request(
    _project_id: String,
    _file_path: Option<String>,
    _prompt: String,
    _provider: String,
    _api_key: String,
    _messages: RwSignal<Vec<ChatMessage>>,
    busy: RwSignal<bool>,
) {
    busy.set(false);
}

#[cfg(feature = "hydrate")]
fn current_editor_content() -> String {
    use wasm_bindgen::JsCast;
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else { return String::new() };
    let el = doc.get_element_by_id("code-editor-input").or_else(|| doc.get_element_by_id("note-body-editor"));
    el.and_then(|e| e.dyn_into::<web_sys::HtmlTextAreaElement>().ok()).map(|ta| ta.value()).unwrap_or_default()
}

#[cfg(feature = "hydrate")]
fn run_agent_request(project_id: String, agent: String, prompt: String, api_key: Option<String>, output: RwSignal<String>, busy: RwSignal<bool>) {
    wasm_bindgen_futures::spawn_local(async move {
        let body = AgentRunRequest { agent: &agent, prompt: &prompt, api_key: api_key.as_deref() };
        let result = gloo_net::http::Request::post(&format!("/projects/{}/agent/run", project_id))
            .json(&body)
            .expect("valid json body")
            .send()
            .await;
        match result {
            Ok(resp) => match resp.json::<serde_json::Value>().await {
                Ok(data) => {
                    if let Some(err) = data.get("error").and_then(|v| v.as_str()) {
                        if data.get("stdout").is_none() {
                            output.set(format!("Error: {err}"));
                            busy.set(false);
                            return;
                        }
                    }
                    let stdout = data.get("stdout").and_then(|v| v.as_str()).unwrap_or_default();
                    let stderr = data.get("stderr").and_then(|v| v.as_str()).unwrap_or_default();
                    let exit_code = data.get("exit_code").and_then(|v| v.as_i64()).unwrap_or(-1);
                    let success = data.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
                    let mut text = format!("{stdout}{stderr}");
                    if text.is_empty() {
                        text = "(no output)".to_string();
                    }
                    text.push_str(&format!("\n\n[exit code {exit_code}, {}]", if success { "succeeded" } else { "failed" }));
                    output.set(text);
                }
                Err(e) => output.set(format!("Error parsing response: {e}")),
            },
            Err(e) => output.set(format!("Request failed: {e}")),
        }
        busy.set(false);
    });
}
#[cfg(not(feature = "hydrate"))]
fn run_agent_request(_project_id: String, _agent: String, _prompt: String, _api_key: Option<String>, _output: RwSignal<String>, busy: RwSignal<bool>) {
    busy.set(false);
}

#[cfg(feature = "hydrate")]
fn start_agent_login_request(project_id: String, agent: String, session: RwSignal<Option<String>>, output: RwSignal<String>, code_visible: RwSignal<bool>) {
    wasm_bindgen_futures::spawn_local(async move {
        let body = serde_json::json!({ "agent": agent });
        let result = gloo_net::http::Request::post(&format!("/projects/{}/agent/login/start", project_id))
            .json(&body)
            .expect("valid json body")
            .send()
            .await;
        match result {
            Ok(resp) => match resp.json::<serde_json::Value>().await {
                Ok(data) => {
                    if data.get("success").and_then(|v| v.as_bool()) != Some(true) {
                        let err = data.get("error").and_then(|v| v.as_str()).unwrap_or("Failed to start login");
                        output.set(format!("Error: {err}"));
                        return;
                    }
                    let Some(session_id) = data.get("session_id").and_then(|v| v.as_str()).map(|s| s.to_string()) else {
                        output.set("Error: no session id returned".to_string());
                        return;
                    };
                    if data.get("login_support").and_then(|v| v.as_str()) == Some("paste_code_back") {
                        code_visible.set(true);
                    }
                    session.set(Some(session_id.clone()));
                    poll_login_status(project_id.clone(), session_id, output, code_visible);
                }
                Err(e) => output.set(format!("Request failed: {e}")),
            },
            Err(e) => output.set(format!("Request failed: {e}")),
        }
    });
}
#[cfg(not(feature = "hydrate"))]
fn start_agent_login_request(_project_id: String, _agent: String, _session: RwSignal<Option<String>>, _output: RwSignal<String>, _code_visible: RwSignal<bool>) {}

#[cfg(feature = "hydrate")]
fn poll_login_status(project_id: String, session_id: String, output: RwSignal<String>, code_visible: RwSignal<bool>) {
    wasm_bindgen_futures::spawn_local(async move {
        let result = gloo_net::http::Request::get(&format!("/projects/{}/agent/login/{}/status", project_id, session_id)).send().await;
        let mut keep_polling = true;
        match result {
            Ok(resp) => match resp.json::<serde_json::Value>().await {
                Ok(data) => {
                    if data.get("success").and_then(|v| v.as_bool()) != Some(true) {
                        let err = data.get("error").and_then(|v| v.as_str()).unwrap_or("Unknown error");
                        output.set(format!("Error: {err}"));
                        keep_polling = false;
                    } else {
                        let raw = data.get("output").and_then(|v| v.as_str()).unwrap_or_default();
                        let sanitized = sanitize_term_output(raw);
                        let mut html = linkify(&sanitized);
                        if html.is_empty() {
                            html = "(waiting for output...)".to_string();
                        }
                        if let Some(url) = first_url(&sanitized) {
                            html = format!(
                                "<div style=\"margin-bottom:0.6rem; padding:0.5rem 0.7rem; background:var(--primary-light); border:1px solid var(--primary-border); border-radius:8px;\">\
                                <div style=\"font-size:0.7rem; color:var(--text-sub); margin-bottom:0.35rem;\">This runs inside a sandbox container with no browser of its own -- open this link in <strong>your own browser</strong> to finish signing in:</div>\
                                <a href=\"{url}\" target=\"_blank\" rel=\"noopener noreferrer\" class=\"btn btn-primary btn-sm\" style=\"text-decoration:none;\">🔗 Open sign-in page</a>\
                                </div>{html}"
                            );
                        }
                        let status = data.get("status").and_then(|v| v.as_str()).unwrap_or("running");
                        if status == "succeeded" {
                            html.push_str("<br><br>✅ Logged in. Future runs of this agent will use your account, no API key needed.");
                            code_visible.set(false);
                            keep_polling = false;
                        } else if status == "failed" {
                            let exit_code = data.get("exit_code").and_then(|v| v.as_i64()).unwrap_or(-1);
                            html.push_str(&format!("<br><br>❌ Login did not complete (exit code {exit_code})."));
                            keep_polling = false;
                        }
                        output.set(html);
                    }
                }
                Err(e) => {
                    output.update(|o| o.push_str(&format!("<br><br>Polling failed: {e}")));
                    keep_polling = false;
                }
            },
            Err(e) => {
                output.update(|o| o.push_str(&format!("<br><br>Polling failed: {e}")));
                keep_polling = false;
            }
        }
        if keep_polling {
            schedule_poll(project_id, session_id, output, code_visible);
        }
    });
}
#[cfg(not(feature = "hydrate"))]
fn poll_login_status(_project_id: String, _session_id: String, _output: RwSignal<String>, _code_visible: RwSignal<bool>) {}

#[cfg(feature = "hydrate")]
fn schedule_poll(project_id: String, session_id: String, output: RwSignal<String>, code_visible: RwSignal<bool>) {
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;
    let closure = Closure::once(move || {
        poll_login_status(project_id, session_id, output, code_visible);
    });
    if let Some(win) = web_sys::window() {
        let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(closure.as_ref().unchecked_ref(), 1500);
    }
    closure.forget();
}

#[cfg(feature = "hydrate")]
fn submit_login_code_request(project_id: String, session_id: String, code: String, output: RwSignal<String>) {
    wasm_bindgen_futures::spawn_local(async move {
        let body = format!("code={}", urlencode(&code));
        let result = gloo_net::http::Request::post(&format!("/projects/{}/agent/login/{}/code", project_id, session_id))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .expect("valid form body")
            .send()
            .await;
        if let Ok(resp) = result {
            if let Ok(data) = resp.json::<serde_json::Value>().await {
                if data.get("success").and_then(|v| v.as_bool()) != Some(true) {
                    let err = data.get("error").and_then(|v| v.as_str()).unwrap_or("Unknown error");
                    output.update(|o| o.push_str(&format!("<br><br>Failed to submit code: {err}")));
                }
            }
        }
    });
}
#[cfg(not(feature = "hydrate"))]
fn submit_login_code_request(_project_id: String, _session_id: String, _code: String, _output: RwSignal<String>) {}

#[cfg(feature = "hydrate")]
fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}
