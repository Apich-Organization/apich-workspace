use super::i18n::resolve_language;
use super::i18n::I18n;
use super::i18n::Lang;
use crate::auth::build_clear_cookie;
use crate::auth::build_session_cookie;
use crate::auth::generate_session_token;
use crate::auth::hash_password;
use crate::auth::hash_session_token;
use crate::auth::verify_password;
use crate::auth::AuthUser;
use crate::error::WebError;
use crate::services::KnowledgeSyncService;
use crate::services::SqliteTableService;
use crate::state::AppState;
use apich_db::CreateProjectDto;
use apich_db::CreateUserDto;
use apich_db::IdentityPermissionResolver;
use apich_db::Project;
use apich_vcs::api::ProjectVcs;
use axum::extract::Form;
use axum::extract::Json;
use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::http::header;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::Html;
use axum::response::IntoResponse;
use axum::response::Redirect;
use axum::response::Response;
use axum::routing::get;
use axum::routing::post;
use axum::Router;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
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
    pub return_to: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FormRegister {
    pub username: String,
    pub display_name: String,
    pub email: String,
    pub password: String,
    pub invite_token: Option<String>,
    pub create_org: Option<String>,
    pub org_name: Option<String>,
    pub org_slug: Option<String>,
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
    pub chat_url: Option<String>,
    pub meeting_url: Option<String>,
    pub drive_url: Option<String>,
    pub ai_agent_url: Option<String>,
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
pub(crate) fn get_i18n(
    headers: &HeaderMap,
    params: Option<&HashMap<String, String>>,
) -> I18n {
    let cookie_str = headers.get(header::COOKIE).and_then(|h| h.to_str().ok());
    let accept_lang = headers
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|h| h.to_str().ok());
    let lang = resolve_language(params, cookie_str, accept_lang);
    I18n::new(lang)
}

/// Helper to resolve user ID by UUID string, username, or email
async fn resolve_user_id(
    repo: &apich_db::Repository<'_>,
    id_or_name: &str,
) -> Option<Uuid> {
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
pub(crate) fn redirect_notice(
    path: &str,
    msg: &str,
) -> Response {
    let encoded = urlencoding::encode(msg);
    let sep = if path.contains('?') {
        "&"
    } else {
        "?"
    };
    Redirect::to(&format!("{}{}notice={}", path, sep, encoded)).into_response()
}

/// Helper to build redirect with error query parameter
pub(crate) fn redirect_error(
    path: &str,
    err: impl std::fmt::Display,
) -> Response {
    let err_str = err.to_string();
    let encoded = urlencoding::encode(&err_str);
    let sep = if path.contains('?') {
        "&"
    } else {
        "?"
    };
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
        // Anonymous public-link project viewer (no auth required)
        .route("/shared/:id", get(shared_project_page))
        // Demo project creation & seeding
        .route("/projects/demo/create", post(create_demo_project_action))
        .route("/projects/:id/demo/seed", post(seed_demo_files_action))
        // Project workspace & VCS merge operations
        .route("/projects/new", post(create_project_form))
        .route("/projects/:id", get(project_detail_page))
        .route("/projects/:id/files/new", post(create_file_action))
        .route("/projects/:id/files/delete", post(delete_file_action))
        .route("/projects/:id/files/raw", get(file_raw_action))
        .route("/projects/:id/sandbox/start", post(start_sandbox_action))
        .route("/projects/:id/sandbox/stop", post(stop_sandbox_action))
        .route("/projects/:id/snapshot", post(snapshot_action))
        .route("/projects/:id/merge", post(merge_action))
        .route("/projects/:id/branch-create", post(branch_create_action))
        .route("/projects/:id/branch-switch", post(branch_switch_action))
        .route(
            "/projects/:id/milestone-create",
            post(milestone_create_action),
        )
        .route("/projects/:id/vcs-undo", post(vcs_undo_action))
        .route("/projects/:id/vcs-redo", post(vcs_redo_action))
        .route(
            "/projects/:id/resolve-conflict",
            post(resolve_conflict_action),
        )
        .route("/projects/:id/git-sync", post(git_sync_action))
        .route("/projects/:id/git-remote-add", post(git_remote_add_action))
        .route("/projects/:id/git-fetch", post(git_fetch_action))
        .route("/projects/:id/git-pull", post(git_pull_action))
        .route("/projects/:id/git-push", post(git_push_action))
        .route("/projects/:id/git-rebase", post(git_rebase_action))
        .route(
            "/projects/:id/ignore/profile-toggle",
            post(ignore_profile_toggle_action),
        )
        .route(
            "/projects/:id/ignore/rule-add",
            post(ignore_rule_add_action),
        )
        .route(
            "/projects/:id/ignore/rule-remove",
            post(ignore_rule_remove_action),
        )
        .route(
            "/projects/:id/ignore/file-save",
            post(ignore_file_save_action),
        )
        .route(
            "/vcs-remote/:id/bundle",
            get(vcs_remote_bundle_get_action).post(vcs_remote_bundle_post_action),
        )
        .route(
            "/git/:id/*path",
            get(git_http_backend_action).post(git_http_backend_action),
        )
        .route("/projects/:id/members/add", post(add_project_member_form))
        .route(
            "/projects/:id/members/remove",
            post(remove_project_member_form),
        )
        // Dedicated Document & Slide Editor Studio
        .route("/projects/:id/editor", get(project_editor_page))
        .route("/projects/:id/editor/save", post(save_editor_file_action))
        .route("/projects/:id/editor/latex-pdf", get(latex_pdf_action))
        .route("/projects/:id/editor/latex-sync", post(latex_sync_action))
        .route("/projects/:id/editor/typst-pdf", get(typst_pdf_action))
        .route(
            "/projects/:id/editor/slide-binary/start",
            post(slide_binary_start_action),
        )
        .route(
            "/projects/:id/editor/slide-binary/status",
            get(slide_binary_status_action),
        )
        .route(
            "/projects/:id/editor/slide-binary/download",
            get(slide_binary_download_action),
        )
        .route("/projects/:id/script/run", post(run_script_action))
        .route("/projects/:id/files/share", post(share_file_action))
        .route("/projects/:id/delete", post(delete_project_action))
        .route(
            "/projects/:id/vigilant-mode",
            post(set_vigilant_mode_action),
        )
        .route(
            "/projects/:id/sharing/update",
            post(update_project_sharing_action),
        )
        .route(
            "/projects/:id/render/preview",
            post(render_doc_preview_action),
        )
        .route("/api/ai/chat", post(ai_chat_action))
        .route("/projects/:id/ai/chat", post(ai_chat_action))
        .route("/projects/:id/agent/status", get(agent_status_action))
        .route("/projects/:id/agent/run", post(agent_run_action))
        .route(
            "/projects/:id/agent/login/start",
            post(agent_login_start_action),
        )
        .route(
            "/projects/:id/agent/login/:session_id/status",
            get(agent_login_status_action),
        )
        .route(
            "/projects/:id/agent/login/:session_id/code",
            post(agent_login_submit_code_action),
        )
        // Project Tables (Spreadsheet & SQLite)
        .route("/projects/:id/table", get(project_table_page))
        .route("/projects/:id/table/sql", post(execute_table_sql_action))
        .route(
            "/projects/:id/table/create-db",
            post(create_table_db_action),
        )
        .route(
            "/projects/:id/table/cell-edit",
            post(table_cell_edit_action),
        )
        .route(
            "/projects/:id/table/cell-style",
            post(table_cell_style_action),
        )
        .route("/projects/:id/table/row-add", post(table_row_add_action))
        .route(
            "/projects/:id/table/row-delete",
            post(table_row_delete_action),
        )
        .route("/projects/:id/table/export", get(table_export_action))
        .route(
            "/projects/:id/table/column-view",
            post(table_column_view_action),
        )
        .route("/projects/:id/table/import", post(table_import_action))
        .route(
            "/projects/:id/table/notebook/cell-create",
            post(notebook_cell_create_action),
        )
        .route(
            "/projects/:id/table/notebook/cell-delete",
            post(notebook_cell_delete_action),
        )
        .route(
            "/projects/:id/table/notebook/cell-move",
            post(notebook_cell_move_action),
        )
        .route(
            "/projects/:id/table/notebook/run-cell",
            post(notebook_run_cell_action),
        )
        // Dedicated Unified Note Studio (.anote, .note.md, etc.)
        .route("/projects/:id/note", get(project_note_page))
        .route(
            "/projects/:id/note/create-page",
            post(create_note_page_action),
        )
        .route("/projects/:id/note/save", post(save_note_action))
        .route(
            "/projects/:id/note/whiteboard",
            post(save_whiteboard_action),
        )
        // Project Knowledge Hub (Tasks, Kanban, Wiki, Calendar)
        .route("/projects/:id/knowledge", get(project_knowledge_page))
        .route(
            "/projects/:id/knowledge/toggle-task",
            post(toggle_task_action),
        )
        .route(
            "/projects/:id/knowledge/toggle-task-ajax",
            post(toggle_task_ajax_action),
        )
        .route(
            "/projects/:id/knowledge/kanban/columns/add",
            post(kanban_add_column_action),
        )
        .route(
            "/projects/:id/knowledge/kanban/columns/rename",
            post(kanban_rename_column_action),
        )
        .route(
            "/projects/:id/knowledge/kanban/columns/delete",
            post(kanban_delete_column_action),
        )
        .route(
            "/projects/:id/knowledge/kanban/columns/move",
            post(kanban_move_column_action),
        )
        // Project Interactive Terminal
        .route("/projects/:id/terminal", get(project_terminal_page))
        .route("/projects/:id/terminal/exec", post(terminal_exec_action))
        // Settings & Profile
        .route("/settings", get(settings_page))
        .route("/settings/profile", post(update_profile_form))
        .route("/settings/password", post(change_password_form))
        .route("/settings/pat/create", post(create_pat_action))
        .route("/settings/pat/:id/revoke", post(revoke_pat_action))
        .route("/settings/ssh/add", post(add_ssh_key_action))
        .route("/settings/ssh/:id/delete", post(delete_ssh_key_action))
        .route("/settings/gpg/add", post(add_gpg_key_action))
        .route("/settings/gpg/:id/delete", post(delete_gpg_key_action))
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
        .route(
            "/admin/platform/settings",
            post(update_platform_settings_form),
        )
        .route("/admin/platform/smtp-test", post(test_smtp_form))
        // Template Library -- kept in its own module/router (template_handlers.rs) rather than
        // added inline here; this file is already large and the template library is a
        // self-contained feature with no other handler here depending on it.
        .merge(crate::ui::template_handlers::build_template_router())
}


/// Set language action: sets cookie and redirects back
async fn set_language_action(Query(query): Query<SetLangQuery>) -> Response {
    let lang = Lang::parse(&query.lang).code();
    let cookie = format!(
        "apich_lang={}; Path=/; Max-Age=31536000; SameSite=Lax",
        lang
    );
    let target = query.return_to.unwrap_or_else(|| "/".to_string());
    ([(header::SET_COOKIE, cookie)], Redirect::to(&target)).into_response()
}

/// Helper to build redirect to login with optional session_expired notice and return_to
fn redirect_to_login(
    headers: &HeaderMap,
    target_path: &str,
) -> Response {
    let has_session = headers
        .get(header::COOKIE)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.contains("apich_session="))
        .unwrap_or(false);
    if has_session {
        let encoded_return = urlencoding::encode(target_path);
        Redirect::to(&format!(
            "/login?notice=session_expired&return_to={}",
            encoded_return
        ))
        .into_response()
    } else if target_path == "/" {
        Redirect::to("/login").into_response()
    } else {
        let encoded_return = urlencoding::encode(target_path);
        Redirect::to(&format!("/login?return_to={}", encoded_return)).into_response()
    }
}

/// Dashboard Page: Unauthenticated users are strictly redirected to /login!
async fn dashboard_page(
    auth: Option<AuthUser>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return redirect_to_login(&headers, "/"),
    };

    let i18n = get_i18n(&headers, Some(&params));
    let repo = state.db.repository();
    let projects = repo
        .list_projects_for_user(user.id)
        .await
        .unwrap_or_default();

    let notice = params.get("notice").cloned();
    let error = params.get("error").cloned();
    let is_org_admin = state
        .identity_service
        .is_org_or_team_admin(user.id)
        .await
        .unwrap_or(false);

    let html = crate::app::components::render_document(move || {
        leptos::prelude::view! {
            <crate::app::pages::dashboard::DashboardPage
                user=user
                is_org_or_team_admin=is_org_admin
                projects=projects
                i18n=i18n
                current_path="/".to_string()
                notice=notice
                error=error
            />
        }
    });
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
    let error = params.get("error").cloned();
    let success = params.get("success").cloned();
    let notice = params.get("notice").cloned();
    let return_to = params
        .get("return_to")
        .cloned()
        .unwrap_or_else(|| "/".to_string());

    let html = crate::app::components::render_document(move || {
        leptos::prelude::view! {
            <crate::app::pages::login::LoginPage
                error=error
                success=success
                notice=notice
                return_to=return_to
                i18n=i18n
                current_path="/login".to_string()
            />
        }
    });
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
        repo.get_user_by_username(&payload.login)
            .await
            .unwrap_or(None)
    };

    let user = match user {
        | Some(u) => u,
        | None => {
            return Redirect::to("/login?error=Invalid username or password").into_response();
        },
    };

    if !verify_password(&payload.password, &user.password_hash).unwrap_or(false) {
        return Redirect::to("/login?error=Invalid username or password").into_response();
    }

    let token = generate_session_token();
    let token_hash = hash_session_token(&token);
    let expires_at = Utc::now() + chrono::Duration::days(14);

    if let Err(e) = repo
        .create_user_session(user.id, &token_hash, expires_at, None, None)
        .await
    {
        tracing::error!("Failed to create user session: {}", e);
        return Redirect::to("/login?error=Failed to initialize login session").into_response();
    }

    let target = payload
        .return_to
        .as_deref()
        .filter(|s| s.starts_with('/') && !s.starts_with("//"))
        .unwrap_or("/");

    let cookie = build_session_cookie(&token, 14 * 86400);
    ([(header::SET_COOKIE, cookie)], Redirect::to(target)).into_response()
}

/// Register Page: Authenticated users redirected to /
async fn register_page(
    auth: Option<AuthUser>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Response {
    if auth.is_some() {
        return Redirect::to("/").into_response();
    }

    let repo = state.db.repository();
    let settings = repo.get_system_settings().await.unwrap_or_default();

    let i18n = get_i18n(&headers, Some(&params));
    let error = params.get("error").cloned();
    let registration_mode = settings.registration_mode.clone();

    let html = crate::app::components::render_document(move || {
        leptos::prelude::view! {
            <crate::app::pages::register::RegisterPage
                error=error
                registration_mode=registration_mode
                i18n=i18n
                current_path="/register".to_string()
            />
        }
    });
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
        let token = match payload
            .invite_token
            .as_deref()
            .filter(|t| !t.trim().is_empty())
        {
            | Some(t) => t.trim(),
            | None => {
                return Redirect::to("/register?error=Invitation code required").into_response()
            },
        };
        let invite = match repo.get_invitation_by_token(token).await.ok().flatten() {
            | Some(inv) => inv,
            | None => {
                return Redirect::to("/register?error=Invalid or expired invitation")
                    .into_response()
            },
        };
        if invite.email.to_lowercase() != payload.email.to_lowercase() {
            return Redirect::to("/register?error=Email mismatch with invitation").into_response();
        }
        let _ = repo.mark_invitation_used(&invite.token).await;
    }

    let pwd_hash = match hash_password(&payload.password) {
        | Ok(h) => h,
        | Err(_) => return Redirect::to("/register?error=Password hashing failed").into_response(),
    };

    let user = repo
        .create_user(CreateUserDto {
            username: payload.username,
            email: payload.email,
            password_hash: pwd_hash,
            display_name: payload.display_name,
            role: None,
            is_platform_admin: None,
            storage_quota_bytes: None,
        })
        .await;

    match user {
        | Ok(u) => {
            if payload.create_org.as_deref() == Some("1")
                || payload.create_org.as_deref() == Some("on")
                || payload.create_org.as_deref() == Some("true")
            {
                if let Some(oname) = payload.org_name.filter(|s| !s.trim().is_empty()) {
                    let oslug = payload
                        .org_slug
                        .filter(|s| !s.trim().is_empty())
                        .unwrap_or_else(|| oname.to_lowercase().replace(' ', "-"));
                    let dto = apich_db::CreateOrganizationDto {
                        name: oname,
                        slug: oslug,
                        description: Some("Created during user registration".to_string()),
                        chat_url: None,
                        meeting_url: None,
                        drive_url: None,
                        ai_agent_url: None,
                        allow_team_override: Some(true),
                    };
                    let _ = state.identity_service.create_organization(u.id, dto).await;
                }
            }

            let token = generate_session_token();
            let token_hash = hash_session_token(&token);
            let expires_at = Utc::now() + chrono::Duration::days(14);
            let _ = repo
                .create_user_session(u.id, &token_hash, expires_at, None, None)
                .await;
            let cookie = build_session_cookie(&token, 14 * 86400);
            ([(header::SET_COOKIE, cookie)], Redirect::to("/")).into_response()
        },
        | Err(e) => {
            let msg = if e.to_string().contains("duplicate") {
                "Username or email already in use"
            } else {
                "Failed to create account"
            };
            Redirect::to(&format!("/register?error={}", urlencoding::encode(msg))).into_response()
        },
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
    ([(header::SET_COOKIE, clear_cookie)], Redirect::to("/login")).into_response()
}

/// Helper to find project by UUID string or Slug
pub(crate) async fn resolve_project(
    state: &AppState,
    id_or_slug: &str,
) -> Option<Project> {
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
        | Some(u) => u,
        | None => return redirect_to_login(&headers, &format!("/projects/{}", id_or_slug)),
    };

    let i18n = get_i18n(&headers, Some(&params));

    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => {
            return (
                StatusCode::NOT_FOUND,
                Html("<h3>Project not found 404</h3><p><a href='/'>Return to dashboard</a></p>"),
            )
                .into_response()
        },
    };

    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);

    if !can_access {
        return (StatusCode::FORBIDDEN, Html("<h3>403 Access Forbidden: You do not have permissions for this project</h3><p><a href='/'>Return to dashboard</a></p>")).into_response();
    }

    let repo = state.db.repository();

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

    let signature_statuses = if project.vigilant_mode {
        state
            .project_manager
            .verify_snapshot_signatures(project.id, &snapshots)
            .await
            .unwrap_or_default()
    } else {
        std::collections::HashMap::new()
    };

    let milestones = state
        .project_manager
        .list_milestones(project.id)
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
            remotes: Vec::new(),
            branches: vec!["main".to_string()],
            current_branch: Some("main".to_string()),
        });

    let files = state
        .project_manager
        .list_files(project.id)
        .await
        .unwrap_or_default();

    let ignore_config = state
        .project_manager
        .get_ignore_config(project.id)
        .await
        .unwrap_or_default();
    let gitignore_content = state
        .project_manager
        .read_ignore_file(project.id, ".gitignore")
        .await
        .unwrap_or_default();
    let apichignore_content = state
        .project_manager
        .read_ignore_file(project.id, ".apichignore")
        .await
        .unwrap_or_default();

    let is_org_admin = state
        .identity_service
        .is_org_or_team_admin(user.id)
        .await
        .unwrap_or(false);
    let active_tab = params
        .get("tab")
        .cloned()
        .unwrap_or_else(|| "files".to_string());
    let notice = params.get("notice").map(|s| {
        match s.as_str() {
            | "reconciled" => "Branch merge complete. Any conflicts are marked below.",
            | "conflict_resolved" => "Merge conflict successfully resolved and snapshot recorded.",
            | "git_synced" => "Exported and committed to local Git repository.",
            | "git_fetched" => "Fetched from remote.",
            | "git_pulled" => "Pulled from remote.",
            | "git_pushed" => "Pushed to remote.",
            | "git_rebased" => "Rebased current branch.",
            | "branch_created" => "Branch created.",
            | "branch_switched" => "Switched branch.",
            | "milestone_created" => "Milestone created.",
            | "vcs_undone" => "Undid last operation.",
            | "vcs_nothing_to_undo" => "Nothing to undo.",
            | "vcs_redone" => "Redid last undone operation.",
            | "vcs_nothing_to_redo" => "Nothing to redo.",
            | "collaborator_added" => "Collaborator added successfully.",
            | "collaborator_removed" => "Collaborator removed.",
            | "sharing_updated" => "Sharing settings updated.",
            | "ignore_updated" => "Ignore rules updated.",
            | _ => s.as_str(),
        }
        .to_string()
    });

    let hub_links = repo
        .resolve_hub_links(project.org_id, project.team_id)
        .await
        .ok();

    // Templates the "+ New File" modal can start a brand-new single file from -- kanban-kind
    // templates are excluded (see `create_file_action`'s own comment: they apply onto a
    // project's board settings, not a single file, so they don't fit this flow).
    let mut new_file_templates = Vec::new();
    for kind in ["note", "latex", "typst", "slides"] {
        new_file_templates.extend(
            crate::ui::template_handlers::list_visible_templates_of_kind(&state, user.id, kind)
                .await,
        );
    }

    let current_path = format!("/projects/{}?tab={}", id_or_slug, active_tab);
    let html = crate::app::components::render_document(move || {
        leptos::prelude::view! {
            <crate::app::pages::project_detail::ProjectDetailPage
                user=user
                is_org_or_team_admin=is_org_admin
                project=project
                hub_links=hub_links
                branches=branches
                current_branch=current_branch
                conflicts=conflicts
                snapshots=snapshots
                signature_statuses=signature_statuses
                milestones=milestones
                members=members
                all_users=all_users
                git_status=git_status
                files=files
                ignore_config=ignore_config
                gitignore_content=gitignore_content
                apichignore_content=apichignore_content
                new_file_templates=new_file_templates
                active_tab=active_tab
                notice=notice
                i18n=i18n
                current_path=current_path
            />
        }
    });

    Html(html).into_response()
}

