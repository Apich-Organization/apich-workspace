use crate::app::components::{ActiveNav, AiDrawer, AppShell, FileShareModal};
use crate::app::pages::knowledge_page::{render_calendar, render_kanban, render_wiki};
use crate::services::knowledge_sync::{CalendarEvent, KanbanBoard, KnowledgeGraph, NoteHeading, UnifiedNoteMeta};
use crate::services::FileShareInfo;
use crate::ui::i18n::I18n;
use apich_db::{Project, User};
use apich_islands::WhiteboardIsland;
use leptos::prelude::*;

/// plan.md calls for one integrated space that seamlessly aggregates personal notes, a
/// bidirectional-link wiki, a whiteboard, a calendar, and a task kanban ("一体化空间：无缝聚合
/// 个人笔记、双链 Wiki、白板、日历日程与任务看板") -- not five separate pages. Wiki/Calendar/
/// Kanban used to live on a standalone `/projects/:id/knowledge` page that nothing in the UI
/// actually linked to (a real, orphaned-page bug, not just an organizational preference); now
/// they're tabs here, alongside the note editor and whiteboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NoteView {
    Editor,
    Whiteboard,
    Wiki,
    Calendar,
    Kanban,
}

impl NoteView {
    fn from_str(s: &str) -> Self {
        match s {
            "whiteboard" => NoteView::Whiteboard,
            "wiki" => NoteView::Wiki,
            "calendar" => NoteView::Calendar,
            "kanban" => NoteView::Kanban,
            _ => NoteView::Editor,
        }
    }
}

#[allow(clippy::too_many_arguments)]
#[component]
pub fn NotePage(
    user: User,
    is_org_or_team_admin: bool,
    project: Project,
    file_path: String,
    body_content: String,
    meta: UnifiedNoteMeta,
    headings: Vec<NoteHeading>,
    backlinks: Vec<String>,
    rendered_markdown_html: String,
    task_count: usize,
    completed_task_count: usize,
    graph: KnowledgeGraph,
    kanban: KanbanBoard,
    calendar: Vec<CalendarEvent>,
    file_share: Option<FileShareInfo>,
    all_users: Vec<User>,
    active_view: String,
    notice: Option<String>,
    error: Option<String>,
    i18n: I18n,
    current_path: String,
) -> impl IntoView {
    let project_id = project.id;
    let view = NoteView::from_str(&active_view);

    let alert = if let Some(n) = notice {
        Some(view! { <div class="alert alert-success" style="margin-bottom:1rem;">{n}</div> }.into_any())
    } else {
        error.map(|e| view! { <div class="alert alert-danger" style="margin-bottom:1rem;">{e}</div> }.into_any())
    };

    let share_label = match file_share.as_ref() {
        Some(s) if s.mode == "public" => format!("🌐 Public ({})", s.role),
        Some(s) if s.mode == "specific" => format!("👥 Specific ({})", s.role),
        _ => "🔒 Private".to_string(),
    };
    let share_mode = file_share.as_ref().map(|s| s.mode.clone()).unwrap_or_else(|| "private".to_string());
    let share_role = file_share.as_ref().map(|s| s.role.clone()).unwrap_or_else(|| "read".to_string());
    let share_users = file_share.as_ref().map(|s| s.allowed_users.join(",")).unwrap_or_default();
    let share_detail = serde_json::json!({ "path": file_path, "mode": share_mode, "role": share_role, "users": share_users }).to_string();
    let share_onclick = format!("window.dispatchEvent(new CustomEvent('apich-open-share-modal', {{detail: {}}}))", share_detail);

    let file_path_enc = urlencoding::encode(&file_path).to_string();
    let sub_nav = view! {
        <div style="display:flex; gap:0.4rem; align-items:center; flex-wrap:wrap; margin-bottom:1rem;">
            <a href=format!("/projects/{}/note?file={}&view=editor", project_id, file_path_enc) class="btn btn-sm" class=("btn-primary", view == NoteView::Editor) class=("btn-secondary", view != NoteView::Editor)>
                "📝 Editor & Outline"
            </a>
            <a href=format!("/projects/{}/note?file={}&view=whiteboard", project_id, file_path_enc) class="btn btn-sm" class=("btn-primary", view == NoteView::Whiteboard) class=("btn-secondary", view != NoteView::Whiteboard)>
                "🎨 Whiteboard"
            </a>
            <a href=format!("/projects/{}/note?file={}&view=wiki", project_id, file_path_enc) class="btn btn-sm" class=("btn-primary", view == NoteView::Wiki) class=("btn-secondary", view != NoteView::Wiki)>
                "🕸️ Wiki"
            </a>
            <a href=format!("/projects/{}/note?file={}&view=calendar", project_id, file_path_enc) class="btn btn-sm" class=("btn-primary", view == NoteView::Calendar) class=("btn-secondary", view != NoteView::Calendar)>
                "📅 Calendar"
            </a>
            <a href=format!("/projects/{}/note?file={}&view=kanban", project_id, file_path_enc) class="btn btn-sm" class=("btn-primary", view == NoteView::Kanban) class=("btn-secondary", view != NoteView::Kanban)>
                "📋 Kanban"
            </a>
            <button type="button" class="btn btn-secondary btn-sm" onclick=share_onclick>{format!("🔗 {}", share_label)}</button>
            <label for="ai-drawer-toggle-cb" class="btn btn-secondary btn-sm">"🤖 AI Copilot"</label>
            <a href=format!("/projects/{}?tab=files", project_id) class="btn btn-outline btn-sm" style="margin-left:0.5rem;">"📁 Back to Files"</a>
        </div>
    };

    let content = match view {
        NoteView::Editor => render_editor_view(&project, &file_path, &body_content, &meta, &headings, &backlinks, &rendered_markdown_html, task_count, completed_task_count).into_any(),
        NoteView::Whiteboard => render_whiteboard_view(project_id, &file_path, &meta).into_any(),
        NoteView::Wiki => render_wiki(&graph).into_any(),
        NoteView::Calendar => render_calendar(&calendar).into_any(),
        NoteView::Kanban => render_kanban(&project, &kanban).into_any(),
    };

    view! {
        <AppShell
            user=user.clone()
            is_org_or_team_admin=is_org_or_team_admin
            active_nav=ActiveNav::Projects
            current_path=current_path
            page_title=meta.title.clone()
            i18n=i18n
        >
            <div class="page-header">
                <div>
                    <h1 class="page-title">{meta.title.clone()}</h1>
                    <p class="page-subtitle">{file_path.clone()}</p>
                </div>
            </div>
            {alert}
            {sub_nav}
            {content}
            <FileShareModal
                project_id=project_id
                redirect_to=format!("/projects/{}/note?file={}", project_id, file_path_enc)
                all_users=all_users
                owner_id=project.owner_id
                i18n=i18n
            />
            <AiDrawer project_id=project_id file_path=file_path />
        </AppShell>
    }
}

