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

    let upload_target_folder = RwSignal::new("assets".to_string());
    let is_uploading = RwSignal::new(false);
    let upload_status = RwSignal::new(Option::<(bool, String)>::None);

    let active_doc = StoredValue::new(active_file);

    let project_id_clone = project_id.clone();
    let reload_files = move || {
        fetch_project_files(project_id_clone.clone(), files, is_loading_files);
    };

    wire_attach_modal_listener(visible, active_tab, reload_files);

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

    let p_id_for_upload = project_id.clone();
    let on_submit_upload = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let folder = upload_target_folder.get();
        upload_selected_file(
            p_id_for_upload.clone(),
            folder,
            files,
            is_uploading,
            upload_status,
        );
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
                    // Filter bar + Search
                    <div class="attach-filter-bar">
                        <div class="attach-search-wrap">
                            <span class="attach-search-icon">"🔍"</span>
                            <input
                                type="text"
                                class="form-input attach-search-input"
                                placeholder="Filter files by name or path..."
                                prop:value=move || search_query.get()
                                on:input=move |ev| search_query.set(event_target_value(&ev))
                            />
                            {move || if !search_query.get().is_empty() {
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
                            } else {
                                view! { <span></span> }.into_any()
                            }}
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
                        <div class="attach-dropzone">
                            <span style="font-size:2.5rem; line-height:1; margin-bottom:0.5rem;">"☁️"</span>
                            <div style="font-weight:600; font-size:1rem; margin-bottom:0.25rem;">
                                "Select a file to upload to your project"
                            </div>
                            <div style="font-size:0.8rem; color:var(--text-muted); margin-bottom:1rem;">
                                "Images (.png, .jpg, .svg), documents (.pdf, .typ), data (.csv, .json), and more."
                            </div>
                            <input
                                type="file"
                                id="attach-file-input"
                                class="attach-file-native-input"
                                required
                            />
                        </div>

                        <div class="attach-upload-options">
                            <label class="form-label" style="font-size:0.85rem; font-weight:500;">
                                "Destination Folder:"
                            </label>
                            <div style="display:flex; gap:0.5rem; align-items:center;">
                                <select
                                    class="form-select"
                                    style="max-width:200px;"
                                    prop:value=move || upload_target_folder.get()
                                    on:change=move |ev| upload_target_folder.set(event_target_value(&ev))
                                >
                                    <option value="assets">"assets/ (Recommended)"</option>
                                    <option value="images">"images/"</option>
                                    <option value="data">"data/"</option>
                                    <option value="">"Project Root (/)"</option>
                                </select>
                                <span style="font-size:0.8rem; color:var(--text-muted);">
                                    "Images and external files are commonly referenced from assets/"
                                </span>
                            </div>
                        </div>

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

                        <div class="attach-upload-footer">
                            <button
                                type="submit"
                                class="btn btn-primary"
                                disabled=move || is_uploading.get()
                            >
                                {move || if is_uploading.get() {
                                    "⏳ Uploading..."
                                } else {
                                    "📤 Upload & Make Available"
                                }}
                            </button>
                        </div>
                    </form>
                </div>
            </div>
        </div>
    }
}

/// Generates appropriate syntax snippet based on target file category & active document extension
fn generate_snippet(item: &ProjectFileItem, active_doc: &str) -> String {
    let lower_doc = active_doc.to_lowercase();
    let is_typst = lower_doc.ends_with(".typ") || lower_doc.contains("slide");
    let is_latex = lower_doc.ends_with(".tex") || lower_doc.ends_with(".latex");
    let is_md_or_note = lower_doc.ends_with(".md")
        || lower_doc.ends_with(".anote")
        || lower_doc.ends_with(".txt")
        || lower_doc.is_empty();

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
        path.to_string()
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
) {
    use wasm_bindgen::JsCast;

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

    wasm_bindgen_futures::spawn_local(async move {
        is_uploading.set(true);
        upload_status.set(None);

        let form_data = match web_sys::FormData::new() {
            | Ok(fd) => fd,
            | Err(_) => {
                is_uploading.set(false);
                upload_status.set(Some((false, "Could not create form data.".to_string())));
                return;
            },
        };

        let _ = form_data.append_with_blob("file", &file);
        let _ = form_data.append_with_str("folder", &folder);

        let url = format!("/projects/{}/files/upload.json", project_id);
        let req = match gloo_net::http::Request::post(&url).body(&form_data) {
            | Ok(r) => r,
            | Err(e) => {
                is_uploading.set(false);
                upload_status.set(Some((false, format!("Failed to create request: {e}"))));
                return;
            },
        };

        match req.send().await {
            | Ok(resp) => {
                if let Ok(upload_res) = resp.json::<UploadFileResponse>().await {
                    if upload_res.success {
                        let path = upload_res.file_path.unwrap_or_default();
                        upload_status.set(Some((
                            true,
                            format!("Uploaded successfully to {path}! Available in files list."),
                        )));
                        input.set_value("");
                        // Refresh files list
                        fetch_project_files(project_id, files, RwSignal::new(false));
                    } else {
                        let err = upload_res
                            .error
                            .unwrap_or_else(|| "Upload failed.".to_string());
                        upload_status.set(Some((false, err)));
                    }
                } else {
                    upload_status.set(Some((false, "Invalid response from server.".to_string())));
                }
            },
            | Err(e) => {
                upload_status.set(Some((false, format!("Upload network error: {e}"))));
            },
        }

        is_uploading.set(false);
    });
}

#[cfg(not(feature = "hydrate"))]
fn upload_selected_file(
    _project_id: String,
    _folder: String,
    _files: RwSignal<Vec<ProjectFileItem>>,
    _is_uploading: RwSignal<bool>,
    _upload_status: RwSignal<Option<(bool, String)>>,
) {
}

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
fn insert_text_at_cursor(_snippet: &str) {}

#[cfg(feature = "hydrate")]
fn copy_to_clipboard(text: &str) {
    if let Some(win) = web_sys::window() {
        let nav = win.navigator();
        let clip = nav.clipboard();
        let _ = clip.write_text(text);
    }
}

#[cfg(not(feature = "hydrate"))]
fn copy_to_clipboard(_text: &str) {}

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
