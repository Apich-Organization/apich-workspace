//! Template Library UI pages and components.
//!
//! Provides the gallery (`TemplateGalleryPage`) of publishable templates and the
//! per-template detail and version management page (`TemplateDetailPage`).

use crate::services::template_library;
use crate::ui::i18n::I18n;
use apich_db::Template;
use apich_db::TemplateVersion;
use apich_db::User;
use leptos::prelude::*;
use uuid::Uuid;

/// A resolved, human-readable label for one `apich_db::TemplateShare` row -- the DB only stores
/// `org_id`/`team_id`, so the handler resolves the actual org/team name before handing this to
/// the page (a Leptos view has no async DB access of its own).
pub struct TemplateShareDisplay {
    pub id: Uuid,
    pub label: String,
}

/// One selectable entry in the "share with" picker -- `value` is "org:<uuid>" or "team:<uuid>"
/// (parsed back apart by `template_handlers::add_template_share_action`), `label` a
/// human-readable name built from the real org/team, never a raw slug the owner would have to
/// already know and type correctly.
pub struct ShareTargetOption {
    pub value: String,
    pub label: String,
}

/// The result of actually compiling a "typst"/"slides"-kind version's content for preview
/// (`ui::template_handlers`'s `compile_typst_files_for_preview`) -- unlike kanban/note previews,
/// this needs a real (if ephemeral) compile step before the page can be rendered, so it's computed
/// in the async handler and handed in already-resolved rather than computed lazily inside the view.
pub struct CompiledPreview {
    pub svg_pages: Vec<String>,
    pub error: Option<String>,
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
    use crate::app::components::ActiveNav;
    use crate::app::components::AppShell;

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

    let current_kind = kind_filter.unwrap_or_default();
    let kinds = ["", "kanban", "note", "latex", "typst", "slides"];
    let tabs: Vec<_> = kinds
        .iter()
        .map(|k| {
            let is_active = current_kind == *k;
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

    let empty_state = rows
        .is_empty()
        .then(|| view! { <div class="empty-state"><p>{i18n.template_no_templates()}</p></div> });

    view! {
        <AppShell user=user is_org_or_team_admin=is_org_or_team_admin active_nav=ActiveNav::Templates current_path=current_path page_title=i18n.template_gallery_title().to_string() i18n=i18n>
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
    compiled_previews: std::collections::HashMap<Uuid, CompiledPreview>,
    user_projects: Vec<apich_db::Project>,
    shares: Vec<TemplateShareDisplay>,
    share_targets: Vec<ShareTargetOption>,
    is_owner: bool,
    notice: Option<String>,
    error: Option<String>,
    i18n: I18n,
    current_path: String,
) -> impl IntoView {
    use crate::app::components::ActiveNav;
    use crate::app::components::AppShell;

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

    let template_id = template.id;
    let kind = template.kind.clone();
    let projects = user_projects.into_boxed_slice();
    let mut compiled_previews = compiled_previews;

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
                {if share_targets.is_empty() {
                    view! { <p class="text-muted" style="font-size:0.78rem; margin-top:0.5rem;">{i18n.template_no_share_targets()}</p> }.into_any()
                } else {
                    let target_options: Vec<_> = share_targets
                        .into_iter()
                        .map(|t| view! { <option value=t.value>{t.label}</option> })
                        .collect();
                    view! {
                        <form method="post" action=format!("/templates/{}/share", template_id) class="inline-form" style="display:flex; gap:0.4rem; margin-top:0.5rem; flex-wrap:wrap;">
                            <select name="share_target" class="form-control" style="height:32px; font-size:0.82rem; max-width:280px;" required=true>
                                <option value="" disabled=true selected=true>{i18n.template_share_target_placeholder()}</option>
                                {target_options}
                            </select>
                            <button type="submit" class="btn btn-secondary btn-sm">{i18n.template_add_share()}</button>
                        </form>
                    }.into_any()
                }}
                <form method="post" action=format!("/templates/{}/delete", template_id) style="margin-top:0.75rem;">
                    <apich_islands::ConfirmSubmitButton
                        label=i18n.template_delete().to_string()
                        message="Delete this template and all its versions? This can't be undone.".to_string()
                        button_class="btn btn-danger btn-sm".to_string()
                        button_style=String::new()
                    />
                </form>
            </div>
        }
    });

