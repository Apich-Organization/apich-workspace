//! Kanban/Wiki/Calendar render helpers, shared by the unified note page
//! (`app::pages::note_page::NotePage`). These used to back a standalone `/projects/:id/knowledge`
//! page, but nothing in the UI ever linked to it -- a real orphaned-page bug, not a deliberate
//! design. plan.md calls for one integrated space ("一体化空间") aggregating notes, wiki,
//! whiteboard, calendar, and kanban, so these are tabs on the note page now; the old page route
//! redirects there for any bookmarked links (see `ui::handlers::project_knowledge_page`).

use crate::services::knowledge_sync::{CalendarEvent, KanbanBoard, KnowledgeGraph};
use apich_db::Project;
use leptos::prelude::*;

pub(crate) fn render_kanban(project: &Project, kanban: &KanbanBoard) -> impl IntoView {
    let project_id = project.id;
    let percent = if kanban.total_tasks > 0 { kanban.completed_tasks * 100 / kanban.total_tasks } else { 0 };

    let cols: Vec<_> = kanban
        .columns
        .iter()
        .map(|col| {
            let cards: Vec<_> = if col.tasks.is_empty() {
                vec![view! { <div style="font-size:0.8rem; color:var(--text-sub); text-align:center; padding:1.5rem 0;">"No tasks"</div> }.into_any()]
            } else {
                col.tasks
                    .iter()
                    .map(|t| {
                        let next_status = match t.status.as_str() {
                            "todo" | "in_progress" => "done",
                            _ => "todo",
                        };
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
                                        <input type="hidden" name="view" value="kanban" />
                                        <button type="submit" class="btn btn-ghost btn-sm" style="padding:0 4px; font-size:1.1rem; line-height:1;" title="Toggle task">{check_char}</button>
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

    view! {
        <div style="margin-bottom:1.25rem;">
            <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:0.4rem; font-size:0.85rem; color:var(--text-muted);">
                <span>"Tasks Completion Rate: "<strong>{kanban.completed_tasks}"/"{kanban.total_tasks}</strong>" completed (" {percent} "%)"</span>
            </div>
            <div style="height:6px; background:var(--bg-muted); border-radius:9999px; overflow:hidden;">
                <div style=format!("height:100%; width:{}%; background:linear-gradient(90deg, #2563eb, #16a34a); border-radius:9999px;", percent)></div>
            </div>
        </div>
        <div class="kanban-grid">{cols}</div>
    }
}

pub(crate) fn render_wiki(graph: &KnowledgeGraph) -> impl IntoView {
    let nodes: Vec<_> = graph
        .nodes
        .iter()
        .map(|n| {
            let status_tag = if n.exists {
                view! { <span style="font-size:0.7rem; color:var(--text-sub);">{n.backlink_count} " backlinks • " {n.task_count} " tasks"</span> }.into_any()
            } else {
                view! { <span style="font-size:0.7rem; color:#d97706; background:#fffbeb; padding:1px 5px; border-radius:3px;">"Placeholder / Uncreated"</span> }.into_any()
            };
            let path_info = n.file_path.clone().map(|p| view! { <code style="font-size:0.7rem;">{p}</code> });
            view! {
                <div class="wiki-node-item">
                    <div>
                        <span class="wiki-link-title">"[[" {n.label.clone()} "]]"</span>
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

    view! {
        <div class="wiki-container">
            <div class="wiki-card">
                <h3 style="font-size:1rem; font-weight:700; margin-bottom:0.85rem; color:var(--text-main);">"Notes & Concepts (" {graph.nodes.len()} " nodes)"</h3>
                <div class="wiki-node-list">{nodes}</div>
            </div>
            <div class="wiki-card">
                <h3 style="font-size:1rem; font-weight:700; margin-bottom:0.85rem; color:var(--text-main);">"Bidirectional Wiki Graph Links (" {graph.edges.len()} " connections)"</h3>
                <ul style="padding-left:0;">{edges}</ul>
            </div>
        </div>
    }
}

pub(crate) fn render_calendar(events: &[CalendarEvent]) -> impl IntoView {
    if events.is_empty() {
        return view! {
            <div class="empty-state"><p>"No scheduled tasks or dated files found. Add @YYYY-MM-DD to any task in your Markdown notes to see it on the calendar."</p></div>
        }.into_any();
    }

    let mut days: Vec<(String, Vec<&CalendarEvent>)> = Vec::new();
    for ev in events {
        match days.last_mut() {
            Some((date, list)) if *date == ev.date => list.push(ev),
            _ => days.push((ev.date.clone(), vec![ev])),
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
