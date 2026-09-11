//! Routes and handlers for the Template Library: browsing (`/templates`, `/templates/:id`),
//! visibility/sharing management, and publishing/applying versions for each content kind. Kept
//! in its own module -- see `handlers.rs`'s `build_ui_router`, which merges this router in --
//! since it's a self-contained feature with no other handler depending on it.

use super::handlers::{get_i18n, html_escape, load_project_kanban_columns, redirect_error, redirect_notice, resolve_project};
use crate::app::pages::template_library_page::{CompiledPreview, ShareTargetOption, TemplateDetailPage, TemplateGalleryPage, TemplateShareDisplay};
use crate::auth::AuthUser;
use crate::error::{WebError, WebResult};
use crate::services::{template_library, KnowledgeSyncService};
use crate::state::AppState;
use apich_db::{CreateTemplateDto, IdentityPermissionResolver, PublishTemplateVersionDto};
use axum::{
    extract::{Form, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use uuid::Uuid;

pub fn build_template_router() -> Router<AppState> {
    Router::new()
        .route("/templates", get(template_gallery_page))
        .route("/templates/:id", get(template_detail_page))
        .route("/templates/:id/versions/:version_id/preview.pdf", get(latex_template_preview_pdf_action))
        .route("/templates/:id/visibility", post(update_template_visibility_action))
        .route("/templates/:id/share", post(add_template_share_action))
        .route("/templates/:id/share/:share_id/delete", post(remove_template_share_action))
        .route("/templates/:id/delete", post(delete_template_action))
        .route("/templates/create-from-kanban", post(create_template_from_kanban_action))
        .route("/templates/:id/publish-version-kanban", post(publish_kanban_version_action))
        .route("/templates/create-from-note", post(create_template_from_note_action))
        .route("/templates/:id/publish-version-note", post(publish_note_version_action))
        .route("/projects/:id/knowledge/kanban/apply-template", post(apply_kanban_template_action))
        .route("/projects/:id/note/apply-template", post(apply_note_template_action))
        .route("/templates/create-from-file", post(create_template_from_file_action))
        .route("/templates/:id/publish-version-file", post(publish_file_version_action))
        .route("/projects/:id/apply-file-template", post(apply_file_template_action))
        // The three routes above take the target project from the URL path -- fine for the
        // "Apply a Template" pickers on the kanban board/note editor/document editor themselves,
        // which are already scoped to one project. The Template Library's own gallery/detail
        // pages aren't scoped to any project, so applying *from there* needs the project chosen
        // in the form instead -- these three are that same underlying logic, just addressed by a
        // form field rather than the path, and redirecting to the target project afterward
        // instead of back to the template.
        .route("/templates/apply-to-kanban", post(apply_kanban_template_from_library_action))
        .route("/templates/apply-to-note", post(apply_note_template_from_library_action))
        .route("/templates/apply-to-file", post(apply_file_template_from_library_action))
}

#[derive(Debug, Deserialize)]
pub struct TemplateGalleryQuery {
    pub kind: Option<String>,
    pub notice: Option<String>,
    pub error: Option<String>,
}

async fn template_gallery_page(
    auth: Option<AuthUser>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(query): Query<TemplateGalleryQuery>,
) -> Response {
    let AuthUser(user) = match auth {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };
    let i18n = get_i18n(&headers, None);
    let repo = state.db.repository();
    let is_org_admin = state.identity_service.is_org_or_team_admin(user.id).await.unwrap_or(false);
    let kind = query.kind.filter(|k| !k.is_empty());
    let templates = repo.list_visible_templates(user.id, kind.as_deref()).await.unwrap_or_default();
    let current_path = "/templates".to_string();

    let html = crate::app::components::render_document(move || {
        leptos::prelude::view! {
            <TemplateGalleryPage
                user=user
                is_org_or_team_admin=is_org_admin
                templates=templates
                kind_filter=kind
                notice=query.notice
                error=query.error
                i18n=i18n
                current_path=current_path
            />
        }
    });
    Html(html).into_response()
}

#[derive(Debug, Deserialize)]
pub struct TemplateDetailQuery {
    pub notice: Option<String>,
    pub error: Option<String>,
}

async fn template_detail_page(
    auth: Option<AuthUser>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(query): Query<TemplateDetailQuery>,
) -> Response {
    let AuthUser(user) = match auth {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };
    let can_access = IdentityPermissionResolver::can_access_template(state.db.pool(), user.id, id).await.unwrap_or(false);
    if !can_access {
        return (axum::http::StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }

    let i18n = get_i18n(&headers, None);
    let repo = state.db.repository();
    let is_org_admin = state.identity_service.is_org_or_team_admin(user.id).await.unwrap_or(false);
    let Some(template) = repo.get_template_by_id(id).await.unwrap_or(None) else {
        return (axum::http::StatusCode::NOT_FOUND, Html("<h3>Template not found</h3>")).into_response();
    };
    let versions = repo.list_template_versions(id).await.unwrap_or_default();
    let is_owner = template.owner_user_id == user.id;
    let user_projects = repo.list_projects_for_user(user.id).await.unwrap_or_default();

    // Typst and slides (also Typst under the hood) compile natively, in-process, with no sandbox
    // container needed -- unlike LaTeX (`pdflatex` only exists inside a project's sandbox), so
    // there's no reason these two kinds should be stuck with a source-only preview the way LaTeX
    // currently has to be. Compiled once per page load in an ephemeral scratch directory (cleaned
    // up immediately after), not cached -- acceptable for the version counts a template
    // realistically has.
    let mut compiled_previews = std::collections::HashMap::new();
    if matches!(template.kind.as_str(), "typst" | "slides") {
        for version in &versions {
            if let Ok(files) = template_library::files_from_content(&version.content) {
                let preview = compile_typst_files_for_preview(&files, template.kind == "slides").await;
                compiled_previews.insert(version.id, preview);
            }
        }
    }

    let mut shares = Vec::new();
    let mut share_targets = Vec::new();
    if is_owner {
        for share in repo.list_template_shares(id).await.unwrap_or_default() {
            let label = if let Some(org_id) = share.org_id {
                repo.get_organization_by_id(org_id).await.ok().flatten().map(|o| o.name).unwrap_or_else(|| "(unknown org)".to_string())
            } else if let Some(team_id) = share.team_id {
                match repo.get_team_by_id(team_id).await.ok().flatten() {
                    Some(team) => {
                        let org_name = repo.get_organization_by_id(team.org_id).await.ok().flatten().map(|o| o.name).unwrap_or_default();
                        format!("{org_name} / {}", team.name)
                    }
                    None => "(unknown team)".to_string(),
                }
            } else {
                "(unknown)".to_string()
            };
            shares.push(TemplateShareDisplay { id: share.id, label });
        }

        // Every org/team on the instance, not just ones the owner belongs to -- sharing a
        // template was explicitly asked to not be limited to the publisher's own org/team, so the
        // picker needs to offer all of them, not a "my orgs" shortcut list.
        for org in repo.list_organizations(500).await.unwrap_or_default() {
            share_targets.push(ShareTargetOption { value: format!("org:{}", org.id), label: format!("🏢 {} (org)", org.name) });
            for team in repo.list_teams_by_org(org.id).await.unwrap_or_default() {
                share_targets.push(ShareTargetOption { value: format!("team:{}", team.id), label: format!("　└ {} / {}", org.name, team.name) });
            }
        }
    }

    let current_path = format!("/templates/{id}");
    let html = crate::app::components::render_document(move || {
        leptos::prelude::view! {
            <TemplateDetailPage
                user=user
                is_org_or_team_admin=is_org_admin
                template=template
                versions=versions
                compiled_previews=compiled_previews
                user_projects=user_projects
                shares=shares
                share_targets=share_targets
                is_owner=is_owner
                notice=query.notice
                error=query.error
                i18n=i18n
                current_path=current_path
            />
        }
    });
    Html(html).into_response()
}

/// Serves a LaTeX-kind template version compiled fresh, on demand, in a throwaway sandbox
/// container (see `ProjectManager::compile_latex_preview_ephemeral`) -- embedded via a plain
/// `<iframe>` on the detail page rather than eagerly compiled for every version the way
/// typst/slides previews are, since spinning a whole container up and down takes real seconds,
/// not milliseconds; fetched only once someone actually opens that version's Preview section.
async fn latex_template_preview_pdf_action(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Path((_id, version_id)): Path<(Uuid, Uuid)>,
) -> Response {
    let AuthUser(user) = match auth {
        Some(u) => u,
        None => return (StatusCode::UNAUTHORIZED, Html("Unauthorized")).into_response(),
    };

    let repo = state.db.repository();
    let Some(version) = repo.get_template_version_by_id(version_id).await.unwrap_or(None) else {
        return (StatusCode::NOT_FOUND, Html("Template version not found")).into_response();
    };
    if !IdentityPermissionResolver::can_access_template(state.db.pool(), user.id, version.template_id).await.unwrap_or(false) {
        return (StatusCode::FORBIDDEN, Html("Forbidden")).into_response();
    }
    let Some(template) = repo.get_template_by_id(version.template_id).await.unwrap_or(None) else {
        return (StatusCode::NOT_FOUND, Html("Template not found")).into_response();
    };
    if template.kind != "latex" {
        return (StatusCode::BAD_REQUEST, Html("Not a LaTeX template")).into_response();
    }

    let files = match template_library::files_from_content(&version.content) {
        Ok(f) => f.into_iter().map(|f| (f.path, f.content)).collect::<Vec<_>>(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Html(e.to_string())).into_response(),
    };

    match state.project_manager.compile_latex_preview_ephemeral(&files).await {
        Ok(Ok(pdf_bytes)) => ([(header::CONTENT_TYPE, "application/pdf")], pdf_bytes).into_response(),
        Ok(Err(compile_log)) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            Html(format!(
                "<html><body style=\"background:#1e1e1e; color:#f87171; font-family:monospace; white-space:pre-wrap; padding:1.5rem; margin:0; font-size:0.85rem;\">⚠️ LaTeX compilation failed:\n\n{}</body></html>",
                html_escape(&compile_log)
            )),
        )
            .into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Html(format!("Failed to start preview container: {e}"))).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateVisibilityForm {
    pub visibility: String,
}

async fn update_template_visibility_action(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Form(payload): Form<UpdateVisibilityForm>,
) -> Response {
    let AuthUser(user) = match auth {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };
    if !IdentityPermissionResolver::can_manage_template(state.db.pool(), user.id, id).await.unwrap_or(false) {
        return (axum::http::StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }
    let visibility = if matches!(payload.visibility.as_str(), "shared" | "public") { payload.visibility } else { "private".to_string() };
    let _ = state.db.repository().update_template_visibility(id, &visibility).await;
    Redirect::to(&format!("/templates/{id}")).into_response()
}

#[derive(Debug, Deserialize)]
pub struct AddShareForm {
    /// "org:<uuid>" or "team:<uuid>" -- one flat picker covering every org and team on the
    /// instance (see `template_detail_page`'s `share_targets`), not just the ones the owner
    /// happens to belong to, since sharing was explicitly asked to not be limited that way.
    pub share_target: String,
}

async fn add_template_share_action(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Form(payload): Form<AddShareForm>,
) -> Response {
    let AuthUser(user) = match auth {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };
    if !IdentityPermissionResolver::can_manage_template(state.db.pool(), user.id, id).await.unwrap_or(false) {
        return (axum::http::StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }
    let repo = state.db.repository();
    let (org_id, team_id) = match payload.share_target.split_once(':') {
        Some(("org", rest)) => match Uuid::parse_str(rest) {
            Ok(org_id) => (Some(org_id), None),
            Err(_) => return redirect_error(&format!("/templates/{id}"), "Invalid org selection"),
        },
        Some(("team", rest)) => match Uuid::parse_str(rest) {
            Ok(team_id) => (None, Some(team_id)),
            Err(_) => return redirect_error(&format!("/templates/{id}"), "Invalid team selection"),
        },
        _ => return redirect_error(&format!("/templates/{id}"), "Select an org or team to share with"),
    };
    if repo.add_template_share(id, org_id, team_id).await.is_err() {
        return redirect_error(&format!("/templates/{id}"), "Failed to add share (maybe it already exists)");
    }
    // Sharing only takes effect once visibility is 'shared' -- flip it automatically so adding a
    // share doesn't silently do nothing on a still-'private' template.
    let _ = repo.update_template_visibility(id, "shared").await;
    Redirect::to(&format!("/templates/{id}")).into_response()
}

async fn remove_template_share_action(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Path((id, share_id)): Path<(Uuid, Uuid)>,
) -> Response {
    let AuthUser(user) = match auth {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };
    if !IdentityPermissionResolver::can_manage_template(state.db.pool(), user.id, id).await.unwrap_or(false) {
        return (axum::http::StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }
    let _ = state.db.repository().remove_template_share(share_id).await;
    Redirect::to(&format!("/templates/{id}")).into_response()
}

async fn delete_template_action(auth: Option<AuthUser>, State(state): State<AppState>, Path(id): Path<Uuid>) -> Response {
    let AuthUser(user) = match auth {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };
    if !IdentityPermissionResolver::can_manage_template(state.db.pool(), user.id, id).await.unwrap_or(false) {
        return (axum::http::StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }
    let _ = state.db.repository().delete_template(id).await;
    Redirect::to("/templates").into_response()
}

/// Shared by every "publish this project's content as a NEW template" action -- creates the
/// `templates` row, then immediately publishes version 1 with the given content, so a template
/// never exists with zero versions.
#[allow(clippy::too_many_arguments)]
async fn create_template_and_first_version(
    state: &AppState,
    owner_user_id: Uuid,
    kind: &str,
    name: &str,
    slug: &str,
    description: Option<String>,
    visibility: &str,
    version_label: &str,
    changelog: Option<String>,
    content: serde_json::Value,
) -> WebResult<Uuid> {
    let repo = state.db.repository();
    let template = repo
        .create_template(CreateTemplateDto {
            kind: kind.to_string(),
            name: name.to_string(),
            slug: slug.to_string(),
            description,
            owner_user_id,
            visibility: visibility.to_string(),
        })
        .await
        .map_err(|e| WebError::BadRequest(format!("Failed to create template (slug may already be in use): {e}")))?;

    repo.publish_template_version(PublishTemplateVersionDto {
        template_id: template.id,
        version_label: version_label.to_string(),
        changelog,
        content,
        published_by: owner_user_id,
    })
    .await
    .map_err(|e| WebError::Internal(format!("Failed to publish first version: {e}")))?;

    Ok(template.id)
}

#[derive(Debug, Deserialize)]
pub struct CreateKanbanTemplateForm {
    pub project_id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub visibility: String,
    pub version_label: String,
    pub changelog: Option<String>,
}

async fn create_template_from_kanban_action(auth: Option<AuthUser>, headers: HeaderMap, State(state): State<AppState>, Form(payload): Form<CreateKanbanTemplateForm>) -> Response {
    let AuthUser(user) = match auth {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };
    let Some(project) = resolve_project(&state, &payload.project_id.to_string()).await else {
        return Redirect::to("/").into_response();
    };
    if !apich_db::IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id).await.unwrap_or(false) {
        return (axum::http::StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }

    let i18n = get_i18n(&headers, None);
    let columns = load_project_kanban_columns(&state, &project, i18n).await;
    let content = template_library::kanban_content_from_columns(&columns);
    let visibility = if matches!(payload.visibility.as_str(), "shared" | "public") { payload.visibility.clone() } else { "private".to_string() };

    let back = format!("/projects/{}/note?view=kanban", project.id);
    match create_template_and_first_version(&state, user.id, "kanban", &payload.name, &payload.slug, payload.description, &visibility, &payload.version_label, payload.changelog, content).await {
        Ok(template_id) => redirect_notice(&back, &format!("Published template '{}' (view it at /templates/{})", payload.name, template_id)),
        Err(e) => redirect_error(&back, e),
    }
}

#[derive(Debug, Deserialize)]
pub struct PublishKanbanVersionForm {
    pub project_id: Uuid,
    pub version_label: String,
    pub changelog: Option<String>,
}

async fn publish_kanban_version_action(auth: Option<AuthUser>, headers: HeaderMap, State(state): State<AppState>, Path(id): Path<Uuid>, Form(payload): Form<PublishKanbanVersionForm>) -> Response {
    let AuthUser(user) = match auth {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };
    let Some(project) = resolve_project(&state, &payload.project_id.to_string()).await else {
        return Redirect::to("/").into_response();
    };
    let back = format!("/projects/{}/note?view=kanban", project.id);
    if !apich_db::IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id).await.unwrap_or(false) {
        return (axum::http::StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }
    if !IdentityPermissionResolver::can_manage_template(state.db.pool(), user.id, id).await.unwrap_or(false) {
        return redirect_error(&back, "You don't own that template");
    }

    let i18n = get_i18n(&headers, None);
    let columns = load_project_kanban_columns(&state, &project, i18n).await;
    let content = template_library::kanban_content_from_columns(&columns);
    let repo = state.db.repository();
    match repo
        .publish_template_version(PublishTemplateVersionDto {
            template_id: id,
            version_label: payload.version_label.clone(),
            changelog: payload.changelog,
            content,
            published_by: user.id,
        })
        .await
    {
        Ok(_) => redirect_notice(&back, &format!("Published version {} to your template", payload.version_label)),
        Err(e) => redirect_error(&back, format!("Failed to publish version (label may already exist): {e}")),
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateNoteTemplateForm {
    pub project_id: Uuid,
    pub file: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub visibility: String,
    pub version_label: String,
    pub changelog: Option<String>,
}

async fn create_template_from_note_action(auth: Option<AuthUser>, State(state): State<AppState>, Form(payload): Form<CreateNoteTemplateForm>) -> Response {
    let AuthUser(user) = match auth {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };
    let Some(project) = resolve_project(&state, &payload.project_id.to_string()).await else {
        return Redirect::to("/").into_response();
    };
    let back = format!("/projects/{}/note?file={}&view=editor", project.id, urlencoding::encode(&payload.file));
    if !apich_db::IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id).await.unwrap_or(false) {
        return (axum::http::StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }

    let raw = state.project_manager.read_file(project.id, &payload.file).await.unwrap_or_default();
    let (_, body) = KnowledgeSyncService::parse_unified_note(&raw);
    let content = template_library::note_content_from_body(&body);
    let visibility = if matches!(payload.visibility.as_str(), "shared" | "public") { payload.visibility.clone() } else { "private".to_string() };

    match create_template_and_first_version(&state, user.id, "note", &payload.name, &payload.slug, payload.description, &visibility, &payload.version_label, payload.changelog, content).await {
        Ok(template_id) => redirect_notice(&back, &format!("Published template '{}' (view it at /templates/{})", payload.name, template_id)),
        Err(e) => redirect_error(&back, e),
    }
}

#[derive(Debug, Deserialize)]
pub struct PublishNoteVersionForm {
    pub project_id: Uuid,
    pub file: String,
    pub version_label: String,
    pub changelog: Option<String>,
}

async fn publish_note_version_action(auth: Option<AuthUser>, State(state): State<AppState>, Path(id): Path<Uuid>, Form(payload): Form<PublishNoteVersionForm>) -> Response {
    let AuthUser(user) = match auth {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };
    let Some(project) = resolve_project(&state, &payload.project_id.to_string()).await else {
        return Redirect::to("/").into_response();
    };
    let back = format!("/projects/{}/note?file={}&view=editor", project.id, urlencoding::encode(&payload.file));
    if !apich_db::IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id).await.unwrap_or(false) {
        return (axum::http::StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }
    if !IdentityPermissionResolver::can_manage_template(state.db.pool(), user.id, id).await.unwrap_or(false) {
        return redirect_error(&back, "You don't own that template");
    }

    let raw = state.project_manager.read_file(project.id, &payload.file).await.unwrap_or_default();
    let (_, body) = KnowledgeSyncService::parse_unified_note(&raw);
    let content = template_library::note_content_from_body(&body);
    let repo = state.db.repository();
    match repo
        .publish_template_version(PublishTemplateVersionDto {
            template_id: id,
            version_label: payload.version_label.clone(),
            changelog: payload.changelog,
            content,
            published_by: user.id,
        })
        .await
    {
        Ok(_) => redirect_notice(&back, &format!("Published version {} to your template", payload.version_label)),
        Err(e) => redirect_error(&back, format!("Failed to publish version (label may already exist): {e}")),
    }
}

#[derive(Debug, Deserialize)]
pub struct ApplyKanbanTemplateForm {
    pub template_version_id: Uuid,
}

async fn apply_kanban_template_action(auth: Option<AuthUser>, State(state): State<AppState>, Path(id_or_slug): Path<String>, Form(payload): Form<ApplyKanbanTemplateForm>) -> Response {
    let AuthUser(user) = match auth {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };
    let Some(project) = resolve_project(&state, &id_or_slug).await else {
        return Redirect::to("/").into_response();
    };
    let back = format!("/projects/{}/note?view=kanban", project.id);
    if !apich_db::IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id).await.unwrap_or(false) {
        return (axum::http::StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }

    let repo = state.db.repository();
    let Some(version) = repo.get_template_version_by_id(payload.template_version_id).await.unwrap_or(None) else {
        return redirect_error(&back, "Template version not found");
    };
    if !IdentityPermissionResolver::can_access_template(state.db.pool(), user.id, version.template_id).await.unwrap_or(false) {
        return redirect_error(&back, "You don't have access to that template");
    }

    let columns = match template_library::kanban_columns_from_content(&version.content) {
        Ok(c) => c,
        Err(e) => return redirect_error(&back, e),
    };
    let mut settings = project.settings.clone();
    settings["kanban_columns"] = KnowledgeSyncService::serialize_kanban_columns(&columns);
    let _ = repo.update_project_settings(project.id, settings).await;
    redirect_notice(&back, "Applied template columns to this board")
}

#[derive(Debug, Deserialize)]
pub struct ApplyNoteTemplateForm {
    pub template_version_id: Uuid,
    pub new_file_name: String,
}

async fn apply_note_template_action(auth: Option<AuthUser>, State(state): State<AppState>, Path(id_or_slug): Path<String>, Form(payload): Form<ApplyNoteTemplateForm>) -> Response {
    let AuthUser(user) = match auth {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };
    let Some(project) = resolve_project(&state, &id_or_slug).await else {
        return Redirect::to("/").into_response();
    };
    let back = format!("/projects/{}/note", project.id);
    if !apich_db::IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id).await.unwrap_or(false) {
        return (axum::http::StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }

    let clean_name = payload.new_file_name.trim().trim_start_matches('/');
    if clean_name.is_empty() || clean_name.contains("..") {
        return redirect_error(&back, "Invalid file name");
    }
    let file_name = if clean_name.ends_with(".anote") || clean_name.ends_with(".md") { clean_name.to_string() } else { format!("{clean_name}.anote") };

    let repo = state.db.repository();
    let Some(version) = repo.get_template_version_by_id(payload.template_version_id).await.unwrap_or(None) else {
        return redirect_error(&back, "Template version not found");
    };
    if !IdentityPermissionResolver::can_access_template(state.db.pool(), user.id, version.template_id).await.unwrap_or(false) {
        return redirect_error(&back, "You don't have access to that template");
    }

    let body = match template_library::note_body_from_content(&version.content) {
        Ok(b) => b,
        Err(e) => return redirect_error(&back, e),
    };
    if state.project_manager.read_file(project.id, &file_name).await.map(|c| !c.is_empty()).unwrap_or(false) {
        return redirect_error(&back, format!("{file_name} already exists in this project"));
    }
    if state.project_manager.write_file(project.id, user.id, &file_name, &body).await.is_err() {
        return redirect_error(&back, "Failed to write the new note file");
    }
    redirect_notice(&format!("/projects/{}/note?file={}", project.id, urlencoding::encode(&file_name)), "Created note from template")
}

#[derive(Debug, Deserialize)]
pub struct CreateFileTemplateForm {
    pub project_id: Uuid,
    pub file: String,
    pub kind: String, // "latex" | "typst" | "slides"
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub visibility: String,
    pub version_label: String,
    pub changelog: Option<String>,
}

/// Publishing a LaTeX/Typst/slides template captures exactly one file -- the one the editor
/// panel was rendered for -- not an arbitrary file picker; see `template_library`'s own doc
/// comment on why compiled preview isn't attempted for this kind (no sandbox to compile in
/// outside a real project), which is also why this stays deliberately simple rather than growing
/// a multi-file selection UI to match.
async fn create_template_from_file_action(auth: Option<AuthUser>, State(state): State<AppState>, Form(payload): Form<CreateFileTemplateForm>) -> Response {
    let AuthUser(user) = match auth {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };
    let Some(project) = resolve_project(&state, &payload.project_id.to_string()).await else {
        return Redirect::to("/").into_response();
    };
    let back = format!("/projects/{}/editor?file={}", project.id, urlencoding::encode(&payload.file));
    if !apich_db::IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id).await.unwrap_or(false) {
        return (axum::http::StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }
    if !matches!(payload.kind.as_str(), "latex" | "typst" | "slides") {
        return redirect_error(&back, "Invalid template kind");
    }

    let content = match template_library::read_files_from_project(&project.storage_path, std::slice::from_ref(&payload.file)) {
        Ok(c) => c,
        Err(e) => return redirect_error(&back, e),
    };
    let visibility = if matches!(payload.visibility.as_str(), "shared" | "public") { payload.visibility.clone() } else { "private".to_string() };

    match create_template_and_first_version(&state, user.id, &payload.kind, &payload.name, &payload.slug, payload.description, &visibility, &payload.version_label, payload.changelog, content).await {
        Ok(template_id) => redirect_notice(&back, &format!("Published template '{}' (view it at /templates/{})", payload.name, template_id)),
        Err(e) => redirect_error(&back, e),
    }
}

#[derive(Debug, Deserialize)]
pub struct PublishFileVersionForm {
    pub project_id: Uuid,
    pub file: String,
    pub version_label: String,
    pub changelog: Option<String>,
}

async fn publish_file_version_action(auth: Option<AuthUser>, State(state): State<AppState>, Path(id): Path<Uuid>, Form(payload): Form<PublishFileVersionForm>) -> Response {
    let AuthUser(user) = match auth {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };
    let Some(project) = resolve_project(&state, &payload.project_id.to_string()).await else {
        return Redirect::to("/").into_response();
    };
    let back = format!("/projects/{}/editor?file={}", project.id, urlencoding::encode(&payload.file));
    if !apich_db::IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id).await.unwrap_or(false) {
        return (axum::http::StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }
    if !IdentityPermissionResolver::can_manage_template(state.db.pool(), user.id, id).await.unwrap_or(false) {
        return redirect_error(&back, "You don't own that template");
    }

    let content = match template_library::read_files_from_project(&project.storage_path, std::slice::from_ref(&payload.file)) {
        Ok(c) => c,
        Err(e) => return redirect_error(&back, e),
    };
    let repo = state.db.repository();
    match repo
        .publish_template_version(PublishTemplateVersionDto {
            template_id: id,
            version_label: payload.version_label.clone(),
            changelog: payload.changelog,
            content,
            published_by: user.id,
        })
        .await
    {
        Ok(_) => redirect_notice(&back, &format!("Published version {} to your template", payload.version_label)),
        Err(e) => redirect_error(&back, format!("Failed to publish version (label may already exist): {e}")),
    }
}

#[derive(Debug, Deserialize)]
pub struct ApplyFileTemplateForm {
    pub template_version_id: Uuid,
    #[serde(default)]
    pub dest_subdir: String,
}

async fn apply_file_template_action(auth: Option<AuthUser>, State(state): State<AppState>, Path(id_or_slug): Path<String>, Form(payload): Form<ApplyFileTemplateForm>) -> Response {
    let AuthUser(user) = match auth {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };
    let Some(project) = resolve_project(&state, &id_or_slug).await else {
        return Redirect::to("/").into_response();
    };
    let back = format!("/projects/{}?tab=files", project.id);
    if !apich_db::IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id).await.unwrap_or(false) {
        return (axum::http::StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }

    let repo = state.db.repository();
    let Some(version) = repo.get_template_version_by_id(payload.template_version_id).await.unwrap_or(None) else {
        return redirect_error(&back, "Template version not found");
    };
    if !IdentityPermissionResolver::can_access_template(state.db.pool(), user.id, version.template_id).await.unwrap_or(false) {
        return redirect_error(&back, "You don't have access to that template");
    }
    ensure_slide_helpers_if_applicable(&repo, &project.storage_path, version.template_id).await;

    match template_library::apply_files_content_to_project(&project.storage_path, &version.content, &payload.dest_subdir) {
        Ok(written) => {
            if let Ok(vcs) = apich_vcs::api::ProjectVcs::open_or_init(&project.storage_path) {
                let _ = vcs.snapshot_if_changed(format!("Applied template files: {}", written.join(", ")));
            }
            redirect_notice(&back, &format!("Wrote {} file(s) from the template", written.len()))
        }
        Err(e) => redirect_error(&back, e),
    }
}

/// A "slides"-kind template's file imports `theme.typ`/`slide.typ` -- provision both into the
/// target project if this template needs them and they're not already there (see
/// `cargo_slide_helpers`'s own doc comment on why a fresh project otherwise has neither).
/// Best-effort: a failure here shouldn't block applying the template's own file, since a real
/// write error will surface from `apply_files_content_to_project`/the compile itself anyway.
async fn ensure_slide_helpers_if_applicable(repo: &apich_db::Repository<'_>, storage_path: &str, template_id: Uuid) {
    if let Ok(Some(template)) = repo.get_template_by_id(template_id).await {
        if template.kind == "slides" {
            let _ = crate::services::cargo_slide_helpers::ensure_cargo_slide_helpers(storage_path).await;
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ApplyKanbanTemplateFromLibraryForm {
    pub project_id: Uuid,
    pub template_version_id: Uuid,
}

/// Same logic as `apply_kanban_template_action`, addressed by a `project_id` form field instead
/// of a path segment -- used by the "Apply to Project" action on the template's own detail page
/// (`/templates/:id`), which isn't scoped to any one project the way the kanban board's own
/// picker is.
async fn apply_kanban_template_from_library_action(auth: Option<AuthUser>, State(state): State<AppState>, Form(payload): Form<ApplyKanbanTemplateFromLibraryForm>) -> Response {
    let AuthUser(user) = match auth {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };
    let Some(project) = resolve_project(&state, &payload.project_id.to_string()).await else {
        return Redirect::to("/").into_response();
    };
    let back = format!("/projects/{}/note?view=kanban", project.id);
    if !apich_db::IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id).await.unwrap_or(false) {
        return (axum::http::StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }

    let repo = state.db.repository();
    let Some(version) = repo.get_template_version_by_id(payload.template_version_id).await.unwrap_or(None) else {
        return redirect_error(&back, "Template version not found");
    };
    if !IdentityPermissionResolver::can_access_template(state.db.pool(), user.id, version.template_id).await.unwrap_or(false) {
        return redirect_error(&back, "You don't have access to that template");
    }

    let columns = match template_library::kanban_columns_from_content(&version.content) {
        Ok(c) => c,
        Err(e) => return redirect_error(&back, e),
    };
    let mut settings = project.settings.clone();
    settings["kanban_columns"] = KnowledgeSyncService::serialize_kanban_columns(&columns);
    let _ = repo.update_project_settings(project.id, settings).await;
    redirect_notice(&back, "Applied template columns to this board")
}

#[derive(Debug, Deserialize)]
pub struct ApplyNoteTemplateFromLibraryForm {
    pub project_id: Uuid,
    pub template_version_id: Uuid,
    pub new_file_name: String,
}

async fn apply_note_template_from_library_action(auth: Option<AuthUser>, State(state): State<AppState>, Form(payload): Form<ApplyNoteTemplateFromLibraryForm>) -> Response {
    let AuthUser(user) = match auth {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };
    let Some(project) = resolve_project(&state, &payload.project_id.to_string()).await else {
        return Redirect::to("/").into_response();
    };
    let back = format!("/projects/{}/note", project.id);
    if !apich_db::IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id).await.unwrap_or(false) {
        return (axum::http::StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }

    let clean_name = payload.new_file_name.trim().trim_start_matches('/');
    if clean_name.is_empty() || clean_name.contains("..") {
        return redirect_error(&back, "Invalid file name");
    }
    let file_name = if clean_name.ends_with(".anote") || clean_name.ends_with(".md") { clean_name.to_string() } else { format!("{clean_name}.anote") };

    let repo = state.db.repository();
    let Some(version) = repo.get_template_version_by_id(payload.template_version_id).await.unwrap_or(None) else {
        return redirect_error(&back, "Template version not found");
    };
    if !IdentityPermissionResolver::can_access_template(state.db.pool(), user.id, version.template_id).await.unwrap_or(false) {
        return redirect_error(&back, "You don't have access to that template");
    }

    let body = match template_library::note_body_from_content(&version.content) {
        Ok(b) => b,
        Err(e) => return redirect_error(&back, e),
    };
    if state.project_manager.read_file(project.id, &file_name).await.map(|c| !c.is_empty()).unwrap_or(false) {
        return redirect_error(&back, format!("{file_name} already exists in this project"));
    }
    if state.project_manager.write_file(project.id, user.id, &file_name, &body).await.is_err() {
        return redirect_error(&back, "Failed to write the new note file");
    }
    redirect_notice(&format!("/projects/{}/note?file={}", project.id, urlencoding::encode(&file_name)), "Created note from template")
}

#[derive(Debug, Deserialize)]
pub struct ApplyFileTemplateFromLibraryForm {
    pub project_id: Uuid,
    pub template_version_id: Uuid,
    #[serde(default)]
    pub dest_subdir: String,
}

async fn apply_file_template_from_library_action(auth: Option<AuthUser>, State(state): State<AppState>, Form(payload): Form<ApplyFileTemplateFromLibraryForm>) -> Response {
    let AuthUser(user) = match auth {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };
    let Some(project) = resolve_project(&state, &payload.project_id.to_string()).await else {
        return Redirect::to("/").into_response();
    };
    let back = format!("/projects/{}?tab=files", project.id);
    if !apich_db::IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id).await.unwrap_or(false) {
        return (axum::http::StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }

    let repo = state.db.repository();
    let Some(version) = repo.get_template_version_by_id(payload.template_version_id).await.unwrap_or(None) else {
        return redirect_error(&back, "Template version not found");
    };
    if !IdentityPermissionResolver::can_access_template(state.db.pool(), user.id, version.template_id).await.unwrap_or(false) {
        return redirect_error(&back, "You don't have access to that template");
    }
    ensure_slide_helpers_if_applicable(&repo, &project.storage_path, version.template_id).await;

    match template_library::apply_files_content_to_project(&project.storage_path, &version.content, &payload.dest_subdir) {
        Ok(written) => {
            if let Ok(vcs) = apich_vcs::api::ProjectVcs::open_or_init(&project.storage_path) {
                let _ = vcs.snapshot_if_changed(format!("Applied template files: {}", written.join(", ")));
            }
            redirect_notice(&back, &format!("Wrote {} file(s) from the template", written.len()))
        }
        Err(e) => redirect_error(&back, e),
    }
}

/// Actually compiles a "typst"/"slides"-kind version's file(s) for preview -- writes them into a
/// throwaway directory under the OS temp dir (removed again immediately after), provisioning
/// `theme.typ`/`slide.typ` alongside for `is_slides` the same way applying the template for real
/// would (see `cargo_slide_helpers`), then runs the exact same native Typst compiler this app's
/// own document editor uses. No sandbox container involved -- unlike LaTeX, which is why this
/// exists for these two kinds and not that one.
async fn compile_typst_files_for_preview(files: &[template_library::TemplateFile], is_slides: bool) -> CompiledPreview {
    let dir = std::env::temp_dir().join(format!("apich-template-preview-{}", Uuid::new_v4()));
    if tokio::fs::create_dir_all(&dir).await.is_err() {
        return CompiledPreview { svg_pages: Vec::new(), error: Some("Failed to create a preview workspace".to_string()) };
    }

    for f in files {
        let target = dir.join(&f.path);
        if let Some(parent) = target.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        let _ = tokio::fs::write(&target, &f.content).await;
    }
    if is_slides {
        let _ = crate::services::cargo_slide_helpers::ensure_cargo_slide_helpers(&dir).await;
    }

    let main_file = files.first().map(|f| f.path.clone()).unwrap_or_default();
    let result = crate::services::document_renderer::DocumentRenderer::compile_typst(&dir, &main_file, false).await;
    let _ = tokio::fs::remove_dir_all(&dir).await;

    if result.success {
        CompiledPreview { svg_pages: result.pages_svg, error: None }
    } else {
        CompiledPreview { svg_pages: Vec::new(), error: Some(result.error_message.unwrap_or_else(|| "Compilation failed".to_string())) }
    }
}

/// Small helper for the "publish new version to an existing template of mine" dropdowns --
/// filters `list_templates_owned_by` down to one kind, which is all the caller can actually
/// publish a matching-shaped version to.
pub async fn list_own_templates_of_kind(state: &AppState, user_id: Uuid, kind: &str) -> Vec<apich_db::Template> {
    state
        .db
        .repository()
        .list_templates_owned_by(user_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|t| t.kind == kind)
        .collect()
}

/// Every template of one kind the user can currently see -- feeds the "Apply a Template"
/// dropdown, which needs both the template name and its latest version id in one row.
pub async fn list_visible_templates_of_kind(state: &AppState, user_id: Uuid, kind: &str) -> Vec<apich_db::TemplateWithLatestVersion> {
    state.db.repository().list_visible_templates(user_id, Some(kind)).await.unwrap_or_default()
}
