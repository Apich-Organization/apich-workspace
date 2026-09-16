//! File action dropdown menu, file operations modal, and table sorting islands.
//!
//! Provides:
//! - `FileActionDropdownIsland`: The `...` button next to each file with 10 options.
//! - `FileOperationsModalIsland`: Reusable modals for Move, Rename, Copy, and Duplicate Alert (Replace or Rename).
//! - `FileTableSortIsland`: Interactive column header sorting for project files table (Name, Type, Size, Modified).
//! - `NewFileDuplicateCheckIsland`: Client-side duplicate file prevention and alert dialog for the "+ New File" modal.

use leptos::prelude::*;
use crate::t;

#[cfg(feature = "hydrate")]
use wasm_bindgen::JsCast;

/// Dropdown menu (`...`) displayed after a file name.
#[island]
pub fn FileActionDropdownIsland(
    #[prop(into)] project_id: String,
    #[prop(into)] file_path: String,
    #[prop(into)] file_name: String,
    #[prop(into)] open_url: String,
    #[prop(into, optional)] open_target: Option<String>,
    #[prop(optional)] is_dir: Option<bool>,
    #[prop(into, optional)] share_mode: Option<String>,
    #[prop(into, optional)] share_role: Option<String>,
    #[prop(into, optional)] share_users_csv: Option<String>,
    #[prop(optional)] is_single_file: Option<bool>,
    #[prop(optional)] is_zh: Option<bool>,
) -> impl IntoView {
    let is_dir_val = is_dir.unwrap_or(false);
    let is_single = is_single_file.unwrap_or(false);
    let zh = is_zh.unwrap_or(false);
    let target = open_target.unwrap_or_else(|| "_self".to_string());
    let sh_mode = share_mode.unwrap_or_else(|| "private".to_string());
    let sh_role = share_role.unwrap_or_else(|| "read".to_string());
    let sh_users = share_users_csv.unwrap_or_default();

    let is_open = RwSignal::new(false);
    let is_favorite = RwSignal::new(false);

    // Check favorite status from localStorage
    init_favorite_state(&project_id, &file_path, is_favorite);

    let proj_id_mv = project_id.clone();
    let file_path_mv = file_path.clone();
    let file_name_mv = file_name.clone();
    let on_move = move |_| {
        is_open.set(false);
        dispatch_move_event(&proj_id_mv, &file_path_mv, &file_name_mv);
    };

    let proj_id_rn = project_id.clone();
    let file_path_rn = file_path.clone();
    let file_name_rn = file_name.clone();
    let on_rename = move |_| {
        is_open.set(false);
        dispatch_rename_event(&proj_id_rn, &file_path_rn, &file_name_rn, is_single);
    };

    let proj_id_del = project_id.clone();
    let file_path_del = file_path.clone();
    let file_name_del = file_name.clone();
    let on_delete = move |_| {
        is_open.set(false);
        execute_delete(&proj_id_del, &file_path_del, &file_name_del, is_single, zh);
    };

    let file_path_sh = file_path.clone();
    let sh_mode_val = sh_mode.clone();
    let sh_role_val = sh_role.clone();
    let sh_users_val = sh_users.clone();
    let on_share = move |_| {
        is_open.set(false);
        dispatch_share_event(&file_path_sh, &sh_mode_val, &sh_role_val, &sh_users_val);
    };

    let proj_id_acc = project_id.clone();
    let file_path_acc = file_path.clone();
    let sh_mode_acc = sh_mode.clone();
    let sh_role_acc = sh_role.clone();
    let sh_users_acc = sh_users.clone();
    let on_manage_access = move |_| {
        is_open.set(false);
        if is_single {
            navigate_to(&format!("/projects/{proj_id_acc}?tab=sharing"));
        } else {
            dispatch_share_event(&file_path_acc, &sh_mode_acc, &sh_role_acc, &sh_users_acc);
        }
    };

    let open_url_cp = open_url.clone();
    let on_copy_link = move |_| {
        is_open.set(false);
        copy_link_to_clipboard(&open_url_cp, zh);
    };

    let proj_id_cp = project_id.clone();
    let file_path_cp = file_path.clone();
    let file_name_cp = file_name.clone();
    let on_copy_to = move |_| {
        is_open.set(false);
        dispatch_copy_event(&proj_id_cp, &file_path_cp, &file_name_cp);
    };

    let proj_id_fav = project_id.clone();
    let file_path_fav = file_path.clone();
    let on_toggle_favorite = move |_| {
        is_open.set(false);
        toggle_favorite(&proj_id_fav, &file_path_fav, is_favorite, zh);
    };

    let download_url = format!(
        "/projects/{}/files/raw?file={}&download=1",
        project_id,
        urlencoding::encode(&file_path)
    );

    let container_ref = NodeRef::<leptos::html::Div>::new();
    wire_outside_click(container_ref, is_open);

    view! {
        <div node_ref=container_ref class="file-more-wrap" style="position:relative; display:inline-flex; align-items:center; gap:0.25rem;">
            {move || if is_favorite.get() {
                view! { <span class="favorite-star-indicator" style="font-size:0.8rem; color:#f59e0b; line-height:1;" title="Favorite">"⭐"</span> }.into_any()
            } else {
                view! { <span></span> }.into_any()
            }}
            <button
                type="button"
                class="file-more-btn"
                class:is-active=move || is_open.get()
                title=t(zh, "More options", "更多选项")
                on:click=move |ev| {
                    ev.stop_propagation();
                    is_open.update(|v| *v = !*v);
                }
            >
                "···"
            </button>
            <div
                class="file-action-menu"
                style:display=move || if is_open.get() { "flex" } else { "none" }
            >
                // 1. Open
                <a href=open_url.clone() target=target class="file-action-item">
                    <span class="action-icon">"📂"</span>
                    <span>{t(zh, "Open", "打开")}</span>
                </a>

                // 2. Move file to folder
                {(!is_dir_val).then(|| view! {
                    <button type="button" class="file-action-item" on:click=on_move>
                        <span class="action-icon">"📁"</span>
                        <span>{t(zh, "Move file to folder", "移动到文件夹")}</span>
                    </button>
                })}

                // 3. Rename file
                <button type="button" class="file-action-item" on:click=on_rename>
                    <span class="action-icon">"✏️"</span>
                    <span>{t(zh, "Rename file", "重命名")}</span>
                </button>

                // 4. Download (files only)
                {(!is_dir_val).then(|| view! {
                    <a href=download_url download=file_name.clone() class="file-action-item">
                        <span class="action-icon">"⬇️"</span>
                        <span>{t(zh, "Download", "下载")}</span>
                    </a>
                })}

                // 5. Share
                <button type="button" class="file-action-item" on:click=on_share>
                    <span class="action-icon">"🔗"</span>
                    <span>{t(zh, "Share", "分享")}</span>
                </button>

                // 6. Manage access
                <button type="button" class="file-action-item" on:click=on_manage_access>
                    <span class="action-icon">"👥"</span>
                    <span>{t(zh, "Manage access", "管理访问权限")}</span>
                </button>

                // 7. Copy link
                <button type="button" class="file-action-item" on:click=on_copy_link>
                    <span class="action-icon">"📋"</span>
                    <span>{t(zh, "Copy link", "复制链接")}</span>
                </button>

                // 8. Copy to (files only)
                {(!is_dir_val).then(|| view! {
                    <button type="button" class="file-action-item" on:click=on_copy_to>
                        <span class="action-icon">"📑"</span>
                        <span>{t(zh, "Copy to", "复制到文件夹")}</span>
                    </button>
                })}

                // 9. Add to favorite / Remove
                <button type="button" class="file-action-item" on:click=on_toggle_favorite>
                    <span class="action-icon">{move || if is_favorite.get() { "⭐" } else { "☆" }}</span>
                    <span>{move || if is_favorite.get() { t(zh, "Remove from favorite", "取消收藏") } else { t(zh, "Add to favorite", "添加到收藏") }}</span>
                </button>

                <div class="file-action-divider"></div>

                // 10. Delete
                <button type="button" class="file-action-item item-danger" on:click=on_delete>
                    <span class="action-icon">"🗑️"</span>
                    <span>{t(zh, "Delete", "删除")}</span>
                </button>
            </div>
        </div>
    }
}

