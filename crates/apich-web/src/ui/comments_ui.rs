//! Comments drawer, in-browser PDF viewer, and in-browser share modal UI components.

use crate::ui::handlers::html_escape;

/// Render the comments drawer and in-browser share modal HTML, CSS, and JS engine.
///
/// Can be embedded in the cargo-slide presentation shell or in the PDF viewer.
#[allow(clippy::too_many_arguments)]
pub fn render_comments_drawer_component(
    project_id: &str,
    file_path: &str,
    token: Option<&str>,
    current_user_name: &str,
    is_slide: bool,
    show_fab: bool,
    share_url: Option<&str>,
    is_zh: bool,
    allow_comments: bool,
    expiry_days: i64,
) -> String {
    let esc_project_id = html_escape(project_id);
    let esc_file_path = html_escape(file_path);
    let esc_token = html_escape(token.unwrap_or(""));
    let esc_user = html_escape(current_user_name);
    let esc_share_url = html_escape(share_url.unwrap_or(""));
    let context_type = if is_slide {
        if is_zh { "幻灯片" } else { "Slide" }
    } else {
        if is_zh { "第" } else { "Page" }
    };
    let context_name_js = if is_slide { "Slide" } else { "Page" };

    let (sel_1, sel_7, sel_30, sel_90, sel_365, sel_0) = match expiry_days {
        1 => ("selected", "", "", "", "", ""),
        7 => ("", "selected", "", "", "", ""),
        30 => ("", "", "selected", "", "", ""),
        90 => ("", "", "", "selected", "", ""),
        365 => ("", "", "", "", "selected", ""),
        0 => ("", "", "", "", "", "selected"),
        _ => ("", "", "selected", "", "", ""),
    };
    let comments_checked_attr = if allow_comments { "checked" } else { "" };
    let comments_badge_style = if allow_comments {
        "background: rgba(16, 185, 129, 0.15); color: #34d399;"
    } else {
        "background: rgba(239, 68, 68, 0.15); color: #f87171;"
    };
    let comments_badge_text = if allow_comments {
        if is_zh { "评论已开启" } else { "Comments Enabled" }
    } else {
        if is_zh { "评论已关闭" } else { "Comments Disabled" }
    };
    let disabled_banner_display = if !allow_comments && token.is_some() { "block" } else { "none" };
    let form_display = if !allow_comments && token.is_some() { "display: none;" } else { "" };

    format!(
        r##"
<!-- APICH Comments Drawer & In-Browser Share Component -->
<style>
  :root {{
    --apich-c-bg: #0f172a;
    --apich-c-surface: #1e293b;
    --apich-c-surface-hover: #334155;
    --apich-c-border: #334155;
    --apich-c-primary: #6366f1;
    --apich-c-primary-hover: #4f46e5;
    --apich-c-text: #f8fafc;
    --apich-c-text-muted: #94a3b8;
    --apich-c-badge-bg: #312e81;
    --apich-c-badge-text: #a5b4fc;
  }}

  /* Floating Action Bar (Top-Right of Presentation Player) */
  .apich-floating-bar {{
    position: fixed;
    top: 20px;
    right: 24px;
    z-index: 99998;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    background: rgba(15, 23, 42, 0.85);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 9999px;
    padding: 6px 12px;
    box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.5), 0 0 0 1px rgba(255, 255, 255, 0.05);
    transition: opacity 0.25s ease, transform 0.25s ease;
    user-select: none;
  }}
  .apich-float-btn {{
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: transparent;
    color: #ffffff;
    border: none;
    border-radius: 9999px;
    padding: 6px 12px;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s ease;
  }}
  .apich-float-btn:hover {{
    background: rgba(255, 255, 255, 0.12);
  }}
  .apich-float-btn.primary {{
    background: rgba(99, 102, 241, 0.3);
    color: #c7d2fe;
    border: 1px solid rgba(99, 102, 241, 0.4);
  }}
  .apich-float-btn.primary:hover {{
    background: rgba(99, 102, 241, 0.5);
    color: #ffffff;
  }}
  .apich-comments-fab-badge {{
    background: #6366f1;
    color: #ffffff;
    font-size: 11px;
    font-weight: 700;
    padding: 2px 7px;
    border-radius: 9999px;
    min-width: 18px;
    text-align: center;
    line-height: 1.3;
  }}

  /* Modal & Drawer Overlays */
  .apich-overlay-backdrop {{
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background: rgba(0, 0, 0, 0.5);
    backdrop-filter: blur(4px);
    -webkit-backdrop-filter: blur(4px);
    z-index: 99999;
    opacity: 0;
    visibility: hidden;
    transition: opacity 0.25s ease, visibility 0.25s ease;
  }}
  .apich-overlay-backdrop.open {{
    opacity: 1;
    visibility: visible;
  }}

  /* Drawer Panel */
  .apich-drawer-panel {{
    position: fixed;
    top: 0;
    right: 0;
    width: min(440px, 92vw);
    height: 100vh;
    background: #0f172a;
    border-left: 1px solid #334155;
    z-index: 100000;
    display: flex;
    flex-direction: column;
    box-shadow: -10px 0 35px -5px rgba(0, 0, 0, 0.5);
    transform: translateX(100%);
    transition: transform 0.28s cubic-bezier(0.16, 1, 0.3, 1);
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    color: #f8fafc;
  }}
  .apich-drawer-panel.open {{
    transform: translateX(0);
  }}

  /* Drawer Header */
  .apich-drawer-header {{
    padding: 16px 20px;
    border-bottom: 1px solid #334155;
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: #1e293b;
    flex-shrink: 0;
  }}
  .apich-drawer-title {{
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 16px;
    font-weight: 700;
    margin: 0;
  }}
  .apich-drawer-close {{
    background: none;
    border: none;
    color: #94a3b8;
    font-size: 24px;
    cursor: pointer;
    line-height: 1;
    padding: 4px;
    border-radius: 6px;
    transition: color 0.15s ease;
  }}
  .apich-drawer-close:hover {{
    color: #f8fafc;
    background: rgba(255, 255, 255, 0.08);
  }}

  /* Tab Filter */
  .apich-drawer-tabs {{
    display: flex;
    padding: 10px 20px;
    gap: 8px;
    background: #0f172a;
    border-bottom: 1px solid #1e293b;
    flex-shrink: 0;
  }}
  .apich-tab-btn {{
    padding: 6px 14px;
    font-size: 13px;
    font-weight: 600;
    border-radius: 8px;
    border: 1px solid transparent;
    background: transparent;
    color: #94a3b8;
    cursor: pointer;
    transition: all 0.15s ease;
  }}
  .apich-tab-btn.active {{
    background: #1e293b;
    color: #f8fafc;
    border-color: #334155;
  }}

  /* Comment List */
  .apich-comments-list {{
    flex: 1;
    overflow-y: auto;
    padding: 16px 20px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }}
  .apich-comment-card {{
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 10px;
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }}
  .apich-comment-top {{
    display: flex;
    align-items: center;
    justify-content: space-between;
  }}
  .apich-comment-author-box {{
    display: flex;
    align-items: center;
    gap: 8px;
  }}
  .apich-comment-avatar {{
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: #6366f1;
    color: #ffffff;
    font-size: 11px;
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
  }}
  .apich-comment-author {{
    font-size: 13px;
    font-weight: 600;
    color: #f8fafc;
  }}
  .apich-comment-tag {{
    font-size: 11px;
    font-weight: 600;
    background: #312e81;
    color: #c7d2fe;
    padding: 2px 7px;
    border-radius: 6px;
  }}
  .apich-comment-body {{
    font-size: 13.5px;
    line-height: 1.5;
    color: #cbd5e1;
    white-space: pre-wrap;
    word-break: break-word;
  }}
  .apich-comment-foot {{
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 11px;
    color: #64748b;
    margin-top: 2px;
  }}
  .apich-comment-del {{
    background: none;
    border: none;
    color: #ef4444;
    cursor: pointer;
    font-size: 11px;
    padding: 2px 6px;
    border-radius: 4px;
  }}
  .apich-comment-del:hover {{
    background: rgba(239, 68, 68, 0.15);
  }}

  /* Empty State */
  .apich-comments-empty {{
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 40px 20px;
    color: #64748b;
    gap: 10px;
  }}
  .apich-comments-empty-icon {{
    font-size: 32px;
  }}

  /* Form Area */
  .apich-drawer-form {{
    padding: 16px 20px;
    background: #1e293b;
    border-top: 1px solid #334155;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }}
  .apich-form-row {{
    display: flex;
    gap: 8px;
  }}
  .apich-form-input {{
    background: #0f172a;
    border: 1px solid #334155;
    border-radius: 6px;
    color: #f8fafc;
    padding: 8px 12px;
    font-size: 13px;
    font-family: inherit;
    outline: none;
    transition: border-color 0.15s ease;
  }}
  .apich-form-input:focus {{
    border-color: #6366f1;
  }}
  .apich-form-textarea {{
    background: #0f172a;
    border: 1px solid #334155;
    border-radius: 6px;
    color: #f8fafc;
    padding: 8px 12px;
    font-size: 13px;
    font-family: inherit;
    resize: none;
    outline: none;
    transition: border-color 0.15s ease;
  }}
  .apich-form-textarea:focus {{
    border-color: #6366f1;
  }}
  .apich-btn-submit {{
    background: #6366f1;
    color: #ffffff;
    border: none;
    border-radius: 6px;
    padding: 8px 16px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.15s ease;
  }}
  .apich-btn-submit:hover {{
    background: #4f46e5;
  }}

  /* IN-BROWSER SHARE MODAL */
  .apich-share-dialog {{
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%) scale(0.95);
    width: min(520px, 92vw);
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 14px;
    z-index: 100002;
    padding: 24px;
    box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.6);
    opacity: 0;
    visibility: hidden;
    transition: all 0.25s cubic-bezier(0.16, 1, 0.3, 1);
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    color: #f8fafc;
  }}
  .apich-share-dialog.open {{
    opacity: 1;
    visibility: visible;
    transform: translate(-50%, -50%) scale(1);
  }}
  .apich-share-header {{
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
  }}
  .apich-share-title {{
    font-size: 16px;
    font-weight: 700;
    margin: 0;
    display: flex;
    align-items: center;
    gap: 8px;
  }}
  .apich-share-tabs {{
    display: flex;
    gap: 8px;
    border-bottom: 1px solid #334155;
    margin-bottom: 16px;
  }}
  .apich-share-tab-btn {{
    padding: 8px 14px;
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    color: #94a3b8;
    font-size: 13.5px;
    font-weight: 600;
    cursor: pointer;
  }}
  .apich-share-tab-btn.active {{
    color: #6366f1;
    border-bottom-color: #6366f1;
  }}
  .apich-share-section {{
    display: none;
  }}
  .apich-share-section.active {{
    display: block;
  }}
</style>

<!-- Floating Action Bar for Slides -->
{floating_bar_html}

<!-- Backdrop Overlay for Drawer & Share Modal -->
<div id="apich-overlay-bg" class="apich-overlay-backdrop" onclick="window.apichCloseAllModals()"></div>

<!-- Slide & Document Comments Drawer -->
<aside id="apich-comments-drawer" class="apich-drawer-panel" aria-label="Comments">
  <div class="apich-drawer-header">
    <h3 class="apich-drawer-title">
      <span>💬</span>
      <span>{comments_title}</span>
      <span id="apich-drawer-badge" class="apich-comments-fab-badge">0</span>
    </h3>
    <button type="button" class="apich-drawer-close" onclick="window.apichToggleComments()" title="Close drawer">×</button>
  </div>

  <div class="apich-drawer-tabs">
    <button type="button" id="apich-tab-all" class="apich-tab-btn active" onclick="window.apichSetFilter('all')">
      {all_comments_label}
    </button>
    <button type="button" id="apich-tab-current" class="apich-tab-btn" onclick="window.apichSetFilter('current')">
      <span id="apich-current-context-label">{current_context_label} 1</span>
    </button>
  </div>

  <div id="apich-comments-container" class="apich-comments-list">
    <div class="apich-comments-empty">
      <span class="apich-comments-empty-icon">⏳</span>
      <span>{loading_label}</span>
    </div>
  </div>

  <div id="apich-comments-disabled-banner" style="display: {disabled_banner_display}; padding: 12px 14px; background: rgba(239, 68, 68, 0.1); border: 1px solid rgba(239, 68, 68, 0.25); border-radius: 8px; font-size: 12.5px; color: #fca5a5; margin: 12px; text-align: center;">
    🔒 {comments_disabled_notice}
  </div>

  <form id="apich-comment-form" class="apich-drawer-form" onsubmit="window.apichSubmitComment(event)" style="{form_display}">
    <div class="apich-form-row">
      <input
        type="text"
        id="apich-comment-author"
        class="apich-form-input"
        placeholder="{author_placeholder}"
        value="{esc_user}"
        style="flex: 2;"
        required
      />
      <input
        type="number"
        id="apich-comment-page"
        class="apich-form-input"
        placeholder="{context_type} #"
        style="flex: 1;"
        min="1"
      />
    </div>
    <textarea
      id="apich-comment-content"
      class="apich-form-textarea"
      placeholder="{content_placeholder}"
      rows="2"
      required
    ></textarea>
    <div style="display: flex; justify-content: space-between; align-items: center;">
      <span id="apich-comment-status" style="font-size: 12px; color: #94a3b8;"></span>
      <button type="submit" id="apich-comment-submit-btn" class="apich-btn-submit">{post_btn_label}</button>
    </div>
  </form>
</aside>

<!-- IN-BROWSER SHARE MODAL DIALOG -->
<div id="apich-share-dialog" class="apich-share-dialog">
  <div class="apich-share-header">
    <h3 class="apich-share-title">
      <span>🔗</span>
      <span>{share_modal_title}</span>
    </h3>
    <button type="button" class="apich-drawer-close" onclick="window.apichToggleShareModal()">×</button>
  </div>

  <div class="apich-share-tabs">
    <button type="button" id="apich-share-tab-link" class="apich-share-tab-btn active" onclick="window.apichSwitchShareTab('link')">
      🔗 {share_tab_link_label}
    </button>
    <button type="button" id="apich-share-tab-email" class="apich-share-tab-btn" onclick="window.apichSwitchShareTab('email')">
      ✉️ {share_tab_email_label}
    </button>
  </div>

  <!-- Share Tab 1: Direct Link -->
  <div id="apich-share-sec-link" class="apich-share-section active">
    <p style="font-size: 13px; color: #94a3b8; line-height: 1.4; margin-bottom: 12px;">
      {share_link_desc}
    </p>

    <!-- Share Options (Comments toggle + Expiration selection) -->
    <div id="apich-share-options-box" style="background: rgba(15, 23, 42, 0.6); border: 1px solid #334155; border-radius: 8px; padding: 12px; margin-bottom: 14px; display: flex; flex-direction: column; gap: 10px;">
      <!-- Comment Permission Row -->
      <div style="display: flex; align-items: center; justify-content: space-between;">
        <label for="apich-share-comments-toggle" style="display: flex; align-items: center; gap: 8px; cursor: pointer; user-select: none;">
          <input
            type="checkbox"
            id="apich-share-comments-toggle"
            {comments_checked_attr}
            style="width: 16px; height: 16px; accent-color: #6366f1; cursor: pointer;"
            onchange="window.apichToggleCommentsBadge(this.checked)"
          />
          <span style="font-size: 13px; font-weight: 500; color: #e2e8f0;">{share_comments_label}</span>
        </label>
        <span id="apich-share-comments-badge" style="font-size: 11px; padding: 2px 8px; border-radius: 9999px; {comments_badge_style}">
          {comments_badge_text}
        </span>
      </div>

      <!-- Expiration Days Row -->
      <div style="display: flex; align-items: center; justify-content: space-between;">
        <label for="apich-share-expiry-select" style="display: flex; align-items: center; gap: 6px; font-size: 13px; font-weight: 500; color: #e2e8f0;">
          <span>⏳</span>
          <span>{share_expiry_label}</span>
        </label>
        <select
          id="apich-share-expiry-select"
          class="apich-form-input"
          style="width: auto; padding: 4px 10px; font-size: 12.5px; background: #0f172a; border: 1px solid #334155; border-radius: 6px; color: #f8fafc; cursor: pointer;"
        >
          <option value="1" {sel_1}>{opt_1_day}</option>
          <option value="7" {sel_7}>{opt_7_days}</option>
          <option value="30" {sel_30}>{opt_30_days}</option>
          <option value="90" {sel_90}>{opt_90_days}</option>
          <option value="365" {sel_365}>{opt_365_days}</option>
          <option value="0" {sel_0}>{opt_never}</option>
        </select>
      </div>

      <!-- Action Button: Explicitly generate / update share link -->
      <div style="display: flex; justify-content: flex-end; margin-top: 4px;">
        <button
          type="button"
          id="apich-share-generate-btn"
          class="apich-btn-submit"
          onclick="window.apichGenerateShareLink()"
          style="display: inline-flex; align-items: center; gap: 6px; font-size: 12.5px; padding: 6px 14px;"
        >
          <span>⚡</span>
          <span id="apich-share-generate-btn-text">{generate_link_btn_label}</span>
        </button>
      </div>
    </div>

    <div style="display: flex; gap: 8px; margin-bottom: 6px;">
      <input
        type="text"
        id="apich-share-link-input"
        class="apich-form-input"
        readonly
        placeholder="{share_link_placeholder}"
        style="flex: 1; font-family: monospace; font-size: 12.5px; background: #0f172a;"
        onclick="this.select()"
      />
      <button
        type="button"
        id="apich-share-copy-btn"
        class="apich-btn-submit"
        onclick="window.apichCopyShareModalLink()"
        style="white-space: nowrap;"
      >
        📋 {copy_btn_label}
      </button>
    </div>

    <div id="apich-share-options-status" style="font-size: 12px; min-height: 18px; margin-bottom: 10px; color: #94a3b8;"></div>

    <div style="background: rgba(99,102,241,0.1); border: 1px solid rgba(99,102,241,0.25); border-radius: 8px; padding: 10px 14px; font-size: 12.5px; color: #c7d2fe;">
      💡 {share_token_tip}
    </div>
  </div>

  <!-- Share Tab 2: Send via Email -->
  <div id="apich-share-sec-email" class="apich-share-section">
    <form onsubmit="window.apichSendShareModalEmail(event)">
      <div style="margin-bottom: 12px;">
        <label style="display: block; font-size: 12.5px; font-weight: 600; margin-bottom: 6px; color: #cbd5e1;">
          {email_recipient_label} *
        </label>
        <input
          type="email"
          id="apich-share-email-to"
          class="apich-form-input"
          placeholder="colleague@example.com"
          style="width: 100%;"
          required
        />
      </div>
      <div style="margin-bottom: 14px;">
        <label style="display: block; font-size: 12.5px; font-weight: 600; margin-bottom: 6px; color: #cbd5e1;">
          {email_note_label}
        </label>
        <textarea
          id="apich-share-email-note"
          class="apich-form-textarea"
          placeholder="{email_note_placeholder}"
          rows="3"
          style="width: 100%;"
        ></textarea>
      </div>
      <div id="apich-share-email-feedback" style="display: none; padding: 8px 12px; border-radius: 6px; font-size: 12.5px; margin-bottom: 12px;"></div>
      <div style="display: flex; justify-content: flex-end; gap: 8px;">
        <button type="submit" id="apich-share-email-submit-btn" class="apich-btn-submit" style="display: inline-flex; align-items: center; gap: 6px;">
          <span>✉️</span>
          <span>{send_email_btn_label}</span>
        </button>
      </div>
    </form>
  </div>
</div>

<script>
(function() {{
  const projectId = "{esc_project_id}";
  const filePath = "{esc_file_path}";
  const shareToken = "{esc_token}";
  const configuredShareUrl = "{esc_share_url}";
  const contextName = "{context_name_js}";
  const isZh = {is_zh};
  const isSlide = {is_slide};
  const isGuestShare = window.location.pathname.startsWith("/s/");

  let activeSlideOrPage = 1;
  let currentFilter = "all";
  let comments = [];

  function getBaseCommentsUrl() {{
    if (window.location.pathname.startsWith("/s/")) {{
      const match = window.location.pathname.match(/^\/s\/([^\/]+)/);
      if (match) return `/s/${{match[1]}}/comments`;
    }}
    return `/api/projects/${{projectId}}/files/comments`;
  }}

  let activeOpaqueUrl = configuredShareUrl ? (configuredShareUrl.startsWith("http") ? configuredShareUrl : window.location.origin + configuredShareUrl) : "";

  window.apichToggleCommentsBadge = function(checked) {{
    const commentsBadge = document.getElementById("apich-share-comments-badge");
    if (commentsBadge) {{
      if (checked) {{
        commentsBadge.textContent = isZh ? "评论已开启" : "Comments Enabled";
        commentsBadge.style.background = "rgba(16, 185, 129, 0.15)";
        commentsBadge.style.color = "#34d399";
      }} else {{
        commentsBadge.textContent = isZh ? "评论已关闭" : "Comments Disabled";
        commentsBadge.style.background = "rgba(239, 68, 68, 0.15)";
        commentsBadge.style.color = "#f87171";
      }}
    }}
  }};

  window.apichGenerateShareLink = async function() {{
    const commentsToggle = document.getElementById("apich-share-comments-toggle");
    const expirySelect = document.getElementById("apich-share-expiry-select");
    const statusEl = document.getElementById("apich-share-options-status");
    const linkInput = document.getElementById("apich-share-link-input");
    const btn = document.getElementById("apich-share-generate-btn");
    const btnText = document.getElementById("apich-share-generate-btn-text");

    if (!commentsToggle || !expirySelect) return activeOpaqueUrl;

    if (isGuestShare) {{
      if (statusEl) {{
        statusEl.textContent = isZh ? "访客模式：当前链接设置由创建者锁定" : "Guest Mode: Link settings managed by owner";
        statusEl.style.color = "#94a3b8";
      }}
      return activeOpaqueUrl;
    }}

    const allowComments = commentsToggle.checked;
    const expiryDays = parseInt(expirySelect.value, 10);

    if (btn) btn.disabled = true;
    if (btnText) btnText.textContent = isZh ? "生成中..." : "Generating...";
    if (statusEl) {{
      statusEl.textContent = isZh ? "⏳ 正在生成/更新安全分享链接..." : "⏳ Generating secure share link...";
      statusEl.style.color = "#818cf8";
    }}

    try {{
      const targetType = isSlide ? "slide" : "pdf";
      const resp = await fetch(`/api/projects/${{projectId}}/shared-links`, {{
        method: "POST",
        headers: {{ "Content-Type": "application/json" }},
        body: JSON.stringify({{
          file_path: filePath,
          target_type: targetType,
          expires_in_days: expiryDays > 0 ? expiryDays : null,
          allow_comments: allowComments
        }})
      }});

      if (resp.ok) {{
        const data = await resp.json();
        const shareUrl = data.share_url || (data.shared_link && data.shared_link.url) || "";
        if (shareUrl) {{
          activeOpaqueUrl = shareUrl.startsWith("http") ? shareUrl : `${{window.location.origin}}${{shareUrl}}`;
          if (linkInput) linkInput.value = activeOpaqueUrl;
          if (statusEl) {{
            statusEl.textContent = isZh ? "✓ 分享链接已生成就绪，可直接复制或发送" : "✓ Share link generated and ready to copy or send";
            statusEl.style.color = "#34d399";
            setTimeout(() => {{ if (statusEl && statusEl.textContent.startsWith("✓")) statusEl.textContent = ""; }}, 4000);
          }}
          return activeOpaqueUrl;
        }}
      }} else {{
        const err = await resp.json().catch(() => ({{}}));
        if (statusEl) {{
          statusEl.textContent = err.error || (isZh ? "生成链接失败，请重试" : "Failed to generate link");
          statusEl.style.color = "#f87171";
        }}
      }}
    }} catch (e) {{
      console.error("Error creating shared link:", e);
      if (statusEl) {{
        statusEl.textContent = String(e);
        statusEl.style.color = "#f87171";
      }}
    }} finally {{
      if (btn) btn.disabled = false;
      if (btnText) btnText.textContent = isZh ? "生成分享链接" : "Generate Share Link";
    }}
    return activeOpaqueUrl;
  }};

  window.apichUpdateShareOptions = window.apichGenerateShareLink;

  async function ensureOpaqueShareUrl() {{
    if (activeOpaqueUrl) return activeOpaqueUrl;
    if (isGuestShare) {{
      const match = window.location.pathname.match(/^\/s\/([^\/]+)/);
      if (match) {{
        activeOpaqueUrl = `${{window.location.origin}}/s/${{match[1]}}`;
        return activeOpaqueUrl;
      }}
    }}
    return await window.apichGenerateShareLink();
  }}

  window.apichToggleComments = function() {{
    const drawer = document.getElementById("apich-comments-drawer");
    const backdrop = document.getElementById("apich-overlay-bg");
    const shareDialog = document.getElementById("apich-share-dialog");
    if (!drawer) return;

    if (shareDialog) shareDialog.classList.remove("open");
    const isOpen = drawer.classList.contains("open");
    if (isOpen) {{
      drawer.classList.remove("open");
      if (backdrop) backdrop.classList.remove("open");
    }} else {{
      drawer.classList.add("open");
      if (backdrop) backdrop.classList.add("open");
      fetchComments();
    }}
  }};

  window.apichToggleShareModal = function() {{
    const shareDialog = document.getElementById("apich-share-dialog");
    const backdrop = document.getElementById("apich-overlay-bg");
    const drawer = document.getElementById("apich-comments-drawer");
    if (!shareDialog) return;

    if (drawer) drawer.classList.remove("open");
    const isOpen = shareDialog.classList.contains("open");
    if (isOpen) {{
      shareDialog.classList.remove("open");
      if (backdrop) backdrop.classList.remove("open");
    }} else {{
      shareDialog.classList.add("open");
      if (backdrop) backdrop.classList.add("open");
      const input = document.getElementById("apich-share-link-input");
      const commentsToggle = document.getElementById("apich-share-comments-toggle");
      const expirySelect = document.getElementById("apich-share-expiry-select");

      if (isGuestShare) {{
        if (commentsToggle) commentsToggle.disabled = true;
        if (expirySelect) expirySelect.disabled = true;
      }}

      if (input) {{
        if (activeOpaqueUrl) {{
          input.value = activeOpaqueUrl;
        }} else {{
          input.value = "";
        }}
      }}
    }}
  }};

  window.apichCloseAllModals = function() {{
    const drawer = document.getElementById("apich-comments-drawer");
    const shareDialog = document.getElementById("apich-share-dialog");
    const backdrop = document.getElementById("apich-overlay-bg");
    if (drawer) drawer.classList.remove("open");
    if (shareDialog) shareDialog.classList.remove("open");
    if (backdrop) backdrop.classList.remove("open");
  }};

  window.apichSwitchShareTab = function(tab) {{
    const linkBtn = document.getElementById("apich-share-tab-link");
    const emailBtn = document.getElementById("apich-share-tab-email");
    const linkSec = document.getElementById("apich-share-sec-link");
    const emailSec = document.getElementById("apich-share-sec-email");

    if (tab === "link") {{
      linkBtn.classList.add("active");
      emailBtn.classList.remove("active");
      linkSec.classList.add("active");
      emailSec.classList.remove("active");
    }} else {{
      emailBtn.classList.add("active");
      linkBtn.classList.remove("active");
      emailSec.classList.add("active");
      linkSec.classList.remove("active");
    }}
  }};

  window.apichCopyShareModalLink = async function() {{
    const input = document.getElementById("apich-share-link-input");
    const btn = document.getElementById("apich-share-copy-btn");
    if (!input) return;
    const copyVal = (val) => {{
      navigator.clipboard.writeText(val).then(() => {{
        const orig = btn.textContent;
        btn.textContent = isZh ? "✓ 已复制安全链接" : "✓ Copied Secure Link!";
        setTimeout(() => {{ btn.textContent = orig; }}, 2500);
      }});
    }};
    if (input.value && input.value.startsWith("http")) {{
      copyVal(input.value);
    }} else {{
      const url = await window.apichGenerateShareLink();
      if (input && url) input.value = url;
      if (url && url.startsWith("http")) copyVal(url);
    }}
  }};

  window.apichSendShareModalEmail = async function(e) {{
    e.preventDefault();
    const toEmail = (document.getElementById("apich-share-email-to").value || "").trim();
    const note = (document.getElementById("apich-share-email-note").value || "").trim();
    const commentsToggle = document.getElementById("apich-share-comments-toggle");
    const expirySelect = document.getElementById("apich-share-expiry-select");
    const submitBtn = document.getElementById("apich-share-email-submit-btn");
    const feedback = document.getElementById("apich-share-email-feedback");

    if (!toEmail) return;

    const allowComments = commentsToggle ? commentsToggle.checked : true;
    const expiryDays = expirySelect ? parseInt(expirySelect.value, 10) : 30;

    submitBtn.disabled = true;
    submitBtn.textContent = isZh ? "正在发送..." : "Sending...";
    feedback.style.display = "none";

    try {{
      const emailEndpoint = isGuestShare
        ? window.location.pathname.replace(/\/+$/, "") + "/email"
        : `/projects/${{projectId}}/files/share/email`;

      const res = await fetch(emailEndpoint, {{
        method: "POST",
        headers: {{ "Content-Type": "application/json" }},
        body: JSON.stringify({{
          file: filePath,
          file_path: filePath,
          to_email: toEmail,
          note: note || null,
          allow_comments: allowComments,
          expires_in_days: expiryDays > 0 ? expiryDays : null
        }})
      }});

      if (res.ok) {{
        feedback.style.display = "block";
        feedback.style.background = "#064e3b";
        feedback.style.color = "#6ee7b7";
        feedback.style.border = "1px solid #059669";
        feedback.textContent = isZh ? "✓ 分享链接邮件已成功发送！" : "✓ Share email sent successfully!";
        document.getElementById("apich-share-email-to").value = "";
        document.getElementById("apich-share-email-note").value = "";
      }} else {{
        const errData = await res.json().catch(() => ({{}}));
        feedback.style.display = "block";
        feedback.style.background = "#7f1d1d";
        feedback.style.color = "#fca5a5";
        feedback.style.border = "1px solid #dc2626";
        feedback.textContent = errData.error || (isZh ? "发送失败，请重试。" : "Failed to send email.");
      }}
    }} catch (err) {{
      feedback.style.display = "block";
      feedback.style.background = "#7f1d1d";
      feedback.style.color = "#fca5a5";
      feedback.textContent = String(err);
    }} finally {{
      submitBtn.disabled = false;
      submitBtn.innerHTML = "<span>✉️</span> <span>" + (isZh ? "发送邮件" : "Send Email") + "</span>";
    }}
  }};

  window.apichSetFilter = function(filter) {{
    currentFilter = filter;
    document.getElementById("apich-tab-all").classList.toggle("active", filter === "all");
    document.getElementById("apich-tab-current").classList.toggle("active", filter === "current");
    renderComments();
  }};

  async function fetchComments() {{
    try {{
      let url = `${{getBaseCommentsUrl()}}?file=${{encodeURIComponent(filePath)}}`;
      if (shareToken) url += `&token=${{encodeURIComponent(shareToken)}}`;
      const res = await fetch(url);
      if (res.ok) {{
        const data = await res.json();
        comments = data.comments || [];
        updateBadges(comments.length);
        renderComments();
      }}
    }} catch (e) {{
      console.error("Failed to load comments:", e);
    }}
  }}

  function updateBadges(count) {{
    const drawerBadge = document.getElementById("apich-drawer-badge");
    if (drawerBadge) drawerBadge.textContent = count;
    const fabBadge = document.getElementById("apich-fab-badge");
    if (fabBadge) fabBadge.textContent = count;
    const viewerBadge = document.getElementById("apich-viewer-comments-count");
    if (viewerBadge) viewerBadge.textContent = count;
  }}

  function renderComments() {{
    const container = document.getElementById("apich-comments-container");
    if (!container) return;

    let filtered = comments;
    if (currentFilter === "current") {{
      filtered = comments.filter(c => c.slide_or_page == activeSlideOrPage);
    }}

    if (filtered.length === 0) {{
      container.innerHTML = `
        <div class="apich-comments-empty">
          <span class="apich-comments-empty-icon">💭</span>
          <span style="font-weight:600; color:#cbd5e1;">${{currentFilter === "current" ? (isZh ? "当前页暂无评论" : "No comments for " + contextName + " " + activeSlideOrPage) : (isZh ? "暂无评审评论" : "No comments yet")}}</span>
          <span style="font-size:12px;">${{isZh ? "欢迎在下方发表第一条审阅意见！" : "Be the first to share review feedback below!"}}</span>
        </div>
      `;
      return;
    }}

    container.innerHTML = filtered.map(c => {{
      const initial = (c.author_name || "U").trim().charAt(0).toUpperCase();
      const tag = c.slide_or_page ? `<span class="apich-comment-tag">${{contextName}} ${{c.slide_or_page}}</span>` : "";
      const timeStr = formatRelativeTime(c.created_at);
      const delBtn = c.is_author ? `<button type="button" class="apich-comment-del" onclick="window.apichDeleteComment('${{c.id}}')">${{isZh ? "删除" : "Delete"}}</button>` : "";

      return `
        <div class="apich-comment-card" id="apich-comment-${{c.id}}">
          <div class="apich-comment-top">
            <div class="apich-comment-author-box">
              <span class="apich-comment-avatar">${{initial}}</span>
              <span class="apich-comment-author">${{escapeHtml(c.author_name)}}</span>
            </div>
            ${{tag}}
          </div>
          <div class="apich-comment-body">${{escapeHtml(c.content)}}</div>
          <div class="apich-comment-foot">
            <span>${{timeStr}}</span>
            ${{delBtn}}
          </div>
        </div>
      `;
    }}).join("");
  }}

  window.apichSubmitComment = async function(e) {{
    e.preventDefault();
    const author = (document.getElementById("apich-comment-author").value || "").trim();
    const content = (document.getElementById("apich-comment-content").value || "").trim();
    const pageVal = parseInt(document.getElementById("apich-comment-page").value, 10);
    const submitBtn = document.getElementById("apich-comment-submit-btn");
    const statusEl = document.getElementById("apich-comment-status");

    if (!author || !content) return;

    localStorage.setItem("apich_comment_author", author);
    submitBtn.disabled = true;
    if (statusEl) statusEl.textContent = isZh ? "正在提交..." : "Posting...";

    try {{
      const payload = {{
        file: filePath,
        author_name: author,
        content: content,
        slide_or_page: isNaN(pageVal) ? null : pageVal,
        token: shareToken || null,
      }};

      const res = await fetch(getBaseCommentsUrl(), {{
        method: "POST",
        headers: {{ "Content-Type": "application/json" }},
        body: JSON.stringify(payload)
      }});

      if (res.ok) {{
        const data = await res.json();
        document.getElementById("apich-comment-content").value = "";
        if (data.comment) {{
          comments.push(data.comment);
          updateBadges(comments.length);
          renderComments();
        }}
        if (statusEl) statusEl.textContent = "";
      }} else {{
        if (statusEl) statusEl.textContent = isZh ? "提交失败。" : "Failed to post comment.";
      }}
    }} catch (err) {{
      console.error(err);
      if (statusEl) statusEl.textContent = isZh ? "网络异常。" : "Network error.";
    }} finally {{
      submitBtn.disabled = false;
    }}
  }};

  window.apichDeleteComment = async function(commentId) {{
    if (!confirm(isZh ? "确定要删除此评论吗？" : "Are you sure you want to delete this comment?")) return;
    try {{
      let url = `${{getBaseCommentsUrl()}}/${{commentId}}?file=${{encodeURIComponent(filePath)}}`;
      if (shareToken) url += `&token=${{encodeURIComponent(shareToken)}}`;
      const res = await fetch(url, {{ method: "DELETE" }});
      if (res.ok) {{
        comments = comments.filter(c => c.id !== commentId);
        updateBadges(comments.length);
        renderComments();
      }}
    }} catch (e) {{
      console.error(e);
    }}
  }};

  function formatRelativeTime(dateStr) {{
    try {{
      const d = new Date(dateStr);
      const now = new Date();
      const diffSec = Math.floor((now - d) / 1000);
      if (diffSec < 60) return isZh ? "刚刚" : "Just now";
      if (diffSec < 3600) return isZh ? `${{Math.floor(diffSec / 60)}} 分钟前` : `${{Math.floor(diffSec / 60)}}m ago`;
      if (diffSec < 86400) return isZh ? `${{Math.floor(diffSec / 3600)}} 小时前` : `${{Math.floor(diffSec / 3600)}}h ago`;
      return d.toLocaleDateString();
    }} catch (e) {{
      return dateStr;
    }}
  }}

  function escapeHtml(str) {{
    return (str || "").replace(/[&<>"']/g, function(m) {{
      switch (m) {{
        case "&": return "&amp;";
        case "<": return "&lt;";
        case ">": return "&gt;";
        case '"': return "&quot;";
        case "'": return "&#039;";
        default: return m;
      }}
    }});
  }}

  function syncActiveSlide() {{
    const hash = window.location.hash;
    if (hash) {{
      const num = parseInt(hash.replace(/[^0-9]/g, ""), 10);
      if (!isNaN(num) && num > 0) {{
        activeSlideOrPage = num;
      }}
    }}
    const label = document.getElementById("apich-current-context-label");
    if (label) {{
      label.textContent = `${{contextName}} ${{activeSlideOrPage}}`;
    }}
    const pageInput = document.getElementById("apich-comment-page");
    if (pageInput) {{
      pageInput.value = activeSlideOrPage;
    }}
    if (currentFilter === "current") {{
      renderComments();
    }}
  }}

  window.addEventListener("hashchange", syncActiveSlide);
  window.addEventListener("slide_change", function(e) {{
    if (e.detail && e.detail.slide) {{
      activeSlideOrPage = e.detail.slide;
      syncActiveSlide();
    }}
  }});

  const savedAuthor = localStorage.getItem("apich_comment_author");
  if (savedAuthor) {{
    const authorInput = document.getElementById("apich-comment-author");
    if (authorInput && !authorInput.value) authorInput.value = savedAuthor;
  }}

  syncActiveSlide();
  fetchComments();
}})();
</script>
"##,
        floating_bar_html = if show_fab {
            format!(
                r#"<div class="apich-floating-bar">
  <button type="button" class="apich-float-btn" onclick="window.apichToggleShareModal()">
    <span>🔗</span>
    <span>{}</span>
  </button>
  <button type="button" class="apich-float-btn primary" onclick="window.apichToggleComments()">
    <span>💬</span>
    <span>{}</span>
    <span id="apich-fab-badge" class="apich-comments-fab-badge">0</span>
  </button>
</div>"#,
                if is_zh { "分享" } else { "Share" },
                if is_zh { "评论" } else { "Comments" },
            )
        } else {
            String::new()
        },
        comments_title = if is_zh { "评审评论" } else { "Review Comments" },
        all_comments_label = if is_zh { "全部评论" } else { "All Comments" },
        current_context_label = context_type,
        loading_label = if is_zh { "正在加载评论..." } else { "Loading comments..." },
        author_placeholder = if is_zh { "您的昵称 / 姓名" } else { "Your Name / Nickname" },
        content_placeholder = if is_zh { "输入针对当前内容的评审意见或反馈..." } else { "Type review note or feedback..." },
        post_btn_label = if is_zh { "发表评论" } else { "Post Comment" },
        share_modal_title = if is_zh { "文件分享与协作" } else { "File Sharing & Access" },
        share_tab_link_label = if is_zh { "安全分享链接" } else { "Secure Share Link" },
        share_tab_email_label = if is_zh { "发送邮件" } else { "Send via Email" },
        share_link_desc = if is_zh {
            "此临时链接经安全隔离，任何获得此链接的用户均可在浏览器中无密访问该文件并添加评审评论："
        } else {
            "Anyone with this secure link can view this file in their browser and post review comments without logging in:"
        },
        share_comments_label = if is_zh { "允许访客发表评审评论" } else { "Allow visitor review comments" },
        share_expiry_label = if is_zh { "链接有效期限" } else { "Link Expiration" },
        opt_1_day = if is_zh { "1 天 (1 Day)" } else { "1 Day" },
        opt_7_days = if is_zh { "7 天 (7 Days)" } else { "7 Days" },
        opt_30_days = if is_zh { "30 天 (30 Days)" } else { "30 Days" },
        opt_90_days = if is_zh { "90 天 (90 Days)" } else { "90 Days" },
        opt_365_days = if is_zh { "1 年 (1 Year)" } else { "1 Year" },
        opt_never = if is_zh { "永久有效 (Never Expires)" } else { "Never Expires" },
        comments_disabled_notice = if is_zh {
            "当前分享链接已由创建者禁用评论功能"
        } else {
            "Comments are disabled for this shared link by the owner"
        },
        copy_btn_label = if is_zh { "复制链接" } else { "Copy Link" },
        generate_link_btn_label = if is_zh { "生成分享链接" } else { "Generate Share Link" },
        share_link_placeholder = if is_zh { "请选择有效期限后点击“生成分享链接”" } else { "Select expiration and click 'Generate Share Link'" },
        share_token_tip = if is_zh {
            "安全保障：链接通过受控临时令牌访问，不会暴露项目内部真实路径，您可随时在项目分享管理中撤销。"
        } else {
            "Secure: Accessed via managed temporary token without exposing project routes. Can be revoked at any time."
        },
        email_recipient_label = if is_zh { "接收者邮箱地址" } else { "Recipient Email Address" },
        email_note_label = if is_zh { "附言与评审说明 (可选)" } else { "Message / Review Note (Optional)" },
        email_note_placeholder = if is_zh { "例如：请审阅第3页的数据图表与结论..." } else { "e.g. Please review slide 3 charts and summary..." },
        send_email_btn_label = if is_zh { "发送分享邮件" } else { "Send Share Email" },
    )
}

