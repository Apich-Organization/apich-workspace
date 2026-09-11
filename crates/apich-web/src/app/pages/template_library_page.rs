//! Template Library pages: a public-ish gallery (`TemplateGalleryPage`) of publishable, versioned
//! templates across all 5 content kinds (note/latex/typst/slides/kanban -- see
//! `apich_db::TemplateKind`), and a per-template detail/version/preview/sharing page
//! (`TemplateDetailPage`). Actually *publishing* a version happens contextually from the
//! originating content page instead (the kanban board's own "Publish as Template" panel, the
//! note editor's own, etc.) -- see `ui::template_handlers` -- since a version's content is always
//! "whatever this specific project/board/note currently looks like", which only that page has.

use crate::services::template_library;
use crate::ui::i18n::I18n;
use apich_db::{Template, TemplateVersion, User};
use leptos::prelude::*;
use uuid::Uuid;

/// A resolved, human-readable label for one `apich_db::TemplateShare` row -- the DB only stores
/// `org_id`/`team_id`, so the handler resolves the actual org/team name before handing this to
/// the page (a Leptos view has no async DB access of its own).
pub struct TemplateShareDisplay {
    pub id: Uuid,
    pub label: String,
}

#[allow(clippy::too_many_arguments)]
#[component]
pub fn TemplateGalleryPage(
    user: User,
    is_org_or_team_admin: bool,
    templates: Vec<apich_db::TemplateWithLatestVersion>,
    kind_filter: Option<String>,
    notice: Option<String>,
    error: Option<String>,
    i18n: I18n,
    current_path: String,
) -> impl IntoView {
    use crate::app::components::{ActiveNav, AppShell};

    let alert = if let Some(n) = notice {
        Some(view! { <div class="alert alert-success" style="margin-bottom:1rem;">{n}</div> }.into_any())
    } else {
        error.map(|e| view! { <div class="alert alert-danger" style="margin-bottom:1rem;">{e}</div> }.into_any())
    };

    let kinds = ["", "kanban", "note", "latex", "typst", "slides"];
    let tabs: Vec<_> = kinds
        .iter()
        .map(|k| {
            let is_active = kind_filter.as_deref().unwrap_or("") == *k;
            let href = if k.is_empty() { "/templates".to_string() } else { format!("/templates?kind={k}") };
            view! {
                <a href=href class="tab-item" class:active=is_active>{i18n.template_kind_label(k)}</a>
            }
        })
        .collect();

    let rows: Vec<_> = templates
        .into_iter()
        .map(|t| {
            let detail_href = format!("/templates/{}", t.id);
            view! {
                <a href=detail_href class="template-card" style="display:block; padding:1rem 1.25rem; border:1px solid var(--border-subtle); border-radius:10px; margin-bottom:0.75rem; text-decoration:none; color:inherit;">
                    <div style="display:flex; justify-content:space-between; align-items:center;">
                        <div>
                            <span class=format!("pill-{}", t.kind)>{i18n.template_kind_label(&t.kind)}</span>
                            <strong style="margin-left:0.5rem;">{t.name}</strong>
                        </div>
                        <span class="status-badge badge-idle">{i18n.template_visibility_label(&t.visibility)}</span>
                    </div>
                    {t.description.map(|d| view! { <p class="text-muted" style="font-size:0.85rem; margin:0.4rem 0 0;">{d}</p> })}
                    <div style="font-size:0.75rem; color:var(--text-sub); margin-top:0.5rem;">
                        {i18n.template_owner()}": "{t.owner_username}
                        {t.latest_version_label.map(|v| view! { "  •  "{i18n.template_latest_version()}": "{v} }.into_any())}
                        "  •  "{t.version_count}" "{i18n.template_versions_heading()}
                    </div>
                </a>
            }
        })
        .collect();

    let empty_state = rows.is_empty().then(|| view! { <div class="empty-state"><p>{i18n.template_no_templates()}</p></div> });

    view! {
        <AppShell user=user.clone() is_org_or_team_admin=is_org_or_team_admin active_nav=ActiveNav::Templates current_path=current_path page_title=i18n.template_gallery_title().to_string() i18n=i18n>
            <div class="page-header">
                <h1 class="page-title">{i18n.template_gallery_title()}</h1>
            </div>
            {alert}
            <div class="tab-bar" style="margin-bottom:1rem;">{tabs}</div>
            {rows}
            {empty_state}
        </AppShell>
    }
}