/// Modals for Move, Rename, Copy, and Duplicate Alert (Replace or Rename).
#[island]
pub fn FileOperationsModalIsland(
    #[prop(into)] project_id: String,
    #[prop(into)] folders_json: String,
    #[prop(into)] existing_files_json: String,
    #[prop(optional)] is_zh: Option<bool>,
) -> impl IntoView {
    let zh = is_zh.unwrap_or(false);

    let folders: Vec<String> = serde_json::from_str(&folders_json).unwrap_or_default();
    let existing_files: Vec<String> = serde_json::from_str(&existing_files_json).unwrap_or_default();

    // Move modal state
    let move_visible = RwSignal::new(false);
    let move_file = RwSignal::new(String::new());
    let move_name = RwSignal::new(String::new());
    let move_target_folder = RwSignal::new(String::new());

    // Rename modal state
    let rename_visible = RwSignal::new(false);
    let rename_file = RwSignal::new(String::new());
    let rename_name = RwSignal::new(String::new());
    let rename_is_single = RwSignal::new(false);

    // Copy modal state
    let copy_visible = RwSignal::new(false);
    let copy_file = RwSignal::new(String::new());
    let copy_name = RwSignal::new(String::new());
    let copy_target_folder = RwSignal::new(String::new());

    // Duplicate alert modal state
    let dup_visible = RwSignal::new(false);
    let dup_filename = RwSignal::new(String::new());
    let dup_action_type = RwSignal::new(String::new()); // "new_file", "move", "rename", "copy"
    let dup_pending_data = RwSignal::new(String::new());

    // Wire custom event listeners
    wire_operation_listeners(
        move_visible,
        move_file,
        move_name,
        rename_visible,
        rename_file,
        rename_name,
        rename_is_single,
        copy_visible,
        copy_file,
        copy_name,
        dup_visible,
        dup_filename,
        dup_action_type,
        dup_pending_data,
    );

    let proj_id_for_move = project_id.clone();
    let folders_for_move = folders.clone();
    let ex_files_mv = existing_files.clone();
    let on_submit_move = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let target_dir = move_target_folder.get();
        let filename = move_name.get();
        let candidate_path = if target_dir.trim().is_empty() {
            filename.clone()
        } else {
            format!("{}/{}", target_dir.trim().trim_matches('/'), filename)
        };

        if ex_files_mv.iter().any(|f| f.eq_ignore_ascii_case(&candidate_path)) {
            dup_filename.set(candidate_path);
            dup_action_type.set("move".to_string());
            let payload = serde_json::json!({
                "file": move_file.get(),
                "dest_folder": target_dir
            }).to_string();
            dup_pending_data.set(payload);
            dup_visible.set(true);
        } else {
            submit_form_post(
                &format!("/projects/{proj_id_for_move}/files/move"),
                &[
                    ("file", &move_file.get()),
                    ("dest_folder", &target_dir),
                ],
            );
            move_visible.set(false);
        }
    };

    let proj_id_for_rename = project_id.clone();
    let ex_files_rn = existing_files.clone();
    let on_submit_rename = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let old_path = rename_file.get();
        let new_name = rename_name.get().trim().to_string();
        let is_single = rename_is_single.get();

        if is_single {
            submit_form_post(
                &format!("/projects/{proj_id_for_rename}/rename"),
                &[("name", &new_name)],
            );
            rename_visible.set(false);
            return;
        }

        let parent = std::path::Path::new(&old_path)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or_default();
        let candidate_path = if parent.is_empty() {
            new_name.clone()
        } else {
            format!("{parent}/{new_name}")
        };

        if ex_files_rn.iter().any(|f| f.eq_ignore_ascii_case(&candidate_path)) {
            dup_filename.set(candidate_path);
            dup_action_type.set("rename".to_string());
            let payload = serde_json::json!({
                "file": old_path,
                "new_name": new_name
            }).to_string();
            dup_pending_data.set(payload);
            dup_visible.set(true);
        } else {
            submit_form_post(
                &format!("/projects/{proj_id_for_rename}/files/rename"),
                &[
                    ("file", &old_path),
                    ("new_name", &new_name),
                ],
            );
            rename_visible.set(false);
        }
    };

    let proj_id_for_copy = project_id.clone();
    let folders_for_copy = folders.clone();
    let ex_files_cp = existing_files.clone();
    let on_submit_copy = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let target_dir = copy_target_folder.get();
        let filename = copy_name.get();
        let candidate_path = if target_dir.trim().is_empty() {
            filename.clone()
        } else {
            format!("{}/{}", target_dir.trim().trim_matches('/'), filename)
        };

        if ex_files_cp.iter().any(|f| f.eq_ignore_ascii_case(&candidate_path)) {
            dup_filename.set(candidate_path);
            dup_action_type.set("copy".to_string());
            let payload = serde_json::json!({
                "file": copy_file.get(),
                "dest_folder": target_dir
            }).to_string();
            dup_pending_data.set(payload);
            dup_visible.set(true);
        } else {
            submit_form_post(
                &format!("/projects/{proj_id_for_copy}/files/copy"),
                &[
                    ("file", &copy_file.get()),
                    ("dest_folder", &target_dir),
                ],
            );
            copy_visible.set(false);
        }
    };

    // Duplicate dialog actions
    let proj_id_for_dup = project_id.clone();
    let ex_files_dup = existing_files.clone();
    let on_dup_replace = move |_| {
        let act = dup_action_type.get();
        let data = dup_pending_data.get();
        dup_visible.set(false);
        move_visible.set(false);
        rename_visible.set(false);
        copy_visible.set(false);

        match act.as_str() {
            "new_file" => {
                // Submit pending new file form with replace=true
                submit_new_file_with_replace();
            }
            "move" => {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&data) {
                    let file = v.get("file").and_then(|s| s.as_str()).unwrap_or_default();
                    let dest = v.get("dest_folder").and_then(|s| s.as_str()).unwrap_or_default();
                    submit_form_post(
                        &format!("/projects/{proj_id_for_dup}/files/move"),
                        &[("file", file), ("dest_folder", dest), ("replace", "true")],
                    );
                }
            }
            "rename" => {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&data) {
                    let file = v.get("file").and_then(|s| s.as_str()).unwrap_or_default();
                    let new_name = v.get("new_name").and_then(|s| s.as_str()).unwrap_or_default();
                    submit_form_post(
                        &format!("/projects/{proj_id_for_dup}/files/rename"),
                        &[("file", file), ("new_name", new_name), ("replace", "true")],
                    );
                }
            }
            "copy" => {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&data) {
                    let file = v.get("file").and_then(|s| s.as_str()).unwrap_or_default();
                    let dest = v.get("dest_folder").and_then(|s| s.as_str()).unwrap_or_default();
                    submit_form_post(
                        &format!("/projects/{proj_id_for_dup}/files/copy"),
                        &[("file", file), ("dest_folder", dest), ("replace", "true")],
                    );
                }
            }
            _ => {}
        }
    };

    let proj_id_for_dup_rn = project_id.clone();
    let on_dup_rename = move |_| {
        let act = dup_action_type.get();
        let target_path = dup_filename.get();
        let suggested_path = compute_unique_name(&target_path, &ex_files_dup);
        let suggested_name = std::path::Path::new(&suggested_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&suggested_path)
            .to_string();

        dup_visible.set(false);

        match act.as_str() {
            "new_file" => {
                // Update filename in new file form and submit
                update_new_file_name_and_submit(&suggested_name);
            }
            "move" => {
                let data = dup_pending_data.get();
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&data) {
                    let file = v.get("file").and_then(|s| s.as_str()).unwrap_or_default();
                    let dest = v.get("dest_folder").and_then(|s| s.as_str()).unwrap_or_default();
                    // Copy to new destination name then delete source (or move with rename)
                    submit_form_post(
                        &format!("/projects/{proj_id_for_dup_rn}/files/copy"),
                        &[("file", file), ("dest_folder", dest), ("new_name", &suggested_name)],
                    );
                }
            }
            "rename" => {
                let data = dup_pending_data.get();
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&data) {
                    let file = v.get("file").and_then(|s| s.as_str()).unwrap_or_default();
                    submit_form_post(
                        &format!("/projects/{proj_id_for_dup_rn}/files/rename"),
                        &[("file", file), ("new_name", &suggested_name)],
                    );
                }
            }
            "copy" => {
                let data = dup_pending_data.get();
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&data) {
                    let file = v.get("file").and_then(|s| s.as_str()).unwrap_or_default();
                    let dest = v.get("dest_folder").and_then(|s| s.as_str()).unwrap_or_default();
                    submit_form_post(
                        &format!("/projects/{proj_id_for_dup_rn}/files/copy"),
                        &[("file", file), ("dest_folder", dest), ("new_name", &suggested_name)],
                    );
                }
            }
            _ => {}
        }
    };

    view! {
        // MOVE FILE MODAL
        <div class="modal-backdrop" style:display=move || if move_visible.get() { "flex" } else { "none" }>
            <div class="modal-card" style="max-width:500px; width:min(500px, 95vw); border-radius:12px; padding:1.5rem;">
                <div class="modal-header">
                    <h3 class="modal-title">"📁 " {t(zh, "Move File to Folder", "移动文件到文件夹")}</h3>
                    <button type="button" class="modal-close" on:click=move |_| move_visible.set(false)>"×"</button>
                </div>
                <form on:submit=on_submit_move>
                    <p style="font-size:0.85rem; color:var(--text-sub); margin-bottom:1rem;">
                        {t(zh, "Moving file: ", "正在移动文件: ")}
                        <strong style="color:var(--text-main);">{move || move_name.get()}</strong>
                    </p>
                    <div class="form-group">
                        <label>{t(zh, "Destination Folder", "目标文件夹")}</label>
                        <select
                            class="form-control"
                            prop:value=move || move_target_folder.get()
                            on:change=move |ev| move_target_folder.set(event_target_value(&ev))
                        >
                            <option value="">{t(zh, "/ (Project Root)", "/ (项目根目录)")}</option>
                            {folders_for_move.iter().map(|f| {
                                view! { <option value=f.clone()>{format!("/{f}")}</option> }
                            }).collect::<Vec<_>>()}
                        </select>
                    </div>
                    <div style="display:flex; justify-content:flex-end; gap:0.6rem; margin-top:1.5rem;">
                        <button type="button" class="btn btn-secondary" on:click=move |_| move_visible.set(false)>
                            {t(zh, "Cancel", "取消")}
                        </button>
                        <button type="submit" class="btn btn-primary">
                            {t(zh, "Move File", "确认移动")}
                        </button>
                    </div>
                </form>
            </div>
        </div>

        // RENAME FILE MODAL
        <div class="modal-backdrop" style:display=move || if rename_visible.get() { "flex" } else { "none" }>
            <div class="modal-card" style="max-width:480px; width:min(480px, 95vw); border-radius:12px; padding:1.5rem;">
                <div class="modal-header">
                    <h3 class="modal-title">"✏️ " {t(zh, "Rename File", "重命名文件")}</h3>
                    <button type="button" class="modal-close" on:click=move |_| rename_visible.set(false)>"×"</button>
                </div>
                <form on:submit=on_submit_rename>
                    <div class="form-group">
                        <label>{t(zh, "File Name", "文件名称")}</label>
                        <input
                            type="text"
                            class="form-control"
                            required=true
                            prop:value=move || rename_name.get()
                            on:input=move |ev| rename_name.set(event_target_value(&ev))
                        />
                    </div>
                    <div style="display:flex; justify-content:flex-end; gap:0.6rem; margin-top:1.5rem;">
                        <button type="button" class="btn btn-secondary" on:click=move |_| rename_visible.set(false)>
                            {t(zh, "Cancel", "取消")}
                        </button>
                        <button type="submit" class="btn btn-primary">
                            {t(zh, "Rename", "确认重命名")}
                        </button>
                    </div>
                </form>
            </div>
        </div>

        // COPY FILE MODAL
        <div class="modal-backdrop" style:display=move || if copy_visible.get() { "flex" } else { "none" }>
            <div class="modal-card" style="max-width:500px; width:min(500px, 95vw); border-radius:12px; padding:1.5rem;">
                <div class="modal-header">
                    <h3 class="modal-title">"📑 " {t(zh, "Copy File to Folder", "复制文件到文件夹")}</h3>
                    <button type="button" class="modal-close" on:click=move |_| copy_visible.set(false)>"×"</button>
                </div>
                <form on:submit=on_submit_copy>
                    <p style="font-size:0.85rem; color:var(--text-sub); margin-bottom:1rem;">
                        {t(zh, "Copying file: ", "正在复制文件: ")}
                        <strong style="color:var(--text-main);">{move || copy_name.get()}</strong>
                    </p>
                    <div class="form-group">
                        <label>{t(zh, "Destination Folder", "目标文件夹")}</label>
                        <select
                            class="form-control"
                            prop:value=move || copy_target_folder.get()
                            on:change=move |ev| copy_target_folder.set(event_target_value(&ev))
                        >
                            <option value="">{t(zh, "/ (Project Root)", "/ (项目根目录)")}</option>
                            {folders_for_copy.iter().map(|f| {
                                view! { <option value=f.clone()>{format!("/{f}")}</option> }
                            }).collect::<Vec<_>>()}
                        </select>
                    </div>
                    <div style="display:flex; justify-content:flex-end; gap:0.6rem; margin-top:1.5rem;">
                        <button type="button" class="btn btn-secondary" on:click=move |_| copy_visible.set(false)>
                            {t(zh, "Cancel", "取消")}
                        </button>
                        <button type="submit" class="btn btn-primary">
                            {t(zh, "Copy File", "确认复制")}
                        </button>
                    </div>
                </form>
            </div>
        </div>

        // DUPLICATE ALERT MODAL (Replace or Rename)
        <div class="modal-backdrop" style:display=move || if dup_visible.get() { "flex" } else { "none" } style="z-index:1100;">
            <div class="modal-card" style="max-width:480px; width:min(480px, 95vw); border-radius:14px; padding:1.75rem; border:1px solid rgba(245, 158, 11, 0.4); box-shadow:0 20px 40px rgba(0,0,0,0.3);">
                <div style="display:flex; align-items:center; gap:0.75rem; margin-bottom:1rem;">
                    <div style="width:40px; height:40px; border-radius:50%; background:rgba(245, 158, 11, 0.15); display:flex; align-items:center; justify-content:center; font-size:1.4rem;">
                        "⚠️"
                    </div>
                    <div>
                        <h3 class="modal-title" style="margin:0; font-size:1.15rem; color:var(--text-main);">
                            {t(zh, "File Already Exists", "文件已存在")}
                        </h3>
                        <p style="margin:0.2rem 0 0; font-size:0.8rem; color:var(--text-sub);">
                            {t(zh, "A file with this name already exists in destination.", "目标位置已存在同名文件。")}
                        </p>
                    </div>
                </div>
                <div style="background:var(--bg-muted); border-radius:8px; padding:0.75rem 1rem; margin-bottom:1.25rem; font-family:var(--font-mono); font-size:0.85rem; color:var(--text-main); word-break:break-all;">
                    {move || dup_filename.get()}
                </div>
                <p style="font-size:0.85rem; color:var(--text-muted); margin-bottom:1.5rem; line-height:1.5;">
                    {t(
                        zh,
                        "Would you like to replace the existing file, or automatically rename this file with a unique number?",
                        "您想要替换已存在的文件，还是自动重命名为递增的不冲突名称？"
                    )}
                </p>
                <div style="display:flex; justify-content:flex-end; gap:0.6rem; flex-wrap:wrap;">
                    <button type="button" class="btn btn-secondary" on:click=move |_| dup_visible.set(false)>
                        {t(zh, "Cancel", "取消")}
                    </button>
                    <button type="button" class="btn btn-secondary" style="border-color:var(--primary); color:var(--primary);" on:click=on_dup_rename>
                        "✨ " {t(zh, "Rename File", "重命名保存")}
                    </button>
                    <button type="button" class="btn btn-danger" on:click=on_dup_replace>
                        "⚠️ " {t(zh, "Replace Existing", "替换已有文件")}
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Interactive table column header sorting (Name, Type, Size, Modified).
#[island]
pub fn FileTableSortIsland(
    #[prop(into, optional)] table_id: Option<String>,
    #[prop(optional)] is_zh: Option<bool>,
) -> impl IntoView {
    let tbl_id = table_id.unwrap_or_else(|| "project-files-table".to_string());
    let _zh = is_zh.unwrap_or(false);

    wire_table_sorter(tbl_id);

    view! {
        <span style="display:none;" aria-hidden="true"></span>
    }
}

/// Attaches duplicate detection to the "+ New File" creation form.
#[island]
pub fn NewFileDuplicateCheckIsland(
    #[prop(into)] form_selector: String,
    #[prop(into)] existing_files_json: String,
    #[prop(optional)] is_zh: Option<bool>,
) -> impl IntoView {
    let _zh = is_zh.unwrap_or(false);
    wire_new_file_validator(form_selector, existing_files_json);

    view! {
        <span style="display:none;" aria-hidden="true"></span>
    }
}

// ---------------------------------------------------------------------------
// HYDRATION / DOM LOGIC
// ---------------------------------------------------------------------------

#[cfg(feature = "hydrate")]
fn init_favorite_state(project_id: &str, file_path: &str, is_favorite: RwSignal<bool>) {
    let key = format!("{}:{}", project_id, file_path);
    if let Some(win) = web_sys::window() {
        if let Ok(Some(storage)) = win.local_storage() {
            if let Ok(Some(raw)) = storage.get_item("apich_file_favorites") {
                if let Ok(list) = serde_json::from_str::<Vec<String>>(&raw) {
                    if list.contains(&key) {
                        is_favorite.set(true);
                    }
                }
            }
        }
    }
}
#[cfg(not(feature = "hydrate"))]
fn init_favorite_state(_project_id: &str, _file_path: &str, _is_favorite: RwSignal<bool>) {}

#[cfg(feature = "hydrate")]
fn toggle_favorite(project_id: &str, file_path: &str, is_favorite: RwSignal<bool>, is_zh: bool) {
    let key = format!("{}:{}", project_id, file_path);
    if let Some(win) = web_sys::window() {
        if let Ok(Some(storage)) = win.local_storage() {
            let mut list: Vec<String> = storage
                .get_item("apich_file_favorites")
                .ok()
                .flatten()
                .and_then(|raw| serde_json::from_str(&raw).ok())
                .unwrap_or_default();

            if list.contains(&key) {
                list.retain(|k| k != &key);
                is_favorite.set(false);
                show_toast(t(is_zh, "Removed from favorites", "已从收藏中移除"));
            } else {
                list.push(key);
                is_favorite.set(true);
                show_toast(t(is_zh, "Added to favorites ⭐", "已添加到收藏 ⭐"));
            }

            if let Ok(serialized) = serde_json::to_string(&list) {
                let _ = storage.set_item("apich_file_favorites", &serialized);
            }
        }
    }
}
#[cfg(not(feature = "hydrate"))]
fn toggle_favorite(_project_id: &str, _file_path: &str, _is_favorite: RwSignal<bool>, _is_zh: bool) {}

#[cfg(feature = "hydrate")]
fn copy_link_to_clipboard(path: &str, is_zh: bool) {
    if let Some(win) = web_sys::window() {
        let origin = win.location().origin().unwrap_or_default();
        let full_url = if path.starts_with("http://") || path.starts_with("https://") {
            path.to_string()
        } else {
            format!("{origin}{path}")
        };
        let _ = win.navigator().clipboard().write_text(&full_url);
        show_toast(t(is_zh, "✓ Link copied to clipboard", "✓ 链接已复制到剪贴板"));
    }
}
#[cfg(not(feature = "hydrate"))]
fn copy_link_to_clipboard(_path: &str, _is_zh: bool) {}

#[cfg(feature = "hydrate")]
fn show_toast(msg: &'static str) {
    if let Some(win) = web_sys::window() {
        if let Some(doc) = win.document() {
            let toast = doc.create_element("div").ok();
            if let Some(el) = toast {
                el.set_class_name("apich-toast-notification");
                el.set_text_content(Some(msg));
                el.set_attribute("style", "position:fixed; bottom:24px; right:24px; z-index:9999; background:rgba(15,23,42,0.92); color:#fff; padding:10px 18px; border-radius:8px; box-shadow:0 8px 24px rgba(0,0,0,0.3); font-size:0.85rem; font-weight:500; backdrop-filter:blur(8px); animation:fadeIn 0.2s ease; border:1px solid rgba(255,255,255,0.1);").ok();
                if let Some(body) = doc.body() {
                    let _ = body.append_child(&el);
                    let el_clone = el.clone();
                    gloo_timers::callback::Timeout::new(2500, move || {
                        let _ = el_clone.remove();
                    }).forget();
                }
            }
        }
    }
}
#[cfg(not(feature = "hydrate"))]
fn show_toast(_msg: &'static str) {}

#[cfg(feature = "hydrate")]
fn dispatch_move_event(project_id: &str, file_path: &str, file_name: &str) {
    if let Some(win) = web_sys::window() {
        let detail = serde_json::json!({
            "project_id": project_id,
            "path": file_path,
            "name": file_name,
        }).to_string();
        let init = web_sys::CustomEventInit::new();
        init.set_detail(&wasm_bindgen::JsValue::from_str(&detail));
        if let Ok(ev) = web_sys::CustomEvent::new_with_event_init_dict("apich-open-file-move-modal", &init) {
            let _ = win.dispatch_event(&ev);
        }
    }
}
#[cfg(not(feature = "hydrate"))]
fn dispatch_move_event(_project_id: &str, _file_path: &str, _file_name: &str) {}

#[cfg(feature = "hydrate")]
fn dispatch_rename_event(project_id: &str, file_path: &str, file_name: &str, is_single: bool) {
    if let Some(win) = web_sys::window() {
        let detail = serde_json::json!({
            "project_id": project_id,
            "path": file_path,
            "name": file_name,
            "is_single": is_single,
        }).to_string();
        let init = web_sys::CustomEventInit::new();
        init.set_detail(&wasm_bindgen::JsValue::from_str(&detail));
        if let Ok(ev) = web_sys::CustomEvent::new_with_event_init_dict("apich-open-file-rename-modal", &init) {
            let _ = win.dispatch_event(&ev);
        }
    }
}
#[cfg(not(feature = "hydrate"))]
fn dispatch_rename_event(_project_id: &str, _file_path: &str, _file_name: &str, _is_single: bool) {}

#[cfg(feature = "hydrate")]
fn dispatch_copy_event(project_id: &str, file_path: &str, file_name: &str) {
    if let Some(win) = web_sys::window() {
        let detail = serde_json::json!({
            "project_id": project_id,
            "path": file_path,
            "name": file_name,
        }).to_string();
        let init = web_sys::CustomEventInit::new();
        init.set_detail(&wasm_bindgen::JsValue::from_str(&detail));
        if let Ok(ev) = web_sys::CustomEvent::new_with_event_init_dict("apich-open-file-copy-modal", &init) {
            let _ = win.dispatch_event(&ev);
        }
    }
}
#[cfg(not(feature = "hydrate"))]
fn dispatch_copy_event(_project_id: &str, _file_path: &str, _file_name: &str) {}

#[cfg(feature = "hydrate")]
fn dispatch_share_event(path: &str, mode: &str, role: &str, users: &str) {
    if let Some(win) = web_sys::window() {
        let detail = serde_json::json!({
            "path": path,
            "mode": mode,
            "role": role,
            "users": users,
        }).to_string();
        let init = web_sys::CustomEventInit::new();
        init.set_detail(&wasm_bindgen::JsValue::from_str(&detail));
        if let Ok(ev) = web_sys::CustomEvent::new_with_event_init_dict("apich-open-share-modal", &init) {
            let _ = win.dispatch_event(&ev);
        }
    }
}
#[cfg(not(feature = "hydrate"))]
fn dispatch_share_event(_path: &str, _mode: &str, _role: &str, _users: &str) {}

#[cfg(feature = "hydrate")]
fn execute_delete(project_id: &str, file_path: &str, file_name: &str, is_single: bool, is_zh: bool) {
    if let Some(win) = web_sys::window() {
        let confirm_msg = if is_single {
            format!(
                "{}{}{}",
                t(is_zh, "Permanently delete \"", "确定永久删除 \""),
                file_name,
                t(is_zh, "\"? This cannot be undone.", "\"？此操作无法撤销。")
            )
        } else {
            format!(
                "{}{}{}",
                t(is_zh, "Delete file \"", "确定删除文件 \""),
                file_name,
                t(is_zh, "\"? This cannot be undone.", "\"？此操作无法撤销。")
            )
        };

        if win.confirm_with_message(&confirm_msg).unwrap_or(false) {
            if is_single {
                submit_form_post(&format!("/projects/{project_id}/delete"), &[]);
            } else {
                submit_form_post(&format!("/projects/{project_id}/files/delete"), &[("file", file_path)]);
            }
        }
    }
}
#[cfg(not(feature = "hydrate"))]
fn execute_delete(_project_id: &str, _file_path: &str, _file_name: &str, _is_single: bool, _is_zh: bool) {}

#[cfg(feature = "hydrate")]
fn navigate_to(url: &str) {
    if let Some(win) = web_sys::window() {
        let _ = win.location().set_href(url);
    }
}
#[cfg(not(feature = "hydrate"))]
fn navigate_to(_url: &str) {}

#[cfg(feature = "hydrate")]
fn submit_form_post(action: &str, fields: &[(&str, &str)]) {
    if let Some(win) = web_sys::window() {
        if let Some(doc) = win.document() {
            if let Ok(form) = doc.create_element("form") {
                let _ = form.set_attribute("method", "post");
                let _ = form.set_attribute("action", action);
                let _ = form.set_attribute("style", "display:none;");

                for (name, val) in fields {
                    if let Ok(input) = doc.create_element("input") {
                        let _ = input.set_attribute("type", "hidden");
                        let _ = input.set_attribute("name", name);
                        let _ = input.set_attribute("value", val);
                        let _ = form.append_child(&input);
                    }
                }

                if let Some(body) = doc.body() {
                    let _ = body.append_child(&form);
                    if let Ok(f) = form.dyn_into::<web_sys::HtmlFormElement>() {
                        let _ = f.submit();
                    }
                }
            }
        }
    }
}
#[cfg(not(feature = "hydrate"))]
fn submit_form_post(_action: &str, _fields: &[(&str, &str)]) {}

#[cfg(feature = "hydrate")]
fn wire_outside_click(container: NodeRef<leptos::html::Div>, is_open: RwSignal<bool>) {
    Effect::new(move |_| {
        use wasm_bindgen::closure::Closure;
        use wasm_bindgen::JsCast;

        let cb = Closure::wrap(Box::new(move |ev: web_sys::MouseEvent| {
            if !is_open.get_untracked() {
                return;
            }
            if let Some(node) = container.get_untracked() {
                if let Some(target) = ev.target() {
                    if let Ok(target_node) = target.dyn_into::<web_sys::Node>() {
                        let el: &web_sys::Node = &node;
                        if !el.contains(Some(&target_node)) {
                            is_open.set(false);
                        }
                    }
                }
            }
        }) as Box<dyn FnMut(web_sys::MouseEvent)>);

        if let Some(win) = web_sys::window() {
            let _ = win.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref());
        }
        cb.forget();
    });
}
#[cfg(not(feature = "hydrate"))]
fn wire_outside_click(_container: NodeRef<leptos::html::Div>, _is_open: RwSignal<bool>) {}

#[cfg(feature = "hydrate")]
fn wire_operation_listeners(
    move_visible: RwSignal<bool>,
    move_file: RwSignal<String>,
    move_name: RwSignal<String>,
    rename_visible: RwSignal<bool>,
    rename_file: RwSignal<String>,
    rename_name: RwSignal<String>,
    rename_is_single: RwSignal<bool>,
    copy_visible: RwSignal<bool>,
    copy_file: RwSignal<String>,
    copy_name: RwSignal<String>,
    dup_visible: RwSignal<bool>,
    dup_filename: RwSignal<String>,
    dup_action_type: RwSignal<String>,
    dup_pending_data: RwSignal<String>,
) {
    Effect::new(move |_| {
        use wasm_bindgen::closure::Closure;
        use wasm_bindgen::JsCast;

        // Move listener
        let mv_cb = Closure::wrap(Box::new(move |ev: web_sys::CustomEvent| {
            if let Some(detail_str) = ev.detail().as_string() {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&detail_str) {
                    let path = v.get("path").and_then(|s| s.as_str()).unwrap_or_default();
                    let name = v.get("name").and_then(|s| s.as_str()).unwrap_or_default();
                    move_file.set(path.to_string());
                    move_name.set(name.to_string());
                    move_visible.set(true);
                }
            }
        }) as Box<dyn FnMut(web_sys::CustomEvent)>);

        // Rename listener
        let rn_cb = Closure::wrap(Box::new(move |ev: web_sys::CustomEvent| {
            if let Some(detail_str) = ev.detail().as_string() {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&detail_str) {
                    let path = v.get("path").and_then(|s| s.as_str()).unwrap_or_default();
                    let name = v.get("name").and_then(|s| s.as_str()).unwrap_or_default();
                    let single = v.get("is_single").and_then(|s| s.as_bool()).unwrap_or(false);
                    rename_file.set(path.to_string());
                    rename_name.set(name.to_string());
                    rename_is_single.set(single);
                    rename_visible.set(true);
                }
            }
        }) as Box<dyn FnMut(web_sys::CustomEvent)>);

        // Copy listener
        let cp_cb = Closure::wrap(Box::new(move |ev: web_sys::CustomEvent| {
            if let Some(detail_str) = ev.detail().as_string() {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&detail_str) {
                    let path = v.get("path").and_then(|s| s.as_str()).unwrap_or_default();
                    let name = v.get("name").and_then(|s| s.as_str()).unwrap_or_default();
                    copy_file.set(path.to_string());
                    copy_name.set(name.to_string());
                    copy_visible.set(true);
                }
            }
        }) as Box<dyn FnMut(web_sys::CustomEvent)>);

        // Duplicate alert listener
        let dup_cb = Closure::wrap(Box::new(move |ev: web_sys::CustomEvent| {
            if let Some(detail_str) = ev.detail().as_string() {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&detail_str) {
                    let filename = v.get("filename").and_then(|s| s.as_str()).unwrap_or_default();
                    let action = v.get("action").and_then(|s| s.as_str()).unwrap_or_default();
                    let payload = v.get("payload").map(|p| p.to_string()).unwrap_or_default();
                    dup_filename.set(filename.to_string());
                    dup_action_type.set(action.to_string());
                    dup_pending_data.set(payload);
                    dup_visible.set(true);
                }
            }
        }) as Box<dyn FnMut(web_sys::CustomEvent)>);

        if let Some(win) = web_sys::window() {
            let _ = win.add_event_listener_with_callback("apich-open-file-move-modal", mv_cb.as_ref().unchecked_ref());
            let _ = win.add_event_listener_with_callback("apich-open-file-rename-modal", rn_cb.as_ref().unchecked_ref());
            let _ = win.add_event_listener_with_callback("apich-open-file-copy-modal", cp_cb.as_ref().unchecked_ref());
            let _ = win.add_event_listener_with_callback("apich-check-duplicate", dup_cb.as_ref().unchecked_ref());
        }
        mv_cb.forget();
        rn_cb.forget();
        cp_cb.forget();
        dup_cb.forget();
    });
}
#[cfg(not(feature = "hydrate"))]
fn wire_operation_listeners(
    _move_visible: RwSignal<bool>,
    _move_file: RwSignal<String>,
    _move_name: RwSignal<String>,
    _rename_visible: RwSignal<bool>,
    _rename_file: RwSignal<String>,
    _rename_name: RwSignal<String>,
    _rename_is_single: RwSignal<bool>,
    _copy_visible: RwSignal<bool>,
    _copy_file: RwSignal<String>,
    _copy_name: RwSignal<String>,
    _dup_visible: RwSignal<bool>,
    _dup_filename: RwSignal<String>,
    _dup_action_type: RwSignal<String>,
    _dup_pending_data: RwSignal<String>,
) {}

