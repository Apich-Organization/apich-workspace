//! Kanban/Wiki/Calendar render helpers, shared by the unified note page
//! (`app::pages::note_page::NotePage`). These used to back a standalone `/projects/:id/knowledge`
//! page, but nothing in the UI ever linked to it -- a real orphaned-page bug, not a deliberate
//! design. plan.md calls for one integrated space ("一体化空间") aggregating notes, wiki,
//! whiteboard, calendar, and kanban, so these are tabs on the note page now; the old page route
//! redirects there for any bookmarked links (see `ui::handlers::project_knowledge_page`).

use crate::services::knowledge_sync::CalendarEvent;
use crate::services::knowledge_sync::KanbanBoard;
use crate::services::knowledge_sync::KnowledgeGraph;
use crate::services::knowledge_sync::KANBAN_UNSORTED_COLUMN_ID;
use crate::ui::i18n::I18n;
use apich_db::Project;
use leptos::prelude::*;

pub fn render_kanban(
    project: &Project,
    kanban: &KanbanBoard,
    own_kanban_templates: &[apich_db::Template],
    visible_kanban_templates: &[apich_db::TemplateWithLatestVersion],
    i18n: I18n,
) -> impl IntoView {
    let project_id = project.id;
    let percent = if kanban.total_tasks > 0 {
        kanban.completed_tasks * 100 / kanban.total_tasks
    } else {
        0
    };

    // Cycling only ever walks the board's own *real* configured columns (never lands a card back
    // in the synthetic Unsorted bucket) -- a card sitting in Unsorted, when clicked, "claims" it
    // into the first real column instead.
    let real_columns: Vec<_> = kanban
        .columns
        .iter()
        .filter(|c| c.id != KANBAN_UNSORTED_COLUMN_ID)
        .collect();

    let cols: Vec<_> = kanban
        .columns
        .iter()
        .map(|col| {
            let cards: Vec<_> = if col.tasks.is_empty() {
                vec![view! { <div style="font-size:0.8rem; color:var(--text-sub); text-align:center; padding:1.5rem 0;">{i18n.kanban_no_tasks()}</div> }.into_any()]
            } else {
                col.tasks
                    .iter()
                    .map(|t| {
                        // Cycles through the project's own configured columns in order (see
                        // `real_columns` above) -- this used to be a hardcoded 3-state cycle that
                        // jumped straight from "todo" to "done" on a single click, skipping
                        // "in_progress" entirely, because both non-done states mapped to the same
                        // next step.
                        let cur_idx = real_columns.iter().position(|c| c.id == t.status);
                        let next_idx = match cur_idx {
                            Some(i) if !real_columns.is_empty() => (i + 1) % real_columns.len(),
                            _ => 0,
                        };
                        let next_col = real_columns.get(next_idx);
                        let next_status = next_col.map_or_else(|| "todo".to_string(), |c| c.id.clone());
                        let next_is_done = next_col.is_some_and(|c| c.is_done);
                        let check_char = if t.completed { "☑" } else { "☐" };
                        let title_style = if t.completed { "text-decoration:line-through; color:var(--text-sub);" } else { "" };
                        let tags: Vec<_> = t.tags.iter().map(|tag| view! { <span class="tag-badge">"#" {tag.clone()}</span> }).collect();
                        let date_badge = t.due_date.clone().map(|d| view! { <span class="date-badge">"📅 " {d}</span> });
                        view! {
                            <div class="kanban-card">
                                <div class="kanban-card-title">
                                    <form method="post" action=format!("/projects/{}/knowledge/toggle-task", project_id) class="inline-form">
                                        <input type="hidden" name="file" value=t.file_path.clone() />
                                        <input type="hidden" name="line_number" value=t.line_number.to_string() />
                                        <input type="hidden" name="status" value=next_status />
                                        <input type="hidden" name="done" value=if next_is_done { "true" } else { "false" } />
                                        <input type="hidden" name="view" value="kanban" />
                                        <button type="submit" class="btn btn-ghost btn-sm" style="padding:0 4px; font-size:1.1rem; line-height:1;" title=i18n.kanban_toggle_task()>{check_char}</button>
                                    </form>
                                    <span style=title_style>{t.title.clone()}</span>
                                </div>
                                <div class="kanban-meta">
                                    {tags}
                                    {date_badge}
                                    <span class="file-link-tag">"📄 " {t.file_path.clone()} ":" {t.line_number}</span>
                                </div>
                            </div>
                        }
                        .into_any()
                    })
                    .collect()
            };
            view! {
                <div class="kanban-col">
                    <div class="kanban-col-head">
                        <span>{col.title.clone()}</span>
                        <span class="kanban-count">{col.tasks.len()}</span>
                    </div>
                    {cards}
                </div>
            }
        })
        .collect();

    let customize_panel = render_kanban_column_settings(project_id, &real_columns, i18n);
    let template_panel = render_kanban_template_panel(
        project_id,
        own_kanban_templates,
        visible_kanban_templates,
        i18n,
    );

    view! {
        <div style="margin-bottom:1.25rem;">
            <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:0.4rem; font-size:0.85rem; color:var(--text-muted);">
                <span>{i18n.kanban_completion_rate()}" "<strong>{kanban.completed_tasks}"/"{kanban.total_tasks}</strong>" (" {percent} "%)"</span>
            </div>
            <div style="height:6px; background:var(--bg-muted); border-radius:9999px; overflow:hidden;">
                <div style=format!("height:100%; width:{}%; background:linear-gradient(90deg, #2563eb, #16a34a); border-radius:9999px;", percent)></div>
            </div>
        </div>
        {customize_panel}
        {template_panel}
        <div class="kanban-grid">{cols}</div>
    }
}