fn render_editor_view(
    project: &Project,
    file_path: &str,
    body_content: &str,
    meta: &UnifiedNoteMeta,
    headings: &[NoteHeading],
    backlinks: &[String],
    rendered_html: &str,
    task_count: usize,
    completed_task_count: usize,
) -> impl IntoView {
    let project_id = project.id;
    let tags_joined = meta.tags.join(", ");
    let progress_pct = if task_count > 0 { completed_task_count * 100 / task_count } else { 0 };

    let editor_headings: Vec<apich_islands::NoteHeadingItem> = headings
        .iter()
        .map(|h| apich_islands::NoteHeadingItem { level: h.level as u8, text: h.text.clone(), line: h.line as u32 })
        .collect();
    let task_progress_label = (task_count > 0).then(|| format!("Task Progress: {}/{} ({}%)", completed_task_count, task_count, progress_pct));

    view! {
        <form method="post" action=format!("/projects/{}/note/save", project_id) id="note-save-form">
            <input type="hidden" name="file" value=file_path.to_string() />
            <input type="hidden" name="view" value="editor" />
            <div style="display:grid; grid-template-columns: 2fr 1fr 1fr; gap:0.75rem; margin-bottom:0.85rem;">
                <div class="form-group" style="margin-bottom:0;">
                    <label style="font-size:0.75rem;">"Title"</label>
                    <input type="text" name="meta_title" value=meta.title.clone() class="form-control" style="font-size:0.85rem; height:32px;" />
                </div>
                <div class="form-group" style="margin-bottom:0;">
                    <label style="font-size:0.75rem;">"Author"</label>
                    <input type="text" name="meta_author" value=meta.author.clone().unwrap_or_default() class="form-control" style="font-size:0.85rem; height:32px;" />
                </div>
                <div class="form-group" style="margin-bottom:0;">
                    <label style="font-size:0.75rem;">"Tags"</label>
                    <input type="text" name="meta_tags" value=tags_joined class="form-control" style="font-size:0.85rem; height:32px;" />
                </div>
            </div>

            <div class="editor-studio-grid" style="height:600px; border:1px solid var(--border-subtle); border-radius:12px; overflow:hidden;">
                <apich_islands::NoteEditorIsland
                    project_id=project_id.to_string()
                    file_path=file_path.to_string()
                    body_content=body_content.to_string()
                    headings=editor_headings
                    backlinks=backlinks.to_vec()
                    rendered_html=rendered_html.to_string()
                    task_progress_label=task_progress_label
                />
            </div>

            <div style="margin-top:1rem; display:flex; justify-content:flex-end;">
                <button type="submit" class="btn btn-primary">"Save Note"</button>
            </div>
        </form>
    }
}

fn render_whiteboard_view(project_id: uuid::Uuid, file_path: &str, meta: &UnifiedNoteMeta) -> impl IntoView {
    let initial_strokes = meta
        .whiteboard
        .as_ref()
        .cloned()
        .unwrap_or_else(|| serde_json::json!({ "strokes": [] }));
    let strokes_json = serde_json::to_string(&initial_strokes).unwrap_or_else(|_| "{\"strokes\":[]}".to_string());

    view! {
        <WhiteboardIsland
            project_id=project_id.to_string()
            file_path=file_path.to_string()
            initial_strokes_json=strokes_json
        />
    }
}