#[cfg(feature = "hydrate")]
fn submit_new_file_with_replace() {
    if let Some(win) = web_sys::window() {
        if let Some(doc) = win.document() {
            if let Ok(Some(form)) = doc.query_selector("form[action$=\"/files/new\"]") {
                if let Ok(input) = doc.create_element("input") {
                    let _ = input.set_attribute("type", "hidden");
                    let _ = input.set_attribute("name", "replace");
                    let _ = input.set_attribute("value", "true");
                    let _ = form.append_child(&input);
                }
                if let Ok(f) = form.dyn_into::<web_sys::HtmlFormElement>() {
                    let _ = f.submit();
                }
            }
        }
    }
}
#[cfg(not(feature = "hydrate"))]
fn submit_new_file_with_replace() {}

#[cfg(feature = "hydrate")]
fn update_new_file_name_and_submit(new_name: &str) {
    use wasm_bindgen::JsCast;
    if let Some(win) = web_sys::window() {
        if let Some(doc) = win.document() {
            if let Ok(Some(form)) = doc.query_selector("form[action$=\"/files/new\"]") {
                if let Ok(Some(input)) = form.query_selector("input[name=\"filename\"]") {
                    if let Ok(in_el) = input.dyn_into::<web_sys::HtmlInputElement>() {
                        in_el.set_value(new_name);
                    }
                }
                if let Ok(f) = form.dyn_into::<web_sys::HtmlFormElement>() {
                    let _ = f.submit();
                }
            }
        }
    }
}
#[cfg(not(feature = "hydrate"))]
fn update_new_file_name_and_submit(_new_name: &str) {}