/// Anonymous public-link viewer: honors the project's persisted `share_mode`/`share_role`
/// (set via the Sharing tab's "Public Link" form). No AuthUser required; 404 unless the
/// project has actually been made public.
async fn shared_project_page(
    headers: HeaderMap,
    Path(id_or_slug): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Response {
    let i18n = get_i18n(&headers, Some(&params));

    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => {
            return (StatusCode::NOT_FOUND, Html("<h3>Project not found</h3>")).into_response()
        },
    };

    let share_mode = project
        .settings
        .get("share_mode")
        .and_then(|v| v.as_str())
        .unwrap_or("private");
    if share_mode != "public" {
        return (
            StatusCode::NOT_FOUND,
            Html("<h3>This project is not shared publicly</h3><p><a href='/login'>Sign in</a></p>"),
        )
            .into_response();
    }
    let share_role = project
        .settings
        .get("share_role")
        .and_then(|v| v.as_str())
        .unwrap_or("read_only")
        .to_string();

    let files = state
        .project_manager
        .list_files(project.id)
        .await
        .unwrap_or_default();

    let html = crate::app::components::render_document(move || {
        leptos::prelude::view! {
            <crate::app::pages::shared_project::SharedProjectPage
                project=project
                files=files
                share_role=share_role
                i18n=i18n
            />
        }
    });
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
    let org_id = if let Some(first_org) = orgs.first() {
        first_org.id
    } else {
        let new_org = repo
            .create_organization(
                user.id,
                apich_db::CreateOrganizationDto {
                    name: format!("{}'s Workspace", user.display_name),
                    slug: format!(
                        "{}-ws-{}",
                        user.username,
                        &Uuid::new_v4().simple().to_string()[..6]
                    ),
                    description: Some("Personal workspace organization".to_string()),
                    chat_url: None,
                    meeting_url: None,
                    drive_url: None,
                    ai_agent_url: None,
                    allow_team_override: Some(true),
                },
            )
            .await?;
        new_org.id
    };

    let proj = state
        .project_manager
        .create_project(CreateProjectDto {
            org_id,
            team_id: None,
            owner_id: user.id,
            name: payload.name,
            slug: payload.slug,
            description: payload.description,
            storage_path: String::new(),
            settings: None,
        })
        .await?;

    Ok(Redirect::to(&format!("/projects/{}", proj.id)).into_response())
}

/// Create showcase demo project populated with Typst, cargo-slide, LaTeX, SQLite Table, and Unified Note
async fn create_demo_project_action(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> Result<Response, WebError> {
    let repo = state.db.repository();
    let orgs = repo.list_organizations_for_user(user.id).await?;
    let org_id = if let Some(first_org) = orgs.first() {
        first_org.id
    } else {
        let new_org = repo
            .create_organization(
                user.id,
                apich_db::CreateOrganizationDto {
                    name: format!("{}'s Lab", user.display_name),
                    slug: format!(
                        "{}-lab-{}",
                        user.username,
                        &Uuid::new_v4().simple().to_string()[..6]
                    ),
                    description: Some("Research organization".to_string()),
                    chat_url: None,
                    meeting_url: None,
                    drive_url: None,
                    ai_agent_url: None,
                    allow_team_override: Some(true),
                },
            )
            .await?;
        new_org.id
    };

    let slug = format!(
        "apich-showcase-{}",
        &Uuid::new_v4().simple().to_string()[..6]
    );
    let proj = state.project_manager.create_project(CreateProjectDto {
        org_id,
        team_id: None,
        owner_id: user.id,
        name: "APICH Showcase & Tour".to_string(),
        slug,
        description: Some("Complete tour project demonstrating Typst, cargo-slide, LaTeX, SQLite tables, and unified notes.".to_string()),
        storage_path: String::new(),
        settings: None,
    }).await?;

    // Seed demo files into this project workspace
    crate::services::demo_project::DemoProjectService::seed_demo_files(&proj.storage_path).await?;

    // Demonstrate the project-level public share link and a couple of per-file sharing rules
    // (plan.md's "公开链接／对特定账号的邀请；read only／read and review／read write and review").
    let _ = state
        .project_manager
        .update_project_share_settings(proj.id, "public", "read_and_review")
        .await;
    let _ = state
        .project_manager
        .update_file_share(proj.id, "slides.typ", "public", "read_only", vec![])
        .await;
    let _ = state
        .project_manager
        .update_file_share(
            proj.id,
            "lab_notebook.anote",
            "specific",
            "read_write_and_review",
            vec!["alice".to_string(), "bob".to_string()],
        )
        .await;

    Ok(Redirect::to(&format!(
        "/projects/{}?tab=files&notice=demo_created",
        proj.id
    ))
    .into_response())
}

/// Seed demo files into an existing project
async fn seed_demo_files_action(
    AuthUser(_user): AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Response, WebError> {
    let repo = state.db.repository();
    let proj = repo
        .get_project_by_id(id)
        .await?
        .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

    crate::services::demo_project::DemoProjectService::seed_demo_files(&proj.storage_path).await?;

    Ok(Redirect::to(&format!("/projects/{}?tab=files&notice=demo_seeded", id)).into_response())
}

#[derive(Debug, Deserialize)]
pub struct CreateFileForm {
    pub filename: String,
    pub template: Option<String>,
    /// When set, overrides `template`'s fixed builtin starters entirely -- content comes from a
    /// published Template Library version instead (note body, or a latex/typst/slides template's
    /// single captured file; kanban-kind templates aren't offered here, they apply onto a
    /// project's board settings, not a single new file). A plain `Option<Uuid>` won't do: the
    /// picker's own "no template" option still submits this field with an empty string value
    /// (not an absent key), which fails to parse as a `Uuid` -- so this stays a string and gets
    /// parsed manually below, treating "" the same as "not provided".
    pub template_version_id: Option<String>,
}

/// Create new file in project workspace
async fn create_file_action(
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Form(payload): Form<CreateFileForm>,
) -> Result<Response, WebError> {
    let filename = payload.filename.trim();
    if filename.is_empty() {
        return Ok(Redirect::to(&format!(
            "/projects/{}?tab=files&error=Filename+cannot+be+empty",
            id
        ))
        .into_response());
    }

    let version_id = payload
        .template_version_id
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|s| Uuid::parse_str(s).ok());
    if let Some(version_id) = version_id {
        // `can_access_template` takes a *template* id, but the picker only gives us a *version*
        // id -- resolve version -> template before checking access.
        let repo = state.db.repository();
        let Some(version) = repo.get_template_version_by_id(version_id).await? else {
            return Ok(Redirect::to(&format!(
                "/projects/{}?tab=files&error=Template+version+not+found",
                id
            ))
            .into_response());
        };
        if !IdentityPermissionResolver::can_access_template(
            state.db.pool(),
            user.id,
            version.template_id,
        )
        .await
        .unwrap_or(false)
        {
            return Ok(Redirect::to(&format!(
                "/projects/{}?tab=files&error=No+access+to+that+template",
                id
            ))
            .into_response());
        }
        let Some(template) = repo.get_template_by_id(version.template_id).await? else {
            return Ok(Redirect::to(&format!(
                "/projects/{}?tab=files&error=Template+not+found",
                id
            ))
            .into_response());
        };
        let content = match template.kind.as_str() {
            | "note" => {
                crate::services::template_library::note_body_from_content(&version.content)?
            },
            | "latex" | "typst" | "slides" => {
                let files =
                    crate::services::template_library::files_from_content(&version.content)?;
                files.first().map(|f| f.content.clone()).unwrap_or_default()
            },
            | _ => {
                return Ok(Redirect::to(&format!(
                    "/projects/{}?tab=files&error=That+template+kind+can%27t+start+a+new+file",
                    id
                ))
                .into_response())
            },
        };
        if template.kind == "slides" {
            // Same "slide.typ isn't guaranteed to exist yet" gap as the plain "Cargo-Slide Deck"
            // starter -- see `cargo_slide_helpers`'s own doc comment.
            if let Some(proj) = repo.get_project_by_id(id).await? {
                let _ = crate::services::cargo_slide_helpers::ensure_cargo_slide_helpers(
                    &proj.storage_path,
                )
                .await;
            }
        }
        state
            .project_manager
            .create_file_with_content(id, user.id, filename, &content)
            .await?;
    } else {
        let template = payload.template.as_deref().unwrap_or("empty");
        state
            .project_manager
            .create_file(id, user.id, filename, template)
            .await?;
    }

    let ext = filename.split('.').next_back().unwrap_or("").to_lowercase();
    let redirect_url = match ext.as_str() {
        | "typ" | "tex" | "latex" => {
            format!(
                "/projects/{}/editor?file={}",
                id,
                urlencoding::encode(filename)
            )
        },
        | "table" | "db" | "sqlite" | "sqlite3" => {
            format!(
                "/projects/{}/table?file={}",
                id,
                urlencoding::encode(filename)
            )
        },
        | "anote" | "note" => {
            format!(
                "/projects/{}/note?file={}",
                id,
                urlencoding::encode(filename)
            )
        },
        | _ => format!("/projects/{}?tab=files&notice=file_created", id),
    };

    Ok(Redirect::to(&redirect_url).into_response())
}

#[derive(Debug, Deserialize)]
pub struct DeleteFileForm {
    pub file: String,
}

/// Delete file from project workspace
async fn delete_file_action(
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Form(payload): Form<DeleteFileForm>,
) -> Result<Response, WebError> {
    state
        .project_manager
        .delete_file(id, user.id, &payload.file)
        .await?;
    Ok(Redirect::to(&format!("/projects/{}?tab=files&notice=file_deleted", id)).into_response())
}

#[derive(Debug, Deserialize)]
pub struct FileRawQuery {
    pub file: String,
    /// When present (any value), sets `Content-Disposition: attachment` so the browser downloads
    /// the file instead of trying to display it inline -- the same endpoint serves both the
    /// Files tab's "Download" link and the "open a binary file" case (a PDF, an image) that the
    /// text editor can't render, which wants the opposite (inline).
    pub download: Option<String>,
}

/// Serves a project file's raw bytes -- the Files tab's "Download" action, and also the "open"
/// target for any file type the text-based editor studio can't represent (PDFs, images, SQLite
/// tables, audio -- anything not source code/markdown/notes). Real content-type inferred from the
/// extension rather than a generic `application/octet-stream` for everything, so a PDF/image
/// opened (not downloaded) actually renders in the browser instead of prompting a save dialog for
/// content the browser could have shown directly.
async fn file_raw_action(
    auth: Option<AuthUser>,
    Path(id): Path<Uuid>,
    Query(query): Query<FileRawQuery>,
    State(state): State<AppState>,
) -> Response {
    let Some(AuthUser(user)) = auth else {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    };

    let can_access = IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, id)
        .await
        .unwrap_or(false);
    if !can_access {
        return (StatusCode::FORBIDDEN, "Forbidden").into_response();
    }

    let bytes = match state.project_manager.read_file_bytes(id, &query.file).await {
        | Ok(b) => b,
        | Err(e) => return (StatusCode::NOT_FOUND, e.to_string()).into_response(),
    };

    let content_type = mime_type_for_path(&query.file);
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, content_type.parse().unwrap());
    if query.download.is_some() {
        let name = std::path::Path::new(&query.file)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("download");
        if let Ok(value) = format!("attachment; filename=\"{}\"", name).parse() {
            headers.insert(header::CONTENT_DISPOSITION, value);
        }
    }
    (headers, bytes).into_response()
}

fn mime_type_for_path(path: &str) -> &'static str {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        | "pdf" => "application/pdf",
        | "png" => "image/png",
        | "jpg" | "jpeg" => "image/jpeg",
        | "gif" => "image/gif",
        | "svg" => "image/svg+xml",
        | "webp" => "image/webp",
        | "wav" => "audio/wav",
        | "mp3" => "audio/mpeg",
        | "csv" => "text/csv; charset=utf-8",
        | "json" => "application/json",
        | "txt" | "log" => "text/plain; charset=utf-8",
        | _ => "application/octet-stream",
    }
}

#[derive(Debug, Deserialize)]
pub struct EditorQuery {
    pub file: Option<String>,
    pub notice: Option<String>,
    pub error: Option<String>,
    pub engine: Option<String>,
}

/// Dedicated Document & Slide Editor Studio
async fn project_editor_page(
    auth: Option<AuthUser>,
    headers: HeaderMap,
    Path(id_or_slug): Path<String>,
    Query(query): Query<EditorQuery>,
    State(state): State<AppState>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };

    let i18n = get_i18n(&headers, None);
    let repo = state.db.repository();
    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => {
            return (
                StatusCode::NOT_FOUND,
                Html("<h3>Project not found 404</h3>"),
            )
                .into_response()
        },
    };

    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_access {
        return (StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }

    let is_org_admin = state
        .identity_service
        .is_org_or_team_admin(user.id)
        .await
        .unwrap_or(false);

    let file_path = if let Some(ref f) = query.file {
        f.clone()
    } else {
        let all_files = state
            .project_manager
            .list_files(project.id)
            .await
            .unwrap_or_default();
        if let Some(slides) = all_files
            .iter()
            .find(|f| f.path == "slides.typ" || f.category == "slide")
        {
            slides.path.clone()
        } else if let Some(first_doc) = all_files
            .iter()
            .find(|f| f.category == "typst" || f.category == "latex")
        {
            first_doc.path.clone()
        } else {
            "main.typ".to_string()
        }
    };

    let file_content = state
        .project_manager
        .read_file(project.id, &file_path)
        .await
        .unwrap_or_default();
    // Dispatches to a real per-language outline extractor (Typst's `=` headings, LaTeX's
    // `\section{}`, a script's top-level function/class defs) instead of always assuming
    // Markdown's `#` -- which, for a Typst file, actually matches *every* `#foo(...)` code
    // invocation (`#import`, `#slide(...)`, etc.) as a fake heading, and for a script, every
    // `#`-prefixed comment line. See `KnowledgeSyncService::extract_headings_for_file`.
    let headings = crate::services::knowledge_sync::KnowledgeSyncService::extract_headings_for_file(
        &file_content,
        &file_path,
    );

    let is_script = crate::services::document_renderer::DocumentRenderer::is_script(&file_path);
    let is_slide = file_path == "slides.typ"
        || file_path.ends_with(".slide.typ")
        || file_content.contains("#show: slide-theme")
        || file_content.contains("slide-theme");

    let ws_root = &project.storage_path;
    let typst_res = if !is_script
        && (file_path.ends_with(".typ") || file_path.ends_with(".slide.typ") || is_slide)
    {
        Some(
            crate::services::document_renderer::DocumentRenderer::compile_typst(
                ws_root, &file_path, true,
            )
            .await,
        )
    } else {
        None
    };

    let (typst_pages, compile_error) = match typst_res {
        | Some(ref r) if r.success => (r.pages_svg.clone(), None),
        | Some(ref r) => (Vec::new(), r.error_message.clone()),
        | None => (Vec::new(), None),
    };

    let file_share = state
        .project_manager
        .get_file_share(project.id, &file_path)
        .await
        .ok()
        .flatten();
    let all_users = repo.list_users().await.unwrap_or_default();

    // Template kind this file could be published/applied as -- `None` for a plain script, since
    // scripts aren't one of the 5 template kinds (see `apich_db::TemplateKind`).
    let is_latex_preview_for_kind = !is_script
        && !(file_path.ends_with(".typ") || file_path.ends_with(".slide.typ") || is_slide)
        && (file_path.ends_with(".tex") || file_path.ends_with(".latex"));
    let file_template_kind = if is_script {
        None
    } else if is_slide {
        Some("slides")
    } else if is_latex_preview_for_kind {
        Some("latex")
    } else if file_path.ends_with(".typ") {
        Some("typst")
    } else {
        None
    };
    let (own_file_templates, visible_file_templates) = match file_template_kind {
        | Some(k) => {
            (
                crate::ui::template_handlers::list_own_templates_of_kind(&state, user.id, k).await,
                crate::ui::template_handlers::list_visible_templates_of_kind(&state, user.id, k)
                    .await,
            )
        },
        | None => (Vec::new(), Vec::new()),
    };

    let current_path = format!(
        "/projects/{}/editor?file={}",
        project.id,
        urlencoding::encode(&file_path)
    );
    let notice = query.notice;
    let error = query.error;

    let html = crate::app::components::render_document(move || {
        leptos::prelude::view! {
            <crate::app::pages::document_editor_page::DocumentEditorPage
                user=user
                is_org_or_team_admin=is_org_admin
                project=project
                file_path=file_path
                content=file_content
                headings=headings
                is_slide=is_slide
                is_script=is_script
                typst_pages=typst_pages
                compile_error=compile_error
                file_share=file_share
                all_users=all_users
                template_kind=file_template_kind.map(|k| k.to_string())
                own_file_templates=own_file_templates
                visible_file_templates=visible_file_templates
                notice=notice
                error=error
                i18n=i18n
                current_path=current_path
            />
        }
    });

    Html(html).into_response()
}

