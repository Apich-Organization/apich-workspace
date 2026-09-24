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
    single_files: Vec<Project>,
    i18n: I18n,
    current_path: String,
    notice: Option<String>,
    error: Option<String>,
) -> impl IntoView {
    let alert = notice.map_or_else(
        || error.map(|e| view! { <div class="alert alert-danger">{e}</div> }.into_any()),
        |n| Some(view! { <div class="alert alert-success">{n}</div> }.into_any()),
    );

    let projects_count = projects.len();
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
                    <p style="font-size:0.8rem; color:var(--text-sub); align-self:center;">{if i18n.is_zh() { "或使用上方的“+ 新建项目”" } else { "or use \"+ New Project\" above" }}</p>
                </div>
            </div>
        }.into_any()
    } else {
        let cards = projects
            .into_iter()
            .map(|proj| {
                let desc = proj.description.clone().unwrap_or_else(|| {
                    if i18n.is_zh() {
                        "包含版本控制的统一研究项目库".to_string()
                    } else {
                        "Unified research repository with version control".to_string()
                    }
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
                                <a href=href class="btn btn-primary btn-sm">"📂 " {i18n.open_file()}</a>
                                <a href=href_vcs class="btn btn-secondary btn-sm">"🌿 " {i18n.history()}</a>
                            </div>
                        </div>
                    </div>
                }
            })
            .collect::<Vec<_>>();
        view! { <div class="projects-grid">{cards}</div> }.into_any()
    };

    let single_files_count = single_files.len();
    let single_files_body = if single_files.is_empty() {
        view! {
            <div class="empty-state" style="padding:2.25rem 1.5rem; border:1px dashed var(--border-subtle); border-radius:10px; text-align:center; background:var(--bg-surface);">
                <div style="font-size:2rem; margin-bottom:0.5rem;">"📜"</div>
                <h4 style="font-size:1rem; font-weight:600; color:var(--text-main); margin:0 0 0.35rem 0;">{if i18n.is_zh() { "暂无独立单文件" } else { "No Standalone Files Yet" }}</h4>
                <p class="text-muted" style="font-size:0.85rem; max-width:480px; margin:0 auto; line-height:1.5;">
                    {if i18n.is_zh() {
                        "无需管理完整项目即可快速创建单文件脚本或文档。点击“快速开始”并选择“独立文件”即可开始编写 Python、R、Rust、Typst 或 LaTeX。"
                    } else {
                        "Create standalone scripts or documents without managing a full project. Use Quick Start and select 'Single file' to start writing Python, R, Rust, Typst, or LaTeX."
                    }}
                </p>
            </div>
        }.into_any()
    } else {
        let cards = single_files
            .into_iter()
            .map(|proj| {
                let file_name = proj.settings.get("single_file_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&proj.name)
                    .to_string();
                let ext = std::path::Path::new(&file_name)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                let (icon, type_label, badge_cls) = match ext.as_str() {
                    "py" => ("🐍", if i18n.is_zh() { "Python 脚本" } else { "Python Script" }, "badge-active"),
                    "r" => ("📊", if i18n.is_zh() { "R 脚本" } else { "R Script" }, "badge-active"),
                    "rs" => ("🦀", if i18n.is_zh() { "Rust 脚本" } else { "Rust Script" }, "badge-active"),
                    "typ" => {
                        if file_name.contains("slide") {
                            ("📊", if i18n.is_zh() { "演示文稿" } else { "Slide Deck" }, "badge-idle")
                        } else {
                            ("📄", if i18n.is_zh() { "Typst 文档" } else { "Typst Document" }, "badge-idle")
                        }
                    },
                    "tex" | "latex" => ("📝", if i18n.is_zh() { "LaTeX 文档" } else { "LaTeX Document" }, "badge-idle"),
                    "table" | "db" | "sqlite" => ("🗄️", if i18n.is_zh() { "数据表" } else { "Table" }, "badge-viewer"),
                    "anote" | "note" => ("📔", if i18n.is_zh() { "笔记" } else { "Note" }, "badge-viewer"),
                    "md" => ("📑", "Markdown", "badge-idle"),
                    _ => ("📄", if i18n.is_zh() { "文件" } else { "File" }, "badge-idle"),
                };
                let href = match ext.as_str() {
                    "table" | "db" | "sqlite" => format!("/projects/{}/table?file={}", proj.id, urlencoding::encode(&file_name)),
                    "anote" | "note" => format!("/projects/{}/note?file={}", proj.id, urlencoding::encode(&file_name)),
                    _ => format!("/projects/{}/editor?file={}", proj.id, urlencoding::encode(&file_name)),
                };
                let href_vcs = format!("/projects/{}?tab=vcs", proj.id);
                let updated_str = proj.updated_at.format("%Y-%m-%d %H:%M").to_string();

                view! {
                    <div class="project-card" style="border-left:3px solid var(--primary); display:flex; flex-direction:column; justify-content:space-between;">
                        <div>
                            <div class="project-card-header" style="align-items:flex-start; margin-bottom:0.75rem;">
                                <div style="display:flex; align-items:center; gap:0.5rem; flex:1; min-width:0;">
                                    <span style="font-size:1.35rem; flex-shrink:0;">{icon}</span>
                                    <h3 class="project-name" style="margin:0; font-size:1rem; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;">
                                        <a href=href.clone() title=file_name.clone()>{file_name.clone()}</a>
                                    </h3>
                                    <apich_islands::FileActionDropdownIsland
                                        project_id=proj.id.to_string()
                                        file_path=file_name.clone()
                                        file_name=file_name.clone()
                                        open_url=href.clone()
                                        is_dir=false
                                        is_single_file=true
                                        is_zh=i18n.is_zh()
                                    />
                                </div>
                                <span class=format!("badge {badge_cls}") style="font-size:0.72rem; flex-shrink:0;">
                                    {type_label}
                                </span>
                            </div>
                            <p class="text-muted" style="font-size:0.8rem; margin:0 0 0.85rem 0;">
                                {if i18n.is_zh() {
                                    "拥有独立版本历史与共享权限的单文件工作区。"
                                } else {
                                    "Standalone workspace with dedicated VCS timeline and sharing."
                                }}
                            </p>
                        </div>
                        <div class="project-footer" style="padding-top:0.75rem; border-top:1px solid var(--border-subtle); margin-top:0.5rem;">
                            <div class="project-meta" style="font-size:0.78rem;">
                                <span>"📅 " {updated_str}</span>
                            </div>
                            <div class="project-actions" style="display:flex; align-items:center; gap:0.4rem;">
                                <a href=href class="btn btn-primary btn-sm">"📂 " {i18n.open_file()}</a>
                                <a href=href_vcs class="btn btn-secondary btn-sm">"🌿 " {i18n.history()}</a>
                                <form
                                    method="post"
                                    action=format!("/projects/{}/delete", proj.id)
                                    class="inline-form"
                                    onsubmit=if i18n.is_zh() { "return confirm('确定要删除此文件吗？该文件及其版本历史将被永久删除。');" } else { "return confirm('Are you sure you want to delete this file? The file and its version history will be permanently deleted.');" }
                                >
                                    <button
                                        type="submit"
                                        class="btn btn-danger btn-sm"
                                        style="padding:0.25rem 0.55rem; font-size:0.78rem;"
                                        title=if i18n.is_zh() { "删除此文件" } else { "Delete this file" }
                                    >
                                        "🗑️"
                                    </button>
                                </form>
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
                    // One-click starters. The island asks where the new document should go
                    // (a new project, an existing one, or single file) rather than creating a project the
                    // instant a button is pressed.
                    <apich_islands::QuickStartMenuIsland variant=apich_islands::QuickStartVariant::Dashboard is_zh=i18n.is_zh() />
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
                                <textarea name="description" rows="3" class="form-control" placeholder=if i18n.is_zh() { "项目目标与数据集文档说明" } else { "Project objectives and dataset documentation" }></textarea>
                            </div>
                            <div style="display:flex; justify-content:flex-end; gap:0.5rem; margin-top:1.25rem;">
                                <button type="submit" class="btn btn-primary">{i18n.save()}</button>
                            </div>
                        </form>
                    </apich_islands::ModalIsland>
                </div>
            </div>
            {alert}

            // Section 1: Projects
            <div style="margin-bottom:1rem; display:flex; align-items:center; gap:0.6rem;">
                <h2 class="section-title" style="margin:0; font-size:1.15rem;">"📁 " {i18n.nav_projects()}</h2>
                <span class="badge badge-active" style="font-size:0.72rem;">{format!("{projects_count}")}</span>
            </div>
            {projects_body}

            // Section 2: Standalone Single Files
            <div style="margin-top:2.75rem; margin-bottom:1rem; border-top:1px solid var(--border-subtle); padding-top:1.75rem;">
                <div style="display:flex; align-items:center; justify-content:space-between; flex-wrap:wrap; gap:0.5rem; margin-bottom:0.25rem;">
                    <div style="display:flex; align-items:center; gap:0.6rem;">
                        <h2 class="section-title" style="margin:0; font-size:1.15rem;">{if i18n.is_zh() { "📄 独立单文件" } else { "📄 Standalone Files" }}</h2>
                        <span class="badge badge-idle" style="font-size:0.72rem;">{format!("{single_files_count}")}</span>
                    </div>
                </div>
                <p class="text-muted" style="font-size:0.85rem; margin:0 0 1.25rem 0;">
                    {if i18n.is_zh() {
                        "独立脚本与文档，无需创建多文件项目即可单独编辑与管理。每个文件享有独立的版本历史、运行沙箱与共享权限。"
                    } else {
                        "Individual scripts and documents managed without a multi-file project workspace. Each file has its own version history, execution sandbox, and sharing."
                    }}
                </p>
                {single_files_body}
            </div>
        </AppShell>
    }
}