#[must_use]
pub fn compute_unique_name(candidate_path: &str, existing: &[String]) -> String {
    let p = std::path::Path::new(candidate_path);
    let parent = p.parent().and_then(|p| p.to_str()).unwrap_or_default();
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");

    let mut index: usize = 1;
    loop {
        let test_name = if ext.is_empty() {
            format!("{stem} ({index})")
        } else {
            format!("{stem} ({index}).{ext}")
        };

        let test_path = if parent.is_empty() {
            test_name
        } else {
            format!("{parent}/{test_name}")
        };

        if !existing.iter().any(|f| f.eq_ignore_ascii_case(&test_path)) {
            return test_path;
        }
        index = index.saturating_add(1);
    }
}

#[cfg(feature = "hydrate")]
fn wire_new_file_validator(form_selector: String, existing_files_json: String) {
    Effect::new(move |_| {
        use wasm_bindgen::closure::Closure;
        use wasm_bindgen::JsCast;

        let existing: Vec<String> = serde_json::from_str(&existing_files_json).unwrap_or_default();
        let existing_rc = std::rc::Rc::new(existing);

        if let Some(win) = web_sys::window() {
            if let Some(doc) = win.document() {
                if let Ok(Some(form_el)) = doc.query_selector(&form_selector) {
                    let ex = existing_rc.clone();
                    let form_clone = form_el.clone();
                    let cb = Closure::wrap(Box::new(move |ev: web_sys::SubmitEvent| {
                        let form = &form_clone;
                        let filename_input = form.query_selector("input[name=\"filename\"]").ok().flatten();
                        let folder_select = form.query_selector("select[name=\"folder\"]").ok().flatten();
                        let template_select = form.query_selector("select[name=\"template\"]").ok().flatten();

                        let raw_name = filename_input
                            .and_then(|el| el.dyn_into::<web_sys::HtmlInputElement>().ok())
                            .map(|i| i.value().trim().to_string())
                            .unwrap_or_default();

                        if raw_name.is_empty() {
                            return;
                        }

                        let folder = folder_select
                            .and_then(|el| el.dyn_into::<web_sys::HtmlSelectElement>().ok())
                            .map(|s| s.value().trim().trim_matches('/').to_string())
                            .unwrap_or_default();

                        let template = template_select
                            .and_then(|el| el.dyn_into::<web_sys::HtmlSelectElement>().ok())
                            .map(|s| s.value().trim().to_string())
                            .unwrap_or_default();

                        let has_ext = std::path::Path::new(&raw_name).extension().is_some();
                        let ext = match template.as_str() {
                            "typst" | "slide" => Some("typ"),
                            "latex" => Some("tex"),
                            "note" => Some("anote"),
                            "table" => Some("table"),
                            "script_python" => Some("py"),
                            "script_r" => Some("R"),
                            "script_rust" => Some("rs"),
                            _ => None,
                        };

                        let with_ext = if !has_ext {
                            if let Some(e) = ext {
                                format!("{raw_name}.{e}")
                            } else {
                                raw_name.clone()
                            }
                        } else {
                            raw_name.clone()
                        };

                        let candidate = if folder.is_empty() {
                            with_ext
                        } else {
                            format!("{folder}/{with_ext}")
                        };

                        if ex.iter().any(|f| f.eq_ignore_ascii_case(&candidate)) {
                            ev.prevent_default();

                            // Dispatch duplicate alert modal
                            if let Some(w) = web_sys::window() {
                                let detail = serde_json::json!({
                                    "filename": candidate,
                                    "action": "new_file",
                                    "payload": raw_name
                                }).to_string();
                                let init = web_sys::CustomEventInit::new();
                                init.set_detail(&wasm_bindgen::JsValue::from_str(&detail));
                                if let Ok(custom) = web_sys::CustomEvent::new_with_event_init_dict("apich-check-duplicate", &init) {
                                    let _ = w.dispatch_event(&custom);
                                }
                            }
                        }
                    }) as Box<dyn FnMut(web_sys::SubmitEvent)>);

                    let _ = form_el.add_event_listener_with_callback("submit", cb.as_ref().unchecked_ref());
                    cb.forget();
                }
            }
        }
    });
}
#[cfg(not(feature = "hydrate"))]
fn wire_new_file_validator(_form_selector: String, _existing_files_json: String) {}

