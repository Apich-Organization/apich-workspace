//! Quick-start menu: one-click starters for each document kind this app supports.
//!
//! Clicking a kind opens a dialog asking *where* it should go -- a brand new project (with an
//! editable, pre-filled name) or one of the user's existing projects -- rather than silently
//! creating a project the moment a button is pressed. The existing-project list is fetched from
//! `/projects/mine.json` when the dialog opens, not threaded through every page's `AppShell` as
//! a prop, since it's only ever needed once the dialog is actually open.

use leptos::prelude::*;

/// (kind, icon, label) for each document starter.
pub const QUICK_START_DOC_ITEMS: &[(&str, &str, &str)] = &[
    ("slide", "📊", "Slide Deck"),
    ("typst", "📄", "Typst Document"),
    ("latex", "📝", "LaTeX Document"),
    ("table", "🗄️", "Table"),
    ("note", "📔", "Note"),
];

/// Where the menu is rendered.
///
/// Purely a matter of how the triggers are laid out: the sidebar stacks them as nav rows, the
/// dashboard hangs them in a hover dropdown. The dialog itself is identical either way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum QuickStartVariant {
    #[default]
    Sidebar,
    Dashboard,
}

#[island]
pub fn QuickStartMenuIsland(variant: QuickStartVariant) -> impl IntoView {
    let open = RwSignal::new(false);
    let kind = RwSignal::new(String::new());
    let kind_label = RwSignal::new(String::new());
    // Script language: "python" | "r" | "rust"
    let language = RwSignal::new("python".to_string());
    // "new" | "existing"
    let mode = RwSignal::new("new".to_string());
    let project_name = RwSignal::new(String::new());
    let projects = RwSignal::new(Vec::<(String, String)>::new());
    let selected_project = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    // This dialog renders its own backdrop rather than going through `ModalIsland`, so it needs
    // the same escape hatch that component documents: the sidebar variant lives inside
    // `.app-sidebar`, whose `backdrop-filter` would otherwise make it the containing block for
    // this `position: fixed` overlay and strand the dialog inside the sidebar's own narrow box.
    let backdrop_ref = NodeRef::<leptos::html::Div>::new();
    crate::modal::reparent_to_body(backdrop_ref);

    let choose = move |k: &'static str, label: &'static str, lang: Option<&'static str>| {
        kind.set(k.to_string());
        kind_label.set(label.to_string());
        if let Some(l) = lang {
            language.set(l.to_string());
        } else if k == "script" {
            language.set("python".to_string());
        }
        mode.set("new".to_string());
        project_name.set(String::new());
        selected_project.set(String::new());
        error.set(None);
        busy.set(false);
        open.set(true);
        load_projects(projects, selected_project);
    };

    let submit = move |_| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        error.set(None);
        create_quick_start(QuickStartRequest {
            kind: kind.get_untracked(),
            language: language.get_untracked(),
            mode: mode.get_untracked(),
            name: project_name.get_untracked(),
            project_id: selected_project.get_untracked(),
            busy,
            error,
        });
    };

    let menu = if variant == QuickStartVariant::Sidebar {
        let doc_triggers = QUICK_START_DOC_ITEMS
            .iter()
            .map(|(k, icon, label)| {
                let (k, icon, label) = (*k, *icon, *label);
                view! {
                    <button type="button" class="sidebar-link sidebar-quick-btn" on:click=move |_| choose(k, label, None)>
                        <span class="sidebar-icon">{icon}</span>
                        <span>{label}</span>
                    </button>
                }
            })
            .collect::<Vec<_>>();

        view! {
            <div class="sidebar-section">
                <span class="sidebar-heading">"New"</span>
                {doc_triggers}
                <button
                    type="button"
                    class="sidebar-link sidebar-quick-btn"
                    style="display:flex; align-items:center; justify-content:space-between;"
                    on:click=move |_| choose("script", "Script", Some("python"))
                >
                    <div style="display:flex; align-items:center; gap:0.5rem;">
                        <span class="sidebar-icon">"💻"</span>
                        <span>"Script"</span>
                    </div>
                    <span style="font-size:0.65rem; font-weight:600; color:var(--text-sub); background:var(--bg-muted); padding:1px 6px; border-radius:4px; border:1px solid var(--border-subtle); letter-spacing:0.3px;">
                        "R · Py · Rs"
                    </span>
                </button>
            </div>
        }
        .into_any()
    } else {
        let doc_triggers = QUICK_START_DOC_ITEMS
            .iter()
            .map(|(k, icon, label)| {
                let (k, icon, label) = (*k, *icon, *label);
                view! {
                    <button type="button" class="quick-start-item" on:click=move |_| choose(k, label, None)>
                        <span>{icon}</span>
                        <span>{label}</span>
                    </button>
                }
            })
            .collect::<Vec<_>>();

        view! {
            <div class="quick-start-menu">
                <button type="button" class="btn btn-secondary">"⚡ Quick Start ▾"</button>
                <div class="quick-start-dropdown">
                    <div class="quick-start-panel">
                        {doc_triggers}
                        <div style="height:1px; background:var(--border-subtle); margin:4px 0;"></div>
                        <div style="padding:4px 10px 2px 10px; font-size:0.7rem; font-weight:700; color:var(--text-sub); text-transform:uppercase; letter-spacing:0.5px;">
                            "Scripts"
                        </div>
                        <button type="button" class="quick-start-item" on:click=move |_| choose("script", "Python Script", Some("python"))>
                            <span>"🐍"</span>
                            <span>"Python Script (.py)"</span>
                        </button>
                        <button type="button" class="quick-start-item" on:click=move |_| choose("script", "R Script", Some("r"))>
                            <span>"📊"</span>
                            <span>"R Script (.R)"</span>
                        </button>
                        <button type="button" class="quick-start-item" on:click=move |_| choose("script", "Rust Script", Some("rust"))>
                            <span>"🦀"</span>
                            <span>"Rust Script (.rs)"</span>
                        </button>
                    </div>
                </div>
            </div>
        }
        .into_any()
    };

    view! {
        {menu}
        <div node_ref=backdrop_ref class="modal-backdrop" style:display=move || if open.get() { "flex" } else { "none" }>
            <div class="modal-card" style="max-width:520px; width:100%;">
                <div class="modal-header">
                    <h3 class="modal-title">
                        {move || {
                            if kind.get() == "script" {
                                match language.get().as_str() {
                                    "r" => "New R Script".to_string(),
                                    "rust" => "New Rust Script".to_string(),
                                    _ => "New Python Script".to_string(),
                                }
                            } else {
                                format!("New {}", kind_label.get())
                            }
                        }}
                    </h3>
                    <button type="button" class="modal-close" on:click=move |_| open.set(false)>"×"</button>
                </div>

                // Language selection when creating a script
                {move || (kind.get() == "script").then(|| view! {
                    <div class="form-group" style="margin-bottom:1.25rem;">
                        <label style="display:block; font-size:0.85rem; font-weight:600; margin-bottom:0.5rem; color:var(--text-main);">
                            "Choose Script Language"
                        </label>
                        <div style="display:grid; grid-template-columns: repeat(3, 1fr); gap:0.65rem;">
                            // Python
                            <button
                                type="button"
                                style=move || {
                                    let is_sel = language.get() == "python";
                                    if is_sel {
                                        "padding:0.75rem 0.5rem; border-radius:8px; text-align:center; cursor:pointer; transition:all 0.15s ease; border:2px solid var(--primary); background:var(--bg-muted); box-shadow:0 2px 8px rgba(0,0,0,0.08);"
                                    } else {
                                        "padding:0.75rem 0.5rem; border-radius:8px; text-align:center; cursor:pointer; transition:all 0.15s ease; border:1px solid var(--border-subtle); background:var(--bg-surface); opacity:0.85;"
                                    }
                                }
                                on:click=move |_| language.set("python".to_string())
                            >
                                <div style="font-size:1.5rem; margin-bottom:0.25rem;">"🐍"</div>
                                <div style="font-weight:600; font-size:0.875rem; color:var(--text-main);">"Python"</div>
                                <div style="font-size:0.75rem; color:var(--text-muted); margin-top:2px;">".py"</div>
                            </button>

                            // R
                            <button
                                type="button"
                                style=move || {
                                    let is_sel = language.get() == "r";
                                    if is_sel {
                                        "padding:0.75rem 0.5rem; border-radius:8px; text-align:center; cursor:pointer; transition:all 0.15s ease; border:2px solid var(--primary); background:var(--bg-muted); box-shadow:0 2px 8px rgba(0,0,0,0.08);"
                                    } else {
                                        "padding:0.75rem 0.5rem; border-radius:8px; text-align:center; cursor:pointer; transition:all 0.15s ease; border:1px solid var(--border-subtle); background:var(--bg-surface); opacity:0.85;"
                                    }
                                }
                                on:click=move |_| language.set("r".to_string())
                            >
                                <div style="font-size:1.5rem; margin-bottom:0.25rem;">"📊"</div>
                                <div style="font-weight:600; font-size:0.875rem; color:var(--text-main);">"R"</div>
                                <div style="font-size:0.75rem; color:var(--text-muted); margin-top:2px;">".R"</div>
                            </button>

                            // Rust
                            <button
                                type="button"
                                style=move || {
                                    let is_sel = language.get() == "rust";
                                    if is_sel {
                                        "padding:0.75rem 0.5rem; border-radius:8px; text-align:center; cursor:pointer; transition:all 0.15s ease; border:2px solid var(--primary); background:var(--bg-muted); box-shadow:0 2px 8px rgba(0,0,0,0.08);"
                                    } else {
                                        "padding:0.75rem 0.5rem; border-radius:8px; text-align:center; cursor:pointer; transition:all 0.15s ease; border:1px solid var(--border-subtle); background:var(--bg-surface); opacity:0.85;"
                                    }
                                }
                                on:click=move |_| language.set("rust".to_string())
                            >
                                <div style="font-size:1.5rem; margin-bottom:0.25rem;">"🦀"</div>
                                <div style="font-weight:600; font-size:0.875rem; color:var(--text-main);">"Rust"</div>
                                <div style="font-size:0.75rem; color:var(--text-muted); margin-top:2px;">".rs"</div>
                            </button>
                        </div>
                        <div style="margin-top:0.45rem; font-size:0.78rem; color:var(--text-muted);">
                            {move || match language.get().as_str() {
                                "r" => "📊 Statistical analysis script with ggplot2 support (executed with Rscript).",
                                "rust" => "🦀 High-performance standalone Rust script (compiled with rustc).",
                                _ => "🐍 General-purpose analysis script with NumPy (executed with python3).",
                            }}
                        </div>
                    </div>
                })}

                <p style="font-size:0.85rem; color:var(--text-sub); margin:0 0 1.1rem;">
                    "Where should this go?"
                </p>
                <div class="form-group">
                    <label style="display:flex; align-items:center; gap:0.5rem; font-weight:500;">
                        <input
                            type="radio"
                            name="qs-mode"
                            value="new"
                            prop:checked=move || mode.get() == "new"
                            on:change=move |_| mode.set("new".to_string())
                        />
                        "Create a new project"
                    </label>
                    {move || (mode.get() == "new").then(|| {
                        let ph = if kind.get() == "script" {
                            let l = match language.get().as_str() {
                                "r" => "R Script",
                                "rust" => "Rust Script",
                                _ => "Python Script",
                            };
                            format!("Project name (optional -- defaults to {l} <timestamp>)")
                        } else {
                            "Project name (optional -- defaults to a timestamped name)".to_string()
                        };
                        view! {
                            <input
                                type="text"
                                class="form-control"
                                style="margin-top:0.5rem;"
                                placeholder=ph
                                prop:value=move || project_name.get()
                                on:input=move |ev| project_name.set(event_target_value(&ev))
                            />
                        }
                    })}
                </div>
                <div class="form-group">
                    <label style="display:flex; align-items:center; gap:0.5rem; font-weight:500;">
                        <input
                            type="radio"
                            name="qs-mode"
                            value="existing"
                            prop:checked=move || mode.get() == "existing"
                            on:change=move |_| mode.set("existing".to_string())
                        />
                        "Add to an existing project"
                    </label>
                    {move || (mode.get() == "existing").then(|| {
                        let starter_note = if kind.get() == "script" {
                            match language.get().as_str() {
                                "r" => "Will create and open script.R in the selected project workspace.",
                                "rust" => "Will create and open main.rs in the selected project workspace.",
                                _ => "Will create and open script.py in the selected project workspace.",
                            }
                        } else {
                            ""
                        };
                        view! {
                            <div>
                                <select
                                    class="form-control"
                                    style="margin-top:0.5rem;"
                                    prop:value=move || selected_project.get()
                                    on:change=move |ev| selected_project.set(event_target_value(&ev))
                                >
                                    {move || {
                                        let list = projects.get();
                                        if list.is_empty() {
                                            vec![view! { <option value="">"No projects yet"</option> }.into_any()]
                                        } else {
                                            list.into_iter()
                                                .map(|(id, name)| view! { <option value=id>{name}</option> }.into_any())
                                                .collect::<Vec<_>>()
                                        }
                                    }}
                                </select>
                                {(!starter_note.is_empty()).then(|| view! {
                                    <div style="margin-top:0.35rem; font-size:0.78rem; color:var(--text-muted);">
                                        {starter_note}
                                    </div>
                                })}
                            </div>
                        }
                    })}
                </div>
                {move || error.get().map(|e| view! {
                    <div class="alert alert-danger" style="font-size:0.8rem; padding:0.5rem 0.75rem;">{e}</div>
                })}
                <div style="display:flex; justify-content:flex-end; gap:0.75rem; margin-top:1.5rem;">
                    <button type="button" class="btn btn-secondary" on:click=move |_| open.set(false)>"Cancel"</button>
                    <button type="button" class="btn btn-primary" disabled=move || busy.get() on:click=submit>
                        {move || if busy.get() { "Creating..." } else { "Create" }}
                    </button>
                </div>
            </div>
        </div>
    }
}

