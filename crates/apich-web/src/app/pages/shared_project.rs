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
pub fn SharedProjectPage(project: Project, files: Vec<ProjectFileItem>, share_role: String, i18n: I18n) -> impl IntoView {
    let file_rows = if files.is_empty() {
        view! { <p class="text-muted">"This project has no files yet."</p> }.into_any()
    } else {
        let rows = files
            .iter()
            .map(|f| {
                let icon = match f.category.as_str() {
                    "script" => "🐍",
                    "slide" => "📊",
                    "typst" => "📄",
                    "latex" => "📝",
                    "table" => "🗄️",
                    "note" => "📔",
                    _ => "📎",
                };
                view! {
                    <div class="collaborator-item">
                        <span style="font-size:1.1rem;">{icon}</span>
                        <div class="collab-info">
                            <span class="collab-name">{f.name.clone()}</span>
                            <span style="font-size:0.75rem; color:var(--text-sub);">{f.category.to_uppercase()}</span>
                        </div>
                    </div>
                }
            })
            .collect::<Vec<_>>();
        view! { <div class="collaborator-list">{rows}</div> }.into_any()
    };

    let role_label = match share_role.as_str() {
        "read_and_review" => "Read & Review",
        "read_write_and_review" => "Read, Write & Review (sign in required to edit)",
        _ => "Read Only",
    };

    view! {
        <PageShell title=project.name.clone() i18n=i18n>
            <div class="auth-page">
                <div class="auth-card" style="max-width:640px;">
                    <div class="auth-header">
                        <div class="auth-logo">"APICH"</div>
                        <h1 class="auth-title">{project.name.clone()}</h1>
                        <p class="auth-subtitle">{project.description.clone().unwrap_or_else(|| "Shared via public link".to_string())}</p>
                    </div>
                    <div class="alert alert-info" style="margin-bottom:1.25rem;">
                        "🌐 Public link — viewing at: " <strong>{role_label}</strong>
                    </div>
                    <h3 class="card-subtitle">"Files"</h3>
                    {file_rows}
                    <div class="auth-footer">
                        <p><a href="/login">"Sign in to APICH for full access"</a></p>
                    </div>
                </div>
            </div>
        </PageShell>
    }
}