#[allow(clippy::too_many_arguments)]
#[component]
pub fn TemplateDetailPage(
    user: User,
    is_org_or_team_admin: bool,
    template: Template,
    versions: Vec<TemplateVersion>,
    shares: Vec<TemplateShareDisplay>,
    is_owner: bool,
    notice: Option<String>,
    error: Option<String>,
    i18n: I18n,
    current_path: String,
) -> impl IntoView {
    use crate::app::components::{ActiveNav, AppShell};

    let alert = if let Some(n) = notice {
        Some(view! { <div class="alert alert-success" style="margin-bottom:1rem;">{n}</div> }.into_any())
    } else {
        error.map(|e| view! { <div class="alert alert-danger" style="margin-bottom:1rem;">{e}</div> }.into_any())
    };

    let template_id = template.id;
    let kind = template.kind.clone();

    let management_panel = is_owner.then(|| {
        let share_rows: Vec<_> = shares
            .into_iter()
            .map(|s| {
                view! {
                    <div style="display:flex; justify-content:space-between; align-items:center; padding:0.3rem 0;">
                        <span style="font-size:0.85rem;">{s.label}</span>
                        <form method="post" action=format!("/templates/{}/share/{}/delete", template_id, s.id) class="inline-form">
                            <button type="submit" class="btn btn-ghost btn-sm" style="color:var(--danger);">{i18n.template_remove()}</button>
                        </form>
                    </div>
                }
            })
            .collect();

        view! {
            <div class="section-card" style="margin-bottom:1.25rem;">
                <h3 class="card-subtitle">{i18n.template_sharing_heading()}</h3>
                <div style="display:flex; gap:0.5rem; margin:0.5rem 0;">
                    <form method="post" action=format!("/templates/{}/visibility", template_id) class="inline-form" style="display:flex; gap:0.4rem;">
                        <select name="visibility" class="form-control" style="height:32px; font-size:0.82rem;">
                            <option value="private" selected=template.visibility == "private">{i18n.template_visibility_label("private")}</option>
                            <option value="shared" selected=template.visibility == "shared">{i18n.template_visibility_label("shared")}</option>
                            <option value="public" selected=template.visibility == "public">{i18n.template_visibility_label("public")}</option>
                        </select>
                        <button type="submit" class="btn btn-secondary btn-sm">{i18n.template_change_visibility()}</button>
                    </form>
                </div>
                {share_rows}
                <form method="post" action=format!("/templates/{}/share", template_id) class="inline-form" style="display:flex; gap:0.4rem; margin-top:0.5rem; flex-wrap:wrap;">
                    <input type="text" name="org_slug" placeholder=i18n.template_share_with_org_field() class="form-control" style="height:32px; font-size:0.82rem; max-width:200px;" required=true />
                    <input type="text" name="team_slug" placeholder=i18n.template_share_with_team_field() class="form-control" style="height:32px; font-size:0.82rem; max-width:220px;" />
                    <button type="submit" class="btn btn-secondary btn-sm">{i18n.template_add_share()}</button>
                </form>
                <form method="post" action=format!("/templates/{}/delete", template_id) style="margin-top:0.75rem;">
                    <apich_islands::ConfirmSubmitButton
                        label=i18n.template_delete().to_string()
                        message="Delete this template and all its versions? This can't be undone.".to_string()
                        button_class="btn btn-danger btn-sm".to_string()
                        button_style="".to_string()
                    />
                </form>
            </div>
        }
    });

    let version_items: Vec<_> = versions
        .into_iter()
        .map(|v| {
            let preview = render_version_content_preview(&kind, &v.content, i18n);
            view! {
                <div class="section-card" style="margin-bottom:1rem;">
                    <div style="display:flex; justify-content:space-between; align-items:center;">
                        <strong>{v.version_label.clone()}</strong>
                        <span style="font-size:0.75rem; color:var(--text-sub);">{v.created_at.format("%Y-%m-%d %H:%M").to_string()}</span>
                    </div>
                    {v.changelog.clone().map(|c| view! { <p class="text-muted" style="font-size:0.82rem;">{c}</p> })}
                    <details style="margin-top:0.5rem;">
                        <summary style="cursor:pointer; font-size:0.82rem; font-weight:600;">{i18n.template_preview_heading()}</summary>
                        <div style="margin-top:0.5rem;">{preview}</div>
                    </details>
                </div>
            }
        })
        .collect();

    view! {
        <AppShell user=user.clone() is_org_or_team_admin=is_org_or_team_admin active_nav=ActiveNav::Templates current_path=current_path page_title=template.name.clone() i18n=i18n>
            <div class="page-header">
                <div>
                    <div class="title-with-badge">
                        <h1 class="page-title">{template.name.clone()}</h1>
                        <span class=format!("pill-{}", template.kind)>{i18n.template_kind_label(&template.kind)}</span>
                    </div>
                    {template.description.clone().map(|d| view! { <p class="page-subtitle">{d}</p> })}
                </div>
            </div>
            {alert}
            {management_panel}
            <h3 class="card-subtitle" style="margin-bottom:0.75rem;">{i18n.template_versions_heading()}</h3>
            {version_items.is_empty().then(|| view! { <div class="empty-state"><p>{i18n.template_no_versions()}</p></div> })}
            {version_items}
        </AppShell>
    }
}

