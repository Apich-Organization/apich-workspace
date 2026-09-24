use crate::app::components::ActiveNav;
use crate::app::components::AiDrawer;
use crate::app::components::AppShell;
use crate::app::components::FileShareModal;
use crate::app::pages::knowledge_page::render_calendar;
use crate::app::pages::knowledge_page::render_kanban;
use crate::app::pages::knowledge_page::render_wiki;
use crate::services::knowledge_sync::CalendarEvent;
use crate::services::knowledge_sync::KanbanBoard;
use crate::services::knowledge_sync::KnowledgeGraph;
use crate::services::knowledge_sync::NoteHeading;
use crate::services::knowledge_sync::UnifiedNoteMeta;
use crate::services::FileShareInfo;
use crate::ui::i18n::I18n;
use apich_db::Project;
use apich_db::User;
use apich_islands::AttachModalIsland;
use apich_islands::WhiteboardIsland;
use leptos::prelude::*;

/// Unified note space view selector.
///
/// plan.md calls for one integrated space that seamlessly aggregates personal notes, a
/// bidirectional-link wiki, a whiteboard, a calendar, and a task kanban ("一体化空间：无缝聚合
/// 个人笔记、双链 Wiki、白板、日历日程与任务看板") -- not five separate pages. Wiki/Calendar/
/// Kanban used to live on a standalone `/projects/:id/knowledge` page that nothing in the UI
/// actually linked to (a real, orphaned-page bug, not just an organizational preference); now
/// they're tabs here, alongside the note editor and whiteboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteView {
    Editor,
    Whiteboard,
    Wiki,
    Calendar,
    Kanban,
}

