//! File sharing modal island for project documents and notes.
//!
//! Real Rust replacement for the file-share modal's hand-written JS (`components/mod.rs`'s
//! `FileShareModal`). Per-file "Share" buttons live on several different pages (files list,
//! note editor, document editor) outside this island, so they open it by dispatching a `window`
//! `CustomEvent` (`apich-open-share-modal`, with `path`/`mode`/`role`/`users`/`token` in `detail`) --
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
    #[prop(optional)] is_zh: Option<bool>,
) -> impl IntoView {
    let _ = all_users;
    let zh = is_zh.unwrap_or(false);
    let visible = RwSignal::new(false);
    let file_path = RwSignal::new(String::new());
    let token = RwSignal::new(String::new());
    let mode = RwSignal::new("private".to_string());
    let role = RwSignal::new("read".to_string());
    let selected_users = RwSignal::new(Vec::<String>::new());
    let active_tab = RwSignal::new("links".to_string());

    let search_query = RwSignal::new(String::new());
    let search_results = RwSignal::new(Vec::<crate::user_select::UserItem>::new());
    let is_search_loading = RwSignal::new(false);

    let email_to = RwSignal::new(String::new());
    let email_note = RwSignal::new(String::new());
    let is_email_sending = RwSignal::new(false);
    let email_feedback = RwSignal::new(Option::<Result<String, String>>::None);
    let copy_feedback = RwSignal::new(Option::<String>::None);

    let pid_for_email = project_id.clone();
    let pid_for_urls = project_id.clone();

    wire_open_listener(
        visible,
        file_path,
        token,
        mode,
        role,
        selected_users,
        search_results,
        is_search_loading,
        email_to,
        email_note,
        email_feedback,
        copy_feedback,
    );

    let slide_url = Memo::new({
        let pid = pid_for_urls.clone();
        move |_| {
            let fp = file_path.get();
            if fp.is_empty() {
                return String::new();
            }
            let tok = token.get();
            let tok_param = if tok.is_empty() {
                String::new()
            } else {
                format!("&token={}", urlencoding::encode(&tok))
            };
            format!(
                "{}/projects/{}/presentation/?file={}{}",
                get_window_origin(),
                pid,
                urlencoding::encode(&fp),
                tok_param
            )
        }
    });

    let pdf_url = Memo::new({
        let pid = pid_for_urls;
        move |_| {
            let fp = file_path.get();
            if fp.is_empty() {
                return String::new();
            }
            let tok = token.get();
            let tok_param = if tok.is_empty() {
                String::new()
            } else {
                format!("&token={}", urlencoding::encode(&tok))
            };
            format!(
                "{}/projects/{}/pdf-view?file={}{}",
                get_window_origin(),
                pid,
                urlencoding::encode(&fp),
                tok_param
            )
        }
    });

    view! {
        <div
            class="modal-backdrop"
            style:display=move || if visible.get() { "flex" } else { "none" }
        >
            <div
                class="modal-card"
                style="max-width:680px; width:min(680px, 95vw); border-radius:14px; padding:1.75rem 2rem; max-height:90vh; display:flex; flex-direction:column; box-shadow:0 25px 50px -12px rgba(0,0,0,0.25);"
            >
                // Modal Header
                <div class="modal-header" style="align-items:flex-start; margin-bottom:1rem; flex-shrink:0;">
                    <div>
                        <h3 class="modal-title" style="display:flex; align-items:center; gap:0.5rem; font-size:1.25rem; margin:0; font-weight:700;">
                            <span>"🔗"</span>
                            <span>{if zh { "文件共享、播放与评审" } else { "Share, Play & Review" }}</span>
                        </h3>
                        <div style="font-size:0.82rem; color:var(--text-sub); margin-top:0.35rem; display:flex; align-items:center; gap:0.4rem; flex-wrap:wrap;">
                            <span>{if zh { "当前文件：" } else { "File:" }}</span>
                            <code style="background:var(--bg-muted, #f1f5f9); color:var(--primary, #635bff); padding:2px 8px; border-radius:4px; font-weight:600; font-size:0.85rem;">
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

                // Navigation Tabs
                <div style="display:flex; gap:0.5rem; border-bottom:1px solid var(--border-subtle, #e2e8f0); margin-bottom:1.25rem; flex-shrink:0;">
                    <button
                        type="button"
                        style=move || {
                            let active = active_tab.get() == "links";
                            format!(
                                "padding:0.5rem 1rem; border:none; background:none; font-weight:{}; color:{}; border-bottom:2px solid {}; cursor:pointer; font-size:0.875rem; transition:all 0.15s ease;",
                                if active { "700" } else { "500" },
                                if active { "var(--primary, #635bff)" } else { "var(--text-sub, #64748b)" },
                                if active { "var(--primary, #635bff)" } else { "transparent" },
                            )
                        }
                        on:click=move |_| active_tab.set("links".to_string())
                    >
                        "🔗 " {if zh { "快捷分享链接" } else { "Direct Links" }}
                    </button>
                    <button
                        type="button"
                        style=move || {
                            let active = active_tab.get() == "email";
                            format!(
                                "padding:0.5rem 1rem; border:none; background:none; font-weight:{}; color:{}; border-bottom:2px solid {}; cursor:pointer; font-size:0.875rem; transition:all 0.15s ease;",
                                if active { "700" } else { "500" },
                                if active { "var(--primary, #635bff)" } else { "var(--text-sub, #64748b)" },
                                if active { "var(--primary, #635bff)" } else { "transparent" },
                            )
                        }
                        on:click=move |_| active_tab.set("email".to_string())
                    >
                        "✉️ " {if zh { "发送邮件" } else { "Send via Email" }}
                    </button>
                    <button
                        type="button"
                        style=move || {
                            let active = active_tab.get() == "access";
                            format!(
                                "padding:0.5rem 1rem; border:none; background:none; font-weight:{}; color:{}; border-bottom:2px solid {}; cursor:pointer; font-size:0.875rem; transition:all 0.15s ease;",
                                if active { "700" } else { "500" },
                                if active { "var(--primary, #635bff)" } else { "var(--text-sub, #64748b)" },
                                if active { "var(--primary, #635bff)" } else { "transparent" },
                            )
                        }
                        on:click=move |_| active_tab.set("access".to_string())
                    >
                        "🔒 " {if zh { "权限与协作者" } else { "Access & Permissions" }}
                    </button>
                </div>

                // Modal Content Area
                <div style="flex:1; overflow-y:auto; padding-right:2px; display:flex; flex-direction:column;">
                    // TAB 1: Direct Links
                    <div style:display=move || if active_tab.get() == "links" { "block" } else { "none" }>
                        // Copy Feedback Notification Banner
                        {move || copy_feedback.get().map(|msg| view! {
                            <div class="alert alert-success" style="padding:0.55rem 0.85rem; font-size:0.85rem; margin-bottom:1rem; border-radius:8px;">
                                "✓ " {msg}
                            </div>
                        })}

                        // Slide Presentation Player Link Card
                        <div style="background:var(--bg-muted, #f8fafc); border:1px solid var(--border-subtle, #e2e8f0); border-radius:10px; padding:1rem 1.15rem; margin-bottom:1rem;">
                            <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:0.4rem;">
                                <div style="font-weight:700; font-size:0.9rem; color:var(--text-main); display:flex; align-items:center; gap:0.4rem;">
                                    <span>"🖥️"</span>
                                    <span>{if zh { "浏览器幻灯片播放链接 (Slide Player)" } else { "Browser Slide Player Link" }}</span>
                                </div>
                                <span style="font-size:0.75rem; background:#ecfdf5; color:#059669; padding:2px 8px; border-radius:12px; font-weight:600; border:1px solid #a7f3d0;">
                                    "💬 " {if zh { "支持幻灯片在线评论与白板" } else { "Slide comments & whiteboard" }}
                                </span>
                            </div>
                            <p style="font-size:0.78rem; color:var(--text-sub); margin-bottom:0.6rem; line-height:1.4;">
                                {if zh {
                                    "在浏览器中放映全屏演示，支持翻页过渡动画、绘图标注、激光笔，以及实时附着在幻灯片页面的评审评论。"
                                } else {
                                    "Play fullscreen presentation with slide transitions, whiteboard drawings, laser pointer, and attached slide comments."
                                }}
                            </p>
                            <div style="display:flex; gap:0.5rem; align-items:center;">
                                <input
                                    type="text"
                                    readonly
                                    class="form-control"
                                    style="flex:1; height:36px; font-size:0.82rem; font-family:monospace; background:var(--bg-surface, #fff);"
                                    prop:value=move || slide_url.get()
                                    onclick="this.select()"
                                />
                                <button
                                    type="button"
                                    class="btn btn-secondary btn-sm"
                                    style="height:36px; white-space:nowrap; padding:0 0.85rem;"
                                    on:click=move |_| {
                                        let url = slide_url.get();
                                        let msg = if zh { "演示播放链接已复制到剪贴板！".to_string() } else { "Slide link copied to clipboard!".to_string() };
                                        copy_to_clipboard(url, copy_feedback, msg);
                                    }
                                >
                                    "📋 " {if zh { "复制" } else { "Copy" }}
                                </button>
                                <a
                                    href=move || slide_url.get()
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    class="btn btn-primary btn-sm"
                                    style="height:36px; white-space:nowrap; padding:0 0.85rem; display:inline-flex; align-items:center;"
                                >
                                    "▶ " {if zh { "播放" } else { "Play" }}
                                </a>
                            </div>
                        </div>

                        // PDF Viewer Link Card
                        <div style="background:var(--bg-muted, #f8fafc); border:1px solid var(--border-subtle, #e2e8f0); border-radius:10px; padding:1rem 1.15rem; margin-bottom:1rem;">
                            <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:0.4rem;">
                                <div style="font-weight:700; font-size:0.9rem; color:var(--text-main); display:flex; align-items:center; gap:0.4rem;">
                                    <span>"📄"</span>
                                    <span>{if zh { "浏览器 PDF 在线预览链接 (PDF Viewer)" } else { "Browser PDF Viewer Link" }}</span>
                                </div>
                                <span style="font-size:0.75rem; background:#eff6ff; color:#2563eb; padding:2px 8px; border-radius:12px; font-weight:600; border:1px solid #bfdbfe;">
                                    "💬 " {if zh { "支持按页码添加评审评论" } else { "Page-by-page review comments" }}
                                </span>
                            </div>
                            <p style="font-size:0.78rem; color:var(--text-sub); margin-bottom:0.6rem; line-height:1.4;">
                                {if zh {
                                    "在线查看渲染后的 PDF 格式文档，内置原生缩放、翻页、下载，并支持用户针对具体页码发表评审评论。"
                                } else {
                                    "View rendered PDF with native controls, page navigation, download, and page-specific review comments."
                                }}
                            </p>
                            <div style="display:flex; gap:0.5rem; align-items:center;">
                                <input
                                    type="text"
                                    readonly
                                    class="form-control"
                                    style="flex:1; height:36px; font-size:0.82rem; font-family:monospace; background:var(--bg-surface, #fff);"
                                    prop:value=move || pdf_url.get()
                                    onclick="this.select()"
                                />
                                <button
                                    type="button"
                                    class="btn btn-secondary btn-sm"
                                    style="height:36px; white-space:nowrap; padding:0 0.85rem;"
                                    on:click=move |_| {
                                        let url = pdf_url.get();
                                        let msg = if zh { "PDF 预览链接已复制到剪贴板！".to_string() } else { "PDF link copied to clipboard!".to_string() };
                                        copy_to_clipboard(url, copy_feedback, msg);
                                    }
                                >
                                    "📋 " {if zh { "复制" } else { "Copy" }}
                                </button>
                                <a
                                    href=move || pdf_url.get()
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    class="btn btn-primary btn-sm"
                                    style="height:36px; white-space:nowrap; padding:0 0.85rem; display:inline-flex; align-items:center;"
                                >
                                    "↗ " {if zh { "查看" } else { "View" }}
                                </a>
                            </div>
                        </div>

                        // Token & Access Helper Note
                        <div style="display:flex; align-items:flex-start; gap:0.5rem; background:rgba(99,102,241,0.06); border:1px solid rgba(99,102,241,0.2); border-radius:8px; padding:0.75rem 1rem; font-size:0.8rem; color:var(--text-main);">
                            <span style="font-size:1.1rem; line-height:1;">"💡"</span>
                            <div style="line-height:1.45;">
                                {move || {
                                    let tok = token.get();
                                    if !tok.is_empty() {
                                        if zh {
                                            "以上链接已自动附带安全访问令牌（Token）。获得此链接的任何人无需登录即可直接在浏览器中查看演示/PDF并提交评审评论。".to_string()
                                        } else {
                                            "These links automatically include a secure access token. Anyone with the link can view presentations/PDFs and leave comments without logging in.".to_string()
                                        }
                                    } else {
                                        if zh {
                                            "如需允许外部人员无密码免登录访问，可在【权限与协作者】选项卡中将模式设为【公开链接】。".to_string()
                                        } else {
                                            "To allow external users to view without login, set the sharing mode to 'Public Link' in the Access & Permissions tab.".to_string()
                                        }
                                    }
                                }}
                            </div>
                        </div>
                    </div>

                    // TAB 2: Send via Email
                    <div style:display=move || if active_tab.get() == "email" { "block" } else { "none" }>
                        <div style="margin-bottom:1rem;">
                            <p style="font-size:0.85rem; color:var(--text-sub); line-height:1.4; margin:0 0 1rem 0;">
                                {if zh {
                                    "系统将通过邮件向接收者发送该文件的安全在线查阅链接（包含演示播放和 PDF 评审），对方可直接点击并在浏览器中查看与评论。"
                                } else {
                                    "Send secure view links (Slide Player & PDF Review) directly to anyone's email address for quick in-browser review."
                                }}
                            </p>

                            // Email Feedback Banner
                            {move || email_feedback.get().map(|res| match res {
                                Ok(msg) => view! {
                                    <div class="alert alert-success" style="padding:0.6rem 0.9rem; font-size:0.85rem; margin-bottom:1rem; border-radius:8px;">
                                        "✓ " {msg}
                                    </div>
                                }.into_any(),
                                Err(err) => view! {
                                    <div class="alert alert-danger" style="padding:0.6rem 0.9rem; font-size:0.85rem; margin-bottom:1rem; border-radius:8px;">
                                        "⚠ " {err}
                                    </div>
                                }.into_any(),
                            })}

                            <div class="form-group" style="margin-bottom:1rem;">
                                <label style="font-weight:700; font-size:0.85rem; margin-bottom:0.4rem; display:block; color:var(--text-main);">
                                    {if zh { "收件人邮箱地址 *" } else { "Recipient Email Address *" }}
                                </label>
                                <input
                                    type="email"
                                    class="form-control"
                                    placeholder=if zh { "例如：colleague@company.com" } else { "e.g. colleague@company.com" }
                                    style="width:100%; height:38px; font-size:0.875rem;"
                                    prop:value=move || email_to.get()
                                    on:input=move |ev| email_to.set(event_target_value(&ev))
                                />
                            </div>

                            <div class="form-group" style="margin-bottom:1.25rem;">
                                <label style="font-weight:700; font-size:0.85rem; margin-bottom:0.4rem; display:block; color:var(--text-main);">
                                    {if zh { "附言与评审说明 (可选)" } else { "Message / Review Note (Optional)" }}
                                </label>
                                <textarea
                                    class="form-control"
                                    rows="3"
                                    placeholder=if zh { "例如：请重点审阅第 3 页和第 5 页的数据图表与总结结论，谢谢！" } else { "e.g. Please review the chart and conclusion on slide 3 and 5, thanks!" }
                                    style="width:100%; font-size:0.85rem; resize:vertical;"
                                    prop:value=move || email_note.get()
                                    on:input=move |ev| email_note.set(event_target_value(&ev))
                                />
                            </div>

                            <div style="display:flex; justify-content:flex-end; gap:0.6rem;">
                                <button
                                    type="button"
                                    class="btn btn-primary"
                                    disabled=move || is_email_sending.get()
                                    on:click={
                                        let pid = pid_for_email;
                                        move |_| {
                                            send_share_email(
                                                pid.clone(),
                                                file_path.get(),
                                                email_to.get(),
                                                email_note.get(),
                                                is_email_sending,
                                                email_feedback,
                                                zh,
                                            );
                                        }
                                    }
                                    style="display:inline-flex; align-items:center; gap:0.4rem;"
                                >
                                    {move || if is_email_sending.get() {
                                        view! { <span>"⏳ " {if zh { "正在发送..." } else { "Sending..." }}</span> }.into_any()
                                    } else {
                                        view! { <span>"✉️ " {if zh { "发送分享邮件" } else { "Send Share Email" }}</span> }.into_any()
                                    }}
                                </button>
                            </div>
                        </div>
                    </div>

                    // TAB 3: Access & Permissions (Standard Save Form)
                    <div style:display=move || if active_tab.get() == "access" { "block" } else { "none" }>
                        <form
                            method="post"
                            action=format!("/projects/{}/files/share", project_id)
                            style="display:flex; flex-direction:column;"
                        >
                            <input type="hidden" name="file" prop:value=move || file_path.get() />
                            <input type="hidden" name="redirect_to" value=redirect_to />
                            <input type="hidden" name="allowed_users" prop:value=move || selected_users.get().join(",") />
                            <input type="hidden" name="mode" prop:value=move || mode.get() />

                            // 1. Sharing Mode Cards
                            <div class="form-group" style="margin-bottom:1.25rem;">
                                <label style="font-weight:700; font-size:0.875rem; margin-bottom:0.5rem; display:block; color:var(--text-main);">
                                    {if zh { "共享模式" } else { "Sharing Mode" }}
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
                                            <span>{if zh { "私有" } else { "Private" }}</span>
                                        </div>
                                        <div style="font-size:0.75rem; color:var(--text-sub); line-height:1.3;">
                                            {if zh { "仅项目成员可见" } else { "Project members only" }}
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
                                            <span>{if zh { "指定成员" } else { "Specific Accounts" }}</span>
                                        </div>
                                        <div style="font-size:0.75rem; color:var(--text-sub); line-height:1.3;">
                                            {if zh { "仅指定用户可访问" } else { "Designated users only" }}
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
                                            <span>{if zh { "公开链接" } else { "Public Link" }}</span>
                                        </div>
                                        <div style="font-size:0.75rem; color:var(--text-sub); line-height:1.3;">
                                            {if zh { "拥有链接的任何人" } else { "Anyone with link" }}
                                        </div>
                                    </div>
                                </div>
                            </div>

                            // 2. Permission Level
                            <div class="form-group" style="margin-bottom:1.25rem;">
                                <label style="font-weight:700; font-size:0.875rem; margin-bottom:0.4rem; display:block; color:var(--text-main);">
                                    {if zh { "权限级别" } else { "Permission Level" }}
                                </label>
                                <select
                                    name="role"
                                    class="form-control"
                                    style="height:38px; font-size:0.875rem; width:100%;"
                                    prop:value=move || role.get()
                                    on:change=move |ev| role.set(event_target_value(&ev))
                                >
                                    <option value="read">{if zh { "📖 只读 — 仅供查看，不可修改" } else { "📖 Read Only — Viewer cannot edit" }}</option>
                                    <option value="review">{if zh { "📝 查看与评审 — 可查看并添加评论" } else { "📝 Read & Review — Can view and add comments" }}</option>
                                    <option value="write">{if zh { "✏️ 协同编辑 — 具备完整编辑保存权限" } else { "✏️ Read, Write & Review — Full collaborative edit access" }}</option>
                                </select>
                            </div>

                            // 3. Specific Users Panel (Inline list, scrollable)
                            <div
                                style:display=move || if mode.get() == "specific" { "block" } else { "none" }
                                style="background:var(--bg-muted, #f8fafc); border:1px solid var(--border-subtle, #e2e8f0); border-radius:10px; padding:1rem; margin-bottom:1rem;"
                            >
                                <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:0.5rem;">
                                    <span style="font-weight:700; font-size:0.85rem; color:var(--text-main);">
                                        {if zh { "指定协作者" } else { "Designated Collaborators" }}
                                    </span>
                                    <span style="font-size:0.775rem; color:var(--text-sub); background:var(--bg-surface, #fff); padding:2px 8px; border-radius:10px; border:1px solid var(--border-subtle, #cbd5e1);">
                                        {move || selected_users.get().len()} {if zh { " 人已选" } else { " selected" }}
                                    </span>
                                </div>

                                <div style="display:flex; flex-wrap:wrap; gap:0.4rem; margin-bottom:0.85rem; min-height:28px; align-items:center;">
                                    {move || {
                                        let users = selected_users.get();
                                        if users.is_empty() {
                                            view! {
                                                <span style="font-size:0.8rem; color:var(--text-sub); font-style:italic;">
                                                    {if zh { "尚未添加协作者，请在下方搜索添加。" } else { "No users added yet. Search below to add users." }}
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
                                                            title=if zh { "移除用户" } else { "Remove user" }
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

                                <div style="margin-top:0.4rem;">
                                    <label style="font-size:0.775rem; font-weight:600; color:var(--text-sub); margin-bottom:0.35rem; display:block;">
                                        {if zh { "搜索并添加用户：" } else { "Search & Add Users:" }}
                                    </label>
                                    <div style="display:flex; gap:0.5rem; align-items:center;">
                                        <input
                                            type="text"
                                            class="form-control"
                                            placeholder=if zh { "输入用户名或姓名搜索..." } else { "Type username or name to search..." }
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
                                            if search_query.get().is_empty() {
                                                view! { <span></span> }.into_any()
                                            } else {
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
                                                        {if zh { "清空" } else { "Clear" }}
                                                    </button>
                                                }.into_any()
                                            }
                                        }}
                                    </div>
                                </div>

                                <div style="margin-top:0.6rem; border:1px solid var(--border-subtle, #cbd5e1); border-radius:8px; background:var(--bg-surface, #ffffff); max-height:190px; overflow-y:auto; padding:2px 0;">
                                    {move || {
                                        let items = search_results.get();
                                        let loading = is_search_loading.get();
                                        let current_selected = selected_users.get();

                                        if loading && items.is_empty() {
                                            view! {
                                                <div style="padding:1rem; font-size:0.85rem; color:var(--text-sub); text-align:center;">
                                                    {if zh { "正在搜索用户..." } else { "Searching users..." }}
                                                </div>
                                            }.into_any()
                                        } else if items.is_empty() {
                                            view! {
                                                <div style="padding:1rem; font-size:0.85rem; color:var(--text-sub); text-align:center;">
                                                    {if zh { "未找到用户，请在上方输入姓名或用户名搜索。" } else { "No users found. Type a name or username above to search." }}
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
                                                                    {if zh { "✓ 已添加" } else { "✓ Added" }}
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
                                                                    {if zh { "+ 添加" } else { "+ Add" }}
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

                            <div style="display:flex; justify-content:flex-end; gap:0.6rem; margin-top:1.25rem; padding-top:1rem; border-top:1px solid var(--border-subtle, #e2e8f0);">
                                <button
                                    type="button"
                                    class="btn btn-secondary"
                                    on:click=move |_| visible.set(false)
                                >
                                    {if zh { "取消" } else { "Cancel" }}
                                </button>
                                <button type="submit" class="btn btn-primary">
                                    {if zh { "保存分享设置" } else { "Save Sharing" }}
                                </button>
                            </div>
                        </form>
                    </div>
                </div>

                // Modal Footer (Close button visible on all tabs)
                <div style="display:flex; justify-content:flex-end; margin-top:1rem; padding-top:0.75rem; border-top:1px solid var(--border-subtle, #e2e8f0); flex-shrink:0;">
                    <button
                        type="button"
                        class="btn btn-secondary"
                        on:click=move |_| visible.set(false)
                    >
                        {if zh { "关闭" } else { "Close" }}
                    </button>
                </div>
            </div>
        </div>
    }
}

#[cfg(feature = "hydrate")]
fn get_window_origin() -> String {
    web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_default()
}

#[cfg(not(feature = "hydrate"))]
fn get_window_origin() -> String {
    String::new()
}

#[cfg(feature = "hydrate")]
fn copy_to_clipboard(text: String, feedback: RwSignal<Option<String>>, msg: String) {
    if let Some(win) = web_sys::window() {
        let nav = win.navigator();
        let clip = nav.clipboard();
        let promise = clip.write_text(&text);
        wasm_bindgen_futures::spawn_local(async move {
            let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
            feedback.set(Some(msg));
            gloo_timers::future::TimeoutFuture::new(2500).await;
            feedback.set(None);
        });
    }
}

#[cfg(not(feature = "hydrate"))]
fn copy_to_clipboard(_text: String, _feedback: RwSignal<Option<String>>, _msg: String) {}

#[cfg(feature = "hydrate")]
fn send_share_email(
    project_id: String,
    file_path: String,
    to_email: String,
    note: String,
    is_sending: RwSignal<bool>,
    feedback: RwSignal<Option<Result<String, String>>>,
    zh: bool,
) {
    if to_email.trim().is_empty() {
        feedback.set(Some(Err(if zh {
            "请输入接收者的邮箱地址".to_string()
        } else {
            "Please enter recipient email address".to_string()
        })));
        return;
    }
    is_sending.set(true);
    feedback.set(None);
    wasm_bindgen_futures::spawn_local(async move {
        let url = format!("/projects/{}/files/share/email", project_id);
        let note_trimmed = note.trim();
        let req_body = serde_json::json!({
            "file_path": file_path,
            "to_email": to_email.trim(),
            "note": if note_trimmed.is_empty() { None } else { Some(note_trimmed.to_string()) },
        });
        let body_str = req_body.to_string();
        let resp = gloo_net::http::Request::post(&url)
            .header("Content-Type", "application/json")
            .body(body_str);
        match resp {
            Ok(req) => match req.send().await {
                Ok(res) => {
                    if res.ok() {
                        let text = if zh {
                            "分享链接邮件已成功发送！".to_string()
                        } else {
                            "Share link email sent successfully!".to_string()
                        };
                        feedback.set(Some(Ok(text)));
                    } else {
                        let err_text = match res.json::<serde_json::Value>().await {
                            Ok(v) => v.get("error").and_then(|e| e.as_str()).unwrap_or("Failed to send").to_string(),
                            Err(_) => res.text().await.unwrap_or_else(|_| "Failed to send".into()),
                        };
                        feedback.set(Some(Err(err_text)));
                    }
                }
                Err(e) => {
                    feedback.set(Some(Err(format!("Network error: {}", e))));
                }
            },
            Err(e) => {
                feedback.set(Some(Err(format!("Request error: {}", e))));
            }
        }
        is_sending.set(false);
    });
}

#[cfg(not(feature = "hydrate"))]
fn send_share_email(
    _project_id: String,
    _file_path: String,
    _to_email: String,
    _note: String,
    _is_sending: RwSignal<bool>,
    _feedback: RwSignal<Option<Result<String, String>>>,
    _zh: bool,
) {}

#[cfg(feature = "hydrate")]
fn wire_open_listener(
    visible: RwSignal<bool>,
    file_path: RwSignal<String>,
    token: RwSignal<String>,
    mode: RwSignal<String>,
    role: RwSignal<String>,
    selected_users: RwSignal<Vec<String>>,
    search_results: RwSignal<Vec<crate::user_select::UserItem>>,
    is_search_loading: RwSignal<bool>,
    email_to: RwSignal<String>,
    email_note: RwSignal<String>,
    email_feedback: RwSignal<Option<Result<String, String>>>,
    copy_feedback: RwSignal<Option<String>>,
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
        token.set(get_str("token"));
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
        email_to.set(String::new());
        email_note.set(String::new());
        email_feedback.set(None);
        copy_feedback.set(None);
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
    _token: RwSignal<String>,
    _mode: RwSignal<String>,
    _role: RwSignal<String>,
    _selected_users: RwSignal<Vec<String>>,
    _search_results: RwSignal<Vec<crate::user_select::UserItem>>,
    _is_search_loading: RwSignal<bool>,
    _email_to: RwSignal<String>,
    _email_note: RwSignal<String>,
    _email_feedback: RwSignal<Option<Result<String, String>>>,
    _copy_feedback: RwSignal<Option<String>>,
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
