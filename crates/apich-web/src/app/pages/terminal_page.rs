use crate::app::components::ActiveNav;
use crate::app::components::AppShell;
use crate::ui::i18n::I18n;
use apich_db::Project;
use apich_db::User;
use apich_islands::TerminalIsland;
use leptos::prelude::*;

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
    sandbox_status: String,
    notice: Option<String>,
    error: Option<String>,
    i18n: I18n,
    current_path: String,
) -> impl IntoView {
    let project_id = project.id;
    let is_running = sandbox_status == "running";

    let alert = if let Some(n) = notice {
        Some(
            view! { <div class="alert alert-success" style="margin-bottom:1rem;">{n}</div> }
                .into_any(),
        )
    } else {
        error.map(|e| {
            view! { <div class="alert alert-danger" style="margin-bottom:1rem;">{e}</div> }
                .into_any()
        })
    };

    let status_pill = if is_running {
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
        project.slug,
        i18n.terminal_hint()
    );

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
                    <p class="page-subtitle">{i18n.terminal_subtitle()}</p>
                </div>
            </div>
            {alert}

            <div class="section-card" style="background:#0f172a; padding:1.25rem;">
                <TerminalIsland project_id=project_id.to_string() initial_screen=initial_screen is_zh=i18n.is_zh() />
            </div>
        </AppShell>
    }
}