/// Compiles a `.tex` file to PDF inside the project's sandbox container (real TeX Live, not this
/// dev host -- see `ProjectManagerService::compile_latex_in_sandbox`) and streams it back for
/// the editor's `<iframe>` to display. Recompiles on every request rather than caching, so
/// reloading the iframe after a Save shows the latest content -- LaTeX compiles in low seconds
/// for a document this size, cheap enough not to need a separate "recompile" action.
async fn latex_pdf_action(
    auth: Option<AuthUser>,
    Path(id_or_slug): Path<String>,
    Query(query): Query<EditorQuery>,
    State(state): State<AppState>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return (StatusCode::UNAUTHORIZED, Html("Unauthorized")).into_response(),
    };

    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => return (StatusCode::NOT_FOUND, Html("Project not found")).into_response(),
    };

    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_access {
        return (StatusCode::FORBIDDEN, Html("Forbidden")).into_response();
    }

    let Some(file) = query.file else {
        return (StatusCode::BAD_REQUEST, Html("Missing ?file=")).into_response();
    };

    let engine = sanitize_latex_engine(query.engine.as_deref());
    match state.project_manager.compile_latex_in_sandbox(project.id, user.id, &file, engine).await {
        Ok(Ok(pdf_bytes)) => (
            [(header::CONTENT_TYPE, "application/pdf")],
            pdf_bytes,
        ).into_response(),
        Ok(Err(compile_log)) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            Html(format!(
                "<html><body style=\"background:#1e1e1e; color:#f87171; font-family:monospace; white-space:pre-wrap; padding:1.5rem; margin:0;\">⚠️ LaTeX compilation failed:\n\n{}</body></html>",
                html_escape(&compile_log)
            )),
        ).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            Html(format!(
                "<html><body style=\"background:#1e1e1e; color:#f87171; font-family:monospace; white-space:pre-wrap; padding:1.5rem; margin:0;\">⚠️ {}</body></html>",
                html_escape(&e.to_string())
            )),
        ).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct LatexSyncRequest {
    file: String,
    engine: Option<String>,
    page: u32,
    x: f64,
    y: f64,
}

#[derive(Debug, Serialize)]
struct LatexSyncResponse {
    success: bool,
    line: Option<u32>,
    error: Option<String>,
}

/// PDF-click-to-source-line for the LaTeX studio's canvas-rendered preview (see
/// `document_editor.rs`'s `LATEX_PDF_VIEWER_JS`, which posts here with the click position
/// converted to PDF points) -- backed by the real `synctex` CLI via
/// `ProjectManagerService::synctex_edit_in_sandbox`. `engine` isn't actually needed by synctex
/// itself (the `.synctex.gz` doesn't care which engine wrote it), but is accepted anyway so the
/// request mirrors the same `file`+`engine` pair the PDF was rendered with, in case a future
/// engine ever needs engine-specific handling here.
async fn latex_sync_action(
    auth: Option<AuthUser>,
    Path(id_or_slug): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<LatexSyncRequest>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(LatexSyncResponse {
                    success: false,
                    line: None,
                    error: Some("Unauthorized".to_string()),
                }),
            )
                .into_response()
        },
    };

    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => {
            return (
                StatusCode::NOT_FOUND,
                Json(LatexSyncResponse {
                    success: false,
                    line: None,
                    error: Some("Project not found".to_string()),
                }),
            )
                .into_response()
        },
    };

    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_access {
        return (
            StatusCode::FORBIDDEN,
            Json(LatexSyncResponse {
                success: false,
                line: None,
                error: Some("Forbidden".to_string()),
            }),
        )
            .into_response();
    }

    let _ = sanitize_latex_engine(payload.engine.as_deref());
    match state
        .project_manager
        .synctex_edit_in_sandbox(
            project.id,
            user.id,
            &payload.file,
            payload.page,
            payload.x,
            payload.y,
        )
        .await
    {
        | Ok(line) => {
            Json(LatexSyncResponse {
                success: true,
                line,
                error: None,
            })
            .into_response()
        },
        | Err(e) => {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(LatexSyncResponse {
                    success: false,
                    line: None,
                    error: Some(e.to_string()),
                }),
            )
                .into_response()
        },
    }
}

pub(crate) fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Compiles a `.typ` file to a downloadable PDF (separate from the live SVG preview -- see
/// `DocumentRenderer::compile_typst_pdf`). Used by the editor's "Download PDF" button.
async fn typst_pdf_action(
    auth: Option<AuthUser>,
    Path(id_or_slug): Path<String>,
    Query(query): Query<EditorQuery>,
    State(state): State<AppState>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return (StatusCode::UNAUTHORIZED, Html("Unauthorized")).into_response(),
    };
    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => return (StatusCode::NOT_FOUND, Html("Project not found")).into_response(),
    };
    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_access {
        return (StatusCode::FORBIDDEN, Html("Forbidden")).into_response();
    }
    let Some(file) = query.file else {
        return (StatusCode::BAD_REQUEST, Html("Missing ?file=")).into_response();
    };

    match crate::services::document_renderer::DocumentRenderer::compile_typst_pdf(&project.storage_path, &file).await {
        Ok(pdf_bytes) => {
            let download_name = std::path::Path::new(&file).file_stem().and_then(|s| s.to_str()).unwrap_or("document").to_string();
            (
                [
                    (header::CONTENT_TYPE, "application/pdf".to_string()),
                    (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}.pdf\"", download_name)),
                ],
                pdf_bytes,
            ).into_response()
        }
        Err(compile_log) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            Html(format!(
                "<html><body style=\"background:#1e1e1e; color:#f87171; font-family:monospace; white-space:pre-wrap; padding:1.5rem; margin:0;\">⚠️ Typst compilation failed:\n\n{}</body></html>",
                html_escape(&compile_log)
            )),
        ).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct SlideBuildStartRequest {
    pub file: String,
    pub target: String,
}

/// Starts (asynchronously) compiling a cargo-slide presentation into a standalone binary for the
/// requested platform inside the project's sandbox container -- see
/// `ProjectManagerService::start_slide_build`/`SLIDE_BUILD_TARGETS` for the target list and
/// `crates/apich-islands/src/slide_build.rs` for the client that polls it. Returns a job id
/// immediately rather than blocking for as long as the (potentially cross-compiled, from-scratch)
/// build takes.
async fn slide_binary_start_action(
    auth: Option<AuthUser>,
    Path(id_or_slug): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<SlideBuildStartRequest>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Unauthorized"})),
            )
                .into_response()
        },
    };
    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Project not found"})),
            )
                .into_response()
        },
    };
    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_access {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Forbidden"}))).into_response();
    }
    if !crate::services::ProjectManager::SLIDE_BUILD_TARGETS
        .iter()
        .any(|(id, _)| *id == payload.target)
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Unknown build target"})),
        )
            .into_response();
    }

    match state
        .project_manager
        .start_slide_build(project.id, user.id, &payload.file, &payload.target)
        .await
    {
        | Ok(job_id) => Json(json!({ "job_id": job_id.to_string() })).into_response(),
        | Err(e) => {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        },
    }
}

/// Polls a slide-build job's real progress (see `SlideBuildRegistry`).
async fn slide_binary_status_action(
    auth: Option<AuthUser>,
    Query(query): Query<SlideBuildJobQuery>,
    State(state): State<AppState>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Unauthorized"})),
            )
                .into_response()
        },
    };
    let Ok(job_id) = uuid::Uuid::parse_str(&query.job) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid job id"})),
        )
            .into_response();
    };
    let Some(job) = state.project_manager.slide_builds.get(job_id).await else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Build job not found"})),
        )
            .into_response();
    };
    if job.owner_user_id != user.id {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Forbidden"}))).into_response();
    }
    match job.snapshot().await {
        crate::services::SlideBuildStatus::Running { percent, message } => {
            Json(json!({"status": "running", "percent": percent, "message": message})).into_response()
        }
        crate::services::SlideBuildStatus::Done { filename, size_bytes } => {
            Json(json!({"status": "done", "percent": 100, "filename": filename, "size_bytes": size_bytes})).into_response()
        }
        crate::services::SlideBuildStatus::Failed { message } => {
            Json(json!({"status": "failed", "error": message})).into_response()
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SlideBuildJobQuery {
    pub job: String,
}

/// Downloads a finished slide-build job's binary.
async fn slide_binary_download_action(
    auth: Option<AuthUser>,
    Query(query): Query<SlideBuildJobQuery>,
    State(state): State<AppState>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return (StatusCode::UNAUTHORIZED, Html("Unauthorized")).into_response(),
    };
    let Ok(job_id) = uuid::Uuid::parse_str(&query.job) else {
        return (StatusCode::BAD_REQUEST, Html("Invalid job id")).into_response();
    };
    let Some(job) = state.project_manager.slide_builds.get(job_id).await else {
        return (StatusCode::NOT_FOUND, Html("Build job not found")).into_response();
    };
    if job.owner_user_id != user.id {
        return (StatusCode::FORBIDDEN, Html("Forbidden")).into_response();
    }
    let crate::services::SlideBuildStatus::Done { filename, .. } = job.snapshot().await else {
        return (StatusCode::CONFLICT, Html("Build has not finished yet")).into_response();
    };
    let Some(bytes) = state.project_manager.slide_builds.binary(job_id).await else {
        return (StatusCode::NOT_FOUND, Html("Binary no longer available")).into_response();
    };
    (
        [
            (header::CONTENT_TYPE, "application/octet-stream".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", filename),
            ),
        ],
        bytes,
    )
        .into_response()
}

/// apich-vcs's own remote protocol -- "clone"/"pull": download a full history bundle. Accepts
/// the same auth as everything else (session cookie or PAT via `Authorization: Bearer`/`Basic`,
/// see `AuthUser`), so `apich remote clone https://user:<PAT>@host/vcs-remote/:id/bundle` works
/// from outside a browser too.
async fn vcs_remote_bundle_get_action(
    auth: Option<AuthUser>,
    Path(id_or_slug): Path<String>,
    State(state): State<AppState>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response(),
    };
    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => return (StatusCode::NOT_FOUND, "Project not found").into_response(),
    };
    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_access {
        return (StatusCode::FORBIDDEN, "Forbidden").into_response();
    }

    match state.project_manager.export_vcs_bundle(project.id).await {
        | Ok(bytes) => {
            (
                [
                    (header::CONTENT_TYPE, "application/gzip".to_string()),
                    (
                        header::CONTENT_DISPOSITION,
                        format!("attachment; filename=\"{}.apich-bundle\"", project.slug),
                    ),
                ],
                bytes,
            )
                .into_response()
        },
        | Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// apich-vcs's own remote protocol -- "push": upload a bundle to be merged in, fast-forward-only
/// (see `apich_vcs::bundle::ProjectBundle::accept_push`). Requires edit access, not just read.
async fn vcs_remote_bundle_post_action(
    auth: Option<AuthUser>,
    Path(id_or_slug): Path<String>,
    State(state): State<AppState>,
    body: axum::body::Bytes,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Unauthorized"})),
            )
                .into_response()
        },
    };
    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Project not found"})),
            )
                .into_response()
        },
    };
    let can_edit =
        IdentityPermissionResolver::can_edit_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_edit {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Write access required"})),
        )
            .into_response();
    }

    match state
        .project_manager
        .accept_vcs_push(project.id, &body)
        .await
    {
        | Ok(outcome) => {
            Json(json!({
                "accepted_branches": outcome.accepted_branches,
                "rejected_branches": outcome.rejected_branches,
            }))
            .into_response()
        },
        | Err(e) => {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        },
    }
}

/// Self-hosted git-over-HTTP: bridges a real Git client's smart-HTTP requests to the real
/// `git http-backend` CGI program against a bare mirror of the project (see
/// `crate::services::git_server`). Read access follows the same rules as the rest of the app;
/// write access (`git push`) additionally requires edit access, and its result is synced back
/// into the project's tracked working directory + apich-vcs history.
async fn git_http_backend_action(
    auth: Option<AuthUser>,
    Path((id_or_slug, path)): Path<(String, String)>,
    axum::extract::RawQuery(query): axum::extract::RawQuery,
    method: axum::http::Method,
    headers: HeaderMap,
    State(state): State<AppState>,
    body: axum::body::Bytes,
) -> Response {
    let query = query.unwrap_or_default();
    let is_write = path.contains("receive-pack") || query.contains("service=git-receive-pack");

    let project_key = id_or_slug.strip_suffix(".git").unwrap_or(&id_or_slug);
    let project = match resolve_project(&state, project_key).await {
        | Some(p) => p,
        | None => return (StatusCode::NOT_FOUND, "Repository not found").into_response(),
    };

    let www_authenticate = [(header::WWW_AUTHENTICATE, "Basic realm=\"APICH Git\"")];
    let user = match auth {
        | Some(AuthUser(u)) => u,
        | None => {
            return (
                StatusCode::UNAUTHORIZED,
                www_authenticate,
                "Authentication required",
            )
                .into_response()
        },
    };

    if is_write {
        let can_edit =
            IdentityPermissionResolver::can_edit_project(state.db.pool(), user.id, project.id)
                .await
                .unwrap_or(false);
        if !can_edit {
            return (StatusCode::FORBIDDEN, "Write access required").into_response();
        }
    } else {
        let can_access =
            IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id)
                .await
                .unwrap_or(false);
        if !can_access {
            return (StatusCode::FORBIDDEN, "Read access required").into_response();
        }
    }

    let storage_path = std::path::PathBuf::from(&project.storage_path);
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok());
    let content_encoding = headers
        .get(header::CONTENT_ENCODING)
        .and_then(|v| v.to_str().ok());

    let cgi_result = crate::services::git_server::run_git_http_backend(
        &storage_path,
        method.as_str(),
        &path,
        &query,
        content_type,
        content_encoding,
        &user.username,
        &body,
    )
    .await;

    let cgi = match cgi_result {
        | Ok(r) => r,
        | Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("git http-backend failed: {}", e),
            )
                .into_response()
        },
    };

    if is_write && (200..300).contains(&cgi.status) {
        let sync_path = storage_path.clone();
        let _ =
            crate::services::git_server::sync_working_from_mirror_and_snapshot(&sync_path).await;
    }

    let mut builder = Response::builder().status(cgi.status);
    for (k, v) in &cgi.headers {
        builder = builder.header(k, v);
    }
    builder
        .body(axum::body::Body::from(cgi.body))
        .unwrap_or_else(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to build response",
            )
                .into_response()
        })
}

#[derive(Debug, Deserialize)]
pub struct SaveEditorFileForm {
    #[serde(alias = "path")]
    pub file: String,
    pub content: String,
}

/// Save file content from Document & Slide Editor
async fn save_editor_file_action(
    auth: Option<AuthUser>,
    Path(id_or_slug): Path<String>,
    State(state): State<AppState>,
    Form(payload): Form<SaveEditorFileForm>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };

    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => return Redirect::to("/").into_response(),
    };

    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_access {
        return (StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }

    if let Err(e) = state
        .project_manager
        .write_file(project.id, user.id, &payload.file, &payload.content)
        .await
    {
        return Redirect::to(&format!(
            "/projects/{}/editor?file={}&error={}",
            project.id,
            urlencoding::encode(&payload.file),
            urlencoding::encode(&e.to_string())
        ))
        .into_response();
    }

    Redirect::to(&format!(
        "/projects/{}/editor?file={}&notice=file_saved",
        project.id,
        urlencoding::encode(&payload.file)
    ))
    .into_response()
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
    let _ = state
        .project_manager
        .snapshot_project(id, user.id, &payload.message)
        .await?;
    Ok(Redirect::to(&format!("/projects/{}?tab=timeline", id)).into_response())
}

/// Weave-free merge from web UI
async fn merge_action(
    AuthUser(_user): AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Form(payload): Form<MergeForm>,
) -> Result<Response, WebError> {
    let _ = state
        .project_manager
        .merge_branch(id, &payload.branch)
        .await?;
    Ok(Redirect::to(&format!("/projects/{}?tab=merge&notice=reconciled", id)).into_response())
}

#[derive(Debug, Deserialize)]
pub struct BranchNameForm {
    pub name: String,
}

async fn branch_create_action(
    AuthUser(_user): AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Form(payload): Form<BranchNameForm>,
) -> Result<Response, WebError> {
    let name = payload.name.trim();
    if name.is_empty() {
        return Ok(Redirect::to(&format!(
            "/projects/{}?tab=vcs&error=Branch+name+cannot+be+empty",
            id
        ))
        .into_response());
    }
    state.project_manager.branch_create(id, name).await?;
    Ok(Redirect::to(&format!("/projects/{}?tab=vcs&notice=branch_created", id)).into_response())
}

async fn branch_switch_action(
    AuthUser(_user): AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Form(payload): Form<BranchNameForm>,
) -> Result<Response, WebError> {
    state
        .project_manager
        .branch_switch(id, payload.name.trim())
        .await?;
    Ok(Redirect::to(&format!("/projects/{}?tab=vcs&notice=branch_switched", id)).into_response())
}

#[derive(Debug, Deserialize)]
pub struct MilestoneCreateForm {
    pub name: String,
    #[serde(default)]
    pub desc: String,
}

async fn milestone_create_action(
    AuthUser(_user): AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Form(payload): Form<MilestoneCreateForm>,
) -> Result<Response, WebError> {
    let name = payload.name.trim();
    if name.is_empty() {
        return Ok(Redirect::to(&format!(
            "/projects/{}?tab=vcs&error=Milestone+name+cannot+be+empty",
            id
        ))
        .into_response());
    }
    state
        .project_manager
        .create_milestone(id, name, payload.desc.trim())
        .await?;
    Ok(Redirect::to(&format!(
        "/projects/{}?tab=vcs&notice=milestone_created",
        id
    ))
    .into_response())
}

