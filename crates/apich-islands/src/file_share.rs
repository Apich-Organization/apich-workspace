//! File sharing modal island for project documents and notes.
//!
//! Real Rust replacement for the file-share modal's hand-written JS (`components/mod.rs`'s
//! `FileShareModal`). Per-file "Share" buttons live on several different pages (files list,
//! note editor, document editor) outside this island, so they open it by dispatching a `window`
//! `CustomEvent` (`apich-open-share-modal`, with `path`/`mode`/`role`/`users` in `detail`) --
//! the same minimal cross-boundary bridge used for the AI drawer's toggle button.

use leptos::prelude::*;
use serde::Deserialize;
use serde::Serialize;

/// Candidate user record that can be granted access in the file share dialog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareableUser {
    /// Account username.
    pub username: String,
    /// Human-readable display name.
    pub display_name: String,
}

#[island]
pub fn FileShareModalIsland(
    #[prop(into)] project_id: String,
    #[prop(into)] redirect_to: String,
    #[prop(optional)] all_users: Option<Vec<ShareableUser>>,
) -> impl IntoView {
    let _ = all_users;
    let visible = RwSignal::new(false);
    let file_path = RwSignal::new(String::new());
    let mode = RwSignal::new("private".to_string());
    let role = RwSignal::new("read".to_string());
    let selected_users = RwSignal::new(Vec::<String>::new());

    let search_query = RwSignal::new(String::new());
    let search_results = RwSignal::new(Vec::<crate::user_select::UserItem>::new());
    let is_search_loading = RwSignal::new(false);

    wire_open_listener(
        visible,
        file_path,
        mode,
        role,
        selected_users,
        search_results,
        is_search_loading,
    );

    view! {
        <div
            class="modal-backdrop"
            style:display=move || if visible.get() { "flex" } else { "none" }
        >
            <div
                class="modal-card"
                style="max-width:620px; width:min(620px, 95vw); border-radius:14px; padding:1.75rem 2rem; max-height:88vh; display:flex; flex-direction:column; box-shadow:0 25px 50px -12px rgba(0,0,0,0.25);"
            >
                // Modal Header
                <div class="modal-header" style="align-items:flex-start; margin-bottom:1.25rem; flex-shrink:0;">
                    <div>
                        <h3 class="modal-title" style="display:flex; align-items:center; gap:0.5rem; font-size:1.2rem; margin:0;">
                            <span>"🔗"</span>
                            <span>"File Sharing & Access"</span>
                        </h3>
                        <div style="font-size:0.8rem; color:var(--text-sub); margin-top:0.35rem; display:flex; align-items:center; gap:0.35rem;">
                            <span>"File:"</span>
                            <code style="background:var(--bg-muted, #f1f5f9); color:var(--primary, #635bff); padding:2px 6px; border-radius:4px; font-weight:600; font-size:0.82rem;">
                                {move || file_path.get()}
                            </code>
                        </div>
                    </div>
                    <button
                        type="button"
                        class="modal-close"
                        style="font-size:1.6rem; line-height:1; padding:0 4px; cursor:pointer;"
                        on:click=move |_| visible.set(false)
                    >
                        "×"
                    </button>
                </div>

                // Modal Body (Scrollable form)
                <form
                    method="post"
                    action=format!("/projects/{}/files/share", project_id)
                    style="display:flex; flex-direction:column; flex:1; overflow-y:auto; padding-right:2px;"
                >
                    <input type="hidden" name="file" prop:value=move || file_path.get() />
                    <input type="hidden" name="redirect_to" value=redirect_to />
                    <input type="hidden" name="allowed_users" prop:value=move || selected_users.get().join(",") />
                    <input type="hidden" name="mode" prop:value=move || mode.get() />

                    // 1. Sharing Mode Cards
                    <div class="form-group" style="margin-bottom:1.25rem;">
                        <label style="font-weight:700; font-size:0.875rem; margin-bottom:0.5rem; display:block; color:var(--text-main);">
                            "Sharing Mode"
                        </label>
                        <div style="display:grid; grid-template-columns:repeat(auto-fit, minmax(160px, 1fr)); gap:0.6rem;">
                            // Private
                            <div
                                style=move || {
                                    let active = mode.get() == "private";
                                    format!(
                                        "display:flex; flex-direction:column; gap:0.25rem; padding:0.75rem 0.85rem; border:2px solid {}; border-radius:10px; cursor:pointer; background:{}; transition:all 0.15s ease;",
                                        if active { "var(--primary, #635bff)" } else { "var(--border-subtle, #e2e8f0)" },
                                        if active { "var(--primary-light, #f0efff)" } else { "var(--bg-surface, #ffffff)" }
                                    )
                                }
                                on:click=move |_| mode.set("private".to_string())
                            >
                                <div style="display:flex; align-items:center; gap:0.4rem; font-weight:700; font-size:0.875rem; color:var(--text-main);">
                                    <span>"🔒"</span>
                                    <span>"Private"</span>
                                </div>
                                <div style="font-size:0.75rem; color:var(--text-sub); line-height:1.3;">
                                    "Project members only"
                                </div>
                            </div>

                            // Specific Accounts
                            <div
                                style=move || {
                                    let active = mode.get() == "specific";
                                    format!(
                                        "display:flex; flex-direction:column; gap:0.25rem; padding:0.75rem 0.85rem; border:2px solid {}; border-radius:10px; cursor:pointer; background:{}; transition:all 0.15s ease;",
                                        if active { "var(--primary, #635bff)" } else { "var(--border-subtle, #e2e8f0)" },
                                        if active { "var(--primary-light, #f0efff)" } else { "var(--bg-surface, #ffffff)" }
                                    )
                                }
                                on:click=move |_| {
                                    mode.set("specific".to_string());
                                    fetch_share_users(search_query.get(), search_results, is_search_loading);
                                }
                            >
                                <div style="display:flex; align-items:center; gap:0.4rem; font-weight:700; font-size:0.875rem; color:var(--text-main);">
                                    <span>"👥"</span>
                                    <span>"Specific Accounts"</span>
                                </div>
                                <div style="font-size:0.75rem; color:var(--text-sub); line-height:1.3;">
                                    "Designated users only"
                                </div>
                            </div>

                            // Public
                            <div
                                style=move || {
                                    let active = mode.get() == "public";
                                    format!(
                                        "display:flex; flex-direction:column; gap:0.25rem; padding:0.75rem 0.85rem; border:2px solid {}; border-radius:10px; cursor:pointer; background:{}; transition:all 0.15s ease;",
                                        if active { "var(--primary, #635bff)" } else { "var(--border-subtle, #e2e8f0)" },
                                        if active { "var(--primary-light, #f0efff)" } else { "var(--bg-surface, #ffffff)" }
                                    )
                                }
                                on:click=move |_| mode.set("public".to_string())
                            >
                                <div style="display:flex; align-items:center; gap:0.4rem; font-weight:700; font-size:0.875rem; color:var(--text-main);">
                                    <span>"🌐"</span>
                                    <span>"Public Link"</span>
                                </div>
                                <div style="font-size:0.75rem; color:var(--text-sub); line-height:1.3;">
                                    "Anyone with link"
                                </div>
                            </div>
                        </div>
                    </div>

                    // 2. Permission Level
                    <div class="form-group" style="margin-bottom:1.25rem;">
                        <label style="font-weight:700; font-size:0.875rem; margin-bottom:0.4rem; display:block; color:var(--text-main);">
                            "Permission Level"
                        </label>
                        <select
                            name="role"
                            class="form-control"
                            style="height:38px; font-size:0.875rem; width:100%;"
                            prop:value=move || role.get()
                            on:change=move |ev| role.set(event_target_value(&ev))
                        >
                            <option value="read">"📖 Read Only — Viewer cannot edit"</option>
                            <option value="review">"📝 Read & Review — Can view and add comments"</option>
                            <option value="write">"✏️ Read, Write & Review — Full collaborative edit access"</option>
                        </select>
                    </div>

                    // 3. Specific Users Panel (Inline list, no backdrop, scrollable!)
                    <div
                        style:display=move || if mode.get() == "specific" { "block" } else { "none" }
                        style="background:var(--bg-muted, #f8fafc); border:1px solid var(--border-subtle, #e2e8f0); border-radius:10px; padding:1rem; margin-bottom:1rem;"
                    >
                        // Collaborators list header & chips
                        <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:0.5rem;">
                            <span style="font-weight:700; font-size:0.85rem; color:var(--text-main);">
                                "Designated Collaborators"
                            </span>
                            <span style="font-size:0.775rem; color:var(--text-sub); background:var(--bg-surface, #fff); padding:2px 8px; border-radius:10px; border:1px solid var(--border-subtle, #cbd5e1);">
                                {move || selected_users.get().len()} " selected"
                            </span>
                        </div>

                        // Selected chips
                        <div style="display:flex; flex-wrap:wrap; gap:0.4rem; margin-bottom:0.85rem; min-height:28px; align-items:center;">
                            {move || {
                                let users = selected_users.get();
                                if users.is_empty() {
                                    view! {
                                        <span style="font-size:0.8rem; color:var(--text-sub); font-style:italic;">
                                            "No users added yet. Search below to add users."
                                        </span>
                                    }.into_any()
                                } else {
                                    let chips: Vec<_> = users.into_iter().map(|uname| {
                                        let u_toggle = uname.clone();
                                        view! {
                                            <span style="display:inline-flex; align-items:center; gap:0.35rem; background:var(--bg-surface, #ffffff); border:1px solid var(--border-strong, #cbd5e1); border-radius:16px; padding:3px 10px; font-size:0.82rem; font-weight:600; color:var(--text-main); box-shadow:0 1px 2px rgba(0,0,0,0.05);">
                                                <span>"@" {uname}</span>
                                                <button
                                                    type="button"
                                                    title="Remove user"
                                                    style="background:none; border:none; cursor:pointer; color:var(--text-sub); font-size:1.1rem; line-height:1; padding:0 2px; border-radius:3px;"
                                                    on:click=move |_| {
                                                        selected_users.update(|s| {
                                                            if let Some(pos) = s.iter().position(|x| *x == u_toggle) {
                                                                s.remove(pos);
                                                            }
                                                        });
                                                    }
                                                >
                                                    "×"
                                                </button>
                                            </span>
                                        }
                                    }).collect();
                                    view! { <div style="display:flex; flex-wrap:wrap; gap:0.4rem;">{chips}</div> }.into_any()
                                }
                            }}
                        </div>

                        // Search input
                        <div style="margin-top:0.4rem;">
                            <label style="font-size:0.775rem; font-weight:600; color:var(--text-sub); margin-bottom:0.35rem; display:block;">
                                "Search & Add Users:"
                            </label>
                            <div style="display:flex; gap:0.5rem; align-items:center;">
                                <input
                                    type="text"
                                    class="form-control"
                                    placeholder="Type username or name to search..."
                                    autocomplete="off"
                                    style="flex:1; height:38px; font-size:0.875rem;"
                                    prop:value=move || search_query.get()
                                    on:input=move |ev| {
                                        let val = event_target_value(&ev);
                                        search_query.set(val.clone());
                                        fetch_share_users(val, search_results, is_search_loading);
                                    }
                                    on:focus=move |_| {
                                        fetch_share_users(search_query.get(), search_results, is_search_loading);
                                    }
                                />
                                {move || {
                                    if !search_query.get().is_empty() {
                                        view! {
                                            <button
                                                type="button"
                                                class="btn btn-secondary btn-sm"
                                                style="height:38px; white-space:nowrap;"
                                                on:click=move |_| {
                                                    search_query.set(String::new());
                                                    fetch_share_users(String::new(), search_results, is_search_loading);
                                                }
                                            >
                                                "Clear"
                                            </button>
                                        }.into_any()
                                    } else {
                                        view! { <span></span> }.into_any()
                                    }
                                }}
                            </div>
                        </div>

                        // Inline scrollable search results list (NEVER blocked, NO backdrop!)
                        <div style="margin-top:0.6rem; border:1px solid var(--border-subtle, #cbd5e1); border-radius:8px; background:var(--bg-surface, #ffffff); max-height:190px; overflow-y:auto; padding:2px 0;">
                            {move || {
                                let items = search_results.get();
                                let loading = is_search_loading.get();
                                let current_selected = selected_users.get();

                                if loading && items.is_empty() {
                                    view! {
                                        <div style="padding:1rem; font-size:0.85rem; color:var(--text-sub); text-align:center;">
                                            "Searching users..."
                                        </div>
                                    }.into_any()
                                } else if items.is_empty() {
                                    view! {
                                        <div style="padding:1rem; font-size:0.85rem; color:var(--text-sub); text-align:center;">
                                            "No users found. Type a name or username above to search."
                                        </div>
                                    }.into_any()
                                } else {
                                    let rows: Vec<_> = items.into_iter().map(|u| {
                                        let is_already = current_selected.contains(&u.username);
                                        let uname = u.username.clone();
                                        let initial = u.display_name.chars().next().unwrap_or('U').to_uppercase().to_string();

                                        view! {
                                            <div
                                                style="display:flex; align-items:center; justify-content:space-between; gap:0.6rem; padding:0.45rem 0.85rem; border-bottom:1px solid var(--border-subtle, #f1f5f9); transition:background 0.15s ease;"
                                                class="user-dropdown-item"
                                            >
                                                <div style="display:flex; align-items:center; gap:0.6rem; overflow:hidden; flex:1;">
                                                    <span style="display:inline-flex; align-items:center; justify-content:center; width:28px; height:28px; border-radius:50%; background:var(--primary, #635bff); color:#fff; font-size:0.75rem; font-weight:700; flex-shrink:0;">
                                                        {initial}
                                                    </span>
                                                    <div style="display:flex; flex-direction:column; overflow:hidden;">
                                                        <span style="font-weight:600; font-size:0.85rem; color:var(--text-main, inherit); overflow:hidden; text-overflow:ellipsis; white-space:nowrap;">
                                                            {u.display_name}
                                                        </span>
                                                        <span style="font-size:0.75rem; color:var(--text-sub, #64748b);">
                                                            "@" {u.username}
                                                        </span>
                                                    </div>
                                                </div>
                                                {if is_already {
                                                    view! {
                                                        <span style="font-size:0.775rem; color:var(--success, #22c55e); font-weight:600; display:inline-flex; align-items:center; gap:0.25rem;">
                                                            "✓ Added"
                                                        </span>
                                                    }.into_any()
                                                } else {
                                                    view! {
                                                        <button
                                                            type="button"
                                                            class="btn btn-secondary btn-sm"
                                                            style="font-size:0.775rem; padding:0.25rem 0.75rem;"
                                                            on:click=move |_| {
                                                                selected_users.update(|s| {
                                                                    if !s.contains(&uname) {
                                                                        s.push(uname.clone());
                                                                    }
                                                                });
                                                            }
                                                        >
                                                            "+ Add"
                                                        </button>
                                                    }.into_any()
                                                }}
                                            </div>
                                        }
                                    }).collect();
                                    view! { <div>{rows}</div> }.into_any()
                                }
                            }}
                        </div>
                    </div>

                    // Modal Footer
                    <div style="display:flex; justify-content:flex-end; gap:0.6rem; margin-top:1.25rem; padding-top:1rem; border-top:1px solid var(--border-subtle, #e2e8f0); flex-shrink:0;">
                        <button
                            type="button"
                            class="btn btn-secondary"
                            on:click=move |_| visible.set(false)
                        >
                            "Cancel"
                        </button>
                        <button type="submit" class="btn btn-primary">
                            "Save Sharing"
                        </button>
                    </div>
                </form>
            </div>
        </div>
    }
}

