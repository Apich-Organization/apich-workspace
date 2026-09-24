use crate::app::components::PageShell;
use crate::services::project_manager::ProjectFileItem;
use crate::ui::i18n::I18n;
use apich_db::Project;
use leptos::prelude::*;

/// Minimal read-only view served to anonymous visitors of a project's public share link.
/// No sidebar, no mutation affordances -- an anonymous visitor has no account to attribute
/// edits to, so this only ever shows content, never write actions, regardless of the
/// configured share role (which is shown for information only).
#[component]
pub fn SharedProjectPage(
    project: Project,
    files: Vec<ProjectFileItem>,
    share_role: String,
    i18n: I18n,
) -> impl IntoView {
    let Project {
        name: project_name,
        description: project_desc,
        ..
    } = project;

    let file_rows = if files.is_empty() {
        view! { <p class="text-muted">{if i18n.is_zh() { "此项目暂无文件。" } else { "This project has no files yet." }}</p> }.into_any()
    } else {
        let rows = files
            .into_iter()
            .map(|f| {
                let icon = match f.category.as_str() {
                    | "script" => "🐍",
                    | "slide" => "📊",
                    | "typst" => "📄",
                    | "latex" => "📝",
                    | "table" => "🗄️",
                    | "note" => "📔",
                    | _ => "📎",
                };
                let cat = f.category.to_uppercase();
                view! {
                    <div class="collaborator-item">
                        <span style="font-size:1.1rem;">{icon}</span>
                        <div class="collab-info">
                            <span class="collab-name">{f.name}</span>
                            <span style="font-size:0.75rem; color:var(--text-sub);">{cat}</span>
                        </div>
                    </div>
                }
            })
            .collect::<Vec<_>>();
        view! { <div class="collaborator-list">{rows}</div> }.into_any()
    };

    let role_label = match share_role.as_str() {
        | "read_and_review" => i18n.role_read_and_review().to_string(),
        | "read_write_and_review" => if i18n.is_zh() { "可读、写与审阅（需登录编辑）".to_string() } else { "Read, Write & Review (sign in required to edit)".to_string() },
        | "read" | "read_only" => i18n.role_read_only().to_string(),
        | _ => share_role,
    };

    let page_title = project_name.clone();

    view! {
        <PageShell title=page_title i18n=i18n>
            <div class="auth-page">
                <div class="auth-card" style="max-width:640px;">
                    <div class="auth-header">
                        <div class="auth-logo">"APICH"</div>
                        <h1 class="auth-title">{project_name}</h1>
                        <p class="auth-subtitle">{project_desc.unwrap_or_else(|| if i18n.is_zh() { "通过公开链接共享".to_string() } else { "Shared via public link".to_string() })}</p>
                    </div>
                    <div class="alert alert-info" style="margin-bottom:1.25rem;">
                        {if i18n.is_zh() { "🌐 公开链接 — 访问权限：" } else { "🌐 Public link — viewing at: " }}<strong>{role_label}</strong>
                    </div>
                    <h3 class="card-subtitle">{if i18n.is_zh() { "文件列表" } else { "Files" }}</h3>
                    {file_rows}
                    <div class="auth-footer">
                        <p><a href="/login">{if i18n.is_zh() { "登录 APICH 获取完整访问权限" } else { "Sign in to APICH for full access" }}</a></p>
                    </div>
                </div>
            </div>
        </PageShell>
    }
}