#[cfg(feature = "hydrate")]
fn wire_table_sorter(table_id: String) {
    Effect::new(move |_| {
        use wasm_bindgen::closure::Closure;
        use wasm_bindgen::JsCast;

        if let Some(win) = web_sys::window() {
            if let Some(doc) = win.document() {
                if let Some(tbl) = doc.get_element_by_id(&table_id) {
                    let sortable_ths = tbl.query_selector_all("th.sortable-th").ok();
                    if let Some(ths) = sortable_ths {
                        let current_col = std::rc::Rc::new(std::cell::RefCell::new(String::from("name")));
                        let is_asc = std::rc::Rc::new(std::cell::RefCell::new(true));

                        for i in 0..ths.length() {
                            if let Some(th_node) = ths.get(i) {
                                if let Ok(th) = th_node.dyn_into::<web_sys::HtmlTableCellElement>() {
                                    let tbl_rc = tbl.clone();
                                    let cur_col_rc = current_col.clone();
                                    let is_asc_rc = is_asc.clone();

                                    let cb = Closure::wrap(Box::new(move |ev: web_sys::MouseEvent| {
                                        let target = ev.current_target();
                                        let Some(target_th) = target.and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return; };
                                        let col = target_th.get_attribute("data-sort-col").unwrap_or_default();
                                        if col.is_empty() { return; }

                                        let mut asc = is_asc_rc.borrow_mut();
                                        let mut cur = cur_col_rc.borrow_mut();

                                        if *cur == col {
                                            *asc = !*asc;
                                        } else {
                                            *cur = col.clone();
                                            // Date and Size default to descending on first click (newest / largest first)
                                            if col == "size" || col == "modified" {
                                                *asc = false;
                                            } else {
                                                *asc = true;
                                            }
                                        }

                                        let asc_val = *asc;
                                        sort_table_rows(&tbl_rc, &col, asc_val);
                                        update_header_indicators(&tbl_rc, &col, asc_val);
                                    }) as Box<dyn FnMut(web_sys::MouseEvent)>);

                                    let _ = th.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref());
                                    cb.forget();
                                }
                            }
                        }
                    }
                }
            }
        }
    });
}
#[cfg(not(feature = "hydrate"))]
fn wire_table_sorter(_table_id: String) {}