#[cfg(feature = "hydrate")]
fn wire_open_listener(
    visible: RwSignal<bool>,
    file_path: RwSignal<String>,
    mode: RwSignal<String>,
    role: RwSignal<String>,
    selected_users: RwSignal<Vec<String>>,
    search_results: RwSignal<Vec<crate::user_select::UserItem>>,
    is_search_loading: RwSignal<bool>,
) {
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;

    let closure = Closure::<dyn Fn(web_sys::CustomEvent)>::new(move |ev: web_sys::CustomEvent| {
        let detail = ev.detail();
        let get_str = |key: &str| -> String {
            js_sys::Reflect::get(&detail, &key.into())
                .ok()
                .and_then(|v| v.as_string())
                .unwrap_or_default()
        };
        file_path.set(get_str("path"));
        let m = get_str("mode");
        mode.set(if m.is_empty() {
            "private".to_string()
        } else {
            m
        });
        let r = get_str("role");
        role.set(if r.is_empty() {
            "read".to_string()
        } else {
            r
        });
        let users_csv = get_str("users");
        selected_users.set(
            users_csv
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
        );
        fetch_share_users(String::new(), search_results, is_search_loading);
        visible.set(true);
    });
    if let Some(win) = web_sys::window() {
        let _ = win.add_event_listener_with_callback(
            "apich-open-share-modal",
            closure.as_ref().unchecked_ref(),
        );
    }
    closure.forget();
}

