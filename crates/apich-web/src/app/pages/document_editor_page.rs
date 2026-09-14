use crate::app::components::ActiveNav;
use crate::app::components::AiDrawer;
use crate::app::components::AppShell;
use crate::app::components::FileShareModal;
use crate::services::knowledge_sync::NoteHeading;
use crate::services::FileShareInfo;
use crate::ui::i18n::I18n;
use apich_db::Project;
use apich_db::User;
use apich_islands::DocEditorFlags;
use apich_islands::DocumentEditorIsland;
use apich_islands::DocumentPreviewKind;
use apich_islands::HeadingItem;
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
    let Project {
        id: project_id,
        name: project_name,
        owner_id: project_owner_id,
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

    let kind_label = if is_script {
        let ext = std::path::Path::new(&file_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        match ext.as_str() {
            | "r" => "R SCRIPT",
            | "rs" => "RUST SCRIPT",
            | "py" => "PYTHON SCRIPT",
            | "sh" | "bash" => "BASH SCRIPT",
            | _ => "SCRIPT",
        }
    } else if is_slide {
        "CARGO-SLIDE"
    } else {
        "TYPST / LATEX"
    };

    let (share_label, share_mode, share_role, share_users) = match file_share {
        | Some(s) => {
            let label = match s.mode.as_str() {
                | "public" => format!("🌐 Public ({})", s.role),
                | "specific" => format!("👥 Specific ({})", s.role),
                | _ => "🔒 Private".to_string(),
            };
            let users = s.allowed_users.join(",");
            (label, s.mode, s.role, users)
        },
        | None => {
            (
                "🔒 Private".to_string(),
                "private".to_string(),
                "read".to_string(),
                String::new(),
            )
        },
    };
    let share_detail = serde_json::json!({ "path": file_path, "mode": share_mode, "role": share_role, "users": share_users }).to_string();
    let share_onclick = format!(
        "window.dispatchEvent(new CustomEvent('apich-open-share-modal', {{detail: {share_detail}}}))"
    );

    let slide_present_btn = is_slide.then(|| view! {
        <button type="button" class="btn btn-secondary btn-sm" onclick="document.getElementById('editor-present-trigger')?.click()">"🖥️ " {i18n.present_mode()}</button>
    });

    let is_typst_preview = is_slide
        || std::path::Path::new(&file_path)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("typ"));
    let is_latex_preview = !is_script
        && !is_typst_preview
        && std::path::Path::new(&file_path)
            .extension()
            .is_some_and(|ext| {
                ext.eq_ignore_ascii_case("tex") || ext.eq_ignore_ascii_case("latex")
            });
    let preview_kind = if is_slide {
        DocumentPreviewKind::Slide
    } else if is_typst_preview {
        DocumentPreviewKind::Typst
    } else if is_latex_preview {
        DocumentPreviewKind::Latex
    } else {
        DocumentPreviewKind::None
    };

    // LaTeX has its own "Download PDF" link inside `DocumentEditorIsland`'s preview toolbar (it
    // needs to stay in sync with the reactive engine selector there); only Typst gets one here.
    let download_pdf_href = is_typst_preview.then(|| {
        format!(
            "/projects/{}/editor/typst-pdf?file={}",
            project_id,
            urlencoding::encode(&file_path)
        )
    });
    let download_pdf_name = std::path::Path::new(&file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("document")
        .to_string();
    let download_pdf_btn = download_pdf_href.map(|href| view! {
        <a href=href download=format!("{}.pdf", download_pdf_name) class="btn btn-secondary btn-sm">"⬇️ Download PDF"</a>
    });
    let download_slide_bin_btn = is_slide.then(|| view! {
        <apich_islands::SlideBuildIsland project_id=project_id.to_string() file_path=file_path.clone() />
    });
    let rendered_markdown_html =
        (!is_script && !is_typst_preview && !is_latex_preview).then(|| {
            crate::services::document_renderer::DocumentRenderer::render_markdown_interactive(
                &content, &file_path, project_id,
            )
            .html
        });
    let editor_headings: Vec<HeadingItem> = headings
        .into_iter()
        .map(|h| {
            HeadingItem {
                level: h.level as u8,
                text: h.text,
            }
        })
        .collect();
    let template_panel = template_kind.map(|kind| {
        render_file_template_panel(
            project_id,
            &file_path,
            &kind,
            own_file_templates,
            visible_file_templates,
            i18n,
        )
    });

    let is_single_file = project
        .settings
        .get("is_single_file")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    view! {
        <AppShell
            user=user
            is_org_or_team_admin=is_org_or_team_admin
            active_nav=ActiveNav::Projects
            current_path=current_path
            page_title=format!("{} - {}", project_name, file_path)
            i18n=i18n
        >
            // Back-navigation gets a real, bordered button rather than the ghost style it had
            // (transparent, no border -- it read as inert text, not the way out of the editor),
            // and the toolbar is grouped: navigation | document identity :: tools | primary
            // action, so Save is visually separated from the five secondary buttons it used to
            // sit flush against.
            <div class="editor-top-bar">
                <div class="editor-top-left">
                    {if is_single_file {
                        view! {
                            <a href="/" class="btn btn-secondary btn-sm editor-back-btn" title="Back to Dashboard">
                                <span aria-hidden="true">"←"</span>
                                <span>"Dashboard"</span>
                            </a>
                        }.into_any()
                    } else {
                        view! {
                            <a href=format!("/projects/{}?tab=files", project_id) class="btn btn-secondary btn-sm editor-back-btn" title="Back to the project's file list">
                                <span aria-hidden="true">"←"</span>
                                <span>"Back to Files"</span>
                            </a>
                        }.into_any()
                    }}
                    <span class="editor-top-divider" aria-hidden="true"></span>
                    <span class="editor-file-name" title=file_path.clone()>"📄 " {file_path.clone()}</span>
                    <span class="file-type-pill pill-doc" style="font-size:0.7rem;">{kind_label}</span>
                </div>
                <div class="editor-top-right">
                    <a href=format!("/projects/{}?tab=vcs", project_id) class="btn btn-secondary btn-sm" title="View version control timeline and file history">"🌿 History"</a>
                    <button type="button" class="btn btn-secondary btn-sm" onclick=share_onclick>{format!("🔗 {share_label}")}</button>
                    <label for="ai-drawer-toggle-cb" class="btn btn-secondary btn-sm">"🤖 AI Copilot"</label>
                    {download_pdf_btn}
                    {download_slide_bin_btn}
                    {slide_present_btn}
                    <span class="editor-top-divider" aria-hidden="true"></span>
                    <button type="submit" form="editor-form" class="btn btn-primary btn-sm">"💾 " {i18n.save()}</button>
                </div>
            </div>

            {alert}

            <DocumentEditorIsland
                project_id=project_id.to_string()
                file_path=file_path.clone()
                content=content
                headings=editor_headings
                flags=DocEditorFlags::new(preview_kind, is_script)
                typst_pages=typst_pages
                compile_error=compile_error
                rendered_markdown_html=rendered_markdown_html
            />

            {template_panel}

            <FileShareModal
                project_id=project_id
                redirect_to=format!("/projects/{}/editor?file={}", project_id, urlencoding::encode(&file_path))
                all_users=all_users
                owner_id=project_owner_id
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
    own_templates: Vec<apich_db::Template>,
    visible_templates: Vec<apich_db::TemplateWithLatestVersion>,
    i18n: I18n,
) -> impl IntoView {
    let publish_existing = (!own_templates.is_empty()).then(|| {
        let rows: Vec<_> = own_templates
            .into_iter()
            .map(|t| {
                view! {
                    <form method="post" action=format!("/templates/{}/publish-version-file", t.id) class="inline-form" style="display:flex; gap:0.4rem; margin-top:0.3rem; align-items:center;">
                        <input type="hidden" name="project_id" value=project_id.to_string() />
                        <input type="hidden" name="file" value=file_path.to_string() />
                        <span style="font-size:0.8rem; min-width:120px;">{t.name}</span>
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
        .into_iter()
        .filter_map(|t| t.latest_version_id.map(|vid| (t, vid)))
        .map(|(t, vid)| view! { <option value=vid.to_string()>{t.name}" ("{t.latest_version_label.unwrap_or_default()}")"</option> })
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
