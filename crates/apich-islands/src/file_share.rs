//! Real Rust replacement for the file-share modal's hand-written JS (`components/mod.rs`'s
//! `FileShareModal`). Per-file "Share" buttons live on several different pages (files list,
//! note editor, document editor) outside this island, so they open it by dispatching a `window`
//! `CustomEvent` (`apich-open-share-modal`, with `path`/`mode`/`role`/`users` in `detail`) --
//! the same minimal cross-boundary bridge used for the AI drawer's toggle button.

use leptos::prelude::*;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareableUser {
    pub username: String,
    pub display_name: String,
}

#[island]
pub fn FileShareModalIsland(
    #[prop(into)] project_id: String,
    #[prop(into)] redirect_to: String,
    all_users: Vec<ShareableUser>,
) -> impl IntoView {
    let visible = RwSignal::new(false);
    let file_path = RwSignal::new(String::new());
    let mode = RwSignal::new("private".to_string());
    let role = RwSignal::new("read".to_string());
    let selected_users = RwSignal::new(Vec::<String>::new());

    wire_open_listener(visible, file_path, mode, role, selected_users);

    let user_checkboxes: Vec<_> = all_users
        .into_iter()
        .map(|u| {
            let username = u.username.clone();
            let username_toggle = username.clone();
            let is_checked = move || selected_users.with(|s| s.contains(&username));
            view! {
                <label style="display:flex; align-items:center; gap:0.5rem; font-size:0.8rem; padding:3px 0;">
                    <input
                        type="checkbox"
                        prop:checked=is_checked
                        on:change=move |_| {
                            selected_users.update(|s| {
                                if let Some(pos) = s.iter().position(|x| *x == username_toggle) {
                                    s.remove(pos);
                                } else {
                                    s.push(username_toggle.clone());
                                }
                            });
                        }
                    />
                    <span>{u.display_name}" (@"{u.username}")"</span>
                </label>
            }
        })
        .collect();

    view! {
        <div class="modal-backdrop" style:display=move || if visible.get() { "flex" } else { "none" }>
            <div class="modal-card">
                <div class="modal-header">
                    <h3 class="modal-title">"🔗 File Sharing: " <code>{move || file_path.get()}</code></h3>
                    <button type="button" class="modal-close" on:click=move |_| visible.set(false)>"×"</button>
                </div>
                <form method="post" action=format!("/projects/{}/files/share", project_id)>
                    <input type="hidden" name="file" prop:value=move || file_path.get() />
                    <input type="hidden" name="redirect_to" value=redirect_to />
                    <input type="hidden" name="allowed_users" prop:value=move || selected_users.get().join(",") />

                    <div class="form-group">
                        <label style="font-weight:700;">"Sharing Mode"</label>
                        <div style="display:flex; flex-direction:column; gap:0.5rem;">
                            <label style="display:flex; align-items:center; gap:0.6rem; font-size:0.875rem;">
                                <input type="radio" name="mode" value="public" prop:checked=move || mode.get() == "public" on:change=move |_| mode.set("public".to_string()) />
                                "🌐 Public — anyone with the link"
                            </label>
                            <label style="display:flex; align-items:center; gap:0.6rem; font-size:0.875rem;">
                                <input type="radio" name="mode" value="specific" prop:checked=move || mode.get() == "specific" on:change=move |_| mode.set("specific".to_string()) />
                                "👥 Specific accounts only"
                            </label>
                            <label style="display:flex; align-items:center; gap:0.6rem; font-size:0.875rem;">
                                <input type="radio" name="mode" value="private" prop:checked=move || mode.get() == "private" on:change=move |_| mode.set("private".to_string()) />
                                "🔒 Private — repository members only"
                            </label>
                        </div>
                    </div>

                    <div class="form-group">
                        <label style="font-weight:700;">"Permission Level"</label>
                        <select name="role" class="form-control" prop:value=move || role.get() on:change=move |ev| role.set(event_target_value(&ev))>
                            <option value="read">"📖 Read Only"</option>
                            <option value="review">"📝 Read & Review"</option>
                            <option value="write">"✏️ Read, Write & Review"</option>
                        </select>
                    </div>

                    <div style:display=move || if mode.get() == "specific" { "block" } else { "none" } style="background:var(--bg-muted); border:1px solid var(--border-subtle); border-radius:8px; padding:0.75rem 1rem;">
                        <div style="font-weight:600; font-size:0.775rem; color:var(--text-sub); margin-bottom:0.5rem;">"Allowed Users:"</div>
                        <div style="max-height:120px; overflow-y:auto;">{user_checkboxes}</div>
                    </div>

                    <div style="display:flex; justify-content:flex-end; gap:0.5rem; margin-top:1.25rem;">
                        <button type="button" class="btn btn-secondary" on:click=move |_| visible.set(false)>"Cancel"</button>
                        <button type="submit" class="btn btn-primary">"Save Sharing"</button>
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
fn wire_open_listener(
    _visible: RwSignal<bool>,
    _file_path: RwSignal<String>,
    _mode: RwSignal<String>,
    _role: RwSignal<String>,
    _selected_users: RwSignal<Vec<String>>,
) {
}
