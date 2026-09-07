use crate::app::styles::EMBEDDED_CSS;
use crate::services::project_manager::{ConflictFileView, GitStatusView};
use crate::ui::i18n::I18n;
use apich_db::{
    Organization, OrgMemberWithUser, Project, ProjectMemberWithUser, SystemSettings,
    Team, TeamMemberWithUser, TeamTreeNode, User,
};
use apich_vcs::Snapshot;
use std::collections::HashMap;

/// Render standard HTML5 page shell with embedded styling and language tags
pub fn render_page(title: &str, body_html: &str, i18n: &I18n) -> String {
    let lang_attr = if i18n.is_zh() { "zh-CN" } else { "en" };
    format!(
        r#"<!DOCTYPE html>
<html lang="{}">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{} - APICH {}</title>
    <style>{}</style>
</head>
<body>
    <div id="app">{}</div>
</body>
</html>"#,
        lang_attr,
        html_escape(title),
        i18n.brand_title(),
        EMBEDDED_CSS,
        body_html
    )
}

/// Render unified top navigation bar with user info, language switcher, and active links
pub fn render_navbar(
    user_name: &str,
    is_admin: bool,
    active_tab: &str,
    i18n: &I18n,
    current_path: &str,
) -> String {
    let admin_link = if is_admin {
        format!(
            r#"<a href="/admin/platform" class="nav-item {} nav-admin">{}</a>"#,
            if active_tab == "admin" { "active" } else { "" },
            i18n.nav_admin()
        )
    } else {
        String::new()
    };

    let lang_toggle = format!(
        r#"<a href="/set-lang?lang={}&return_to={}" class="lang-toggle" title="Switch Language">
            🌐 {}
        </a>"#,
        i18n.lang.toggle_code(),
        urlencoding::encode(current_path),
        i18n.lang.toggle_label()
    );

    format!(
        r#"<header class="navbar">
    <div class="nav-brand">
        <a href="/" class="brand-link">
            <span class="brand-badge">APICH</span>
            <span class="brand-title">{}</span>
        </a>
    </div>
    <nav class="nav-links">
        <a href="/" class="nav-item {}">{}</a>
        <a href="/admin/orgs" class="nav-item {}">{}</a>
        {}
        <a href="/settings" class="nav-item {}">{}</a>
    </nav>
    <div class="nav-user">
        {}
        <span class="user-greeting">{} {}</span>
        <form method="post" action="/logout" class="logout-form">
            <button type="submit" class="btn btn-ghost btn-sm">{}</button>
        </form>
    </div>
</header>"#,
        i18n.brand_title(),
        if active_tab == "projects" { "active" } else { "" },
        i18n.nav_projects(),
        if active_tab == "orgs" { "active" } else { "" },
        i18n.nav_teams_orgs(),
        admin_link,
        if active_tab == "settings" { "active" } else { "" },
        i18n.nav_settings(),
        lang_toggle,
        i18n.welcome_prefix(),
        html_escape(user_name),
        i18n.logout()
    )
}

