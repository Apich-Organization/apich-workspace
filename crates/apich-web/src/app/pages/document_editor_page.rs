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
    template_kind: Option<String>,
    own_file_templates: Vec<apich_db::Template>,
    visible_file_templates: Vec<apich_db::TemplateWithLatestVersion>,
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
    let template_panel = template_kind.map(|kind| render_file_template_panel(project_id, &file_path, &kind, &own_file_templates, &visible_file_templates, i18n));

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

            {template_panel}

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

/// "Publish as Template" / "Apply a Template" for LaTeX/Typst/slides files. Simpler than the
/// Kanban/note panels in one respect (publishing always captures exactly this one file, no
/// multi-file picker -- see `template_handlers::create_template_from_file_action`'s own comment)
/// and adds one thing they don't need: a destination subfolder for applying, since a files-kind
/// template's "apply" always creates new file(s) that must land somewhere in the project tree.
fn render_file_template_panel(
    project_id: uuid::Uuid,
    file_path: &str,
    kind: &str,
    own_templates: &[apich_db::Template],
    visible_templates: &[apich_db::TemplateWithLatestVersion],
    i18n: I18n,
) -> impl IntoView {
    let publish_existing = (!own_templates.is_empty()).then(|| {
        let rows: Vec<_> = own_templates
            .iter()
            .map(|t| {
                view! {
                    <form method="post" action=format!("/templates/{}/publish-version-file", t.id) class="inline-form" style="display:flex; gap:0.4rem; margin-top:0.3rem; align-items:center;">
                        <input type="hidden" name="project_id" value=project_id.to_string() />
                        <input type="hidden" name="file" value=file_path.to_string() />
                        <span style="font-size:0.8rem; min-width:120px;">{t.name.clone()}</span>
                        <input type="text" name="version_label" placeholder=i18n.template_version_label_field() class="form-control" style="height:30px; font-size:0.8rem; max-width:140px;" required=true />
                        <input type="text" name="changelog" placeholder=i18n.template_changelog_field() class="form-control" style="height:30px; font-size:0.8rem; max-width:220px;" />
                        <button type="submit" class="btn btn-secondary btn-sm">{i18n.template_publish()}</button>
                    </form>
                }
            })
            .collect();
        view! {
            <div style="margin-top:0.5rem;">
                <span style="font-size:0.8rem; color:var(--text-sub);">{i18n.template_publish_new_version_to()}</span>
                {rows}
            </div>
        }
    });

    let apply_options: Vec<_> = visible_templates
        .iter()
        .filter_map(|t| t.latest_version_id.map(|vid| (t, vid)))
        .map(|(t, vid)| view! { <option value=vid.to_string()>{t.name.clone()}" ("{t.latest_version_label.clone().unwrap_or_default()}")"</option> })
        .collect();
    let apply_panel = (!apply_options.is_empty()).then(|| {
        view! {
            <form method="post" action=format!("/projects/{}/apply-file-template", project_id) class="inline-form" style="display:flex; gap:0.4rem; margin-top:0.5rem; align-items:center; flex-wrap:wrap;">
                <select name="template_version_id" class="form-control" style="height:32px; font-size:0.82rem; max-width:260px;" required=true>
                    <option value="" disabled=true selected=true>{i18n.template_select_version()}</option>
                    {apply_options}
                </select>
                <input type="text" name="dest_subdir" placeholder=i18n.template_dest_folder_field() class="form-control" style="height:32px; font-size:0.82rem; max-width:180px;" />
                <button type="submit" class="btn btn-primary btn-sm">{i18n.template_apply()}</button>
            </form>
        }
    });

    view! {
        <details style="margin-top:1rem; background:var(--bg-muted); border-radius:10px; padding:0.85rem 1rem;">
            <summary style="cursor:pointer; font-weight:600; font-size:0.85rem;">{i18n.template_publish_as_template()}" / "{i18n.template_apply_template()}</summary>
            <div style="margin-top:0.6rem;">
                <form method="post" action="/templates/create-from-file" class="inline-form" style="display:flex; gap:0.4rem; flex-wrap:wrap; align-items:center;">
                    <input type="hidden" name="project_id" value=project_id.to_string() />
                    <input type="hidden" name="file" value=file_path.to_string() />
                    <input type="hidden" name="kind" value=kind.to_string() />
                    <input type="text" name="name" placeholder=i18n.template_name_field() class="form-control" style="height:32px; font-size:0.82rem; max-width:160px;" required=true />
                    <input type="text" name="slug" placeholder=i18n.template_slug_field() class="form-control" style="height:32px; font-size:0.82rem; max-width:160px;" required=true />
                    <input type="text" name="description" placeholder=i18n.template_description_field() class="form-control" style="height:32px; font-size:0.82rem; max-width:200px;" />
                    <select name="visibility" class="form-control" style="height:32px; font-size:0.82rem;">
                        <option value="private">{i18n.template_visibility_label("private")}</option>
                        <option value="shared">{i18n.template_visibility_label("shared")}</option>
                        <option value="public">{i18n.template_visibility_label("public")}</option>
                    </select>
                    <input type="text" name="version_label" placeholder=i18n.template_version_label_field() class="form-control" style="height:32px; font-size:0.82rem; max-width:140px;" required=true value="1.0.0" />
                    <button type="submit" class="btn btn-primary btn-sm">{i18n.template_create_new()}</button>
                </form>
                {publish_existing}
                {apply_panel.is_none().then(|| view! { <p class="text-muted" style="font-size:0.78rem; margin-top:0.5rem;">{i18n.template_no_templates()}</p> })}
                {apply_panel}
                <div style="margin-top:0.4rem;"><a href=format!("/templates?kind={kind}") style="font-size:0.78rem;">{i18n.template_gallery_title()}" →"</a></div>
            </div>
        </details>
    }
}