impl NoteView {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            | "whiteboard" => Self::Whiteboard,
            | "wiki" => Self::Wiki,
            | "calendar" => Self::Calendar,
            | "kanban" => Self::Kanban,
            | _ => Self::Editor,
        }
    }

    pub fn from_string(s: &str) -> Self {
        Self::from_str(s)
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
    own_kanban_templates: Vec<apich_db::Template>,
    visible_kanban_templates: Vec<apich_db::TemplateWithLatestVersion>,
    own_note_templates: Vec<apich_db::Template>,
    visible_note_templates: Vec<apich_db::TemplateWithLatestVersion>,
    file_share: Option<FileShareInfo>,
    all_users: Vec<User>,
    active_view: NoteView,
    notice: Option<String>,
    error: Option<String>,
    i18n: I18n,
    current_path: String,
) -> impl IntoView {
    let Project {
        id: project_id,
        owner_id: project_owner_id,
        storage_path: _storage_path,
        ..
    } = project;
    let view = active_view;

    let alert = notice.map_or_else(
        || {
            error.map(|e| {
                view! { <div class="alert alert-danger" style="margin-bottom:1rem;">{e}</div> }
                    .into_any()
            })
        },
        |n| {
            Some(
                view! { <div class="alert alert-success" style="margin-bottom:1rem;">{n}</div> }
                    .into_any(),
            )
        },
    );

    let (share_label, share_mode, share_role, share_users) = match file_share {
        | Some(s) => {
            let label = match s.mode.as_str() {
                | "public" => format!("🌐 {}", i18n.share_public(&s.role)),
                | "specific" => format!("👥 {}", i18n.share_specific(&s.role)),
                | _ => format!("🔒 {}", i18n.share_private()),
            };
            let users = s.allowed_users.join(",");
            (label, s.mode, s.role, users)
        },
        | None => {
            (
                format!("🔒 {}", i18n.share_private()),
                "private".to_string(),
                "read".to_string(),
                String::new(),
            )
        },
    };
    let share_detail = serde_json::json!({ "path": file_path, "mode": share_mode, "role": share_role, "users": share_users }).to_string();
    let share_onclick = format!(
        "window.dispatchEvent(new CustomEvent('apich-open-share-modal', {{detail: {share_detail}}}))"
    );

    let file_path_enc = urlencoding::encode(&file_path).to_string();
    let sub_nav = view! {
        <div style="display:flex; gap:0.4rem; align-items:center; flex-wrap:wrap; margin-bottom:1rem;">
            <a href=format!("/projects/{}/note?file={}&view=editor", project_id, file_path_enc) class="btn btn-sm" class=("btn-primary", view == NoteView::Editor) class=("btn-secondary", view != NoteView::Editor)>
                {if i18n.is_zh() { "📝 编辑与大纲" } else { "📝 Editor & Outline" }}
            </a>
            <a href=format!("/projects/{}/note?file={}&view=whiteboard", project_id, file_path_enc) class="btn btn-sm" class=("btn-primary", view == NoteView::Whiteboard) class=("btn-secondary", view != NoteView::Whiteboard)>
                {if i18n.is_zh() { "🎨 白板" } else { "🎨 Whiteboard" }}
            </a>
            <a href=format!("/projects/{}/note?file={}&view=wiki", project_id, file_path_enc) class="btn btn-sm" class=("btn-primary", view == NoteView::Wiki) class=("btn-secondary", view != NoteView::Wiki)>
                {if i18n.is_zh() { "🕸️ 知识库 Wiki" } else { "🕸️ Wiki" }}
            </a>
            <a href=format!("/projects/{}/note?file={}&view=calendar", project_id, file_path_enc) class="btn btn-sm" class=("btn-primary", view == NoteView::Calendar) class=("btn-secondary", view != NoteView::Calendar)>
                {if i18n.is_zh() { "📅 日历" } else { "📅 Calendar" }}
            </a>
            <a href=format!("/projects/{}/note?file={}&view=kanban", project_id, file_path_enc) class="btn btn-sm" class=("btn-primary", view == NoteView::Kanban) class=("btn-secondary", view != NoteView::Kanban)>
                {if i18n.is_zh() { "📋 看板" } else { "📋 Kanban" }}
            </a>
            <button
                type="button"
                class="btn btn-secondary btn-sm"
                onclick="window.dispatchEvent(new CustomEvent('apich-open-attach-modal', {detail: {mode: 'upload'}}))"
                title=if i18n.is_zh() { "上传本地文件或图片至项目" } else { "Upload local file or image to project" }
            >
                "📤 " {i18n.upload()}
            </button>
            <button
                type="button"
                class="btn btn-secondary btn-sm"
                onclick="window.dispatchEvent(new CustomEvent('apich-open-attach-modal', {detail: {mode: 'attach'}}))"
                title=if i18n.is_zh() { "插入项目文件或图片引用" } else { "Insert reference to project file or image" }
            >
                "📎 " {i18n.attach()}
            </button>
            <button type="button" class="btn btn-secondary btn-sm" onclick=share_onclick>{format!("🔗 {share_label}")}</button>
            <label for="ai-drawer-toggle-cb" class="btn btn-secondary btn-sm">"🤖 " {i18n.ai_copilot()}</label>
            <a href=format!("/projects/{}?tab=vcs", project_id) class="btn btn-secondary btn-sm">"🌿 " {i18n.history()}</a>
            {if project.settings.get("is_single_file").and_then(|v| v.as_bool()).unwrap_or(false) {
                let cf_file = file_path.clone();
                let confirm_msg = i18n.delete_file_confirm(&cf_file);
                view! {
                    <form
                                method="post"
                                action=format!("/projects/{}/delete", project_id)
                                class="inline-form"
                                onsubmit=format!("return confirm('{}');", confirm_msg.replace('\'', "\\'"))
                            >
                                <button
                                    type="submit"
                                    class="btn btn-danger btn-sm"
                                    title=i18n.delete_file()
                                >
                                    "🗑️ " {i18n.delete()}
                                </button>
                            </form>
                    <a href="/" class="btn btn-outline btn-sm" style="margin-left:0.5rem;">"← " {i18n.back_to_dashboard()}</a>
                }.into_any()
            } else {
                view! { <a href=format!("/projects/{}?tab=files", project_id) class="btn btn-outline btn-sm" style="margin-left:0.5rem;">"📁 " {i18n.back_to_files()}</a> }.into_any()
            }}
        </div>
    };

    let content = match view {
        | NoteView::Editor => {
            render_editor_view(
                project_id,
                &file_path,
                body_content,
                &meta,
                headings,
                backlinks,
                rendered_markdown_html,
                task_count,
                completed_task_count,
                i18n.is_zh(),
                own_note_templates,
                visible_note_templates,
                i18n,
            )
            .into_any()
        },
        | NoteView::Whiteboard => render_whiteboard_view(project_id, &file_path, &meta).into_any(),
        | NoteView::Wiki => render_wiki(project_id, &graph, i18n).into_any(),
        | NoteView::Calendar => render_calendar(&calendar, i18n).into_any(),
        | NoteView::Kanban => {
            render_kanban(
                project_id,
                &kanban,
                own_kanban_templates,
                visible_kanban_templates,
                i18n,
            )
            .into_any()
        },
    };
    drop((graph, kanban, calendar));

    view! {
        <AppShell
            user=user
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
                owner_id=project_owner_id
                i18n=i18n
            />
            <AttachModalIsland
                project_id=project_id.to_string()
                active_file=file_path.clone()
                is_zh=i18n.is_zh()
            />
            <AiDrawer project_id=project_id file_path=file_path is_zh=i18n.is_zh() />
        </AppShell>
    }
}

