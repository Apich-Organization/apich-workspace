use crate::app::components::ActiveNav;
use crate::app::components::AppShell;
use crate::ui::i18n::I18n;
use apich_db::Project;
use apich_db::User;
use apich_islands::TerminalIsland;
use leptos::prelude::*;

/// Project terminal page.
///
/// No manual "start" button here: the underlying environment starts automatically on the first
/// command (see `ProjectManager::exec_in_sandbox`), matching plan.md's requirement that it's
/// never something the user has to turn on themselves -- and, per direct user feedback, the page
/// itself says nothing about a "container" or "sandbox": this reads as a plain terminal for the
/// project, the same way it would on a real command line, not as a cloud-dev-environment control
/// panel.
#[component]
pub fn TerminalPage(
    user: User,
    is_org_or_team_admin: bool,
    project: Project,
    sandbox_is_running: bool,
    notice: Option<String>,
    error: Option<String>,
    i18n: I18n,
    current_path: String,
) -> impl IntoView {
    let Project {
        id: project_id,
        slug: project_slug,
        name: project_name,
        ..
    } = project;

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

    let status_pill = if sandbox_is_running {
        view! { <span class="status-badge badge-active"><span class="status-dot"></span>{i18n.sandbox_running()}</span> }.into_any()
    } else {
        view! { <span class="status-badge badge-idle"><span class="status-dot"></span>{i18n.terminal_idle_status()}</span> }.into_any()
    };

    // Doubles as the real answer to "how do I init/build/run a whole multi-file Rust project (not
    // a single script) here" -- there was previously no UI path to this terminal at all, so
    // nobody could find out that a real `cargo` toolchain (and git, python3, R, LaTeX/Typst -- see
    // `docker/Containerfile.sandbox`) has always been available. Says nothing about a "sandbox" or
    // "container", per direct user feedback -- this reads like a plain project shell.
    let initial_screen = format!(
        "{} / {}\n\n{}\n",
        user.username,
        project_slug,
        i18n.terminal_hint()
    );

    view! {
        <AppShell
            user=user
            is_org_or_team_admin=is_org_or_team_admin
            active_nav=ActiveNav::Projects
            current_path=current_path
            page_title=i18n.tab_terminal().to_string()
            i18n=i18n
        >
            <div class="breadcrumb-nav" style="display:flex; align-items:center; gap:0.5rem; font-size:0.875rem; color:var(--text-muted); margin-bottom:0.75rem; flex-wrap:wrap;">
                <a href="/" style="color:var(--text-muted); text-decoration:none; display:inline-flex; align-items:center; gap:0.3rem;">
                    <span>"🏠"</span>
                    <span>{if i18n.is_zh() { "项目看板" } else { "Projects" }}</span>
                </a>
                <span style="opacity:0.5;">"/"</span>
                <a href=format!("/projects/{}", project_id) style="color:var(--primary); font-weight:500; text-decoration:none;">
                    {project_name.clone()}
                </a>
                <span style="opacity:0.5;">"/"</span>
                <span style="color:var(--text-main); font-weight:600;">{i18n.tab_terminal()}</span>
            </div>

            <div class="page-header" style="display:flex; justify-content:space-between; align-items:center; flex-wrap:wrap; gap:1rem;">
                <div>
                    <div class="title-with-badge">
                        <h1 class="page-title">{i18n.tab_terminal()}</h1>
                        {status_pill}
                    </div>
                    <p class="page-subtitle">{i18n.terminal_subtitle()}</p>
                </div>
                <div class="header-actions" style="display:flex; gap:0.5rem; flex-wrap:wrap; align-items:center;">
                    <a href=format!("/projects/{}", project_id) class="btn btn-secondary btn-sm" title={if i18n.is_zh() { "返回项目根目录/概览" } else { "Back to Project Root / Overview" }}>
                        <span>{if i18n.is_zh() { "← 项目根目录" } else { "← Project Root" }}</span>
                    </a>
                    <a href=format!("/projects/{}?tab=files", project_id) class="btn btn-secondary btn-sm" title={if i18n.is_zh() { "项目文件列表" } else { "Project Files" }}>
                        <span>"📁 " {i18n.tab_files()}</span>
                    </a>
                    <a href=format!("/projects/{}?tab=vcs", project_id) class="btn btn-secondary btn-sm" title={if i18n.is_zh() { "版本历史" } else { "VCS History" }}>
                        <span>"🌿 " {i18n.tab_vcs()}</span>
                    </a>
                    <a href=format!("/projects/{}/knowledge", project_id) class="btn btn-secondary btn-sm" title={if i18n.is_zh() { "任务与知识库" } else { "Knowledge & Tasks" }}>
                        <span>"📋 " {if i18n.is_zh() { "知识库" } else { "Knowledge" }}</span>
                    </a>
                    <a href="/" class="btn btn-outline btn-sm" title={if i18n.is_zh() { "返回主控台" } else { "Back to Dashboard" }}>
                        <span>"🏠 " {if i18n.is_zh() { "主控台" } else { "Dashboard" }}</span>
                    </a>
                </div>
            </div>
            {alert}

            <div class="section-card" style="background:#0f172a; padding:1.25rem;">
                <TerminalIsland project_id=project_id.to_string() initial_screen=initial_screen is_zh=i18n.is_zh() />
            </div>
        </AppShell>
    }
}