async fn vcs_undo_action(
    AuthUser(_user): AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Response, WebError> {
    let result = state.project_manager.vcs_undo(id).await?;
    let notice = if result.is_some() {
        "vcs_undone"
    } else {
        "vcs_nothing_to_undo"
    };
    Ok(Redirect::to(&format!("/projects/{}?tab=vcs&notice={}", id, notice)).into_response())
}

async fn vcs_redo_action(
    AuthUser(_user): AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Response, WebError> {
    let result = state.project_manager.vcs_redo(id).await?;
    let notice = if result.is_some() {
        "vcs_redone"
    } else {
        "vcs_nothing_to_redo"
    };
    Ok(Redirect::to(&format!("/projects/{}?tab=vcs&notice={}", id, notice)).into_response())
}

/// Resolve conflict from web UI
async fn resolve_conflict_action(
    AuthUser(_user): AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Form(payload): Form<ResolveConflictForm>,
) -> Result<Response, WebError> {
    state
        .project_manager
        .resolve_conflict(
            id,
            &payload.file,
            &payload.choice,
            payload.custom_content.as_deref(),
        )
        .await?;

    Ok(Redirect::to(&format!(
        "/projects/{}?tab=merge&notice=conflict_resolved",
        id
    ))
    .into_response())
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

#[derive(Debug, Deserialize)]
pub struct GitRemoteAddForm {
    pub name: String,
    pub url: String,
}

async fn git_remote_add_action(
    AuthUser(_user): AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Form(payload): Form<GitRemoteAddForm>,
) -> Result<Response, WebError> {
    state
        .project_manager
        .git_add_remote(id, payload.name.trim(), payload.url.trim())
        .await?;
    Ok(Redirect::to(&format!("/projects/{}?tab=vcs", id)).into_response())
}

#[derive(Debug, Deserialize)]
pub struct GitRemoteForm {
    pub remote: String,
}

async fn git_fetch_action(
    AuthUser(_user): AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Form(payload): Form<GitRemoteForm>,
) -> Result<Response, WebError> {
    state
        .project_manager
        .git_fetch(id, payload.remote.trim())
        .await?;
    Ok(Redirect::to(&format!("/projects/{}?tab=vcs&notice=git_fetched", id)).into_response())
}

#[derive(Debug, Deserialize)]
pub struct GitRemoteBranchForm {
    pub remote: String,
    pub branch: String,
}

async fn git_pull_action(
    AuthUser(_user): AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Form(payload): Form<GitRemoteBranchForm>,
) -> Result<Response, WebError> {
    state
        .project_manager
        .git_pull(id, payload.remote.trim(), payload.branch.trim())
        .await?;
    Ok(Redirect::to(&format!("/projects/{}?tab=vcs&notice=git_pulled", id)).into_response())
}

async fn git_push_action(
    AuthUser(_user): AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Form(payload): Form<GitRemoteBranchForm>,
) -> Result<Response, WebError> {
    state
        .project_manager
        .git_push(id, payload.remote.trim(), payload.branch.trim())
        .await?;
    Ok(Redirect::to(&format!("/projects/{}?tab=vcs&notice=git_pushed", id)).into_response())
}

#[derive(Debug, Deserialize)]
pub struct GitRebaseForm {
    pub upstream: String,
}

async fn git_rebase_action(
    AuthUser(_user): AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Form(payload): Form<GitRebaseForm>,
) -> Result<Response, WebError> {
    state
        .project_manager
        .git_rebase(id, payload.upstream.trim())
        .await?;
    Ok(Redirect::to(&format!("/projects/{}?tab=vcs&notice=git_rebased", id)).into_response())
}

#[derive(Debug, Deserialize)]
pub struct IgnoreProfileToggleForm {
    pub profile: String,
    pub enabled: String,
}

async fn ignore_profile_toggle_action(
    AuthUser(_user): AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Form(payload): Form<IgnoreProfileToggleForm>,
) -> Result<Response, WebError> {
    let enabled = payload.enabled.trim() == "true";
    state
        .project_manager
        .set_ignore_profile(id, payload.profile.trim(), enabled)
        .await?;
    Ok(Redirect::to(&format!("/projects/{}?tab=vcs&notice=ignore_updated", id)).into_response())
}

#[derive(Debug, Deserialize)]
pub struct IgnoreRuleForm {
    pub rule: String,
}

async fn ignore_rule_add_action(
    AuthUser(_user): AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Form(payload): Form<IgnoreRuleForm>,
) -> Result<Response, WebError> {
    let rule = payload.rule.trim();
    if !rule.is_empty() {
        state.project_manager.add_ignore_rule(id, rule).await?;
    }
    Ok(Redirect::to(&format!("/projects/{}?tab=vcs&notice=ignore_updated", id)).into_response())
}

async fn ignore_rule_remove_action(
    AuthUser(_user): AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Form(payload): Form<IgnoreRuleForm>,
) -> Result<Response, WebError> {
    state
        .project_manager
        .remove_ignore_rule(id, payload.rule.trim())
        .await?;
    Ok(Redirect::to(&format!("/projects/{}?tab=vcs&notice=ignore_updated", id)).into_response())
}

#[derive(Debug, Deserialize)]
pub struct IgnoreFileSaveForm {
    pub file_name: String,
    #[serde(default)]
    pub content: String,
}

async fn ignore_file_save_action(
    AuthUser(_user): AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Form(payload): Form<IgnoreFileSaveForm>,
) -> Result<Response, WebError> {
    state
        .project_manager
        .write_ignore_file(id, payload.file_name.trim(), &payload.content)
        .await?;
    Ok(Redirect::to(&format!("/projects/{}?tab=vcs&notice=ignore_updated", id)).into_response())
}

/// Add collaborator to project
async fn add_project_member_form(
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Form(payload): Form<AddProjectMemberForm>,
) -> Result<Response, WebError> {
    let repo = state.db.repository();
    let proj = repo
        .get_project_by_id(id)
        .await?
        .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

    if proj.owner_id != user.id && !user.is_platform_admin {
        return Err(WebError::Forbidden(
            "Only owner or admin can add collaborators".to_string(),
        ));
    }

    let target_uid = resolve_user_id(&repo, &payload.user_id_or_username)
        .await
        .ok_or_else(|| WebError::BadRequest("Target user not found".to_string()))?;

    let role = match payload.role.as_str() {
        | "read_only" | "viewer" => "read_only",
        | "read_and_review" => "read_and_review",
        | _ => "read_write_and_review",
    };

    repo.add_project_member(id, target_uid, role).await?;
    Ok(Redirect::to(&format!(
        "/projects/{}?tab=members&notice=collaborator_added",
        id
    ))
    .into_response())
}

/// Remove collaborator from project
async fn remove_project_member_form(
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Form(payload): Form<RemoveProjectMemberForm>,
) -> Result<Response, WebError> {
    let repo = state.db.repository();
    let proj = repo
        .get_project_by_id(id)
        .await?
        .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

    if proj.owner_id != user.id && !user.is_platform_admin {
        return Err(WebError::Forbidden(
            "Only owner or admin can remove collaborators".to_string(),
        ));
    }

    repo.remove_project_member(id, payload.user_id).await?;
    Ok(Redirect::to(&format!(
        "/projects/{}?tab=members&notice=collaborator_removed",
        id
    ))
    .into_response())
}

#[derive(Debug, Deserialize)]
pub struct TablePageQuery {
    pub file: Option<String>,
    pub table: Option<String>,
    pub page: Option<usize>,
    pub search: Option<String>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub mode: Option<String>,
    pub notice: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TableSqlForm {
    pub file: String,
    pub sql: String,
    #[serde(default)]
    pub max_rows: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct TableCellEditForm {
    pub file: String,
    pub table: String,
    pub row_id_col: String,
    pub row_id_val: String,
    pub col: String,
    pub val: String,
}

#[derive(Debug, Deserialize)]
pub struct TableRowAddForm {
    pub file: String,
    pub table: String,
}

#[derive(Debug, Deserialize)]
pub struct TableRowDeleteForm {
    pub file: String,
    pub table: String,
    pub row_id_col: String,
    pub row_id_val: String,
}

#[derive(Debug, Deserialize)]
pub struct TableExportQuery {
    pub file: String,
    pub table: String,
    #[serde(default)]
    pub format: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TableImportForm {
    pub file: String,
    pub table: String,
    pub csv_data: String,
}

#[derive(Debug, Deserialize)]
pub struct NotebookCellCreateForm {
    pub file: String,
    pub table: String,
    pub language: String,
}

#[derive(Debug, Deserialize)]
pub struct NotebookCellDeleteForm {
    pub file: String,
    pub table: String,
    pub cell_id: i64,
}

#[derive(Debug, Deserialize)]
pub struct NotebookCellMoveForm {
    pub file: String,
    pub table: String,
    pub cell_id: i64,
    pub direction: String,
}

#[derive(Debug, Deserialize)]
pub struct NotebookRunCellForm {
    pub file: String,
    pub cell_id: i64,
    pub language: String,
    pub code: String,
}

#[derive(Debug, Deserialize)]
pub struct NoteQuery {
    pub file: Option<String>,
    pub view: Option<String>,
    pub notice: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SaveNoteForm {
    pub file: String,
    pub view: Option<String>,
    pub meta_title: String,
    pub meta_author: Option<String>,
    pub meta_tags: Option<String>,
    pub body: String,
}

#[derive(Debug, Deserialize)]
pub struct KnowledgePageQuery {
    pub view: Option<String>,
    pub notice: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ToggleTaskForm {
    pub file: String,
    pub line_number: usize,
    pub status: String,
    /// Whether `status`'s column counts as "done" (see `KanbanColumnDef::is_done`) -- sent by the
    /// Kanban board's own card button, which already knows this from the project's column config
    /// it just rendered. `None` (an old bookmarked/cached form) falls back to the pre-custom-
    /// columns behavior of treating only the literal "done" id as done.
    pub done: Option<String>,
    pub view: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TerminalExecForm {
    pub command: String,
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
    pub chat_url: Option<String>,
    pub meeting_url: Option<String>,
    pub drive_url: Option<String>,
    pub ai_agent_url: Option<String>,
    pub allow_team_override: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateOrgForm {
    pub org_id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub chat_url: Option<String>,
    pub meeting_url: Option<String>,
    pub drive_url: Option<String>,
    pub ai_agent_url: Option<String>,
    pub allow_team_override: Option<String>,
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
    pub chat_url: Option<String>,
    pub meeting_url: Option<String>,
    pub drive_url: Option<String>,
    pub ai_agent_url: Option<String>,
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
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return redirect_to_login(&headers, "/settings"),
    };

    let i18n = get_i18n(&headers, Some(&params));
    let notice = params.get("notice").cloned();
    let error = params.get("error").cloned();
    let is_org_admin = state
        .identity_service
        .is_org_or_team_admin(user.id)
        .await
        .unwrap_or(false);
    let repo = state.db.repository();
    let passkeys = repo
        .get_fido2_credentials_by_user(user.id)
        .await
        .unwrap_or_default();
    let pats = repo
        .list_personal_access_tokens(user.id)
        .await
        .unwrap_or_default();
    let ssh_keys = repo.list_ssh_public_keys(user.id).await.unwrap_or_default();
    let gpg_keys = repo.list_gpg_public_keys(user.id).await.unwrap_or_default();
    let new_pat_token = params.get("new_pat_token").cloned();

    let html = crate::app::components::render_document(move || {
        leptos::prelude::view! {
            <crate::app::pages::settings::SettingsPage
                user=user
                is_org_or_team_admin=is_org_admin
                passkeys=passkeys
                pats=pats
                ssh_keys=ssh_keys
                gpg_keys=gpg_keys
                new_pat_token=new_pat_token
                notice=notice
                error=error
                i18n=i18n
                current_path="/settings".to_string()
            />
        }
    });
    Html(html).into_response()
}

/// Generates a new random personal access token, returning (plaintext, sha256_hash, display_prefix).
/// Reuses the same random-byte + SHA-256 scheme as session tokens (see `auth::session`), just with
/// a distinguishing `apat_` prefix so a PAT is visually recognizable as such wherever it's pasted.
fn generate_pat() -> (String, String, String) {
    let raw = generate_session_token();
    let token = format!("apat_{}", raw);
    let hash = hash_session_token(&token);
    let prefix = token.chars().take(12).collect::<String>();
    (token, hash, prefix)
}

#[derive(Debug, Deserialize)]
pub struct CreatePatForm {
    pub name: String,
}

async fn create_pat_action(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Form(payload): Form<CreatePatForm>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };
    let (token, hash, prefix) = generate_pat();
    let repo = state.db.repository();
    match repo
        .create_personal_access_token(user.id, payload.name.trim(), &hash, &prefix, None)
        .await
    {
        | Ok(_) => {
            Redirect::to(&format!(
                "/settings?new_pat_token={}#pat",
                urlencoding::encode(&token)
            ))
            .into_response()
        },
        | Err(e) => redirect_error("/settings", e),
    }
}

async fn revoke_pat_action(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Path(token_id): Path<Uuid>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };
    let _ = state
        .db
        .repository()
        .revoke_personal_access_token(user.id, token_id)
        .await;
    Redirect::to("/settings#pat").into_response()
}

#[derive(Debug, Deserialize)]
pub struct AddSshKeyForm {
    pub name: String,
    pub public_key: String,
}

async fn add_ssh_key_action(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Form(payload): Form<AddSshKeyForm>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };
    let trimmed = payload.public_key.trim();
    let mut parts = trimmed.split_whitespace();
    let key_type =
        match parts.next() {
            | Some(t) if t.starts_with("ssh-") || t.starts_with("ecdsa-") => t.to_string(),
            | _ => return redirect_error(
                "/settings",
                "Not a recognizable SSH public key (expected \"ssh-ed25519 AAAA...\" or similar)",
            ),
        };
    let Some(key_body) = parts.next() else {
        return redirect_error(
            "/settings",
            "Not a recognizable SSH public key (missing key body)",
        );
    };
    let key_bytes =
        match base64::Engine::decode(&base64::engine::general_purpose::STANDARD, key_body) {
            | Ok(b) => b,
            | Err(_) => {
                return redirect_error(
                    "/settings",
                    "Not a recognizable SSH public key (invalid base64 body)",
                )
            },
        };
    use sha2::Digest as _;
    let mut hasher = sha2::Sha256::new();
    sha2::Digest::update(&mut hasher, &key_bytes);
    let fingerprint = format!(
        "SHA256:{}",
        base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD_NO_PAD,
            sha2::Digest::finalize(hasher)
        )
    );

    let repo = state.db.repository();
    match repo
        .add_ssh_public_key(
            user.id,
            payload.name.trim(),
            &key_type,
            trimmed,
            &fingerprint,
        )
        .await
    {
        | Ok(_) => Redirect::to("/settings#ssh").into_response(),
        | Err(e) => redirect_error("/settings", e),
    }
}

async fn delete_ssh_key_action(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Path(key_id): Path<Uuid>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };
    let _ = state
        .db
        .repository()
        .delete_ssh_public_key(user.id, key_id)
        .await;
    Redirect::to("/settings#ssh").into_response()
}

#[derive(Debug, Deserialize)]
pub struct AddGpgKeyForm {
    pub name: String,
    pub public_key: String,
}

async fn add_gpg_key_action(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Form(payload): Form<AddGpgKeyForm>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };
    let armored = payload.public_key.trim();
    if !armored.contains("BEGIN PGP PUBLIC KEY BLOCK") {
        return redirect_error("/settings", "Not an ASCII-armored GPG public key (expected a \"-----BEGIN PGP PUBLIC KEY BLOCK-----\" block)");
    }
    let fingerprint = match crate::services::gpg_key_fingerprint(armored) {
        | Ok(fp) => fp,
        | Err(e) => return redirect_error("/settings", format!("Could not read this key: {}", e)),
    };

    let repo = state.db.repository();
    match repo
        .add_gpg_public_key(user.id, payload.name.trim(), armored, &fingerprint)
        .await
    {
        | Ok(_) => Redirect::to("/settings#gpg").into_response(),
        | Err(e) => redirect_error("/settings", e),
    }
}

async fn delete_gpg_key_action(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Path(key_id): Path<Uuid>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };
    let _ = state
        .db
        .repository()
        .delete_gpg_public_key(user.id, key_id)
        .await;
    Redirect::to("/settings#gpg").into_response()
}

/// Update user profile
async fn update_profile_form(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Form(payload): Form<UpdateProfileForm>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };

    let avatar = payload.avatar_url.filter(|s| !s.trim().is_empty());
    let dto = apich_db::UpdateUserProfileDto {
        display_name: Some(payload.display_name.trim().to_string()),
        email: Some(payload.email.trim().to_string()),
        avatar_url: avatar,
    };

    match state
        .identity_service
        .update_user_profile(user.id, dto)
        .await
    {
        | Ok(_) => redirect_notice("/settings", "Profile updated successfully"),
        | Err(e) => redirect_error("/settings", e),
    }
}

/// Change password
async fn change_password_form(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Form(payload): Form<ChangePasswordForm>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };

    if payload.new_password != payload.confirm_password {
        return redirect_error(
            "/settings",
            "New password and confirm password do not match",
        );
    }

    match state
        .identity_service
        .change_user_password(user.id, &payload.current_password, &payload.new_password)
        .await
    {
        | Ok(_) => redirect_notice("/settings", "Password changed successfully"),
        | Err(e) => redirect_error("/settings", e),
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
        | Some(u) => u,
        | None => return redirect_to_login(&headers, "/admin/orgs"),
    };

    let i18n = get_i18n(&headers, Some(&params));
    let repo = state.db.repository();
    let is_org_admin = state
        .identity_service
        .is_org_or_team_admin(user.id)
        .await
        .unwrap_or(false);

    // Only admins (platform, org, or team level) have any legitimate use for this page.
    if !user.is_platform_admin && !is_org_admin {
        return (StatusCode::FORBIDDEN, Html("<h3>403 Access Forbidden: Administrator privileges required</h3><p><a href='/'>Return to dashboard</a></p>".to_string())).into_response();
    }

    // Scoped listing: platform admins see every org, everyone else only the orgs they belong to.
    let orgs = state
        .identity_service
        .list_user_organizations(user.id)
        .await
        .unwrap_or_default();
    let all_users = repo.list_users().await.unwrap_or_default();

    let mut org_cards = Vec::with_capacity(orgs.len());
    for org in orgs {
        let can_manage =
            IdentityPermissionResolver::can_manage_org(state.db.pool(), user.id, org.id)
                .await
                .unwrap_or(false);
        let members = repo.list_org_members(org.id).await.unwrap_or_default();
        let trees = repo.get_team_tree_for_org(org.id).await.unwrap_or_default();
        let teams = crate::app::pages::org_teams::flatten_team_tree(
            state.db.pool(),
            user.id,
            &repo,
            &trees,
        )
        .await;

        org_cards.push(crate::app::pages::org_teams::OrgCard {
            org,
            can_manage,
            members,
            teams,
        });
    }

    let notice = params.get("notice").map(|s| {
        match s.as_str() {
            | "org_created" => "Organization created successfully.",
            | "org_updated" => "Organization updated successfully.",
            | "org_deleted" => "Organization deleted successfully.",
            | "team_created" => "Team created successfully.",
            | "team_updated" => "Team updated successfully.",
            | "team_deleted" => "Team deleted successfully.",
            | "member_added" => "Member added to organization.",
            | "member_removed" => "Member removed from organization.",
            | "team_member_added" => "Member added to team.",
            | "team_member_removed" => "Member removed from team.",
            | _ => s.as_str(),
        }
        .to_string()
    });
    let error = params.get("error").cloned();

    let html = crate::app::components::render_document(move || {
        leptos::prelude::view! {
            <crate::app::pages::org_teams::OrgTeamsPage
                user=user
                is_org_or_team_admin=is_org_admin
                orgs=org_cards
                all_users=all_users
                notice=notice
                error=error
                i18n=i18n
                current_path="/admin/orgs".to_string()
            />
        }
    });

    Html(html).into_response()
}

/// Create organization
async fn create_org_form(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Form(payload): Form<CreateOrgForm>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };

    let allow_team_override = payload
        .allow_team_override
        .as_deref()
        .map(|s| s == "true" || s == "on" || s == "1")
        .unwrap_or(true);

    let dto = apich_db::CreateOrganizationDto {
        slug: payload.slug.trim().to_lowercase(),
        name: payload.name.trim().to_string(),
        description: payload.description.filter(|s| !s.trim().is_empty()),
        chat_url: payload.chat_url.filter(|s| !s.trim().is_empty()),
        meeting_url: payload.meeting_url.filter(|s| !s.trim().is_empty()),
        drive_url: payload.drive_url.filter(|s| !s.trim().is_empty()),
        ai_agent_url: payload.ai_agent_url.filter(|s| !s.trim().is_empty()),
        allow_team_override: Some(allow_team_override),
    };

    match state
        .identity_service
        .create_organization(user.id, dto)
        .await
    {
        | Ok(_) => redirect_notice("/admin/orgs", "org_created"),
        | Err(e) => redirect_error("/admin/orgs", e),
    }
}

/// Update organization
async fn update_org_form(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Form(payload): Form<UpdateOrgForm>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };

    let allow_team_override = payload
        .allow_team_override
        .as_deref()
        .map(|s| s == "true" || s == "on" || s == "1");

    let dto = apich_db::UpdateOrganizationDto {
        name: Some(payload.name.trim().to_string()),
        slug: Some(payload.slug.trim().to_lowercase()),
        description: payload.description.filter(|s| !s.trim().is_empty()),
        chat_url: payload.chat_url.filter(|s| !s.trim().is_empty()),
        meeting_url: payload.meeting_url.filter(|s| !s.trim().is_empty()),
        drive_url: payload.drive_url.filter(|s| !s.trim().is_empty()),
        ai_agent_url: payload.ai_agent_url.filter(|s| !s.trim().is_empty()),
        allow_team_override,
    };

    match state
        .identity_service
        .update_organization(user.id, payload.org_id, dto)
        .await
    {
        | Ok(_) => redirect_notice("/admin/orgs", "org_updated"),
        | Err(e) => redirect_error("/admin/orgs", e),
    }
}

/// Delete organization
async fn delete_org_form(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Form(payload): Form<DeleteOrgForm>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };

    match state
        .identity_service
        .delete_organization(user.id, payload.org_id)
        .await
    {
        | Ok(_) => redirect_notice("/admin/orgs", "org_deleted"),
        | Err(e) => redirect_error("/admin/orgs", e),
    }
}