#[allow(clippy::too_many_arguments)]
fn render_editor_view(
    project_id: uuid::Uuid,
    file_path: &str,
    body_content: String,
    meta: &UnifiedNoteMeta,
    headings: Vec<NoteHeading>,
    backlinks: Vec<String>,
    rendered_html: String,
    task_count: usize,
    completed_task_count: usize,
    is_zh: bool,
    own_note_templates: Vec<apich_db::Template>,
    visible_note_templates: Vec<apich_db::TemplateWithLatestVersion>,
    i18n: I18n,
) -> impl IntoView {
    let tags_joined = meta.tags.join(", ");
    let progress_pct = completed_task_count
        .saturating_mul(100)
        .checked_div(task_count)
        .unwrap_or(0);

    let editor_headings: Vec<apich_islands::NoteHeadingItem> = headings
        .into_iter()
        .map(|h| {
            apich_islands::NoteHeadingItem {
                level: h.level as u8,
                text: h.text,
                line: h.line as u32,
            }
        })
        .collect();
    let task_progress_label = (task_count > 0).then(|| {
        if is_zh {
            format!("任务进度：{completed_task_count}/{task_count}（{progress_pct}%）")
        } else {
            format!("Task Progress: {completed_task_count}/{task_count} ({progress_pct}%)")
        }
    });

    view! {
        <form method="post" action=format!("/projects/{}/note/save", project_id) id="note-save-form">
            <input type="hidden" name="file" value=file_path.to_string() />
            <input type="hidden" name="view" value="editor" />
            <div style="display:grid; grid-template-columns: 2fr 1fr 1fr; gap:0.75rem; margin-bottom:0.85rem;">
                <div class="form-group" style="margin-bottom:0;">
                    <label style="font-size:0.75rem;">{if is_zh { "标题" } else { "Title" }}</label>
                    <input type="text" name="meta_title" value=meta.title.clone() class="form-control" style="font-size:0.85rem; height:32px;" />
                </div>
                <div class="form-group" style="margin-bottom:0;">
                    <label style="font-size:0.75rem;">{if is_zh { "作者" } else { "Author" }}</label>
                    <input type="text" name="meta_author" value=meta.author.clone().unwrap_or_default() class="form-control" style="font-size:0.85rem; height:32px;" />
                </div>
                <div class="form-group" style="margin-bottom:0;">
                    <label style="font-size:0.75rem;">{if is_zh { "标签" } else { "Tags" }}</label>
                    <input type="text" name="meta_tags" value=tags_joined class="form-control" style="font-size:0.85rem; height:32px;" />
                </div>
            </div>

            <div class="editor-studio-grid" style="height:600px; border:1px solid var(--border-subtle); border-radius:12px; overflow:hidden;">
                <apich_islands::NoteEditorIsland
                    project_id=project_id.to_string()
                    file_path=file_path.to_string()
                    body_content=body_content
                    headings=editor_headings
                    backlinks=backlinks
                    rendered_html=rendered_html
                    task_progress_label=task_progress_label
                    is_zh=is_zh
                />
            </div>

            <div style="margin-top:1rem; display:flex; justify-content:flex-end;">
                <button type="submit" class="btn btn-primary">{if is_zh { "保存笔记" } else { "Save Note" }}</button>
            </div>
        </form>
        {render_note_template_panel(project_id, file_path, own_note_templates, visible_note_templates, i18n)}
    }
}

