//! Enhanced File Upload Modal Island with real-time progress, speed, ETA, and drag-and-drop.
//!
//! Provides `FileUploadModalIsland` for project files upload, featuring:
//! - Drag & Drop dropzone with hover highlight and click-to-browse
//! - File info preview (icon, filename, human-readable size)
//! - Destination folder selection (Project Root `/` and all project folders)
//! - Duplicate file pre-detection with replacement warning
//! - Asynchronous `XmlHttpRequest` upload streaming (never freezing the page)
//! - Live real-time progress bar (0% - 100%)
//! - Upload speed calculation in KB/s or MB/s with moving average smoothing
//! - Transferred / total size display
//! - Estimated Time Remaining (ETA)
//! - Cancellation support via `xhr.abort()`
//! - Completion banner and seamless redirect to destination folder

use leptos::prelude::*;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
struct UploadJsonResponse {
    success: bool,
    #[serde(default)]
    file_path: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    size_bytes: Option<u64>,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

/// Format bytes into a human-readable string (e.g. `14.5 MB`).
#[allow(clippy::cast_precision_loss)]
pub fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

/// Format upload speed in bytes/second into human-readable string.
pub fn format_speed(bps: f64) -> String {
    if bps < 10.0 {
        "-".to_string()
    } else if bps < 1024.0 {
        format!("{bps:.0} B/s")
    } else if bps < 1024.0 * 1024.0 {
        format!("{:.1} KB/s", bps / 1024.0)
    } else {
        format!("{:.1} MB/s", bps / (1024.0 * 1024.0))
    }
}

/// Format estimated remaining seconds into human-readable string.
pub fn format_eta(secs: u64, is_zh: bool) -> String {
    if secs == 0 {
        if is_zh { "即将完成" } else { "Almost done" }.to_string()
    } else if secs < 60 {
        if is_zh {
            format!("约 {secs} 秒")
        } else {
            format!("~{secs}s left")
        }
    } else if secs < 3600 {
        let mins = secs / 60;
        let s = secs % 60;
        if is_zh {
            format!("约 {mins} 分 {s} 秒")
        } else {
            format!("~{mins}m {s}s left")
        }
    } else {
        let hrs = secs / 3600;
        let mins = (secs % 3600) / 60;
        if is_zh {
            format!("约 {hrs} 小时 {mins} 分")
        } else {
            format!("~{hrs}h {mins}m left")
        }
    }
}

/// Helper to get an icon based on filename extension.
pub fn get_file_icon(name: &str) -> &'static str {
    let lower = name.to_lowercase();
    let ext = lower.rsplit('.').next().unwrap_or("");
    match ext {
        | "png" | "jpg" | "jpeg" | "svg" | "webp" | "gif" | "bmp" | "ico" => "🖼️",
        | "mp4" | "webm" | "mov" | "avi" | "mkv" => "🎬",
        | "mp3" | "wav" | "ogg" | "flac" | "m4a" => "🎵",
        | "pdf" => "📕",
        | "typ" => "📄",
        | "tex" | "latex" => "📜",
        | "md" | "anote" | "txt" | "rtf" => "📝",
        | "csv" | "tsv" | "xlsx" | "xls" => "📊",
        | "json" | "yaml" | "yml" | "toml" | "xml" => "⚙️",
        | "zip" | "tar" | "gz" | "7z" | "rar" => "📦",
        | "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "html" | "css" | "sh" => "💻",
        | _ => "📁",
    }
}

