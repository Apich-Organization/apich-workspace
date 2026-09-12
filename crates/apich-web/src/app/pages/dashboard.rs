use crate::app::components::ActiveNav;
use crate::app::components::AppShell;
use crate::ui::i18n::I18n;
use apich_db::Project;
use apich_db::User;
use leptos::prelude::*;

#[component]
pub fn DashboardPage(
    user: User,
    is_org_or_team_admin: bool,
    projects: Vec<Project>,
    i18n: I18n,
    current_path: String,
    notice: Option<String>,
    error: Option<String>,
) -> impl IntoView {
    let alert = if let Some(n) = notice {
        Some(view! { <div class="alert alert-success">{n}</div> }.into_any())
    } else {
        error.map(|e| view! { <div class="alert alert-danger">{e}</div> }.into_any())
    };

    let projects_body = if projects.is_empty() {
        view! {
            <div class="empty-state">
                <div style="font-size:2.5rem; margin-bottom:0.75rem;">"🔬"</div>
                <h3 class="empty-title">{i18n.no_projects()}</h3>
                <p class="empty-desc">{i18n.no_projects_desc()}</p>
                <div style="display:flex; justify-content:center; gap:1rem; margin-top:1.25rem;">
                    <form method="post" action="/projects/demo/create" class="inline-form">
                        <button type="submit" class="btn btn-primary btn-lg">"✨ " {i18n.load_demo_project()}</button>
                    </form>
                    <p style="font-size:0.8rem; color:var(--text-sub); align-self:center;">"or use \"+ New Project\" above"</p>
                </div>
            </div>
        }.into_any()
    } else {
        let cards = projects
            .into_iter()
            .map(|proj| {
                let desc = proj.description.clone().unwrap_or_else(|| {
                    "Unified research repository with version control".to_string()
                });
                let created_str = proj.created_at.format("%Y-%m-%d %H:%M").to_string();
                let href = format!("/projects/{}", proj.id);
                let href_vcs = format!("/projects/{}?tab=vcs", proj.id);
                view! {
                    <div class="project-card">
                        <div class="project-card-header">
                            <div>
                                <h3 class="project-name"><a href=href.clone()>{proj.name}</a></h3>
                            </div>
                        </div>
                        <p class="project-desc">{desc}</p>
                        <div class="project-footer">
                            <div class="project-meta">
                                <span>"📅 " {created_str}</span>
                            </div>
                            <div class="project-actions">
                                <a href=href class="btn btn-primary btn-sm">"📂 Open"</a>
                                <a href=href_vcs class="btn btn-secondary btn-sm">"🌿 VCS"</a>
                            </div>
                        </div>
                    </div>
                }
            })
            .collect::<Vec<_>>();
        view! { <div class="projects-grid">{cards}</div> }.into_any()
    };

    view! {
        <AppShell
            user=user
            is_org_or_team_admin=is_org_or_team_admin
            active_nav=ActiveNav::Projects
            current_path=current_path
            page_title=i18n.nav_projects().to_string()
            i18n=i18n
        >
            <div class="page-header">
                <div>
                    <h1 class="page-title">{i18n.dashboard_title()}</h1>
                    <p class="page-subtitle">{i18n.dashboard_subtitle()}</p>
                </div>
                <div class="header-actions">
                    <form method="post" action="/projects/demo/create" class="inline-form">
                        <button type="submit" class="btn btn-secondary">"✨ " {i18n.load_demo_project()}</button>
                    </form>
                    <apich_islands::ModalIsland trigger_label=format!("+ {}", i18n.new_project()) trigger_class="btn btn-primary".to_string() title=i18n.new_project().to_string()>
                        <form method="post" action="/projects/new">
                            <apich_islands::NameSlugFieldsIsland
                                name_field="name".to_string()
                                slug_field="slug".to_string()
                                name_label=i18n.project_name().to_string()
                                slug_label=i18n.project_slug().to_string()
                                name_placeholder="Quantum Measurements".to_string()
                                slug_placeholder="quantum-measurements".to_string()
                            />
                            <div class="form-group">
                                <label>{i18n.description_label()}</label>
                                <textarea name="description" rows="3" class="form-control" placeholder="Project objectives and dataset documentation"></textarea>
                            </div>
                            <div style="display:flex; justify-content:flex-end; gap:0.5rem; margin-top:1.25rem;">
                                <button type="submit" class="btn btn-primary">{i18n.save()}</button>
                            </div>
                        </form>
                    </apich_islands::ModalIsland>
                </div>
            </div>
            {alert}
            {projects_body}
        </AppShell>
    }
}
