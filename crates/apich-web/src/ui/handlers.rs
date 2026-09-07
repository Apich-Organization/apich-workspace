use super::i18n::{resolve_language, I18n, Lang};
use super::views;
use crate::auth::{
    build_clear_cookie, build_session_cookie, generate_session_token, hash_password,
    hash_session_token, verify_password, AuthUser,
};
use crate::error::WebError;
use crate::state::AppState;
use apich_db::{CreateProjectDto, CreateUserDto, IdentityPermissionResolver, Project, TeamTreeNode};
use axum::{
    extract::{Form, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Router,
};
use chrono::Utc;
use serde::Deserialize;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct SetLangQuery {
    pub lang: String,
    pub return_to: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FormLogin {
    pub login: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct FormRegister {
    pub username: String,
    pub display_name: String,
    pub email: String,
    pub password: String,
    pub invite_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct NewProjectForm {
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SnapshotForm {
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct MergeForm {
    pub branch: String,
}

#[derive(Debug, Deserialize)]
pub struct ResolveConflictForm {
    pub file: String,
    pub choice: String,
    pub custom_content: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GitSyncForm {
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct NewTeamForm {
    pub org_id: Uuid,
    pub parent_team_id: Option<Uuid>,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddOrgMemberForm {
    pub org_id: Uuid,
    pub user_id_or_username: String,
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct RemoveOrgMemberForm {
    pub org_id: Uuid,
    pub user_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct AddTeamMemberForm {
    pub team_id: Uuid,
    pub user_id_or_username: String,
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct RemoveTeamMemberForm {
    pub team_id: Uuid,
    pub user_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct AddProjectMemberForm {
    pub user_id_or_username: String,
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct RemoveProjectMemberForm {
    pub user_id: Uuid,
}

/// Helper to get current active language
fn get_i18n(headers: &HeaderMap, params: Option<&HashMap<String, String>>) -> I18n {
    let cookie_str = headers.get(header::COOKIE).and_then(|h| h.to_str().ok());
    let accept_lang = headers.get(header::ACCEPT_LANGUAGE).and_then(|h| h.to_str().ok());
    let lang = resolve_language(params, cookie_str, accept_lang);
    I18n::new(lang)
}

/// Helper to resolve user ID by UUID string, username, or email
async fn resolve_user_id(repo: &apich_db::Repository<'_>, id_or_name: &str) -> Option<Uuid> {
    let clean = id_or_name.trim();
    if let Ok(u) = Uuid::parse_str(clean) {
        return Some(u);
    }
    if let Ok(Some(u)) = repo.get_user_by_username(clean).await {
        return Some(u.id);
    }
    if let Ok(Some(u)) = repo.get_user_by_email(clean).await {
        return Some(u.id);
    }
    None
}

/// Helper to build redirect with notice query parameter
fn redirect_notice(path: &str, msg: &str) -> Response {
    let encoded = urlencoding::encode(msg);
    let sep = if path.contains('?') { "&" } else { "?" };
    Redirect::to(&format!("{}{}notice={}", path, sep, encoded)).into_response()
}

/// Helper to build redirect with error query parameter
fn redirect_error(path: &str, err: impl std::fmt::Display) -> Response {
    let err_str = err.to_string();
    let encoded = urlencoding::encode(&err_str);
    let sep = if path.contains('?') { "&" } else { "?" };
    Redirect::to(&format!("{}{}error={}", path, sep, encoded)).into_response()
}

/// Build the top-level UI router providing seamless browser navigation and admin controls
pub fn build_ui_router() -> Router<AppState> {
    Router::new()
        // Language switch endpoint
        .route("/set-lang", get(set_language_action))
        // Authentication pages & redirects
        .route("/", get(dashboard_page))
        .route("/login", get(login_page).post(login_form))
        .route("/register", get(register_page).post(register_form))
        .route("/logout", get(logout_action).post(logout_action))
        // Project workspace & VCS merge operations
        .route("/projects/new", post(create_project_form))
        .route("/projects/:id", get(project_detail_page))
        .route("/projects/:id/sandbox/start", post(start_sandbox_action))
        .route("/projects/:id/sandbox/stop", post(stop_sandbox_action))
        .route("/projects/:id/snapshot", post(snapshot_action))
        .route("/projects/:id/merge", post(merge_action))
        .route("/projects/:id/resolve-conflict", post(resolve_conflict_action))
        .route("/projects/:id/git-sync", post(git_sync_action))
        .route("/projects/:id/members/add", post(add_project_member_form))
        .route("/projects/:id/members/remove", post(remove_project_member_form))
        // Settings & Profile
        .route("/settings", get(settings_page))
        .route("/settings/profile", post(update_profile_form))
        .route("/settings/password", post(change_password_form))
        // Admin Organizations & Teams
        .route("/admin/orgs", get(admin_orgs_page))
        .route("/admin/orgs/new", post(create_org_form))
        .route("/admin/orgs/edit", post(update_org_form))
        .route("/admin/orgs/delete", post(delete_org_form))
        .route("/admin/teams/new", post(create_team_form))
        .route("/admin/teams/edit", post(update_team_form))
        .route("/admin/teams/delete", post(delete_team_form))
        .route("/admin/orgs/members/add", post(add_org_member_form))
        .route("/admin/orgs/members/remove", post(remove_org_member_form))
        .route("/admin/teams/members/add", post(add_team_member_form))
        .route("/admin/teams/members/remove", post(remove_team_member_form))
        // Admin Platform & Outbound Mail Settings
        .route("/admin/platform", get(admin_platform_page))
        .route("/admin/platform/settings", post(update_platform_settings_form))
        .route("/admin/platform/smtp-test", post(test_smtp_form))
}


/// Set language action: sets cookie and redirects back
async fn set_language_action(
    Query(query): Query<SetLangQuery>,
) -> Response {
    let lang = Lang::parse(&query.lang).code();
    let cookie = format!("apich_lang={}; Path=/; Max-Age=31536000; SameSite=Lax", lang);
    let target = query.return_to.unwrap_or_else(|| "/".to_string());
    ([(header::SET_COOKIE, cookie)], Redirect::to(&target)).into_response()
}

/// Dashboard Page: Unauthenticated users are strictly redirected to /login!
async fn dashboard_page(
    auth: Option<AuthUser>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Response {
    let AuthUser(user) = match auth {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };

    let i18n = get_i18n(&headers, Some(&params));
    let repo = state.db.repository();
    let projects = repo.list_projects_for_user(user.id).await.unwrap_or_default();

    // Query real active sandbox count for current user
    let mut running_count = 0;
    for proj in &projects {
        if let Ok(Some(sb)) = repo.get_project_sandbox(proj.id, user.id).await {
            if sb.status == "running" {
                running_count += 1;
            }
        }
    }

    let html = views::render_dashboard(&user, &projects, running_count, &i18n, "/");
    Html(html).into_response()
}

/// Login Page: Authenticated users are redirected to /
async fn login_page(
    auth: Option<AuthUser>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    if auth.is_some() {
        return Redirect::to("/").into_response();
    }

    let i18n = get_i18n(&headers, Some(&params));
    let error = params.get("error").map(|s| s.as_str());
    let success = params.get("success").map(|s| s.as_str());

    let html = views::render_login_page(error, success, &i18n, "/login");
    Html(html).into_response()
}

/// Handle Form-based Login
async fn login_form(
    State(state): State<AppState>,
    Form(payload): Form<FormLogin>,
) -> Response {
    let repo = state.db.repository();
    let user = if payload.login.contains('@') {
        repo.get_user_by_email(&payload.login).await.unwrap_or(None)
    } else {
        repo.get_user_by_username(&payload.login).await.unwrap_or(None)
    };

    let user = match user {
        Some(u) => u,
        None => {
            return Redirect::to("/login?error=Invalid username or password").into_response();
        }
    };

    if !verify_password(&payload.password, &user.password_hash).unwrap_or(false) {
        return Redirect::to("/login?error=Invalid username or password").into_response();
    }

    let token = generate_session_token();
    let token_hash = hash_session_token(&token);
    let expires_at = Utc::now() + chrono::Duration::days(14);

    if let Err(e) = repo.create_user_session(user.id, &token_hash, expires_at, None, None).await {
        tracing::error!("Failed to create user session: {}", e);
        return Redirect::to("/login?error=Failed to initialize login session").into_response();
    }

    let cookie = build_session_cookie(&token, 14 * 86400);
    ([(header::SET_COOKIE, cookie)], Redirect::to("/")).into_response()
}

/// Register Page: Authenticated users redirected to /
async fn register_page(
    auth: Option<AuthUser>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    if auth.is_some() {
        return Redirect::to("/").into_response();
    }

    let i18n = get_i18n(&headers, Some(&params));
    let error = params.get("error").map(|s| s.as_str());
    let html = views::render_register_page(error, &i18n, "/register");
    Html(html).into_response()
}

/// Handle Form-based Register
async fn register_form(
    State(state): State<AppState>,
    Form(payload): Form<FormRegister>,
) -> Response {
    let repo = state.db.repository();
    let settings = repo.get_system_settings().await.unwrap_or_default();

    if settings.registration_mode == "admin_only" {
        return Redirect::to("/register?error=Self-serve registration is disabled").into_response();
    }

    if settings.registration_mode == "invite_only" {
        let token = match payload.invite_token.as_deref().filter(|t| !t.trim().is_empty()) {
            Some(t) => t.trim(),
            None => return Redirect::to("/register?error=Invitation code required").into_response(),
        };
        let invite = match repo.get_invitation_by_token(token).await.ok().flatten() {
            Some(inv) => inv,
            None => return Redirect::to("/register?error=Invalid or expired invitation").into_response(),
        };
        if invite.email.to_lowercase() != payload.email.to_lowercase() {
            return Redirect::to("/register?error=Email mismatch with invitation").into_response();
        }
        let _ = repo.mark_invitation_used(&invite.token).await;
    }

    let pwd_hash = match hash_password(&payload.password) {
        Ok(h) => h,
        Err(_) => return Redirect::to("/register?error=Password hashing failed").into_response(),
    };

    let user = repo.create_user(CreateUserDto {
        username: payload.username,
        email: payload.email,
        password_hash: pwd_hash,
        display_name: payload.display_name,
        role: None,
        is_platform_admin: None,
        storage_quota_bytes: None,
    }).await;

    match user {
        Ok(u) => {
            let token = generate_session_token();
            let token_hash = hash_session_token(&token);
            let expires_at = Utc::now() + chrono::Duration::days(14);
            let _ = repo.create_user_session(u.id, &token_hash, expires_at, None, None).await;
            let cookie = build_session_cookie(&token, 14 * 86400);
            ([(header::SET_COOKIE, cookie)], Redirect::to("/")).into_response()
        }
        Err(e) => {
            let msg = if e.to_string().contains("duplicate") {
                "Username or email already in use"
            } else {
                "Failed to create account"
            };
            Redirect::to(&format!("/register?error={}", urlencoding::encode(msg))).into_response()
        }
    }
}

/// Logout Action: Clears session cookie and redirects with 303 to /login!
async fn logout_action(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
) -> Response {
    if let Some(AuthUser(user)) = auth {
        let _ = sqlx::query("DELETE FROM user_sessions WHERE user_id = $1")
            .bind(user.id)
            .execute(state.db.pool())
            .await;
    }

    let clear_cookie = build_clear_cookie();
    (
        [(header::SET_COOKIE, clear_cookie)],
        Redirect::to("/login"),
    )
        .into_response()
}

/// Helper to find project by UUID string or Slug
async fn resolve_project(state: &AppState, id_or_slug: &str) -> Option<Project> {
    let repo = state.db.repository();
    if let Ok(u) = Uuid::parse_str(id_or_slug) {
        if let Ok(Some(p)) = repo.get_project_by_id(u).await {
            return Some(p);
        }
    }

    if let Ok(projects) = repo.list_all_projects_admin().await {
        if let Some(p) = projects.into_iter().find(|p| p.slug == id_or_slug) {
            return Some(p);
        }
    }

    None
}

/// Project Detail Page: Sandboxes, Merge & Conflicts, Snapshots, Git, Collaborators
async fn project_detail_page(
    auth: Option<AuthUser>,
    headers: HeaderMap,
    Path(id_or_slug): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Response {
    let AuthUser(user) = match auth {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };

    let i18n = get_i18n(&headers, Some(&params));

    let project = match resolve_project(&state, &id_or_slug).await {
        Some(p) => p,
        None => return (StatusCode::NOT_FOUND, Html("<h3>Project not found 404</h3><p><a href='/'>Return to dashboard</a></p>")).into_response(),
    };

    let can_access = IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id)
        .await
        .unwrap_or(false);

    if !can_access {
        return (StatusCode::FORBIDDEN, Html("<h3>403 Access Forbidden: You do not have permissions for this project</h3><p><a href='/'>Return to dashboard</a></p>")).into_response();
    }

    let repo = state.db.repository();
    let sb = repo
        .get_project_sandbox(project.id, user.id)
        .await
        .unwrap_or(None);
    let sandbox_status = sb.map(|s| s.status).unwrap_or_else(|| "stopped".to_string());

    let (current_branch, branches) = state
        .project_manager
        .get_branches(project.id)
        .await
        .unwrap_or((Some("main".to_string()), vec!["main".to_string()]));

    let conflicts = state
        .project_manager
        .detect_conflicts(project.id)
        .await
        .unwrap_or_default();

    let snapshots = state
        .project_manager
        .get_project_timeline(project.id)
        .await
        .unwrap_or_default();

    let members = repo
        .list_project_members(project.id)
        .await
        .unwrap_or_default();

    let all_users = repo.list_users().await.unwrap_or_default();

    let git_status = state
        .project_manager
        .get_git_status(project.id)
        .await
        .unwrap_or(crate::services::project_manager::GitStatusView {
            initialized: false,
            remote_url: None,
            branches: vec!["main".to_string()],
            current_branch: Some("main".to_string()),
        });

    let active_tab = params.get("tab").map(|s| s.as_str()).unwrap_or("overview");
    let notice = params.get("notice").map(|s| match s.as_str() {
        "reconciled" => "Branch merge complete. Any conflicts are marked below.",
        "conflict_resolved" => "Merge conflict successfully resolved and snapshot recorded.",
        "git_synced" => "Exported and committed to local Git repository.",
        "collaborator_added" => "Collaborator added successfully.",
        "collaborator_removed" => "Collaborator removed.",
        _ => s.as_str(),
    });

    let current_path = format!("/projects/{}?tab={}", id_or_slug, active_tab);
    let html = views::render_project_detail_page(
        &user,
        &project,
        &sandbox_status,
        &branches,
        current_branch.as_deref(),
        &conflicts,
        &snapshots,
        &members,
        &all_users,
        &git_status,
        active_tab,
        notice,
        &i18n,
        &current_path,
    );

    Html(html).into_response()
}

/// Create new project from modal form
async fn create_project_form(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Form(payload): Form<NewProjectForm>,
) -> Result<Response, WebError> {
    let repo = state.db.repository();
    let orgs = repo.list_organizations_for_user(user.id).await?;
    let org_id = orgs.first().map(|o| o.id).unwrap_or_else(Uuid::now_v7);

    let proj = state.project_manager.create_project(CreateProjectDto {
        org_id,
        team_id: None,
        owner_id: user.id,
        name: payload.name,
        slug: payload.slug,
        description: payload.description,
        storage_path: String::new(),
        settings: None,
    }).await?;

    Ok(Redirect::to(&format!("/projects/{}", proj.id)).into_response())
}

/// Start container sandbox from web UI
async fn start_sandbox_action(
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Response, WebError> {
    let _ = state.project_manager.launch_sandbox(id, user.id).await?;
    Ok(Redirect::to(&format!("/projects/{}", id)).into_response())
}

/// Stop container sandbox from web UI
async fn stop_sandbox_action(
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Response, WebError> {
    let _ = state.project_manager.stop_sandbox(id, user.id).await?;
    Ok(Redirect::to(&format!("/projects/{}", id)).into_response())
}

/// Snapshot from web UI
async fn snapshot_action(
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Form(payload): Form<SnapshotForm>,
) -> Result<Response, WebError> {
    let _ = state.project_manager.snapshot_project(id, user.id, &payload.message).await?;
    Ok(Redirect::to(&format!("/projects/{}?tab=timeline", id)).into_response())
}

/// Weave-free merge from web UI
async fn merge_action(
    AuthUser(_user): AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Form(payload): Form<MergeForm>,
) -> Result<Response, WebError> {
    let _ = state.project_manager.merge_branch(id, &payload.branch).await?;
    Ok(Redirect::to(&format!("/projects/{}?tab=merge&notice=reconciled", id)).into_response())
}

/// Resolve conflict from web UI
async fn resolve_conflict_action(
    AuthUser(_user): AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Form(payload): Form<ResolveConflictForm>,
) -> Result<Response, WebError> {
    state.project_manager.resolve_conflict(
        id,
        &payload.file,
        &payload.choice,
        payload.custom_content.as_deref(),
    ).await?;

    Ok(Redirect::to(&format!("/projects/{}?tab=merge&notice=conflict_resolved", id)).into_response())
}

/// Git sync from web UI
async fn git_sync_action(
    AuthUser(_user): AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Form(payload): Form<GitSyncForm>,
) -> Result<Response, WebError> {
    let _ = state.project_manager.git_sync(id, &payload.message).await?;
    Ok(Redirect::to(&format!("/projects/{}?tab=git&notice=git_synced", id)).into_response())
}

/// Add collaborator to project
async fn add_project_member_form(
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Form(payload): Form<AddProjectMemberForm>,
) -> Result<Response, WebError> {
    let repo = state.db.repository();
    let proj = repo.get_project_by_id(id).await?.ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

    if proj.owner_id != user.id && !user.is_platform_admin {
        return Err(WebError::Forbidden("Only owner or admin can add collaborators".to_string()));
    }

    let target_uid = resolve_user_id(&repo, &payload.user_id_or_username)
        .await
        .ok_or_else(|| WebError::BadRequest("Target user not found".to_string()))?;

    let role = match payload.role.as_str() {
        "viewer" => "viewer",
        _ => "editor",
    };

    repo.add_project_member(id, target_uid, role).await?;
    Ok(Redirect::to(&format!("/projects/{}?tab=members&notice=collaborator_added", id)).into_response())
}

/// Remove collaborator from project
async fn remove_project_member_form(
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Form(payload): Form<RemoveProjectMemberForm>,
) -> Result<Response, WebError> {
    let repo = state.db.repository();
    let proj = repo.get_project_by_id(id).await?.ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

    if proj.owner_id != user.id && !user.is_platform_admin {
        return Err(WebError::Forbidden("Only owner or admin can remove collaborators".to_string()));
    }

    repo.remove_project_member(id, payload.user_id).await?;
    Ok(Redirect::to(&format!("/projects/{}?tab=members&notice=collaborator_removed", id)).into_response())
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfileForm {
    pub display_name: String,
    pub email: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordForm {
    pub current_password: String,
    pub new_password: String,
    pub confirm_password: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateOrgForm {
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateOrgForm {
    pub org_id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteOrgForm {
    pub org_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTeamForm {
    pub team_id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub parent_team_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteTeamForm {
    pub team_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePlatformSettingsForm {
    pub section: Option<String>,
    pub registration_mode: Option<String>,
    pub smtp_enabled: Option<String>,
    pub smtp_host: Option<String>,
    pub smtp_port: Option<i32>,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
    pub smtp_from_email: Option<String>,
    pub smtp_from_name: Option<String>,
    pub smtp_use_tls: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TestSmtpForm {
    pub test_email: String,
}

/// Settings Page
async fn settings_page(
    auth: Option<AuthUser>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let AuthUser(user) = match auth {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };

    let i18n = get_i18n(&headers, Some(&params));
    let notice = params.get("notice").map(|s| s.as_str());
    let error = params.get("error").map(|s| s.as_str());
    let html = views::render_settings_page(&user, notice, error, &i18n, "/settings");
    Html(html).into_response()
}

/// Update user profile
async fn update_profile_form(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Form(payload): Form<UpdateProfileForm>,
) -> Response {
    let AuthUser(user) = match auth {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };

    let avatar = payload.avatar_url.filter(|s| !s.trim().is_empty());
    let dto = apich_db::UpdateUserProfileDto {
        display_name: Some(payload.display_name.trim().to_string()),
        email: Some(payload.email.trim().to_string()),
        avatar_url: avatar,
    };

    match state.identity_service.update_user_profile(user.id, dto).await {
        Ok(_) => redirect_notice("/settings", "Profile updated successfully"),
        Err(e) => redirect_error("/settings", e),
    }
}

/// Change password
async fn change_password_form(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Form(payload): Form<ChangePasswordForm>,
) -> Response {
    let AuthUser(user) = match auth {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };

    if payload.new_password != payload.confirm_password {
        return redirect_error("/settings", "New password and confirm password do not match");
    }

    match state
        .identity_service
        .change_user_password(user.id, &payload.current_password, &payload.new_password)
        .await
    {
        Ok(_) => redirect_notice("/settings", "Password changed successfully"),
        Err(e) => redirect_error("/settings", e),
    }
}

/// Admin Organizations Page with full member and team management
async fn admin_orgs_page(
    auth: Option<AuthUser>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Response {
    let AuthUser(user) = match auth {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };

    let i18n = get_i18n(&headers, Some(&params));
    let repo = state.db.repository();
    let orgs = repo.list_organizations(100).await.unwrap_or_default();
    let all_users = repo.list_users().await.unwrap_or_default();

    let mut org_members_map = HashMap::new();
    let mut team_trees_map = HashMap::new();
    let mut team_members_map = HashMap::new();

    fn collect_team_ids(nodes: &[TeamTreeNode], acc: &mut Vec<Uuid>) {
        for n in nodes {
            acc.push(n.team.id);
            collect_team_ids(&n.children, acc);
        }
    }

    for org in &orgs {
        let members = repo.list_org_members(org.id).await.unwrap_or_default();
        org_members_map.insert(org.id, members);

        let trees = repo.get_team_tree_for_org(org.id).await.unwrap_or_default();
        let mut team_ids = Vec::new();
        collect_team_ids(&trees, &mut team_ids);

        for tid in team_ids {
            let t_members = repo.list_team_members(tid).await.unwrap_or_default();
            team_members_map.insert(tid, t_members);
        }

        team_trees_map.insert(org.id, trees);
    }

    let notice = params.get("notice").map(|s| match s.as_str() {
        "org_created" => "Organization created successfully.",
        "org_updated" => "Organization updated successfully.",
        "org_deleted" => "Organization deleted successfully.",
        "team_created" => "Team created successfully.",
        "team_updated" => "Team updated successfully.",
        "team_deleted" => "Team deleted successfully.",
        "member_added" => "Member added to organization.",
        "member_removed" => "Member removed from organization.",
        "team_member_added" => "Member added to team.",
        "team_member_removed" => "Member removed from team.",
        _ => s.as_str(),
    });
    let error = params.get("error").map(|s| s.as_str());

    let html = views::render_admin_orgs_page(
        &user,
        &orgs,
        &all_users,
        &org_members_map,
        &team_trees_map,
        &team_members_map,
        notice,
        error,
        &i18n,
        "/admin/orgs",
    );

    Html(html).into_response()
}

/// Create organization
async fn create_org_form(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Form(payload): Form<CreateOrgForm>,
) -> Response {
    let AuthUser(user) = match auth {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };

    let dto = apich_db::CreateOrganizationDto {
        slug: payload.slug.trim().to_lowercase(),
        name: payload.name.trim().to_string(),
        description: payload.description.filter(|s| !s.trim().is_empty()),
    };

    match state.identity_service.create_organization(user.id, dto).await {
        Ok(_) => redirect_notice("/admin/orgs", "org_created"),
        Err(e) => redirect_error("/admin/orgs", e),
    }
}

/// Update organization
async fn update_org_form(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Form(payload): Form<UpdateOrgForm>,
) -> Response {
    let AuthUser(user) = match auth {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };

    let dto = apich_db::UpdateOrganizationDto {
        name: Some(payload.name.trim().to_string()),
        slug: Some(payload.slug.trim().to_lowercase()),
        description: payload.description.filter(|s| !s.trim().is_empty()),
    };

    match state.identity_service.update_organization(user.id, payload.org_id, dto).await {
        Ok(_) => redirect_notice("/admin/orgs", "org_updated"),
        Err(e) => redirect_error("/admin/orgs", e),
    }
}

/// Delete organization
async fn delete_org_form(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Form(payload): Form<DeleteOrgForm>,
) -> Response {
    let AuthUser(user) = match auth {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };

    match state.identity_service.delete_organization(user.id, payload.org_id).await {
        Ok(_) => redirect_notice("/admin/orgs", "org_deleted"),
        Err(e) => redirect_error("/admin/orgs", e),
    }
}

/// Create new team in organization
async fn create_team_form(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Form(payload): Form<NewTeamForm>,
) -> Response {
    let AuthUser(user) = match auth {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };

    match state.identity_service.create_team(user.id, apich_db::CreateTeamDto {
        org_id: payload.org_id,
        parent_team_id: payload.parent_team_id,
        name: payload.name.trim().to_string(),
        slug: payload.slug.trim().to_lowercase(),
        description: payload.description.filter(|s| !s.trim().is_empty()),
    }).await {
        Ok(_) => redirect_notice("/admin/orgs", "team_created"),
        Err(e) => redirect_error("/admin/orgs", e),
    }
}

/// Update existing team
async fn update_team_form(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Form(payload): Form<UpdateTeamForm>,
) -> Response {
    let AuthUser(user) = match auth {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };

    let parent_id = match payload.parent_team_id {
        Some(ref s) if !s.trim().is_empty() => match Uuid::parse_str(s.trim()) {
            Ok(id) => Some(Some(id)),
            Err(_) => None,
        },
        Some(_) => Some(None), // cleared to root
        None => None,
    };

    let dto = apich_db::UpdateTeamDto {
        name: Some(payload.name.trim().to_string()),
        slug: Some(payload.slug.trim().to_lowercase()),
        description: payload.description.filter(|s| !s.trim().is_empty()),
        parent_team_id: parent_id,
    };

    match state.identity_service.update_team(user.id, payload.team_id, dto).await {
        Ok(_) => redirect_notice("/admin/orgs", "team_updated"),
        Err(e) => redirect_error("/admin/orgs", e),
    }
}

/// Delete team
async fn delete_team_form(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Form(payload): Form<DeleteTeamForm>,
) -> Response {
    let AuthUser(user) = match auth {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };

    match state.identity_service.delete_team(user.id, payload.team_id).await {
        Ok(_) => redirect_notice("/admin/orgs", "team_deleted"),
        Err(e) => redirect_error("/admin/orgs", e),
    }
}

/// Add member to organization
async fn add_org_member_form(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Form(payload): Form<AddOrgMemberForm>,
) -> Response {
    let repo = state.db.repository();
    let target_uid = match resolve_user_id(&repo, &payload.user_id_or_username).await {
        Some(id) => id,
        None => return redirect_error("/admin/orgs", "Target user not found"),
    };

    match state.identity_service.add_org_member(user.id, payload.org_id, target_uid, &payload.role).await {
        Ok(_) => redirect_notice("/admin/orgs", "member_added"),
        Err(e) => redirect_error("/admin/orgs", e),
    }
}

/// Remove member from organization
async fn remove_org_member_form(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Form(payload): Form<RemoveOrgMemberForm>,
) -> Response {
    match state.identity_service.remove_org_member(user.id, payload.org_id, payload.user_id).await {
        Ok(_) => redirect_notice("/admin/orgs", "member_removed"),
        Err(e) => redirect_error("/admin/orgs", e),
    }
}

/// Add member to team
async fn add_team_member_form(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Form(payload): Form<AddTeamMemberForm>,
) -> Response {
    let repo = state.db.repository();
    let target_uid = match resolve_user_id(&repo, &payload.user_id_or_username).await {
        Some(id) => id,
        None => return redirect_error("/admin/orgs", "Target user not found"),
    };

    match state.identity_service.add_team_member(user.id, payload.team_id, target_uid, &payload.role).await {
        Ok(_) => redirect_notice("/admin/orgs", "team_member_added"),
        Err(e) => redirect_error("/admin/orgs", e),
    }
}

/// Remove member from team
async fn remove_team_member_form(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Form(payload): Form<RemoveTeamMemberForm>,
) -> Response {
    match state.identity_service.remove_team_member(user.id, payload.team_id, payload.user_id).await {
        Ok(_) => redirect_notice("/admin/orgs", "team_member_removed"),
        Err(e) => redirect_error("/admin/orgs", e),
    }
}

/// Admin Platform Page
async fn admin_platform_page(
    auth: Option<AuthUser>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Response {
    let AuthUser(user) = match auth {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };

    if !user.is_platform_admin {
        return (StatusCode::FORBIDDEN, Html("<h3>403 Access Denied: Administrator access required</h3><p><a href='/'>Return to dashboard</a></p>")).into_response();
    }

    let i18n = get_i18n(&headers, Some(&params));
    let repo = state.db.repository();
    let settings = repo.get_system_settings().await.unwrap_or_default();
    let notice = params.get("notice").map(|s| s.as_str());
    let error = params.get("error").map(|s| s.as_str());

    let html = views::render_admin_platform_page(&user, &settings, notice, error, &i18n, "/admin/platform");
    Html(html).into_response()
}

/// Update platform registration or SMTP settings
async fn update_platform_settings_form(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Form(payload): Form<UpdatePlatformSettingsForm>,
) -> Response {
    let AuthUser(user) = match auth {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };

    if !user.is_platform_admin {
        return (StatusCode::FORBIDDEN, "Administrator access required").into_response();
    }

    let is_smtp = payload.section.as_deref() == Some("smtp");
    let smtp_enabled = if is_smtp {
        Some(payload.smtp_enabled.as_deref() == Some("true") || payload.smtp_enabled.as_deref() == Some("on"))
    } else {
        None
    };
    let smtp_use_tls = if is_smtp {
        Some(payload.smtp_use_tls.as_deref() == Some("true") || payload.smtp_use_tls.as_deref() == Some("on"))
    } else {
        None
    };

    let pwd = payload.smtp_password.filter(|s| !s.trim().is_empty());

    let dto = apich_db::UpdateSystemSettingsDto {
        registration_mode: payload.registration_mode.filter(|s| !s.trim().is_empty()),
        smtp_host: payload.smtp_host.filter(|s| !s.trim().is_empty()),
        smtp_port: payload.smtp_port,
        smtp_username: payload.smtp_username.filter(|s| !s.trim().is_empty()),
        smtp_password: pwd,
        smtp_from_email: payload.smtp_from_email.filter(|s| !s.trim().is_empty()),
        smtp_from_name: payload.smtp_from_name.filter(|s| !s.trim().is_empty()),
        smtp_use_tls,
        smtp_enabled,
    };

    match state.db.repository().update_system_settings(dto).await {
        Ok(_) => redirect_notice("/admin/platform", "Configuration updated successfully"),
        Err(e) => redirect_error("/admin/platform", e),
    }
}

/// Test SMTP delivery
async fn test_smtp_form(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Form(payload): Form<TestSmtpForm>,
) -> Response {
    let AuthUser(user) = match auth {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };

    if !user.is_platform_admin {
        return (StatusCode::FORBIDDEN, "Administrator access required").into_response();
    }

    let settings = match state.db.repository().get_system_settings().await {
        Ok(s) => s,
        Err(e) => return redirect_error("/admin/platform", format!("Failed to read settings: {}", e)),
    };

    match state.mailer.send_test_email(&settings, &payload.test_email).await {
        Ok(_) => redirect_notice(
            "/admin/platform",
            &format!("Verification email dispatched successfully to {}", payload.test_email),
        ),
        Err(e) => redirect_error("/admin/platform", format!("SMTP delivery error: {}", e)),
    }
}

