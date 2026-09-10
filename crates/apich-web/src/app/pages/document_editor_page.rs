use crate::app::components::{ActiveNav, AiDrawer, AppShell, FileShareModal};
use crate::services::knowledge_sync::NoteHeading;
use crate::services::FileShareInfo;
use crate::ui::i18n::I18n;
use apich_db::{Project, User};
use apich_islands::{DocumentEditorIsland, HeadingItem};
use leptos::prelude::*;

#[allow(clippy::too_many_arguments)]
#[component]
pub fn DocumentEditorPage(
    user: User,
    is_org_or_team_admin: bool,
    project: Project,
    file_path: String,
    content: String,
    headings: Vec<NoteHeading>,
    is_slide: bool,
    is_script: bool,
    typst_pages: Vec<String>,
    compile_error: Option<String>,
    file_share: Option<FileShareInfo>,
    all_users: Vec<User>,
    notice: Option<String>,
    error: Option<String>,
    i18n: I18n,
    current_path: String,
) -> impl IntoView {
    let project_id = project.id;

    let alert = if let Some(n) = notice {
        Some(view! { <div class="alert alert-success" style="margin-bottom:1rem;">{n}</div> }.into_any())
    } else {
        error.map(|e| view! { <div class="alert alert-danger" style="margin-bottom:1rem;">{e}</div> }.into_any())
    };

    let kind_label = if is_script { "PYTHON / SCRIPT" } else if is_slide { "CARGO-SLIDE" } else { "TYPST / LATEX" };

    let share_label = match file_share.as_ref() {
        Some(s) if s.mode == "public" => format!("🌐 Public ({})", s.role),
        Some(s) if s.mode == "specific" => format!("👥 Specific ({})", s.role),
        _ => "🔒 Private".to_string(),
    };
    let share_mode = file_share.as_ref().map(|s| s.mode.clone()).unwrap_or_else(|| "private".to_string());
    let share_role = file_share.as_ref().map(|s| s.role.clone()).unwrap_or_else(|| "read".to_string());
    let share_users = file_share.as_ref().map(|s| s.allowed_users.join(",")).unwrap_or_default();
    let share_detail = serde_json::json!({ "path": file_path, "mode": share_mode, "role": share_role, "users": share_users }).to_string();
    let share_onclick = format!("window.dispatchEvent(new CustomEvent('apich-open-share-modal', {{detail: {}}}))", share_detail);

    let slide_present_btn = is_slide.then(|| view! {
        <button type="button" class="btn btn-secondary btn-sm" onclick="document.getElementById('editor-present-trigger')?.click()">"🖥️ " {i18n.present_mode()}</button>
    });

    let is_typst_preview = is_slide || file_path.ends_with(".typ");
    let is_latex_preview = !is_script && !is_typst_preview && (file_path.ends_with(".tex") || file_path.ends_with(".latex"));

    // LaTeX has its own "Download PDF" link inside `DocumentEditorIsland`'s preview toolbar (it
    // needs to stay in sync with the reactive engine selector there); only Typst gets one here.
    let download_pdf_href = is_typst_preview
        .then(|| format!("/projects/{}/editor/typst-pdf?file={}", project_id, urlencoding::encode(&file_path)));
    let download_pdf_name = std::path::Path::new(&file_path).file_stem().and_then(|s| s.to_str()).unwrap_or("document").to_string();
    let download_pdf_btn = download_pdf_href.map(|href| view! {
        <a href=href download=format!("{}.pdf", download_pdf_name) class="btn btn-secondary btn-sm">"⬇️ Download PDF"</a>
    });
    let download_slide_bin_btn = is_slide.then(|| view! {
        <a
            href=format!("/projects/{}/editor/slide-binary?file={}", project_id, urlencoding::encode(&file_path))
            target="_blank"
            class="btn btn-secondary btn-sm"
            title="Builds a standalone presentation binary for this server's platform. Can take a few minutes on first build."
        >"⬇️ Build & Download Binary"</a>
    });
    let rendered_markdown_html = (!is_script && !is_typst_preview && !is_latex_preview).then(|| {
        crate::services::document_renderer::DocumentRenderer::render_markdown_interactive(&content, &file_path, project_id).html
    });
    let editor_headings: Vec<HeadingItem> = headings.iter().map(|h| HeadingItem { level: h.level as u8, text: h.text.clone() }).collect();

    view! {
        <AppShell
            user=user.clone()
            is_org_or_team_admin=is_org_or_team_admin
            active_nav=ActiveNav::Projects
            current_path=current_path
            page_title=format!("{} - {}", project.name, file_path)
            i18n=i18n
        >
            <div class="editor-top-bar">
                <div style="display:flex; align-items:center; gap:0.75rem;">
                    <a href=format!("/projects/{}?tab=files", project_id) class="btn btn-ghost btn-sm">"← Files"</a>
                    <span style="font-weight:600; font-size:0.9rem;">"📄 " {file_path.clone()}</span>
                    <span class="file-type-pill pill-doc" style="font-size:0.7rem;">{kind_label}</span>
                </div>
                <div style="display:flex; align-items:center; gap:0.5rem;">
                    <button type="button" class="btn btn-secondary btn-sm" onclick=share_onclick>{format!("🔗 {}", share_label)}</button>
                    <label for="ai-drawer-toggle-cb" class="btn btn-secondary btn-sm">"🤖 AI Copilot"</label>
                    {download_pdf_btn}
                    {download_slide_bin_btn}
                    {slide_present_btn}
                    <button type="submit" form="editor-form" class="btn btn-primary btn-sm">"💾 " {i18n.save()}</button>
                </div>
            </div>

            {alert}

            <DocumentEditorIsland
                project_id=project_id.to_string()
                file_path=file_path.clone()
                content=content
                headings=editor_headings
                is_slide=is_slide
                is_script=is_script
                is_typst_preview=is_typst_preview
                typst_pages=typst_pages
                compile_error=compile_error
                rendered_markdown_html=rendered_markdown_html
                is_latex_preview=is_latex_preview
            />

            <FileShareModal
                project_id=project_id
                redirect_to=format!("/projects/{}/editor?file={}", project_id, urlencoding::encode(&file_path))
                all_users=all_users
                owner_id=project.owner_id
                i18n=i18n
            />
            <AiDrawer project_id=project_id file_path=file_path />
        </AppShell>
    }
}