    let version_items: Vec<_> = versions
        .into_iter()
        .map(|v| {
            let compiled = compiled_previews.remove(&v.id);
            let preview = render_version_content_preview(template_id, v.id, &kind, &v.content, compiled.as_ref(), i18n);
            let apply_form = render_apply_to_project_form(&kind, v.id, &projects, i18n);
            view! {
                <div class="section-card" style="margin-bottom:1rem;">
                    <div style="display:flex; justify-content:space-between; align-items:center;">
                        <strong>{v.version_label.clone()}</strong>
                        <span style="font-size:0.75rem; color:var(--text-sub);">{v.created_at.format("%Y-%m-%d %H:%M").to_string()}</span>
                    </div>
                    {v.changelog.clone().map(|c| view! { <p class="text-muted" style="font-size:0.82rem;">{c}</p> })}
                    {apply_form}
                    <details style="margin-top:0.5rem;">
                        <summary style="cursor:pointer; font-size:0.82rem; font-weight:600;">{i18n.template_preview_heading()}</summary>
                        <div style="margin-top:0.5rem;">{preview}</div>
                    </details>
                </div>
            }
        })
        .collect();

    view! {
        <AppShell user=user is_org_or_team_admin=is_org_or_team_admin active_nav=ActiveNav::Templates current_path=current_path page_title=template.name.clone() i18n=i18n>
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

/// "Apply to Project" -- the piece that was missing entirely before: browsing the Template
/// Library had no way to actually *use* a template, only the originating content page's own
/// picker did (kanban board / note editor / document editor), which meant discovering a template
/// here and then hunting for where to apply it. One project picker per version, routed to the
/// kind-appropriate `/templates/apply-to-*` action (see `template_handlers.rs`).
fn render_apply_to_project_form(
    kind: &str,
    version_id: Uuid,
    user_projects: &[apich_db::Project],
    i18n: I18n,
) -> impl IntoView {
    if user_projects.is_empty() {
        return view! { <p class="text-muted" style="font-size:0.78rem; margin-top:0.5rem;">{i18n.template_no_projects_to_apply()}</p> }.into_any();
    }
    let project_options: Vec<_> = user_projects
        .iter()
        .map(|p| view! { <option value=p.id.to_string()>{p.name.clone()}</option> })
        .collect();

    match kind {
        "kanban" => view! {
            <form method="post" action="/templates/apply-to-kanban" class="inline-form" style="display:flex; gap:0.4rem; margin-top:0.6rem; flex-wrap:wrap; align-items:center;">
                <input type="hidden" name="template_version_id" value=version_id.to_string() />
                <select name="project_id" class="form-control" style="height:32px; font-size:0.82rem; max-width:220px;" required=true>
                    {project_options}
                </select>
                <button type="submit" class="btn btn-primary btn-sm">{i18n.template_apply_to_project()}</button>
            </form>
        }
        .into_any(),
        "note" => view! {
            <form method="post" action="/templates/apply-to-note" class="inline-form" style="display:flex; gap:0.4rem; margin-top:0.6rem; flex-wrap:wrap; align-items:center;">
                <input type="hidden" name="template_version_id" value=version_id.to_string() />
                <select name="project_id" class="form-control" style="height:32px; font-size:0.82rem; max-width:220px;" required=true>
                    {project_options}
                </select>
                <input type="text" name="new_file_name" placeholder=i18n.template_new_file_name_field() class="form-control" style="height:32px; font-size:0.82rem; max-width:180px;" required=true />
                <button type="submit" class="btn btn-primary btn-sm">{i18n.template_apply_to_project()}</button>
            </form>
        }
        .into_any(),
        _ => view! {
            <form method="post" action="/templates/apply-to-file" class="inline-form" style="display:flex; gap:0.4rem; margin-top:0.6rem; flex-wrap:wrap; align-items:center;">
                <input type="hidden" name="template_version_id" value=version_id.to_string() />
                <select name="project_id" class="form-control" style="height:32px; font-size:0.82rem; max-width:220px;" required=true>
                    {project_options}
                </select>
                <input type="text" name="dest_subdir" placeholder=i18n.template_dest_folder_field() class="form-control" style="height:32px; font-size:0.82rem; max-width:180px;" />
                <button type="submit" class="btn btn-primary btn-sm">{i18n.template_apply_to_project()}</button>
            </form>
        }
        .into_any(),
    }
}

#[allow(clippy::too_many_arguments)]
fn render_version_content_preview(
    template_id: Uuid,
    version_id: Uuid,
    kind: &str,
    content: &serde_json::Value,
    compiled: Option<&CompiledPreview>,
    i18n: I18n,
) -> impl IntoView {
    match kind {
        | "kanban" => {
            match template_library::kanban_columns_from_content(content) {
                | Ok(columns) => {
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
                    view! { <div style="display:flex; gap:0.6rem; flex-wrap:wrap;">{cols}</div> }
                        .into_any()
                },
                | Err(e) => {
                    view! { <div class="alert alert-danger">{e.to_string()}</div> }.into_any()
                },
            }
        },
        | "note" => {
            match template_library::note_body_from_content(content) {
                | Ok(body) => {
                    let rendered = crate::services::document_renderer::DocumentRenderer::render_markdown_interactive(&body, "template-preview.md", Uuid::nil());
                    view! { <div inner_html=rendered.html></div> }.into_any()
                },
                | Err(e) => {
                    view! { <div class="alert alert-danger">{e.to_string()}</div> }.into_any()
                },
            }
        },
        | "typst" | "slides" => {
            // Compiled ahead of time in the async handler (see `compile_typst_files_for_preview`)
            // -- Typst (and slides, which are Typst underneath) compiles natively with no sandbox
            // needed, unlike LaTeX, so these two kinds get a real rendered preview instead of a
            // source dump.
            match compiled {
                | Some(c) if c.error.is_none() && !c.svg_pages.is_empty() => {
                    let pages: Vec<_> = c
                        .svg_pages
                        .iter()
                        .map(|svg| view! { <div class="svg-page-box" style="margin-bottom:0.75rem;" inner_html=svg.clone()></div> })
                        .collect();
                    view! { <div>{pages}</div> }.into_any()
                },
                | Some(c) => {
                    let msg = c
                        .error
                        .clone()
                        .unwrap_or_else(|| "Compilation produced no pages".to_string());
                    view! { <div class="alert alert-danger" style="white-space:pre-wrap; font-family:var(--font-mono); font-size:0.8rem;">{msg}</div> }.into_any()
                },
                | None => {
                    view! { <div class="alert alert-danger">"Preview not compiled"</div> }
                        .into_any()
                },
            }
        },
        | "latex" => {
            // Compiled on demand in a throwaway sandbox container (see
            // `latex_template_preview_pdf_action` / `ProjectManager::compile_latex_preview_ephemeral`)
            // -- real `pdflatex` output, not a source dump. The container only spins up once this
            // <iframe> actually loads, which browsers defer while its `<details>` stays collapsed.
            let pdf_url = format!("/templates/{template_id}/versions/{version_id}/preview.pdf");
            view! {
                <iframe
                    src=pdf_url
                    style="width:100%; height:600px; border:1px solid var(--border-subtle); border-radius:8px;"
                    title=i18n.template_preview_heading().to_string()
                ></iframe>
            }
            .into_any()
        },
        | _ => {
            match template_library::files_from_content(content) {
                | Ok(files) => {
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
                },
                | Err(e) => {
                    view! { <div class="alert alert-danger">{e.to_string()}</div> }.into_any()
                },
            }
        },
    }
}
