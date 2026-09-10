use crate::app::components::{ActiveNav, AppShell};
use crate::ui::i18n::I18n;
use apich_db::{Project, User};
use apich_islands::TerminalIsland;
use leptos::prelude::*;

/// No manual "Start Sandbox" button here: the container starts automatically on the first
/// command (see `ProjectManager::exec_in_sandbox`), matching plan.md's requirement that
/// containers are never something the user has to turn on themselves.
#[component]
pub fn TerminalPage(
    user: User,
    is_org_or_team_admin: bool,
    project: Project,
    sandbox_status: String,
    notice: Option<String>,
    error: Option<String>,
    i18n: I18n,
    current_path: String,
) -> impl IntoView {
    let project_id = project.id;
    let is_running = sandbox_status == "running";

    let alert = if let Some(n) = notice {
        Some(view! { <div class="alert alert-success" style="margin-bottom:1rem;">{n}</div> }.into_any())
    } else {
        error.map(|e| view! { <div class="alert alert-danger" style="margin-bottom:1rem;">{e}</div> }.into_any())
    };

    let status_pill = if is_running {
        view! { <span class="status-badge badge-active"><span class="status-dot"></span>{i18n.sandbox_running()}</span> }.into_any()
    } else {
        view! { <span class="status-badge badge-idle"><span class="status-dot"></span>"Starts automatically on first command"</span> }.into_any()
    };

    let initial_screen = format!("User: {}\nProject: {}\nMount: /workspace <--> {}\n\n[Ready] Type a command below and press Enter or click 'Run'.\n", user.username, project.slug, project.storage_path);

    view! {
        <AppShell
            user=user.clone()
            is_org_or_team_admin=is_org_or_team_admin
            active_nav=ActiveNav::Projects
            current_path=current_path
            page_title=i18n.tab_terminal().to_string()
            i18n=i18n
        >
            <div class="page-header">
                <div>
                    <div class="title-with-badge">
                        <h1 class="page-title">{i18n.tab_terminal()}</h1>
                        {status_pill}
                    </div>
                    <p class="page-subtitle">"Runs inside this project's isolated container"</p>
                </div>
            </div>
            {alert}

            <div class="section-card" style="background:#0f172a; padding:1.25rem;">
                <TerminalIsland project_id=project_id.to_string() initial_screen=initial_screen />
            </div>
        </AppShell>
    }
}