struct QuickStartRequest {
    kind: String,
    language: String,
    mode: String,
    name: String,
    project_id: String,
    busy: RwSignal<bool>,
    error: RwSignal<Option<String>>,
}

#[cfg(feature = "hydrate")]
fn load_projects(
    projects: RwSignal<Vec<(String, String)>>,
    selected_project: RwSignal<String>,
) {
    wasm_bindgen_futures::spawn_local(async move {
        let Ok(resp) = gloo_net::http::Request::get("/projects/mine.json").send().await else {
            return;
        };
        let Ok(data) = resp.json::<serde_json::Value>().await else {
            return;
        };
        let list: Vec<(String, String)> = data
            .get("projects")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|p| {
                        Some((
                            p.get("id")?.as_str()?.to_string(),
                            p.get("name")?.as_str()?.to_string(),
                        ))
                    })
                    .collect()
            })
            .unwrap_or_default();
        // Default the picker to the first project, so submitting without touching the dropdown
        // does the obvious thing instead of erroring on an empty selection.
        if selected_project.get_untracked().is_empty() {
            if let Some((id, _)) = list.first() {
                selected_project.set(id.clone());
            }
        }
        projects.set(list);
    });
}
#[cfg(not(feature = "hydrate"))]
const fn load_projects(
    _projects: RwSignal<Vec<(String, String)>>,
    _selected_project: RwSignal<String>,
) {
}