#[cfg(not(feature = "hydrate"))]
const fn wire_open_listener(
    _visible: RwSignal<bool>,
    _file_path: RwSignal<String>,
    _mode: RwSignal<String>,
    _role: RwSignal<String>,
    _selected_users: RwSignal<Vec<String>>,
    _search_results: RwSignal<Vec<crate::user_select::UserItem>>,
    _is_search_loading: RwSignal<bool>,
) {
}

#[cfg(feature = "hydrate")]
fn fetch_share_users(
    query: String,
    results: RwSignal<Vec<crate::user_select::UserItem>>,
    is_loading: RwSignal<bool>,
) {
    wasm_bindgen_futures::spawn_local(async move {
        is_loading.set(true);
        let url = format!(
            "/users/search.json?q={}&limit=30",
            urlencoding::encode(&query)
        );
        if let Ok(resp) = gloo_net::http::Request::get(&url).send().await {
            if let Ok(data) = resp.json::<serde_json::Value>().await {
                if let Some(arr) = data.get("users").and_then(|v| v.as_array()) {
                    let items: Vec<crate::user_select::UserItem> = arr
                        .iter()
                        .filter_map(|val| serde_json::from_value(val.clone()).ok())
                        .collect();
                    results.set(items);
                }
            }
        }
        is_loading.set(false);
    });
}

#[cfg(not(feature = "hydrate"))]
fn fetch_share_users(
    _query: String,
    _results: RwSignal<Vec<crate::user_select::UserItem>>,
    _is_loading: RwSignal<bool>,
) {
}
