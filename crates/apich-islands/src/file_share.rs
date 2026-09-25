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

/// Managed temporary share link representation for the UI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ManagedSharedLink {
    pub token: String,
    pub project_id: String,
    pub file_path: String,
    pub target_type: String,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub is_revoked: bool,
    #[serde(default)]
    pub allow_comments: bool,
    #[serde(default)]
    pub view_count: i64,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub url: String,
}

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

    let shared_links = RwSignal::new(Vec::<ManagedSharedLink>::new());
    let is_links_loading = RwSignal::new(false);
    let is_creating_link = RwSignal::new(false);
    let new_link_target = RwSignal::new("slide".to_string());
    let new_link_expiry = RwSignal::new("7".to_string());
    let new_link_comments = RwSignal::new(true);

    // All-project shared links management
    let all_links = RwSignal::new(Vec::<ManagedSharedLink>::new());
    let is_all_links_loading = RwSignal::new(false);
    let all_links_loaded = RwSignal::new(false);

    let pid_for_email = project_id.clone();
    let pid_for_urls = project_id.clone();
    let pid_for_listener = project_id.clone();
    let pid_for_create = project_id.clone();
    let pid_for_list = project_id.clone();
    let pid_for_form = project_id.clone();
    let pid_for_all_links = project_id.clone();
    let pid_for_all_rev = project_id.clone();

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
        pid_for_listener,
        shared_links,
        is_links_loading,
        new_link_target,
    );

    let primary_opaque_link = Memo::new({
        move |_| {
            let links = shared_links.get();
            let cur_target = new_link_target.get();
            links.into_iter().find(|l| !l.is_revoked && l.target_type == cur_target)
                .or_else(|| {
                    let fp = file_path.get();
                    let is_slide = fp.ends_with(".typ") || fp.ends_with(".slide");
                    if is_slide {
                        shared_links.get().into_iter().find(|l| !l.is_revoked && (l.target_type == "present" || l.target_type == "slide"))
                    } else {
                        shared_links.get().into_iter().find(|l| !l.is_revoked && l.target_type == "pdf")
                    }
                })
                .or_else(|| shared_links.get().into_iter().find(|l| !l.is_revoked))
        }
    });

    let slide_internal_url = Memo::new({
        let pid = pid_for_urls.clone();
        move |_| {
            let fp = file_path.get();
            if fp.is_empty() {
                return String::new();
            }
            format!(
                "{}/projects/{}/presentation/?file={}",
                get_window_origin(),
                pid,
                urlencoding::encode(&fp)
            )
        }
    });

    let pdf_internal_url = Memo::new({
        let pid = pid_for_urls;
        move |_| {
            let fp = file_path.get();
            if fp.is_empty() {
                return String::new();
            }
            format!(
                "{}/projects/{}/pdf-view?file={}",
                get_window_origin(),
                pid,
                urlencoding::encode(&fp)
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
                    <button
                        type="button"
                        style=move || {
                            let active = active_tab.get() == "all_links";
                            format!(
                                "padding:0.5rem 1rem; border:none; background:none; font-weight:{}; color:{}; border-bottom:2px solid {}; cursor:pointer; font-size:0.875rem; transition:all 0.15s ease;",
                                if active { "700" } else { "500" },
                                if active { "var(--primary, #635bff)" } else { "var(--text-sub, #64748b)" },
                                if active { "var(--primary, #635bff)" } else { "transparent" },
                            )
                        }
                        on:click={
                            let pid = pid_for_all_links.clone();
                            move |_| {
                                active_tab.set("all_links".to_string());
                                if !all_links_loaded.get() {
                                    all_links_loaded.set(true);
                                    fetch_shared_links(pid.clone(), String::new(), all_links, is_all_links_loading);
                                }
                            }
                        }
                    >
                        "📋 " {if zh { "全部分享链接" } else { "All Shared Links" }}
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

                        // SECTION 1: Secure Temporary Links (Opaque /s/:token)
                        <div style="background:var(--bg-muted, #f8fafc); border:1px solid var(--border-subtle, #e2e8f0); border-radius:10px; padding:1.1rem; margin-bottom:1.25rem;">
                            <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:0.4rem; flex-wrap:wrap; gap:0.4rem;">
                                <div style="font-weight:700; font-size:0.92rem; color:var(--text-main); display:flex; align-items:center; gap:0.4rem;">
                                    <span>"🛡️"</span>
                                    <span>{if zh { "安全临时分享链接 (推荐)" } else { "Secure Temporary Links (Recommended)" }}</span>
                                </div>
                                <span style="font-size:0.75rem; background:#eff6ff; color:#2563eb; padding:2px 8px; border-radius:12px; font-weight:600; border:1px solid #bfdbfe;">
                                    {if zh { "不暴露文件名 · 防篡改 · 支持撤销" } else { "Opaque URL · Tamper-proof · Revocable" }}
                                </span>
                            </div>
                            <p style="font-size:0.78rem; color:var(--text-sub); margin-bottom:0.85rem; line-height:1.4;">
                                {if zh {
                                    "生成形如 /s/:token 的安全混淆链接，外部访客无法获知项目路径或文件名，可单独设定有效天数和评审评论权限，且支持随时一键失效撤销。"
                                } else {
                                    "Generates an opaque /s/:token URL hiding project ID and file paths. Set an expiration date and comments permission, or revoke anytime."
                                }}
                            </p>

                            // Primary Opaque Link Hero Card
                            {move || {
                                primary_opaque_link.get().map(|lnk| {
                                    let full_url = lnk.url.clone();
                                    let full_url_copy = full_url.clone();
                                    let full_url_play = full_url.clone();
                                    let target_type = lnk.target_type.as_str();
                                    let is_present = target_type == "present";
                                    let is_slide = target_type == "slide" || is_present;

                                    view! {
                                        <div style="background:linear-gradient(135deg, rgba(99,102,241,0.06) 0%, rgba(168,85,247,0.06) 100%); border:2px solid var(--primary, #635bff); border-radius:10px; padding:1.15rem; margin-bottom:1.15rem; box-shadow: 0 4px 12px rgba(99,102,241,0.08);">
                                            <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:0.4rem; flex-wrap:wrap; gap:0.4rem;">
                                                <div style="font-weight:700; font-size:0.95rem; color:var(--text-main); display:flex; align-items:center; gap:0.4rem;">
                                                    <span>{if is_present { "⚡" } else if is_slide { "🖥️" } else { "📄" }}</span>
                                                    <span>{if is_present {
                                                        if zh { "当前文件专属极速全屏演示链接" } else { "Fast Presentation Share Link (Instant SVG)" }
                                                    } else if is_slide {
                                                        if zh { "当前文件专属交互式播放链接" } else { "Interactive Slide Player Share Link" }
                                                    } else {
                                                        if zh { "当前文件专属安全预览链接" } else { "Secure Document Share Link" }
                                                    }}</span>
                                                </div>
                                                <span style=format!(
                                                    "font-size:0.75rem; padding:2px 8px; border-radius:12px; font-weight:700; border:1px solid {}; background:{}; color:{};",
                                                    if is_present { "#fde68a" } else { "#a7f3d0" },
                                                    if is_present { "#fef3c7" } else { "#ecfdf5" },
                                                    if is_present { "#b45309" } else { "#059669" },
                                                )>
                                                    "✓ " {if is_present { if zh { "极速加载 · 免WASM" } else { "Fast · Zero WASM" } } else { if zh { "防篡改 · 隐藏文件名" } else { "Tamper-Proof · Opaque" } }}
                                                </span>
                                            </div>
                                            <p style="font-size:0.78rem; color:var(--text-sub); margin-bottom:0.75rem; line-height:1.4;">
                                                {if is_present {
                                                    if zh {
                                                        "访客通过此链接可立即以极速全屏模式播放演示文稿，无需加载庞大 WASM 引擎，毫秒级响应并支持键盘滚轮切换。"
                                                    } else {
                                                        "Instant full-screen presentation playback without loading heavy WASM engines, with millisecond response and keyboard/wheel navigation."
                                                    }
                                                } else if is_slide {
                                                    if zh {
                                                        "外部访客通过此链接可在浏览器中以完整过渡动画、白板画笔和激光笔进行交互式放映与评审。"
                                                    } else {
                                                        "Interactive player with transitions, whiteboard ink, laser pointer, and audio support."
                                                    }
                                                } else {
                                                    if zh {
                                                        "外部访客通过此链接可直接在线浏览 PDF 文档并参与行级批注与评论评审。"
                                                    } else {
                                                        "Viewers can read the PDF document online and participate in line-level review comments."
                                                    }
                                                }}
                                            </p>
                                            <div style="display:flex; gap:0.5rem; align-items:center;">
                                                <input
                                                    type="text"
                                                    readonly
                                                    class="form-control"
                                                    style="flex:1; height:38px; font-size:0.85rem; font-family:monospace; background:var(--bg-surface, #fff); font-weight:600;"
                                                    prop:value=full_url
                                                    onclick="this.select()"
                                                />
                                                <button
                                                    type="button"
                                                    class="btn btn-primary"
                                                    style="height:38px; white-space:nowrap; padding:0 1rem; font-weight:600; display:inline-flex; align-items:center; gap:0.35rem;"
                                                    on:click=move |_| {
                                                        let msg = if zh { "安全分享链接已复制到剪贴板！".to_string() } else { "Secure link copied to clipboard!".to_string() };
                                                        copy_to_clipboard(full_url_copy.clone(), copy_feedback, msg);
                                                    }
                                                >
                                                    "📋 " {if zh { "复制安全链接" } else { "Copy Secure Link" }}
                                                </button>
                                                <a
                                                    href=full_url_play
                                                    target="_blank"
                                                    rel="noopener noreferrer"
                                                    class="btn btn-secondary"
                                                    style="height:38px; white-space:nowrap; padding:0 0.85rem; display:inline-flex; align-items:center; gap:0.35rem; font-weight:600;"
                                                >
                                                    {if is_present { "⚡ " } else if is_slide { "▶ " } else { "↗ " }}
                                                    {if is_present {
                                                        if zh { "极速演示" } else { "Present" }
                                                    } else if is_slide {
                                                        if zh { "立即播放" } else { "Play" }
                                                    } else {
                                                        if zh { "在线预览" } else { "View" }
                                                    }}
                                                </a>
                                            </div>
                                        </div>
                                    }
                                })
                            }}

                            // Create controls bar
                            <div style="display:flex; gap:0.5rem; align-items:center; flex-wrap:wrap; margin-bottom:1rem; background:var(--bg-surface, #fff); padding:0.6rem 0.85rem; border-radius:8px; border:1px solid var(--border-subtle, #cbd5e1);">
                                <div style="display:flex; align-items:center; gap:0.35rem;">
                                    <label style="font-size:0.8rem; font-weight:600; color:var(--text-sub);">{if zh { "类型:" } else { "Type:" }}</label>
                                    <select
                                        class="form-control"
                                        style="height:32px; font-size:0.8rem; padding:0 6px;"
                                        prop:value=move || new_link_target.get()
                                        on:change=move |ev| new_link_target.set(event_target_value(&ev))
                                    >
                                        <option value="present">{if zh { "⚡ 极速全屏演示 (Present)" } else { "⚡ Fast Presentation (Present)" }}</option>
                                        <option value="slide">{if zh { "🖥️ 交互式放映 (Slide Player)" } else { "🖥️ Interactive Player (WASM)" }}</option>
                                        <option value="pdf">{if zh { "📄 PDF 预览 (Viewer)" } else { "📄 PDF Viewer" }}</option>
                                    </select>
                                </div>

                                <div style="display:flex; align-items:center; gap:0.35rem;">
                                    <label style="font-size:0.8rem; font-weight:600; color:var(--text-sub);">{if zh { "有效期限:" } else { "Expiry:" }}</label>
                                    <select
                                        class="form-control"
                                        style="height:32px; font-size:0.8rem; padding:0 6px;"
                                        prop:value=move || new_link_expiry.get()
                                        on:change=move |ev| new_link_expiry.set(event_target_value(&ev))
                                    >
                                        <option value="7">{if zh { "7 天有效" } else { "7 Days" }}</option>
                                        <option value="30">{if zh { "30 天有效" } else { "30 Days" }}</option>
                                        <option value="90">{if zh { "90 天有效" } else { "90 Days" }}</option>
                                        <option value="0">{if zh { "永久有效" } else { "Permanent" }}</option>
                                    </select>
                                </div>

                                <label style="display:flex; align-items:center; gap:0.3rem; font-size:0.8rem; cursor:pointer; margin:0 0.25rem;">
                                    <input
                                        type="checkbox"
                                        prop:checked=move || new_link_comments.get()
                                        on:change=move |ev| new_link_comments.set(event_target_checked(&ev))
                                    />
                                    <span>{if zh { "允许评论" } else { "Comments" }}</span>
                                </label>

                                <button
                                    type="button"
                                    class="btn btn-primary btn-sm"
                                    style="height:32px; font-size:0.8rem; padding:0 0.85rem; margin-left:auto; display:inline-flex; align-items:center; gap:0.3rem;"
                                    disabled=move || is_creating_link.get()
                                    on:click={
                                        let pid = pid_for_create.clone();
                                        move |_| {
                                            let exp = match new_link_expiry.get().as_str() {
                                                "0" => None,
                                                other => other.parse::<i64>().ok(),
                                            };
                                            create_managed_link(
                                                pid.clone(),
                                                file_path.get(),
                                                new_link_target.get(),
                                                exp,
                                                new_link_comments.get(),
                                                is_creating_link,
                                                shared_links,
                                                copy_feedback,
                                                zh,
                                            );
                                        }
                                    }
                                >
                                    {move || if is_creating_link.get() {
                                        view! { <span>"⏳ " {if zh { "生成中..." } else { "Generating..." }}</span> }.into_any()
                                    } else {
                                        view! { <span>"+ " {if zh { "生成安全临时链接" } else { "Generate Safe Link" }}</span> }.into_any()
                                    }}
                                </button>
                            </div>

                            // Active Links List
                            <div style="display:flex; flex-direction:column; gap:0.5rem;">
                                {
                                    let pid_rev = pid_for_list.clone();
                                    move || {
                                        let links = shared_links.get();
                                        let loading = is_links_loading.get();
                                        if loading && links.is_empty() {
                                            view! {
                                                <div style="padding:0.75rem; text-align:center; font-size:0.8rem; color:var(--text-sub);">
                                                    "⏳ " {if zh { "正在加载已生成的分享链接..." } else { "Loading active links..." }}
                                                </div>
                                            }.into_any()
                                        } else if links.is_empty() {
                                            view! {
                                                <div style="padding:0.75rem; text-align:center; font-size:0.8rem; color:var(--text-sub); background:rgba(0,0,0,0.02); border-radius:6px; border:1px dashed var(--border-subtle, #cbd5e1);">
                                                    {if zh { "当前文件暂无生成的安全临时分享链接，点击上方按钮可快速生成。" } else { "No active safe temporary links for this file. Click above to generate one." }}
                                                </div>
                                            }.into_any()
                                        } else {
                                            let items: Vec<_> = links.into_iter().map(|lnk| {
                                                let full_url = lnk.url.clone();
                                                let full_url_copy = full_url.clone();
                                                let full_url_open = full_url.clone();
                                                let tok = lnk.token.clone();
                                                let pid_this = pid_rev.clone();
                                                let is_present = lnk.target_type == "present";
                                                let is_slide = lnk.target_type == "slide";

                                                view! {
                                                    <div style="display:flex; align-items:center; justify-content:space-between; gap:0.5rem; background:var(--bg-surface, #fff); border:1px solid var(--border-subtle, #e2e8f0); border-radius:8px; padding:0.6rem 0.85rem; font-size:0.82rem;">
                                                        <div style="display:flex; align-items:center; gap:0.5rem; overflow:hidden; flex:1;">
                                                            <span style=format!(
                                                                "font-size:0.75rem; padding:2px 6px; border-radius:4px; font-weight:700; white-space:nowrap; background:{}; color:{};",
                                                                if is_present { "#fef3c7" } else if is_slide { "#ecfdf5" } else { "#eff6ff" },
                                                                if is_present { "#d97706" } else if is_slide { "#059669" } else { "#2563eb" },
                                                            )>
                                                                {if is_present { "⚡ Present" } else if is_slide { "🖥️ Slide" } else { "📄 PDF" }}
                                                            </span>
                                                        <input
                                                            type="text"
                                                            readonly
                                                            class="form-control"
                                                            style="height:30px; font-size:0.8rem; font-family:monospace; flex:1; min-width:180px;"
                                                            prop:value=full_url
                                                            onclick="this.select()"
                                                        />
                                                        <span style="font-size:0.75rem; color:var(--text-sub); white-space:nowrap;" title=if zh { "访问次数" } else { "View count" }>
                                                            "👁️ " {lnk.view_count}
                                                        </span>
                                                        <span style="font-size:0.75rem; color:var(--text-sub); white-space:nowrap;">
                                                            {if let Some(exp) = lnk.expires_at {
                                                                let short_exp = exp.chars().take(10).collect::<String>();
                                                                format!("⏳ {short_exp}")
                                                            } else {
                                                                (if zh { "永久有效" } else { "Permanent" }).to_string()
                                                            }}
                                                        </span>
                                                    </div>
                                                    <div style="display:flex; align-items:center; gap:0.35rem; flex-shrink:0;">
                                                        <button
                                                            type="button"
                                                            class="btn btn-secondary btn-sm"
                                                            style="height:30px; font-size:0.75rem; padding:0 0.55rem;"
                                                            title=if zh { "复制链接" } else { "Copy Link" }
                                                            on:click=move |_| {
                                                                let msg = if zh { "安全分享链接已复制到剪贴板！".to_string() } else { "Secure link copied to clipboard!".to_string() };
                                                                copy_to_clipboard(full_url_copy.clone(), copy_feedback, msg);
                                                            }
                                                        >
                                                            "📋"
                                                        </button>
                                                        <a
                                                            href=full_url_open
                                                            target="_blank"
                                                            rel="noopener noreferrer"
                                                            class="btn btn-secondary btn-sm"
                                                            style="height:30px; font-size:0.75rem; padding:0 0.55rem; display:inline-flex; align-items:center;"
                                                            title=if zh { "打开测试" } else { "Open" }
                                                        >
                                                            "↗"
                                                        </a>
                                                        <button
                                                            type="button"
                                                            class="btn btn-danger btn-sm"
                                                            style="height:30px; font-size:0.75rem; padding:0 0.55rem; background:#fee2e2; color:#b91c1c; border:1px solid #fecaca;"
                                                            title=if zh { "撤销失效此链接" } else { "Revoke link" }
                                                            on:click={
                                                                let p = pid_this.clone();
                                                                let t = tok.clone();
                                                                move |_| {
                                                                    revoke_managed_link(p.clone(), t.clone(), shared_links, copy_feedback, zh);
                                                                }
                                                            }
                                                        >
                                                            {if zh { "🚫 撤销" } else { "🚫 Revoke" }}
                                                        </button>
                                                    </div>
                                                </div>
                                            }
                                        }).collect();
                                        view! { <div style="display:flex; flex-direction:column; gap:0.5rem;">{items}</div> }.into_any()
                                    }
                                }}
                            </div>
                        </div>

                        // SECTION 2: Internal Debug Links
                        <details style="background:var(--bg-muted, #f8fafc); border:1px solid var(--border-subtle, #e2e8f0); border-radius:10px; padding:0.75rem 1rem; margin-bottom:1rem;">
                            <summary style="font-weight:700; font-size:0.875rem; color:var(--text-sub); cursor:pointer; user-select:none;">
                                "🛠️ " {if zh { "项目成员内部调试直达路径 (仅限内部登录访问，不带Token)" } else { "Internal Workspace URL (For logged-in members only, no token)" }}
                            </summary>
                            <div style="margin-top:0.75rem;">
                                <p style="font-size:0.75rem; color:var(--text-sub); margin-bottom:0.6rem; line-height:1.4;">
                                    {if zh {
                                        "⚠️ 注意：以下内部路径直接包含项目 ID 与文件名，仅供已登录工作区成员本地预览调试使用，禁止对外公开发送。"
                                    } else {
                                        "Note: These direct routes expose project IDs and file paths. Use only for internal logged-in members."
                                    }}
                                </p>

                                // Slide Presentation Player Link Card
                                <div style="background:var(--bg-surface, #fff); border:1px solid var(--border-subtle, #e2e8f0); border-radius:8px; padding:0.75rem 1rem; margin-bottom:0.75rem;">
                                    <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:0.3rem;">
                                        <div style="font-weight:600; font-size:0.85rem; color:var(--text-main); display:flex; align-items:center; gap:0.4rem;">
                                            <span>"🖥️"</span>
                                            <span>{if zh { "内部幻灯片播放路径" } else { "Internal Slide Player Link" }}</span>
                                        </div>
                                    </div>
                                    <div style="display:flex; gap:0.5rem; align-items:center;">
                                        <input
                                            type="text"
                                            readonly
                                            class="form-control"
                                            style="flex:1; height:32px; font-size:0.8rem; font-family:monospace; background:var(--bg-surface, #fff);"
                                            prop:value=move || slide_internal_url.get()
                                            onclick="this.select()"
                                        />
                                        <button
                                            type="button"
                                            class="btn btn-secondary btn-sm"
                                            style="height:32px; white-space:nowrap; padding:0 0.75rem; font-size:0.8rem;"
                                            on:click=move |_| {
                                                let url = slide_internal_url.get();
                                                let msg = if zh { "内部路径已复制！".to_string() } else { "Internal URL copied!".to_string() };
                                                copy_to_clipboard(url, copy_feedback, msg);
                                            }
                                        >
                                            "📋 " {if zh { "复制" } else { "Copy" }}
                                        </button>
                                        <a
                                            href=move || slide_internal_url.get()
                                            target="_blank"
                                            rel="noopener noreferrer"
                                            class="btn btn-secondary btn-sm"
                                            style="height:32px; white-space:nowrap; padding:0 0.75rem; font-size:0.8rem; display:inline-flex; align-items:center;"
                                        >
                                            "▶ " {if zh { "内部放映" } else { "Play" }}
                                        </a>
                                    </div>
                                </div>

                                // PDF Viewer Link Card
                                <div style="background:var(--bg-surface, #fff); border:1px solid var(--border-subtle, #e2e8f0); border-radius:8px; padding:0.75rem 1rem; margin-bottom:0.75rem;">
                                    <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:0.3rem;">
                                        <div style="font-weight:600; font-size:0.85rem; color:var(--text-main); display:flex; align-items:center; gap:0.4rem;">
                                            <span>"📄"</span>
                                            <span>{if zh { "内部 PDF 预览路径" } else { "Internal PDF Viewer Link" }}</span>
                                        </div>
                                    </div>
                                    <div style="display:flex; gap:0.5rem; align-items:center;">
                                        <input
                                            type="text"
                                            readonly
                                            class="form-control"
                                            style="flex:1; height:32px; font-size:0.8rem; font-family:monospace; background:var(--bg-surface, #fff);"
                                            prop:value=move || pdf_internal_url.get()
                                            onclick="this.select()"
                                        />
                                        <button
                                            type="button"
                                            class="btn btn-secondary btn-sm"
                                            style="height:32px; white-space:nowrap; padding:0 0.75rem; font-size:0.8rem;"
                                            on:click=move |_| {
                                                let url = pdf_internal_url.get();
                                                let msg = if zh { "内部路径已复制！".to_string() } else { "Internal URL copied!".to_string() };
                                                copy_to_clipboard(url, copy_feedback, msg);
                                            }
                                        >
                                            "📋 " {if zh { "复制" } else { "Copy" }}
                                        </button>
                                        <a
                                            href=move || pdf_internal_url.get()
                                            target="_blank"
                                            rel="noopener noreferrer"
                                            class="btn btn-secondary btn-sm"
                                            style="height:32px; white-space:nowrap; padding:0 0.75rem; font-size:0.8rem; display:inline-flex; align-items:center;"
                                        >
                                            "↗ " {if zh { "内部查看" } else { "View" }}
                                        </a>
                                    </div>
                                </div>
                            </div>
                        </details>
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
                            action=format!("/projects/{}/files/share", pid_for_form)
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

                    // TAB 4: All Project Shared Links Management
                    <div style:display=move || if active_tab.get() == "all_links" { "flex" } else { "none" } style="flex-direction:column; gap:0; min-height:200px;">
                        <div style="margin-bottom:1rem; display:flex; align-items:center; justify-content:space-between; flex-wrap:wrap; gap:0.5rem;">
                            <div style="font-size:0.85rem; color:var(--text-sub); line-height:1.4; flex:1;">
                                {if zh {
                                    "管理本项目中所有文件的对外分享临时链接，可随时复制或一键撤销失效。"
                                } else {
                                    "Manage all temporary share links across every file in this project. Copy or revoke any link at any time."
                                }}
                            </div>
                            <button
                                type="button"
                                class="btn btn-secondary btn-sm"
                                style="height:32px; font-size:0.8rem; padding:0 0.85rem; display:inline-flex; align-items:center; gap:0.35rem; flex-shrink:0;"
                                disabled=move || is_all_links_loading.get()
                                on:click={
                                    let pid = pid_for_all_rev.clone();
                                    move |_| {
                                        fetch_shared_links(pid.clone(), String::new(), all_links, is_all_links_loading);
                                    }
                                }
                            >
                                {move || if is_all_links_loading.get() {
                                    view! { <span>"⏳"</span> }.into_any()
                                } else {
                                    view! { <span>"🔄 " {if zh { "刷新" } else { "Refresh" }}</span> }.into_any()
                                }}
                            </button>
                        </div>

                        // All-links table
                        {move || {
                            let links = all_links.get();
                            let loading = is_all_links_loading.get();

                            if loading && links.is_empty() {
                                view! {
                                    <div style="padding:2rem; text-align:center; font-size:0.85rem; color:var(--text-sub);">
                                        "⏳ " {if zh { "正在加载全部分享链接..." } else { "Loading all shared links..." }}
                                    </div>
                                }.into_any()
                            } else if links.is_empty() {
                                view! {
                                    <div style="padding:2rem; text-align:center; font-size:0.85rem; color:var(--text-sub); background:rgba(0,0,0,0.02); border-radius:8px; border:1px dashed var(--border-subtle, #cbd5e1);">
                                        {if zh { "该项目暂无任何对外分享链接。" } else { "No shared links have been created for this project yet." }}
                                    </div>
                                }.into_any()
                            } else {
                                // Group by file_path for a readable layout
                                let mut by_file: Vec<(String, Vec<ManagedSharedLink>)> = Vec::new();
                                for lnk in links {
                                    if let Some(group) = by_file.iter_mut().find(|(fp, _)| fp == &lnk.file_path) {
                                        group.1.push(lnk);
                                    } else {
                                        by_file.push((lnk.file_path.clone(), vec![lnk]));
                                    }
                                }

                                let groups: Vec<_> = by_file.into_iter().map(|(file, file_links)| {
                                    let file_display = file.clone();
                                    let rows: Vec<_> = file_links.into_iter().map(|lnk| {
                                        let full_url = lnk.url.clone();
                                        let full_url_copy = full_url.clone();
                                        let tok = lnk.token.clone();
                                        let pid_rev = pid_for_form.clone();
                                        let is_present = lnk.target_type == "present";
                                        let is_slide = lnk.target_type == "slide";
                                        let is_revoked = lnk.is_revoked;

                                        view! {
                                            <div style=format!(
                                                "display:flex; align-items:center; justify-content:space-between; gap:0.5rem; padding:0.55rem 0.85rem; border-bottom:1px solid var(--border-subtle, #f1f5f9); font-size:0.82rem; {};",
                                                if is_revoked { "opacity:0.45; text-decoration:line-through;" } else { "" }
                                            )>
                                                <div style="display:flex; align-items:center; gap:0.5rem; overflow:hidden; flex:1; min-width:0;">
                                                    <span style=format!(
                                                        "font-size:0.7rem; padding:2px 5px; border-radius:4px; font-weight:700; white-space:nowrap; flex-shrink:0; background:{}; color:{};",
                                                        if is_present { "#fef3c7" } else if is_slide { "#ecfdf5" } else { "#eff6ff" },
                                                        if is_present { "#d97706" } else if is_slide { "#059669" } else { "#2563eb" },
                                                    )>
                                                        {if is_present { "⚡" } else if is_slide { "🖥️" } else { "📄" }}
                                                    </span>
                                                    <input
                                                        type="text"
                                                        readonly
                                                        class="form-control"
                                                        style="height:28px; font-size:0.78rem; font-family:monospace; flex:1; min-width:0;"
                                                        prop:value=full_url
                                                        onclick="this.select()"
                                                    />
                                                    <div style="display:flex; flex-direction:column; align-items:flex-end; gap:1px; flex-shrink:0;">
                                                        <span style="font-size:0.7rem; color:var(--text-sub); white-space:nowrap;">
                                                            "👁️ " {lnk.view_count}
                                                        </span>
                                                        <span style="font-size:0.7rem; color:var(--text-sub); white-space:nowrap;">
                                                            {if let Some(exp) = lnk.expires_at {
                                                                format!("⏳ {}", exp.chars().take(10).collect::<String>())
                                                            } else {
                                                                (if zh { "永久" } else { "∞ never" }).to_string()
                                                            }}
                                                        </span>
                                                        <span style=format!(
                                                            "font-size:0.7rem; padding:1px 5px; border-radius:9999px; white-space:nowrap; background:{}; color:{};",
                                                            if lnk.allow_comments { "rgba(16,185,129,0.12)" } else { "rgba(239,68,68,0.1)" },
                                                            if lnk.allow_comments { "#059669" } else { "#dc2626" },
                                                        )>
                                                            {if lnk.allow_comments {
                                                                if zh { "💬 评论" } else { "💬 cmts" }
                                                            } else {
                                                                if zh { "🔇 无评论" } else { "🔇 no cmts" }
                                                            }}
                                                        </span>
                                                    </div>
                                                </div>
                                                <div style="display:flex; align-items:center; gap:0.3rem; flex-shrink:0;">
                                                    {if !is_revoked {
                                                        view! {
                                                            <button
                                                                type="button"
                                                                class="btn btn-secondary btn-sm"
                                                                style="height:28px; font-size:0.75rem; padding:0 0.45rem;"
                                                                title=if zh { "复制链接" } else { "Copy link" }
                                                                on:click=move |_| {
                                                                    let msg = if zh { "链接已复制！".to_string() } else { "Link copied!".to_string() };
                                                                    copy_to_clipboard(full_url_copy.clone(), copy_feedback, msg);
                                                                }
                                                            >
                                                                "📋"
                                                            </button>
                                                        }.into_any()
                                                    } else {
                                                        view! { <span style="font-size:0.7rem; color:#b91c1c; font-weight:700;">"✕"</span> }.into_any()
                                                    }}
                                                    {if !is_revoked {
                                                        view! {
                                                            <button
                                                                type="button"
                                                                class="btn btn-danger btn-sm"
                                                                style="height:28px; font-size:0.72rem; padding:0 0.5rem; background:#fee2e2; color:#b91c1c; border:1px solid #fecaca; white-space:nowrap;"
                                                                title=if zh { "撤销此链接" } else { "Revoke" }
                                                                on:click={
                                                                    let p = pid_rev.clone();
                                                                    let t = tok.clone();
                                                                    move |_| {
                                                                        revoke_managed_link(p.clone(), t.clone(), all_links, copy_feedback, zh);
                                                                    }
                                                                }
                                                            >
                                                                {if zh { "🚫 撤销" } else { "🚫 Revoke" }}
                                                            </button>
                                                        }.into_any()
                                                    } else {
                                                        view! { <span style="font-size:0.7rem; color:#94a3b8;">" "</span> }.into_any()
                                                    }}
                                                </div>
                                            </div>
                                        }
                                    }).collect();

                                    view! {
                                        <div style="margin-bottom:0.85rem; background:var(--bg-muted, #f8fafc); border:1px solid var(--border-subtle, #e2e8f0); border-radius:8px; overflow:hidden;">
                                            <div style="padding:0.5rem 0.85rem; background:var(--bg-surface, #fff); border-bottom:1px solid var(--border-subtle, #e2e8f0); display:flex; align-items:center; gap:0.4rem;">
                                                <span style="font-size:0.8rem; font-weight:700; color:var(--primary, #635bff); font-family:monospace;">{file_display}</span>
                                            </div>
                                            {rows}
                                        </div>
                                    }
                                }).collect::<Vec<_>>();

                                view! { <div style="display:flex; flex-direction:column;">{groups}</div> }.into_any()
                            }
                        }}
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
    project_id: String,
    shared_links: RwSignal<Vec<ManagedSharedLink>>,
    is_links_loading: RwSignal<bool>,
    new_link_target: RwSignal<String>,
) {
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;

    let pid_closure = project_id.clone();
    let closure = Closure::<dyn Fn(web_sys::CustomEvent)>::new(move |ev: web_sys::CustomEvent| {
        let raw_detail = ev.detail();
        let detail = if raw_detail.is_string() {
            raw_detail
                .as_string()
                .and_then(|s| js_sys::JSON::parse(&s).ok())
                .unwrap_or(raw_detail)
        } else {
            raw_detail
        };
        let get_str = |key: &str| -> String {
            js_sys::Reflect::get(&detail, &key.into())
                .ok()
                .and_then(|v| v.as_string())
                .unwrap_or_default()
        };
        let p = get_str("path");
        file_path.set(p.clone());
        let raw_target = get_str("target");
        let initial_target = if !raw_target.is_empty() {
            raw_target
        } else if p.ends_with(".typ") || p.ends_with(".slide") {
            "present".to_string()
        } else {
            "pdf".to_string()
        };
        new_link_target.set(initial_target);
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
        fetch_shared_links(pid_closure.clone(), p, shared_links, is_links_loading);
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
    _project_id: String,
    _shared_links: RwSignal<Vec<ManagedSharedLink>>,
    _is_links_loading: RwSignal<bool>,
    _new_link_target: RwSignal<String>,
) {
}

#[cfg(feature = "hydrate")]
fn fetch_shared_links(
    project_id: String,
    file_path: String,
    links: RwSignal<Vec<ManagedSharedLink>>,
    is_loading: RwSignal<bool>,
) {
    if project_id.is_empty() {
        return;
    }
    is_loading.set(true);
    wasm_bindgen_futures::spawn_local(async move {
        let file_query = if file_path.is_empty() {
            String::new()
        } else {
            format!("?file={}", urlencoding::encode(&file_path))
        };
        let url = format!("/api/projects/{}/shared-links{}", project_id, file_query);
        if let Ok(resp) = gloo_net::http::Request::get(&url).send().await {
            if let Ok(data) = resp.json::<serde_json::Value>().await {
                if let Some(arr) = data.get("shared_links").and_then(|v| v.as_array()) {
                    let mut items: Vec<ManagedSharedLink> = arr
                        .iter()
                        .filter_map(|val| serde_json::from_value(val.clone()).ok())
                        .collect();
                    if items.is_empty() && !file_path.is_empty() {
                        let is_slide = file_path.ends_with(".typ") || file_path.ends_with(".slide");
                        let target_type = if is_slide { "present" } else { "pdf" };
                        let post_url = format!("/api/projects/{}/shared-links", project_id);
                        let post_body = serde_json::json!({
                            "file_path": file_path,
                            "target_type": target_type,
                            "expiry_days": 30,
                            "allow_comments": true,
                        });
                        if let Ok(req) = gloo_net::http::Request::post(&post_url)
                            .header("Content-Type", "application/json")
                            .body(post_body.to_string())
                        {
                            if let Ok(res) = req.send().await {
                                if let Ok(created_json) = res.json::<serde_json::Value>().await {
                                    if let Some(created_val) = created_json.get("shared_link") {
                                        if let Ok(new_l) = serde_json::from_value::<ManagedSharedLink>(created_val.clone()) {
                                            items.push(new_l);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    links.set(items);
                }
            }
        }
        is_loading.set(false);
    });
}

#[cfg(not(feature = "hydrate"))]
fn fetch_shared_links(
    _project_id: String,
    _file_path: String,
    _links: RwSignal<Vec<ManagedSharedLink>>,
    _is_loading: RwSignal<bool>,
) {
}

#[cfg(feature = "hydrate")]
fn create_managed_link(
    project_id: String,
    file_path: String,
    target_type: String,
    expiry_days: Option<i64>,
    allow_comments: bool,
    is_creating: RwSignal<bool>,
    links: RwSignal<Vec<ManagedSharedLink>>,
    feedback: RwSignal<Option<String>>,
    zh: bool,
) {
    if project_id.is_empty() || file_path.is_empty() {
        return;
    }
    is_creating.set(true);
    wasm_bindgen_futures::spawn_local(async move {
        let url = format!("/api/projects/{}/shared-links", project_id);
        let req_body = serde_json::json!({
            "file_path": file_path,
            "target_type": target_type,
            "expiry_days": expiry_days,
            "allow_comments": allow_comments,
        });
        let resp = gloo_net::http::Request::post(&url)
            .header("Content-Type", "application/json")
            .body(req_body.to_string());
        if let Ok(req) = resp {
            if let Ok(res) = req.send().await {
                if res.ok() {
                    if let Ok(data) = res.json::<serde_json::Value>().await {
                        if let Some(link_val) = data.get("shared_link") {
                            if let Ok(new_link) =
                                serde_json::from_value::<ManagedSharedLink>(link_val.clone())
                            {
                                links.update(|l| l.insert(0, new_link));
                                let msg = if zh {
                                    "已生成新的安全临时分享链接！".to_string()
                                } else {
                                    "New secure shared link generated!".to_string()
                                };
                                feedback.set(Some(msg));
                            }
                        }
                    }
                }
            }
        }
        is_creating.set(false);
    });
}

#[cfg(not(feature = "hydrate"))]
fn create_managed_link(
    _project_id: String,
    _file_path: String,
    _target_type: String,
    _expiry_days: Option<i64>,
    _allow_comments: bool,
    _is_creating: RwSignal<bool>,
    _links: RwSignal<Vec<ManagedSharedLink>>,
    _feedback: RwSignal<Option<String>>,
    _zh: bool,
) {
}

#[cfg(feature = "hydrate")]
fn revoke_managed_link(
    project_id: String,
    token: String,
    links: RwSignal<Vec<ManagedSharedLink>>,
    feedback: RwSignal<Option<String>>,
    zh: bool,
) {
    if project_id.is_empty() || token.is_empty() {
        return;
    }
    let tok_for_update = token.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let url = format!("/api/projects/{}/shared-links/{}/revoke", project_id, token);
        let resp = gloo_net::http::Request::post(&url).send().await;
        if let Ok(res) = resp {
            if res.ok() {
                links.update(|l| l.retain(|item| item.token != tok_for_update));
                let msg = if zh {
                    "分享链接已成功撤销，访问已失效。".to_string()
                } else {
                    "Shared link revoked successfully.".to_string()
                };
                feedback.set(Some(msg));
            }
        }
    });
}

#[cfg(not(feature = "hydrate"))]
fn revoke_managed_link(
    _project_id: String,
    _token: String,
    _links: RwSignal<Vec<ManagedSharedLink>>,
    _feedback: RwSignal<Option<String>>,
    _zh: bool,
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