/// Render the in-browser PDF viewer page with native top bar controls and embedded comments & share drawer.
#[allow(clippy::too_many_arguments)]
pub fn render_pdf_viewer_page(
    project_id: &str,
    project_name: &str,
    file_path: &str,
    token: Option<&str>,
    is_authenticated: bool,
    current_user_name: &str,
    is_zh: bool,
    custom_raw_pdf_url: Option<&str>,
    custom_share_url: Option<&str>,
    allow_comments: bool,
    expiry_days: i64,
) -> String {
    let esc_project_id = html_escape(project_id);
    let esc_project_name = html_escape(project_name);
    let esc_file_path = html_escape(file_path);

    let comments_component = if is_authenticated {
        render_comments_drawer_component(
            project_id,
            file_path,
            token,
            current_user_name,
            false, // PDF document mode
            false, // Nav button triggers it
            custom_share_url,
            is_zh,
            allow_comments,
            expiry_days,
        )
    } else {
        String::new()
    };

    let raw_pdf_url = if let Some(custom) = custom_raw_pdf_url {
        custom.to_string()
    } else if let Some(tok) = token.filter(|t| !t.is_empty()) {
        format!(
            "/projects/{}/pdf-raw?file={}&token={}",
            esc_project_id,
            urlencoding::encode(file_path),
            urlencoding::encode(tok)
        )
    } else {
        format!(
            "/projects/{}/pdf-raw?file={}",
            esc_project_id,
            urlencoding::encode(file_path)
        )
    };

    let download_url = if raw_pdf_url.contains('?') {
        format!("{raw_pdf_url}&download=1")
    } else {
        format!("{raw_pdf_url}?download=1")
    };

    let back_btn_html = if is_authenticated {
        format!(
            r#"<a href="/projects/{}?tab=files" class="apich-nav-btn" style="text-decoration:none;">
          <span>←</span>
          <span>{}</span>
        </a>"#,
            esc_project_id,
            if is_zh { "返回项目" } else { "Back to Project" }
        )
    } else {
        String::new()
    };

    let title_prefix = if is_zh { "文档预览" } else { "Document Viewer" };
    let share_label = if is_zh { "分享" } else { "Share" };
    let download_label = if is_zh { "下载 PDF" } else { "Download PDF" };
    let comments_label = if is_zh { "评论" } else { "Comments" };

    let (share_btn_html, comments_btn_html) = if is_authenticated {
        (
            format!(
                r#"<button type="button" class="apich-nav-btn" onclick="window.apichToggleShareModal()">
        <span>🔗</span>
        <span>{share_label}</span>
      </button>"#
            ),
            format!(
                r#"<button type="button" class="apich-nav-btn primary" onclick="window.apichToggleComments()">
        <span>💬</span>
        <span>{comments_label}</span>
        <span id="apich-viewer-comments-count" class="apich-nav-badge">0</span>
      </button>"#
            ),
        )
    } else {
        (String::new(), String::new())
    };

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{esc_file_path} - {title_prefix} - APICH</title>
  <style>
    * {{
      box-sizing: border-box;
      margin: 0;
      padding: 0;
    }}
    body {{
      background: #0f172a;
      color: #f8fafc;
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
      height: 100vh;
      overflow: hidden;
      display: flex;
      flex-direction: column;
    }}
    /* Top Bar */
    .apich-viewer-nav {{
      height: 54px;
      background: #1e293b;
      border-bottom: 1px solid #334155;
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 0 16px;
      flex-shrink: 0;
      z-index: 100;
      box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
    }}
    .apich-nav-left {{
      display: flex;
      align-items: center;
      gap: 12px;
      overflow: hidden;
    }}
    .apich-brand-badge {{
      display: inline-flex;
      align-items: center;
      gap: 6px;
      font-weight: 700;
      font-size: 14px;
      color: #a5b4fc;
      background: rgba(99, 102, 241, 0.15);
      border: 1px solid rgba(99, 102, 241, 0.3);
      padding: 4px 10px;
      border-radius: 8px;
      flex-shrink: 0;
    }}
    .apich-doc-info {{
      display: flex;
      align-items: center;
      gap: 8px;
      overflow: hidden;
      white-space: nowrap;
    }}
    .apich-doc-title {{
      font-weight: 700;
      font-size: 14px;
      color: #ffffff;
      overflow: hidden;
      text-overflow: ellipsis;
    }}
    .apich-doc-project {{
      font-size: 12px;
      color: #94a3b8;
    }}
    .apich-nav-right {{
      display: flex;
      align-items: center;
      gap: 8px;
      flex-shrink: 0;
    }}
    .apich-nav-btn {{
      display: inline-flex;
      align-items: center;
      gap: 6px;
      background: #334155;
      color: #ffffff;
      border: 1px solid #475569;
      border-radius: 8px;
      padding: 6px 12px;
      font-size: 13px;
      font-weight: 600;
      cursor: pointer;
      transition: all 0.15s ease;
      user-select: none;
    }}
    .apich-nav-btn:hover {{
      background: #475569;
    }}
    .apich-nav-btn.primary {{
      background: #6366f1;
      border-color: #6366f1;
    }}
    .apich-nav-btn.primary:hover {{
      background: #4f46e5;
    }}
    .apich-nav-badge {{
      background: rgba(255, 255, 255, 0.2);
      font-size: 11px;
      padding: 1px 6px;
      border-radius: 9999px;
      font-weight: 700;
    }}

    /* Viewer Body */
    .apich-viewer-body {{
      flex: 1;
      width: 100%;
      height: calc(100vh - 54px);
      position: relative;
      background: #0f172a;
    }}
    .apich-viewer-body iframe {{
      width: 100%;
      height: 100%;
      border: none;
      display: block;
    }}
  </style>
</head>
<body>
  <!-- Top Navigation Bar -->
  <header class="apich-viewer-nav">
    <div class="apich-nav-left">
      {back_btn_html}
      <span class="apich-brand-badge">📄 PDF</span>
      <div class="apich-doc-info">
        <span class="apich-doc-title">{esc_file_path}</span>
        <span class="apich-doc-project">({esc_project_name})</span>
      </div>
    </div>

    <div class="apich-nav-right">
      {share_btn_html}

      <a href="{download_url}" class="apich-nav-btn" download style="text-decoration:none;">
        <span>⬇️</span>
        <span>{download_label}</span>
      </a>

      {comments_btn_html}
    </div>
  </header>

  <!-- PDF Viewer Body -->
  <main class="apich-viewer-body">
    <iframe src="{raw_pdf_url}" title="Document PDF Preview"></iframe>
  </main>

  {comments_component}
</body>
</html>"##
    )
}