/// Create new team in organization
async fn create_team_form(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Form(payload): Form<NewTeamForm>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };

    match state
        .identity_service
        .create_team(
            user.id,
            apich_db::CreateTeamDto {
                org_id: payload.org_id,
                parent_team_id: payload.parent_team_id,
                name: payload.name.trim().to_string(),
                slug: payload.slug.trim().to_lowercase(),
                description: payload.description.filter(|s| !s.trim().is_empty()),
                chat_url: payload.chat_url.filter(|s| !s.trim().is_empty()),
                meeting_url: payload.meeting_url.filter(|s| !s.trim().is_empty()),
                drive_url: payload.drive_url.filter(|s| !s.trim().is_empty()),
                ai_agent_url: payload.ai_agent_url.filter(|s| !s.trim().is_empty()),
            },
        )
        .await
    {
        | Ok(_) => redirect_notice("/admin/orgs", "team_created"),
        | Err(e) => redirect_error("/admin/orgs", e),
    }
}

/// Update existing team
async fn update_team_form(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Form(payload): Form<UpdateTeamForm>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };

    let parent_id = match payload.parent_team_id {
        | Some(ref s) if !s.trim().is_empty() => {
            match Uuid::parse_str(s.trim()) {
                | Ok(id) => Some(Some(id)),
                | Err(_) => None,
            }
        },
        | Some(_) => Some(None), // cleared to root
        | None => None,
    };

    let dto = apich_db::UpdateTeamDto {
        name: Some(payload.name.trim().to_string()),
        slug: Some(payload.slug.trim().to_lowercase()),
        description: payload.description.filter(|s| !s.trim().is_empty()),
        parent_team_id: parent_id,
        chat_url: payload.chat_url.filter(|s| !s.trim().is_empty()),
        meeting_url: payload.meeting_url.filter(|s| !s.trim().is_empty()),
        drive_url: payload.drive_url.filter(|s| !s.trim().is_empty()),
        ai_agent_url: payload.ai_agent_url.filter(|s| !s.trim().is_empty()),
    };

    match state
        .identity_service
        .update_team(user.id, payload.team_id, dto)
        .await
    {
        | Ok(_) => redirect_notice("/admin/orgs", "team_updated"),
        | Err(e) => redirect_error("/admin/orgs", e),
    }
}

/// Delete team
async fn delete_team_form(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Form(payload): Form<DeleteTeamForm>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };

    match state
        .identity_service
        .delete_team(user.id, payload.team_id)
        .await
    {
        | Ok(_) => redirect_notice("/admin/orgs", "team_deleted"),
        | Err(e) => redirect_error("/admin/orgs", e),
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
        | Some(id) => id,
        | None => return redirect_error("/admin/orgs", "Target user not found"),
    };

    match state
        .identity_service
        .add_org_member(user.id, payload.org_id, target_uid, &payload.role)
        .await
    {
        | Ok(_) => redirect_notice("/admin/orgs", "member_added"),
        | Err(e) => redirect_error("/admin/orgs", e),
    }
}

/// Remove member from organization
async fn remove_org_member_form(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Form(payload): Form<RemoveOrgMemberForm>,
) -> Response {
    match state
        .identity_service
        .remove_org_member(user.id, payload.org_id, payload.user_id)
        .await
    {
        | Ok(_) => redirect_notice("/admin/orgs", "member_removed"),
        | Err(e) => redirect_error("/admin/orgs", e),
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
        | Some(id) => id,
        | None => return redirect_error("/admin/orgs", "Target user not found"),
    };

    match state
        .identity_service
        .add_team_member(user.id, payload.team_id, target_uid, &payload.role)
        .await
    {
        | Ok(_) => redirect_notice("/admin/orgs", "team_member_added"),
        | Err(e) => redirect_error("/admin/orgs", e),
    }
}

/// Remove member from team
async fn remove_team_member_form(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Form(payload): Form<RemoveTeamMemberForm>,
) -> Response {
    match state
        .identity_service
        .remove_team_member(user.id, payload.team_id, payload.user_id)
        .await
    {
        | Ok(_) => redirect_notice("/admin/orgs", "team_member_removed"),
        | Err(e) => redirect_error("/admin/orgs", e),
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
        | Some(u) => u,
        | None => return redirect_to_login(&headers, "/admin/platform"),
    };

    if !user.is_platform_admin {
        return (StatusCode::FORBIDDEN, Html("<h3>403 Access Denied: Administrator access required</h3><p><a href='/'>Return to dashboard</a></p>")).into_response();
    }

    let i18n = get_i18n(&headers, Some(&params));
    let repo = state.db.repository();
    let settings = repo.get_system_settings().await.unwrap_or_default();
    let sso_clients = repo.list_oauth_clients().await.unwrap_or_default();
    let notice = params.get("notice").cloned();
    let error = params.get("error").cloned();

    let html = crate::app::components::render_document(move || {
        leptos::prelude::view! {
            <crate::app::pages::admin_platform::AdminPlatformPage
                user=user
                settings=settings
                sso_clients=sso_clients
                notice=notice
                error=error
                i18n=i18n
                current_path="/admin/platform".to_string()
            />
        }
    });
    Html(html).into_response()
}

/// Update platform registration or SMTP settings
async fn update_platform_settings_form(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Form(payload): Form<UpdatePlatformSettingsForm>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };

    if !user.is_platform_admin {
        return (StatusCode::FORBIDDEN, "Administrator access required").into_response();
    }

    let is_smtp = payload.section.as_deref() == Some("smtp");
    let smtp_enabled = if is_smtp {
        Some(
            payload.smtp_enabled.as_deref() == Some("true")
                || payload.smtp_enabled.as_deref() == Some("on"),
        )
    } else {
        None
    };
    let smtp_use_tls = if is_smtp {
        Some(
            payload.smtp_use_tls.as_deref() == Some("true")
                || payload.smtp_use_tls.as_deref() == Some("on"),
        )
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
        | Ok(_) => redirect_notice("/admin/platform", "Configuration updated successfully"),
        | Err(e) => redirect_error("/admin/platform", e),
    }
}

/// Test SMTP delivery
async fn test_smtp_form(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Form(payload): Form<TestSmtpForm>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };

    if !user.is_platform_admin {
        return (StatusCode::FORBIDDEN, "Administrator access required").into_response();
    }

    let settings = match state.db.repository().get_system_settings().await {
        | Ok(s) => s,
        | Err(e) => {
            return redirect_error("/admin/platform", format!("Failed to read settings: {}", e))
        },
    };

    match state
        .mailer
        .send_test_email(&settings, &payload.test_email)
        .await
    {
        | Ok(_) => {
            redirect_notice(
                "/admin/platform",
                &format!(
                    "Verification email dispatched successfully to {}",
                    payload.test_email
                ),
            )
        },
        | Err(e) => redirect_error("/admin/platform", format!("SMTP delivery error: {}", e)),
    }
}

/// Project Tables (SQLite) Page
async fn project_table_page(
    auth: Option<AuthUser>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id_or_slug): Path<String>,
    Query(params): Query<TablePageQuery>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };

    let i18n = get_i18n(&headers, None);

    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => {
            return (
                StatusCode::NOT_FOUND,
                Html("<h3>Project not found 404</h3>"),
            )
                .into_response()
        },
    };

    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_access {
        return (StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }

    let databases =
        SqliteTableService::discover_databases(&project.storage_path).unwrap_or_default();

    let selected_file = params
        .file
        .clone()
        .or_else(|| databases.first().map(|d| d.relative_path.clone()));

    let (schema, selected_table, table_data) = if let Some(ref file) = selected_file {
        if let Ok(full_path) = SqliteTableService::resolve_db_path(&project.storage_path, file) {
            let schema = SqliteTableService::get_database_schema(&full_path).ok();
            let selected_tbl = params.table.clone().or_else(|| {
                schema
                    .as_ref()
                    .and_then(|s| s.tables.first().map(|t| t.name.clone()))
            });
            let td = if let Some(tbl) = &selected_tbl {
                SqliteTableService::get_table_data(
                    &full_path,
                    tbl,
                    params.page.unwrap_or(1),
                    50,
                    params.sort_by.as_deref(),
                    params.sort_order.as_deref(),
                    params.search.as_deref(),
                )
                .ok()
            } else {
                None
            };
            (schema, selected_tbl, td)
        } else {
            (None, None, None)
        }
    } else {
        (None, None, None)
    };

    let notebook_cells = notebook_cells_for(
        &project.storage_path,
        selected_file.as_deref(),
        selected_table.as_deref(),
    );

    let column_view = match (&selected_file, &selected_table) {
        | (Some(file), Some(tbl)) => {
            SqliteTableService::resolve_db_path(&project.storage_path, file)
                .ok()
                .and_then(|p| SqliteTableService::get_column_view(&p, tbl).ok())
                .unwrap_or_default()
        },
        | _ => Default::default(),
    };
    let query_history = selected_file
        .as_deref()
        .and_then(|file| SqliteTableService::resolve_db_path(&project.storage_path, file).ok())
        .and_then(|p| SqliteTableService::list_query_history(&p).ok())
        .unwrap_or_default();

    let mode = params.mode.unwrap_or_else(|| "grid".to_string());
    let current_path = format!("/projects/{}/table", project.id);
    let is_org_admin = state
        .identity_service
        .is_org_or_team_admin(user.id)
        .await
        .unwrap_or(false);
    let search = params.search;
    let notice = params.notice;
    let error = params.error;

    let html = crate::app::components::render_document(move || {
        leptos::prelude::view! {
            <crate::app::pages::table_page::TablePage
                user=user
                is_org_or_team_admin=is_org_admin
                project=project
                databases=databases
                selected_file=selected_file
                schema=schema
                selected_table=selected_table
                table_data=table_data
                column_view=column_view
                query_history=query_history
                sql_query="".to_string()
                sql_result=None
                mode=mode
                search=search
                notice=notice
                error=error
                notebook_cells=notebook_cells
                i18n=i18n
                current_path=current_path
            />
        }
    });

    Html(html).into_response()
}

/// Shared by both places `TablePage` gets rendered (the plain GET and the SQL-console POST
/// redirect-render) -- loads the current table's notebook cells, or an empty list when there's
/// no selected file/table yet to attach one to.
fn notebook_cells_for(
    storage_path: &str,
    file: Option<&str>,
    table: Option<&str>,
) -> Vec<crate::services::sqlite_table::NotebookCell> {
    let (Some(file), Some(table)) = (file, table) else {
        return Vec::new();
    };
    let Ok(db_path) = SqliteTableService::resolve_db_path(storage_path, file) else {
        return Vec::new();
    };
    SqliteTableService::list_notebook_cells(&db_path, table).unwrap_or_default()
}

/// Execute SQL Query or Statement in Project SQLite Database
async fn execute_table_sql_action(
    auth: Option<AuthUser>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id_or_slug): Path<String>,
    Form(payload): Form<TableSqlForm>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };

    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => return Redirect::to("/").into_response(),
    };

    let can_manage =
        IdentityPermissionResolver::can_manage_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_manage {
        return redirect_error(
            &format!(
                "/projects/{}/table?file={}&mode=sql",
                project.id,
                urlencoding::encode(&payload.file)
            ),
            "Permission denied to execute SQL",
        );
    }

    let full_path = match SqliteTableService::resolve_db_path(&project.storage_path, &payload.file)
    {
        | Ok(p) => p,
        | Err(e) => {
            return redirect_error(
                &format!(
                    "/projects/{}/table?file={}&mode=sql",
                    project.id,
                    urlencoding::encode(&payload.file)
                ),
                e,
            )
        },
    };

    let i18n = get_i18n(&headers, None);
    let is_org_admin = state
        .identity_service
        .is_org_or_team_admin(user.id)
        .await
        .unwrap_or(false);
    let databases =
        SqliteTableService::discover_databases(&project.storage_path).unwrap_or_default();
    let schema = SqliteTableService::get_database_schema(&full_path).ok();

    let max_rows = payload.max_rows.unwrap_or(500);
    match SqliteTableService::execute_sql(&full_path, &payload.sql, max_rows) {
        | Ok(sql_res) => {
            let _ = SqliteTableService::record_query_history(&full_path, payload.sql.trim());
            let current_path = format!("/projects/{}/table", project.id);
            let first_table = schema
                .as_ref()
                .and_then(|s| s.tables.first().map(|t| t.name.clone()));
            let table_data = if let Some(ref t) = first_table {
                SqliteTableService::get_table_data(&full_path, t, 1, 50, None, None, None).ok()
            } else {
                None
            };
            let notice = Some(sql_res.message.clone());
            let selected_file = Some(payload.file.clone());
            let notebook_cells = notebook_cells_for(
                &project.storage_path,
                selected_file.as_deref(),
                first_table.as_deref(),
            );
            let column_view = first_table
                .as_deref()
                .and_then(|t| SqliteTableService::get_column_view(&full_path, t).ok())
                .unwrap_or_default();
            let query_history =
                SqliteTableService::list_query_history(&full_path).unwrap_or_default();

            let html = crate::app::components::render_document(move || {
                leptos::prelude::view! {
                    <crate::app::pages::table_page::TablePage
                        user=user
                        is_org_or_team_admin=is_org_admin
                        project=project
                        databases=databases
                        selected_file=selected_file
                        schema=schema
                        selected_table=first_table
                        table_data=table_data
                        column_view=column_view
                        query_history=query_history
                        sql_query=payload.sql
                        sql_result=Some(sql_res)
                        mode="sql".to_string()
                        search=None
                        notice=notice
                        error=None
                        notebook_cells=notebook_cells
                        i18n=i18n
                        current_path=current_path
                    />
                }
            });
            Html(html).into_response()
        },
        | Err(e) => {
            redirect_error(
                &format!(
                    "/projects/{}/table?file={}&mode=sql",
                    project.id,
                    urlencoding::encode(&payload.file)
                ),
                e,
            )
        },
    }
}

/// Create a new SQLite database file
async fn create_table_db_action(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Path(id_or_slug): Path<String>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };

    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => return Redirect::to("/").into_response(),
    };

    let can_manage =
        IdentityPermissionResolver::can_manage_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_manage {
        return redirect_error(
            &format!("/projects/{}/table", project.id),
            "Permission denied",
        );
    }

    let db_path = std::path::Path::new(&project.storage_path).join("data.db");
    if let Err(e) = SqliteTableService::create_empty_database(&db_path) {
        return redirect_error(&format!("/projects/{}/table", project.id), e);
    }

    if let Ok(vcs) = ProjectVcs::open_or_init(&project.storage_path) {
        let _ = vcs.snapshot_if_changed("Initialized scientific SQLite database data.db");
    }

    redirect_notice(
        &format!("/projects/{}/table?file=data.db", project.id),
        "Database data.db created successfully.",
    )
}

/// Direct Spreadsheet cell edit API
async fn table_cell_edit_action(
    auth: Option<AuthUser>,
    Path(id_or_slug): Path<String>,
    State(state): State<AppState>,
    Form(payload): Form<TableCellEditForm>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => {
            return (
                StatusCode::UNAUTHORIZED,
                axum::Json(serde_json::json!({ "error": "Unauthorized" })),
            )
                .into_response()
        },
    };

    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => {
            return (
                StatusCode::NOT_FOUND,
                axum::Json(serde_json::json!({ "error": "Project not found" })),
            )
                .into_response()
        },
    };

    let can_manage =
        IdentityPermissionResolver::can_manage_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_manage {
        return (
            StatusCode::FORBIDDEN,
            axum::Json(serde_json::json!({ "error": "Forbidden" })),
        )
            .into_response();
    }

    // Must go through `resolve_db_path`, not a raw path join: `payload.file` can be a `.csv`
    // path (the grid's "file" hidden field always echoes whatever was selected), and a raw join
    // would hand a plain-text CSV file straight to a SQLite writer -- resolve_db_path is what
    // knows to redirect a `.csv` selection to its real SQLite-backed `.csv.table` sibling.
    let db_path = match crate::services::sqlite_table::SqliteTableService::resolve_db_path(
        &project.storage_path,
        &payload.file,
    ) {
        | Ok(p) => p,
        | Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        },
    };
    if let Err(e) = crate::services::sqlite_table::SqliteTableService::update_cell(
        &db_path,
        &payload.table,
        &payload.row_id_col,
        &payload.row_id_val,
        &payload.col,
        &payload.val,
    ) {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response();
    }

    if let Ok(vcs) = ProjectVcs::open_or_init(&project.storage_path) {
        let _ = vcs.snapshot_if_changed(format!(
            "Edit cell in {}: table {} row {} col {}",
            payload.file, payload.table, payload.row_id_val, payload.col
        ));
    }

    axum::Json(serde_json::json!({ "ok": true })).into_response()
}

#[derive(Debug, Deserialize)]
struct TableCellStyleJson {
    file: String,
    table: String,
    row_id: i64,
    col: String,
    bold: bool,
    italic: bool,
    color: Option<String>,
    bg_color: Option<String>,
}

/// Direct spreadsheet cell *formatting* API (bold/italic/text-color/background-color) --
/// separate from `table_cell_edit_action` (which changes the cell's real data), this only ever
/// touches the `_apich_cell_styles` metadata table alongside it.
async fn table_cell_style_action(
    auth: Option<AuthUser>,
    Path(id_or_slug): Path<String>,
    State(state): State<AppState>,
    axum::Json(payload): axum::Json<TableCellStyleJson>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => {
            return (
                StatusCode::UNAUTHORIZED,
                axum::Json(serde_json::json!({ "error": "Unauthorized" })),
            )
                .into_response()
        },
    };

    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => {
            return (
                StatusCode::NOT_FOUND,
                axum::Json(serde_json::json!({ "error": "Project not found" })),
            )
                .into_response()
        },
    };

    let can_manage =
        IdentityPermissionResolver::can_manage_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_manage {
        return (
            StatusCode::FORBIDDEN,
            axum::Json(serde_json::json!({ "error": "Forbidden" })),
        )
            .into_response();
    }

    let db_path = match crate::services::sqlite_table::SqliteTableService::resolve_db_path(
        &project.storage_path,
        &payload.file,
    ) {
        | Ok(p) => p,
        | Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        },
    };

    let style = crate::services::sqlite_table::CellStyle {
        bold: payload.bold,
        italic: payload.italic,
        color: payload.color,
        bg_color: payload.bg_color,
    };

    if let Err(e) = crate::services::sqlite_table::SqliteTableService::set_cell_style(
        &db_path,
        &payload.table,
        payload.row_id,
        &payload.col,
        &style,
    ) {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response();
    }

    axum::Json(serde_json::json!({ "ok": true })).into_response()
}

/// Add empty/default row to spreadsheet table
async fn table_row_add_action(
    auth: Option<AuthUser>,
    Path(id_or_slug): Path<String>,
    State(state): State<AppState>,
    Form(payload): Form<TableRowAddForm>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };

    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => return Redirect::to("/").into_response(),
    };

    let can_manage =
        IdentityPermissionResolver::can_manage_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_manage {
        return (StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }

    let db_path = match crate::services::sqlite_table::SqliteTableService::resolve_db_path(
        &project.storage_path,
        &payload.file,
    ) {
        | Ok(p) => p,
        | Err(_) => {
            return Redirect::to(&format!(
                "/projects/{}/table?file={}&error=table_not_found",
                project.id,
                urlencoding::encode(&payload.file)
            ))
            .into_response()
        },
    };
    if let Ok(rowid) =
        crate::services::sqlite_table::SqliteTableService::insert_row(&db_path, &payload.table)
    {
        if let Ok(vcs) = ProjectVcs::open_or_init(&project.storage_path) {
            let _ =
                vcs.snapshot_if_changed(format!("Added row #{} to table {}", rowid, payload.table));
        }
    }

    Redirect::to(&format!(
        "/projects/{}/table?file={}&table={}",
        project.id,
        urlencoding::encode(&payload.file),
        urlencoding::encode(&payload.table)
    ))
    .into_response()
}