fn render_version_content_preview(kind: &str, content: &serde_json::Value, i18n: I18n) -> impl IntoView {
    match kind {
        "kanban" => match template_library::kanban_columns_from_content(content) {
            Ok(columns) => {
                let cols: Vec<_> = columns
                    .iter()
                    .map(|c| {
                        view! {
                            <div style="background:var(--bg-muted); border-radius:8px; padding:0.5rem 0.75rem; min-width:120px;">
                                <div style="font-weight:600; font-size:0.85rem;">{c.title.clone()}</div>
                                {c.is_done.then(|| view! { <span style="font-size:0.7rem; color:var(--success);">"✓ done"</span> })}
                            </div>
                        }
                    })
                    .collect();
                view! { <div style="display:flex; gap:0.6rem; flex-wrap:wrap;">{cols}</div> }.into_any()
            }
            Err(e) => view! { <div class="alert alert-danger">{e.to_string()}</div> }.into_any(),
        },
        "note" => match template_library::note_body_from_content(content) {
            Ok(body) => {
                let rendered = crate::services::document_renderer::DocumentRenderer::render_markdown_interactive(&body, "template-preview.md", Uuid::nil());
                view! { <div inner_html=rendered.html></div> }.into_any()
            }
            Err(e) => view! { <div class="alert alert-danger">{e.to_string()}</div> }.into_any(),
        },
        _ => match template_library::files_from_content(content) {
            Ok(files) => {
                let blocks: Vec<_> = files
                    .iter()
                    .map(|f| {
                        view! {
                            <div style="margin-bottom:0.75rem;">
                                <code style="font-size:0.75rem; color:var(--text-sub);">{f.path.clone()}</code>
                                <pre style="background:#0f172a; color:#e2e8f0; padding:0.75rem; border-radius:6px; font-size:0.8rem; overflow-x:auto; margin-top:0.25rem;">{f.content.clone()}</pre>
                            </div>
                        }
                    })
                    .collect();
                view! {
                    <div>
                        <p class="text-muted" style="font-size:0.78rem;">{i18n.template_source_only_preview_note()}</p>
                        {blocks}
                    </div>
                }
                .into_any()
            }
            Err(e) => view! { <div class="alert alert-danger">{e.to_string()}</div> }.into_any(),
        },
    }
}
