//! Attach and Upload modal island for document and note editors.
//!
//! Allows inserting files/images into the active editor:
//! 1. "Attach" tab: lists existing project files with category filters (All, Images, Documents, Data, Other)
//!    and a live search input.
//! 2. "Upload" tab: uploads local files to the project's `assets/` directory and can immediately insert
//!    the markdown/Typst/LaTeX reference snippet at the cursor.
//!
//! Cross-boundary trigger:
//! Dispatches `window.dispatchEvent(new CustomEvent('apich-open-attach-modal', { detail: { mode: 'attach' | 'upload' } }))`.

use leptos::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use crate::file_upload::{format_eta, format_speed};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectFileItem {
    pub path: String,
    pub name: String,
    pub size_bytes: u64,
    pub extension: String,
    pub category: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ListFilesResponse {
    files: Vec<ProjectFileItem>,
}

#[derive(Debug, Clone, Deserialize)]
struct UploadFileResponse {
    success: bool,
    #[serde(default)]
    file_path: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

#[island]
pub fn AttachModalIsland(
    #[prop(into)] project_id: String,
    #[prop(into)] active_file: String,
) -> impl IntoView {
    let visible = RwSignal::new(false);
    let active_tab = RwSignal::new("attach".to_string()); // "attach" or "upload"
    let category_filter = RwSignal::new("all".to_string()); // "all", "image", "document", "data", "other"
    let search_query = RwSignal::new(String::new());

    let files = RwSignal::new(Vec::<ProjectFileItem>::new());
    let is_loading_files = RwSignal::new(false);

    let upload_folder_preset = RwSignal::new("assets".to_string());
    let upload_custom_folder = RwSignal::new(String::new());
    let selected_file_info = RwSignal::new(Option::<(String, String)>::None);
    let is_uploading = RwSignal::new(false);
    let upload_status = RwSignal::new(Option::<(bool, String)>::None);
    let upload_progress_pct = RwSignal::new(0.0f64);
    let upload_speed_bps = RwSignal::new(0.0f64);
    let upload_loaded_bytes = RwSignal::new(0u64);
    let upload_total_bytes = RwSignal::new(0u64);
    let upload_eta_secs = RwSignal::new(0u64);
    let is_processing_upload = RwSignal::new(false);

    let active_doc = StoredValue::new(active_file);

    let project_id_clone = project_id.clone();
    let reload_files = move || {
        fetch_project_files(project_id_clone.clone(), files, is_loading_files);
    };

    wire_attach_modal_listener(visible, active_tab, reload_files);

    #[allow(clippy::cast_precision_loss)]
    let format_file_size = |bytes: u64| -> String {
        if bytes < 1024 {
            format!("{bytes} B")
        } else if bytes < 1024 * 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else {
            format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
        }
    };

    let get_file_icon = |cat: &str, ext: &str| -> &'static str {
        match cat {
            | "image" => "🖼️",
            | "document" => match ext {
                | "pdf" => "📕",
                | "typ" => "📄",
                | "tex" => "📜",
                | "md" | "anote" => "📝",
                | _ => "📃",
            },
            | "data" => match ext {
                | "csv" | "tsv" | "xlsx" => "📊",
                | "json" => "📦",
                | "sql" | "db" | "sqlite" => "🗄️",
                | _ => "📁",
            },
            | _ => "📎",
        }
    };

    let filtered_files = move || {
        let query = search_query.get().trim().to_lowercase();
        let cat = category_filter.get();
        let list = files.get();

        list.into_iter()
            .filter(|f| {
                if cat != "all" && f.category != cat {
                    return false;
                }
                if !query.is_empty()
                    && !f.name.to_lowercase().contains(&query)
                    && !f.path.to_lowercase().contains(&query)
                {
                    return false;
                }
                true
            })
            .collect::<Vec<_>>()
    };

    let counts = move || {
        let list = files.get();
        let all = list.len();
        let img = list.iter().filter(|f| f.category == "image").count();
        let doc = list.iter().filter(|f| f.category == "document").count();
        let data = list.iter().filter(|f| f.category == "data").count();
        let other = list.iter().filter(|f| f.category == "other").count();
        (all, img, doc, data, other)
    };

    let p_id_for_upload = project_id;
    let on_submit_upload = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let preset = upload_folder_preset.get();
        let folder = if preset == "custom" {
            upload_custom_folder.get().trim().to_string()
        } else {
            preset
        };
        upload_selected_file(
            p_id_for_upload.clone(),
            folder,
            files,
            is_uploading,
            upload_status,
            selected_file_info,
            upload_progress_pct,
            upload_speed_bps,
            upload_loaded_bytes,
            upload_total_bytes,
            upload_eta_secs,
            is_processing_upload,
        );
    };

    let on_file_change = move |ev: leptos::ev::Event| {
        if let Some((name, size_bytes)) = extract_selected_file_info(&ev) {
            selected_file_info.set(Some((name, format_file_size(size_bytes))));
        } else {
            selected_file_info.set(None);
        }
    };

    let on_clear_file = move |_| {
        selected_file_info.set(None);
        clear_file_input();
    };

    view! {
        <div
            class="modal-backdrop attach-modal-backdrop"
            style:display=move || if visible.get() { "flex" } else { "none" }
        >
            <div class="modal-card attach-modal-card">
                // Modal Header
                <div class="modal-header attach-modal-header">
                    <div style="display:flex; align-items:center; gap:0.5rem;">
                        <span style="font-size:1.4rem;">"📎"</span>
                        <div>
                            <h3 style="margin:0; font-size:1.15rem; font-weight:600;">"Insert Files & Images"</h3>
                            <p style="margin:0.2rem 0 0; font-size:0.8rem; color:var(--text-muted);">
                                "Insert file references or upload new assets into this project."
                            </p>
                        </div>
                    </div>
                    <button
                        type="button"
                        class="modal-close"
                        on:click=move |_| visible.set(false)
                        title="Close (Esc)"
                    >
                        "×"
                    </button>
                </div>

                // Tab Switcher
                <div class="attach-modal-tabs">
                    <button
                        type="button"
                        class="attach-modal-tab"
                        class=("active", move || active_tab.get() == "attach")
                        on:click=move |_| active_tab.set("attach".to_string())
                    >
                        <span>"🗂️ Project Files"</span>
                        <span class="attach-badge">{move || files.get().len()}</span>
                    </button>
                    <button
                        type="button"
                        class="attach-modal-tab"
                        class=("active", move || active_tab.get() == "upload")
                        on:click=move |_| active_tab.set("upload".to_string())
                    >
                        <span>"📤 Upload New File"</span>
                    </button>
                </div>

                // Body: Attach / Browse Files Tab
                <div
                    class="attach-tab-pane"
                    style:display=move || if active_tab.get() == "attach" { "flex" } else { "none" }
                >
                    // Filter bar: Modern Search Input + Category Dropdown
                    <div class="attach-filter-bar">
                        <div class="attach-search-row">
                            <div class="attach-search-wrap">
                                <span class="attach-search-icon">"🔍"</span>
                                <input
                                    type="text"
                                    class="attach-modern-input attach-search-input"
                                    placeholder="Search project files by name or path..."
                                    prop:value=move || search_query.get()
                                    on:input=move |ev| search_query.set(event_target_value(&ev))
                                />
                                {move || if search_query.get().is_empty() {
                                    view! { <span></span> }.into_any()
                                } else {
                                    view! {
                                        <button
                                            type="button"
                                            class="attach-search-clear"
                                            on:click=move |_| search_query.set(String::new())
                                            title="Clear search"
                                        >
                                            "×"
                                        </button>
                                    }.into_any()
                                }}
                            </div>
                            <div class="attach-dropdown-wrap">
                                <select
                                    class="attach-modern-select attach-filter-select"
                                    prop:value=move || category_filter.get()
                                    on:change=move |ev| category_filter.set(event_target_value(&ev))
                                    aria-label="Filter by file type"
                                >
                                    {move || {
                                        let (all_cnt, img_cnt, doc_cnt, data_cnt, other_cnt) = counts();
                                        view! {
                                            <option value="all">{format!("📁 All Categories ({all_cnt})")}</option>
                                            <option value="image">{format!("🖼️ Images ({img_cnt})")}</option>
                                            <option value="document">{format!("📄 Documents ({doc_cnt})")}</option>
                                            <option value="data">{format!("📊 Data & Tables ({data_cnt})")}</option>
                                            {(other_cnt > 0).then(|| {
                                                view! { <option value="other">{format!("📦 Other Files ({other_cnt})")}</option> }
                                            })}
                                        }
                                    }}
                                </select>
                            </div>
                        </div>

                        // Category Filter Pills
                        <div class="attach-category-pills">
                            {move || {
                                let (all_cnt, img_cnt, doc_cnt, data_cnt, other_cnt) = counts();
                                view! {
                                    <button
                                        type="button"
                                        class="attach-pill"
                                        class=("active", category_filter.get() == "all")
                                        on:click=move |_| category_filter.set("all".to_string())
                                    >
                                        "All (" {all_cnt} ")"
                                    </button>
                                    <button
                                        type="button"
                                        class="attach-pill"
                                        class=("active", category_filter.get() == "image")
                                        on:click=move |_| category_filter.set("image".to_string())
                                    >
                                        "🖼️ Images (" {img_cnt} ")"
                                    </button>
                                    <button
                                        type="button"
                                        class="attach-pill"
                                        class=("active", category_filter.get() == "document")
                                        on:click=move |_| category_filter.set("document".to_string())
                                    >
                                        "📄 Documents (" {doc_cnt} ")"
                                    </button>
                                    <button
                                        type="button"
                                        class="attach-pill"
                                        class=("active", category_filter.get() == "data")
                                        on:click=move |_| category_filter.set("data".to_string())
                                    >
                                        "📊 Data (" {data_cnt} ")"
                                    </button>
                                    {if other_cnt > 0 {
                                        view! {
                                            <button
                                                type="button"
                                                class="attach-pill"
                                                class=("active", category_filter.get() == "other")
                                                on:click=move |_| category_filter.set("other".to_string())
                                            >
                                                "📦 Other (" {other_cnt} ")"
                                            </button>
                                        }.into_any()
                                    } else {
                                        view! { <span></span> }.into_any()
                                    }}
                                }
                            }}
                        </div>
                    </div>

                    // File List
                    <div class="attach-file-list">
                        {move || {
                            if is_loading_files.get() {
                                view! {
                                    <div class="attach-empty-state">
                                        <div class="spinner" style="width:24px; height:24px; margin-bottom:0.5rem;"></div>
                                        <span>"Loading project files..."</span>
                                    </div>
                                }.into_any()
                            } else {
                                let list = filtered_files();
                                if list.is_empty() {
                                    view! {
                                        <div class="attach-empty-state">
                                            <span style="font-size:2rem; margin-bottom:0.5rem;">"📂"</span>
                                            <p style="margin:0 0 0.5rem; font-weight:500;">"No files found"</p>
                                            <p style="margin:0; font-size:0.8rem; color:var(--text-muted);">
                                                "No files match your query. You can switch to the Upload tab to add files."
                                            </p>
                                            <button
                                                type="button"
                                                class="btn btn-secondary btn-sm"
                                                style="margin-top:0.8rem;"
                                                on:click=move |_| active_tab.set("upload".to_string())
                                            >
                                                "📤 Upload a File"
                                            </button>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div class="attach-items-grid">
                                            {list.into_iter().map(|item| {
                                                let item_ins = item.clone();
                                                let item_copy = item.clone();
                                                let icon = get_file_icon(&item.category, &item.extension);
                                                let size_str = format_file_size(item.size_bytes);
                                                let on_copy = move |_| {
                                                    let doc = active_doc.get_value();
                                                    let snippet = generate_snippet(&item_copy, &doc);
                                                    copy_to_clipboard(&snippet);
                                                };
                                                let on_ins = move |_| {
                                                    let doc = active_doc.get_value();
                                                    let snippet = generate_snippet(&item_ins, &doc);
                                                    insert_text_at_cursor(&snippet);
                                                    visible.set(false);
                                                };
                                                view! {
                                                    <div class="attach-file-item">
                                                        <div class="attach-file-icon">{icon}</div>
                                                        <div class="attach-file-info">
                                                            <div class="attach-file-name" title=item.path.clone()>{item.name}</div>
                                                            <div class="attach-file-sub">
                                                                <span class="attach-file-path">{item.path}</span>
                                                                <span class="attach-file-size">" • " {size_str}</span>
                                                            </div>
                                                        </div>
                                                        <div class="attach-file-actions">
                                                            <button
                                                                type="button"
                                                                class="btn btn-secondary btn-xs attach-btn-copy"
                                                                title="Copy snippet syntax to clipboard"
                                                                on:click=on_copy
                                                            >
                                                                "📋 Copy"
                                                            </button>
                                                            <button
                                                                type="button"
                                                                class="btn btn-primary btn-xs attach-btn-insert"
                                                                title="Insert at cursor into active document"
                                                                on:click=on_ins
                                                            >
                                                                "↵ Insert"
                                                            </button>
                                                        </div>
                                                    </div>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    }.into_any()
                                }
                            }
                        }}
                    </div>
                </div>

                // Body: Upload Tab
                <div
                    class="attach-tab-pane"
                    style:display=move || if active_tab.get() == "upload" { "flex" } else { "none" }
                >
                    <form on:submit=on_submit_upload class="attach-upload-form">
                        <div
                            class="attach-dropzone"
                            onclick="document.getElementById('attach-file-input')?.click()"
                        >
                            <span class="attach-dropzone-icon">"☁️"</span>
                            <div class="attach-dropzone-title">
                                "Choose a file or drag & drop here"
                            </div>
                            <div class="attach-dropzone-sub">
                                "Supports images (.png, .jpg, .svg, .webp), documents (.pdf, .typ), data (.csv, .json), and more"
                            </div>
                            <button
                                type="button"
                                class="btn btn-secondary attach-browse-btn"
                                onclick="event.stopPropagation(); document.getElementById('attach-file-input')?.click()"
                            >
                                "📁 Browse Local File"
                            </button>
                            <input
                                type="file"
                                id="attach-file-input"
                                class="attach-file-native-input"
                                style="display:none;"
                                on:change=on_file_change
                            />
                        </div>

                        // Selected file preview card
                        {move || {
                            selected_file_info.get().map(|(name, size)| {
                                let title_name = name.clone();
                                view! {
                                    <div class="attach-selected-card">
                                        <div style="display:flex; align-items:center; gap:0.75rem; min-width:0;">
                                            <span style="font-size:1.5rem;">"📄"</span>
                                            <div style="min-width:0;">
                                                <div class="attach-selected-name" title=title_name>{name}</div>
                                                <div class="attach-selected-size">{size}</div>
                                            </div>
                                        </div>
                                        <button
                                            type="button"
                                            class="btn btn-secondary btn-xs"
                                            on:click=on_clear_file
                                            title="Change selected file"
                                        >
                                            "✕ Change"
                                        </button>
                                    </div>
                                }
                            })
                        }}

                        <div class="attach-form-group">
                            <label class="attach-form-label">
                                "📁 Destination Folder in Project"
                            </label>
                            <div class="attach-folder-row">
                                <select
                                    class="attach-modern-select attach-folder-select"
                                    prop:value=move || upload_folder_preset.get()
                                    on:change=move |ev| upload_folder_preset.set(event_target_value(&ev))
                                >
                                    <option value="assets">"assets/ (Standard - Recommended for images & docs)"</option>
                                    <option value="images">"images/ (Dedicated images folder)"</option>
                                    <option value="data">"data/ (Data files & spreadsheets)"</option>
                                    <option value="">"Project Root (/)"</option>
                                    <option value="custom">"✏️ Custom folder path..."</option>
                                </select>
                            </div>
                            {move || if upload_folder_preset.get() == "custom" {
                                view! {
                                    <div class="attach-custom-folder-row">
                                        <label class="attach-form-sublabel">"Enter custom folder path:"</label>
                                        <input
                                            type="text"
                                            class="attach-modern-input"
                                            placeholder="e.g. assets/diagrams or docs/images"
                                            prop:value=move || upload_custom_folder.get()
                                            on:input=move |ev| upload_custom_folder.set(event_target_value(&ev))
                                        />
                                    </div>
                                }.into_any()
                            } else {
                                view! { <span></span> }.into_any()
                            }}
                        </div>

                        // Upload Progress and Live Metrics View
                        {move || if is_uploading.get() {
                            let pct = upload_progress_pct.get();
                            let speed = format_speed(upload_speed_bps.get());
                            let transferred = format!("{}/{}", format_file_size(upload_loaded_bytes.get()), format_file_size(upload_total_bytes.get()));
                            let transferred_title = transferred.clone();
                            let eta = format_eta(upload_eta_secs.get(), false);
                            let is_proc = is_processing_upload.get();

                            view! {
                                <div class="upload-live-metrics-panel" style="margin:1rem 0; display:flex; flex-direction:column; gap:0.75rem;">
                                    <div style="display:flex; justify-content:space-between; align-items:center; font-size:0.9rem;">
                                        <span style="font-weight:600; color:var(--text-main, #f0f3f6);">{if is_proc { "Finalizing on server disk..." } else { "Uploading to project..." }}</span>
                                        <span style="font-weight:700; color:var(--primary, #3b82f6);">{format!("{pct:.1}%")}</span>
                                    </div>
                                    <div class="upload-progress-container" style="background:var(--bg-elevated, #282c37); border-radius:10px; height:10px; overflow:hidden; position:relative;">
                                        <div
                                            class="upload-progress-fill"
                                            style=format!("width: {pct:.2}%; height:100%; background: linear-gradient(90deg, #3b82f6, #60a5fa, #38bdf8); transition: width 0.15s ease-out; border-radius:10px; position:relative;")
                                        >
                                            <div class="upload-progress-shine" style="position:absolute; inset:0; background:linear-gradient(90deg, transparent, rgba(255,255,255,0.3), transparent); animation:uploadShine 1.5s infinite linear;"></div>
                                        </div>
                                    </div>
                                    <div class="upload-metrics-grid" style="display:grid; grid-template-columns:repeat(3, 1fr); gap:0.5rem; text-align:center;">
                                        <div class="upload-metric-card" style="background:var(--bg-elevated, #282c37); border:1px solid var(--border-subtle, #3a3f50); border-radius:6px; padding:0.4rem;">
                                            <div style="font-size:0.7rem; color:var(--text-sub, #9aa0a6);">"Speed"</div>
                                            <div style="font-weight:700; font-size:0.85rem; color:#38bdf8;">{speed}</div>
                                        </div>
                                        <div class="upload-metric-card" style="background:var(--bg-elevated, #282c37); border:1px solid var(--border-subtle, #3a3f50); border-radius:6px; padding:0.4rem;">
                                            <div style="font-size:0.7rem; color:var(--text-sub, #9aa0a6);">"Transferred"</div>
                                            <div style="font-weight:700; font-size:0.8rem; color:var(--text-main, #f0f3f6); overflow:hidden; text-overflow:ellipsis; white-space:nowrap;" title=transferred_title>{transferred}</div>
                                        </div>
                                        <div class="upload-metric-card" style="background:var(--bg-elevated, #282c37); border:1px solid var(--border-subtle, #3a3f50); border-radius:6px; padding:0.4rem;">
                                            <div style="font-size:0.7rem; color:var(--text-sub, #9aa0a6);">"ETA"</div>
                                            <div style="font-weight:700; font-size:0.85rem; color:#a78bfa;">{if is_proc { "Processing...".to_string() } else { eta }}</div>
                                        </div>
                                    </div>
                                </div>
                            }.into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }}

                        // Upload Status Banner
                        {move || {
                            upload_status.get().map(|(success, msg)| {
                                view! {
                                    <div
                                        class="attach-status-banner"
                                        class=("success", success)
                                        class=("error", !success)
                                    >
                                        {if success { "✅ " } else { "⚠️ " }} {msg}
                                    </div>
                                }
                            })
                        }}

                        <div class="attach-upload-footer" style="display:flex; justify-content:flex-end; gap:0.75rem;">
                            {move || if is_uploading.get() {
                                view! {
                                    <button
                                        type="button"
                                        class="btn btn-danger btn-sm"
                                        onclick="window.dispatchEvent(new CustomEvent('apich-abort-attach-upload'))"
                                    >
                                        "🛑 Cancel Upload"
                                    </button>
                                }.into_any()
                            } else {
                                view! {
                                    <button
                                        type="submit"
                                        class="btn btn-primary attach-submit-btn"
                                    >
                                        "📤 Upload & Make Available"
                                    </button>
                                }.into_any()
                            }}
                        </div>
                    </form>
                </div>
            </div>
        </div>
    }
}

/// Generates appropriate syntax snippet based on target file category & active document extension
fn generate_snippet(item: &ProjectFileItem, active_doc: &str) -> String {
    let path_buf = std::path::Path::new(active_doc);
    let ext = path_buf.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
    let is_typst = ext == "typ" || active_doc.to_lowercase().contains("slide");
    let is_latex = ext == "tex" || ext == "latex";
    let is_md_or_note = ext == "md" || ext == "anote" || ext == "txt" || ext.is_empty();

    let path = &item.path;
    let name = &item.name;

    if is_typst {
        if item.category == "image" {
            format!("#image(\"{path}\")")
        } else if item.category == "document" {
            format!("#link(\"{path}\")[{name}]")
        } else {
            format!("\"{path}\"")
        }
    } else if is_latex {
        if item.category == "image" {
            format!("\\includegraphics[width=0.8\\linewidth]{{{path}}}")
        } else {
            format!("\\url{{{path}}}")
        }
    } else if is_md_or_note {
        if item.category == "image" {
            format!("![{name}]({path})")
        } else {
            format!("[{name}]({path})")
        }
    } else {
        path.clone()
    }
}

#[cfg(feature = "hydrate")]
fn wire_attach_modal_listener(
    visible: RwSignal<bool>,
    active_tab: RwSignal<String>,
    reload_files: impl Fn() + 'static,
) {
    use wasm_bindgen::prelude::*;
    let reload = std::rc::Rc::new(reload_files);

    let closure = Closure::<dyn Fn(web_sys::CustomEvent)>::new(move |ev: web_sys::CustomEvent| {
        let detail = ev.detail();
        if let Ok(js_str) = js_sys::JSON::stringify(&detail) {
            let str_val: String = js_str.into();
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&str_val) {
                if let Some(tab) = val.get("mode").and_then(|v| v.as_str()) {
                    active_tab.set(tab.to_string());
                }
            }
        }
        visible.set(true);
        reload();
    });

    if let Some(win) = web_sys::window() {
        let _ = win.add_event_listener_with_callback(
            "apich-open-attach-modal",
            closure.as_ref().unchecked_ref(),
        );
    }
    closure.forget();
}

#[cfg(not(feature = "hydrate"))]
fn wire_attach_modal_listener(
    _visible: RwSignal<bool>,
    _active_tab: RwSignal<String>,
    _reload_files: impl Fn() + 'static,
) {
}

#[cfg(feature = "hydrate")]
fn fetch_project_files(
    project_id: String,
    files: RwSignal<Vec<ProjectFileItem>>,
    is_loading: RwSignal<bool>,
) {
    wasm_bindgen_futures::spawn_local(async move {
        is_loading.set(true);
        let url = format!("/projects/{}/files.json", project_id);
        if let Ok(resp) = gloo_net::http::Request::get(&url).send().await {
            if let Ok(res) = resp.json::<ListFilesResponse>().await {
                files.set(res.files);
            }
        }
        is_loading.set(false);
    });
}

#[cfg(not(feature = "hydrate"))]
fn fetch_project_files(
    _project_id: String,
    _files: RwSignal<Vec<ProjectFileItem>>,
    _is_loading: RwSignal<bool>,
) {
}

#[cfg(feature = "hydrate")]
fn upload_selected_file(
    project_id: String,
    folder: String,
    files: RwSignal<Vec<ProjectFileItem>>,
    is_uploading: RwSignal<bool>,
    upload_status: RwSignal<Option<(bool, String)>>,
    selected_file_info: RwSignal<Option<(String, String)>>,
    upload_progress_pct: RwSignal<f64>,
    upload_speed_bps: RwSignal<f64>,
    upload_loaded_bytes: RwSignal<u64>,
    upload_total_bytes: RwSignal<u64>,
    upload_eta_secs: RwSignal<u64>,
    is_processing_upload: RwSignal<bool>,
) {
    use wasm_bindgen::JsCast;
    use wasm_bindgen::closure::Closure;

    let Some(win) = web_sys::window() else { return };
    let Some(doc) = win.document() else { return };
    let Some(el) = doc.get_element_by_id("attach-file-input") else { return };
    let Ok(input) = el.dyn_into::<web_sys::HtmlInputElement>() else { return };

    let Some(file_list) = input.files() else {
        upload_status.set(Some((false, "No file selected.".to_string())));
        return;
    };

    if file_list.length() == 0 {
        upload_status.set(Some((false, "Please select a file to upload.".to_string())));
        return;
    }

    let Some(file) = file_list.get(0) else { return };
    let file_size = file.size() as u64;

    if file_size > 200 * 1024 * 1024 {
        upload_status.set(Some((false, "File exceeds 200MB limit.".to_string())));
        return;
    }

    let form_data = match web_sys::FormData::new() {
        | Ok(fd) => fd,
        | Err(_) => {
            upload_status.set(Some((false, "Could not create form data.".to_string())));
            return;
        },
    };

    let _ = form_data.append_with_blob("file", &file);
    let _ = form_data.append_with_str("folder", &folder);

    let xhr = match web_sys::XmlHttpRequest::new() {
        | Ok(x) => x,
        | Err(_) => {
            upload_status.set(Some((false, "Failed to create request.".to_string())));
            return;
        },
    };

    let url = format!("/projects/{}/files/upload.json", project_id);
    if xhr.open_with_async("POST", &url, true).is_err() {
        upload_status.set(Some((false, "Failed to open connection.".to_string())));
        return;
    }

    is_uploading.set(true);
    is_processing_upload.set(false);
    upload_progress_pct.set(0.0);
    upload_loaded_bytes.set(0);
    upload_total_bytes.set(file_size);
    upload_speed_bps.set(0.0);
    upload_eta_secs.set(0);
    upload_status.set(None);

    if let Ok(upload_target) = xhr.upload() {
        let start_time = js_sys::Date::now();
        let mut last_calc_time = start_time;
        let mut last_loaded = 0.0f64;

        let on_progress = Closure::wrap(Box::new(move |ev: web_sys::ProgressEvent| {
            let total = ev.total();
            let loaded = ev.loaded();
            let effective_total = if total > 0.0 { total } else { file_size as f64 };

            let pct = if effective_total > 0.0 {
                (loaded / effective_total * 100.0).clamp(0.0, 100.0)
            } else {
                0.0
            };

            upload_progress_pct.set(pct);
            upload_loaded_bytes.set(loaded as u64);
            upload_total_bytes.set(effective_total as u64);

            let now = js_sys::Date::now();
            let dt = (now - last_calc_time) / 1000.0;
            if dt >= 0.2 || pct >= 99.9 {
                let bytes_diff = loaded - last_loaded;
                let instant_speed = if dt > 0.0 { (bytes_diff / dt).max(0.0) } else { 0.0 };
                let total_elapsed = (now - start_time) / 1000.0;
                let average_speed = if total_elapsed > 0.0 { (loaded / total_elapsed).max(0.0) } else { 0.0 };
                let current_speed = 0.7 * instant_speed + 0.3 * average_speed;
                upload_speed_bps.set(current_speed);

                if current_speed > 10.0 {
                    let remaining_bytes = (effective_total - loaded).max(0.0);
                    let remaining_secs = (remaining_bytes / current_speed).round() as u64;
                    upload_eta_secs.set(remaining_secs);
                }

                last_calc_time = now;
                last_loaded = loaded;
            }

            if pct >= 99.99 {
                is_processing_upload.set(true);
            }
        }) as Box<dyn FnMut(web_sys::ProgressEvent)>);

        let _ = upload_target.add_event_listener_with_callback("progress", on_progress.as_ref().unchecked_ref());
        on_progress.forget();
    }

    let xhr_load = xhr.clone();
    let project_id_refresh = project_id.clone();
    let file_name_copy = file.name();

    let on_load = Closure::wrap(Box::new(move |_ev: web_sys::Event| {
        is_uploading.set(false);
        is_processing_upload.set(false);

        let status = xhr_load.status().unwrap_or(0);
        let resp_text = xhr_load.response_text().ok().flatten().unwrap_or_default();

        if status >= 200 && status < 300 {
            if let Ok(upload_res) = serde_json::from_str::<UploadFileResponse>(&resp_text) {
                if upload_res.success {
                    let path = upload_res.file_path.unwrap_or_else(|| file_name_copy.clone());
                    upload_status.set(Some((
                        true,
                        format!("Uploaded successfully to {path}! Available in files list."),
                    )));
                    if let Some(win) = web_sys::window() {
                        if let Some(doc) = win.document() {
                            if let Some(el) = doc.get_element_by_id("attach-file-input") {
                                if let Ok(input_el) = el.dyn_into::<web_sys::HtmlInputElement>() {
                                    input_el.set_value("");
                                }
                            }
                        }
                    }
                    selected_file_info.set(None);
                    // Refresh files list
                    fetch_project_files(project_id_refresh.clone(), files, RwSignal::new(false));
                    return;
                } else {
                    let err = upload_res.error.unwrap_or_else(|| "Upload failed.".to_string());
                    upload_status.set(Some((false, err)));
                    return;
                }
            }
        }

        upload_status.set(Some((false, "Upload failed. Please try again.".to_string())));
    }) as Box<dyn FnMut(web_sys::Event)>);

    let _ = xhr.add_event_listener_with_callback("load", on_load.as_ref().unchecked_ref());
    on_load.forget();

    let on_error = Closure::wrap(Box::new(move |_ev: web_sys::Event| {
        is_uploading.set(false);
        is_processing_upload.set(false);
        upload_status.set(Some((false, "Network error occurred during upload.".to_string())));
    }) as Box<dyn FnMut(web_sys::Event)>);
    let _ = xhr.add_event_listener_with_callback("error", on_error.as_ref().unchecked_ref());
    on_error.forget();

    let on_abort = Closure::wrap(Box::new(move |_ev: web_sys::Event| {
        is_uploading.set(false);
        is_processing_upload.set(false);
        upload_status.set(Some((false, "Upload cancelled.".to_string())));
    }) as Box<dyn FnMut(web_sys::Event)>);
    let _ = xhr.add_event_listener_with_callback("abort", on_abort.as_ref().unchecked_ref());
    on_abort.forget();

    let xhr_abort = xhr.clone();
    let on_cancel_event = Closure::wrap(Box::new(move |_ev: web_sys::CustomEvent| {
        let _ = xhr_abort.abort();
    }) as Box<dyn FnMut(web_sys::CustomEvent)>);
    let _ = win.add_event_listener_with_callback("apich-abort-attach-upload", on_cancel_event.as_ref().unchecked_ref());
    on_cancel_event.forget();

    let _ = xhr.send_with_opt_form_data(Some(&form_data));
}

#[cfg(not(feature = "hydrate"))]
#[allow(clippy::too_many_arguments)]
fn upload_selected_file(
    _project_id: String,
    _folder: String,
    _files: RwSignal<Vec<ProjectFileItem>>,
    _is_uploading: RwSignal<bool>,
    _upload_status: RwSignal<Option<(bool, String)>>,
    _selected_file_info: RwSignal<Option<(String, String)>>,
    _upload_progress_pct: RwSignal<f64>,
    _upload_speed_bps: RwSignal<f64>,
    _upload_loaded_bytes: RwSignal<u64>,
    _upload_total_bytes: RwSignal<u64>,
    _upload_eta_secs: RwSignal<u64>,
    _is_processing_upload: RwSignal<bool>,
) {
}

#[cfg(feature = "hydrate")]
fn extract_selected_file_info(ev: &leptos::ev::Event) -> Option<(String, u64)> {
    use wasm_bindgen::JsCast;
    let target = ev.target()?;
    let input = target.dyn_into::<web_sys::HtmlInputElement>().ok()?;
    let files = input.files()?;
    let f = files.get(0)?;
    Some((f.name(), f.size() as u64))
}

#[cfg(not(feature = "hydrate"))]
const fn extract_selected_file_info(_ev: &leptos::ev::Event) -> Option<(String, u64)> {
    None
}

#[cfg(feature = "hydrate")]
fn clear_file_input() {
    use wasm_bindgen::JsCast;
    if let Some(win) = web_sys::window() {
        if let Some(doc) = win.document() {
            if let Some(el) = doc.get_element_by_id("attach-file-input") {
                if let Ok(input) = el.dyn_into::<web_sys::HtmlInputElement>() {
                    input.set_value("");
                }
            }
        }
    }
}

#[cfg(not(feature = "hydrate"))]
const fn clear_file_input() {}

#[cfg(feature = "hydrate")]
fn insert_text_at_cursor(snippet: &str) {
    use wasm_bindgen::JsCast;

    let Some(win) = web_sys::window() else { return };
    let Some(doc) = win.document() else { return };

    // Look for document editor textarea, note body textarea, or generic code textarea
    let textarea_el = doc
        .get_element_by_id("code-editor-input")
        .or_else(|| doc.get_element_by_id("note-body-editor"))
        .or_else(|| {
            doc.query_selector(".code-textarea")
                .ok()
                .flatten()
        });

    let Some(el) = textarea_el else { return };
    let Ok(ta) = el.dyn_into::<web_sys::HtmlTextAreaElement>() else { return };

    let value = ta.value();
    let sel_start_u16 = ta.selection_start().ok().flatten().unwrap_or(0) as usize;
    let sel_end_u16 = ta.selection_end().ok().flatten().unwrap_or(0) as usize;

    let start = utf16_offset_to_byte(&value, sel_start_u16);
    let end = utf16_offset_to_byte(&value, sel_end_u16);

    let mut new_value = String::with_capacity(value.len() + snippet.len());
    new_value.push_str(&value[..start]);
    new_value.push_str(snippet);
    new_value.push_str(&value[end..]);

    ta.set_value(&new_value);

    let new_pos = start + snippet.len();
    let new_pos_u16 = byte_offset_to_utf16(&new_value, new_pos) as u32;
    let _ = ta.set_selection_range(new_pos_u16, new_pos_u16);
    let _ = ta.focus();

    // Trigger input event so reactive listeners update state and preview
    if let Ok(event) = web_sys::Event::new("input") {
        let _ = ta.dispatch_event(&event);
    }

    // Also refresh syntax highlight overlay if available
    let id = ta.id();
    if !id.is_empty() {
        if let Some(refresh_fn) = js_sys::Reflect::get(&win, &wasm_bindgen::JsValue::from_str("refreshHighlightOverlay"))
            .ok()
            .and_then(|f| f.dyn_into::<js_sys::Function>().ok())
        {
            let _ = refresh_fn.call1(&win, &wasm_bindgen::JsValue::from_str(&id));
        }
    }
}

#[cfg(not(feature = "hydrate"))]
const fn insert_text_at_cursor(_snippet: &str) {}

#[cfg(feature = "hydrate")]
fn copy_to_clipboard(text: &str) {
    if let Some(win) = web_sys::window() {
        let nav = win.navigator();
        let clip = nav.clipboard();
        let _ = clip.write_text(text);
    }
}

#[cfg(not(feature = "hydrate"))]
const fn copy_to_clipboard(_text: &str) {}

#[cfg(feature = "hydrate")]
fn utf16_offset_to_byte(s: &str, utf16_offset: usize) -> usize {
    let mut units = 0usize;
    for (byte_idx, ch) in s.char_indices() {
        if units >= utf16_offset {
            return byte_idx;
        }
        units += ch.len_utf16();
    }
    s.len()
}

#[cfg(feature = "hydrate")]
fn byte_offset_to_utf16(s: &str, byte_offset: usize) -> usize {
    let mut units = 0usize;
    for (byte_idx, ch) in s.char_indices() {
        if byte_idx >= byte_offset {
            return units;
        }
        units += ch.len_utf16();
    }
    units
}