/// Delete selected row from spreadsheet table
async fn table_row_delete_action(
    auth: Option<AuthUser>,
    Path(id_or_slug): Path<String>,
    State(state): State<AppState>,
    Form(payload): Form<TableRowDeleteForm>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };

    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => return Redirect::to("/").into_response(),
    };

    let can_manage =
        IdentityPermissionResolver::can_manage_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_manage {
        return (StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }

    let db_path = match crate::services::sqlite_table::SqliteTableService::resolve_db_path(
        &project.storage_path,
        &payload.file,
    ) {
        | Ok(p) => p,
        | Err(_) => {
            return Redirect::to(&format!(
                "/projects/{}/table?file={}&error=table_not_found",
                project.id,
                urlencoding::encode(&payload.file)
            ))
            .into_response()
        },
    };
    let _ = crate::services::sqlite_table::SqliteTableService::delete_row(
        &db_path,
        &payload.table,
        &payload.row_id_col,
        &payload.row_id_val,
    );

    if let Ok(vcs) = ProjectVcs::open_or_init(&project.storage_path) {
        let _ = vcs.snapshot_if_changed(format!("Deleted row from table {}", payload.table));
    }

    Redirect::to(&format!(
        "/projects/{}/table?file={}&table={}",
        project.id,
        urlencoding::encode(&payload.file),
        urlencoding::encode(&payload.table)
    ))
    .into_response()
}

/// Export table to CSV
async fn table_export_action(
    auth: Option<AuthUser>,
    Path(id_or_slug): Path<String>,
    Query(query): Query<TableExportQuery>,
    State(state): State<AppState>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };

    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => {
            return (
                StatusCode::NOT_FOUND,
                Html("<h3>Project not found 404</h3>"),
            )
                .into_response()
        },
    };

    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_access {
        return (StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }

    let db_path = match crate::services::sqlite_table::SqliteTableService::resolve_db_path(
        &project.storage_path,
        &query.file,
    ) {
        | Ok(p) => p,
        | Err(e) => {
            return (StatusCode::NOT_FOUND, Html(format!("<h3>{}</h3>", e))).into_response()
        },
    };
    let format = query.format.as_deref().unwrap_or("csv");
    let (content_type, ext, result) = match format {
        | "json" => {
            (
                "application/json; charset=utf-8",
                "json",
                crate::services::sqlite_table::SqliteTableService::export_json(
                    &db_path,
                    &query.table,
                ),
            )
        },
        | "tsv" => {
            (
                "text/tab-separated-values; charset=utf-8",
                "tsv",
                crate::services::sqlite_table::SqliteTableService::export_tsv(
                    &db_path,
                    &query.table,
                ),
            )
        },
        | "md" | "markdown" => {
            (
                "text/markdown; charset=utf-8",
                "md",
                crate::services::sqlite_table::SqliteTableService::export_markdown(
                    &db_path,
                    &query.table,
                ),
            )
        },
        | _ => {
            (
                "text/csv; charset=utf-8",
                "csv",
                crate::services::sqlite_table::SqliteTableService::export_csv(
                    &db_path,
                    &query.table,
                ),
            )
        },
    };
    match result {
        | Ok(data) => {
            let filename = format!("{}.{}", query.table, ext);
            (
                [
                    (header::CONTENT_TYPE, content_type.to_string()),
                    (
                        header::CONTENT_DISPOSITION,
                        format!("attachment; filename=\"{}\"", filename),
                    ),
                ],
                data,
            )
                .into_response()
        },
        | Err(e) => {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(format!("Failed to export {}: {}", format.to_uppercase(), e)),
            )
                .into_response()
        },
    }
}

#[derive(Debug, Deserialize)]
pub struct ColumnViewForm {
    pub file: String,
    pub table: String,
    pub action: String,
    #[serde(default)]
    pub column: String,
    #[serde(default)]
    pub mode: String,
}

/// Hide/show a column or move it left/right in the grid's display order (or reset back to the
/// table's real schema order). Purely a display preference -- never touches the actual table
/// schema, so a hidden column is still fully visible/queryable via the SQL console below the grid.
async fn table_column_view_action(
    auth: Option<AuthUser>,
    Path(id_or_slug): Path<String>,
    State(state): State<AppState>,
    Form(payload): Form<ColumnViewForm>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };
    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => {
            return (
                StatusCode::NOT_FOUND,
                Html("<h3>Project not found 404</h3>"),
            )
                .into_response()
        },
    };
    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_access {
        return (StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }
    let db_path = match crate::services::sqlite_table::SqliteTableService::resolve_db_path(
        &project.storage_path,
        &payload.file,
    ) {
        | Ok(p) => p,
        | Err(e) => {
            return (StatusCode::NOT_FOUND, Html(format!("<h3>{}</h3>", e))).into_response()
        },
    };

    let all_columns: Vec<String> =
        crate::services::sqlite_table::SqliteTableService::get_database_schema(&db_path)
            .ok()
            .and_then(|s| s.tables.into_iter().find(|t| t.name == payload.table))
            .map(|t| t.columns.into_iter().map(|c| c.name).collect())
            .unwrap_or_default();

    let mut config = crate::services::sqlite_table::SqliteTableService::get_column_view(
        &db_path,
        &payload.table,
    )
    .unwrap_or_default();

    match payload.action.as_str() {
        | "hide" => {
            if !config.hidden.iter().any(|c| c == &payload.column) {
                config.hidden.push(payload.column.clone());
            }
        },
        | "show" => {
            config.hidden.retain(|c| c != &payload.column);
        },
        | "move-left" | "move-right" => {
            let visible = config.apply(&all_columns);
            let mut order = visible.clone();
            if let Some(pos) = order.iter().position(|c| c == &payload.column) {
                if payload.action == "move-left" && pos > 0 {
                    order.swap(pos, pos - 1);
                } else if payload.action == "move-right" && pos + 1 < order.len() {
                    order.swap(pos, pos + 1);
                }
            }
            for c in &all_columns {
                if !order.contains(c) {
                    order.push(c.clone());
                }
            }
            config.order = order;
        },
        | "reset" => {
            config = crate::services::sqlite_table::ColumnViewConfig::default();
        },
        | _ => {},
    }

    if let Err(e) = crate::services::sqlite_table::SqliteTableService::set_column_view(
        &db_path,
        &payload.table,
        &config,
    ) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("Failed to save column view: {}", e)),
        )
            .into_response();
    }

    let mode = if payload.mode.is_empty() {
        "grid".to_string()
    } else {
        payload.mode.clone()
    };
    Redirect::to(&format!(
        "/projects/{}/table?file={}&table={}&mode={}",
        project.id,
        urlencoding::encode(&payload.file),
        urlencoding::encode(&payload.table),
        mode
    ))
    .into_response()
}

/// Import CSV data into table
async fn table_import_action(
    auth: Option<AuthUser>,
    Path(id_or_slug): Path<String>,
    State(state): State<AppState>,
    Form(payload): Form<TableImportForm>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };

    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => return Redirect::to("/").into_response(),
    };

    let can_manage =
        IdentityPermissionResolver::can_manage_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_manage {
        return (StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }

    let db_path = match crate::services::sqlite_table::SqliteTableService::resolve_db_path(
        &project.storage_path,
        &payload.file,
    ) {
        | Ok(p) => p,
        | Err(e) => {
            return Redirect::to(&format!(
                "/projects/{}/table?file={}&error={}",
                project.id,
                urlencoding::encode(&payload.file),
                urlencoding::encode(&e.to_string())
            ))
            .into_response()
        },
    };
    match crate::services::sqlite_table::SqliteTableService::import_csv(
        &db_path,
        &payload.table,
        &payload.csv_data,
    ) {
        | Ok(count) => {
            if let Ok(vcs) = ProjectVcs::open_or_init(&project.storage_path) {
                let _ = vcs.snapshot_if_changed(format!(
                    "Imported {} rows into table {}",
                    count, payload.table
                ));
            }
            Redirect::to(&format!(
                "/projects/{}/table?file={}&table={}&notice=csv_imported",
                project.id,
                urlencoding::encode(&payload.file),
                urlencoding::encode(&payload.table)
            ))
            .into_response()
        },
        | Err(e) => {
            Redirect::to(&format!(
                "/projects/{}/table?file={}&table={}&error={}",
                project.id,
                urlencoding::encode(&payload.file),
                urlencoding::encode(&payload.table),
                urlencoding::encode(&e.to_string())
            ))
            .into_response()
        },
    }
}

async fn notebook_cell_create_action(
    auth: Option<AuthUser>,
    Path(id_or_slug): Path<String>,
    State(state): State<AppState>,
    Form(payload): Form<NotebookCellCreateForm>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };
    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => return Redirect::to("/").into_response(),
    };
    let can_manage =
        IdentityPermissionResolver::can_manage_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_manage {
        return (StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }
    let redirect_url = format!(
        "/projects/{}/table?file={}&table={}&mode=notebook",
        project.id,
        urlencoding::encode(&payload.file),
        urlencoding::encode(&payload.table)
    );
    let db_path = match crate::services::sqlite_table::SqliteTableService::resolve_db_path(
        &project.storage_path,
        &payload.file,
    ) {
        | Ok(p) => p,
        | Err(e) => {
            return Redirect::to(&format!(
                "{}&error={}",
                redirect_url,
                urlencoding::encode(&e.to_string())
            ))
            .into_response()
        },
    };
    let starter = crate::services::sqlite_table::SqliteTableService::notebook_cell_starter(
        &payload.language,
        &payload.table,
    );
    match crate::services::sqlite_table::SqliteTableService::create_notebook_cell(
        &db_path,
        &payload.table,
        &payload.language,
        &starter,
    ) {
        | Ok(_) => Redirect::to(&redirect_url).into_response(),
        | Err(e) => {
            Redirect::to(&format!(
                "{}&error={}",
                redirect_url,
                urlencoding::encode(&e.to_string())
            ))
            .into_response()
        },
    }
}

async fn notebook_cell_delete_action(
    auth: Option<AuthUser>,
    Path(id_or_slug): Path<String>,
    State(state): State<AppState>,
    Form(payload): Form<NotebookCellDeleteForm>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };
    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => return Redirect::to("/").into_response(),
    };
    let can_manage =
        IdentityPermissionResolver::can_manage_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_manage {
        return (StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }
    let redirect_url = format!(
        "/projects/{}/table?file={}&table={}&mode=notebook",
        project.id,
        urlencoding::encode(&payload.file),
        urlencoding::encode(&payload.table)
    );
    if let Ok(db_path) = crate::services::sqlite_table::SqliteTableService::resolve_db_path(
        &project.storage_path,
        &payload.file,
    ) {
        let _ = crate::services::sqlite_table::SqliteTableService::delete_notebook_cell(
            &db_path,
            payload.cell_id,
        );
    }
    Redirect::to(&redirect_url).into_response()
}

/// Move a notebook cell up or down one slot in its table's cell order.
async fn notebook_cell_move_action(
    auth: Option<AuthUser>,
    Path(id_or_slug): Path<String>,
    State(state): State<AppState>,
    Form(payload): Form<NotebookCellMoveForm>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };
    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => return Redirect::to("/").into_response(),
    };
    let can_manage =
        IdentityPermissionResolver::can_manage_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_manage {
        return (StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }
    let redirect_url = format!(
        "/projects/{}/table?file={}&table={}&mode=notebook",
        project.id,
        urlencoding::encode(&payload.file),
        urlencoding::encode(&payload.table)
    );
    if let Ok(db_path) = crate::services::sqlite_table::SqliteTableService::resolve_db_path(
        &project.storage_path,
        &payload.file,
    ) {
        let _ = crate::services::sqlite_table::SqliteTableService::move_notebook_cell(
            &db_path,
            payload.cell_id,
            &payload.direction,
        );
    }
    Redirect::to(&redirect_url).into_response()
}

/// Runs one notebook cell's Python/R code for real inside the project's sandbox, against the
/// real table it's attached to. Design constraints (see `NotebookCell`'s own doc comment for the
/// full reasoning): no persistent kernel -- each run is the cell's current code, written to a
/// real scratch script file (`.apich_notebook_tmp/cell_<id>.py`/`.r`, a plain project-root scratch
/// dir, deliberately separate from `.apich/` which is apich-vcs's own reserved bookkeeping
/// directory -- writing notebook scratch files into that would risk colliding with VCS internals)
/// and executed via the *existing* `run_script_in_sandbox_with_env`, which already handles
/// interpreter dispatch and plot-image capture; this handler's only new piece is resolving the
/// table's real SQLite path and handing it to the cell as `APICH_TABLE_DB`. Both the code and the
/// fresh output are persisted back into the cell's own row (`update_notebook_cell`) so a page
/// reload doesn't lose either.
async fn notebook_run_cell_action(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Path(id_or_slug): Path<String>,
    body: axum::body::Bytes,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Unauthorized"})),
            )
                .into_response()
        },
    };
    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Project not found"})),
            )
                .into_response()
        },
    };
    let can_manage =
        IdentityPermissionResolver::can_manage_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_manage {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Forbidden"}))).into_response();
    }

    let payload: NotebookRunCellForm = match serde_json::from_slice(&body) {
        | Ok(p) => p,
        | Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        },
    };

    let db_path = match crate::services::sqlite_table::SqliteTableService::resolve_db_path(
        &project.storage_path,
        &payload.file,
    ) {
        | Ok(p) => p,
        | Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        },
    };

    // `payload.file` is the *logical* file name shown in the UI (e.g. "assets/data.csv"), which
    // can differ from the real on-disk SQLite file `resolve_db_path` just resolved it to (e.g.
    // "assets/data.csv.table" -- see that function's own `.csv` handling). The cell's own process
    // needs the container-relative path to the *real* file, not the logical one, or
    // `sqlite3.connect(os.environ["APICH_TABLE_DB"])` opens the wrong file entirely (the raw CSV
    // itself in the `.csv` case, which fails with "file is not a database").
    let table_db_rel = match db_path.strip_prefix(&project.storage_path) {
        | Ok(rel) => rel.to_string_lossy().to_string(),
        | Err(_) => payload.file.clone(),
    };

    let ext = if payload.language == "r" {
        "r"
    } else {
        "py"
    };
    let rel_script_path = format!(".apich_notebook_tmp/cell_{}.{}", payload.cell_id, ext);
    if let Err(e) = state
        .project_manager
        .write_file(project.id, user.id, &rel_script_path, &payload.code)
        .await
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response();
    }

    let result = state
        .project_manager
        .run_script_in_sandbox_with_env(
            project.id,
            user.id,
            &rel_script_path,
            "",
            &[("APICH_TABLE_DB", &table_db_rel)],
        )
        .await;

    match result {
        | Ok(res) => {
            let images: Vec<crate::services::sqlite_table::NotebookCellImage> = res
                .output_images
                .iter()
                .map(|img| {
                    crate::services::sqlite_table::NotebookCellImage {
                        name: img.name.clone(),
                        data_uri: img.data_uri.clone(),
                    }
                })
                .collect();
            let combined_output = format!(
                "{}{}",
                res.stdout,
                if res.stderr.is_empty() {
                    String::new()
                } else {
                    format!("\n--- stderr ---\n{}", res.stderr)
                }
            );
            let _ = crate::services::sqlite_table::SqliteTableService::update_notebook_cell(
                &db_path,
                payload.cell_id,
                &payload.code,
                Some(&combined_output),
                Some(&images),
            );
            Json(json!({
                "success": res.success,
                "output": combined_output,
                "execution_time_ms": res.execution_time_ms,
                "output_images": images,
            }))
            .into_response()
        },
        | Err(e) => {
            let _ = crate::services::sqlite_table::SqliteTableService::update_notebook_cell(
                &db_path,
                payload.cell_id,
                &payload.code,
                Some(&e.to_string()),
                Some(&[]),
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        },
    }
}

/// Dedicated Unified Note Studio (.anote, .note.md, etc.)
async fn project_note_page(
    auth: Option<AuthUser>,
    headers: HeaderMap,
    Path(id_or_slug): Path<String>,
    Query(query): Query<NoteQuery>,
    State(state): State<AppState>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };

    let i18n = get_i18n(&headers, None);
    let repo = state.db.repository();
    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => {
            return (
                StatusCode::NOT_FOUND,
                Html("<h3>Project not found 404</h3>"),
            )
                .into_response()
        },
    };

    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_access {
        return (StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }

    let is_org_admin = state
        .identity_service
        .is_org_or_team_admin(user.id)
        .await
        .unwrap_or(false);

    let file_path = if let Some(ref f) = query.file {
        f.clone()
    } else {
        let all_files = state
            .project_manager
            .list_files(project.id)
            .await
            .unwrap_or_default();
        if let Some(note) = all_files.iter().find(|f| f.category == "note") {
            note.path.clone()
        } else {
            "lab_notebook.anote".to_string()
        }
    };

    let raw_content = state
        .project_manager
        .read_file(project.id, &file_path)
        .await
        .unwrap_or_default();
    let (meta, body_content) =
        crate::services::knowledge_sync::KnowledgeSyncService::parse_unified_note(&raw_content);
    let headings =
        crate::services::knowledge_sync::KnowledgeSyncService::extract_headings(&body_content);

    // The note page is the one integrated space plan.md asks for ("一体化空间"): notes, wiki,
    // whiteboard, calendar, and kanban all live here as tabs, not scattered across pages nothing
    // links to. Backlinks (below) are derived from the same graph the Wiki tab renders.
    let graph = KnowledgeSyncService::build_knowledge_graph(&project.storage_path).unwrap_or(
        crate::services::knowledge_sync::KnowledgeGraph {
            nodes: Vec::new(),
            edges: Vec::new(),
        },
    );
    let kanban_columns = KnowledgeSyncService::parse_kanban_columns(
        &project.settings,
        KnowledgeSyncService::default_kanban_columns(
            i18n.kanban_col_todo(),
            i18n.kanban_col_in_progress(),
            i18n.kanban_col_done(),
        ),
    );
    let kanban = KnowledgeSyncService::build_kanban_board(
        &project.storage_path,
        &kanban_columns,
        i18n.kanban_col_unsorted(),
    )
    .unwrap_or(crate::services::knowledge_sync::KanbanBoard {
        columns: Vec::new(),
        total_tasks: 0,
        completed_tasks: 0,
    });
    let own_kanban_templates =
        crate::ui::template_handlers::list_own_templates_of_kind(&state, user.id, "kanban").await;
    let visible_kanban_templates =
        crate::ui::template_handlers::list_visible_templates_of_kind(&state, user.id, "kanban")
            .await;
    let own_note_templates =
        crate::ui::template_handlers::list_own_templates_of_kind(&state, user.id, "note").await;
    let visible_note_templates =
        crate::ui::template_handlers::list_visible_templates_of_kind(&state, user.id, "note").await;
    let calendar =
        KnowledgeSyncService::extract_calendar_events(&project.storage_path).unwrap_or_default();
    let note_id = std::path::Path::new(&file_path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| file_path.clone());
    let backlinks: Vec<String> = graph
        .edges
        .iter()
        .filter(|e| e.target.eq_ignore_ascii_case(&note_id))
        .map(|e| e.source.clone())
        .collect();

    let rendered_markdown =
        crate::services::document_renderer::DocumentRenderer::render_markdown_interactive(
            &body_content,
            &file_path,
            project.id,
        );
    let file_share = state
        .project_manager
        .get_file_share(project.id, &file_path)
        .await
        .ok()
        .flatten();
    let all_users = repo.list_users().await.unwrap_or_default();

    let view = query.view.unwrap_or_else(|| "editor".to_string());
    let current_path = format!(
        "/projects/{}/note?file={}&view={}",
        project.id,
        urlencoding::encode(&file_path),
        view
    );
    let notice = query.notice;
    let error = query.error;

    let html = crate::app::components::render_document(move || {
        leptos::prelude::view! {
            <crate::app::pages::note_page::NotePage
                user=user
                is_org_or_team_admin=is_org_admin
                project=project
                file_path=file_path
                body_content=body_content
                meta=meta
                headings=headings
                backlinks=backlinks
                rendered_markdown_html=rendered_markdown.html
                task_count=rendered_markdown.tasks.len()
                completed_task_count=rendered_markdown.tasks.iter().filter(|t| t.completed).count()
                graph=graph
                kanban=kanban
                calendar=calendar
                own_kanban_templates=own_kanban_templates
                visible_kanban_templates=visible_kanban_templates
                own_note_templates=own_note_templates
                visible_note_templates=visible_note_templates
                file_share=file_share
                all_users=all_users
                active_view=view
                notice=notice
                error=error
                i18n=i18n
                current_path=current_path
            />
        }
    });

    Html(html).into_response()
}