/// Interactive File Upload Modal Island for Project Detail page.
#[island]
pub fn FileUploadModalIsland(
    #[prop(into)] project_id: String,
    #[prop(into, optional)] folders_json: Option<String>,
    #[prop(into, optional)] existing_files_json: Option<String>,
    #[prop(into, optional)] cur_dir: Option<String>,
    #[prop(optional)] is_zh: Option<bool>,
    #[prop(into, optional)] trigger_label: Option<String>,
    #[prop(into, optional)] trigger_class: Option<String>,
) -> impl IntoView {
    let zh = is_zh.unwrap_or(false);
    let initial_dir = cur_dir.unwrap_or_default();
    let btn_label = trigger_label.unwrap_or_else(|| {
        if zh { "⬆ 上传文件".to_string() } else { "⬆ Upload File".to_string() }
    });
    let btn_class = trigger_class.unwrap_or_else(|| "btn btn-secondary".to_string());

    // Parse folder options
    let folders: Vec<String> = folders_json
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    // Parse existing files list for duplicate pre-check
    let existing_files: Vec<String> = existing_files_json
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    let _project_id_stored = StoredValue::new(project_id);
    let folders_stored = StoredValue::new(folders);
    let existing_files_stored = StoredValue::new(existing_files);

    let is_open = RwSignal::new(false);
    let is_dragover = RwSignal::new(false);
    let selected_name = RwSignal::new(Option::<String>::None);
    let selected_size = RwSignal::new(0u64);
    let dest_folder = RwSignal::new(initial_dir.clone());

    // Upload state signals
    let is_uploading = RwSignal::new(false);
    let is_processing = RwSignal::new(false);
    let is_completed = RwSignal::new(false);
    let progress_pct = RwSignal::new(0.0f64);
    let loaded_bytes = RwSignal::new(0u64);
    let total_bytes = RwSignal::new(0u64);
    let speed_bps = RwSignal::new(0.0f64);
    let eta_secs = RwSignal::new(0u64);
    let status_msg = RwSignal::new(Option::<(bool, String)>::None);

    let reset_upload_state = move || {
        is_uploading.set(false);
        is_processing.set(false);
        is_completed.set(false);
        progress_pct.set(0.0);
        loaded_bytes.set(0);
        total_bytes.set(0);
        speed_bps.set(0.0);
        eta_secs.set(0);
        status_msg.set(None);
    };

    let on_open = move |_| {
        reset_upload_state();
        dest_folder.set(initial_dir.clone());
        is_open.set(true);
    };

    let on_close = move |_| {
        if !is_uploading.get() || is_completed.get() {
            is_open.set(false);
            reset_upload_state();
            selected_name.set(None);
            selected_size.set(0);
        }
    };

    // Check if currently selected file already exists in dest_folder
    let check_is_duplicate = move || {
        let Some(name) = selected_name.get() else { return false };
        let folder = dest_folder.get();
        let folder_clean = folder.trim().trim_matches('/');
        let target_rel = if folder_clean.is_empty() {
            name
        } else {
            format!("{folder_clean}/{name}")
        };
        existing_files_stored.with_value(|list| list.contains(&target_rel))
    };

    // Drag-and-drop handlers
    let on_drag_enter = move |ev: leptos::ev::DragEvent| {
        ev.prevent_default();
        ev.stop_propagation();
        is_dragover.set(true);
    };

    let on_drag_over = move |ev: leptos::ev::DragEvent| {
        ev.prevent_default();
        ev.stop_propagation();
        is_dragover.set(true);
    };

    let on_drag_leave = move |ev: leptos::ev::DragEvent| {
        ev.prevent_default();
        ev.stop_propagation();
        is_dragover.set(false);
    };

    let on_drop = move |ev: leptos::ev::DragEvent| {
        ev.prevent_default();
        ev.stop_propagation();
        is_dragover.set(false);

        #[cfg(feature = "hydrate")]
        {
            use wasm_bindgen::JsCast;
            if let Some(dt) = ev.data_transfer() {
                if let Some(files) = dt.files() {
                    if files.length() > 0 {
                        if let Some(file) = files.get(0) {
                            selected_name.set(Some(file.name()));
                            selected_size.set(file.size() as u64);
                            status_msg.set(None);

                            // Sync with input element
                            if let Some(win) = web_sys::window() {
                                if let Some(doc) = win.document() {
                                    if let Some(el) = doc.get_element_by_id("file-upload-island-input") {
                                        if let Ok(input) = el.dyn_into::<web_sys::HtmlInputElement>() {
                                            let _ = input.set_files(Some(&files));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    };

    let on_file_input_change = move |_ev: leptos::ev::Event| {
        #[cfg(feature = "hydrate")]
        {
            use wasm_bindgen::JsCast;
            let ev = _ev;
            if let Some(target) = ev.target() {
                if let Ok(input) = target.dyn_into::<web_sys::HtmlInputElement>() {
                    if let Some(files) = input.files() {
                        if files.length() > 0 {
                            if let Some(file) = files.get(0) {
                                selected_name.set(Some(file.name()));
                                selected_size.set(file.size() as u64);
                                status_msg.set(None);
                            }
                        }
                    }
                }
            }
        }
    };

    let on_clear_selection = move |_| {
        selected_name.set(None);
        selected_size.set(0);
        status_msg.set(None);
        #[cfg(feature = "hydrate")]
        {
            use wasm_bindgen::JsCast;
            if let Some(win) = web_sys::window() {
                if let Some(doc) = win.document() {
                    if let Some(el) = doc.get_element_by_id("file-upload-island-input") {
                        if let Ok(input) = el.dyn_into::<web_sys::HtmlInputElement>() {
                            input.set_value("");
                        }
                    }
                }
            }
        }
    };

    // Cancellation handler
    let on_cancel_upload = move |_| {
        #[cfg(feature = "hydrate")]
        {
            if let Some(win) = web_sys::window() {
                // Dispatch abort event to active upload
                let _ = win.dispatch_event(&web_sys::CustomEvent::new("apich-abort-upload").unwrap());
            }
        }
    };

    // Upload start handler
    let on_start_upload = move |()| {
        #[cfg(feature = "hydrate")]
        {
            use wasm_bindgen::JsCast;
            use wasm_bindgen::closure::Closure;

            let Some(win) = web_sys::window() else { return };
            let Some(doc) = win.document() else { return };
            let Some(el) = doc.get_element_by_id("file-upload-island-input") else { return };
            let Ok(input) = el.dyn_into::<web_sys::HtmlInputElement>() else { return };

            let Some(files) = input.files() else {
                status_msg.set(Some((false, if zh { "请先选择一个文件".to_string() } else { "Please select a file to upload.".to_string() })));
                return;
            };

            if files.length() == 0 {
                status_msg.set(Some((false, if zh { "请先选择一个文件".to_string() } else { "Please select a file to upload.".to_string() })));
                return;
            }

            let Some(file) = files.get(0) else { return };
            let file_size = file.size() as u64;

            // 200MB limit check client-side
            if file_size > 200 * 1024 * 1024 {
                status_msg.set(Some((false, if zh { "文件超过 200MB 上传限制".to_string() } else { "File exceeds 200MB upload limit.".to_string() })));
                return;
            }

            let form_data = match web_sys::FormData::new() {
                | Ok(fd) => fd,
                | Err(_) => {
                    status_msg.set(Some((false, "Could not initialize upload data.".to_string())));
                    return;
                },
            };

            let target_folder = dest_folder.get();
            let _ = form_data.append_with_blob("file", &file);
            let _ = form_data.append_with_str("folder", &target_folder);

            let xhr = match web_sys::XmlHttpRequest::new() {
                | Ok(x) => x,
                | Err(_) => {
                    status_msg.set(Some((false, "Failed to create XMLHttpRequest.".to_string())));
                    return;
                },
            };

            let project_id_upload = _project_id_stored.get_value();
            let upload_url = format!("/projects/{}/files/upload.json", project_id_upload);
            if xhr.open_with_async("POST", &upload_url, true).is_err() {
                status_msg.set(Some((false, "Failed to open upload connection.".to_string())));
                return;
            }

            is_uploading.set(true);
            is_processing.set(false);
            is_completed.set(false);
            progress_pct.set(0.0);
            loaded_bytes.set(0);
            total_bytes.set(file_size);
            speed_bps.set(0.0);
            eta_secs.set(0);
            status_msg.set(None);

            if let Ok(upload_target) = xhr.upload() {
                // Progress event
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

                    progress_pct.set(pct);
                    loaded_bytes.set(loaded as u64);
                    total_bytes.set(effective_total as u64);

                    let now = js_sys::Date::now();
                    let dt = (now - last_calc_time) / 1000.0;
                    if dt >= 0.2 || pct >= 99.9 {
                        let bytes_diff = loaded - last_loaded;
                        let instant_speed = if dt > 0.0 { (bytes_diff / dt).max(0.0) } else { 0.0 };
                        let total_elapsed = (now - start_time) / 1000.0;
                        let average_speed = if total_elapsed > 0.0 { (loaded / total_elapsed).max(0.0) } else { 0.0 };
                        let current_speed = 0.7 * instant_speed + 0.3 * average_speed;
                        speed_bps.set(current_speed);

                        if current_speed > 10.0 {
                            let remaining_bytes = (effective_total - loaded).max(0.0);
                            let remaining_secs = (remaining_bytes / current_speed).round() as u64;
                            eta_secs.set(remaining_secs);
                        }

                        last_calc_time = now;
                        last_loaded = loaded;
                    }

                    if pct >= 99.99 {
                        is_processing.set(true);
                    }
                }) as Box<dyn FnMut(web_sys::ProgressEvent)>);

                let _ = upload_target.add_event_listener_with_callback("progress", on_progress.as_ref().unchecked_ref());
                on_progress.forget();
            }

            // Load completion event
            let xhr_load = xhr.clone();
            let dest_folder_for_redirect = target_folder.clone();
            let project_id_for_redirect = _project_id_stored.get_value();
            let file_name_str = file.name();

            let on_load = Closure::wrap(Box::new(move |_ev: web_sys::Event| {
                is_uploading.set(false);
                is_processing.set(false);

                let status = xhr_load.status().unwrap_or(0);
                let resp_text = xhr_load.response_text().ok().flatten().unwrap_or_default();

                if status >= 200 && status < 300 {
                    let parse_res: Result<UploadJsonResponse, _> = serde_json::from_str(&resp_text);
                    if let Ok(res) = parse_res {
                        if res.success {
                            is_completed.set(true);
                            progress_pct.set(100.0);
                            loaded_bytes.set(total_bytes.get());
                            let path = res.file_path.unwrap_or_else(|| file_name_str.clone());
                            let msg = if zh {
                                format!("文件上传成功: {path}")
                            } else {
                                format!("Uploaded successfully: {path}")
                            };
                            status_msg.set(Some((true, msg)));

                            // Auto-refresh/redirect to destination folder after 850ms
                            let redirect_url = format!(
                                "/projects/{}?tab=files&dir={}&notice=file_uploaded",
                                project_id_for_redirect,
                                urlencoding::encode(&dest_folder_for_redirect)
                            );
                            if let Some(win) = web_sys::window() {
                                let redirect_closure = Closure::once(Box::new(move || {
                                    if let Some(win) = web_sys::window() {
                                        let _ = win.location().set_href(&redirect_url);
                                    }
                                }));
                                let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(
                                    redirect_closure.as_ref().unchecked_ref(),
                                    850,
                                );
                                redirect_closure.forget();
                            }
                            return;
                        } else {
                            let err = res.error.unwrap_or_else(|| "Server rejected upload.".to_string());
                            status_msg.set(Some((false, err)));
                            return;
                        }
                    }
                }

                // Error fallback
                let err_msg = if status == 413 {
                    if zh { "文件过大 (最大 200MB)" } else { "File too large (maximum 200MB)" }
                } else if status == 403 {
                    if zh { "没有写入权限" } else { "Permission denied" }
                } else if status == 401 {
                    if zh { "登录已过期，请重新登录" } else { "Session expired, please log in" }
                } else {
                    if zh { "上传失败，请重试" } else { "Upload failed, please try again" }
                };
                status_msg.set(Some((false, err_msg.to_string())));
            }) as Box<dyn FnMut(web_sys::Event)>);

            let _ = xhr.add_event_listener_with_callback("load", on_load.as_ref().unchecked_ref());
            on_load.forget();

            // Network error event
            let on_error = Closure::wrap(Box::new(move |_ev: web_sys::Event| {
                is_uploading.set(false);
                is_processing.set(false);
                status_msg.set(Some((false, if zh { "网络连接中断，上传失败" } else { "Network error occurred during upload." }.to_string())));
            }) as Box<dyn FnMut(web_sys::Event)>);
            let _ = xhr.add_event_listener_with_callback("error", on_error.as_ref().unchecked_ref());
            on_error.forget();

            // Abort event
            let on_abort = Closure::wrap(Box::new(move |_ev: web_sys::Event| {
                is_uploading.set(false);
                is_processing.set(false);
                status_msg.set(Some((false, if zh { "上传已取消" } else { "Upload was cancelled." }.to_string())));
            }) as Box<dyn FnMut(web_sys::Event)>);
            let _ = xhr.add_event_listener_with_callback("abort", on_abort.as_ref().unchecked_ref());
            on_abort.forget();

            // Wire abort listener on window
            let xhr_for_abort = xhr.clone();
            let on_custom_abort = Closure::wrap(Box::new(move |_ev: web_sys::CustomEvent| {
                let _ = xhr_for_abort.abort();
            }) as Box<dyn FnMut(web_sys::CustomEvent)>);
            let _ = win.add_event_listener_with_callback("apich-abort-upload", on_custom_abort.as_ref().unchecked_ref());
            on_custom_abort.forget();

            // Send
            let _ = xhr.send_with_opt_form_data(Some(&form_data));
        }
    };

    view! {
        <div style="display:inline-block;">
            // Trigger button
            <button
                type="button"
                class=btn_class
                on:click=on_open
                title=if zh { "上传本地文件到当前项目" } else { "Upload local file to project" }
            >
                {btn_label}
            </button>

            // Modal overlay
            {move || if is_open.get() {
                view! {
                    <div
                        class="modal-backdrop"
                        style="display:flex; position:fixed; inset:0; background:rgba(0,0,0,0.65); backdrop-filter:blur(4px); z-index:9999; align-items:center; justify-content:center; padding:1rem;"
                        on:click=move |ev| {
                            if !is_uploading.get() || is_completed.get() {
                                on_close(ev);
                            }
                        }
                    >
                        <div
                            class="modal-box upload-modal-container"
                            style="background:var(--bg-surface, #1e2029); border:1px solid var(--border-subtle, #333846); border-radius:12px; width:100%; max-width:540px; box-shadow:0 20px 40px rgba(0,0,0,0.4); padding:1.5rem; display:flex; flex-direction:column; gap:1.2rem; color:var(--text-main, #f0f3f6);"
                            on:click=move |ev| ev.stop_propagation()
                        >
                            // Header
                            <div style="display:flex; justify-content:space-between; align-items:center; border-bottom:1px solid var(--border-subtle, #333846); padding-bottom:0.75rem;">
                                <div style="display:flex; align-items:center; gap:0.5rem; font-size:1.15rem; font-weight:600;">
                                    <span style="font-size:1.3rem;">"⬆"</span>
                                    <span>{if zh { "上传文件" } else { "Upload File" }}</span>
                                </div>
                                {move || if !is_uploading.get() || is_completed.get() {
                                    view! {
                                        <button
                                            type="button"
                                            style="background:transparent; border:none; color:var(--text-sub, #9aa0a6); font-size:1.25rem; cursor:pointer; padding:0.25rem 0.5rem; border-radius:4px;"
                                            on:click=on_close
                                            title="Close"
                                        >
                                            "✕"
                                        </button>
                                    }.into_any()
                                } else {
                                    view! { <span></span> }.into_any()
                                }}
                            </div>

                            // Drag & drop dropzone (when not uploading)
                            {move || if !is_uploading.get() && !is_completed.get() {
                                view! {
                                    <div>
                                        // Hidden native file input
                                        <input
                                            type="file"
                                            id="file-upload-island-input"
                                            style="display:none;"
                                            on:change=on_file_input_change
                                        />

                                        // Dropzone UI
                                        {move || {
                                            #[allow(clippy::option_if_let_else)]
                                            match selected_name.get() {
                                                Some(name) => {
                                                    let size_str = format_bytes(selected_size.get());
                                                    let icon = get_file_icon(&name);
                                                    view! {
                                                        <div class="upload-selected-card" style="display:flex; align-items:center; justify-content:space-between; background:var(--bg-elevated, #282c37); border:1px solid var(--border-subtle, #3a3f50); border-radius:8px; padding:0.85rem 1rem;">
                                                            <div style="display:flex; align-items:center; gap:0.75rem; min-width:0;">
                                                                <span style="font-size:1.8rem;">{icon}</span>
                                                                <div style="min-width:0;">
                                                                    <div style="font-weight:600; font-size:0.95rem; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;" title=name.clone()>{name.clone()}</div>
                                                                    <div style="font-size:0.8rem; color:var(--text-sub, #9aa0a6);">{size_str}</div>
                                                                </div>
                                                            </div>
                                                            <button
                                                                type="button"
                                                                class="btn btn-secondary btn-xs"
                                                                on:click=on_clear_selection
                                                                style="font-size:0.8rem; padding:0.35rem 0.65rem;"
                                                            >
                                                                {if zh { "✕ 更改" } else { "✕ Change" }}
                                                            </button>
                                                        </div>
                                                    }.into_any()
                                                }
                                                None => {
                                                    view! {
                                                    <div
                                                        class="upload-dropzone"
                                                        class=("is-dragover", move || is_dragover.get())
                                                        style="border:2px dashed var(--border-subtle, #424758); border-radius:10px; padding:2rem 1.5rem; text-align:center; cursor:pointer; background:var(--bg-elevated, rgba(255,255,255,0.02)); transition:all 0.2s ease;"
                                                        on:dragenter=on_drag_enter
                                                        on:dragover=on_drag_over
                                                        on:dragleave=on_drag_leave
                                                        on:drop=on_drop
                                                        onclick="document.getElementById('file-upload-island-input')?.click()"
                                                    >
                                                        <div style="font-size:2.5rem; margin-bottom:0.5rem;">"☁️"</div>
                                                        <div style="font-weight:600; font-size:1.05rem; margin-bottom:0.25rem;">
                                                            {if zh { "拖拽文件到此处，或点击浏览" } else { "Drag & drop file here, or click to browse" }}
                                                        </div>
                                                        <div style="font-size:0.8rem; color:var(--text-sub, #9aa0a6); margin-bottom:1rem;">
                                                            {if zh { "支持所有类型文件 (视频, 音频, 图片, 文档, 压缩包等) · 单文件最大 200MB" } else { "Any file type (video, audio, images, documents, archives) · Max 200MB" }}
                                                        </div>
                                                        <button
                                                            type="button"
                                                            class="btn btn-secondary btn-sm"
                                                            onclick="event.stopPropagation(); document.getElementById('file-upload-island-input')?.click()"
                                                        >
                                                            {if zh { "📁 选择本地文件" } else { "📁 Select File" }}
                                                        </button>
                                                    </div>
                                                    }.into_any()
                                                }
                                            }
                                        }}

                                        // Destination folder selector
                                        <div style="margin-top:1rem;">
                                            <label style="display:block; font-size:0.85rem; font-weight:600; margin-bottom:0.4rem; color:var(--text-main, #f0f3f6);">
                                                {if zh { "📁 保存至目标文件夹" } else { "📁 Destination Folder" }}
                                            </label>
                                            <select
                                                class="form-control"
                                                style="width:100%; background:var(--bg-elevated, #282c37); border:1px solid var(--border-subtle, #3a3f50); color:var(--text-main, #f0f3f6); padding:0.55rem 0.75rem; border-radius:6px;"
                                                prop:value=move || dest_folder.get()
                                                on:change=move |ev| dest_folder.set(event_target_value(&ev))
                                            >
                                                <option value="">{if zh { "项目根目录 (/)" } else { "Project Root (/)" }}</option>
                                                {folders_stored.with_value(|flds| {
                                                    flds.iter().map(|f| {
                                                        let val = f.clone();
                                                        let label = format!("{f}/");
                                                        view! {
                                                            <option value=val>{label}</option>
                                                        }
                                                    }).collect::<Vec<_>>()
                                                })}
                                            </select>
                                        </div>

                                        // Duplicate warning alert
                                        {move || if check_is_duplicate() {
                                            view! {
                                                <div style="margin-top:0.75rem; padding:0.65rem 0.85rem; background:rgba(234, 179, 8, 0.15); border:1px solid rgba(234, 179, 8, 0.35); border-radius:6px; font-size:0.82rem; color:#facc15; display:flex; align-items:center; gap:0.5rem;">
                                                    <span style="font-size:1.1rem;">"⚠️"</span>
                                                    <span>
                                                        {if zh {
                                                            "所选文件夹中已存在同名文件，上传后将被替换覆盖。"
                                                        } else {
                                                            "A file with this name already exists in this folder and will be replaced."
                                                        }}
                                                    </span>
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! { <span></span> }.into_any()
                                        }}
                                    </div>
                                }.into_any()
                            } else {
                                view! { <span></span> }.into_any()
                            }}

                            // Upload Progress and Live Metrics View (when uploading or completed)
                            {move || if is_uploading.get() || is_completed.get() {
                                let pct = progress_pct.get();
                                let speed_str = format_speed(speed_bps.get());
                                let transferred_str = format!("{}/{}", format_bytes(loaded_bytes.get()), format_bytes(total_bytes.get()));
                                let transferred_title = transferred_str.clone();
                                let eta_str = format_eta(eta_secs.get(), zh);
                                let is_done = is_completed.get();
                                let is_proc = is_processing.get() && !is_done;

                                view! {
                                    <div class="upload-live-metrics-panel" style="display:flex; flex-direction:column; gap:1rem; padding:0.5rem 0;">
                                        // Header file indicator
                                        <div style="display:flex; align-items:center; justify-content:space-between;">
                                            <div style="display:flex; align-items:center; gap:0.5rem; min-width:0;">
                                                <span style="font-size:1.4rem;">{selected_name.get().map_or("📄", |n| get_file_icon(&n))}</span>
                                                <span style="font-weight:600; font-size:0.95rem; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;">
                                                    {selected_name.get().unwrap_or_default()}
                                                </span>
                                            </div>
                                            <div style="font-size:1.1rem; font-weight:700; color:var(--primary, #3b82f6);">
                                                {format!("{pct:.1}%")}
                                            </div>
                                        </div>

                                        // Animated progress bar
                                        <div class="upload-progress-container" style="background:var(--bg-elevated, #282c37); border-radius:10px; height:12px; overflow:hidden; position:relative; box-shadow:inset 0 1px 3px rgba(0,0,0,0.3);">
                                            <div
                                                class="upload-progress-fill"
                                                style=format!("width: {pct:.2}%; height:100%; background: linear-gradient(90deg, #3b82f6, #60a5fa, #38bdf8); transition: width 0.15s ease-out; border-radius:10px; position:relative;")
                                            >
                                                <div class="upload-progress-shine" style="position:absolute; inset:0; background:linear-gradient(90deg, transparent, rgba(255,255,255,0.3), transparent); animation:uploadShine 1.5s infinite linear;"></div>
                                            </div>
                                        </div>

                                        // 4-Card live metrics dashboard
                                        <div class="upload-metrics-grid" style="display:grid; grid-template-columns:repeat(4, 1fr); gap:0.5rem; text-align:center;">
                                            // 1. Progress
                                            <div class="upload-metric-card" style="background:var(--bg-elevated, #282c37); border:1px solid var(--border-subtle, #3a3f50); border-radius:8px; padding:0.6rem 0.4rem;">
                                                <div style="font-size:0.72rem; color:var(--text-sub, #9aa0a6); margin-bottom:0.2rem;">
                                                    {if zh { "进度" } else { "Progress" }}
                                                </div>
                                                <div style="font-weight:700; font-size:0.95rem; color:var(--text-main, #f0f3f6);">
                                                    {format!("{pct:.1}%")}
                                                </div>
                                            </div>

                                            // 2. Speed
                                            <div class="upload-metric-card" style="background:var(--bg-elevated, #282c37); border:1px solid var(--border-subtle, #3a3f50); border-radius:8px; padding:0.6rem 0.4rem;">
                                                <div style="font-size:0.72rem; color:var(--text-sub, #9aa0a6); margin-bottom:0.2rem;">
                                                    {if zh { "上传速度" } else { "Speed" }}
                                                </div>
                                                <div style="font-weight:700; font-size:0.95rem; color:#38bdf8;">
                                                    {if is_proc || is_done { "-".to_string() } else { speed_str }}
                                                </div>
                                            </div>

                                            // 3. Transferred / Total
                                            <div class="upload-metric-card" style="background:var(--bg-elevated, #282c37); border:1px solid var(--border-subtle, #3a3f50); border-radius:8px; padding:0.6rem 0.4rem;">
                                                <div style="font-size:0.72rem; color:var(--text-sub, #9aa0a6); margin-bottom:0.2rem;">
                                                    {if zh { "已传输" } else { "Transferred" }}
                                                </div>
                                                 <div style="font-weight:700; font-size:0.85rem; color:var(--text-main, #f0f3f6); overflow:hidden; text-overflow:ellipsis; white-space:nowrap;" title=transferred_title>
                                                    {transferred_str}
                                                </div>
                                            </div>

                                            // 4. ETA
                                            <div class="upload-metric-card" style="background:var(--bg-elevated, #282c37); border:1px solid var(--border-subtle, #3a3f50); border-radius:8px; padding:0.6rem 0.4rem;">
                                                <div style="font-size:0.72rem; color:var(--text-sub, #9aa0a6); margin-bottom:0.2rem;">
                                                    {if zh { "预计剩余" } else { "Remaining" }}
                                                </div>
                                                <div style="font-weight:700; font-size:0.85rem; color:#a78bfa;">
                                                    {if is_done {
                                                        if zh { "已完成" } else { "Done" }.to_string()
                                                    } else if is_proc {
                                                        if zh { "处理中" } else { "Processing" }.to_string()
                                                    } else {
                                                        eta_str
                                                    }}
                                                </div>
                                            </div>
                                        </div>

                                        // Server processing status indicator
                                        {move || if is_proc {
                                            view! {
                                                <div style="display:flex; align-items:center; justify-content:center; gap:0.5rem; font-size:0.85rem; color:var(--text-sub, #9aa0a6); background:rgba(59, 130, 246, 0.1); border:1px solid rgba(59, 130, 246, 0.25); border-radius:6px; padding:0.5rem;">
                                                    <span class="upload-spin-icon" style="animation:uploadSpin 1s infinite linear;">"⏳"</span>
                                                    <span>{if zh { "数据已传输完毕，正在写入服务器磁盘..." } else { "Upload finished, writing to server disk..." }}</span>
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! { <span></span> }.into_any()
                                        }}
                                    </div>
                                }.into_any()
                            } else {
                                view! { <span></span> }.into_any()
                            }}

                            // Status banner (Success or Error)
                            {move || {
                                status_msg.get().map(|(success, msg)| {
                                    let bg = if success { "rgba(34, 197, 94, 0.15)" } else { "rgba(239, 68, 68, 0.15)" };
                                    let border = if success { "rgba(34, 197, 94, 0.35)" } else { "rgba(239, 68, 68, 0.35)" };
                                    let color = if success { "#4ade80" } else { "#f87171" };
                                    let icon = if success { "✅ " } else { "⚠️ " };
                                    view! {
                                        <div style=format!("background:{bg}; border:1px solid {border}; color:{color}; border-radius:6px; padding:0.75rem 1rem; font-size:0.88rem; display:flex; align-items:center; gap:0.5rem; line-height:1.4;")>
                                            <span style="font-size:1.1rem;">{icon}</span>
                                            <span style="flex:1;">{msg}</span>
                                        </div>
                                    }
                                })
                            }}

                            // Footer Buttons
                            <div style="display:flex; justify-content:flex-end; gap:0.75rem; margin-top:0.5rem; border-top:1px solid var(--border-subtle, #333846); padding-top:1rem;">
                                {move || {
                                    if is_uploading.get() && !is_completed.get() {
                                        view! {
                                            <button
                                                type="button"
                                                class="btn btn-danger"
                                                on:click=on_cancel_upload
                                                style="padding:0.55rem 1.25rem; font-size:0.9rem;"
                                            >
                                                {if zh { "🛑 取消上传" } else { "🛑 Cancel Upload" }}
                                            </button>
                                        }.into_any()
                                    } else if is_completed.get() {
                                        view! {
                                            <button
                                                type="button"
                                                class="btn btn-primary"
                                                on:click=on_close
                                                style="padding:0.55rem 1.25rem; font-size:0.9rem;"
                                            >
                                                {if zh { "完成" } else { "Done" }}
                                            </button>
                                        }.into_any()
                                    } else {
                                        let has_file = selected_name.get().is_some();
                                        view! {
                                            <div style="display:flex; gap:0.75rem;">
                                                <button
                                                    type="button"
                                                    class="btn btn-secondary"
                                                    on:click=on_close
                                                    style="padding:0.55rem 1.25rem; font-size:0.9rem;"
                                                >
                                                    {if zh { "取消" } else { "Cancel" }}
                                                </button>
                                                <button
                                                    type="button"
                                                    class="btn btn-primary"
                                                    disabled=move || !has_file
                                                    on:click=move |_| on_start_upload(())
                                                    style="padding:0.55rem 1.5rem; font-size:0.9rem; font-weight:600;"
                                                >
                                                    {if zh { "开始上传" } else { "Start Upload" }}
                                                </button>
                                            </div>
                                        }.into_any()
                                    }
                                }}
                            </div>
                        </div>
                    </div>
                }.into_any()
            } else {
                view! { <span></span> }.into_any()
            }}
        </div>
    }
}
