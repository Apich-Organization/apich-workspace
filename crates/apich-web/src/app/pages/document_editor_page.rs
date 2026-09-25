use crate::app::components::ActiveNav;
use crate::app::components::AiDrawer;
use crate::app::components::AppShell;
use crate::app::components::FileShareModal;
use crate::services::knowledge_sync::NoteHeading;
use crate::services::FileShareInfo;
use crate::ui::i18n::I18n;
use apich_db::Project;
use apich_db::User;
use apich_islands::AttachModalIsland;
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
    available_sibling_files: Vec<String>,
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

    let (share_label, share_mode, share_role, share_users, share_token) = match file_share {
        | Some(s) => {
            let label = match s.mode.as_str() {
                | "public" => format!("🌐 {}", i18n.share_public(&s.role)),
                | "specific" => format!("👥 {}", i18n.share_specific(&s.role)),
                | _ => format!("🔒 {}", i18n.share_private()),
            };
            let users = s.allowed_users.join(",");
            (label, s.mode, s.role, users, s.token)
        },
        | None => {
            (
                format!("🔒 {}", i18n.share_private()),
                "private".to_string(),
                "read".to_string(),
                String::new(),
                String::new(),
            )
        },
    };
    let share_detail = serde_json::json!({ "path": file_path, "mode": share_mode, "role": share_role, "users": share_users, "token": share_token }).to_string();
    let share_onclick = format!(
        "window.dispatchEvent(new CustomEvent('apich-open-share-modal', {{detail: {share_detail}}}))"
    );

    let slide_play_browser_btn = is_slide.then(|| {
        let play_href = format!(
            "/projects/{}/presentation/?file={}",
            project_id,
            urlencoding::encode(&file_path)
        );
        let play_label = if i18n.is_zh() { "浏览器播放" } else { "Play in Browser" };
        view! {
            <a
                href=play_href
                target="_blank"
                rel="noopener noreferrer"
                class="btn btn-primary btn-sm"
                title=if i18n.is_zh() { "在浏览器中以完整过渡、白板标注、激光笔和音频放映演示" } else { "Play presentation in browser with full transitions, whiteboard annotations, laser pointer, and audio" }
                style="display:inline-flex; align-items:center; gap:0.35rem; font-weight:600;"
            >
                "▶ " {play_label}
            </a>
        }
    });

    let slide_present_btn = is_slide.then(|| view! {
        <button type="button" class="btn btn-secondary btn-sm" onclick="document.getElementById('editor-present-trigger')?.click()">"🖥️ " {i18n.present_mode()}</button>
    });

    let is_typst_preview = is_slide
        || std::path::Path::new(&file_path)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("typ"));

    let pdf_view_browser_btn = is_typst_preview.then(|| {
        let pdf_href = format!(
            "/projects/{}/pdf-view?file={}",
            project_id,
            urlencoding::encode(&file_path)
        );
        let pdf_label = if i18n.is_zh() { "浏览器预览 PDF" } else { "View PDF in Browser" };
        view! {
            <a
                href=pdf_href
                target="_blank"
                rel="noopener noreferrer"
                class="btn btn-secondary btn-sm"
                title=if i18n.is_zh() { "在浏览器中查看渲染后的 PDF 并支持添加页面评审意见" } else { "View rendered PDF in browser with page review comments" }
                style="display:inline-flex; align-items:center; gap:0.35rem; font-weight:600;"
            >
                "📄 " {pdf_label}
            </a>
        }
    });

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
        <a href=href download=format!("{}.pdf", download_pdf_name) class="btn btn-secondary btn-sm">"⬇️ " {i18n.download_pdf()}</a>
    });
    let download_slide_bin_btn = is_slide.then(|| view! {
        <apich_islands::SlideBuildIsland project_id=project_id.to_string() file_path=file_path.clone() />
    });
    let multi_render_btn = (is_typst_preview || is_latex_preview).then(|| {
        let doc_type = if is_typst_preview { "typst" } else { "latex" };
        view! {
            <apich_islands::MultiFileRenderModalIsland
                project_id=project_id.to_string()
                active_file=file_path.clone()
                doc_type=doc_type.to_string()
                available_files=available_sibling_files.clone()
                is_zh=i18n.is_zh()
            />
        }
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
                            <a href="/" class="btn btn-secondary btn-sm editor-back-btn" title=i18n.back_to_dashboard()>
                                <span aria-hidden="true">"←"</span>
                                <span>{i18n.back_to_dashboard()}</span>
                            </a>
                        }.into_any()
                    } else {
                        view! {
                            <a href=format!("/projects/{}?tab=files", project_id) class="btn btn-secondary btn-sm editor-back-btn" title=i18n.back_to_files()>
                                <span aria-hidden="true">"←"</span>
                                <span>{i18n.back_to_files()}</span>
                            </a>
                        }.into_any()
                    }}
                    <span class="editor-top-divider" aria-hidden="true"></span>
                    <span class="editor-file-name" title=file_path.clone()>"📄 " {file_path.clone()}</span>
                    <span class="file-type-pill pill-doc" style="font-size:0.7rem;">{kind_label}</span>
                </div>
                <div class="editor-top-right">
                    <button
                        type="button"
                        class="btn btn-secondary btn-sm"
                        onclick="window.dispatchEvent(new CustomEvent('apich-open-attach-modal', {detail: {mode: 'upload'}}))"
                        title=if i18n.is_zh() { "上传本地文件或图片至项目" } else { "Upload local file or image to project" }
                    >
                        "📤 " {i18n.upload()}
                    </button>
                    <button
                        type="button"
                        class="btn btn-secondary btn-sm"
                        onclick="window.dispatchEvent(new CustomEvent('apich-open-attach-modal', {detail: {mode: 'attach'}}))"
                        title=if i18n.is_zh() { "插入项目文件或图片引用" } else { "Insert reference to project file or image" }
                    >
                        "📎 " {i18n.attach()}
                    </button>
                    <a href=format!("/projects/{}?tab=vcs&file={}", project_id, urlencoding::encode(&file_path)) class="btn btn-secondary btn-sm" title=if i18n.is_zh() { "查看版本控制时间线和文件历史" } else { "View version control timeline and file history" }>"🌿 " {i18n.history()}</a>
                    <button type="button" class="btn btn-secondary btn-sm" onclick=share_onclick title=share_label>"🔗 " {i18n.share()}</button>
                    <label for="ai-drawer-toggle-cb" class="btn btn-secondary btn-sm">"🤖 " {i18n.ai_copilot()}</label>
                    <apich_islands::SearchReplaceModalIsland
                        project_id=project_id.to_string()
                        active_file=file_path.clone()
                        files_json=serde_json::to_string(&available_sibling_files).unwrap_or_else(|_| "[]".to_string())
                        is_zh=i18n.is_zh()
                        trigger_label=if i18n.is_zh() { "🔍 查找与替换".to_string() } else { "🔍 Find / Replace".to_string() }
                        trigger_class="btn btn-secondary btn-sm".to_string()
                    />
                    {multi_render_btn}
                    {download_pdf_btn}
                    {download_slide_bin_btn}
                    {slide_play_browser_btn}
                    {pdf_view_browser_btn}
                    {slide_present_btn}
                    {if is_single_file {
                        let cf_file = file_path.clone();
                        let confirm_msg = i18n.delete_file_confirm(&cf_file);
                        view! {
                            <form
                                method="post"
                                action=format!("/projects/{}/delete", project_id)
                                class="inline-form"
                                onsubmit=format!("return confirm('{}');", confirm_msg.replace('\'', "\\'"))
                            >
                                <button
                                    type="submit"
                                    class="btn btn-danger btn-sm"
                                    title=i18n.delete_file()
                                >
                                    "🗑️ " {i18n.delete()}
                                </button>
                            </form>
                        }.into_any()
                    } else {
                        view! { <span></span> }.into_any()
                    }}
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
                is_zh=i18n.is_zh()
            />

            {template_panel}

            <FileShareModal
                project_id=project_id
                redirect_to=format!("/projects/{}/editor?file={}", project_id, urlencoding::encode(&file_path))
                all_users=all_users
                owner_id=project_owner_id
                i18n=i18n
            />
            <AttachModalIsland
                project_id=project_id.to_string()
                active_file=file_path.clone()
                is_zh=i18n.is_zh()
            />
            <AiDrawer project_id=project_id file_path=file_path is_zh=i18n.is_zh() />
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
            <form method="post" action=format!("/projects/{}/apply-file-template", project_id) class="inline-form" style="display:flex; gap:0.5rem; margin-top:0.6rem; align-items:center; flex-wrap:wrap;">
                <select name="template_version_id" class="form-control" style="min-width:320px; max-width:520px; width:auto; font-size:0.875rem;" required=true>
                    <option value="" disabled=true selected=true>{i18n.template_select_version()}</option>
                    {apply_options}
                </select>
                <input type="text" name="dest_subdir" placeholder=i18n.template_dest_folder_field() class="form-control" style="min-width:180px; font-size:0.875rem;" />
                <button type="submit" class="btn btn-primary btn-sm">{i18n.template_apply()}</button>
            </form>
        }
    });

    view! {
        <details style="margin-top:1rem; background:var(--bg-muted); border-radius:10px; padding:0.85rem 1rem;">
            <summary style="cursor:pointer; font-weight:600; font-size:0.85rem;">{i18n.template_publish_as_template()}" / "{i18n.template_apply_template()}</summary>
            <div style="margin-top:0.6rem;">
                <form method="post" action="/templates/create-from-file" class="inline-form" style="display:flex; gap:0.5rem; flex-wrap:wrap; align-items:center;">
                    <input type="hidden" name="project_id" value=project_id.to_string() />
                    <input type="hidden" name="file" value=file_path.to_string() />
                    <input type="hidden" name="kind" value=kind.to_string() />
                    <input type="text" name="name" placeholder=i18n.template_name_field() class="form-control" style="min-width:160px; font-size:0.875rem;" required=true />
                    <input type="text" name="slug" placeholder=i18n.template_slug_field() class="form-control" style="min-width:160px; font-size:0.875rem;" required=true />
                    <input type="text" name="description" placeholder=i18n.template_description_field() class="form-control" style="min-width:200px; font-size:0.875rem;" />
                    <select name="visibility" class="form-control" style="min-width:140px; font-size:0.875rem;">
                        <option value="private">{i18n.template_visibility_label("private")}</option>
                        <option value="shared">{i18n.template_visibility_label("shared")}</option>
                        <option value="public">{i18n.template_visibility_label("public")}</option>
                    </select>
                    <input type="text" name="version_label" placeholder=i18n.template_version_label_field() class="form-control" style="min-width:120px; font-size:0.875rem;" required=true value="1.0.0" />
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