/// Save Unified Note content & metadata
async fn save_note_action(
    auth: Option<AuthUser>,
    Path(id_or_slug): Path<String>,
    State(state): State<AppState>,
    Form(payload): Form<SaveNoteForm>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };

    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => return Redirect::to("/").into_response(),
    };

    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_access {
        return (StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }

    let tags: Vec<String> = payload
        .meta_tags
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().trim_start_matches('#').to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Read-modify-write: the editor form only ever touches title/author/tags/body, so we must
    // preserve created_at and whiteboard from the existing file rather than dropping them.
    let existing_raw = state
        .project_manager
        .read_file(project.id, &payload.file)
        .await
        .unwrap_or_default();
    let (existing_meta, _) =
        crate::services::knowledge_sync::KnowledgeSyncService::parse_unified_note(&existing_raw);

    let meta = crate::services::knowledge_sync::UnifiedNoteMeta {
        title: if payload.meta_title.trim().is_empty() {
            "Untitled Note".to_string()
        } else {
            payload.meta_title.trim().to_string()
        },
        created_at: existing_meta
            .created_at
            .or_else(|| Some(chrono::Utc::now().to_rfc3339())),
        updated_at: Some(chrono::Utc::now().to_rfc3339()),
        tags,
        author: payload.meta_author.filter(|s| !s.trim().is_empty()),
        whiteboard: existing_meta.whiteboard,
    };

    let full_content =
        crate::services::knowledge_sync::KnowledgeSyncService::serialize_unified_note(
            &meta,
            &payload.body,
        );

    if let Err(e) = state
        .project_manager
        .write_file(project.id, user.id, &payload.file, &full_content)
        .await
    {
        return Redirect::to(&format!(
            "/projects/{}/note?file={}&error={}",
            project.id,
            urlencoding::encode(&payload.file),
            urlencoding::encode(&e.to_string())
        ))
        .into_response();
    }

    let view = payload.view.as_deref().unwrap_or("editor");
    Redirect::to(&format!(
        "/projects/{}/note?file={}&view={}&notice=note_saved",
        project.id,
        urlencoding::encode(&payload.file),
        view
    ))
    .into_response()
}

#[derive(Debug, Deserialize)]
pub struct SaveWhiteboardForm {
    pub file: String,
    pub whiteboard_json: String,
}

/// Persist the whiteboard's stroke data into the note's frontmatter, preserving the note's
/// title/tags/author/body untouched. This is the real save path the whiteboard panel needed --
/// previously "Export PNG" was the only thing the whiteboard could do; nothing was ever written
/// back to the .anote file despite the format already having a `whiteboard` field for it.
async fn save_whiteboard_action(
    auth: Option<AuthUser>,
    headers: HeaderMap,
    Path(id_or_slug): Path<String>,
    State(state): State<AppState>,
    body: axum::body::Bytes,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Unauthorized"})),
            )
                .into_response()
        },
    };

    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Project not found"})),
            )
                .into_response()
        },
    };

    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_access {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Forbidden"}))).into_response();
    }

    let payload: SaveWhiteboardForm = match parse_payload(&headers, &body) {
        | Ok(p) => p,
        | Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        },
    };

    let whiteboard_value: serde_json::Value = match serde_json::from_str(&payload.whiteboard_json) {
        | Ok(v) => v,
        | Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("Invalid whiteboard JSON: {}", e)})),
            )
                .into_response()
        },
    };

    let existing_raw = state
        .project_manager
        .read_file(project.id, &payload.file)
        .await
        .unwrap_or_default();
    let (mut meta, body_content) =
        crate::services::knowledge_sync::KnowledgeSyncService::parse_unified_note(&existing_raw);
    meta.whiteboard = Some(whiteboard_value);
    meta.updated_at = Some(chrono::Utc::now().to_rfc3339());
    if meta.created_at.is_none() {
        meta.created_at = Some(chrono::Utc::now().to_rfc3339());
    }

    let full_content =
        crate::services::knowledge_sync::KnowledgeSyncService::serialize_unified_note(
            &meta,
            &body_content,
        );
    if let Err(e) = state
        .project_manager
        .write_file(project.id, user.id, &payload.file, &full_content)
        .await
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response();
    }

    Json(json!({"status": "saved"})).into_response()
}

/// The Kanban/Wiki/Calendar hub used to be a standalone page here. plan.md calls for one
/// integrated space aggregating notes, wiki, whiteboard, calendar, and kanban -- they're tabs
/// on the unified note page now (`NotePage`, in `note_page.rs`). This route survives only to
/// send old bookmarks/links to the right place.
async fn project_knowledge_page(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Path(id_or_slug): Path<String>,
    Query(params): Query<KnowledgePageQuery>,
) -> Response {
    if auth.is_none() {
        return Redirect::to("/login").into_response();
    }

    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => {
            return (
                StatusCode::NOT_FOUND,
                Html("<h3>Project not found 404</h3>"),
            )
                .into_response()
        },
    };

    let view = params.view.unwrap_or_else(|| "kanban".to_string());
    Redirect::to(&format!("/projects/{}/note?view={}", project.id, view)).into_response()
}

#[derive(Debug, Deserialize)]
pub struct CreateNotePageForm {
    pub title: String,
    #[serde(default)]
    pub view: String,
}

/// Manually create a new page in the unified note space -- previously the *only* way to start a
/// new note-kind file was the generic Files tab's "+ New File" picker (pick a filename with the
/// right extension, then a template), which doesn't read as "add a page" the way this feature's
/// own Wiki view (a graph of notes and the [[links]] between them) implies it should support.
/// This gives that view its own direct "+ New Page" affordance, and lets a placeholder node (a
/// [[Link]] with no backing file yet, shown in the Wiki tab as "Placeholder / Uncreated") be
/// materialized into a real page with one click, using its exact label as both filename and title
/// so the link that pointed to it starts resolving immediately.
async fn create_note_page_action(
    auth: Option<AuthUser>,
    Path(id_or_slug): Path<String>,
    State(state): State<AppState>,
    Form(payload): Form<CreateNotePageForm>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };
    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => return Redirect::to("/").into_response(),
    };
    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_access {
        return (StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }

    let title = payload.title.trim();
    let view = if payload.view.is_empty() {
        "wiki".to_string()
    } else {
        payload.view.clone()
    };
    if title.is_empty() {
        return redirect_error(
            &format!("/projects/{}/note?view={}", project.id, view),
            "Page title cannot be empty",
        );
    }

    let base_slug =
        crate::services::knowledge_sync::KnowledgeSyncService::slugify_page_title(title);
    let safe_title = title.replace('"', "'");
    let content = format!("---\ntitle: \"{}\"\n---\n\n# {}\n", safe_title, title);

    let mut filename = format!("{}.anote", base_slug);
    let mut attempt = 2;
    loop {
        match state
            .project_manager
            .create_file_with_content(project.id, user.id, &filename, &content)
            .await
        {
            | Ok(()) => break,
            | Err(WebError::Conflict(_)) if attempt <= 50 => {
                filename = format!("{}-{}.anote", base_slug, attempt);
                attempt += 1;
            },
            | Err(e) => {
                return redirect_error(&format!("/projects/{}/note?view={}", project.id, view), e)
            },
        }
    }

    Redirect::to(&format!(
        "/projects/{}/note?file={}&view=editor&notice={}",
        project.id,
        urlencoding::encode(&filename),
        urlencoding::encode("Page created.")
    ))
    .into_response()
}

/// Toggle task status in the physical Markdown document
async fn toggle_task_action(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Path(id_or_slug): Path<String>,
    Form(payload): Form<ToggleTaskForm>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };

    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => return Redirect::to("/").into_response(),
    };

    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_access {
        return (StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }

    let view = payload.view.as_deref().unwrap_or("kanban");
    let is_done_column = payload
        .done
        .as_deref()
        .map(|d| d == "true")
        .unwrap_or_else(|| payload.status == "done");
    if let Err(e) = KnowledgeSyncService::update_task_status(
        &project.storage_path,
        &payload.file,
        payload.line_number,
        &payload.status,
        is_done_column,
    ) {
        return redirect_error(&format!("/projects/{}/note?view={}", project.id, view), e);
    }

    if let Ok(vcs) = ProjectVcs::open_or_init(&project.storage_path) {
        let _ = vcs.snapshot_if_changed(format!(
            "Updated task in {}:{} to {}",
            payload.file, payload.line_number, payload.status
        ));
    }

    Redirect::to(&format!("/projects/{}/note?view={}", project.id, view)).into_response()
}

#[derive(Debug, Deserialize)]
pub struct KanbanAddColumnForm {
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct KanbanRenameColumnForm {
    pub col_id: String,
    pub title: String,
    /// A missing checkbox field means "unchecked" in a standard HTML form post, not absent --
    /// `Option` here would silently treat every unchecked submission as "leave `is_done`
    /// unchanged" instead of "set it to false".
    #[serde(default)]
    pub is_done: bool,
}

#[derive(Debug, Deserialize)]
pub struct KanbanDeleteColumnForm {
    pub col_id: String,
}

#[derive(Debug, Deserialize)]
pub struct KanbanMoveColumnForm {
    pub col_id: String,
    pub direction: String, // "left" | "right"
}

/// Loads a project's current Kanban column layout (falling back to the localized default 3
/// columns if it's never been customized) -- shared by all 4 column-management handlers below so
/// each mutates the same starting point `render_kanban_column_settings`'s forms were rendered
/// against.
pub(crate) async fn load_project_kanban_columns(
    state: &AppState,
    project: &apich_db::Project,
    i18n: crate::ui::i18n::I18n,
) -> Vec<crate::services::knowledge_sync::KanbanColumnDef> {
    let _ = state;
    crate::services::knowledge_sync::KnowledgeSyncService::parse_kanban_columns(
        &project.settings,
        crate::services::knowledge_sync::KnowledgeSyncService::default_kanban_columns(
            i18n.kanban_col_todo(),
            i18n.kanban_col_in_progress(),
            i18n.kanban_col_done(),
        ),
    )
}

async fn kanban_add_column_action(
    auth: Option<AuthUser>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id_or_slug): Path<String>,
    Form(payload): Form<KanbanAddColumnForm>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };
    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => return Redirect::to("/").into_response(),
    };
    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_access {
        return (StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }

    let i18n = get_i18n(&headers, None);
    let title = payload.title.trim().to_string();
    if !title.is_empty() {
        let mut columns = load_project_kanban_columns(&state, &project, i18n).await;
        let id = crate::services::knowledge_sync::KnowledgeSyncService::slugify_kanban_column_id(
            &title, &columns,
        );
        columns.push(crate::services::knowledge_sync::KanbanColumnDef {
            id,
            title,
            is_done: false,
        });
        let mut settings = project.settings.clone();
        settings["kanban_columns"] =
            crate::services::knowledge_sync::KnowledgeSyncService::serialize_kanban_columns(
                &columns,
            );
        let _ = state
            .db
            .repository()
            .update_project_settings(project.id, settings)
            .await;
    }

    Redirect::to(&format!("/projects/{}/note?view=kanban", project.id)).into_response()
}

async fn kanban_rename_column_action(
    auth: Option<AuthUser>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id_or_slug): Path<String>,
    Form(payload): Form<KanbanRenameColumnForm>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };
    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => return Redirect::to("/").into_response(),
    };
    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_access {
        return (StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }

    let i18n = get_i18n(&headers, None);
    let title = payload.title.trim().to_string();
    if !title.is_empty() {
        let mut columns = load_project_kanban_columns(&state, &project, i18n).await;
        if let Some(col) = columns.iter_mut().find(|c| c.id == payload.col_id) {
            col.title = title;
            col.is_done = payload.is_done;
        }
        let mut settings = project.settings.clone();
        settings["kanban_columns"] =
            crate::services::knowledge_sync::KnowledgeSyncService::serialize_kanban_columns(
                &columns,
            );
        let _ = state
            .db
            .repository()
            .update_project_settings(project.id, settings)
            .await;
    }

    Redirect::to(&format!("/projects/{}/note?view=kanban", project.id)).into_response()
}

async fn kanban_delete_column_action(
    auth: Option<AuthUser>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id_or_slug): Path<String>,
    Form(payload): Form<KanbanDeleteColumnForm>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };
    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => return Redirect::to("/").into_response(),
    };
    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_access {
        return (StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }

    let i18n = get_i18n(&headers, None);
    let mut columns = load_project_kanban_columns(&state, &project, i18n).await;
    // Never delete down to zero columns -- `render_kanban`'s cycling and `Unsorted` fallback both
    // assume at least one real column exists (the delete button is also disabled client-side for
    // the last remaining column, this is the server-side backstop for that same invariant).
    if columns.len() > 1 {
        columns.retain(|c| c.id != payload.col_id);
        let mut settings = project.settings.clone();
        settings["kanban_columns"] =
            crate::services::knowledge_sync::KnowledgeSyncService::serialize_kanban_columns(
                &columns,
            );
        let _ = state
            .db
            .repository()
            .update_project_settings(project.id, settings)
            .await;
    }

    Redirect::to(&format!("/projects/{}/note?view=kanban", project.id)).into_response()
}

async fn kanban_move_column_action(
    auth: Option<AuthUser>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id_or_slug): Path<String>,
    Form(payload): Form<KanbanMoveColumnForm>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };
    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => return Redirect::to("/").into_response(),
    };
    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_access {
        return (StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }

    let i18n = get_i18n(&headers, None);
    let mut columns = load_project_kanban_columns(&state, &project, i18n).await;
    if let Some(idx) = columns.iter().position(|c| c.id == payload.col_id) {
        let swap_with = if payload.direction == "left" {
            idx.checked_sub(1)
        } else {
            (idx + 1 < columns.len()).then_some(idx + 1)
        };
        if let Some(j) = swap_with {
            columns.swap(idx, j);
            let mut settings = project.settings.clone();
            settings["kanban_columns"] =
                crate::services::knowledge_sync::KnowledgeSyncService::serialize_kanban_columns(
                    &columns,
                );
            let _ = state
                .db
                .repository()
                .update_project_settings(project.id, settings)
                .await;
        }
    }

    Redirect::to(&format!("/projects/{}/note?view=kanban", project.id)).into_response()
}

/// Project Interactive Terminal Page
async fn project_terminal_page(
    auth: Option<AuthUser>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id_or_slug): Path<String>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };

    let i18n = get_i18n(&headers, None);
    let repo = state.db.repository();
    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => {
            return (
                StatusCode::NOT_FOUND,
                Html("<h3>Project not found 404</h3>"),
            )
                .into_response()
        },
    };

    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_access {
        return (StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }

    let is_org_admin = state
        .identity_service
        .is_org_or_team_admin(user.id)
        .await
        .unwrap_or(false);
    let sb = repo
        .get_project_sandbox(project.id, user.id)
        .await
        .unwrap_or(None);
    let sandbox_status = sb
        .map(|s| s.status)
        .unwrap_or_else(|| "stopped".to_string());

    let current_path = format!("/projects/{}/terminal", project.id);

    let html = crate::app::components::render_document(move || {
        leptos::prelude::view! {
            <crate::app::pages::terminal_page::TerminalPage
                user=user
                is_org_or_team_admin=is_org_admin
                project=project
                sandbox_status=sandbox_status
                notice=None
                error=None
                i18n=i18n
                current_path=current_path
            />
        }
    });

    Html(html).into_response()
}

/// Execute command in project container/workspace
async fn terminal_exec_action(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Path(id_or_slug): Path<String>,
    Form(payload): Form<TerminalExecForm>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => {
            return (
                StatusCode::UNAUTHORIZED,
                axum::Json(serde_json::json!({ "error": "Unauthorized" })),
            )
                .into_response()
        },
    };

    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => {
            return (
                StatusCode::NOT_FOUND,
                axum::Json(serde_json::json!({ "error": "Project not found" })),
            )
                .into_response()
        },
    };

    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_access {
        return (
            StatusCode::FORBIDDEN,
            axum::Json(serde_json::json!({ "error": "Forbidden" })),
        )
            .into_response();
    }

    let cmd = payload.command.trim();
    if cmd.is_empty() {
        return axum::Json(serde_json::json!({ "output": "" })).into_response();
    }

    // Runs inside the project's real sandbox container (auto-starting it if needed), not on
    // the host -- this used to shell out directly on the web server process with no isolation
    // and no permission check at all.
    match state
        .project_manager
        .exec_in_sandbox(project.id, user.id, cmd)
        .await
    {
        | Ok(output) => axum::Json(serde_json::json!({ "output": output })).into_response(),
        | Err(e) => axum::Json(serde_json::json!({ "error": e.to_string() })).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct RunScriptForm {
    pub file: String,
    pub args: Option<String>,
    pub stdin: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ShareFileForm {
    pub file: String,
    pub mode: String,
    pub role: String,
    pub allowed_users: Option<String>,
    pub redirect_to: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProjectSharingForm {
    pub is_public: Option<serde_json::Value>,
    pub default_role: Option<String>,
    pub invite_user_id: Option<String>,
    pub invite_role: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RenderDocPreviewForm {
    pub file: String,
    pub content: String,
    pub engine: Option<String>,
}

/// Validate a requested LaTeX engine name against what's actually installed in the sandbox image
/// (`texlive-xetex` + `texlive-luatex`, see `docker/Containerfile.sandbox`) -- an unrecognized or
/// missing value falls back to pdflatex rather than passing an arbitrary string through to `exec`.
fn sanitize_latex_engine(engine: Option<&str>) -> &'static str {
    match engine {
        | Some("xelatex") => "xelatex",
        | Some("lualatex") => "lualatex",
        | _ => "pdflatex",
    }
}

/// Helper to parse either JSON or URL-encoded form body
fn parse_payload<T: for<'de> Deserialize<'de>>(
    headers: &HeaderMap,
    body: &[u8],
) -> Result<T, WebError> {
    let is_json = headers
        .get(header::CONTENT_TYPE)
        .and_then(|h| h.to_str().ok())
        .map(|ct| ct.contains("application/json"))
        .unwrap_or(false)
        || body.first() == Some(&b'{');

    if is_json {
        serde_json::from_slice(body).map_err(|e| WebError::BadRequest(e.to_string()))
    } else {
        let s = std::str::from_utf8(body).map_err(|e| WebError::BadRequest(e.to_string()))?;
        let mut map = serde_json::Map::new();
        for pair in s.split('&') {
            if pair.is_empty() {
                continue;
            }
            let mut parts = pair.splitn(2, '=');
            if let Some(key) = parts.next() {
                let key_decoded = urlencoding::decode(key).unwrap_or_default();
                let val_decoded = parts
                    .next()
                    .and_then(|v| urlencoding::decode(v).ok())
                    .unwrap_or_default();
                if !key_decoded.is_empty() {
                    if let Ok(num) = val_decoded.parse::<i64>() {
                        map.insert(
                            key_decoded.to_string(),
                            serde_json::Value::Number(num.into()),
                        );
                    } else if val_decoded == "true" {
                        map.insert(key_decoded.to_string(), serde_json::Value::Bool(true));
                    } else if val_decoded == "false" {
                        map.insert(key_decoded.to_string(), serde_json::Value::Bool(false));
                    } else {
                        map.insert(
                            key_decoded.to_string(),
                            serde_json::Value::String(val_decoded.to_string()),
                        );
                    }
                }
            }
        }
        match serde_json::from_value(serde_json::Value::Object(map.clone())) {
            | Ok(val) => Ok(val),
            | Err(first_err) => {
                let mut str_map = serde_json::Map::new();
                for (k, v) in map {
                    let s_val = match v {
                        | serde_json::Value::String(s) => s,
                        | serde_json::Value::Bool(b) => b.to_string(),
                        | serde_json::Value::Number(n) => n.to_string(),
                        | other => other.to_string(),
                    };
                    str_map.insert(k, serde_json::Value::String(s_val));
                }
                serde_json::from_value(serde_json::Value::Object(str_map))
                    .map_err(|_| WebError::BadRequest(first_err.to_string()))
            },
        }
    }
}

/// Interactive script runner action (executes Python, Bash, R scripts, captures plots and exit code)
async fn run_script_action(
    auth: Option<AuthUser>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id_or_slug): Path<String>,
    body: axum::body::Bytes,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Unauthorized"})),
            )
                .into_response()
        },
    };

    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Project not found"})),
            )
                .into_response()
        },
    };

    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_access {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Forbidden"}))).into_response();
    }

    let payload: RunScriptForm = match parse_payload(&headers, &body) {
        | Ok(p) => p,
        | Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        },
    };

    let args_str = payload.args.as_deref().unwrap_or("");

    match state
        .project_manager
        .run_script_in_sandbox(project.id, user.id, &payload.file, args_str)
        .await
    {
        | Ok(res) => {
            Json(json!({
                "success": res.success,
                "exit_code": res.exit_code,
                "stdout": res.stdout,
                "stderr": res.stderr,
                "execution_time_ms": res.execution_time_ms,
                "output_images": res.output_images
            }))
            .into_response()
        },
        | Err(e) => {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        },
    }
}