/// The "self-define able" half of the Kanban feedback: add/rename/reorder/delete columns as a
/// quick, per-project setting (stored in `project.settings["kanban_columns"]`, see
/// `KnowledgeSyncService::parse_kanban_columns`) -- independent of the separate template-library
/// publish/apply flow that shares the same underlying column data.
fn render_kanban_column_settings(
    project_id: uuid::Uuid,
    columns: &[&crate::services::knowledge_sync::KanbanColumn],
    i18n: I18n,
) -> impl IntoView {
    let base = format!("/projects/{project_id}/knowledge/kanban/columns");
    let rows: Vec<_> = columns
        .iter()
        .enumerate()
        .map(|(idx, col)| {
            let is_first = idx == 0;
            let is_last = idx == columns.len() - 1;
            let only_column = columns.len() == 1;
            view! {
                <div style="display:flex; align-items:center; gap:0.4rem; padding:0.4rem 0; border-bottom:1px solid var(--border-subtle);">
                    <form method="post" action=format!("{base}/rename") class="inline-form" style="display:flex; align-items:center; gap:0.4rem; flex:1;">
                        <input type="hidden" name="col_id" value=col.id.clone() />
                        <input type="text" name="title" value=col.title.clone() class="form-control" style="font-size:0.82rem; height:30px; max-width:220px;" required=true />
                        <label style="display:flex; align-items:center; gap:0.25rem; font-size:0.72rem; color:var(--text-sub); white-space:nowrap;">
                            <input type="checkbox" name="is_done" value="true" checked=col.is_done />
                            {i18n.kanban_mark_as_done_column()}
                        </label>
                        <button type="submit" class="btn btn-secondary btn-sm">{i18n.kanban_rename()}</button>
                    </form>
                    <form method="post" action=format!("{base}/move") class="inline-form">
                        <input type="hidden" name="col_id" value=col.id.clone() />
                        <input type="hidden" name="direction" value="left" />
                        <button type="submit" class="btn btn-ghost btn-sm" disabled=is_first title=i18n.kanban_move_left()>"←"</button>
                    </form>
                    <form method="post" action=format!("{base}/move") class="inline-form">
                        <input type="hidden" name="col_id" value=col.id.clone() />
                        <input type="hidden" name="direction" value="right" />
                        <button type="submit" class="btn btn-ghost btn-sm" disabled=is_last title=i18n.kanban_move_right()>"→"</button>
                    </form>
                    <form method="post" action=format!("{base}/delete") class="inline-form">
                        <input type="hidden" name="col_id" value=col.id.clone() />
                        <button type="submit" class="btn btn-ghost btn-sm" style="color:var(--danger);" disabled=only_column title=i18n.kanban_delete_column()>"🗑"</button>
                    </form>
                </div>
            }
        })
        .collect();

    view! {
        <details style="margin-bottom:1.25rem; background:var(--bg-muted); border-radius:10px; padding:0.85rem 1rem;">
            <summary style="cursor:pointer; font-weight:600; font-size:0.85rem;">"⚙ "{i18n.kanban_customize_columns()}</summary>
            <div style="margin-top:0.6rem;">
                {rows}
                <form method="post" action=format!("{base}/add") class="inline-form" style="display:flex; gap:0.4rem; margin-top:0.6rem;">
                    <input type="text" name="title" placeholder=i18n.kanban_column_name_placeholder() class="form-control" style="font-size:0.82rem; height:32px; max-width:220px;" required=true />
                    <button type="submit" class="btn btn-primary btn-sm">"+ "{i18n.kanban_add_column()}</button>
                </form>
            </div>
        </details>
    }
}

