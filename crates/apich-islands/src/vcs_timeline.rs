//! Interactive VCS Timeline, Diff Viewer & Snapshot Restoration Island
//!
//! Provides:
//! - Visual chronological timeline rail with node states, commit messages, author/hash, and date clustering
//! - Search & filter by commit message, author, or file path
//! - Click-to-inspect snapshot metadata and changed file pills (+ added, ~ modified, - removed)
//! - Line-by-line colorized unified diff viewer (+ additions, - deletions, line numbers)
//! - Restoration actions:
//!   - Mode A: Replace current working copy file (with automatic safety snapshot)
//!   - Mode B: Restore as a brand new file (pre-populated with suggested name)
//!   - Mode C: Roll back entire workspace to snapshot

use leptos::prelude::*;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotTimelineItem {
    pub id: String,
    pub parent_snapshot_id: Option<String>,
    pub created_at: String,
    pub message: String,
    pub author: String,
    pub is_milestone: bool,
    pub milestone_name: Option<String>,
    pub tree_hash: String,
    pub is_verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotDetailsDto {
    pub id: String,
    pub parent_snapshot_id: Option<String>,
    pub created_at: String,
    pub message: String,
    pub author: String,
    pub is_milestone: bool,
    pub milestone_name: Option<String>,
    pub tree_hash: String,
    pub gpg_signature: Option<String>,
    pub added_files: Vec<String>,
    pub modified_files: Vec<String>,
    pub removed_files: Vec<String>,
    pub all_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLineDto {
    pub kind: String, // "added", "removed", "context"
    pub old_lineno: Option<usize>,
    pub new_lineno: Option<usize>,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiffDto {
    pub file_path: String,
    pub is_binary: bool,
    pub status: String,
    pub old_content: Option<String>,
    pub new_content: Option<String>,
    pub diff_lines: Vec<DiffLineDto>,
}

#[derive(Debug, Clone, Serialize)]
struct RestorePayload {
    file: String,
    target_file: String,
}

#[island]
pub fn VcsTimelineIsland(
    #[prop(into)] project_id: String,
    #[prop(into)] initial_snapshots: Vec<SnapshotTimelineItem>,
    #[prop(into)] active_file: String,
    #[prop(into)] is_zh: bool,
) -> impl IntoView {
    let initial_selected_id = initial_snapshots.first().map(|s| s.id.clone());
    let initial_file_filter = active_file;

    let search_query = RwSignal::new(String::new());
    let milestone_only = RwSignal::new(false);
    let file_filter = RwSignal::new(initial_file_filter.clone());

    let selected_snapshot_id = RwSignal::new(initial_selected_id);
    let selected_snapshot_details = RwSignal::new(None::<SnapshotDetailsDto>);
    let is_loading_details = RwSignal::new(false);

    let selected_diff_file = RwSignal::new(if initial_file_filter.is_empty() {
        None
    } else {
        Some(initial_file_filter)
    });
    let current_diff = RwSignal::new(None::<FileDiffDto>);
    let is_loading_diff = RwSignal::new(false);

    let status_banner = RwSignal::new(None::<(bool, String)>); // (is_success, message)
    let restore_modal_open = RwSignal::new(false);
    let restore_modal_mode = RwSignal::new("replace".to_string()); // "replace" | "new" | "revert"
    let restore_target_filename = RwSignal::new(String::new());
    let is_restoring = RwSignal::new(false);

    let backdrop_ref = NodeRef::<leptos::html::Div>::new();
    crate::modal::reparent_to_body(backdrop_ref);

    // Helper to generate a default target filename for "Restore to new file"
    let make_default_target_name = |source_path: &str, snap_id: &str| -> String {
        let short_id = if snap_id.len() >= 7 { &snap_id[..7] } else { snap_id };
        source_path.rfind('.').map_or_else(
            || format!("{source_path}_v_{short_id}"),
            |pos| {
                let stem = &source_path[..pos];
                let ext = &source_path[pos..];
                format!("{stem}_v_{short_id}{ext}")
            },
        )
    };

    // Filter snapshots based on user search and filters
    let snapshots_pool = initial_snapshots;
    let filtered_snapshots = Memo::new(move |_| {
        let q = search_query.get().to_lowercase().trim().to_string();
        let ms_only = milestone_only.get();

        snapshots_pool
            .iter()
            .filter(|s| {
                if ms_only && !s.is_milestone {
                    return false;
                }
                if !q.is_empty() {
                    let match_msg = s.message.to_lowercase().contains(&q);
                    let match_author = s.author.to_lowercase().contains(&q);
                    let match_id = s.id.to_lowercase().contains(&q);
                    let match_ms = s.milestone_name.as_deref().unwrap_or("").to_lowercase().contains(&q);
                    if !match_msg && !match_author && !match_id && !match_ms {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect::<Vec<_>>()
    });

    // Reactive effect to fetch snapshot details whenever selected_snapshot_id changes
    #[cfg(feature = "hydrate")]
    {
        let p_id = project_id.clone();
        Effect::new(move |_| {
            let Some(snap_id) = selected_snapshot_id.get() else {
                selected_snapshot_details.set(None);
                return;
            };
            let project_id = p_id.clone();
            is_loading_details.set(true);

            leptos::task::spawn_local(async move {
                let url = format!("/api/projects/{project_id}/vcs/snapshots/{snap_id}/details");
                let resp = gloo_net::http::Request::get(&url).send().await;
                match resp {
                    Ok(res) if res.ok() => {
                        if let Ok(details) = res.json::<SnapshotDetailsDto>().await {
                            let curr_f = selected_diff_file.get_untracked();
                            let is_curr_valid = curr_f.as_ref().map_or(false, |f| {
                                details.all_files.contains(f)
                                    || details.modified_files.contains(f)
                                    || details.added_files.contains(f)
                                    || details.removed_files.contains(f)
                            });

                            if !is_curr_valid {
                                let auto_pick = details.modified_files.first().cloned()
                                    .or_else(|| details.added_files.first().cloned())
                                    .or_else(|| details.removed_files.first().cloned())
                                    .or_else(|| details.all_files.first().cloned());
                                selected_diff_file.set(auto_pick);
                            }
                            selected_snapshot_details.set(Some(details));
                        } else {
                            selected_snapshot_details.set(None);
                        }
                    }
                    _ => {
                        selected_snapshot_details.set(None);
                    }
                }
                is_loading_details.set(false);
            });
        });
    }

    // Reactive effect to fetch line diff whenever (selected_snapshot_id, selected_diff_file) change
    #[cfg(feature = "hydrate")]
    {
        let p_id = project_id.clone();
        Effect::new(move |_| {
            let snap_id = selected_snapshot_id.get();
            let file_opt = selected_diff_file.get();

            let (Some(s_id), Some(rel_file)) = (snap_id, file_opt) else {
                current_diff.set(None);
                return;
            };

            let project_id = p_id.clone();
            is_loading_diff.set(true);

            leptos::task::spawn_local(async move {
                let encoded_file = urlencoding::encode(&rel_file);
                let url = format!("/api/projects/{project_id}/vcs/snapshots/{s_id}/diff?file={encoded_file}");
                let resp = gloo_net::http::Request::get(&url).send().await;
                match resp {
                    Ok(res) if res.ok() => {
                        if let Ok(dto) = res.json::<FileDiffDto>().await {
                            current_diff.set(Some(dto));
                        } else {
                            current_diff.set(None);
                        }
                    }
                    _ => {
                        current_diff.set(None);
                    }
                }
                is_loading_diff.set(false);
            });
        });
    }

    let p_id_for_action = project_id;

    view! {
        <div class="vcs-timeline-container">
            // Filter and Search Toolbar
            <div class="vcs-timeline-toolbar">
                <div class="vcs-toolbar-search">
                    <span class="vcs-search-icon">"🔍"</span>
                    <input
                        type="text"
                        class="form-control form-control-sm vcs-search-input"
                        placeholder=crate::t(is_zh, "Search snapshots by message, author, or hash...", "按提交信息、作者或快照哈希搜索...")
                        prop:value=move || search_query.get()
                        on:input=move |ev| search_query.set(event_target_value(&ev))
                    />
                    {move || {
                        if search_query.get().is_empty() {
                            None
                        } else {
                            Some(view! {
                                <button
                                    type="button"
                                    class="vcs-clear-btn"
                                    on:click=move |_| search_query.set(String::new())
                                >"✕"</button>
                            })
                        }
                    }}
                </div>

                <div class="vcs-toolbar-actions">
                    <button
                        type="button"
                        class=move || {
                            if milestone_only.get() {
                                "btn btn-sm btn-primary"
                            } else {
                                "btn btn-sm btn-secondary"
                            }
                        }
                        on:click=move |_| milestone_only.update(|v| *v = !*v)
                    >
                        "🏁 " {crate::t(is_zh, "Milestones Only", "仅里程碑")}
                    </button>
                    {move || {
                        let ff = file_filter.get();
                        if ff.is_empty() {
                            None
                        } else {
                            Some(view! {
                                <span class="vcs-file-filter-badge">
                                    "📄 " {ff}
                                    <button
                                        type="button"
                                        class="vcs-badge-close"
                                        title=crate::t(is_zh, "Clear file filter", "清除文件筛选")
                                        on:click=move |_| {
                                            file_filter.set(String::new());
                                            selected_diff_file.set(None);
                                        }
                                    >"✕"</button>
                                </span>
                            })
                        }
                    }}
                </div>
            </div>

            // Optional Notification Banner
            {move || {
                status_banner.get().map(|(is_succ, msg)| {
                    let cls = if is_succ { "alert alert-success vcs-status-banner" } else { "alert alert-danger vcs-status-banner" };
                    view! {
                        <div class=cls>
                            <span>{msg}</span>
                            <button
                                type="button"
                                class="vcs-banner-close"
                                on:click=move |_| status_banner.set(None)
                            >"✕"</button>
                        </div>
                    }
                })
            }}

            // Two-column Main Workspace: Left Timeline Rail | Right Snapshot Inspector & Diff
            <div class="vcs-main-split">
                // Left Column: Interactive Timeline Rail
                <div class="vcs-timeline-rail">
                    <div class="vcs-rail-header">
                        <span class="vcs-rail-title">{crate::t(is_zh, "Snapshot History", "快照历史")}</span>
                        <span class="vcs-rail-count">{move || filtered_snapshots.get().len()} {crate::t(is_zh, "commits", "次记录")}</span>
                    </div>

                    <div class="vcs-rail-scroll">
                        {move || {
                            let items = filtered_snapshots.get();
                            if items.is_empty() {
                                view! {
                                    <div class="empty-state" style="padding:1.5rem;">
                                        <div style="font-size:1.5rem; margin-bottom:0.5rem;">"🔍"</div>
                                        <div class="empty-title" style="font-size:0.9rem;">{crate::t(is_zh, "No snapshots match", "无匹配快照")}</div>
                                    </div>
                                }.into_any()
                            } else {
                                let total = items.len();
                                let nodes: Vec<_> = items
                                    .into_iter()
                                    .enumerate()
                                    .map(|(idx, s)| {
                                        let sid = s.id.clone();
                                        let sid_clone = s.id.clone();
                                        let is_head = idx == 0;
                                        let is_last = idx == total.saturating_sub(1);
                                        let short_id = if s.id.len() >= 8 { s.id[..8].to_string() } else { s.id.clone() };

                                        view! {
                                            <div
                                                class="vcs-node-item"
                                                class:selected=move || selected_snapshot_id.get().as_deref() == Some(&sid)
                                                class:is-head=is_head
                                                on:click=move |_| {
                                                    selected_snapshot_id.set(Some(sid_clone.clone()));
                                                }
                                            >
                                                <div class="vcs-node-track">
                                                    <div class="vcs-track-line-top" class:hidden=is_head></div>
                                                    <div
                                                        class="vcs-node-dot"
                                                        class:milestone=s.is_milestone
                                                        class:head=is_head
                                                    >
                                                        {if s.is_milestone { "🏁" } else if is_head { "●" } else { "○" }}
                                                    </div>
                                                    <div class="vcs-track-line-bottom" class:hidden=is_last></div>
                                                </div>

                                                <div class="vcs-node-info">
                                                    <div class="vcs-node-top">
                                                        <span class="vcs-node-msg" title=s.message.clone()>{s.message.clone()}</span>
                                                        {if s.is_verified {
                                                             Some(view! { <span class="vcs-verified-badge" title=crate::t(is_zh, "GPG Verified", "GPG 已验证")>"🛡️"</span> })
                                                        } else {
                                                            None
                                                        }}
                                                    </div>
                                                    <div class="vcs-node-meta">
                                                        <code class="vcs-short-hash">{short_id}</code>
                                                        <span class="vcs-node-time">{s.created_at}</span>
                                                        {s.milestone_name.map(|ms| view! { <span class="badge badge-active vcs-ms-badge">"🏁 " {ms}</span> })}
                                                    </div>
                                                </div>
                                            </div>
                                        }
                                    })
                                    .collect();

                                view! { <div class="vcs-nodes-list">{nodes}</div> }.into_any()
                            }
                        }}
                    </div>
                </div>

                // Right Column: Snapshot Details & Diff Viewer
                <div class="vcs-inspector-column">
                    {move || {
                        if is_loading_details.get() {
                            return view! {
                                <div class="vcs-inspector-loading">
                                    <div class="spinner"></div>
                                    <span>{crate::t(is_zh, "Loading snapshot details...", "正在加载快照详情...")}</span>
                                </div>
                            }.into_any();
                        }

                        let details_opt = selected_snapshot_details.get();
                        let Some(details) = details_opt else {
                            return view! {
                                <div class="empty-state" style="padding:4rem 2rem;">
                                    <div style="font-size:2.5rem; margin-bottom:1rem;">"🌿"</div>
                                    <h3 class="empty-title">{crate::t(is_zh, "Select a Snapshot", "请选择一个快照")}</h3>
                                    <p class="empty-desc">
                                        {crate::t(is_zh, "Click any snapshot on the left timeline rail to inspect its changed files, diffs, and restore options.", "点击左侧时间线上的任意快照以查看其变更文件、代码差异及恢复选项。")}
                                    </p>
                                </div>
                            }.into_any();
                        };

                        let snap_id = details.id.clone();
                        let short_id = if snap_id.len() >= 8 { snap_id[..8].to_string() } else { snap_id.clone() };
                        let short_tree = if details.tree_hash.len() >= 8 { details.tree_hash[..8].to_string() } else { details.tree_hash.clone() };

                        let added_count = details.added_files.len();
                        let mod_count = details.modified_files.len();
                        let rem_count = details.removed_files.len();
                        let total_changed = added_count.saturating_add(mod_count).saturating_add(rem_count);

                        view! {
                            <div class="vcs-inspector-card">
                                // Snapshot Header Card
                                <div class="vcs-snap-header">
                                    <div class="vcs-snap-title-row">
                                        <h3 class="vcs-snap-message">{details.message.clone()}</h3>
                                        // Actions Toolbar
                                        <div class="vcs-snap-actions">
                                            {move || {
                                                let sel_file = selected_diff_file.get();
                                                let has_file = sel_file.is_some();
                                                let sf = sel_file.unwrap_or_default();
                                                let sf_for_replace = sf.clone();
                                                let sid = snap_id.clone();
                                                let default_target = make_default_target_name(&sf, &sid);
                                                let dt_for_new = default_target;

                                                view! {
                                                    <div class="btn-group">
                                                        <button
                                                            type="button"
                                                            class="btn btn-sm btn-primary"
                                                            disabled=!has_file
                                                            title=crate::t(is_zh, "Restore and replace current working copy of this file", "恢复并覆盖此文件的当前工作区版本")
                                                            on:click=move |_| {
                                                                restore_modal_mode.set("replace".to_string());
                                                                restore_target_filename.set(sf_for_replace.clone());
                                                                restore_modal_open.set(true);
                                                            }
                                                        >
                                                            "🔄 " {crate::t(is_zh, "Restore File (Replace)", "恢复文件 (覆盖)")}
                                                        </button>
                                                        <button
                                                            type="button"
                                                            class="btn btn-sm btn-secondary"
                                                            disabled=!has_file
                                                            title=crate::t(is_zh, "Restore snapshot version as a brand new file", "将快照版本恢复为全新文件")
                                                            on:click=move |_| {
                                                                restore_modal_mode.set("new".to_string());
                                                                restore_target_filename.set(dt_for_new.clone());
                                                                restore_modal_open.set(true);
                                                            }
                                                        >
                                                            "📄 " {crate::t(is_zh, "Restore as New File...", "恢复为新文件...")}
                                                        </button>
                                                        <button
                                                            type="button"
                                                            class="btn btn-sm btn-outline-danger"
                                                            title=crate::t(is_zh, "Roll back whole project to this snapshot", "将整个项目回滚到此快照")
                                                            on:click=move |_| {
                                                                restore_modal_mode.set("revert".to_string());
                                                                restore_modal_open.set(true);
                                                            }
                                                        >
                                                            "⏪ " {crate::t(is_zh, "Rollback Project", "回滚项目")}
                                                        </button>
                                                    </div>
                                                }
                                            }}
                                        </div>
                                    </div>

                                    <div class="vcs-snap-meta-grid">
                                        <div class="vcs-meta-item">
                                            <span class="vcs-meta-label">{crate::t(is_zh, "Snapshot ID:", "快照哈希:")}</span>
                                            <code class="vcs-meta-val">{short_id}</code>
                                        </div>
                                        <div class="vcs-meta-item">
                                            <span class="vcs-meta-label">{crate::t(is_zh, "Tree Hash:", "目录树哈希:")}</span>
                                            <code class="vcs-meta-val">{short_tree}</code>
                                        </div>
                                        <div class="vcs-meta-item">
                                            <span class="vcs-meta-label">{crate::t(is_zh, "Author:", "提交人:")}</span>
                                            <span class="vcs-meta-val">{details.author.clone()}</span>
                                        </div>
                                        <div class="vcs-meta-item">
                                            <span class="vcs-meta-label">{crate::t(is_zh, "Timestamp:", "时间:")}</span>
                                            <span class="vcs-meta-val">{details.created_at.clone()}</span>
                                        </div>
                                    </div>
                                </div>

                                // Changed Files Section
                                <div class="vcs-files-section">
                                    <div class="vcs-files-header">
                                        <span class="vcs-files-title">
                                            {crate::t(is_zh, "Changed Files", "变更文件")}
                                            " (" {total_changed} ")"
                                        </span>
                                        <div class="vcs-files-stats">
                                            {if added_count > 0 {
                                                Some(view! { <span class="vcs-stat-added">"+" {added_count} " added"</span> })
                                            } else { None }}
                                            {if mod_count > 0 {
                                                Some(view! { <span class="vcs-stat-mod">"~" {mod_count} " modified"</span> })
                                            } else { None }}
                                            {if rem_count > 0 {
                                                Some(view! { <span class="vcs-stat-rem">"-" {rem_count} " removed"</span> })
                                            } else { None }}
                                        </div>
                                    </div>

                                    <div class="vcs-file-pills-list">
                                        // Added files
                                        {details.added_files.iter().map(|f| {
                                            let file_str = f.clone();
                                            let f_clone = f.clone();
                                            view! {
                                                <button
                                                    type="button"
                                                    class="vcs-file-pill pill-added"
                                                    class:active=move || selected_diff_file.get().as_deref() == Some(&file_str)
                                                    on:click=move |_| selected_diff_file.set(Some(f_clone.clone()))
                                                >
                                                    <span class="pill-badge">"+"</span>
                                                    <span class="pill-name">{f.clone()}</span>
                                                </button>
                                            }
                                        }).collect::<Vec<_>>()}

                                        // Modified files
                                        {details.modified_files.iter().map(|f| {
                                            let file_str = f.clone();
                                            let f_clone = f.clone();
                                            view! {
                                                <button
                                                    type="button"
                                                    class="vcs-file-pill pill-mod"
                                                    class:active=move || selected_diff_file.get().as_deref() == Some(&file_str)
                                                    on:click=move |_| selected_diff_file.set(Some(f_clone.clone()))
                                                >
                                                    <span class="pill-badge">"~"</span>
                                                    <span class="pill-name">{f.clone()}</span>
                                                </button>
                                            }
                                        }).collect::<Vec<_>>()}

                                        // Removed files
                                        {details.removed_files.iter().map(|f| {
                                            let file_str = f.clone();
                                            let f_clone = f.clone();
                                            view! {
                                                <button
                                                    type="button"
                                                    class="vcs-file-pill pill-rem"
                                                    class:active=move || selected_diff_file.get().as_deref() == Some(&file_str)
                                                    on:click=move |_| selected_diff_file.set(Some(f_clone.clone()))
                                                >
                                                    <span class="pill-badge">"-"</span>
                                                    <span class="pill-name">{f.clone()}</span>
                                                </button>
                                            }
                                        }).collect::<Vec<_>>()}

                                        // If initial snapshot with all files
                                        {if total_changed == 0 && !details.all_files.is_empty() {
                                            details.all_files.iter().take(20).map(|f| {
                                                let file_str = f.clone();
                                                let f_clone = f.clone();
                                                view! {
                                                    <button
                                                        type="button"
                                                        class="vcs-file-pill pill-added"
                                                        class:active=move || selected_diff_file.get().as_deref() == Some(&file_str)
                                                        on:click=move |_| selected_diff_file.set(Some(f_clone.clone()))
                                                    >
                                                        <span class="pill-badge">"•"</span>
                                                        <span class="pill-name">{f.clone()}</span>
                                                    </button>
                                                }
                                            }).collect::<Vec<_>>()
                                        } else {
                                            Vec::new()
                                        }}
                                    </div>
                                </div>

                                // Line-level Diff Viewer Section
                                <div class="vcs-diff-section">
                                    {move || {
                                        if is_loading_diff.get() {
                                            return view! {
                                                <div class="vcs-diff-loading">
                                                    <div class="spinner"></div>
                                                    <span>{crate::t(is_zh, "Computing diff...", "正在计算差异...")}</span>
                                                </div>
                                            }.into_any();
                                        }

                                        let diff_opt = current_diff.get();
                                        let Some(diff) = diff_opt else {
                                            return view! {
                                                <div class="vcs-diff-placeholder">
                                                    <span>{crate::t(is_zh, "Select a changed file above to inspect its line diff", "点击上方变更文件以查看行级代码差异")}</span>
                                                </div>
                                            }.into_any();
                                        };

                                        if diff.is_binary {
                                            return view! {
                                                <div class="alert alert-info" style="margin:1rem;">
                                                    "⚠️ " {crate::t(is_zh, "Binary file modification (diff preview not supported for binary assets).", "二进制文件变更（不支持二进制资源的差异预览）。")}
                                                </div>
                                            }.into_any();
                                        }

                                        let additions = diff.diff_lines.iter().filter(|l| l.kind == "added").count();
                                        let deletions = diff.diff_lines.iter().filter(|l| l.kind == "removed").count();

                                        let line_rows: Vec<_> = diff.diff_lines.iter().map(|l| {
                                            let (row_cls, prefix) = match l.kind.as_str() {
                                                "added" => ("diff-row diff-row-add", "+"),
                                                "removed" => ("diff-row diff-row-del", "-"),
                                                _ => ("diff-row diff-row-ctx", " "),
                                            };
                                            let old_no_str = l.old_lineno.map(|n| n.to_string()).unwrap_or_default();
                                            let new_no_str = l.new_lineno.map(|n| n.to_string()).unwrap_or_default();

                                            view! {
                                                <tr class=row_cls>
                                                    <td class="diff-lineno">{old_no_str}</td>
                                                    <td class="diff-lineno">{new_no_str}</td>
                                                    <td class="diff-sign">{prefix}</td>
                                                    <td class="diff-code"><code>{l.content.clone()}</code></td>
                                                </tr>
                                            }
                                        }).collect();

                                        view! {
                                            <div class="vcs-diff-viewer">
                                                <div class="vcs-diff-bar">
                                                    <span class="vcs-diff-filepath">"📄 " {diff.file_path.clone()}</span>
                                                    <div class="vcs-diff-counts">
                                                        <span class="diff-count-add">"+" {additions}</span>
                                                        <span class="diff-count-del">"-" {deletions}</span>
                                                    </div>
                                                </div>

                                                <div class="vcs-diff-table-container">
                                                    <table class="vcs-diff-table">
                                                        <tbody>{line_rows}</tbody>
                                                    </table>
                                                </div>
                                            </div>
                                        }.into_any()
                                    }}
                                </div>
                            </div>
                        }.into_any()
                    }}
                </div>
            </div>

            // Interactive Restoration / Revert Modal
            <div
                node_ref=backdrop_ref
                class="modal-backdrop"
                style:display=move || if restore_modal_open.get() { "flex" } else { "none" }
            >
                <div class="modal-card" style="max-width:560px; width:92vw;">
                    <div class="modal-header">
                        <h3 class="modal-title">
                            {move || match restore_modal_mode.get().as_str() {
                                "replace" => crate::t(is_zh, "Restore & Overwrite Working File", "恢复并覆盖工作区文件"),
                                "new" => crate::t(is_zh, "Restore to Brand New File", "恢复为独立新文件"),
                                _ => crate::t(is_zh, "Rollback Entire Project", "回滚整个项目"),
                            }}
                        </h3>
                        <button
                            type="button"
                            class="modal-close"
                            on:click=move |_| restore_modal_open.set(false)
                        >"×"</button>
                    </div>

                    <div class="modal-body" style="padding:1.25rem;">
                        {move || match restore_modal_mode.get().as_str() {
                            "replace" => {
                                let f = selected_diff_file.get().unwrap_or_default();
                                view! {
                                    <div>
                                        <div class="alert alert-warning" style="margin-bottom:1rem; font-size:0.875rem;">
                                            <strong>"⚠️ " {crate::t(is_zh, "Warning: Overwrite Confirmation", "警告：覆盖确认")}</strong>
                                            <p style="margin:0.25rem 0 0;">
                                                {crate::t(
                                                    is_zh,
                                                    "This action will replace your active working copy of this file with the snapshot content.",
                                                    "此操作将使用所选快照中的内容直接覆盖您当前工作区中的该文件。"
                                                )}
                                            </p>
                                        </div>
                                        <p style="font-size:0.9rem; margin-bottom:0.5rem;">
                                            {crate::t(is_zh, "File to be restored:", "待恢复文件：")}
                                            <code style="font-weight:600; color:var(--primary);">{f}</code>
                                        </p>
                                        <p style="font-size:0.8rem; color:var(--text-muted); margin-bottom:1.25rem;">
                                            {crate::t(
                                                is_zh,
                                                "A safety snapshot will be automatically recorded before and after restoring.",
                                                "系统会在恢复后自动生成一条新的版本快照以记录此恢复操作，确保历史完全可追溯。"
                                            )}
                                        </p>
                                    </div>
                                }.into_any()
                            },
                            "new" => {
                                let f = selected_diff_file.get().unwrap_or_default();
                                view! {
                                    <div>
                                        <p style="font-size:0.875rem; color:var(--text-muted); margin-bottom:1rem;">
                                            {crate::t(
                                                is_zh,
                                                "Extract the historical version from the snapshot into a separate new file, leaving your current working copy untouched.",
                                                "将快照中的历史版本写入一个独立的新文件中，原工作区文件将保持原样不变。"
                                            )}
                                        </p>
                                        <div class="form-group" style="margin-bottom:1rem;">
                                            <label style="font-size:0.85rem; font-weight:600;">{crate::t(is_zh, "Source Snapshot File", "快照原文件")}</label>
                                            <input type="text" class="form-control" disabled=true prop:value=f />
                                        </div>
                                        <div class="form-group" style="margin-bottom:1.25rem;">
                                            <label style="font-size:0.85rem; font-weight:600;">{crate::t(is_zh, "New Target File Path", "恢复保存的新文件路径")}</label>
                                            <input
                                                type="text"
                                                class="form-control"
                                                placeholder="e.g. document_v1.typ"
                                                prop:value=move || restore_target_filename.get()
                                                on:input=move |ev| restore_target_filename.set(event_target_value(&ev))
                                            />
                                        </div>
                                    </div>
                                }.into_any()
                            },
                            _ => {
                                let snap_str = selected_snapshot_id.get().unwrap_or_default();
                                let short_id = if snap_str.len() >= 8 { snap_str[..8].to_string() } else { snap_str };
                                view! {
                                    <div>
                                        <div class="alert alert-danger" style="margin-bottom:1rem; font-size:0.875rem;">
                                            <strong>"🚨 " {crate::t(is_zh, "Project Rollback Warning", "项目全面回滚警告")}</strong>
                                            <p style="margin:0.25rem 0 0;">
                                                {crate::t(
                                                    is_zh,
                                                    "This will revert ALL files in this project workspace to the exact state at snapshot ",
                                                    "这将把该项目中所有的文件完整还原到快照 "
                                                )}
                                                <code>{short_id}</code>
                                                {crate::t(is_zh, ". Any uncommitted changes will be lost.", " 时的状态。任何未提交的修改将被覆盖。")}
                                            </p>
                                        </div>
                                    </div>
                                }.into_any()
                            },
                        }}

                        <div style="display:flex; justify-content:flex-end; gap:0.75rem; margin-top:1.5rem;">
                            <button
                                type="button"
                                class="btn btn-secondary"
                                on:click=move |_| restore_modal_open.set(false)
                            >
                                {crate::t(is_zh, "Cancel", "取消")}
                            </button>
                            {
                                let p_id = p_id_for_action;
                                view! {
                                    <button
                                        type="button"
                                        class=move || if restore_modal_mode.get() == "revert" { "btn btn-danger" } else { "btn btn-primary" }
                                        disabled=move || is_restoring.get()
                                        on:click={
                                            let project_id = p_id;
                                            move |_| {
                                                let mode = restore_modal_mode.get();
                                                let Some(snap_id) = selected_snapshot_id.get() else {
                                                    return;
                                                };

                                                is_restoring.set(true);

                                                #[cfg(feature = "hydrate")]
                                                {
                                                    let project_id = project_id.clone();
                                                    let is_zh = is_zh;
                                                    leptos::task::spawn_local(async move {
                                                        if mode == "revert" {
                                                            let url = format!("/api/projects/{project_id}/vcs/snapshots/{snap_id}/revert");
                                                            let res = gloo_net::http::Request::post(&url).send().await;
                                                            match res {
                                                                Ok(r) if r.ok() => {
                                                                    status_banner.set(Some((
                                                                        true,
                                                                        crate::t(is_zh, "Project workspace successfully reverted to snapshot! Reloading...", "项目工作区已成功回滚至该快照！正在刷新...").to_string(),
                                                                    )));
                                                                    restore_modal_open.set(false);
                                                                    if let Some(w) = web_sys::window() {
                                                                        let _ = w.location().reload();
                                                                    }
                                                                }
                                                                Ok(r) => {
                                                                    let err_text = r.text().await.unwrap_or_else(|_| "Failed to revert".to_string());
                                                                    status_banner.set(Some((false, err_text)));
                                                                }
                                                                Err(e) => {
                                                                    status_banner.set(Some((false, e.to_string())));
                                                                }
                                                            }
                                                        } else {
                                                            let source_file = selected_diff_file.get().unwrap_or_default();
                                                            let target_file = if mode == "replace" {
                                                                source_file.clone()
                                                            } else {
                                                                restore_target_filename.get().trim().to_string()
                                                            };

                                                            if target_file.is_empty() {
                                                                status_banner.set(Some((
                                                                    false,
                                                                    crate::t(is_zh, "Target file name cannot be empty.", "目标文件名不能为空。").to_string(),
                                                                )));
                                                                is_restoring.set(false);
                                                                return;
                                                            }

                                                            let payload = RestorePayload {
                                                                file: source_file.clone(),
                                                                target_file: target_file.clone(),
                                                            };

                                                            let url = format!("/api/projects/{project_id}/vcs/snapshots/{snap_id}/restore");
                                                            let req = gloo_net::http::Request::post(&url)
                                                                .header("Content-Type", "application/json")
                                                                .json(&payload);

                                                            match req {
                                                                Ok(r) => match r.send().await {
                                                                    Ok(resp) if resp.ok() => {
                                                                        let succ_msg = if mode == "replace" {
                                                                            format!(
                                                                                "{} '{target_file}'",
                                                                                crate::t(is_zh, "Successfully restored and replaced", "已成功恢复并覆盖")
                                                                            )
                                                                        } else {
                                                                            format!(
                                                                                "{} '{target_file}'",
                                                                                crate::t(is_zh, "Successfully restored snapshot version to new file", "已成功将快照版本恢复到新文件")
                                                                            )
                                                                        };
                                                                        status_banner.set(Some((true, succ_msg)));
                                                                        restore_modal_open.set(false);
                                                                    }
                                                                    Ok(resp) => {
                                                                        let err = resp.text().await.unwrap_or_else(|_| "Restore failed".to_string());
                                                                        status_banner.set(Some((false, err)));
                                                                    }
                                                                    Err(e) => {
                                                                        status_banner.set(Some((false, e.to_string())));
                                                                    }
                                                                },
                                                                Err(e) => {
                                                                    status_banner.set(Some((false, e.to_string())));
                                                                }
                                                            }
                                                        }
                                                        is_restoring.set(false);
                                                    });
                                                }
                                                #[cfg(not(feature = "hydrate"))]
                                                {
                                                    let _ = (&project_id, &mode, &snap_id);
                                                    is_restoring.set(false);
                                                }
                                            }
                                        }
                                    >
                                        {move || if is_restoring.get() {
                                            crate::t(is_zh, "Processing...", "处理中...").to_string()
                                        } else {
                                            match restore_modal_mode.get().as_str() {
                                                "replace" => crate::t(is_zh, "Confirm Overwrite & Restore", "确认覆盖并恢复").to_string(),
                                                "new" => crate::t(is_zh, "Create Restored File", "保存为新文件").to_string(),
                                                _ => crate::t(is_zh, "Revert Entire Project", "立即回滚项目").to_string(),
                                            }
                                        }}
                                    </button>
                                }
                            }
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}
