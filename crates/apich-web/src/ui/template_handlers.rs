//! Routes and handlers for the Template Library: browsing (`/templates`, `/templates/:id`),
//! visibility/sharing management, and publishing/applying versions for each content kind. Kept
//! in its own module -- see `handlers.rs`'s `build_ui_router`, which merges this router in --
//! since it's a self-contained feature with no other handler depending on it.

use super::handlers::{get_i18n, load_project_kanban_columns, redirect_error, redirect_notice, resolve_project};
use crate::app::pages::template_library_page::{TemplateDetailPage, TemplateGalleryPage, TemplateShareDisplay};
use crate::auth::AuthUser;
use crate::error::{WebError, WebResult};
use crate::services::{template_library, KnowledgeSyncService};
use crate::state::AppState;
use apich_db::{CreateTemplateDto, IdentityPermissionResolver, PublishTemplateVersionDto};
use axum::{
    extract::{Form, Path, Query, State},
    http::HeaderMap,
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

    let mut shares = Vec::new();
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
    }

    let current_path = format!("/templates/{id}");
    let html = crate::app::components::render_document(move || {
        leptos::prelude::view! {
            <TemplateDetailPage
                user=user
                is_org_or_team_admin=is_org_admin
                template=template
                versions=versions
                shares=shares
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
    pub org_slug: String,
    pub team_slug: Option<String>,
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
    let org_slug = payload.org_slug.trim();
    let Some(org) = repo.get_organization_by_slug(org_slug).await.unwrap_or(None) else {
        return redirect_error(&format!("/templates/{id}"), format!("No org found with slug '{org_slug}'"));
    };

    let team_slug = payload.team_slug.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let team_id = if let Some(team_slug) = team_slug {
        let teams = repo.list_teams_by_org(org.id).await.unwrap_or_default();
        match teams.into_iter().find(|t| t.slug == team_slug) {
            Some(t) => Some(t.id),
            None => return redirect_error(&format!("/templates/{id}"), format!("No team '{team_slug}' found in org '{org_slug}'")),
        }
    } else {
        None
    };

    let (org_id, team_id) = if team_id.is_some() { (None, team_id) } else { (Some(org.id), None) };
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
