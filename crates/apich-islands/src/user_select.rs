//! User selection island with dynamic search and autocomplete dropdown.
//!
//! Replaces static full-user `<select>` menus, querying `/users/search.json` dynamically so
//! platform instances with thousands of users can select users quickly without loading all
//! users into initial page HTML.

use leptos::prelude::*;
use serde::Deserialize;
use serde::Serialize;

/// Minimal user item returned from `/users/search.json`.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct UserItem {
    /// User UUID string.
    pub id: String,
    /// Username handle.
    pub username: String,
    /// Human-readable display name.
    pub display_name: String,
    /// Optional avatar image URL.
    #[serde(default)]
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[allow(dead_code)]
struct SearchResponse {
    #[serde(default)]
    pub users: Vec<UserItem>,
}

#[island]
pub fn UserSelectIsland(
    #[prop(into)] name: String,
    #[prop(into, optional)] placeholder: Option<String>,
    #[prop(optional)] exclude_user_ids: Option<Vec<String>>,
) -> impl IntoView {
    let selected_id = RwSignal::new(String::new());
    let selected_username = RwSignal::new(String::new());
    let selected_name = RwSignal::new(String::new());
    let search_text = RwSignal::new(String::new());
    let results = RwSignal::new(Vec::<UserItem>::new());
    let is_open = RwSignal::new(false);
    let is_loading = RwSignal::new(false);

    let placeholder_text = placeholder.unwrap_or_else(|| "Type username or display name...".to_string());
    let exclude_user_ids = exclude_user_ids.unwrap_or_default();

    let on_input = move |ev| {
        let val = event_target_value(&ev);
        search_text.set(val.clone());
        is_open.set(true);
        fetch_users(val, results, is_loading);
    };

    let on_focus = move |_| {
        is_open.set(true);
        fetch_users(search_text.get(), results, is_loading);
    };

    let clear_selection = move |_| {
        selected_id.set(String::new());
        selected_username.set(String::new());
        selected_name.set(String::new());
        search_text.set(String::new());
        is_open.set(true);
        fetch_users(String::new(), results, is_loading);
    };

    let has_selection = move || !selected_id.get().is_empty();

    view! {
        <div style="position:relative; width:100%;">
            // Hidden input holding the selected user ID for the parent form
            <input
                type="hidden"
                name=name
                prop:value=move || selected_id.get()
                required=true
            />

            // Backdrop to close dropdown when clicking outside
            {move || {
                if is_open.get() && !has_selection() {
                    view! {
                        <div
                            style="position:fixed; inset:0; z-index:1040; background:transparent;"
                            on:click=move |_| is_open.set(false)
                        />
                    }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }
            }}

            // Selected User View
            <div style:display=move || if has_selection() { "flex" } else { "none" }
                style="align-items:center; justify-content:space-between; gap:0.5rem; background:var(--bg-muted, #f1f5f9); border:1px solid var(--border-color, #cbd5e1); border-radius:6px; padding:0.35rem 0.65rem; min-height:38px; width:100%; box-sizing:border-box;"
            >
                <div style="display:flex; align-items:center; gap:0.5rem; overflow:hidden;">
                    <span style="display:inline-flex; align-items:center; justify-content:center; width:24px; height:24px; border-radius:50%; background:var(--primary, #2563eb); color:#fff; font-size:0.75rem; font-weight:bold; flex-shrink:0;">
                        {move || selected_name.get().chars().next().unwrap_or('U').to_uppercase().to_string()}
                    </span>
                    <span style="font-weight:600; font-size:0.875rem; text-overflow:ellipsis; overflow:hidden; white-space:nowrap; color:var(--text-main, inherit);">
                        {move || selected_name.get()}
                    </span>
                    <span style="font-size:0.8rem; color:var(--text-sub, #64748b); white-space:nowrap;">
                        "(@" {move || selected_username.get()} ")"
                    </span>
                </div>
                <button
                    type="button"
                    title="Change selected user"
                    style="background:none; border:none; cursor:pointer; color:var(--text-sub, #64748b); font-size:1.2rem; line-height:1; padding:0 4px; border-radius:4px;"
                    on:click=clear_selection
                >
                    "×"
                </button>
            </div>

            // Search Input View
            <div style:display=move || if has_selection() { "none" } else { "block" } style="position:relative; width:100%;">
                <input
                    type="text"
                    class="form-control"
                    placeholder=placeholder_text
                    autocomplete="off"
                    prop:value=move || search_text.get()
                    on:input=on_input
                    on:focus=on_focus
                />

                // Floating Dropdown Menu
                {move || {
                    if !is_open.get() || has_selection() {
                        return view! { <div></div> }.into_any();
                    }

                    let user_items = results.get();
                    let loading = is_loading.get();
                    let excludes = exclude_user_ids.clone();

                    view! {
                        <div
                            class="user-autocomplete-dropdown"
                            style="position:absolute; top:calc(100% + 4px); left:0; right:0; max-height:220px; overflow-y:auto; background:var(--bg-surface, #ffffff); border:1px solid var(--border-color, #cbd5e1); border-radius:8px; box-shadow:0 10px 25px -5px rgba(0,0,0,0.15), 0 8px 10px -6px rgba(0,0,0,0.1); z-index:1050; padding:4px 0;"
                        >
                            {if loading && user_items.is_empty() {
                                view! {
                                    <div style="padding:0.6rem 1rem; font-size:0.85rem; color:var(--text-sub, #64748b); text-align:center;">
                                        "Searching users..."
                                    </div>
                                }.into_any()
                            } else if user_items.is_empty() {
                                view! {
                                    <div style="padding:0.6rem 1rem; font-size:0.85rem; color:var(--text-sub, #64748b); text-align:center;">
                                        "No users found"
                                    </div>
                                }.into_any()
                            } else {
                                let rows: Vec<_> = user_items.into_iter().map(|u| {
                                    let is_excluded = excludes.contains(&u.id);
                                    let u_id = u.id.clone();
                                    let u_username = u.username.clone();
                                    let u_name = u.display_name.clone();
                                    let initial = u.display_name.chars().next().unwrap_or('U').to_uppercase().to_string();

                                    view! {
                                        <div
                                            style=if is_excluded {
                                                "display:flex; align-items:center; gap:0.6rem; padding:0.5rem 0.75rem; opacity:0.5; cursor:not-allowed;"
                                            } else {
                                                "display:flex; align-items:center; gap:0.6rem; padding:0.5rem 0.75rem; cursor:pointer; transition:background 0.15s ease;"
                                            }
                                            class=if is_excluded { "" } else { "user-dropdown-item" }
                                            on:click=move |_| {
                                                if !is_excluded {
                                                    selected_id.set(u_id.clone());
                                                    selected_username.set(u_username.clone());
                                                    selected_name.set(u_name.clone());
                                                    is_open.set(false);
                                                }
                                            }
                                        >
                                            <span style="display:inline-flex; align-items:center; justify-content:center; width:26px; height:26px; border-radius:50%; background:var(--primary, #2563eb); color:#fff; font-size:0.75rem; font-weight:bold; flex-shrink:0;">
                                                {initial}
                                            </span>
                                            <div style="display:flex; flex-direction:column; overflow:hidden; flex:1;">
                                                <span style="font-weight:600; font-size:0.875rem; color:var(--text-main, inherit); overflow:hidden; text-overflow:ellipsis; white-space:nowrap;">
                                                    {u.display_name}
                                                </span>
                                                <span style="font-size:0.775rem; color:var(--text-sub, #64748b);">
                                                    "@" {u.username}
                                                </span>
                                            </div>
                                            {if is_excluded {
                                                view! {
                                                    <span style="font-size:0.75rem; color:var(--text-sub, #64748b); font-style:italic;">
                                                        "Already a member"
                                                    </span>
                                                }.into_any()
                                            } else {
                                                view! { <span></span> }.into_any()
                                            }}
                                        </div>
                                    }
                                }).collect();
                                view! { <div>{rows}</div> }.into_any()
                            }}
                        </div>
                    }.into_any()
                }}
            </div>
        </div>
    }
}

#[cfg(feature = "hydrate")]
fn fetch_users(query: String, results: RwSignal<Vec<UserItem>>, is_loading: RwSignal<bool>) {
    wasm_bindgen_futures::spawn_local(async move {
        is_loading.set(true);
        let url = format!(
            "/users/search.json?q={}&limit=10",
            urlencoding::encode(&query)
        );
        if let Ok(resp) = gloo_net::http::Request::get(&url).send().await {
            if let Ok(data) = resp.json::<SearchResponse>().await {
                results.set(data.users);
            }
        }
        is_loading.set(false);
    });
}

#[cfg(not(feature = "hydrate"))]
fn fetch_users(_query: String, _results: RwSignal<Vec<UserItem>>, _is_loading: RwSignal<bool>) {}