/// Toggle task status via AJAX and return updated note body to maintain editor cursor
async fn toggle_task_ajax_action(
    auth: Option<AuthUser>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id_or_slug): Path<String>,
    body: axum::body::Bytes,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Unauthorized"})),
            )
                .into_response()
        },
    };

    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Project not found"})),
            )
                .into_response()
        },
    };

    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_access {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Forbidden"}))).into_response();
    }

    let payload: ToggleTaskForm = match parse_payload(&headers, &body) {
        | Ok(p) => p,
        | Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        },
    };

    // The inline note-view checkbox only ever sends "done" or "todo" (see note_editor.rs's
    // `toggle_task`), so deriving `is_done_column` from that literal id reproduces the exact
    // pre-custom-columns behavior of this endpoint.
    let is_done_column = payload.status == "done";
    if let Err(e) = KnowledgeSyncService::update_task_status(
        &project.storage_path,
        &payload.file,
        payload.line_number,
        &payload.status,
        is_done_column,
    ) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": e.to_string()})),
        )
            .into_response();
    }

    if let Ok(vcs) = ProjectVcs::open_or_init(&project.storage_path) {
        let _ = vcs.snapshot_if_changed(format!(
            "Live toggle task in {}:{} to {}",
            payload.file, payload.line_number, payload.status
        ));
    }

    let updated_content = state
        .project_manager
        .read_file(project.id, &payload.file)
        .await
        .unwrap_or_default();
    let body = if payload.file.ends_with(".anote") {
        crate::services::knowledge_sync::KnowledgeSyncService::parse_unified_note(&updated_content)
            .1
    } else {
        updated_content
    };

    Json(json!({
        "success": true,
        "updated_body": body,
        "file": payload.file,
        "line_number": payload.line_number,
        "status": payload.status
    }))
    .into_response()
}

/// Set file-level sharing permissions
async fn share_file_action(
    auth: Option<AuthUser>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id_or_slug): Path<String>,
    body: axum::body::Bytes,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };

    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => return Redirect::to("/").into_response(),
    };

    let is_owner = project.owner_id == user.id || user.is_platform_admin;
    if !is_owner {
        return (StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }

    let payload: ShareFileForm = match parse_payload(&headers, &body) {
        | Ok(p) => p,
        | Err(e) => {
            return (StatusCode::BAD_REQUEST, Html(format!("Bad Request: {}", e))).into_response()
        },
    };

    let allowed_users: Vec<String> = payload
        .allowed_users
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let _ = state
        .project_manager
        .update_file_share(
            project.id,
            &payload.file,
            &payload.mode,
            &payload.role,
            allowed_users,
        )
        .await;

    let target = payload
        .redirect_to
        .unwrap_or_else(|| format!("/projects/{}?tab=files&notice=file_shared", project.id));
    Redirect::to(&target).into_response()
}

/// Soft-delete a project (owner or platform admin only) -- `ProjectManagerService::delete_project`
/// already existed but was never wired to a route or a UI control.
async fn delete_project_action(
    auth: Option<AuthUser>,
    Path(id_or_slug): Path<String>,
    State(state): State<AppState>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };

    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => return Redirect::to("/").into_response(),
    };

    let is_owner = project.owner_id == user.id || user.is_platform_admin;
    if !is_owner {
        return (StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }

    let _ = state.project_manager.delete_project(project.id).await;
    Redirect::to("/?notice=project_deleted").into_response()
}

#[derive(Debug, Deserialize)]
pub struct SetVigilantModeForm {
    pub enabled: bool,
}

/// Toggle a project's "vigilant mode" (bugs.md's own term): when on, the VCS timeline flags
/// snapshots without a valid GPG signature as unverified instead of showing them like signed ones.
async fn set_vigilant_mode_action(
    auth: Option<AuthUser>,
    Path(id_or_slug): Path<String>,
    State(state): State<AppState>,
    Form(payload): Form<SetVigilantModeForm>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };

    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => return Redirect::to("/").into_response(),
    };

    let is_owner = project.owner_id == user.id || user.is_platform_admin;
    if !is_owner {
        return (StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }

    let _ = state
        .db
        .repository()
        .set_project_vigilant_mode(project.id, payload.enabled)
        .await;
    Redirect::to(&format!("/projects/{}?tab=vcs", project.id)).into_response()
}

/// Update project-level sharing and access settings
async fn update_project_sharing_action(
    auth: Option<AuthUser>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id_or_slug): Path<String>,
    body: axum::body::Bytes,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => return Redirect::to("/login").into_response(),
    };

    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => return Redirect::to("/").into_response(),
    };

    let is_owner = project.owner_id == user.id || user.is_platform_admin;
    if !is_owner {
        return (StatusCode::FORBIDDEN, Html("<h3>403 Forbidden</h3>")).into_response();
    }

    let payload: UpdateProjectSharingForm = match parse_payload(&headers, &body) {
        | Ok(p) => p,
        | Err(e) => {
            return (StatusCode::BAD_REQUEST, Html(format!("Bad Request: {}", e))).into_response()
        },
    };

    let is_pub = match payload.is_public {
        | Some(serde_json::Value::Bool(b)) => b,
        | Some(serde_json::Value::String(ref s)) => s == "true" || s == "on" || s == "1",
        | _ => false,
    };
    let mode = if is_pub {
        "public"
    } else {
        "private"
    };
    let role = payload.default_role.as_deref().unwrap_or("read_only");
    let _ = state
        .project_manager
        .update_project_share_settings(project.id, mode, role)
        .await;

    if let Some(ref uid_str) = payload.invite_user_id {
        if let Ok(target_uid) = uid_str.parse::<uuid::Uuid>() {
            let role_str = payload.invite_role.as_deref().unwrap_or("read_only");
            let _ = state
                .db
                .repository()
                .add_project_member(project.id, target_uid, role_str)
                .await;
        }
    }

    Redirect::to(&format!(
        "/projects/{}?tab=members&notice=sharing_updated",
        project.id
    ))
    .into_response()
}

/// Live render document preview for Typst and Markdown
async fn render_doc_preview_action(
    auth: Option<AuthUser>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id_or_slug): Path<String>,
    body: axum::body::Bytes,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Unauthorized"})),
            )
                .into_response()
        },
    };

    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Project not found"})),
            )
                .into_response()
        },
    };

    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_access {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Forbidden"}))).into_response();
    }

    let payload: RenderDocPreviewForm = match parse_payload(&headers, &body) {
        | Ok(p) => p,
        | Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        },
    };

    let is_script = crate::services::document_renderer::DocumentRenderer::is_script(&payload.file);
    if is_script {
        return Json(json!({
            "is_script": true,
            "success": true
        }))
        .into_response();
    }

    // The outline panel (`document_editor.rs`/`note_editor.rs`'s `outline_items`) used to be
    // computed once from the page's initial server-rendered `headings` prop and never touched
    // again -- every other part of this same hot-reload response (pages/HTML/PDF) already
    // recomputes live as the user types, but headings silently didn't, so adding or renaming a
    // section left the outline showing stale text (or none at all for a document created empty)
    // until the next full page load. Recomputed here on every debounced call, from the same
    // extension-dispatching extractor the initial page load already uses, so both islands can
    // just swap this into their outline signal alongside the rest of the live preview.
    let headings: Vec<serde_json::Value> =
        KnowledgeSyncService::extract_headings_for_file(&payload.content, &payload.file)
            .into_iter()
            .map(|h| json!({ "level": h.level, "text": h.text, "line": h.line }))
            .collect();

    if payload.file.ends_with(".typ") || payload.file.ends_with(".slide.typ") {
        let ws = &project.storage_path;
        let _ = state
            .project_manager
            .write_file(project.id, user.id, &payload.file, &payload.content)
            .await;
        let res = crate::services::document_renderer::DocumentRenderer::compile_typst(
            ws,
            &payload.file,
            true,
        )
        .await;
        Json(json!({
            "is_typst": true,
            "success": res.success,
            "pages": res.pages_svg,
            "total_pages": res.total_pages,
            "error": res.error_message,
            "headings": headings
        }))
        .into_response()
    } else if payload.file.ends_with(".tex") || payload.file.ends_with(".latex") {
        // Real hot reload for LaTeX: save what's currently in the editor (the same autosave
        // side effect the Typst branch above already has), then let the browser reload the
        // `<iframe src="/projects/:id/editor/latex-pdf?...">` itself -- that endpoint always
        // recompiles on request, so a real PDF byte stream is never the shape to return from
        // this JSON endpoint; this just reports whether the save+compile succeeded so the
        // island knows whether reloading the iframe is worth doing.
        let _ = state
            .project_manager
            .write_file(project.id, user.id, &payload.file, &payload.content)
            .await;
        let engine = sanitize_latex_engine(payload.engine.as_deref());
        match state.project_manager.compile_latex_in_sandbox(project.id, user.id, &payload.file, engine).await {
            Ok(Ok(_pdf_bytes)) => Json(json!({ "is_latex": true, "success": true, "error": null, "headings": headings })).into_response(),
            Ok(Err(compile_log)) => Json(json!({ "is_latex": true, "success": false, "error": compile_log, "headings": headings })).into_response(),
            Err(e) => Json(json!({ "is_latex": true, "success": false, "error": e.to_string(), "headings": headings })).into_response(),
        }
    } else {
        let res = crate::services::document_renderer::DocumentRenderer::render_markdown_interactive(
            &payload.content,
            &payload.file,
            project.id,
        );
        Json(json!({
            "is_markdown": true,
            "success": true,
            "html": res.html,
            "headings": headings
        }))
        .into_response()
    }
}

/// AI Copilot and BYOK assistant endpoint
async fn ai_chat_action(
    auth: Option<AuthUser>,
    headers: HeaderMap,
    State(_state): State<AppState>,
    body: axum::body::Bytes,
) -> Response {
    if auth.is_none() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Unauthorized"})),
        )
            .into_response();
    }

    let payload: crate::services::ai_service::AiChatRequest = match parse_payload(&headers, &body) {
        | Ok(p) => p,
        | Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        },
    };

    match crate::services::ai_service::AiAssistantService::chat(payload).await {
        | Ok(resp) => {
            Json(json!({
                "success": true,
                "response": resp.reply,
                "reply": resp.reply,
                "suggested_code": resp.suggested_code,
                "provider": resp.provider
            }))
            .into_response()
        },
        | Err(e) => {
            Json(json!({
                "success": false,
                "error": e.to_string(),
                "response": format!("Error: {}", e),
                "reply": format!("Error: {}", e)
            }))
            .into_response()
        },
    }
}

/// Which CLI coding agents (claude/codex/opencode/aider/goose) are actually installed in this
/// project's sandbox image -- checked live against a real container, not assumed, since an
/// image build can silently skip an install step (see docker/Containerfile.sandbox).
async fn agent_status_action(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Path(id_or_slug): Path<String>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "Unauthorized" })),
            )
                .into_response()
        },
    };

    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "Project not found" })),
            )
                .into_response()
        },
    };

    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_access {
        return (StatusCode::FORBIDDEN, Json(json!({ "error": "Forbidden" }))).into_response();
    }

    match state
        .project_manager
        .agent_availability(project.id, user.id)
        .await
    {
        | Ok(list) => {
            let agents: Vec<_> = list
                .into_iter()
                .map(|(kind, available)| {
                    json!({
                        "id": kind.binary(),
                        "name": kind.display_name(),
                        "available": available,
                        "login_support": login_support_json(kind.login_support()),
                    })
                })
                .collect();
            Json(json!({ "success": true, "agents": agents })).into_response()
        },
        | Err(e) => Json(json!({ "success": false, "error": e.to_string() })).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct AgentRunForm {
    pub agent: String,
    pub prompt: String,
    pub api_key: Option<String>,
}

/// Run a CLI coding agent non-interactively inside the project's real sandbox container,
/// against the real project files -- per plan.md this is not a hosted chat API call, it's a
/// real agent process with a real shell in the real workspace. Requires write access since the
/// agent can (and by default, in its non-interactive mode, will) edit files.
async fn agent_run_action(
    auth: Option<AuthUser>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id_or_slug): Path<String>,
    body: axum::body::Bytes,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "Unauthorized" })),
            )
                .into_response()
        },
    };

    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "Project not found" })),
            )
                .into_response()
        },
    };

    let can_manage =
        IdentityPermissionResolver::can_manage_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_manage {
        return (StatusCode::FORBIDDEN, Json(json!({ "error": "Forbidden: running an agent requires write access to this project" }))).into_response();
    }

    let payload: AgentRunForm = match parse_payload(&headers, &body) {
        | Ok(p) => p,
        | Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response()
        },
    };

    let kind = match apich_sandbox::tools::AgentKind::parse(&payload.agent) {
        | Some(k) => k,
        | None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("Unknown agent: {}", payload.agent) })),
            )
                .into_response()
        },
    };

    let prompt = payload.prompt.trim();
    if prompt.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Prompt is required" })),
        )
            .into_response();
    }

    let api_key = payload.api_key.as_deref().filter(|k| !k.trim().is_empty());

    match state
        .project_manager
        .run_agent_in_sandbox(project.id, user.id, kind, prompt, api_key)
        .await
    {
        | Ok(res) => {
            Json(json!({
                "success": res.success(),
                "exit_code": res.exit_code,
                "stdout": res.stdout_lossy(),
                "stderr": res.stderr_lossy(),
                "agent": kind.display_name(),
            }))
            .into_response()
        },
        | Err(e) => Json(json!({ "success": false, "error": e.to_string() })).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct AgentLoginStartForm {
    pub agent: String,
}

fn login_support_json(support: apich_sandbox::tools::LoginSupport) -> &'static str {
    match support {
        | apich_sandbox::tools::LoginSupport::None => "none",
        | apich_sandbox::tools::LoginSupport::DeviceCode => "device_code",
        | apich_sandbox::tools::LoginSupport::PasteCodeBack => "paste_code_back",
    }
}

/// Starts a real account-login flow for a CLI agent (`claude auth login`, `codex login
/// --device-auth`) inside the project's sandbox -- not another shape of API-key entry. Once
/// this succeeds, the agent's own credentials live in the container and later `agent/run` calls
/// need no `api_key` param at all. Requires write access, same as running an agent.
async fn agent_login_start_action(
    auth: Option<AuthUser>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id_or_slug): Path<String>,
    body: axum::body::Bytes,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "Unauthorized" })),
            )
                .into_response()
        },
    };

    let project = match resolve_project(&state, &id_or_slug).await {
        | Some(p) => p,
        | None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "Project not found" })),
            )
                .into_response()
        },
    };

    let can_manage =
        IdentityPermissionResolver::can_manage_project(state.db.pool(), user.id, project.id)
            .await
            .unwrap_or(false);
    if !can_manage {
        return (StatusCode::FORBIDDEN, Json(json!({ "error": "Forbidden: logging in an agent requires write access to this project" }))).into_response();
    }

    let payload: AgentLoginStartForm = match parse_payload(&headers, &body) {
        | Ok(p) => p,
        | Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response()
        },
    };

    let kind = match apich_sandbox::tools::AgentKind::parse(&payload.agent) {
        | Some(k) => k,
        | None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("Unknown agent: {}", payload.agent) })),
            )
                .into_response()
        },
    };

    if kind.login_support() == apich_sandbox::tools::LoginSupport::None {
        return (StatusCode::BAD_REQUEST, Json(json!({
            "error": format!("{} has no real account-login flow -- use an API key instead", kind.display_name())
        }))).into_response();
    }

    match state.project_manager.start_agent_login(project.id, user.id, kind).await {
        Ok(Some(session_id)) => Json(json!({
            "success": true,
            "session_id": session_id,
            "login_support": login_support_json(kind.login_support()),
        })).into_response(),
        Ok(None) => (StatusCode::BAD_REQUEST, Json(json!({
            "error": format!("{} has no real account-login flow -- use an API key instead", kind.display_name())
        }))).into_response(),
        Err(e) => Json(json!({ "success": false, "error": e.to_string() })).into_response(),
    }
}

/// Poll a running or finished agent-login session for its accumulated output (the OAuth URL and
/// code appear here as the real CLI prints them) and current status.
async fn agent_login_status_action(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Path((id_or_slug, session_id)): Path<(String, Uuid)>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "Unauthorized" })),
            )
                .into_response()
        },
    };

    if resolve_project(&state, &id_or_slug).await.is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Project not found" })),
        )
            .into_response();
    }

    match state.project_manager.agent_login_snapshot(session_id).await {
        | Some((agent, owner_user_id, output, status)) => {
            if owner_user_id != user.id {
                return (StatusCode::FORBIDDEN, Json(json!({ "error": "Forbidden" })))
                    .into_response();
            }
            let (status_str, exit_code) = match status {
                | crate::services::AgentLoginStatus::Running => ("running", None),
                | crate::services::AgentLoginStatus::Succeeded => ("succeeded", Some(0)),
                | crate::services::AgentLoginStatus::Failed { exit_code } => {
                    ("failed", Some(exit_code))
                },
            };
            Json(json!({
                "success": true,
                "agent": agent.display_name(),
                "output": output,
                "status": status_str,
                "exit_code": exit_code,
                "login_support": login_support_json(agent.login_support()),
            }))
            .into_response()
        },
        | None => {
            (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "Login session not found or expired" })),
            )
                .into_response()
        },
    }
}

#[derive(Debug, Deserialize)]
pub struct AgentLoginCodeForm {
    pub code: String,
}

/// Submit the code a user copied from their browser after completing an agent's OAuth flow
/// (only meaningful for `PasteCodeBack` agents like Claude Code; `DeviceCode` agents like Codex
/// poll on their own and don't need this).
async fn agent_login_submit_code_action(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Path((id_or_slug, session_id)): Path<(String, Uuid)>,
    Form(payload): Form<AgentLoginCodeForm>,
) -> Response {
    let AuthUser(user) = match auth {
        | Some(u) => u,
        | None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "Unauthorized" })),
            )
                .into_response()
        },
    };

    if resolve_project(&state, &id_or_slug).await.is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Project not found" })),
        )
            .into_response();
    }

    let code = payload.code.trim();
    if code.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Code is required" })),
        )
            .into_response();
    }

    match state
        .project_manager
        .submit_agent_login_input(session_id, user.id, code)
        .await
    {
        | Ok(()) => Json(json!({ "success": true })).into_response(),
        | Err(e) => Json(json!({ "success": false, "error": e.to_string() })).into_response(),
    }
}
