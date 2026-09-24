//! Comments drawer and in-browser PDF viewer UI components.

use crate::ui::handlers::html_escape;

/// Render the comments drawer HTML, CSS, and JS engine.
///
/// Can be embedded in the cargo-slide presentation shell or in the PDF viewer.
pub fn render_comments_drawer_component(
    project_id: &str,
    file_path: &str,
    token: Option<&str>,
    current_user_name: &str,
    is_slide: bool,
    show_fab: bool,
) -> String {
    let esc_project_id = html_escape(project_id);
    let esc_file_path = html_escape(file_path);
    let esc_token = html_escape(token.unwrap_or(""));
    let esc_user = html_escape(current_user_name);
    let context_type = if is_slide { "Slide" } else { "Page" };

    format!(
        r##"
<!-- APICH Comments Drawer Component -->
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

  /* Floating Action Button (Slide player) */
  .apich-comments-fab {{
    position: fixed;
    bottom: 24px;
    right: 24px;
    z-index: 99998;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    background: rgba(30, 41, 59, 0.88);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    color: #ffffff;
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 9999px;
    padding: 10px 18px;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.4), 0 0 0 1px rgba(255, 255, 255, 0.05);
    transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
    user-select: none;
  }}
  .apich-comments-fab:hover {{
    background: rgba(49, 65, 88, 0.95);
    transform: translateY(-2px);
    box-shadow: 0 14px 28px -4px rgba(0, 0, 0, 0.5);
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

  /* Drawer Overlay & Sliding Panel */
  .apich-drawer-overlay {{
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background: rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(4px);
    z-index: 99999;
    opacity: 0;
    visibility: hidden;
    transition: opacity 0.25s ease, visibility 0.25s ease;
  }}
  .apich-drawer-overlay.open {{
    opacity: 1;
    visibility: visible;
  }}
  .apich-drawer-panel {{
    position: fixed;
    top: 0;
    right: 0;
    width: min(440px, 94vw);
    height: 100vh;
    background: #0f172a;
    border-left: 1px solid #1e293b;
    box-shadow: -10px 0 35px rgba(0, 0, 0, 0.5);
    display: flex;
    flex-direction: column;
    z-index: 100000;
    transform: translateX(100%);
    transition: transform 0.3s cubic-bezier(0.16, 1, 0.3, 1);
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    color: #f8fafc;
  }}
  .apich-drawer-overlay.open .apich-drawer-panel {{
    transform: translateX(0);
  }}

  /* Header */
  .apich-drawer-header {{
    padding: 18px 20px;
    background: #1e293b;
    border-bottom: 1px solid #334155;
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-shrink: 0;
  }}
  .apich-drawer-title-wrap {{
    display: flex;
    align-items: center;
    gap: 10px;
  }}
  .apich-drawer-title {{
    font-size: 16px;
    font-weight: 700;
    margin: 0;
    display: flex;
    align-items: center;
    gap: 6px;
  }}
  .apich-drawer-count {{
    background: #312e81;
    color: #a5b4fc;
    font-size: 12px;
    font-weight: 700;
    padding: 2px 8px;
    border-radius: 12px;
  }}
  .apich-drawer-close {{
    background: none;
    border: none;
    color: #94a3b8;
    font-size: 20px;
    line-height: 1;
    cursor: pointer;
    padding: 6px 10px;
    border-radius: 6px;
    transition: background 0.15s ease, color 0.15s ease;
  }}
  .apich-drawer-close:hover {{
    background: #334155;
    color: #ffffff;
  }}

  /* Filter Tabs */
  .apich-drawer-tabs {{
    display: flex;
    background: #1e293b;
    padding: 6px 16px;
    gap: 8px;
    border-bottom: 1px solid #334155;
    flex-shrink: 0;
  }}
  .apich-drawer-tab {{
    background: none;
    border: none;
    color: #94a3b8;
    font-size: 13px;
    font-weight: 600;
    padding: 6px 12px;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s ease;
  }}
  .apich-drawer-tab.active {{
    background: #334155;
    color: #ffffff;
  }}

  /* Comments Scroll Area */
  .apich-comments-list {{
    flex: 1;
    overflow-y: auto;
    padding: 16px 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }}
  .apich-comment-card {{
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 10px;
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    transition: border-color 0.15s ease;
  }}
  .apich-comment-card:hover {{
    border-color: #475569;
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
    width: 26px;
    height: 26px;
    border-radius: 50%;
    background: #6366f1;
    color: #fff;
    font-size: 12px;
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }}
  .apich-comment-author {{
    font-weight: 600;
    font-size: 13px;
    color: #f1f5f9;
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
    padding: 10px 12px;
    font-size: 13px;
    font-family: inherit;
    outline: none;
    resize: vertical;
    min-height: 70px;
    max-height: 160px;
  }}
  .apich-form-textarea:focus {{
    border-color: #6366f1;
  }}
  .apich-form-btn {{
    background: #6366f1;
    color: #ffffff;
    border: none;
    border-radius: 6px;
    padding: 9px 16px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    transition: background 0.15s ease;
  }}
  .apich-form-btn:hover {{
    background: #4f46e5;
  }}
  .apich-form-btn:disabled {{
    opacity: 0.6;
    cursor: not-allowed;
  }}
</style>

{fab_html}

<div id="apich-drawer-overlay" class="apich-drawer-overlay" onclick="window.apichCloseComments(event)">
  <div class="apich-drawer-panel" onclick="event.stopPropagation()">
    <!-- Drawer Header -->
    <div class="apich-drawer-header">
      <div class="apich-drawer-title-wrap">
        <h3 class="apich-drawer-title">
          <span>💬</span>
          <span id="apich-drawer-title-text">{title_text}</span>
        </h3>
        <span id="apich-drawer-badge" class="apich-drawer-count">0</span>
      </div>
      <button type="button" class="apich-drawer-close" onclick="window.apichCloseComments()" title="Close">&times;</button>
    </div>

    <!-- Filter Tabs -->
    <div class="apich-drawer-tabs">
      <button type="button" id="apich-tab-all" class="apich-drawer-tab active" onclick="window.apichSetCommentFilter('all')">
        All ({all_label})
      </button>
      <button type="button" id="apich-tab-current" class="apich-drawer-tab" onclick="window.apichSetCommentFilter('current')">
        Current {context_type} (<span id="apich-tab-current-num">1</span>)
      </button>
    </div>

    <!-- Comments List -->
    <div id="apich-comments-container" class="apich-comments-list">
      <div class="apich-comments-empty">
        <span class="apich-comments-empty-icon">💭</span>
        <span>Loading comments...</span>
      </div>
    </div>

    <!-- Input Form -->
    <form id="apich-comment-form" class="apich-drawer-form" onsubmit="window.apichSubmitComment(event)">
      <div class="apich-form-row">
        <input
          type="text"
          id="apich-comment-author"
          class="apich-form-input"
          style="flex: 1;"
          placeholder="Your name or nickname"
          value="{esc_user}"
          required
        />
        <div style="display: flex; align-items: center; gap: 4px; font-size: 12px; color: #94a3b8;">
          <span>{context_type}:</span>
          <input
            type="number"
            id="apich-comment-page"
            class="apich-form-input"
            style="width: 60px; text-align: center; padding: 6px 4px;"
            min="1"
            value="1"
          />
        </div>
      </div>
      <textarea
        id="apich-comment-content"
        class="apich-form-textarea"
        placeholder="Add your comment, note, or review suggestion here..."
        required
      ></textarea>
      <div style="display: flex; justify-content: flex-end; align-items: center; gap: 8px;">
        <span id="apich-comment-status" style="font-size: 12px; color: #94a3b8;"></span>
        <button type="submit" id="apich-comment-submit-btn" class="apich-form-btn">
          <span>Post Comment</span>
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
  const isSlide = {is_slide_bool};
  const contextName = isSlide ? "Slide" : "Page";

  let comments = [];
  let currentFilter = "all";
  let activeSlideOrPage = 1;

  // Restore guest author name from local storage if available
  const storedAuthor = localStorage.getItem("apich_comment_author");
  const authorInput = document.getElementById("apich-comment-author");
  if (storedAuthor && authorInput && !authorInput.value) {{
    authorInput.value = storedAuthor;
  }}

  // Read current slide from window hash (e.g. #3) or DOM
  function detectActiveIndex() {{
    const hash = window.location.hash.replace(/[^0-9]/g, "");
    if (hash) {{
      const n = parseInt(hash, 10);
      if (!isNaN(n) && n > 0) return n;
    }}
    return activeSlideOrPage || 1;
  }}

  function updateActiveContext(n) {{
    activeSlideOrPage = n || detectActiveIndex();
    const curNumEl = document.getElementById("apich-tab-current-num");
    if (curNumEl) curNumEl.textContent = activeSlideOrPage;
    const pageInput = document.getElementById("apich-comment-page");
    if (pageInput) pageInput.value = activeSlideOrPage;
  }}

  window.addEventListener("hashchange", () => {{
    updateActiveContext();
    if (currentFilter === "current") renderComments();
  }});

  // Global methods
  window.apichToggleComments = function() {{
    const overlay = document.getElementById("apich-drawer-overlay");
    if (overlay.classList.contains("open")) {{
      window.apichCloseComments();
    }} else {{
      window.apichOpenComments();
    }}
  }};

  window.apichOpenComments = function() {{
    updateActiveContext();
    const overlay = document.getElementById("apich-drawer-overlay");
    overlay.classList.add("open");
    fetchComments();
  }};

  window.apichCloseComments = function() {{
    const overlay = document.getElementById("apich-drawer-overlay");
    overlay.classList.remove("open");
  }};

  window.apichSetCommentFilter = function(filter) {{
    currentFilter = filter;
    document.getElementById("apich-tab-all").classList.toggle("active", filter === "all");
    document.getElementById("apich-tab-current").classList.toggle("active", filter === "current");
    renderComments();
  }};

  async function fetchComments() {{
    try {{
      let url = `/api/projects/${{projectId}}/files/comments?file=${{encodeURIComponent(filePath)}}`;
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
          <span style="font-weight:600; color:#cbd5e1;">${{currentFilter === "current" ? "No comments for " + contextName + " " + activeSlideOrPage : "No comments yet"}}</span>
          <span style="font-size:12px;">Be the first to share feedback or review notes below!</span>
        </div>
      `;
      return;
    }}

    container.innerHTML = filtered.map(c => {{
      const initial = (c.author_name || "U").trim().charAt(0).toUpperCase();
      const tag = c.slide_or_page ? `<span class="apich-comment-tag">${{contextName}} ${{c.slide_or_page}}</span>` : "";
      const timeStr = formatRelativeTime(c.created_at);
      const delBtn = c.is_author ? `<button type="button" class="apich-comment-del" onclick="window.apichDeleteComment('${{c.id}}')">Delete</button>` : "";

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
    if (statusEl) statusEl.textContent = "Posting...";

    try {{
      const payload = {{
        file: filePath,
        author_name: author,
        content: content,
        slide_or_page: isNaN(pageVal) ? null : pageVal,
        token: shareToken || null,
      }};

      const res = await fetch(`/api/projects/${{projectId}}/files/comments`, {{
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
        if (statusEl) statusEl.textContent = "Failed to post comment.";
      }}
    }} catch (err) {{
      console.error(err);
      if (statusEl) statusEl.textContent = "Network error.";
    }} finally {{
      submitBtn.disabled = false;
    }}
  }};

  window.apichDeleteComment = async function(commentId) {{
    if (!confirm("Are you sure you want to delete this comment?")) return;
    try {{
      let url = `/api/projects/${{projectId}}/files/comments/${{commentId}}?file=${{encodeURIComponent(filePath)}}`;
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
      if (diffSec < 60) return "Just now";
      if (diffSec < 3600) return `${{Math.floor(diffSec / 60)}}m ago`;
      if (diffSec < 86400) return `${{Math.floor(diffSec / 3600)}}h ago`;
      return d.toLocaleDateString();
    }} catch (e) {{
      return dateStr;
    }}
  }}

  function escapeHtml(str) {{
    const div = document.createElement("div");
    div.textContent = str || "";
    return div.innerHTML;
  }}

  // Initial load
  updateActiveContext();
  fetchComments();
}})();
</script>
<!-- End APICH Comments Drawer Component -->
"##,
        fab_html = if show_fab {
            format!(
                r#"<button type="button" id="apich-comments-fab" class="apich-comments-fab" onclick="window.apichToggleComments()" title="View and add comments">
  <span>💬</span>
  <span>Comments</span>
  <span id="apich-fab-badge" class="apich-comments-fab-badge">0</span>
</button>"#
            )
        } else {
            String::new()
        },
        title_text = if is_slide { "Slide Comments" } else { "Document Comments" },
        all_label = "All",
        is_slide_bool = if is_slide { "true" } else { "false" },
    )
}

/// Render the complete standalone in-browser PDF viewer HTML.
pub fn render_pdf_viewer_page(
    project_id: &str,
    project_name: &str,
    file_path: &str,
    token: Option<&str>,
    is_authenticated: bool,
    current_user_name: &str,
    is_zh: bool,
) -> String {
    let esc_project_id = html_escape(project_id);
    let esc_project_name = html_escape(project_name);
    let esc_file_path = html_escape(file_path);
    let _esc_token = html_escape(token.unwrap_or(""));

    let comments_component = render_comments_drawer_component(
        project_id,
        file_path,
        token,
        current_user_name,
        false, // PDF document mode
        false, // Show comments button in top nav instead of floating fab
    );

    let raw_pdf_url = if let Some(tok) = token.filter(|t| !t.is_empty()) {
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

    let download_url = format!("{raw_pdf_url}&download=1");

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

    /* Toast Notification */
    .apich-toast {{
      position: fixed;
      bottom: 24px;
      left: 50%;
      transform: translateX(-50%) translateY(100px);
      background: #22c55e;
      color: #ffffff;
      padding: 10px 20px;
      border-radius: 8px;
      font-size: 14px;
      font-weight: 600;
      box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.4);
      transition: transform 0.25s cubic-bezier(0.16, 1, 0.3, 1);
      z-index: 100001;
      display: flex;
      align-items: center;
      gap: 6px;
    }}
    .apich-toast.show {{
      transform: translateX(-50%) translateY(0);
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
      <button type="button" class="apich-nav-btn" onclick="window.apichCopyShareLink()">
        <span>🔗</span>
        <span>{share_label}</span>
      </button>

      <a href="{download_url}" class="apich-nav-btn" download style="text-decoration:none;">
        <span>⬇️</span>
        <span>{download_label}</span>
      </a>

      <button type="button" class="apich-nav-btn primary" onclick="window.apichToggleComments()">
        <span>💬</span>
        <span>{comments_label}</span>
        <span id="apich-viewer-comments-count" class="apich-nav-badge">0</span>
      </button>
    </div>
  </header>

  <!-- PDF Viewer Body -->
  <main class="apich-viewer-body">
    <iframe src="{raw_pdf_url}" title="Document PDF Preview"></iframe>
  </main>

  <div id="apich-toast" class="apich-toast">
    <span>✓</span>
    <span id="apich-toast-text">{copied_label}</span>
  </div>

  <script>
    window.apichCopyShareLink = function() {{
      const url = window.location.href;
      navigator.clipboard.writeText(url).then(() => {{
        const toast = document.getElementById("apich-toast");
        toast.classList.add("show");
        setTimeout(() => toast.classList.remove("show"), 2500);
      }}).catch(err => {{
        prompt("Copy this share link:", url);
      }});
    }};
  </script>

  {comments_component}
</body>
</html>
"##,
        share_label = if is_zh { "分享链接" } else { "Share Link" },
        download_label = if is_zh { "下载 PDF" } else { "Download PDF" },
        comments_label = if is_zh { "评论与评审" } else { "Comments" },
        copied_label = if is_zh { "链接已复制到剪贴板！" } else { "Share link copied to clipboard!" },
    )
}