/// "Publish as Template" (this board's current column layout, as a new template or a new version
/// of one of the user's own) / "Apply a Template" (pull a chosen version's columns onto this
/// board, overwriting the current layout the same way "Customize columns" edits do). Content is
/// always the columns this board *actually has right now* -- read server-side from
/// `project.settings`, never something the form itself carries, so what gets published can't
/// drift from what's really on screen.
fn render_kanban_template_panel(
    project_id: uuid::Uuid,
    own_templates: &[apich_db::Template],
    visible_templates: &[apich_db::TemplateWithLatestVersion],
    i18n: I18n,
) -> impl IntoView {
    let publish_existing = (!own_templates.is_empty()).then(|| {
        let rows: Vec<_> = own_templates
            .iter()
            .map(|t| {
                view! {
                    <form method="post" action=format!("/templates/{}/publish-version-kanban", t.id) class="inline-form" style="display:flex; gap:0.4rem; margin-top:0.3rem; align-items:center;">
                        <input type="hidden" name="project_id" value=project_id.to_string() />
                        <span style="font-size:0.8rem; min-width:120px;">{t.name.clone()}</span>
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
        .iter()
        .filter_map(|t| t.latest_version_id.map(|vid| (t, vid)))
        .map(|(t, vid)| view! { <option value=vid.to_string()>{t.name.clone()}" ("{t.latest_version_label.clone().unwrap_or_default()}")"</option> })
        .collect();
    let apply_panel = (!apply_options.is_empty()).then(|| {
        view! {
            <form method="post" action=format!("/projects/{}/knowledge/kanban/apply-template", project_id) class="inline-form" style="display:flex; gap:0.4rem; margin-top:0.5rem; align-items:center;">
                <select name="template_version_id" class="form-control" style="height:32px; font-size:0.82rem; max-width:260px;" required=true>
                    <option value="" disabled=true selected=true>{i18n.template_select_version()}</option>
                    {apply_options}
                </select>
                <button type="submit" class="btn btn-primary btn-sm">{i18n.template_apply()}</button>
            </form>
        }
    });

    view! {
        <details style="margin-bottom:1.25rem; background:var(--bg-muted); border-radius:10px; padding:0.85rem 1rem;">
            <summary style="cursor:pointer; font-weight:600; font-size:0.85rem;">{i18n.template_publish_as_template()}" / "{i18n.template_apply_template()}</summary>
            <div style="margin-top:0.6rem;">
                <form method="post" action="/templates/create-from-kanban" class="inline-form" style="display:flex; gap:0.4rem; flex-wrap:wrap; align-items:center;">
                    <input type="hidden" name="project_id" value=project_id.to_string() />
                    <input type="text" name="name" placeholder=i18n.template_name_field() class="form-control" style="height:32px; font-size:0.82rem; max-width:160px;" required=true />
                    <input type="text" name="slug" placeholder=i18n.template_slug_field() class="form-control" style="height:32px; font-size:0.82rem; max-width:160px;" required=true />
                    <input type="text" name="description" placeholder=i18n.template_description_field() class="form-control" style="height:32px; font-size:0.82rem; max-width:200px;" />
                    <select name="visibility" class="form-control" style="height:32px; font-size:0.82rem;">
                        <option value="private">{i18n.template_visibility_label("private")}</option>
                        <option value="shared">{i18n.template_visibility_label("shared")}</option>
                        <option value="public">{i18n.template_visibility_label("public")}</option>
                    </select>
                    <input type="text" name="version_label" placeholder=i18n.template_version_label_field() class="form-control" style="height:32px; font-size:0.82rem; max-width:140px;" required=true value="1.0.0" />
                    <button type="submit" class="btn btn-primary btn-sm">{i18n.template_create_new()}</button>
                </form>
                {publish_existing}
                {apply_panel.is_none().then(|| view! { <p class="text-muted" style="font-size:0.78rem; margin-top:0.5rem;">{i18n.template_no_templates()}</p> })}
                {apply_panel}
                <div style="margin-top:0.4rem;"><a href="/templates?kind=kanban" style="font-size:0.78rem;">{i18n.template_gallery_title()}" →"</a></div>
            </div>
        </details>
    }
}

pub fn render_wiki(
    project_id: uuid::Uuid,
    graph: &KnowledgeGraph,
) -> impl IntoView {
    let nodes: Vec<_> = graph
        .nodes
        .iter()
        .map(|n| {
            let status_tag = if n.exists {
                view! { <span style="font-size:0.7rem; color:var(--text-sub);">{n.backlink_count} " backlinks • " {n.task_count} " tasks"</span> }.into_any()
            } else {
                let create_action = format!("/projects/{project_id}/note/create-page");
                view! {
                    <div style="display:flex; align-items:center; gap:0.5rem;">
                        <span style="font-size:0.7rem; color:#d97706; background:#fffbeb; padding:1px 5px; border-radius:3px;">"Placeholder / Uncreated"</span>
                        <form method="post" action=create_action class="inline-form">
                            <input type="hidden" name="title" value=n.label.clone() />
                            <button type="submit" class="btn btn-primary btn-sm" style="padding:0.15rem 0.5rem; font-size:0.72rem;">"+ Create this page"</button>
                        </form>
                    </div>
                }.into_any()
            };
            let title_view = if let Some(path) = &n.file_path {
                let href = format!("/projects/{}/note?file={}&view=editor", project_id, urlencoding::encode(path));
                view! { <a href=href class="wiki-link-title" style="text-decoration:none;">"[[" {n.label.clone()} "]]"</a> }.into_any()
            } else {
                view! { <span class="wiki-link-title">"[[" {n.label.clone()} "]]"</span> }.into_any()
            };
            let path_info = n.file_path.clone().map(|p| view! { <code style="font-size:0.7rem;">{p}</code> });
            view! {
                <div class="wiki-node-item">
                    <div>
                        {title_view}
                        <div style="margin-top:2px;">{path_info}</div>
                    </div>
                    <div>{status_tag}</div>
                </div>
            }
        })
        .collect();

    let edges: Vec<_> = if graph.edges.is_empty() {
        vec![view! { <li style="list-style:none;">"No [[Wiki Links]] found yet. Link notes using [[Note Name]] in your markdown documents."</li> }.into_any()]
    } else {
        graph
            .edges
            .iter()
            .map(|e| {
                let label = e.label.clone().map(|l| view! { <span style="font-size:0.7rem; color:var(--text-sub);">" (" {l} ")"</span> });
                view! {
                    <li style="font-size:0.825rem; margin-bottom:0.4rem; list-style:none;">
                        <code>"[[" {e.source.clone()} "]]"</code> " ➔ " <code>"[[" {e.target.clone()} "]]"</code>
                        {label}
                    </li>
                }
                .into_any()
            })
            .collect()
    };

    let new_page_form = {
        let action = format!("/projects/{project_id}/note/create-page");
        view! {
            <form method="post" action=action class="form-row" style="align-items:flex-end; margin-bottom:1rem;">
                <div class="form-group" style="margin-bottom:0; flex-grow:1;">
                    <label style="font-size:0.75rem;">"New page title"</label>
                    <input type="text" name="title" placeholder="e.g. Project Roadmap" required=true class="form-control" />
                </div>
                <input type="hidden" name="view" value="wiki" />
                <button type="submit" class="btn btn-primary btn-sm">"+ New Page"</button>
            </form>
        }
    };

    view! {
        <div class="wiki-container">
            <div class="wiki-card">
                <h3 style="font-size:1rem; font-weight:700; margin-bottom:0.85rem; color:var(--text-main);">"Notes & Concepts (" {graph.nodes.len()} " nodes)"</h3>
                {new_page_form}
                <div class="wiki-node-list">{nodes}</div>
            </div>
            <div class="wiki-card">
                <h3 style="font-size:1rem; font-weight:700; margin-bottom:0.85rem; color:var(--text-main);">"Bidirectional Wiki Graph Links (" {graph.edges.len()} " connections)"</h3>
                <ul style="padding-left:0;">{edges}</ul>
            </div>
        </div>
    }
}

pub fn render_calendar(events: &[CalendarEvent]) -> impl IntoView {
    if events.is_empty() {
        return view! {
            <div class="empty-state"><p>"No scheduled tasks or dated files found. Add @YYYY-MM-DD to any task in your Markdown notes to see it on the calendar."</p></div>
        }.into_any();
    }

    let mut days: Vec<(String, Vec<&CalendarEvent>)> = Vec::new();
    for ev in events {
        match days.last_mut() {
            | Some((date, list)) if *date == ev.date => list.push(ev),
            | _ => days.push((ev.date.clone(), vec![ev])),
        }
    }

    let day_cards: Vec<_> = days
        .into_iter()
        .map(|(date, evs)| {
            let items: Vec<_> = evs
                .into_iter()
                .map(|ev| {
                    let icon = if ev.is_task { if ev.completed { "✅" } else { "⬜" } } else { "📄" };
                    let badge = if ev.is_task {
                        if ev.completed {
                            view! { <span class="status-badge badge-active" style="font-size:0.7rem;">"Completed"</span> }.into_any()
                        } else {
                            view! { <span class="status-badge badge-idle" style="font-size:0.7rem;">"To Do"</span> }.into_any()
                        }
                    } else {
                        view! { <span class="status-badge" style="font-size:0.7rem; background:var(--bg-muted); color:var(--text-sub);">"Note"</span> }.into_any()
                    };
                    view! {
                        <div class="calendar-event-item">
                            <div style="display:flex; align-items:center; gap:0.5rem;">
                                <span>{icon}</span>
                                <span style="font-weight:600;">{ev.title.clone()}</span>
                                <span style="font-size:0.725rem; color:var(--text-sub); font-family:var(--font-mono);">{ev.source_file.clone()}</span>
                            </div>
                            <div>{badge}</div>
                        </div>
                    }
                })
                .collect();
            view! {
                <div class="calendar-day-card">
                    <div class="calendar-date-header">"📅 " {date}</div>
                    {items}
                </div>
            }
        })
        .collect();

    view! { <div class="calendar-wrap">{day_cards}</div> }.into_any()
}