#[cfg(feature = "hydrate")]
fn update_header_indicators(tbl: &web_sys::Element, active_col: &str, asc: bool) {
    if let Ok(ths) = tbl.query_selector_all("th.sortable-th") {
        for i in 0..ths.length() {
            if let Some(th_node) = ths.get(i) {
                if let Ok(th) = th_node.dyn_into::<web_sys::Element>() {
                    let col = th.get_attribute("data-sort-col").unwrap_or_default();
                    if let Ok(Some(icon)) = th.query_selector(".sort-icon") {
                        if col == active_col {
                            let _ = th.class_list().remove_1("sorted-asc");
                            let _ = th.class_list().remove_1("sorted-desc");
                            if asc {
                                let _ = th.class_list().add_1("sorted-asc");
                                icon.set_text_content(Some(" ▲"));
                            } else {
                                let _ = th.class_list().add_1("sorted-desc");
                                icon.set_text_content(Some(" ▼"));
                            }
                        } else {
                            let _ = th.class_list().remove_1("sorted-asc");
                            let _ = th.class_list().remove_1("sorted-desc");
                            icon.set_text_content(Some(" ↕"));
                        }
                    }
                }
            }
        }
    }
}

#[cfg(feature = "hydrate")]
fn sort_table_rows(tbl: &web_sys::Element, col: &str, asc: bool) {
    let Some(tbody) = tbl.query_selector("tbody").ok().flatten() else { return; };
    let rows_nl = tbody.children();
    let len = rows_nl.length();
    if len <= 1 { return; }

    let mut parent_row: Option<web_sys::Element> = None;
    let mut rows: Vec<web_sys::Element> = Vec::new();

    for i in 0..len {
        if let Some(el) = rows_nl.get_with_index(i) {
            if el.get_attribute("data-is-parent").as_deref() == Some("1") {
                parent_row = Some(el);
            } else {
                rows.push(el);
            }
        }
    }

    rows.sort_by(|a, b| {
        let is_dir_a = a.get_attribute("data-is-dir").as_deref() == Some("1");
        let is_dir_b = b.get_attribute("data-is-dir").as_deref() == Some("1");

        // Keep directories at top regardless of column
        if is_dir_a && !is_dir_b {
            return std::cmp::Ordering::Less;
        }
        if !is_dir_a && is_dir_b {
            return std::cmp::Ordering::Greater;
        }

        let ord = match col {
            "size" => {
                let s_a: u64 = a.get_attribute("data-size").and_then(|s| s.parse().ok()).unwrap_or(0);
                let s_b: u64 = b.get_attribute("data-size").and_then(|s| s.parse().ok()).unwrap_or(0);
                s_a.cmp(&s_b)
            }
            "modified" => {
                let m_a = a.get_attribute("data-modified").unwrap_or_default();
                let m_b = b.get_attribute("data-modified").unwrap_or_default();
                m_a.cmp(&m_b)
            }
            "type" => {
                let t_a = a.get_attribute("data-type").unwrap_or_default().to_lowercase();
                let t_b = b.get_attribute("data-type").unwrap_or_default().to_lowercase();
                t_a.cmp(&t_b)
            }
            "sharing" => {
                let sh_a = a.get_attribute("data-sharing").unwrap_or_default().to_lowercase();
                let sh_b = b.get_attribute("data-sharing").unwrap_or_default().to_lowercase();
                sh_a.cmp(&sh_b)
            }
            _ => { // "name" default
                let n_a = a.get_attribute("data-name").unwrap_or_default().to_lowercase();
                let n_b = b.get_attribute("data-name").unwrap_or_default().to_lowercase();
                n_a.cmp(&n_b)
            }
        };

        if asc {
            ord
        } else {
            ord.reverse()
        }
    });

    // Re-append in sorted order
    if let Some(parent) = parent_row {
        let _ = tbody.append_child(&parent);
    }
    for r in rows {
        let _ = tbody.append_child(&r);
    }
}