#[cfg(feature = "hydrate")]
fn create_quick_start(req: QuickStartRequest) {
    let QuickStartRequest {
        kind,
        language,
        mode,
        name,
        project_id,
        busy,
        error,
    } = req;
    wasm_bindgen_futures::spawn_local(async move {
        let body = serde_json::json!({
            "kind": kind,
            "language": language,
            "mode": mode,
            "name": name,
            "project_id": project_id,
        });
        let result = gloo_net::http::Request::post("/projects/quick-start")
            .json(&body)
            .expect("valid json body")
            .send()
            .await;

        let Ok(resp) = result else {
            error.set(Some("Network error -- please try again.".to_string()));
            busy.set(false);
            return;
        };
        let Ok(data) = resp.json::<serde_json::Value>().await else {
            error.set(Some("Unexpected server response.".to_string()));
            busy.set(false);
            return;
        };
        if let Some(url) = data.get("url").and_then(|v| v.as_str()) {
            if let Some(win) = web_sys::window() {
                let _ = win.location().set_href(url);
            }
            return;
        }
        let msg = data
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("Could not create it.")
            .to_string();
        error.set(Some(msg));
        busy.set(false);
    });
}
#[cfg(not(feature = "hydrate"))]
fn create_quick_start(req: QuickStartRequest) {
    req.busy.set(false);
}