/// Render login page
pub fn render_login_page(
    error: Option<&str>,
    success: Option<&str>,
    i18n: &I18n,
    current_path: &str,
) -> String {
    let error_alert = if let Some(err) = error {
        format!(r#"<div class="alert alert-danger">{}</div>"#, html_escape(err))
    } else {
        String::new()
    };

    let success_alert = if let Some(msg) = success {
        format!(r#"<div class="alert alert-success">{}</div>"#, html_escape(msg))
    } else {
        String::new()
    };

    let lang_toggle = format!(
        r#"<div style="display:flex; justify-content:flex-end; margin-bottom:1rem;">
            <a href="/set-lang?lang={}&return_to={}" class="lang-toggle">
                🌐 {}
            </a>
        </div>"#,
        i18n.lang.toggle_code(),
        urlencoding::encode(current_path),
        i18n.lang.toggle_label()
    );

    let body = format!(
        r#"<div class="auth-page">
    <div class="auth-card">
        {}
        <div class="auth-header">
            <div class="auth-logo">APICH</div>
            <h1 class="auth-title">{}</h1>
            <p class="auth-subtitle">{}</p>
        </div>
        {}
        {}
        <div class="passkey-section" style="margin-bottom: 1.25rem;">
            <button type="button" id="btn-passkey-login" class="btn btn-primary btn-block btn-lg">
                <svg class="icon" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" style="margin-right: 6px;">
                    <path d="M12 2a5 5 0 0 0-5 5v3H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-8a2 2 0 0 0-2-2h-1V7a5 5 0 0 0-5-5zm-3 5a3 3 0 1 1 6 0v3H9V7z"></path>
                </svg>
                {}
            </button>
            <p class="hint-text">{}</p>
        </div>
        <div class="divider"><span>{}</span></div>
        <form method="post" action="/login" class="auth-form">
            <div class="form-group">
                <label for="login">{}</label>
                <input type="text" id="login" name="login" required placeholder="admin@apich.org" class="form-control" autofocus>
            </div>
            <div class="form-group">
                <label for="password">{}</label>
                <input type="password" id="password" name="password" required placeholder="••••••••••••" class="form-control">
            </div>
            <button type="submit" class="btn btn-primary btn-block btn-lg" style="margin-top: 0.5rem;">{}</button>
        </form>
        <div class="auth-footer">
            <p><a href="/register">{}</a></p>
        </div>
    </div>
</div>"#,
        lang_toggle,
        i18n.sign_in_title(),
        i18n.sign_in_subtitle(),
        error_alert,
        success_alert,
        i18n.passkey_login_btn(),
        i18n.passkey_hint(),
        i18n.divider_or(),
        i18n.login_label(),
        i18n.password_label(),
        i18n.submit_login(),
        i18n.no_account_prompt()
    );

    render_page(i18n.submit_login(), &body, i18n)
}

/// Render register page
pub fn render_register_page(
    error: Option<&str>,
    i18n: &I18n,
    current_path: &str,
) -> String {
    let error_alert = if let Some(err) = error {
        format!(r#"<div class="alert alert-danger">{}</div>"#, html_escape(err))
    } else {
        String::new()
    };

    let lang_toggle = format!(
        r#"<div style="display:flex; justify-content:flex-end; margin-bottom:1rem;">
            <a href="/set-lang?lang={}&return_to={}" class="lang-toggle">
                🌐 {}
            </a>
        </div>"#,
        i18n.lang.toggle_code(),
        urlencoding::encode(current_path),
        i18n.lang.toggle_label()
    );

    let body = format!(
        r#"<div class="auth-page">
    <div class="auth-card">
        {}
        <div class="auth-header">
            <div class="auth-logo">APICH</div>
            <h1 class="auth-title">{}</h1>
            <p class="auth-subtitle">{}</p>
        </div>
        {}
        <form method="post" action="/register" class="auth-form">
            <div class="form-group">
                <label for="username">{}</label>
                <input type="text" id="username" name="username" required placeholder="quantum_dev" class="form-control">
            </div>
            <div class="form-group">
                <label for="display_name">{}</label>
                <input type="text" id="display_name" name="display_name" required placeholder="Dr. Alice Smith" class="form-control">
            </div>
            <div class="form-group">
                <label for="email">{}</label>
                <input type="email" id="email" name="email" required placeholder="researcher@lab.org" class="form-control">
            </div>
            <div class="form-group">
                <label for="invite_token">{}</label>
                <input type="text" id="invite_token" name="invite_token" placeholder="Optional or required depending on platform policy" class="form-control">
            </div>
            <div class="form-group">
                <label for="password">{}</label>
                <input type="password" id="password" name="password" required placeholder="••••••••••••" class="form-control">
            </div>
            <button type="submit" class="btn btn-primary btn-block btn-lg" style="margin-top: 0.5rem;">{}</button>
        </form>
        <div class="auth-footer">
            <p><a href="/login">{}</a></p>
        </div>
    </div>
</div>"#,
        lang_toggle,
        i18n.register_title(),
        i18n.register_subtitle(),
        error_alert,
        i18n.username(),
        i18n.display_name(),
        i18n.email(),
        i18n.invite_code_label(),
        i18n.password_label(),
        i18n.submit_register(),
        i18n.have_account_prompt()
    );

    render_page(i18n.submit_register(), &body, i18n)
}

/// Render main dashboard page with Real Data only (no mock data, no promotional slogans)
pub fn render_dashboard(
    user: &User,
    projects: &[Project],
    running_count: usize,
    i18n: &I18n,
    current_path: &str,
) -> String {
    let navbar = render_navbar(&user.display_name, user.is_platform_admin, "projects", i18n, current_path);

    let projects_html = if projects.is_empty() {
        format!(
            r#"<div class="empty-state">
                <h3 class="empty-title">{}</h3>
                <p class="empty-desc">{}</p>
                <button type="button" class="btn btn-primary" onclick="document.getElementById('modal-new-project').style.display='flex'">
                    + {}
                </button>
            </div>"#,
            i18n.no_projects(),
            i18n.no_projects_desc(),
            i18n.new_project()
        )
    } else {
        let mut html = r#"<div class="projects-grid">"#.to_string();
        for proj in projects {
            let desc = proj.description.as_deref().unwrap_or("");
            html.push_str(&format!(
                r#"<div class="project-card">
    <div class="project-card-header">
        <div>
            <h3 class="project-name"><a href="/projects/{}">{}</a></h3>
            <span class="team-tag">FastCDC + Git</span>
        </div>
        <span class="status-badge badge-idle"><span class="status-dot"></span>Ready</span>
    </div>
    <p class="project-desc">{}</p>
    <div class="project-footer">
        <div class="project-meta">
            <span>Slug: <code>{}</code></span>
        </div>
        <div class="project-actions">
            <form method="post" action="/projects/{}/sandbox/start" class="inline-form">
                <button type="submit" class="btn btn-primary btn-sm">{}</button>
            </form>
            <a href="/projects/{}" class="btn btn-secondary btn-sm">{}</a>
        </div>
    </div>
</div>"#,
                proj.id,
                html_escape(&proj.name),
                html_escape(desc),
                html_escape(&proj.slug),
                proj.id,
                i18n.launch_sandbox(),
                proj.id,
                i18n.enter_workspace()
            ));
        }
        html.push_str("</div>");
        html
    };

    let body = format!(
        r#"{}
<div class="main-content">
    <div class="page-header">
        <div>
            <h1 class="page-title">{}</h1>
            <p class="page-subtitle">{}</p>
        </div>
        <div class="header-actions">
            <button type="button" class="btn btn-primary" onclick="document.getElementById('modal-new-project').style.display='flex'">
                <span class="btn-icon">+</span> {}
            </button>
        </div>
    </div>

    <div class="stats-grid">
        <div class="stat-card">
            <span class="stat-title">{}</span>
            <span class="stat-value">{}</span>
            <span class="stat-subtitle">{}</span>
        </div>
        <div class="stat-card">
            <span class="stat-title">{}</span>
            <span class="stat-value">{}</span>
            <span class="stat-subtitle">{}</span>
        </div>
    </div>

    <div class="section-card">
        <div class="section-header">
            <h2 class="section-title">{}</h2>
            <span class="text-muted" style="font-size: 0.85rem;">{}</span>
        </div>
        {}
    </div>
</div>

<!-- New Project Modal -->
<div id="modal-new-project" style="display:none; position:fixed; inset:0; background:rgba(15,23,42,0.5); align-items:center; justify-content:center; z-index:999;">
    <div style="background:#fff; border-radius:14px; padding:2rem; width:100%; max-width:500px; box-shadow:var(--shadow-lg);">
        <h3 style="font-size:1.25rem; font-weight:700; margin-bottom:0.5rem; color:var(--text-main);">{}</h3>
        <p style="font-size:0.85rem; color:var(--text-muted); margin-bottom:1.25rem;">Initializes a native FastCDC version repository and Git bridge</p>
        <form method="post" action="/projects/new">
            <div class="form-group">
                <label for="name">Project Name</label>
                <input type="text" id="name" name="name" required placeholder="Superconducting Qubit Simulation" class="form-control">
            </div>
            <div class="form-group">
                <label for="slug">Project Slug</label>
                <input type="text" id="slug" name="slug" required placeholder="sc-qubit-sim" class="form-control">
            </div>
            <div class="form-group">
                <label for="description">{}</label>
                <textarea id="description" name="description" rows="3" class="form-control" placeholder="Brief project description and dataset targets..."></textarea>
            </div>
            <div style="display:flex; justify-content:flex-end; gap:0.75rem; margin-top:1.5rem;">
                <button type="button" class="btn btn-secondary" onclick="document.getElementById('modal-new-project').style.display='none'">{}</button>
                <button type="submit" class="btn btn-primary">{}</button>
            </div>
        </form>
    </div>
</div>"#,
        navbar,
        i18n.dashboard_title(),
        i18n.dashboard_subtitle(),
        i18n.new_project(),
        i18n.stat_total_projects(),
        projects.len(),
        i18n.stat_total_projects_sub(),
        i18n.stat_active_sandboxes(),
        running_count,
        i18n.stat_active_sandboxes_sub(),
        i18n.active_projects_section(),
        i18n.active_projects_hint(),
        projects_html,
        i18n.new_project(),
        i18n.description(),
        i18n.cancel(),
        i18n.new_project()
    );

    render_page(i18n.dashboard_title(), &body, i18n)
}

/// Render project detail page with Real Data: Sandboxes, Merge & Conflicts, Snapshots, Git, Collaborators
#[allow(clippy::too_many_arguments)]
pub fn render_project_detail_page(
    user: &User,
    project: &Project,
    sandbox_status: &str,
    branches: &[String],
    current_branch: Option<&str>,
    conflicts: &[ConflictFileView],
    snapshots: &[Snapshot],
    members: &[ProjectMemberWithUser],
    all_users: &[User],
    git_status: &GitStatusView,
    active_tab: &str,
    notice: Option<&str>,
    i18n: &I18n,
    current_path: &str,
) -> String {
    let navbar = render_navbar(&user.display_name, user.is_platform_admin, "projects", i18n, current_path);

    let is_running = sandbox_status == "running";
    let status_pill = if is_running {
        format!(r#"<span class="status-badge badge-active"><span class="status-dot"></span>{}</span>"#, i18n.sandbox_running())
    } else {
        format!(r#"<span class="status-badge badge-idle"><span class="status-dot"></span>{}</span>"#, i18n.sandbox_stopped())
    };

    let sandbox_action_btn = if is_running {
        format!(
            r#"<form method="post" action="/projects/{}/sandbox/stop" class="inline-form">
                <button type="submit" class="btn btn-secondary">{}</button>
            </form>"#,
            project.id, i18n.stop_sandbox()
        )
    } else {
        format!(
            r#"<form method="post" action="/projects/{}/sandbox/start" class="inline-form">
                <button type="submit" class="btn btn-primary">{}</button>
            </form>"#,
            project.id, i18n.launch_sandbox()
        )
    };

    let notice_alert = if let Some(n) = notice {
        format!(r#"<div class="alert alert-success" style="margin-bottom:1.5rem;">{}</div>"#, html_escape(n))
    } else {
        String::new()
    };

    let tab_bar = format!(
        r#"<div class="tab-bar">
    <a href="/projects/{}?tab=overview" class="tab-item {}">{}</a>
    <a href="/projects/{}?tab=merge" class="tab-item {}">{} {}</a>
    <a href="/projects/{}?tab=timeline" class="tab-item {}">{} ({})</a>
    <a href="/projects/{}?tab=git" class="tab-item {}">{}</a>
    <a href="/projects/{}?tab=members" class="tab-item {}">{} ({})</a>
</div>"#,
        project.id, if active_tab == "overview" { "active" } else { "" }, i18n.tab_overview(),
        project.id, if active_tab == "merge" { "active" } else { "" }, i18n.tab_merge(),
        if !conflicts.is_empty() { format!(r#"<span class="status-badge badge-warning" style="margin-left:4px;">{}</span>"#, conflicts.len()) } else { String::new() },
        project.id, if active_tab == "timeline" { "active" } else { "" }, i18n.tab_timeline(), snapshots.len(),
        project.id, if active_tab == "git" { "active" } else { "" }, i18n.tab_git(),
        project.id, if active_tab == "members" { "active" } else { "" }, i18n.tab_members(), members.len()
    );

    let is_owner = project.owner_id == user.id || user.is_platform_admin;

    let tab_content = match active_tab {
        "merge" => render_merge_tab(project, branches, current_branch, conflicts, i18n),
        "timeline" => render_timeline_tab(snapshots, i18n),
        "git" => render_git_tab(project, git_status, i18n),
        "members" => render_members_tab(project, members, all_users, is_owner, i18n),
        _ => render_overview_tab(project, sandbox_status, members, i18n),
    };

    let body = format!(
        r#"{}
<div class="main-content">
    <div class="breadcrumb">
        <a href="/">{}</a>
        <span> / </span>
        <span class="active">{}</span>
    </div>

    <div class="page-header">
        <div>
            <div class="title-with-badge">
                <h1 class="page-title">{}</h1>
                {}
            </div>
            <p class="page-subtitle">{} • Storage: <code>{}</code></p>
        </div>
        <div class="header-actions">
            {}
            <button type="button" class="btn btn-secondary" onclick="document.getElementById('modal-snapshot').style.display='flex'">{}</button>
        </div>
    </div>

    {}
    {}
    {}
</div>

<!-- Snapshot Modal -->
<div id="modal-snapshot" style="display:none; position:fixed; inset:0; background:rgba(15,23,42,0.5); align-items:center; justify-content:center; z-index:999;">
    <div style="background:#fff; border-radius:14px; padding:2rem; width:100%; max-width:480px; box-shadow:var(--shadow-lg);">
        <h3 style="font-size:1.2rem; font-weight:700; margin-bottom:0.5rem; color:var(--text-main);">{}</h3>
        <p style="font-size:0.85rem; color:var(--text-muted); margin-bottom:1.25rem;">{}</p>
        <form method="post" action="/projects/{}/snapshot">
            <div class="form-group">
                <label for="message">{}</label>
                <input type="text" id="message" name="message" required placeholder="Updated Hamiltonian simulation equations..." class="form-control" autofocus>
            </div>
            <div style="display:flex; justify-content:flex-end; gap:0.75rem; margin-top:1.5rem;">
                <button type="button" class="btn btn-secondary" onclick="document.getElementById('modal-snapshot').style.display='none'">{}</button>
                <button type="submit" class="btn btn-primary">{}</button>
            </div>
        </form>
    </div>
</div>"#,
        navbar,
        i18n.nav_projects(),
        html_escape(&project.name),
        html_escape(&project.name),
        status_pill,
        html_escape(project.description.as_deref().unwrap_or("")),
        html_escape(&project.storage_path),
        sandbox_action_btn,
        i18n.create_snapshot(),
        notice_alert,
        tab_bar,
        tab_content,
        i18n.modal_snapshot_title(),
        i18n.modal_snapshot_desc(),
        project.id,
        i18n.snapshot_message_label(),
        i18n.cancel(),
        i18n.snapshot_submit()
    );

    render_page(&project.name, &body, i18n)
}

/// Render Merge & Conflict Resolution Tab
fn render_merge_tab(
    project: &Project,
    branches: &[String],
    current_branch: Option<&str>,
    conflicts: &[ConflictFileView],
    i18n: &I18n,
) -> String {
    let curr = current_branch.unwrap_or("main");

    let branch_options: String = branches
        .iter()
        .filter(|b| b.as_str() != curr)
        .map(|b| format!(r#"<option value="{}">{}</option>"#, html_escape(b), html_escape(b)))
        .collect();

    let merge_trigger_form = if branches.len() > 1 {
        format!(
            r#"<form method="post" action="/projects/{}/merge" class="form-row" style="align-items:flex-end;">
                <div class="form-group" style="margin-bottom:0; flex-grow:1;">
                    <label for="branch">{} ({})</label>
                    <select id="branch" name="branch" class="form-control">
                        {}
                    </select>
                </div>
                <button type="submit" class="btn btn-primary" style="height:38px;">{}</button>
            </form>"#,
            project.id, i18n.select_merge_target(), curr, branch_options, i18n.run_merge()
        )
    } else {
        format!(
            r#"<div style="font-size:0.875rem; color:var(--text-muted);">
                {}
            </div>"#,
            i18n.single_branch_hint()
        )
    };

    let conflicts_section = if conflicts.is_empty() {
        format!(
            r#"<div class="alert alert-success" style="margin-top:1.25rem;">
                <strong>{}</strong> {}
            </div>"#,
            i18n.conflicts_clean_title(),
            i18n.conflicts_clean_desc()
        )
    } else {
        let mut html = format!(
            r#"<div class="alert alert-warning" style="margin-top:1.25rem;">
                <strong>⚠️ {} ({}):</strong> {}
            </div>"#,
            i18n.conflicts_warn_title(),
            conflicts.len(),
            i18n.conflicts_warn_desc()
        );

        for c in conflicts {
            html.push_str(&format!(
                r#"<div class="conflict-card">
    <div class="conflict-header">
        <span class="conflict-file">{} {}</span>
        <div class="conflict-actions">
            <form method="post" action="/projects/{}/resolve-conflict" class="inline-form">
                <input type="hidden" name="file" value="{}">
                <input type="hidden" name="choice" value="ours">
                <button type="submit" class="btn btn-secondary btn-sm">{}</button>
            </form>
            <form method="post" action="/projects/{}/resolve-conflict" class="inline-form">
                <input type="hidden" name="file" value="{}">
                <input type="hidden" name="choice" value="theirs">
                <button type="submit" class="btn btn-primary btn-sm">{}</button>
            </form>
        </div>
    </div>
    <div class="diff-box">
        <div style="margin-bottom:0.5rem;"><span class="diff-marker">&lt;&lt;&lt;&lt;&lt;&lt;&lt; {}</span></div>
        <pre class="diff-local">{}</pre>
        <div style="margin:0.5rem 0;"><span class="diff-marker">=======</span></div>
        <pre class="diff-remote">{}</pre>
        <div style="margin-top:0.5rem;"><span class="diff-marker">&gt;&gt;&gt;&gt;&gt;&gt;&gt; {}</span></div>
    </div>
</div>"#,
                i18n.conflict_file_prefix(),
                html_escape(&c.path),
                project.id, html_escape(&c.path), i18n.accept_ours(),
                project.id, html_escape(&c.path), i18n.accept_theirs(),
                i18n.ours_marker(),
                html_escape(&c.ours_snippet),
                html_escape(&c.theirs_snippet),
                i18n.theirs_marker()
            ));
        }
        html
    };

    format!(
        r#"<div class="section-card">
    <h2 class="section-title">{}</h2>
    <p class="text-muted" style="font-size:0.875rem; margin-bottom:1.25rem;">
        {}
    </p>

    <div style="background:#f8fafc; border:1px solid var(--border-subtle); border-radius:var(--radius-md); padding:1.25rem; margin-bottom:1.5rem;">
        <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:0.85rem;">
            <span style="font-size:0.85rem; font-weight:600; color:var(--text-main);">{} <code style="color:var(--primary); font-size:0.9rem;">{}</code></span>
            <span style="font-size:0.8rem; color:var(--text-sub);">{} {}</span>
        </div>
        {}
    </div>

    <h3 class="card-subtitle">{}</h3>
    {}
</div>"#,
        i18n.merge_title(),
        i18n.merge_desc(),
        i18n.current_branch(),
        curr,
        i18n.available_branches(),
        branches.len(),
        merge_trigger_form,
        i18n.conflicts_warn_title(),
        conflicts_section
    )
}

/// Render VCS Timeline & Snapshots Tab
fn render_timeline_tab(snapshots: &[Snapshot], i18n: &I18n) -> String {
    let timeline_items = if snapshots.is_empty() {
        r#"<div class="empty-state">
            <h4 class="empty-title">No snapshots yet</h4>
            <p class="empty-desc">Click 'Create Snapshot' above to record the initial FastCDC version milestone.</p>
        </div>"#.to_string()
    } else {
        let mut html = r#"<div class="timeline-list">"#.to_string();
        for (idx, snap) in snapshots.iter().enumerate() {
            let active_class = if idx == 0 { "active" } else { "" };
            let time_str = snap.created_at.format("%Y-%m-%d %H:%M:%S UTC").to_string();
            let id_str = snap.id.to_string();
            let short_id = &id_str[..8];
            let short_tree = if snap.tree_hash.len() >= 8 { &snap.tree_hash[..8] } else { &snap.tree_hash };

            html.push_str(&format!(
                r#"<div class="timeline-item">
    <div class="timeline-dot {}"></div>
    <div class="timeline-content">
        <div class="timeline-header">
            <span class="timeline-msg">{}</span>
            <span class="timeline-time">{}</span>
        </div>
        <div class="timeline-meta">
            <span>Snapshot: <code>{}</code></span>
            <span>Tree: <code>{}</code></span>
        </div>
    </div>
</div>"#,
                active_class,
                html_escape(&snap.message),
                time_str,
                short_id,
                short_tree
            ));
        }
        html.push_str("</div>");
        html
    };

    format!(
        r#"<div class="section-card">
    <h2 class="section-title">{}</h2>
    {}
</div>"#,
        i18n.tab_timeline(),
        timeline_items
    )
}

/// Render Git Bridge Compatibility Tab
fn render_git_tab(project: &Project, git: &GitStatusView, i18n: &I18n) -> String {
    let remote_desc = git
        .remote_url
        .as_deref()
        .unwrap_or(i18n.git_no_remote());

    format!(
        r#"<div class="section-card">
    <h2 class="section-title">{}</h2>
    <p class="text-muted" style="font-size:0.875rem; margin-bottom:1.5rem;">
        {}
    </p>

    <div class="info-list" style="background:#f8fafc; border:1px solid var(--border-subtle); border-radius:var(--radius-md); padding:1.25rem; margin-bottom:1.5rem;">
        <div class="info-row">
            <span class="info-label">{}</span>
            <span class="info-val" style="color:var(--success); font-weight:600;">{}</span>
        </div>
        <div class="info-row">
            <span class="info-label">{}</span>
            <span class="info-val">{}</span>
        </div>
        <div class="info-row">
            <span class="info-label">{}</span>
            <span class="info-val">{}</span>
        </div>
    </div>

    <div style="display:flex; gap:1rem;">
        <form method="post" action="/projects/{}/git-sync">
            <input type="hidden" name="message" value="APICH VCS sync to Git">
            <button type="submit" class="btn btn-primary">{}</button>
        </form>
    </div>
</div>"#,
        i18n.git_title(),
        i18n.git_desc(),
        i18n.git_status_label(),
        if git.initialized { i18n.git_ready() } else { i18n.git_pending() },
        i18n.git_remote_label(),
        html_escape(remote_desc),
        i18n.git_lfs_label(),
        i18n.git_lfs_val(),
        project.id,
        i18n.git_sync_btn()
    )
}

/// Render Collaborators Tab with real member add/remove management
fn render_members_tab(
    project: &Project,
    members: &[ProjectMemberWithUser],
    all_users: &[User],
    is_owner: bool,
    i18n: &I18n,
) -> String {
    let mut rows = String::new();
    for m in members {
        let initials = if m.display_name.len() >= 2 {
            &m.display_name[..2]
        } else {
            &m.username[..2.min(m.username.len())]
        };

        let role_badge = match m.role.as_str() {
            "owner" => r#"<span class="role-badge role-badge-admin">Owner</span>"#,
            "editor" => r#"<span class="role-badge role-badge-editor">Editor</span>"#,
            _ => r#"<span class="role-badge role-badge-viewer">Viewer</span>"#,
        };

        let remove_btn = if is_owner && m.role != "owner" {
            format!(
                r#"<form method="post" action="/projects/{}/members/remove" class="inline-form" onsubmit="return confirm('Remove collaborator?');">
                    <input type="hidden" name="user_id" value="{}">
                    <button type="submit" class="btn btn-danger btn-sm">{}</button>
                </form>"#,
                project.id, m.user_id, i18n.remove()
            )
        } else {
            String::new()
        };

        rows.push_str(&format!(
            r#"<div class="collaborator-item" style="justify-content:space-between;">
    <div style="display:flex; align-items:center; gap:0.75rem;">
        <div class="collab-avatar">{}</div>
        <div class="collab-info">
            <span class="collab-name">{} <small style="color:var(--text-sub);">(@{})</small></span>
            <span style="font-size:0.75rem; color:var(--text-sub);">{}</span>
        </div>
    </div>
    <div style="display:flex; align-items:center; gap:0.75rem;">
        {}
        {}
    </div>
</div>"#,
            html_escape(initials),
            html_escape(&m.display_name),
            html_escape(&m.username),
            html_escape(&m.email),
            role_badge,
            remove_btn
        ));
    }

    let add_member_section = if is_owner {
        let mut user_options = String::new();
        for u in all_users {
            if !members.iter().any(|m| m.user_id == u.id) {
                user_options.push_str(&format!(
                    r#"<option value="{}">{} (@{})</option>"#,
                    u.id, html_escape(&u.display_name), html_escape(&u.username)
                ));
            }
        }

        format!(
            r#"<div class="section-card" style="margin-top:1.5rem;">
    <h3 class="card-subtitle">{}</h3>
    <form method="post" action="/projects/{}/members/add" class="form-row" style="align-items:flex-end; display:grid; grid-template-columns: 2fr 1fr auto; gap:1rem;">
        <div class="form-group" style="margin-bottom:0;">
            <label for="user_id">{}</label>
            <select id="user_id" name="user_id_or_username" class="form-control" required>
                <option value="">-- {} --</option>
                {}
            </select>
        </div>
        <div class="form-group" style="margin-bottom:0;">
            <label for="role">{}</label>
            <select id="role" name="role" class="form-control">
                <option value="editor">Editor (Read/Write)</option>
                <option value="viewer">Viewer (Read-Only)</option>
            </select>
        </div>
        <button type="submit" class="btn btn-primary" style="height:38px;">{}</button>
    </form>
</div>"#,
            i18n.add_collaborator(),
            project.id,
            i18n.select_user(),
            i18n.select_user(),
            user_options,
            i18n.role(),
            i18n.add_collaborator()
        )
    } else {
        String::new()
    };

    format!(
        r#"<div class="section-card">
    <h2 class="section-title">{}</h2>
    <div class="collaborator-list" style="margin-top:1rem;">
        {}
    </div>
</div>
{}"#,
        i18n.tab_members(),
        rows,
        add_member_section
    )
}

/// Render Overview Tab
fn render_overview_tab(
    project: &Project,
    sandbox_status: &str,
    members: &[ProjectMemberWithUser],
    i18n: &I18n,
) -> String {
    let mut member_items = String::new();
    for m in members.iter().take(3) {
        let initials = if m.display_name.len() >= 2 { &m.display_name[..2] } else { &m.username[..2.min(m.username.len())] };
        member_items.push_str(&format!(
            r#"<div class="collaborator-item">
                <div class="collab-avatar">{}</div>
                <div class="collab-info">
                    <span class="collab-name">{}</span>
                    <span class="collab-role">{}</span>
                </div>
            </div>"#,
            html_escape(initials),
            html_escape(&m.display_name),
            m.role
        ));
    }

    format!(
        r#"<div class="detail-grid">
    <div class="detail-main">
        <div class="section-card">
            <h2 class="section-title">Project Summary</h2>
            <p style="color:var(--text-muted); font-size:0.9rem; margin-top:0.5rem; line-height:1.6;">
                {}
            </p>
        </div>

        <div class="section-card">
            <h2 class="section-title">Execution Environment & Storage</h2>
            <div class="info-list" style="margin-top:0.75rem;">
                <div class="info-row">
                    <span class="info-label">Isolation Runtime</span>
                    <span class="info-val">Rootless Podman OCI Container</span>
                </div>
                <div class="info-row">
                    <span class="info-label">Mount Path</span>
                    <span class="info-val">/workspace &lt;-&gt; {}</span>
                </div>
                <div class="info-row">
                    <span class="info-label">VCS Deduplication</span>
                    <span class="info-val">FastCDC Chunk-Level CAS</span>
                </div>
            </div>
        </div>
    </div>

    <div class="detail-sidebar">
        <div class="section-card">
            <h3 class="card-subtitle">Sandbox Status</h3>
            <div style="margin:1rem 0;">
                <span class="status-badge {}" style="font-size:0.85rem; padding:6px 12px;">
                    <span class="status-dot"></span>
                    {}
                </span>
            </div>
            <p style="font-size:0.8rem; color:var(--text-muted); line-height:1.5;">
                Every collaborator runs in their own container sandbox, preventing process and memory conflicts.
            </p>
        </div>

        <div class="section-card">
            <h3 class="card-subtitle">{}</h3>
            <div class="collaborator-list">
                {}
            </div>
        </div>
    </div>
</div>"#,
        html_escape(project.description.as_deref().unwrap_or("No description provided.")),
        html_escape(&project.storage_path),
        if sandbox_status == "running" { "badge-active" } else { "badge-idle" },
        if sandbox_status == "running" { i18n.sandbox_running() } else { i18n.sandbox_stopped() },
        i18n.tab_members(),
        member_items
    )
}

/// Render settings page with profile chip cards (NOT fake inputs)
pub fn render_settings_page(
    user: &User,
    notice: Option<&str>,
    error: Option<&str>,
    i18n: &I18n,
    current_path: &str,
) -> String {
    let navbar = render_navbar(&user.display_name, user.is_platform_admin, "settings", i18n, current_path);

    let role_chip = if user.is_platform_admin {
        format!(r#"<span class="role-badge role-badge-admin">{}</span>"#, i18n.platform_admin())
    } else {
        format!(r#"<span class="role-badge role-badge-viewer">{}</span>"#, i18n.researcher())
    };

    let alert_html = if let Some(n) = notice {
        format!(r#"<div class="alert alert-success" style="margin-bottom:1.5rem;">{}</div>"#, html_escape(n))
    } else if let Some(e) = error {
        format!(r#"<div class="alert alert-danger" style="background:#fef2f2; border:1px solid #fecaca; color:#b91c1c; padding:0.75rem 1rem; border-radius:8px; margin-bottom:1.5rem;">{}</div>"#, html_escape(e))
    } else {
        String::new()
    };

    let body = format!(
        r#"{}
<div class="main-content">
    <div class="page-header">
        <div>
            <h1 class="page-title">{}</h1>
            <p class="page-subtitle">{}</p>
        </div>
    </div>

    {}

    <div class="detail-grid">
        <!-- Card 1: Edit Profile Information -->
        <div class="section-card">
            <h2 class="section-title">{}</h2>
            <form method="post" action="/settings/profile" style="margin-top:1rem;">
                <div class="form-group">
                    <label>{}</label>
                    <input type="text" value="{}" readonly class="form-control readonly">
                </div>
                <div class="form-group">
                    <label>{}</label>
                    <input type="text" name="display_name" value="{}" required class="form-control">
                </div>
                <div class="form-group">
                    <label>{}</label>
                    <input type="email" name="email" value="{}" required class="form-control">
                </div>
                <div class="form-group">
                    <label>{}</label>
                    <input type="text" name="avatar_url" value="{}" placeholder="https://avatars.example.org/user.png" class="form-control">
                </div>
                <div class="form-group">
                    <label>{}</label>
                    <div style="margin-top:0.25rem;">{}</div>
                </div>
                <button type="submit" class="btn btn-primary" style="margin-top:0.5rem;">{}</button>
            </form>
        </div>

        <!-- Card 2: Security & Change Password -->
        <div class="section-card">
            <h2 class="section-title">{}</h2>
            <form method="post" action="/settings/password" style="margin-top:1rem;">
                <div class="form-group">
                    <label>{}</label>
                    <input type="password" name="current_password" required class="form-control">
                </div>
                <div class="form-group">
                    <label>{}</label>
                    <input type="password" name="new_password" minlength="8" required placeholder="••••••••" class="form-control">
                </div>
                <div class="form-group">
                    <label>{}</label>
                    <input type="password" name="confirm_password" minlength="8" required placeholder="••••••••" class="form-control">
                </div>
                <button type="submit" class="btn btn-secondary" style="margin-top:0.5rem;">{}</button>
            </form>

            <div style="margin-top:2rem; padding-top:1.5rem; border-top:1px solid var(--border-subtle);">
                <h3 style="font-size:1.05rem; font-weight:600; color:var(--text-main); margin-bottom:0.5rem;">{}</h3>
                <p class="text-muted" style="font-size:0.85rem;">
                    {}
                </p>
                <div style="margin-top:1rem;">
                    <button type="button" class="btn btn-secondary btn-block">{}</button>
                </div>
            </div>
        </div>
    </div>
</div>"#,
        navbar,
        i18n.settings_title(),
        i18n.settings_subtitle(),
        alert_html,
        i18n.edit_profile(),
        i18n.username(),
        html_escape(&user.username),
        i18n.display_name(),
        html_escape(&user.display_name),
        i18n.email(),
        html_escape(&user.email),
        i18n.avatar_url(),
        html_escape(user.avatar_url.as_deref().unwrap_or("")),
        i18n.system_role(),
        role_chip,
        i18n.save_profile(),
        i18n.change_password(),
        i18n.current_password(),
        i18n.new_password(),
        i18n.confirm_password(),
        i18n.update_password_btn(),
        i18n.passkeys_title(),
        i18n.passkeys_desc(),
        i18n.register_passkey_btn()
    );

    render_page(i18n.settings_title(), &body, i18n)
}


/// Helper to recursively collect all teams in an org tree for parent selection
fn collect_all_teams(nodes: &[TeamTreeNode], acc: &mut Vec<Team>) {
    for n in nodes {
        acc.push(n.team.clone());
        collect_all_teams(&n.children, acc);
    }
}

/// Render admin organizations and team hierarchy page with real admin management
#[allow(clippy::too_many_arguments)]
pub fn render_admin_orgs_page(
    user: &User,
    orgs: &[Organization],
    all_users: &[User],
    org_members_map: &HashMap<uuid::Uuid, Vec<OrgMemberWithUser>>,
    team_trees_map: &HashMap<uuid::Uuid, Vec<TeamTreeNode>>,
    team_members_map: &HashMap<uuid::Uuid, Vec<TeamMemberWithUser>>,
    notice: Option<&str>,
    error: Option<&str>,
    i18n: &I18n,
    current_path: &str,
) -> String {
    let navbar = render_navbar(&user.display_name, user.is_platform_admin, "orgs", i18n, current_path);

    let alert_html = if let Some(n) = notice {
        let msg = match n {
            "org_created" => if i18n.is_zh() { "机构创建成功" } else { "Organization created successfully" },
            "org_updated" => if i18n.is_zh() { "机构信息更新成功" } else { "Organization updated successfully" },
            "org_deleted" => if i18n.is_zh() { "机构已成功删除" } else { "Organization deleted successfully" },
            "team_created" => if i18n.is_zh() { "团队创建成功" } else { "Team created successfully" },
            "team_updated" => if i18n.is_zh() { "团队信息更新成功" } else { "Team updated successfully" },
            "team_deleted" => if i18n.is_zh() { "团队已成功删除" } else { "Team deleted successfully" },
            "member_added" => if i18n.is_zh() { "机构成员添加成功" } else { "Organization member added successfully" },
            "member_removed" => if i18n.is_zh() { "机构成员已移除" } else { "Organization member removed successfully" },
            "team_member_added" => if i18n.is_zh() { "团队成员添加成功" } else { "Team member added successfully" },
            "team_member_removed" => if i18n.is_zh() { "团队成员已移除" } else { "Team member removed successfully" },
            other => other,
        };
        format!(r#"<div class="alert alert-success" style="margin-bottom:1.5rem;">{}</div>"#, html_escape(msg))
    } else if let Some(e) = error {
        format!(r#"<div class="alert alert-danger" style="background:#fef2f2; border:1px solid #fecaca; color:#b91c1c; padding:0.75rem 1rem; border-radius:8px; margin-bottom:1.5rem;">{}</div>"#, html_escape(e))
    } else {
        String::new()
    };

    let org_cards = if orgs.is_empty() {
        r#"<div class="empty-state">
            <h3 class="empty-title">No Organizations</h3>
            <p class="empty-desc">Create your first organization to organize teams and collaborative research projects.</p>
        </div>"#.to_string()
    } else {
        let mut html = String::new();
        for org in orgs {
            let empty_members = Vec::new();
            let empty_trees = Vec::new();
            let members = org_members_map.get(&org.id).unwrap_or(&empty_members);
            let team_trees = team_trees_map.get(&org.id).unwrap_or(&empty_trees);

            let mut all_org_teams = Vec::new();
            collect_all_teams(team_trees, &mut all_org_teams);

            // Organization Members Rows
            let mut member_rows = String::new();
            for m in members {
                let initials = if m.display_name.len() >= 2 { &m.display_name[..2] } else { &m.username[..2.min(m.username.len())] };
                let role_badge = if m.role == "admin" || m.role == "owner" {
                    r#"<span class="role-badge role-badge-admin">Admin</span>"#
                } else {
                    r#"<span class="role-badge role-badge-viewer">Member</span>"#
                };

                let remove_btn = if user.is_platform_admin && m.user_id != user.id {
                    format!(
                        r#"<form method="post" action="/admin/orgs/members/remove" class="inline-form" onsubmit="return confirm('Remove member?');">
                            <input type="hidden" name="org_id" value="{}">
                            <input type="hidden" name="user_id" value="{}">
                            <button type="submit" class="btn btn-danger btn-sm">{}</button>
                        </form>"#,
                        org.id, m.user_id, i18n.remove()
                    )
                } else {
                    String::new()
                };

                member_rows.push_str(&format!(
                    r#"<div class="collaborator-item" style="justify-content:space-between; margin-bottom:0.5rem;">
    <div style="display:flex; align-items:center; gap:0.65rem;">
        <div class="collab-avatar" style="width:28px; height:28px; font-size:0.7rem;">{}</div>
        <div>
            <span style="font-weight:600; font-size:0.85rem;">{}</span>
            <span style="font-size:0.75rem; color:var(--text-sub);"> (@{})</span>
        </div>
    </div>
    <div style="display:flex; align-items:center; gap:0.5rem;">
        {}
        {}
    </div>
</div>"#,
                    html_escape(initials),
                    html_escape(&m.display_name),
                    html_escape(&m.username),
                    role_badge,
                    remove_btn
                ));
            }

            // User dropdown options
            let mut user_options = String::new();
            for u in all_users {
                user_options.push_str(&format!(
                    r#"<option value="{}">{} (@{})</option>"#,
                    u.id, html_escape(&u.display_name), html_escape(&u.username)
                ));
            }

            // Teams Tree rendering
            let team_nodes_html = render_team_tree_nodes(
                team_trees,
                &all_org_teams,
                all_users,
                team_members_map,
                org.id,
                false,
                i18n,
            );

            html.push_str(&format!(
                r#"<div class="section-card" style="margin-bottom:2rem;">
    <div style="display:flex; justify-content:space-between; align-items:center; border-bottom:1px solid var(--border-subtle); padding-bottom:1rem; margin-bottom:1.25rem;">
        <div>
            <h3 style="font-size:1.3rem; font-weight:700; color:var(--text-main);">{}</h3>
            <span style="font-size:0.8rem; color:var(--text-sub);">Slug: <code>{}</code> • {}</span>
        </div>
        <div style="display:flex; gap:0.5rem; align-items:center;">
            <button type="button" class="btn btn-secondary btn-sm" onclick="document.getElementById('modal-team-{}').style.display='flex'">
                + {}
            </button>
            <button type="button" class="btn btn-secondary btn-sm" onclick="document.getElementById('modal-org-member-{}').style.display='flex'">
                + {}
            </button>
            <button type="button" class="btn btn-secondary btn-sm" onclick="document.getElementById('modal-edit-org-{}').style.display='flex'">
                {}
            </button>
            <form method="post" action="/admin/orgs/delete" class="inline-form" onsubmit="return confirm('Are you sure you want to delete this organization? All associated teams and projects will be permanently removed.');">
                <input type="hidden" name="org_id" value="{}">
                <button type="submit" class="btn btn-danger btn-sm">{}</button>
            </form>
        </div>
    </div>

    <!-- Teams Hierarchy Section -->
    <div style="margin-bottom:1.5rem;">
        <h4 style="font-size:1rem; font-weight:600; margin-bottom:0.75rem; color:var(--text-main);">Teams & Groups</h4>
        {}
    </div>

    <!-- Organization Members Section -->
    <div>
        <h4 style="font-size:1rem; font-weight:600; margin-bottom:0.75rem; color:var(--text-main);">Organization Members ({})</h4>
        <div style="max-height:260px; overflow-y:auto;">
            {}
        </div>
    </div>
</div>

<!-- Edit Organization Modal -->
<div id="modal-edit-org-{}" style="display:none; position:fixed; inset:0; background:rgba(15,23,42,0.5); align-items:center; justify-content:center; z-index:999;">
    <div style="background:#fff; border-radius:14px; padding:2rem; width:100%; max-width:480px; box-shadow:var(--shadow-lg);">
        <h3 style="font-size:1.2rem; font-weight:700; margin-bottom:0.5rem;">{}</h3>
        <form method="post" action="/admin/orgs/edit">
            <input type="hidden" name="org_id" value="{}">
            <div class="form-group">
                <label>{}</label>
                <input type="text" name="name" value="{}" required class="form-control">
            </div>
            <div class="form-group">
                <label>{}</label>
                <input type="text" name="slug" value="{}" required class="form-control">
            </div>
            <div class="form-group">
                <label>{}</label>
                <textarea name="description" rows="2" class="form-control">{}</textarea>
            </div>
            <div style="display:flex; justify-content:flex-end; gap:0.75rem; margin-top:1.25rem;">
                <button type="button" class="btn btn-secondary" onclick="document.getElementById('modal-edit-org-{}').style.display='none'">{}</button>
                <button type="submit" class="btn btn-primary">{}</button>
            </div>
        </form>
    </div>
</div>

<!-- Add Team Modal for Org -->
<div id="modal-team-{}" style="display:none; position:fixed; inset:0; background:rgba(15,23,42,0.5); align-items:center; justify-content:center; z-index:999;">
    <div style="background:#fff; border-radius:14px; padding:2rem; width:100%; max-width:480px; box-shadow:var(--shadow-lg);">
        <h3 style="font-size:1.2rem; font-weight:700; margin-bottom:0.5rem;">{}</h3>
        <form method="post" action="/admin/teams/new">
            <input type="hidden" name="org_id" value="{}">
            <div class="form-group">
                <label>{}</label>
                <input type="text" name="name" required placeholder="Quantum Hardware" class="form-control">
            </div>
            <div class="form-group">
                <label>{}</label>
                <input type="text" name="slug" required placeholder="quantum-hw" class="form-control">
            </div>
            <div class="form-group">
                <label>{}</label>
                <textarea name="description" rows="2" class="form-control"></textarea>
            </div>
            <div style="display:flex; justify-content:flex-end; gap:0.75rem; margin-top:1.25rem;">
                <button type="button" class="btn btn-secondary" onclick="document.getElementById('modal-team-{}').style.display='none'">{}</button>
                <button type="submit" class="btn btn-primary">{}</button>
            </div>
        </form>
    </div>
</div>

<!-- Add Org Member Modal -->
<div id="modal-org-member-{}" style="display:none; position:fixed; inset:0; background:rgba(15,23,42,0.5); align-items:center; justify-content:center; z-index:999;">
    <div style="background:#fff; border-radius:14px; padding:2rem; width:100%; max-width:480px; box-shadow:var(--shadow-lg);">
        <h3 style="font-size:1.2rem; font-weight:700; margin-bottom:0.5rem;">{}</h3>
        <form method="post" action="/admin/orgs/members/add">
            <input type="hidden" name="org_id" value="{}">
            <div class="form-group">
                <label>{}</label>
                <select name="user_id_or_username" class="form-control" required>
                    {}
                </select>
            </div>
            <div class="form-group">
                <label>{}</label>
                <select name="role" class="form-control">
                    <option value="member">Member</option>
                    <option value="admin">Administrator</option>
                </select>
            </div>
            <div style="display:flex; justify-content:flex-end; gap:0.75rem; margin-top:1.25rem;">
                <button type="button" class="btn btn-secondary" onclick="document.getElementById('modal-org-member-{}').style.display='none'">{}</button>
                <button type="submit" class="btn btn-primary">{}</button>
            </div>
        </form>
    </div>
</div>"#,
                html_escape(&org.name),
                html_escape(&org.slug),
                html_escape(org.description.as_deref().unwrap_or("")),
                org.id, i18n.create_team(),
                org.id, i18n.add_member(),
                org.id, i18n.edit_organization(),
                org.id, i18n.delete_organization(),
                team_nodes_html,
                members.len(),
                member_rows,
                // Edit org modal
                org.id,
                i18n.edit_organization(),
                org.id,
                i18n.org_name(),
                html_escape(&org.name),
                i18n.org_slug(),
                html_escape(&org.slug),
                i18n.org_desc(),
                html_escape(org.description.as_deref().unwrap_or("")),
                org.id, i18n.cancel(), i18n.save_profile(),
                // Add team modal
                org.id,
                i18n.create_team(),
                org.id,
                i18n.team_name(),
                i18n.team_slug(),
                i18n.description(),
                org.id, i18n.cancel(), i18n.create_team(),
                // Add member modal
                org.id,
                i18n.add_member(),
                org.id,
                i18n.select_user(),
                user_options,
                i18n.role(),
                org.id, i18n.cancel(), i18n.add_member()
            ));
        }
        html
    };

    let body = format!(
        r#"{}
<div class="main-content">
    <div class="page-header">
        <div>
            <h1 class="page-title">{}</h1>
            <p class="page-subtitle">{}</p>
        </div>
        <div>
            <button type="button" class="btn btn-primary" onclick="document.getElementById('modal-create-org').style.display='flex'">
                {}
            </button>
        </div>
    </div>
    {}
    {}

    <!-- Create Organization Modal -->
    <div id="modal-create-org" style="display:none; position:fixed; inset:0; background:rgba(15,23,42,0.5); align-items:center; justify-content:center; z-index:999;">
        <div style="background:#fff; border-radius:14px; padding:2rem; width:100%; max-width:480px; box-shadow:var(--shadow-lg);">
            <h3 style="font-size:1.2rem; font-weight:700; margin-bottom:0.5rem;">{}</h3>
            <form method="post" action="/admin/orgs/new">
                <div class="form-group">
                    <label>{}</label>
                    <input type="text" name="name" required placeholder="Institute for Theoretical Physics" class="form-control">
                </div>
                <div class="form-group">
                    <label>{}</label>
                    <input type="text" name="slug" required placeholder="itp" class="form-control">
                </div>
                <div class="form-group">
                    <label>{}</label>
                    <textarea name="description" rows="2" placeholder="Scientific research department" class="form-control"></textarea>
                </div>
                <div style="display:flex; justify-content:flex-end; gap:0.75rem; margin-top:1.25rem;">
                    <button type="button" class="btn btn-secondary" onclick="document.getElementById('modal-create-org').style.display='none'">{}</button>
                    <button type="submit" class="btn btn-primary">{}</button>
                </div>
            </form>
        </div>
    </div>
</div>"#,
        navbar,
        i18n.org_admin_title(),
        i18n.org_admin_subtitle(),
        i18n.create_organization(),
        alert_html,
        org_cards,
        i18n.create_organization(),
        i18n.org_name(),
        i18n.org_slug(),
        i18n.org_desc(),
        i18n.cancel(),
        i18n.create_organization()
    );

    render_page(i18n.org_admin_title(), &body, i18n)
}

/// Recursive helper to render team tree nodes with member management, editing, and deletion
fn render_team_tree_nodes(
    nodes: &[TeamTreeNode],
    all_org_teams: &[Team],
    all_users: &[User],
    team_members_map: &HashMap<uuid::Uuid, Vec<TeamMemberWithUser>>,
    org_id: uuid::Uuid,
    is_sub: bool,
    i18n: &I18n,
) -> String {
    if nodes.is_empty() {
        if is_sub {
            return String::new();
        }
        return r#"<div style="font-size:0.875rem; color:var(--text-muted); padding:0.5rem 0;">No teams created in this organization yet. Click '+ Create Team' above.</div>"#.to_string();
    }

    let mut user_options = String::new();
    for u in all_users {
        user_options.push_str(&format!(
            r#"<option value="{}">{} (@{})</option>"#,
            u.id, html_escape(&u.display_name), html_escape(&u.username)
        ));
    }

    let mut html = r#"<div class="team-tree">"#.to_string();
    for node in nodes {
        let sub_class = if is_sub { "team-node-sub" } else { "" };
        let empty_members = Vec::new();
        let members = team_members_map.get(&node.team.id).unwrap_or(&empty_members);

        let mut member_chips = String::new();
        for m in members {
            member_chips.push_str(&format!(
                r#"<span class="member-chip">
                    {} <small>({})</small>
                    <form method="post" action="/admin/teams/members/remove" style="display:inline;" onsubmit="return confirm('Remove team member?');">
                        <input type="hidden" name="team_id" value="{}">
                        <input type="hidden" name="user_id" value="{}">
                        <button type="submit" class="member-chip-remove">&times;</button>
                    </form>
                </span>"#,
                html_escape(&m.display_name),
                m.role,
                node.team.id,
                m.user_id
            ));
        }

        let children_html = render_team_tree_nodes(
            &node.children,
            all_org_teams,
            all_users,
            team_members_map,
            org_id,
            true,
            i18n,
        );

        // Build parent selector options for re-parenting
        let mut parent_options = format!(
            r#"<option value="" {}>{}</option>"#,
            if node.team.parent_team_id.is_none() { "selected" } else { "" },
            i18n.top_level_team()
        );
        for t in all_org_teams {
            if t.id != node.team.id {
                let sel = if node.team.parent_team_id == Some(t.id) { "selected" } else { "" };
                parent_options.push_str(&format!(
                    r#"<option value="{}" {}>{}</option>"#,
                    t.id, sel, html_escape(&t.name)
                ));
            }
        }

        html.push_str(&format!(
            r#"<div class="team-node {}">
    <div class="team-node-header">
        <div>
            <div class="team-node-title">
                <span>{}</span>
                <span class="team-node-slug"><code>{}</code></span>
            </div>
            <p style="font-size:0.825rem; color:var(--text-muted); margin-top:0.25rem;">{}</p>
        </div>
        <div style="display:flex; gap:0.4rem; align-items:center;">
            <button type="button" class="btn btn-secondary btn-sm" onclick="document.getElementById('modal-subteam-{}').style.display='flex'">
                + Subteam
            </button>
            <button type="button" class="btn btn-secondary btn-sm" onclick="document.getElementById('modal-edit-team-{}').style.display='flex'">
                {}
            </button>
            <form method="post" action="/admin/teams/delete" class="inline-form" onsubmit="return confirm('Delete this team? Subteams and members will be removed.');">
                <input type="hidden" name="team_id" value="{}">
                <button type="submit" class="btn btn-danger btn-sm" style="padding:0.25rem 0.5rem; font-size:0.75rem;">{}</button>
            </form>
        </div>
    </div>

    <!-- Team Members & Inline Add Form -->
    <div class="team-members-bar">
        <div class="member-chips">
            <span style="font-size:0.775rem; font-weight:600; color:var(--text-sub);">Members ({}):</span>
            {}
        </div>
        <form method="post" action="/admin/teams/members/add" class="inline-form" style="display:inline-flex; gap:0.35rem; align-items:center;">
            <input type="hidden" name="team_id" value="{}">
            <select name="user_id_or_username" class="form-control" style="padding:0.25rem 0.5rem; font-size:0.75rem;" required>
                <option value="">+ {}</option>
                {}
            </select>
            <select name="role" class="form-control" style="padding:0.25rem 0.5rem; font-size:0.75rem;">
                <option value="member">Member</option>
                <option value="lead">Lead</option>
            </select>
            <button type="submit" class="btn btn-secondary btn-sm" style="padding:0.25rem 0.5rem; font-size:0.75rem;">Add</button>
        </form>
    </div>

    {}
</div>

<!-- Edit Team Modal -->
<div id="modal-edit-team-{}" style="display:none; position:fixed; inset:0; background:rgba(15,23,42,0.5); align-items:center; justify-content:center; z-index:999;">
    <div style="background:#fff; border-radius:14px; padding:2rem; width:100%; max-width:480px; box-shadow:var(--shadow-lg);">
        <h3 style="font-size:1.2rem; font-weight:700; margin-bottom:0.5rem;">{}</h3>
        <form method="post" action="/admin/teams/edit">
            <input type="hidden" name="team_id" value="{}">
            <div class="form-group">
                <label>{}</label>
                <input type="text" name="name" value="{}" required class="form-control">
            </div>
            <div class="form-group">
                <label>{}</label>
                <input type="text" name="slug" value="{}" required class="form-control">
            </div>
            <div class="form-group">
                <label>{}</label>
                <textarea name="description" rows="2" class="form-control">{}</textarea>
            </div>
            <div class="form-group">
                <label>{}</label>
                <select name="parent_team_id" class="form-control">
                    {}
                </select>
            </div>
            <div style="display:flex; justify-content:flex-end; gap:0.75rem; margin-top:1.25rem;">
                <button type="button" class="btn btn-secondary" onclick="document.getElementById('modal-edit-team-{}').style.display='none'">{}</button>
                <button type="submit" class="btn btn-primary">{}</button>
            </div>
        </form>
    </div>
</div>

<!-- Add Subteam Modal -->
<div id="modal-subteam-{}" style="display:none; position:fixed; inset:0; background:rgba(15,23,42,0.5); align-items:center; justify-content:center; z-index:999;">
    <div style="background:#fff; border-radius:14px; padding:2rem; width:100%; max-width:480px; box-shadow:var(--shadow-lg);">
        <h3 style="font-size:1.2rem; font-weight:700; margin-bottom:0.5rem;">Add Subteam to '{}'</h3>
        <form method="post" action="/admin/teams/new">
            <input type="hidden" name="org_id" value="{}">
            <input type="hidden" name="parent_team_id" value="{}">
            <div class="form-group">
                <label>{}</label>
                <input type="text" name="name" required placeholder="Cryogenics Sub-unit" class="form-control">
            </div>
            <div class="form-group">
                <label>{}</label>
                <input type="text" name="slug" required placeholder="cryo-sub" class="form-control">
            </div>
            <div class="form-group">
                <label>{}</label>
                <textarea name="description" rows="2" class="form-control"></textarea>
            </div>
            <div style="display:flex; justify-content:flex-end; gap:0.75rem; margin-top:1.25rem;">
                <button type="button" class="btn btn-secondary" onclick="document.getElementById('modal-subteam-{}').style.display='none'">{}</button>
                <button type="submit" class="btn btn-primary">Create Subteam</button>
            </div>
        </form>
    </div>
</div>"#,
            sub_class,
            html_escape(&node.team.name),
            html_escape(&node.team.slug),
            html_escape(node.team.description.as_deref().unwrap_or("")),
            node.team.id,
            node.team.id,
            i18n.edit_team(),
            node.team.id,
            i18n.delete_team(),
            members.len(),
            member_chips,
            node.team.id,
            i18n.add_member(),
            user_options,
            children_html,
            // Edit Team Modal
            node.team.id,
            i18n.edit_team(),
            node.team.id,
            i18n.team_name(),
            html_escape(&node.team.name),
            i18n.team_slug(),
            html_escape(&node.team.slug),
            i18n.team_desc(),
            html_escape(node.team.description.as_deref().unwrap_or("")),
            i18n.parent_team(),
            parent_options,
            node.team.id, i18n.cancel(), i18n.save_profile(),
            // Add Subteam Modal
            node.team.id,
            html_escape(&node.team.name),
            org_id,
            node.team.id,
            i18n.team_name(),
            i18n.team_slug(),
            i18n.description(),
            node.team.id, i18n.cancel()
        ));
    }
    html.push_str("</div>");
    html
}

/// Render platform administration page with complete SMTP configuration and live test mailer
pub fn render_admin_platform_page(
    user: &User,
    settings: &SystemSettings,
    notice: Option<&str>,
    error: Option<&str>,
    i18n: &I18n,
    current_path: &str,
) -> String {
    let navbar = render_navbar(&user.display_name, user.is_platform_admin, "admin", i18n, current_path);

    let alert_html = if let Some(n) = notice {
        format!(r#"<div class="alert alert-success" style="margin-bottom:1.5rem;">{}</div>"#, html_escape(n))
    } else if let Some(e) = error {
        format!(r#"<div class="alert alert-danger" style="background:#fef2f2; border:1px solid #fecaca; color:#b91c1c; padding:0.75rem 1rem; border-radius:8px; margin-bottom:1.5rem;">{}</div>"#, html_escape(e))
    } else {
        String::new()
    };

    let smtp_status_badge = if settings.smtp_enabled {
        r#"<span class="role-badge role-badge-admin">Active / Real Delivery</span>"#
    } else {
        r#"<span class="role-badge role-badge-viewer">Simulated Mode (In-Memory Queue)</span>"#
    };

    let pwd_placeholder = if settings.smtp_password.is_some() {
        "•••••••• (Password configured - leave empty to keep unchanged)"
    } else {
        "Enter SMTP password"
    };

    let body = format!(
        r#"{}
<div class="main-content">
    <div class="page-header">
        <div>
            <h1 class="page-title">{}</h1>
            <p class="page-subtitle">{}</p>
        </div>
    </div>

    {}

    <div class="detail-grid">
        <!-- Card 1: Registration & Admission Policy -->
        <div class="section-card">
            <h2 class="section-title">Registration & Admission Policy</h2>
            <p class="text-muted" style="font-size:0.85rem; margin-top:0.25rem; margin-bottom:1rem;">
                Controls how new researchers can sign up and join workspace projects.
            </p>
            <form method="post" action="/admin/platform/settings">
                <input type="hidden" name="section" value="registration">
                <div class="form-group">
                    <label>Registration Admission Mode</label>
                    <select name="registration_mode" class="form-control" style="margin-top:0.5rem;">
                        <option value="invite_only" {}>Invitation Only (Strict Access)</option>
                        <option value="open" {}>Open Self-Registration</option>
                        <option value="admin_only" {}>Platform Administrator Only</option>
                    </select>
                </div>
                <button type="submit" class="btn btn-primary" style="margin-top:0.5rem;">Save Policy</button>
            </form>
        </div>

        <!-- Card 2: Full SMTP Mail Server Configuration -->
        <div class="section-card" style="grid-column: span 2;">
            <div style="display:flex; justify-content:space-between; align-items:center; border-bottom:1px solid var(--border-subtle); padding-bottom:1rem; margin-bottom:1.25rem;">
                <div>
                    <h2 class="section-title" style="margin-bottom:0.25rem;">{}</h2>
                    <p class="text-muted" style="font-size:0.85rem;">{}</p>
                </div>
                <div>
                    {}
                </div>
            </div>

            <form method="post" action="/admin/platform/settings">
                <input type="hidden" name="section" value="smtp">

                <div class="form-group" style="margin-bottom:1.25rem; background:var(--bg-subtle, #f8fafc); padding:1rem; border-radius:8px; border:1px solid var(--border-subtle);">
                    <label style="display:flex; align-items:center; gap:0.6rem; cursor:pointer; font-weight:600;">
                        <input type="checkbox" name="smtp_enabled" value="true" {} style="width:18px; height:18px;">
                        <span>{}</span>
                    </label>
                    <p class="text-muted" style="font-size:0.8rem; margin-top:0.25rem; margin-left:1.75rem;">
                        When enabled, all invitations and system alerts are sent through this authenticated SMTP relay.
                    </p>
                </div>

                <div style="display:grid; grid-template-columns: 2fr 1fr; gap:1rem;">
                    <div class="form-group">
                        <label>{}</label>
                        <input type="text" name="smtp_host" value="{}" placeholder="smtp.institution.edu" class="form-control">
                    </div>
                    <div class="form-group">
                        <label>{}</label>
                        <input type="number" name="smtp_port" value="{}" placeholder="587" class="form-control">
                    </div>
                </div>

                <div style="display:grid; grid-template-columns: 1fr 1fr; gap:1rem;">
                    <div class="form-group">
                        <label>{}</label>
                        <input type="text" name="smtp_username" value="{}" placeholder="mailer@institution.edu" class="form-control">
                    </div>
                    <div class="form-group">
                        <label>{}</label>
                        <input type="password" name="smtp_password" placeholder="{}" class="form-control">
                    </div>
                </div>

                <div style="display:grid; grid-template-columns: 1fr 1fr; gap:1rem;">
                    <div class="form-group">
                        <label>{}</label>
                        <input type="email" name="smtp_from_email" value="{}" placeholder="notifications@apich.org" class="form-control">
                    </div>
                    <div class="form-group">
                        <label>{}</label>
                        <input type="text" name="smtp_from_name" value="{}" placeholder="APICH Platform" class="form-control">
                    </div>
                </div>

                <div class="form-group" style="margin-bottom:1.25rem;">
                    <label style="display:flex; align-items:center; gap:0.6rem; cursor:pointer;">
                        <input type="checkbox" name="smtp_use_tls" value="true" {} style="width:16px; height:16px;">
                        <span>{}</span>
                    </label>
                </div>

                <button type="submit" class="btn btn-primary">{}</button>
            </form>

            <!-- Test Outbound SMTP Delivery -->
            <div style="margin-top:2rem; padding-top:1.5rem; border-top:1px solid var(--border-subtle);">
                <h3 style="font-size:1.05rem; font-weight:600; margin-bottom:0.5rem; color:var(--text-main);">{}</h3>
                <p class="text-muted" style="font-size:0.85rem; margin-bottom:1rem;">
                    Dispatch a real verification message to confirm connectivity, authentication, and TLS handshake.
                </p>
                <form method="post" action="/admin/platform/smtp-test" style="display:flex; gap:0.75rem; align-items:flex-end;">
                    <div class="form-group" style="flex:1; margin-bottom:0;">
                        <label>{}</label>
                        <input type="email" name="test_email" required placeholder="admin@lab.org" class="form-control">
                    </div>
                    <button type="submit" class="btn btn-secondary">{}</button>
                </form>
            </div>
        </div>
    </div>
</div>"#,
        navbar,
        i18n.nav_admin(),
        i18n.smtp_config_subtitle(),
        alert_html,
        // Registration mode
        if settings.registration_mode == "invite_only" { "selected" } else { "" },
        if settings.registration_mode == "open" { "selected" } else { "" },
        if settings.registration_mode == "admin_only" { "selected" } else { "" },
        // SMTP Card
        i18n.smtp_config_title(),
        i18n.smtp_config_subtitle(),
        smtp_status_badge,
        if settings.smtp_enabled { "checked" } else { "" },
        i18n.smtp_enabled(),
        i18n.smtp_host(),
        html_escape(settings.smtp_host.as_deref().unwrap_or("")),
        i18n.smtp_port(),
        settings.smtp_port.map(|p| p.to_string()).unwrap_or_else(|| "587".to_string()),
        i18n.smtp_username(),
        html_escape(settings.smtp_username.as_deref().unwrap_or("")),
        i18n.smtp_password(),
        pwd_placeholder,
        i18n.smtp_from_email(),
        html_escape(settings.smtp_from_email.as_deref().unwrap_or("")),
        i18n.smtp_from_name(),
        html_escape(settings.smtp_from_name.as_deref().unwrap_or("")),
        if settings.smtp_use_tls { "checked" } else { "" },
        i18n.smtp_security(),
        i18n.save_smtp_settings(),
        // Test section
        i18n.test_smtp_title(),
        i18n.test_recipient(),
        i18n.send_test_email()
    );

    render_page(i18n.nav_admin(), &body, i18n)
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

