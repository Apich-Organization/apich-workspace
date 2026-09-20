//! Search and replace modal island for searching and replacing text across single or multiple files in a project.
//!
//! Provides:
//! - Flexible scope: Single file or Project-wide (all files or filtered by extension)
//! - Matching options: Case sensitive (`Aa`), Match whole word (`\b`), Regular expression (`.*`)
//! - Interactive preview: Accordion of matching files, line numbers, match highlights, and preview diffs
//! - Granular selection: Checkboxes to selectively include/exclude files before executing replacement
//! - Automatic VCS snapshotting upon replacement execution

use leptos::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashSet;
use std::rc::Rc;
use crate::t;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// A single occurrence of a matched line in a file.
pub struct SearchOccurrenceData {
    /// 1-indexed line number in the source file
    pub line_number: usize,
    /// Content of the matched line
    pub line_content: String,
    /// Exact text matched
    #[serde(default)]
    pub match_text: String,
    /// Byte/character start offset in line
    pub match_start: usize,
    /// Byte/character end offset in line
    pub match_end: usize,
    /// Optional replacement preview of the line
    pub preview_replaced: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// Search results grouped under a single file.
pub struct SearchFileData {
    /// Project-relative file path
    pub file_path: String,
    /// Number of matching occurrences
    pub match_count: usize,
    /// List of occurrences
    pub matches: Vec<SearchOccurrenceData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Server response payload for searching text.
pub struct SearchProjectResponse {
    /// Whether the search query executed successfully
    #[serde(default)]
    pub success: bool,
    /// The search query
    #[serde(default)]
    pub query: String,
    /// Total matches across all files
    #[serde(default)]
    pub total_matches: usize,
    /// Total number of files with matches
    #[serde(default)]
    pub files_count: usize,
    /// Results by file
    #[serde(default)]
    pub results: Vec<SearchFileData>,
    /// Error message if any
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Server response payload for replacing text.
pub struct ReplaceProjectResponse {
    /// Whether the replacement succeeded
    pub success: bool,
    /// Number of files modified
    #[serde(default)]
    pub files_modified: usize,
    /// Total occurrences replaced
    #[serde(default)]
    pub total_replacements: usize,
    /// List of modified file paths
    #[serde(default)]
    pub modified_files: Vec<String>,
    /// Error message if any
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct SearchApiPayload {
    query: String,
    replacement: Option<String>,
    case_sensitive: bool,
    whole_word: bool,
    is_regex: bool,
    file_paths: Option<Vec<String>>,
    extension_filter: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ReplaceApiPayload {
    query: String,
    replacement: String,
    case_sensitive: bool,
    whole_word: bool,
    is_regex: bool,
    file_paths: Vec<String>,
}

/// Search & Replace Modal Island for single or multiple files in a project.
#[island]
#[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
pub fn SearchReplaceModalIsland(
    #[prop(into)] project_id: String,
    #[prop(into, optional)] active_file: Option<String>,
    #[prop(into, optional)] files_json: Option<String>,
    #[prop(optional)] is_zh: Option<bool>,
    #[prop(into, optional)] trigger_label: Option<String>,
    #[prop(into, optional)] trigger_class: Option<String>,
) -> impl IntoView {
    let zh = is_zh.unwrap_or(false);
    let open = RwSignal::new(false);
    let backdrop_ref = NodeRef::<leptos::html::Div>::new();
    crate::modal::reparent_to_body(backdrop_ref);

    let parsed_files: Vec<String> = files_json
        .as_deref()
        .and_then(|raw| serde_json::from_str::<Vec<String>>(raw).ok())
        .unwrap_or_default();

    let initial_selected_file = active_file
        .clone()
        .or_else(|| parsed_files.first().cloned())
        .unwrap_or_default();

    let initial_scope = if active_file.is_some() {
        "single"
    } else {
        "all"
    };

    let query = RwSignal::new(String::new());
    let replacement = RwSignal::new(String::new());
    let case_sensitive = RwSignal::new(false);
    let whole_word = RwSignal::new(false);
    let is_regex = RwSignal::new(false);
    let scope = RwSignal::new(initial_scope.to_string()); // "all" or "single"
    let selected_file = RwSignal::new(initial_selected_file);
    let ext_filter = RwSignal::new(String::new());
    let active_tab = RwSignal::new("find".to_string()); // "find" or "replace"

    let is_searching = RwSignal::new(false);
    let is_replacing = RwSignal::new(false);
    let search_error = RwSignal::new(None::<String>);
    let action_message = RwSignal::new(None::<(bool, String)>); // (is_success, text)

    let search_response = RwSignal::new(None::<SearchProjectResponse>);
    let selected_files_for_replace = RwSignal::new(HashSet::<String>::new());
    let expanded_files = RwSignal::new(HashSet::<String>::new());

    // Wire custom event for external buttons to trigger this modal
    wire_search_replace_listener(
        open,
        active_tab,
        scope,
        selected_file,
        query,
    );

    let proj_id_for_search = project_id.clone();
    let perform_search = Rc::new(move || {
        let q = query.get();
        if q.trim().is_empty() {
            search_error.set(Some(t(zh, "Please enter a search query.", "请输入搜索内容。").to_string()));
            return;
        }

        is_searching.set(true);
        search_error.set(None);
        action_message.set(None);

        let rep_val = replacement.get();
        let rep_opt = if active_tab.get() == "replace" && !rep_val.is_empty() {
            Some(rep_val)
        } else {
            None
        };
        let cur_scope = scope.get();
        let target_paths = if cur_scope == "single" {
            let sel = selected_file.get();
            if sel.is_empty() { None } else { Some(vec![sel]) }
        } else {
            None
        };

        let ext_val = ext_filter.get().trim().to_string();
        let ext_opt = if ext_val.is_empty() { None } else { Some(ext_val) };

        let payload = SearchApiPayload {
            query: q,
            replacement: rep_opt,
            case_sensitive: case_sensitive.get(),
            whole_word: whole_word.get(),
            is_regex: is_regex.get(),
            file_paths: target_paths,
            extension_filter: ext_opt,
        };

        let endpoint = format!("/projects/{proj_id_for_search}/search-text");

        #[cfg(feature = "hydrate")]
        {
            let is_searching = is_searching;
            let search_error = search_error;
            let search_response = search_response;
            let selected_files_for_replace = selected_files_for_replace;
            let expanded_files = expanded_files;
            let _zh = zh;

            leptos::task::spawn_local(async move {
                let json_body = match serde_json::to_string(&payload) {
                    Ok(s) => s,
                    Err(e) => {
                        is_searching.set(false);
                        search_error.set(Some(format!("Failed to serialize request: {e}")));
                        return;
                    }
                };

                let res = gloo_net::http::Request::post(&endpoint)
                    .header("Content-Type", "application/json")
                    .body(json_body);

                match res {
                    Ok(req) => {
                        match req.send().await {
                            Ok(resp) => {
                                is_searching.set(false);
                                if resp.ok() {
                                    match resp.json::<SearchProjectResponse>().await {
                                        Ok(data) => {
                                            // Select all returned files by default for easy replace
                                            let mut file_set = HashSet::new();
                                            let mut expand_set = HashSet::new();
                                            for f in &data.results {
                                                file_set.insert(f.file_path.clone());
                                                expand_set.insert(f.file_path.clone());
                                            }
                                            selected_files_for_replace.set(file_set);
                                            expanded_files.set(expand_set);
                                            search_response.set(Some(data));
                                        }
                                        Err(e) => {
                                            search_error.set(Some(format!("Failed to parse response: {e}")));
                                        }
                                    }
                                } else {
                                    let err_text = resp.text().await.unwrap_or_else(|_| "Search request failed".to_string());
                                    search_error.set(Some(err_text));
                                }
                            }
                            Err(e) => {
                                is_searching.set(false);
                                search_error.set(Some(format!("Network error: {e}")));
                            }
                        }
                    }
                    Err(e) => {
                        is_searching.set(false);
                        search_error.set(Some(format!("Request error: {e}")));
                    }
                }
            });
        }
        #[cfg(not(feature = "hydrate"))]
        {
            let _ = (endpoint, payload, zh);
        }
    });

    let on_search_kd1 = {
        let ps = perform_search.clone();
        move |ev: leptos::ev::KeyboardEvent| {
            if ev.key() == "Enter" {
                ps();
            }
        }
    };
    let on_search_kd2 = {
        let ps = perform_search.clone();
        move |ev: leptos::ev::KeyboardEvent| {
            if ev.key() == "Enter" {
                ps();
            }
        }
    };
    let on_search_click = {
        let ps = perform_search.clone();
        move |_| ps()
    };

    let proj_id_for_replace = project_id.clone();
    let perform_replace = move || {
        let q = query.get();
        if q.trim().is_empty() {
            search_error.set(Some(t(zh, "Please enter a search query.", "请输入搜索内容。").to_string()));
            return;
        }

        let rep = replacement.get();
        let sel_files: Vec<String> = selected_files_for_replace.get().into_iter().collect();
        if sel_files.is_empty() {
            action_message.set(Some((false, t(zh, "Please select at least one file to replace in.", "请至少选择一个进行替换的文件。").to_string())));
            return;
        }

        is_replacing.set(true);
        action_message.set(None);

        let payload = ReplaceApiPayload {
            query: q.clone(),
            replacement: rep,
            case_sensitive: case_sensitive.get(),
            whole_word: whole_word.get(),
            is_regex: is_regex.get(),
            file_paths: sel_files,
        };

        let endpoint = format!("/projects/{proj_id_for_replace}/replace-text");

        #[cfg(feature = "hydrate")]
        {
            let is_replacing = is_replacing;
            let action_message = action_message;
            let zh = zh;
            let perform_search_after = perform_search.clone();

            leptos::task::spawn_local(async move {
                let json_body = match serde_json::to_string(&payload) {
                    Ok(s) => s,
                    Err(e) => {
                        is_replacing.set(false);
                        action_message.set(Some((false, format!("Failed to serialize request: {e}"))));
                        return;
                    }
                };

                let res = gloo_net::http::Request::post(&endpoint)
                    .header("Content-Type", "application/json")
                    .body(json_body);

                match res {
                    Ok(req) => {
                        match req.send().await {
                            Ok(resp) => {
                                is_replacing.set(false);
                                if resp.ok() {
                                    match resp.json::<ReplaceProjectResponse>().await {
                                        Ok(data) => {
                                            if data.success {
                                                let count = data.total_replacements;
                                                let files = data.files_modified;
                                                let msg = if zh {
                                                    format!("替换成功！已在 {files} 个文件中完成 {count} 处替换。")
                                                } else {
                                                    format!("Successfully replaced {count} occurrences across {files} files.")
                                                };
                                                action_message.set(Some((true, msg)));
                                                // Refresh search results to show updated state
                                                perform_search_after();
                                            } else {
                                                let err = data.error.unwrap_or_else(|| "Replace failed".to_string());
                                                action_message.set(Some((false, err)));
                                            }
                                        }
                                        Err(e) => {
                                            action_message.set(Some((false, format!("Failed to parse response: {e}"))));
                                        }
                                    }
                                } else {
                                    let err_text = resp.text().await.unwrap_or_else(|_| "Replace request failed".to_string());
                                    action_message.set(Some((false, err_text)));
                                }
                            }
                            Err(e) => {
                                is_replacing.set(false);
                                action_message.set(Some((false, format!("Network error: {e}"))));
                            }
                        }
                    }
                    Err(e) => {
                        is_replacing.set(false);
                        action_message.set(Some((false, format!("Request error: {e}"))));
                    }
                }
            });
        }
        #[cfg(not(feature = "hydrate"))]
        {
            let _ = (endpoint, payload, zh);
        }
    };

    let select_all_files = move |val: bool| {
        if let Some(resp) = search_response.get() {
            let mut set = HashSet::new();
            if val {
                for f in resp.results {
                    set.insert(f.file_path);
                }
            }
            selected_files_for_replace.set(set);
        }
    };

    let toggle_file_expanded = move |path: String| {
        expanded_files.update(|set| {
            if set.contains(&path) {
                set.remove(&path);
            } else {
                set.insert(path);
            }
        });
    };

    let toggle_file_selected = move |path: String| {
        selected_files_for_replace.update(|set| {
            if set.contains(&path) {
                set.remove(&path);
            } else {
                set.insert(path);
            }
        });
    };

    let btn_label = trigger_label.unwrap_or_else(|| if zh { "🔍 查找与替换".to_string() } else { "🔍 Search / Replace".to_string() });
    let btn_cls = trigger_class.unwrap_or_else(|| "btn btn-secondary btn-sm".to_string());
    let modal_project_id = project_id.clone();
    let current_opened_file = active_file.clone();

    view! {
        <button
            type="button"
            class=btn_cls
            title=t(zh, "Search and replace text across files in this project", "在项目中单文件或多文件查找与替换文本")
            on:click=move |_| open.set(true)
        >
            {btn_label}
        </button>

        <div
            node_ref=backdrop_ref
            class="modal-backdrop"
            style:display=move || if open.get() { "flex" } else { "none" }
        >
            <div
                class="modal-card search-replace-modal"
                style="max-width:780px; width:min(780px, 96vw); max-height:90vh; display:flex; flex-direction:column; overflow:hidden; padding:0;"
            >
                // Modal Header
                <div class="modal-header sr-modal-header">
                    <div style="display:flex; align-items:center; gap:0.6rem;">
                        <span style="font-size:1.2rem;">"🔍"</span>
                        <h3 class="modal-title" style="margin:0; font-size:1.1rem; font-weight:600; color:var(--text-main, #172033);">
                            {t(zh, "Project Text Search & Replace", "项目文本查找与替换")}
                        </h3>
                    </div>
                    <button
                        type="button"
                        class="modal-close"
                        style="background:none; border:none; font-size:1.4rem; cursor:pointer; color:var(--text-sub, #667085); line-height:1;"
                        on:click=move |_| open.set(false)
                    >
                        "×"
                    </button>
                </div>

                // Modal Body (Scrollable)
                <div style="padding:1.4rem; overflow-y:auto; flex:1; display:flex; flex-direction:column; gap:1.1rem;">
                    // Mode Switcher Tabs
                    <div class="sr-tab-group">
                        <button
                            type="button"
                            class=move || if active_tab.get() == "find" { "sr-tab-btn active" } else { "sr-tab-btn" }
                            on:click=move |_| active_tab.set("find".to_string())
                        >
                            "🔍 " {t(zh, "Find Text", "查找文本")}
                        </button>
                        <button
                            type="button"
                            class=move || if active_tab.get() == "replace" { "sr-tab-btn active" } else { "sr-tab-btn" }
                            on:click=move |_| active_tab.set("replace".to_string())
                        >
                            "🔄 " {t(zh, "Search & Replace", "查找与替换")}
                        </button>
                    </div>

                    // Notification messages
                    {move || search_error.get().map(|err| {
                        view! { <div class="alert alert-danger" style="padding:0.6rem 1rem; border-radius:8px; font-size:0.85rem; margin:0;">{err}</div> }
                    })}
                    {move || action_message.get().map(|(success, msg)| {
                        let cls = if success { "alert alert-success" } else { "alert alert-danger" };
                        view! { <div class=cls style="padding:0.6rem 1rem; border-radius:8px; font-size:0.85rem; margin:0;">{msg}</div> }
                    })}

                    // Search & Replace Inputs
                    <div style="display:grid; grid-template-columns:1fr; gap:0.9rem;">
                        // Search Row
                        <div>
                            <label class="sr-field-label">
                                {t(zh, "Find / Search Query", "查找内容")}
                            </label>
                            <div style="display:flex; gap:0.4rem; align-items:center;">
                                <input
                                    type="text"
                                    class="form-control"
                                    placeholder=t(zh, "Text or regex pattern...", "输入查找文本或正则...")
                                    prop:value=move || query.get()
                                    on:input=move |ev| query.set(event_target_value(&ev))
                                    on:keydown=on_search_kd1
                                    style="flex:1;"
                                />
                                // Option Toggles
                                <div style="display:flex; gap:0.25rem;">
                                    <button
                                        type="button"
                                        class=move || if case_sensitive.get() { "sr-opt-btn active" } else { "sr-opt-btn" }
                                        title=t(zh, "Match Case (Case Sensitive)", "区分大小写 (Aa)")
                                        on:click=move |_| case_sensitive.set(!case_sensitive.get())
                                    >
                                        "Aa"
                                    </button>
                                    <button
                                        type="button"
                                        class=move || if whole_word.get() { "sr-opt-btn active" } else { "sr-opt-btn" }
                                        title=t(zh, "Match Whole Word (\\b)", "全词匹配 (\\b)")
                                        on:click=move |_| whole_word.set(!whole_word.get())
                                    >
                                        r"\b"
                                    </button>
                                    <button
                                        type="button"
                                        class=move || if is_regex.get() { "sr-opt-btn active" } else { "sr-opt-btn" }
                                        title=t(zh, "Use Regular Expression (.*)", "使用正则表达式 (.*)")
                                        on:click=move |_| is_regex.set(!is_regex.get())
                                    >
                                        ".*"
                                    </button>
                                </div>
                            </div>
                        </div>

                        // Replace Row (only visible in Replace mode)
                        <div style:display=move || if active_tab.get() == "replace" { "block" } else { "none" }>
                            <label class="sr-field-label">
                                {t(zh, "Replace With", "替换为")}
                            </label>
                            <input
                                type="text"
                                class="form-control"
                                placeholder=t(zh, "Replacement text...", "输入替换后的内容...")
                                prop:value=move || replacement.get()
                                on:input=move |ev| replacement.set(event_target_value(&ev))
                                on:keydown=on_search_kd2
                            />
                        </div>

                        // Contextual Hint Banner
                        {move || if is_regex.get() {
                            let hint = if active_tab.get() == "replace" {
                                t(
                                    zh,
                                    "Regex active: Supports capture groups $1, $2 (or \\1, \\2) and $0 (\\0) for entire match.",
                                    "已开启正则：替换支持捕获组 $1, $2（或 \\1, \\2）及整段匹配 $0（\\0）。",
                                )
                            } else {
                                t(
                                    zh,
                                    "Regex active: Enter patterns like \\b\\w+, fn\\s+\\w+, or TODO:.*",
                                    "已开启正则：支持如 \\b\\w+、fn\\s+\\w+ 或 TODO:.* 等正则模式。",
                                )
                            };
                            view! {
                                <div class="sr-regex-hint">
                                    <span>"💡"</span>
                                    <span>{hint}</span>
                                </div>
                            }.into_any()
                        } else if active_tab.get() == "find" {
                            view! {
                                <div class="sr-literal-hint">
                                    <span>"💡"</span>
                                    <span>{t(zh, "Literal search: Finds specific text verbatim. Toggle Aa for case sensitivity.", "精确文本查找：按原文本字面匹配。可点击 Aa 切换大小写。")}</span>
                                </div>
                            }.into_any()
                        } else {
                            view! {}.into_any()
                        }}

                        // Scope & Filters Row
                        <div style="display:grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap:0.9rem; align-items:flex-end;">
                            // Scope selection
                            <div>
                                <label class="sr-field-label">
                                    {t(zh, "Search Scope", "查找范围")}
                                </label>
                                <div style="display:flex; gap:0.35rem;">
                                    <button
                                        type="button"
                                        class=move || if scope.get() == "all" { "btn btn-primary btn-sm" } else { "btn btn-secondary btn-sm" }
                                        style="flex:1; font-size:0.8rem; padding:0.3rem 0.5rem;"
                                        on:click=move |_| scope.set("all".to_string())
                                    >
                                        "🌐 " {t(zh, "All Project Files", "所有项目文件")}
                                    </button>
                                    <button
                                        type="button"
                                        class=move || if scope.get() == "single" { "btn btn-primary btn-sm" } else { "btn btn-secondary btn-sm" }
                                        style="flex:1; font-size:0.8rem; padding:0.3rem 0.5rem;"
                                        on:click=move |_| scope.set("single".to_string())
                                    >
                                        "📄 " {t(zh, "Single File", "单文件")}
                                    </button>
                                </div>
                            </div>

                            // File selection or Extension filter
                            {move || if scope.get() == "single" {
                                view! {
                                    <div>
                                        <label class="sr-field-label">
                                            {t(zh, "Target File", "目标文件")}
                                        </label>
                                        <select
                                            class="form-control"
                                            prop:value=move || selected_file.get()
                                            on:change=move |ev| selected_file.set(event_target_value(&ev))
                                        >
                                            {parsed_files.iter().map(|f| {
                                                view! { <option value=f.clone()>{f.clone()}</option> }
                                            }).collect::<Vec<_>>()}
                                        </select>
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <div>
                                        <label class="sr-field-label">
                                            {t(zh, "File Pattern / Extensions", "文件后缀过滤")}
                                        </label>
                                        <input
                                            type="text"
                                            class="form-control"
                                            placeholder=t(zh, "e.g. typ, tex, md, py (blank for all)", "如 typ, tex, md, py (留空匹配全部)")
                                            prop:value=move || ext_filter.get()
                                            on:input=move |ev| ext_filter.set(event_target_value(&ev))
                                        />
                                    </div>
                                }.into_any()
                            }}
                        </div>
                    </div>

                    // Action buttons bar
                    <div style="display:flex; justify-content:space-between; align-items:center; padding-top:0.6rem; border-top:1px solid var(--border-subtle, #e7eaf0);">
                        <div style="display:flex; gap:0.5rem;">
                            <button
                                type="button"
                                class="btn btn-primary btn-sm"
                                disabled=move || is_searching.get() || is_replacing.get()
                                on:click=on_search_click
                            >
                                {move || if is_searching.get() {
                                    t(zh, "Searching...", "正在查找...")
                                } else {
                                    t(zh, "Find Occurrences", "开始查找")
                                }}
                            </button>
                        </div>
                        <div
                            style="display:flex; gap:0.5rem;"
                            style:display=move || if active_tab.get() == "replace" { "flex" } else { "none" }
                        >
                            <button
                                type="button"
                                class="btn btn-secondary btn-sm text-danger"
                                disabled=move || is_replacing.get() || is_searching.get() || search_response.get().is_none()
                                on:click=move |_| perform_replace()
                            >
                                {move || if is_replacing.get() {
                                    t(zh, "Replacing...", "正在替换...")
                                } else {
                                    t(zh, "Replace in Selected", "在所选文件中替换")
                                }}
                            </button>
                        </div>
                    </div>

                    // Results Section
                    {move || search_response.get().map(|resp| {
                        let total = resp.total_matches;
                        let count = resp.files_count;
                        let results = resp.results.clone();

                        if total == 0 {
                            return view! {
                                <div class="sr-empty-results">
                                    {t(zh, "No matches found for the given criteria.", "未找到匹配项。")}
                                </div>
                            }.into_any();
                        }

                        let summary_text = if zh {
                            format!("在 {} 个文件中找到 {} 处匹配", count, total)
                        } else {
                            format!("Found {} matches across {} files", total, count)
                        };

                        let pid_root = modal_project_id.clone();
                        let is_replace_mode = active_tab.get() == "replace";
                        let active_doc_file = current_opened_file.clone();

                        let cards = results.into_iter().map(|f_res| {
                            let file_path = f_res.file_path;
                            let match_cnt = f_res.match_count;
                            let matches = f_res.matches;

                            let fp_chk = file_path.clone();
                            let is_checked = move || selected_files_for_replace.get().contains(&fp_chk);

                            let fp_exp_icon = file_path.clone();
                            let is_exp_icon = move || if expanded_files.get().contains(&fp_exp_icon) { "▲" } else { "▼" };

                            let fp_exp_disp = file_path.clone();
                            let is_exp_display = move || if expanded_files.get().contains(&fp_exp_disp) { "block" } else { "none" };

                            let fp_toggle = file_path.clone();
                            let on_header_click = move |_| toggle_file_expanded(fp_toggle.clone());

                            let fp_chk_click = file_path.clone();
                            let on_chk_click = move |ev: leptos::ev::MouseEvent| {
                                ev.stop_propagation();
                                toggle_file_selected(fp_chk_click.clone());
                            };

                            let is_curr_open_file = active_doc_file.as_deref() == Some(file_path.as_str());
                            let file_editor_url = format!("/projects/{pid_root}/editor?file={}", urlencoding::encode(&file_path));
                            let pid_for_rows = pid_root.clone();
                            let fp_for_rows = file_path.clone();

                            let current_search_query = query.get();
                            let match_rows = matches.into_iter().map(|m| {
                                let line_no = m.line_number;
                                let orig = m.line_content.clone();
                                let orig_title = orig.clone();
                                let preview = m.preview_replaced;
                                let row_editor_url = format!(
                                    "/projects/{pid_for_rows}/editor?file={}&line={line_no}&query={}#L{line_no}",
                                    urlencoding::encode(&fp_for_rows),
                                    urlencoding::encode(&current_search_query),
                                );

                                let start = m.match_start;
                                let end = m.match_end;
                                let highlighted_line = if start < end {
                                    if let (Some(before), Some(matched), Some(after)) = (
                                        orig.get(..start),
                                        orig.get(start..end),
                                        orig.get(end..),
                                    ) {
                                        view! {
                                            <div class="sr-orig-line" title=orig_title>
                                                <span>{before.to_string()}</span>
                                                <mark class="sr-highlight">{matched.to_string()}</mark>
                                                <span>{after.to_string()}</span>
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! { <div class="sr-orig-line" title=orig_title>{orig}</div> }.into_any()
                                    }
                                } else {
                                    view! { <div class="sr-orig-line" title=orig_title>{orig}</div> }.into_any()
                                };

                                let q_jump = current_search_query.clone();
                                let r_jump_url = row_editor_url.clone();
                                let on_jump_click = std::rc::Rc::new(move |ev: leptos::ev::MouseEvent| {
                                    ev.prevent_default();
                                    ev.stop_propagation();
                                    if is_curr_open_file {
                                        open.set(false);
                                        #[cfg(feature = "hydrate")]
                                        {
                                            crate::jump_to_line::jump_to_match_in_dom(
                                                "code-editor-input",
                                                line_no as u32,
                                                Some(&q_jump),
                                                Some(start),
                                                Some(end),
                                            );
                                            if let Some(win) = web_sys::window() {
                                                let _ = win.location().set_hash(&format!("L{line_no}"));
                                            }
                                        }
                                    } else {
                                        #[cfg(feature = "hydrate")]
                                        if let Some(win) = web_sys::window() {
                                            let _ = win.location().set_href(&r_jump_url);
                                        }
                                    }
                                });
                                let on_jump_click2 = on_jump_click.clone();

                                view! {
                                    <div
                                        class="sr-match-row"
                                        on:click=move |ev| on_jump_click(ev)
                                        title=t(zh, "Click to jump and highlight in document", "点击在文档中跳转并高亮")
                                    >
                                        <a
                                            href=row_editor_url
                                            class="sr-jump-link"
                                            title=t(zh, "Jump to line in editor", "在编辑器此行打开")
                                            on:click=move |ev| on_jump_click2(ev)
                                        >
                                            "L"{line_no} " ↗"
                                        </a>
                                        <div class="sr-match-snippet">
                                            {highlighted_line}
                                            {preview.map(|p| {
                                                let p_title = p.clone();
                                                view! {
                                                    <div class="sr-rep-line" title=p_title>
                                                        <span style="color:#10b981; margin-right:0.35rem;">"→"</span>
                                                        {p}
                                                    </div>
                                                }
                                            })}
                                        </div>
                                    </div>
                                }
                            }).collect::<Vec<_>>();

                            let f_head_url = file_editor_url.clone();
                            let on_open_file_click = move |ev: leptos::ev::MouseEvent| {
                                ev.stop_propagation();
                                ev.prevent_default();
                                if is_curr_open_file {
                                    open.set(false);
                                    #[cfg(feature = "hydrate")]
                                    if let Some(win) = web_sys::window() {
                                        if let Some(doc) = win.document() {
                                            if let Some(el) = doc.get_element_by_id("code-editor-input") {
                                                use wasm_bindgen::JsCast;
                                                if let Ok(ta) = el.dyn_into::<web_sys::HtmlTextAreaElement>() {
                                                    let _ = ta.focus();
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    #[cfg(feature = "hydrate")]
                                    if let Some(win) = web_sys::window() {
                                        let _ = win.location().set_href(&f_head_url);
                                    }
                                }
                            };

                            view! {
                                <div class="sr-file-card">
                                    <div class="sr-file-header" on:click=on_header_click>
                                        <div style="display:flex; align-items:center; gap:0.6rem;">
                                            {if is_replace_mode {
                                                view! {
                                                    <input
                                                        type="checkbox"
                                                        prop:checked=is_checked
                                                        on:click=on_chk_click
                                                        title=t(zh, "Include this file in replacement", "勾选以在替换时包含此文件")
                                                    />
                                                }.into_any()
                                            } else {
                                                view! {}.into_any()
                                            }}
                                            <span class="sr-file-title">
                                                "📄 " {file_path}
                                            </span>
                                            <a
                                                href=file_editor_url
                                                class="sr-jump-link"
                                                title=t(zh, "Open file in editor", "在编辑器中打开文件")
                                                on:click=on_open_file_click
                                            >
                                                "↗ " {t(zh, "Open", "打开")}
                                            </a>
                                        </div>
                                        <div style="display:flex; align-items:center; gap:0.5rem;">
                                            <span class="sr-count-pill">
                                                {format!("{match_cnt} {}", if zh { "处" } else { "matches" })}
                                            </span>
                                            <span class="sr-chevron">{is_exp_icon}</span>
                                        </div>
                                    </div>
                                    <div class="sr-file-body" style:display=is_exp_display>
                                        {match_rows}
                                    </div>
                                </div>
                            }
                        }).collect::<Vec<_>>();

                        view! {
                            <div style="display:flex; flex-direction:column; gap:0.6rem; margin-top:0.3rem;">
                                <div style="display:flex; justify-content:space-between; align-items:center;">
                                    <span style="font-size:0.85rem; font-weight:600; color:var(--text-muted, #475467);">
                                        {summary_text}
                                    </span>
                                    {if is_replace_mode {
                                        view! {
                                            <div style="display:flex; gap:0.4rem;">
                                                <button
                                                    type="button"
                                                    class="btn btn-ghost btn-sm"
                                                    style="font-size:0.75rem; padding:0.15rem 0.45rem;"
                                                    on:click=move |_| select_all_files(true)
                                                >
                                                    {t(zh, "Select All", "全选")}
                                                </button>
                                                <button
                                                    type="button"
                                                    class="btn btn-ghost btn-sm"
                                                    style="font-size:0.75rem; padding:0.15rem 0.45rem;"
                                                    on:click=move |_| select_all_files(false)
                                                >
                                                    {t(zh, "Deselect All", "取消全选")}
                                                </button>
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! {}.into_any()
                                    }}
                                </div>
                                <div style="display:flex; flex-direction:column; gap:0.5rem;">
                                    {cards}
                                </div>
                            </div>
                        }.into_any()
                    })}
                </div>

                // Modal Footer
                <div class="modal-footer sr-modal-footer">
                    <button
                        type="button"
                        class="btn btn-secondary"
                        on:click=move |_| open.set(false)
                    >
                        {t(zh, "Close", "关闭")}
                    </button>
                </div>
            </div>
        </div>
    }
}

#[cfg(feature = "hydrate")]
fn wire_search_replace_listener(
    open: RwSignal<bool>,
    active_tab: RwSignal<String>,
    scope: RwSignal<String>,
    selected_file: RwSignal<String>,
    query: RwSignal<String>,
) {
    use wasm_bindgen::JsCast;
    Effect::new(move |_| {
        if let Some(win) = web_sys::window() {
            let cb = wasm_bindgen::closure::Closure::wrap(Box::new(move |ev: web_sys::CustomEvent| {
                let detail_val = ev.detail();
                if let Some(detail_str) = detail_val.as_string() {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&detail_str) {
                        if let Some(mode) = val.get("mode").and_then(|s| s.as_str()) {
                            if mode == "find" || mode == "replace" {
                                active_tab.set(mode.to_string());
                            }
                        }
                        if let Some(f) = val.get("file").and_then(|s| s.as_str()) {
                            if !f.is_empty() {
                                selected_file.set(f.to_string());
                                scope.set("single".to_string());
                            }
                        }
                        if let Some(q) = val.get("query").and_then(|s| s.as_str()) {
                            if !q.is_empty() {
                                query.set(q.to_string());
                            }
                        }
                    }
                }
                open.set(true);
            }) as Box<dyn FnMut(web_sys::CustomEvent)>);

            let _ = win.add_event_listener_with_callback(
                "apich-open-search-replace",
                cb.as_ref().unchecked_ref(),
            );
            cb.forget();
        }
    });
}

#[cfg(not(feature = "hydrate"))]
const fn wire_search_replace_listener(
    _open: RwSignal<bool>,
    _active_tab: RwSignal<String>,
    _scope: RwSignal<String>,
    _selected_file: RwSignal<String>,
    _query: RwSignal<String>,
) {}