/// "Publish as Template" / "Apply a Template" for the note editor -- shares the same
/// create-new-vs-publish-new-version and browse-and-apply shape the Kanban board's own panel
/// uses (`render_kanban_template_panel`), just posting to the note-flavored routes
/// (`ui::template_handlers`'s `*_note_*` actions) and needing a destination file name to apply
/// into, since a note template creates a whole new file rather than updating settings in place.
fn render_note_template_panel(
    project_id: uuid::Uuid,
    file_path: &str,
    own_templates: Vec<apich_db::Template>,
    visible_templates: Vec<apich_db::TemplateWithLatestVersion>,
    i18n: I18n,
) -> impl IntoView {
    let publish_existing = (!own_templates.is_empty()).then(|| {
        let rows: Vec<_> = own_templates
            .into_iter()
            .map(|t| {
                view! {
                    <form method="post" action=format!("/templates/{}/publish-version-note", t.id) class="inline-form" style="display:flex; gap:0.4rem; margin-top:0.3rem; align-items:center;">
                        <input type="hidden" name="project_id" value=project_id.to_string() />
                        <input type="hidden" name="file" value=file_path.to_string() />
                        <span style="font-size:0.8rem; min-width:120px;">{t.name}</span>
                        <input type="text" name="version_label" placeholder=i18n.template_version_label_field() class="form-control" style="height:30px; font-size:0.8rem; max-width:140px;" required=true />
                        <input type="text" name="changelog" placeholder=i18n.template_changelog_field() class="form-control" style="height:30px; font-size:0.8rem; max-width:220px;" />
                        <button type="submit" class="btn btn-secondary btn-sm">{i18n.template_publish()}</button>
                    </form>
                }
            })
            .collect();
        view! {
            <div style="margin-top:0.5rem;">
                <span style="font-size:0.8rem; color:var(--text-sub);">{i18n.template_publish_new_version_to()}</span>
                {rows}
            </div>
        }
    });

    let apply_options: Vec<_> = visible_templates
        .into_iter()
        .filter_map(|t| t.latest_version_id.map(|vid| (t, vid)))
        .map(|(t, vid)| view! { <option value=vid.to_string()>{t.name}" ("{t.latest_version_label.unwrap_or_default()}")"</option> })
        .collect();
    let apply_panel = (!apply_options.is_empty()).then(|| {
        view! {
            <form method="post" action=format!("/projects/{}/note/apply-template", project_id) class="inline-form" style="display:flex; gap:0.5rem; margin-top:0.6rem; align-items:center; flex-wrap:wrap;">
                <select name="template_version_id" class="form-control" style="min-width:320px; max-width:520px; width:auto; font-size:0.875rem;" required=true>
                    <option value="" disabled=true selected=true>{i18n.template_select_version()}</option>
                    {apply_options}
                </select>
                <input type="text" name="new_file_name" placeholder=i18n.template_new_file_name_field() class="form-control" style="min-width:180px; font-size:0.875rem;" required=true />
                <button type="submit" class="btn btn-primary btn-sm">{i18n.template_apply()}</button>
            </form>
        }
    });

    view! {
        <details style="margin-top:1rem; background:var(--bg-muted); border-radius:10px; padding:0.85rem 1rem;">
            <summary style="cursor:pointer; font-weight:600; font-size:0.85rem;">{i18n.template_publish_as_template()}" / "{i18n.template_apply_template()}</summary>
            <div style="margin-top:0.6rem;">
                <form method="post" action="/templates/create-from-note" class="inline-form" style="display:flex; gap:0.5rem; flex-wrap:wrap; align-items:center;">
                    <input type="hidden" name="project_id" value=project_id.to_string() />
                    <input type="hidden" name="file" value=file_path.to_string() />
                    <input type="text" name="name" placeholder=i18n.template_name_field() class="form-control" style="min-width:160px; font-size:0.875rem;" required=true />
                    <input type="text" name="slug" placeholder=i18n.template_slug_field() class="form-control" style="min-width:160px; font-size:0.875rem;" required=true />
                    <input type="text" name="description" placeholder=i18n.template_description_field() class="form-control" style="min-width:200px; font-size:0.875rem;" />
                    <select name="visibility" class="form-control" style="min-width:140px; font-size:0.875rem;">
                        <option value="private">{i18n.template_visibility_label("private")}</option>
                        <option value="shared">{i18n.template_visibility_label("shared")}</option>
                        <option value="public">{i18n.template_visibility_label("public")}</option>
                    </select>
                    <input type="text" name="version_label" placeholder=i18n.template_version_label_field() class="form-control" style="min-width:120px; font-size:0.875rem;" required=true value="1.0.0" />
                    <button type="submit" class="btn btn-primary btn-sm">{i18n.template_create_new()}</button>
                </form>
                {publish_existing}
                {apply_panel.is_none().then(|| view! { <p class="text-muted" style="font-size:0.78rem; margin-top:0.5rem;">{i18n.template_no_templates()}</p> })}
                {apply_panel}
                <div style="margin-top:0.4rem;"><a href="/templates?kind=note" style="font-size:0.78rem;">{i18n.template_gallery_title()}" →"</a></div>
            </div>
        </details>
    }
}

fn render_whiteboard_view(
    project_id: uuid::Uuid,
    file_path: &str,
    meta: &UnifiedNoteMeta,
) -> impl IntoView {
    let initial_strokes = meta
        .whiteboard
        .clone()
        .unwrap_or_else(|| serde_json::json!({ "strokes": [] }));
    let strokes_json =
        serde_json::to_string(&initial_strokes).unwrap_or_else(|_| "{\"strokes\":[]}".to_string());

    view! {
        <WhiteboardIsland
            project_id=project_id.to_string()
            file_path=file_path.to_string()
            initial_strokes_json=strokes_json
        />
    }
}
