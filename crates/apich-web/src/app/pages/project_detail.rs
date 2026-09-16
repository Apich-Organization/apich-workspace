use crate::app::components::ActiveNav;
use crate::app::components::AiDrawer;
use crate::app::components::AppShell;
use crate::services::project_manager::ConflictFileView;
use crate::services::project_manager::GitStatusView;
use crate::services::project_manager::ProjectFileItem;
use crate::ui::i18n::I18n;
use apich_db::EffectiveHubLinks;
use apich_db::Project;
use apich_db::ProjectMemberWithUser;
use apich_db::User;
use apich_vcs::Snapshot;
use leptos::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectTab {
    Files,
    Vcs,
    Sharing,
}

impl ProjectTab {
    pub fn from_str(s: &str) -> Self {
        match s {
            | "vcs" | "merge" | "timeline" | "git" => Self::Vcs,
            | "sharing" | "members" => Self::Sharing,
            | _ => Self::Files,
        }
    }

    pub fn from_string(s: &str) -> Self {
        Self::from_str(s)
    }
}

#[allow(clippy::too_many_arguments)]
#[component]
pub fn ProjectDetailPage(
    user: User,
    is_org_or_team_admin: bool,
    project: Project,
    hub_links: Option<EffectiveHubLinks>,
    branches: Vec<String>,
    current_branch: Option<String>,
    conflicts: Vec<ConflictFileView>,
    snapshots: Vec<Snapshot>,
    members: Vec<ProjectMemberWithUser>,
    all_users: Vec<User>,
    git_status: GitStatusView,
    files: Vec<ProjectFileItem>,
    /// "tree" (browse one directory at a time) or "flat" (every file at once).
    files_view: String,
    /// Directory currently being browsed in "tree" view, relative to the project root ("" = root).
    files_dir: String,
    ignore_config: apich_vcs::IgnoreConfig,
    gitignore_content: String,
    apichignore_content: String,
    new_file_templates: Vec<apich_db::TemplateWithLatestVersion>,
    active_tab: ProjectTab,
    notice: Option<String>,
    signature_statuses: std::collections::HashMap<uuid::Uuid, apich_vcs::SignatureStatus>,
    milestones: Vec<Snapshot>,
    i18n: I18n,
    current_path: String,
) -> impl IntoView {
    let tab = active_tab;
    let is_owner = project.owner_id == user.id || user.is_platform_admin;
    let project_id = project.id;

    let project_name_for_rename = project.name.clone();
    let project_desc_for_rename = project.description.clone().unwrap_or_default();

    let is_single_file = project
        .settings
        .get("is_single_file")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let single_file_name = project
        .settings
        .get("single_file_name")
        .and_then(|v| v.as_str())
        .unwrap_or(&project.name)
        .to_string();
    let single_file_ext = std::path::Path::new(&single_file_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let single_file_editor_href = match single_file_ext.as_str() {
        "table" | "db" | "sqlite" => format!("/projects/{}/table?file={}", project_id, urlencoding::encode(&single_file_name)),
        "anote" | "note" => format!("/projects/{}/note?file={}", project_id, urlencoding::encode(&single_file_name)),
        _ => format!("/projects/{}/editor?file={}", project_id, urlencoding::encode(&single_file_name)),
    };
    let display_title = if is_single_file {
        single_file_name.clone()
    } else {
        project.name.clone()
    };
    let display_subtitle = if is_single_file {
        "Standalone single file with dedicated VCS timeline & sharing".to_string()
    } else {
        project.description.clone().unwrap_or_default()
    };

    let notice_alert = notice.map(
        |n| view! { <div class="alert alert-success" style="margin-bottom:1.5rem;">{n}</div> },
    );

    let hub_bar = hub_links.map(|h| render_hub_links_bar(&h));

    let files_len = files.len();
    let conflicts_len = conflicts.len();
    let snapshots_len = snapshots.len();
    let members_len = members.len();

    let tab_bar = render_tab_bar(
        project_id,
        tab,
        files_len,
        conflicts_len,
        snapshots_len,
        members_len,
        is_single_file,
        i18n,
    );

    let tab_content = match tab {
        | ProjectTab::Files => {
            render_files_tab(
                &project,
                files,
                all_users,
                new_file_templates,
                files_view,
                files_dir,
                i18n,
            )
            .into_any()
        },
        | ProjectTab::Vcs => {
            render_vcs_tab(VcsTabArgs {
                project: &project,
                is_owner,
                branches,
                current_branch,
                conflicts,
                snapshots,
                signature_statuses,
                milestones,
                git: git_status,
                ignore_config,
                gitignore_content,
                apichignore_content,
                i18n,
            })
            .into_any()
        },
        | ProjectTab::Sharing => {
            render_sharing_tab(&project, members, all_users, is_owner, i18n).into_any()
        },
    };

    view! {
        <AppShell
            user=user
            is_org_or_team_admin=is_org_or_team_admin
            active_nav=ActiveNav::Projects
            current_path=current_path
            page_title=display_title.clone()
            i18n=i18n
        >
            <div class="page-header">
                <div>
                    <div class="title-with-badge" style="display:flex; align-items:center; gap:0.6rem;">
                        <h1 class="page-title">{display_title.clone()}</h1>
                        {if is_single_file {
                            view! {
                                <div style="display:flex; align-items:center; gap:0.4rem;">
                                    <span class="badge badge-secondary" style="font-size:0.75rem;">
                                        "📄 Standalone File"
                                    </span>
                                    <apich_islands::FileActionDropdownIsland
                                        project_id=project_id.to_string()
                                        file_path=single_file_name.clone()
                                        file_name=single_file_name.clone()
                                        open_url=single_file_editor_href.clone()
                                        is_dir=false
                                        is_single_file=true
                                        is_zh=i18n.is_zh()
                                    />
                                </div>
                            }.into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }}
                    </div>
                    <p class="page-subtitle">{display_subtitle}</p>
                </div>
                <div class="header-actions">
                    {if is_single_file {
                        view! {
                            <div style="display:flex; gap:0.5rem; align-items:center;">
                                <a href="/" class="btn btn-secondary">"← Dashboard"</a>
                                <a href=single_file_editor_href.clone() class="btn btn-primary">"📂 Open File"</a>
                                <label for="ai-drawer-toggle-cb" class="btn btn-secondary">"🤖 AI Copilot"</label>
                                <apich_islands::ModalIsland trigger_label=i18n.create_snapshot().to_string() trigger_class="btn btn-secondary".to_string() title=i18n.modal_snapshot_title().to_string()>
                                    <p style="font-size:0.85rem; color:var(--text-muted); margin-bottom:1.25rem;">{i18n.modal_snapshot_desc()}</p>
                                    <form method="post" action=format!("/projects/{}/snapshot", project_id)>
                                        <div class="form-group">
                                            <label for="message">{i18n.snapshot_message_label()}</label>
                                            <input type="text" id="message" name="message" required=true placeholder="Updated latest revisions..." class="form-control" autofocus=true />
                                        </div>
                                        <div style="display:flex; justify-content:flex-end; gap:0.75rem; margin-top:1.5rem;">
                                            <button type="submit" class="btn btn-primary">{i18n.snapshot_submit()}</button>
                                        </div>
                                    </form>
                                </apich_islands::ModalIsland>
                                {if is_owner {
                                    let cf_file = single_file_name.clone();
                                    view! {
                                        <form
                                            method="post"
                                            action=format!("/projects/{}/delete", project_id)
                                            class="inline-form"
                                            onsubmit=format!("return confirm('Are you sure you want to permanently delete \"{}\"? This cannot be undone.');", cf_file)
                                        >
                                            <button
                                                type="submit"
                                                class="btn btn-danger"
                                                title="Permanently delete this file"
                                            >
                                                "🗑️ Delete File"
                                            </button>
                                        </form>
                                    }.into_any()
                                } else {
                                    view! { <span></span> }.into_any()
                                }}
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div style="display:flex; gap:0.5rem; align-items:center;">
                                <apich_islands::ModalIsland trigger_label="✏️ Rename".to_string() trigger_class="btn btn-secondary".to_string() title="Rename Project".to_string()>
                                    <form method="post" action=format!("/projects/{}/rename", project_id)>
                                        <div class="form-group">
                                            <label>"Project Name"</label>
                                            <input type="text" name="name" required=true value=project_name_for_rename class="form-control" />
                                        </div>
                                        <div class="form-group">
                                            <label>"Description (optional)"</label>
                                            <textarea name="description" rows="3" class="form-control">{project_desc_for_rename}</textarea>
                                        </div>
                                        <p style="font-size:0.78rem; color:var(--text-sub); margin:0.25rem 0 0;">
                                            "The project's URL and workspace folder stay the same, so existing links keep working."
                                        </p>
                                        <div style="display:flex; justify-content:flex-end; gap:0.75rem; margin-top:1.5rem;">
                                            <button type="submit" class="btn btn-primary">"Save"</button>
                                        </div>
                                    </form>
                                </apich_islands::ModalIsland>
                                <label for="ai-drawer-toggle-cb" class="btn btn-secondary">"🤖 AI Copilot"</label>
                                <a href=format!("/projects/{}/note", project.id) class="btn btn-secondary">"📔 Notes & Wiki"</a>
                                <a href=format!("/projects/{}/terminal", project.id) class="btn btn-secondary">"💻 Terminal"</a>
                                <apich_islands::ModalIsland trigger_label=i18n.create_snapshot().to_string() trigger_class="btn btn-secondary".to_string() title=i18n.modal_snapshot_title().to_string()>
                                    <p style="font-size:0.85rem; color:var(--text-muted); margin-bottom:1.25rem;">{i18n.modal_snapshot_desc()}</p>
                                    <form method="post" action=format!("/projects/{}/snapshot", project_id)>
                                        <div class="form-group">
                                            <label for="message">{i18n.snapshot_message_label()}</label>
                                            <input type="text" id="message" name="message" required=true placeholder="Updated Hamiltonian simulation equations..." class="form-control" autofocus=true />
                                        </div>
                                        <div style="display:flex; justify-content:flex-end; gap:0.75rem; margin-top:1.5rem;">
                                            <button type="submit" class="btn btn-primary">{i18n.snapshot_submit()}</button>
                                        </div>
                                    </form>
                                </apich_islands::ModalIsland>
                            </div>
                        }.into_any()
                    }}
                </div>
            </div>

            {notice_alert}
            {hub_bar}
            {tab_bar}
            {tab_content}
            <AiDrawer project_id=project_id />
        </AppShell>
    }
}

fn render_hub_links_bar(links: &EffectiveHubLinks) -> impl IntoView {
    // Each service gets its own accent (see the `.hub-*` rules in `styles.rs`) so the four are
    // distinguishable at a glance rather than reading as one undifferentiated row of links.
    let items: Vec<_> = [
        ("💬", "Chat", "hub-chat", &links.chat_url),
        ("📹", "Meeting", "hub-meeting", &links.meeting_url),
        ("💾", "Drive", "hub-drive", &links.drive_url),
        ("🤖", "AI Agent", "hub-ai", &links.ai_agent_url),
    ]
    .into_iter()
    .filter_map(|(icon, label, tone, url)| {
        url.as_ref().map(|u| {
            view! {
                <a href=u.clone() target="_blank" class=format!("hub-link-chip {tone}")>
                    <span class="hub-link-icon">{icon}</span>
                    <span class="hub-link-label">{label}</span>
                    <span class="hub-link-arrow">"↗"</span>
                </a>
            }
        })
    })
    .collect();

    if items.is_empty() {
        return view! { <div></div> }.into_any();
    }

    let override_badge = links
        .is_team_override
        .then(|| view! { <span class="badge-override">"Team Override"</span> });

    view! {
        <div class="hub-links-bar">
            {items}
            {override_badge}
        </div>
    }
    .into_any()
}

fn render_tab_bar(
    project_id: uuid::Uuid,
    active: ProjectTab,
    files_count: usize,
    conflicts_count: usize,
    snapshots_count: usize,
    members_count: usize,
    is_single_file: bool,
    i18n: I18n,
) -> impl IntoView {
    if is_single_file {
        view! {
            <div class="tab-bar">
                <a href=format!("/projects/{}?tab=vcs", project_id) class="tab-item" class:active=active == ProjectTab::Vcs>
                    "🌿 " {i18n.tab_vcs()} " & History"
                    {(conflicts_count > 0).then(|| view! { <span class="status-badge badge-warning" style="margin-left:4px;">{conflicts_count}</span> })}
                    {(snapshots_count > 0).then(|| format!(" ({snapshots_count})"))}
                </a>
                <a href=format!("/projects/{}?tab=sharing", project_id) class="tab-item" class:active=active == ProjectTab::Sharing>
                    "👥 " {i18n.tab_sharing()} " & Access"
                    {(members_count > 0).then(|| format!(" ({members_count})"))}
                </a>
            </div>
        }.into_any()
    } else {
        view! {
            <div class="tab-bar">
                <a href=format!("/projects/{}?tab=files", project_id) class="tab-item" class:active=active == ProjectTab::Files>
                    "📁 " {i18n.tab_files()} " (" {files_count} ")"
                </a>
                <a href=format!("/projects/{}?tab=vcs", project_id) class="tab-item" class:active=active == ProjectTab::Vcs>
                    "🌿 " {i18n.tab_vcs()}
                    {(conflicts_count > 0).then(|| view! { <span class="status-badge badge-warning" style="margin-left:4px;">{conflicts_count}</span> })}
                    {(snapshots_count > 0).then(|| format!(" ({snapshots_count})"))}
                </a>
                <a href=format!("/projects/{}?tab=sharing", project_id) class="tab-item" class:active=active == ProjectTab::Sharing>
                    "👥 " {i18n.tab_sharing()}
                    {(members_count > 0).then(|| format!(" ({members_count})"))}
                </a>
            </div>
        }.into_any()
    }
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
/// `<option>` list for a destination-folder `<select>`: the project root plus every existing
/// folder, with `selected` on whichever directory is currently being browsed so the dialog
/// defaults to "right here" rather than always the root.
///
/// Built as owned `AnyView`s (one call per dialog) because each dialog is an island whose
/// children must be `'static` -- a closure borrowing `folders`/`cur_dir` inline in the middle of
/// a modal's `view!` tree would try to carry those borrows past the caller's return.
fn folder_select_options(
    folders: &[String],
    cur_dir: &str,
) -> Vec<AnyView> {
    std::iter::once(&String::new())
        .chain(folders.iter())
        .map(|folder| {
            let label = if folder.is_empty() {
                "/ (project root)".to_string()
            } else {
                folder.clone()
            };
            let selected = folder == cur_dir;
            view! { <option value=folder.clone() selected=selected>{label}</option> }.into_any()
        })
        .collect()
}

fn render_files_tab(
    project: &Project,
    files: Vec<ProjectFileItem>,
    all_users: Vec<User>,
    new_file_templates: Vec<apich_db::TemplateWithLatestVersion>,
    files_view: String,
    files_dir: String,
    i18n: I18n,
) -> impl IntoView {
    let project_id = project.id;
    let is_tree = files_view != "flat";
    let cur_dir = files_dir.trim_matches('/').to_string();
    drop(files_view);
    drop(files_dir);

    // Every folder that exists anywhere in the project, for the destination pickers in the
    // "+ New File"/"Upload File" dialogs -- a real `<select>` of what's actually there beats a
    // free-text field that silently creates a new folder (or fails) on a typo, and saves the user
    // having to remember exact paths. Derived from the full file list *before* it's narrowed to
    // the browsed directory below, since the picker should offer every folder, not just the ones
    // visible at the current level. Includes each ancestor of a nested path, so `assets/videos`
    // contributes both `assets` and `assets/videos`.
    let mut all_folders: Vec<String> = Vec::new();
    for f in &files {
        let dir_path = if f.is_dir {
            f.path.as_str()
        } else {
            match f.path.rsplit_once('/') {
                | Some((parent, _)) => parent,
                | None => continue,
            }
        };
        let mut walked = String::new();
        for seg in dir_path.split('/').filter(|s| !s.is_empty()) {
            if walked.is_empty() {
                walked = seg.to_string();
            } else {
                walked = format!("{walked}/{seg}");
            }
            if !all_folders.contains(&walked) {
                all_folders.push(walked.clone());
            }
        }
    }
    let all_file_paths: Vec<String> = files.iter().map(|f| f.path.clone()).collect();
    let existing_files_json = serde_json::to_string(&all_file_paths).unwrap_or_else(|_| "[]".to_string());
    let folders_json = serde_json::to_string(&all_folders).unwrap_or_else(|_| "[]".to_string());

    // In tree view, `files` (a flat list of every file in the project, at every depth -- see
    // `ProjectManager::list_files`) is collapsed to just what lives *directly* in `cur_dir`:
    // its own immediate files, plus one synthetic row per immediate subdirectory, so a folder is
    // something the user can actually click into rather than only ever being implied by the
    // slashes in some deeper file's path. `list_files` already emits real rows for *empty*
    // directories, so those are kept as-is and only non-empty ones need synthesizing here.
    let (files, breadcrumbs): (Vec<ProjectFileItem>, Vec<(String, String)>) = if is_tree {
        let prefix = if cur_dir.is_empty() {
            String::new()
        } else {
            format!("{cur_dir}/")
        };

        let mut here: Vec<ProjectFileItem> = Vec::new();
        let mut subdirs: Vec<String> = Vec::new();
        for f in files {
            let Some(rest) = f.path.strip_prefix(&prefix) else {
                continue;
            };
            if rest.is_empty() {
                continue;
            }
            match rest.split_once('/') {
                // Lives in a deeper directory -- surface only that directory, once.
                | Some((child_dir, _)) => {
                    let child = format!("{prefix}{child_dir}");
                    if !subdirs.contains(&child) {
                        subdirs.push(child);
                    }
                },
                | None => here.push(f),
            }
        }
        // An empty directory already has its own row from `list_files`; drop it from `subdirs` so
        // it isn't listed twice.
        let existing_dirs: Vec<String> =
            here.iter().filter(|f| f.is_dir).map(|f| f.path.clone()).collect();
        for dir_path in subdirs {
            if existing_dirs.contains(&dir_path) {
                continue;
            }
            let name = dir_path.rsplit('/').next().unwrap_or(&dir_path).to_string();
            here.push(ProjectFileItem {
                path: dir_path,
                name,
                is_dir: true,
                size_bytes: 0,
                extension: String::new(),
                category: "folder".to_string(),
                modified_rfc3339: None,
                open_url: String::new(),
                share_info: None,
            });
        }
        // Folders first, then files, each alphabetical -- the conventional file-browser ordering.
        here.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.path.cmp(&b.path)));

        let mut crumbs = vec![(String::new(), "Project root".to_string())];
        let mut walked = String::new();
        for seg in cur_dir.split('/').filter(|s| !s.is_empty()) {
            if walked.is_empty() {
                walked = seg.to_string();
            } else {
                walked = format!("{walked}/{seg}");
            }
            crumbs.push((walked.clone(), seg.to_string()));
        }
        (here, crumbs)
    } else {
        (files, Vec::new())
    };

    // Built as fully owned data before the `<apich_islands::ModalIsland>` below rather than
    // inline inside its children: islands require their children to be `'static`, but this
    // function only borrows `new_file_templates`/`i18n` -- a closure built inline in the middle
    // of the modal's view! tree would try to carry those borrows past this function's return,
    // which doesn't typecheck (`i18n.template_kind_label` itself already returns `&'static str`,
    // it's the surrounding closure capturing the borrowed slice/reference that doesn't).
    let new_file_template_picker = (!new_file_templates.is_empty()).then(|| {
        let options: Vec<_> = new_file_templates
            .into_iter()
            .filter_map(|t| t.latest_version_id.map(|vid| (t, vid)))
            .map(|(t, vid)| {
                let label = format!(
                    "[{}] {} ({})",
                    i18n.template_kind_label(&t.kind),
                    t.name,
                    t.latest_version_label.unwrap_or_default()
                );
                view! { <option value=vid.to_string()>{label}</option> }
            })
            .collect();
        view! {
            <div class="form-group">
                <label>"Or start from a "<a href="/templates" target="_blank">"Template Library"</a>" template (overrides the starter above, keeps your file name):"</label>
                <select name="template_version_id" class="form-control">
                    <option value="">"-- none, use the starter template above --"</option>
                    {options}
                </select>
            </div>
        }
    });

    // Each modal below is an island whose children must be `'static`, so every one that
    // pre-fills the browsed directory needs its own owned copy.
    let folder_options_for_new_file = folder_select_options(&all_folders, &cur_dir);
    let folder_options_for_upload = folder_select_options(&all_folders, &cur_dir);
    let folder_options_for_new_folder = folder_select_options(&all_folders, &cur_dir);

    // "Browsing: root / assets / videos" -- each ancestor clickable to jump back up. Only
    // meaningful in tree view (flat view isn't scoped to a directory at all).
    let breadcrumb_bar = (is_tree && !breadcrumbs.is_empty()).then(|| {
        let last_idx = breadcrumbs.len().saturating_sub(1);
        let crumbs: Vec<_> = breadcrumbs
            .into_iter()
            .enumerate()
            .map(|(idx, (dir_path, label))| {
                let is_last = idx == last_idx;
                let sep = (idx > 0).then(|| view! { <span style="color:var(--text-sub); margin:0 0.35rem;">"/"</span> });
                let crumb = if is_last {
                    view! { <span style="font-weight:600;">{label}</span> }.into_any()
                } else {
                    let href = format!(
                        "/projects/{}?tab=files&dir={}",
                        project_id,
                        urlencoding::encode(&dir_path)
                    );
                    view! { <a href=href>{label}</a> }.into_any()
                };
                view! { <span>{sep}{crumb}</span> }
            })
            .collect();
        view! {
            <div style="display:flex; align-items:center; font-size:0.85rem; margin-bottom:0.85rem;">
                <span style="color:var(--text-sub); margin-right:0.5rem;">"📂"</span>
                {crumbs}
            </div>
        }
    });

    let flat_url = format!("/projects/{project_id}?tab=files&view=flat");
    let tree_url = format!("/projects/{project_id}?tab=files");
    let view_toggle = view! {
        <div style="display:flex; gap:0.35rem; align-items:center;">
            <a
                href=tree_url
                class=if is_tree { "btn btn-primary btn-sm" } else { "btn btn-secondary btn-sm" }
                style="padding:0.2rem 0.6rem; font-size:0.75rem;"
                title="Browse one folder at a time"
            >"📂 Folders"</a>
            <a
                href=flat_url
                class=if is_tree { "btn btn-secondary btn-sm" } else { "btn btn-primary btn-sm" }
                style="padding:0.2rem 0.6rem; font-size:0.75rem;"
                title="List every file in the project at once"
            >"☰ All files"</a>
        </div>
    };

    let rows = if files.is_empty() {
        let empty_msg = if is_tree && !cur_dir.is_empty() {
            "This folder is empty. Use 'Upload File' or '+ New File' to add something to it."
        } else {
            "No files yet. Click '+ New File' below to get started."
        };
        view! {
            <tr><td colspan="6" style="text-align:center; padding:2.5rem; color:var(--text-sub);">
                {empty_msg}
            </td></tr>
        }
        .into_any()
    } else {
        files
            .into_iter()
            .map(|f| {
                // A folder has no content/preview/share affordances of its own -- just a name to
                // click into and a way to remove it -- so it gets its own short-circuited row
                // rather than falling through the file-row layout below with a bunch of
                // meaningless blank cells (a 0-byte "size", no share state, an "Open" link to
                // nowhere).
                if f.is_dir {
                    let path = f.path;
                    // Only navigable in tree view: flat view deliberately shows every file at
                    // every depth at once, so "descend into this directory" has no meaning there.
                    let browse_url = format!(
                        "/projects/{}?tab=files&dir={}",
                        project_id,
                        urlencoding::encode(&path)
                    );
                    let label = f.name;
                    let is_parent = label == "..";
                    let name_cell = if is_tree {
                        view! {
                            <a href=browse_url.clone()>{label.clone()} "/"</a>
                        }.into_any()
                    } else {
                        view! {
                            <span style="color:var(--text-sub);">{path.clone()} "/"</span>
                        }.into_any()
                    };
                    return view! {
                        <tr
                            data-name=label.to_lowercase()
                            data-type="folder"
                            data-sharing=""
                            data-size="0"
                            data-modified=""
                            data-is-dir="1"
                            data-is-parent=if is_parent { "1" } else { "0" }
                        >
                            <td>
                                <div class="file-name-cell">
                                    <span style="font-size:1.1rem;">"📁"</span>
                                    {name_cell}
                                    {(!is_parent).then(|| {
                                        view! {
                                            <apich_islands::FileActionDropdownIsland
                                                project_id=project_id.to_string()
                                                file_path=path.clone()
                                                file_name=label.clone()
                                                open_url=browse_url.clone()
                                                is_dir=true
                                                is_zh=i18n.is_zh()
                                            />
                                        }
                                    })}
                                </div>
                            </td>
                            <td><span class="file-type-pill pill-asset">"folder"</span></td>
                            <td></td>
                            <td style="color:var(--text-sub); font-size:0.8rem;">"-"</td>
                            <td style="color:var(--text-sub); font-size:0.8rem;">"-"</td>
                            <td style="text-align:right;">
                                <div style="display:flex; justify-content:flex-end; gap:0.35rem; align-items:center;">
                                    {is_tree.then(|| view! {
                                        <a href=browse_url class="btn btn-secondary btn-sm" style="padding:0.2rem 0.55rem; font-size:0.75rem;">"Open"</a>
                                    })}
                                    <form method="post" action=format!("/projects/{}/files/delete", project_id) class="inline-form">
                                        <input type="hidden" name="file" value=path />
                                        <apich_islands::ConfirmSubmitButton
                                            label="Del".to_string()
                                            message="Delete this folder and everything in it?".to_string()
                                            button_class="btn btn-ghost btn-sm text-danger".to_string()
                                            button_style="padding:0.2rem 0.45rem; font-size:0.75rem;".to_string()
                                        />
                                    </form>
                                </div>
                            </td>
                        </tr>
                    }.into_any();
                }
                let (icon, pill_class) = match f.category.as_str() {
                    "script" => ("🐍", "pill-script"),
                    "slide" => ("📊", "pill-slide"),
                    "typst" => ("📄", "pill-typst"),
                    "latex" => ("📝", "pill-latex"),
                    "table" => ("🗄️", "pill-table"),
                    "note" => ("📔", "pill-note"),
                    _ => ("📎", "pill-asset"),
                };
                let kb = f.size_bytes / 1024;
                let tenths = (f.size_bytes % 1024).saturating_mul(10) / 1024;
                let size_kb = format!("{kb}.{tenths} KB");
                let mod_time = f.modified_rfc3339.as_deref().unwrap_or("-").to_string();
                let mod_raw = f.modified_rfc3339.clone().unwrap_or_default();
                let share_badge = match f.share_info.as_ref() {
                    Some(info) if info.mode == "public" => view! { <span class="share-badge share-badge-public">"🌐 Public (" {info.role.clone()} ")"</span> }.into_any(),
                    Some(info) if info.mode == "specific" => view! { <span class="share-badge share-badge-specific">"👥 Specific (" {info.role.clone()} ")"</span> }.into_any(),
                    _ => view! { <span class="share-badge share-badge-private">"🔒 Private"</span> }.into_any(),
                };

                let mode = f.share_info.as_ref().map_or_else(|| "private".to_string(), |s| s.mode.clone());
                let role = f.share_info.as_ref().map_or_else(|| "read".to_string(), |s| s.role.clone());
                let users_csv = f.share_info.as_ref().map(|s| s.allowed_users.join(",")).unwrap_or_default();
                let path = f.path;
                // Tree view is already scoped to one directory (and shows a breadcrumb for where
                // that is), so repeating the full path on every row would be pure noise; flat
                // view has no such context and needs the path to disambiguate same-named files
                // in different folders.
                let display_name = if is_tree { f.name.clone() } else { path.clone() };
                let sort_name = display_name.to_lowercase();
                let cat = f.category.clone();
                let open_url = f.open_url;
                // "asset" files (PDFs, images, audio -- anything routed to the raw-bytes endpoint
                // rather than an in-app editor route, see `ProjectFileItem::open_url`'s doc
                // comment) used to open with a plain same-tab link: for a PDF specifically, the
                // browser's native inline viewer then *replaces* this whole app page -- there's
                // no "back to the file list" without an actual browser Back navigation, and any
                // in-progress state (open editors, unsaved form fields) on this page is gone. The
                // in-app editor routes (slide/typst/latex/script/table/note/other -- "other" now
                // also opens in the plain-text editor, see `open_url`'s doc comment) stay
                // same-tab on purpose -- only "asset" is raw, non-SPA content.
                let open_target = if f.category == "asset" { "_blank" } else { "_self" };
                let share_detail = serde_json::json!({ "path": path, "mode": mode, "role": role, "users": users_csv }).to_string();
                let onclick = format!("window.dispatchEvent(new CustomEvent('apich-open-share-modal', {{detail: {share_detail}}}))");

                view! {
                    <tr
                        data-name=sort_name
                        data-type=cat.clone()
                        data-sharing=mode.clone()
                        data-size=format!("{}", f.size_bytes)
                        data-modified=mod_raw
                        data-is-dir="0"
                        data-is-parent="0"
                    >
                        <td>
                            <div class="file-name-cell">
                                <span style="font-size:1.1rem;">{icon}</span>
                                <a href=open_url.clone() target=open_target>{display_name.clone()}</a>
                                <apich_islands::FileActionDropdownIsland
                                    project_id=project_id.to_string()
                                    file_path=path.clone()
                                    file_name=f.name.clone()
                                    open_url=open_url.clone()
                                    open_target=open_target.to_string()
                                    is_dir=false
                                    share_mode=mode.clone()
                                    share_role=role.clone()
                                    share_users_csv=users_csv.clone()
                                    is_zh=i18n.is_zh()
                                />
                            </div>
                        </td>
                        <td><span class=format!("file-type-pill {}", pill_class)>{cat.clone()}</span></td>
                        <td>{share_badge}</td>
                        <td style="color:var(--text-sub); font-size:0.8rem;">{size_kb}</td>
                        <td style="color:var(--text-sub); font-size:0.8rem;">{mod_time}</td>
                        <td style="text-align:right;">
                            <div style="display:flex; justify-content:flex-end; gap:0.35rem; align-items:center;">
                                <a href=open_url target=open_target class="btn btn-secondary btn-sm" style="padding:0.2rem 0.55rem; font-size:0.75rem;">"Open"</a>
                                <button type="button" class="btn btn-secondary btn-sm" style="padding:0.2rem 0.55rem; font-size:0.75rem;" onclick=onclick>"Share"</button>
                                <form method="post" action=format!("/projects/{}/files/delete", project_id) class="inline-form">
                                    <input type="hidden" name="file" value=path />
                                    <apich_islands::ConfirmSubmitButton
                                        label="Del".to_string()
                                        message="Delete this file?".to_string()
                                        button_class="btn btn-ghost btn-sm text-danger".to_string()
                                        button_style="padding:0.2rem 0.45rem; font-size:0.75rem;".to_string()
                                    />
                                </form>
                            </div>
                        </td>
                    </tr>
                }.into_any()
            })
            .collect::<Vec<_>>()
            .into_any()
    };

    view! {
        <div class="section-card" style="margin-bottom:2rem;">
            <div style="display:flex; justify-content:space-between; align-items:center; border-bottom:1px solid var(--border-subtle); padding-bottom:1rem; margin-bottom:1.25rem;">
                <div>
                    <h3 class="card-subtitle">"Project Files"</h3>
                    <p style="font-size:0.85rem; color:var(--text-sub); margin:0;">
                        "Multi-file document studio with inline editors for Typst, LaTeX, slides, notes, tables, and scripts."
                    </p>
                </div>
                <div class="header-actions">
                    {view_toggle}
                    <apich_islands::ModalIsland trigger_label="+ New File".to_string() trigger_class="btn btn-primary".to_string() title="Create New File".to_string()>
                        <form method="post" action=format!("/projects/{}/files/new", project_id)>
                            <div class="form-group">
                                <label>"File Name (extension optional -- added from the type below if omitted)"</label>
                                <input type="text" name="filename" required=true placeholder="e.g. paper, slides.typ, analysis.py, script.R, main.rs" class="form-control" />
                            </div>
                            <div class="form-group">
                                <label>"Destination Folder"</label>
                                <select name="folder" class="form-control">{folder_options_for_new_file}</select>
                            </div>
                            <div class="form-group">
                                <label>"Initial Content / Starter Template"</label>
                                // Values here must match one of `ProjectManager::create_file`'s
                                // recognized `template` strings ("table"/"slide"/"typst"/"latex"/
                                // "note"/"script_python"/"script_r"/"script_rust", anything else falls back to a blank starter).
                                <select name="template" class="form-control">
                                    <option value="empty">"Empty file"</option>
                                    <option value="typst">"Typst Paper (template)"</option>
                                    <option value="slide">"Typst Presentation Slides"</option>
                                    <option value="latex">"LaTeX Article"</option>
                                    <option value="note">"Unified Note"</option>
                                    <option value="table">"SQLite Table"</option>
                                    <option value="script_python">"🐍 Python Script (.py)"</option>
                                    <option value="script_r">"📊 R Script (.R)"</option>
                                    <option value="script_rust">"🦀 Rust Script (.rs)"</option>
                                </select>
                            </div>
                            {new_file_template_picker}
                            <div style="display:flex; justify-content:flex-end; gap:0.75rem; margin-top:1.5rem;">
                                <button type="submit" class="btn btn-primary">"Create File"</button>
                            </div>
                        </form>
                    </apich_islands::ModalIsland>
                    <apich_islands::ModalIsland trigger_label="+ New Folder".to_string() trigger_class="btn btn-secondary".to_string() title="Create New Folder".to_string()>
                        <form method="post" action=format!("/projects/{}/folders/new", project_id)>
                            <div class="form-group">
                                <label>"Create Inside"</label>
                                <select name="parent" class="form-control">{folder_options_for_new_folder}</select>
                            </div>
                            <div class="form-group">
                                <label>"Folder Name"</label>
                                <input type="text" name="folder" required=true placeholder="e.g. assets, videos" class="form-control" />
                            </div>
                            <div style="display:flex; justify-content:flex-end; gap:0.75rem; margin-top:1.5rem;">
                                <button type="submit" class="btn btn-primary">"Create Folder"</button>
                            </div>
                        </form>
                    </apich_islands::ModalIsland>
                    <apich_islands::ModalIsland trigger_label="⬆ Upload File".to_string() trigger_class="btn btn-secondary".to_string() title="Upload File".to_string()>
                        <form method="post" action=format!("/projects/{}/files/upload", project_id) enctype="multipart/form-data">
                            <div class="form-group">
                                <label>"File"</label>
                                <input type="file" name="file" required=true class="form-control" />
                            </div>
                            <div class="form-group">
                                <label>"Destination Folder"</label>
                                <select name="folder" class="form-control">{folder_options_for_upload}</select>
                            </div>
                            <p style="font-size:0.78rem; color:var(--text-sub); margin:0.25rem 0 0;">
                                "Any file type -- video, audio, images, etc. Max 200MB per file."
                            </p>
                            <div style="display:flex; justify-content:flex-end; gap:0.75rem; margin-top:1.5rem;">
                                <button type="submit" class="btn btn-primary">"Upload"</button>
                            </div>
                        </form>
                    </apich_islands::ModalIsland>
                </div>
            </div>
            {breadcrumb_bar}
            <div class="file-table-wrap">
                <table class="file-table" id="project-files-table">
                    <thead>
                        <tr>
                            <th class="sortable-th" data-sort-col="name" title="Click to sort by Name">
                                "Name" <span class="sort-icon">" ↕"</span>
                            </th>
                            <th class="sortable-th" data-sort-col="type" title="Click to sort by Type">
                                "Type" <span class="sort-icon">" ↕"</span>
                            </th>
                            <th class="sortable-th" data-sort-col="sharing" title="Click to sort by Sharing">
                                "Sharing" <span class="sort-icon">" ↕"</span>
                            </th>
                            <th class="sortable-th" data-sort-col="size" title="Click to sort by Size">
                                "Size" <span class="sort-icon">" ↕"</span>
                            </th>
                            <th class="sortable-th" data-sort-col="modified" title="Click to sort by Date Modified">
                                "Modified" <span class="sort-icon">" ↕"</span>
                            </th>
                            <th style="text-align:right;">"Actions"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {rows}
                    </tbody>
                </table>
            </div>
        </div>

        <apich_islands::FileTableSortIsland table_id="project-files-table".to_string() is_zh=i18n.is_zh() />
        <apich_islands::FileOperationsModalIsland
            project_id=project_id.to_string()
            folders_json=folders_json.clone()
            existing_files_json=existing_files_json.clone()
            is_zh=i18n.is_zh()
        />
        <apich_islands::NewFileDuplicateCheckIsland
            form_selector="form[action$=\"/files/new\"]".to_string()
            existing_files_json=existing_files_json.clone()
            is_zh=i18n.is_zh()
        />

        <crate::app::components::FileShareModal
            project_id=project_id
            redirect_to=format!("/projects/{}?tab=files", project_id)
            all_users=all_users
            owner_id=project.owner_id
            i18n=i18n
        />
    }
}

struct VcsTabArgs<'a> {
    project: &'a Project,
    is_owner: bool,
    branches: Vec<String>,
    current_branch: Option<String>,
    conflicts: Vec<ConflictFileView>,
    snapshots: Vec<Snapshot>,
    signature_statuses: std::collections::HashMap<uuid::Uuid, apich_vcs::SignatureStatus>,
    milestones: Vec<Snapshot>,
    git: GitStatusView,
    ignore_config: apich_vcs::IgnoreConfig,
    gitignore_content: String,
    apichignore_content: String,
    i18n: I18n,
}

fn render_vcs_tab(args: VcsTabArgs<'_>) -> impl IntoView {
    let VcsTabArgs {
        project,
        is_owner,
        branches,
        current_branch,
        conflicts,
        snapshots,
        signature_statuses,
        milestones,
        git,
        ignore_config,
        gitignore_content,
        apichignore_content,
        i18n,
    } = args;
    let project_id = project.id;
    let curr = current_branch.unwrap_or_else(|| "main".to_string());

    let branch_options: Vec<_> = branches
        .iter()
        .filter(|b| b.as_str() != curr)
        .map(|b| view! { <option value=b.clone()>{b.clone()}</option> })
        .collect();

    let branch_list: Vec<_> = branches
        .iter()
        .map(|b| {
            let is_curr = b.as_str() == curr;
            if is_curr {
                view! {
                    <div class="passkey-item">
                        <div class="passkey-info"><span class="passkey-name">{b.clone()}</span></div>
                        <span class="badge badge-active">"Current"</span>
                    </div>
                }.into_any()
            } else {
                view! {
                    <div class="passkey-item">
                        <div class="passkey-info"><span class="passkey-name">{b.clone()}</span></div>
                        <form method="post" action=format!("/projects/{}/branch-switch", project_id) class="inline-form">
                            <input type="hidden" name="name" value=b.clone() />
                            <button type="submit" class="btn btn-secondary btn-sm">"Switch"</button>
                        </form>
                    </div>
                }.into_any()
            }
        })
        .collect();

    let merge_form = if branches.len() > 1 {
        view! {
            <form method="post" action=format!("/projects/{}/merge", project_id) class="form-row" style="align-items:flex-end;">
                <div class="form-group" style="margin-bottom:0; flex-grow:1;">
                    <label>{i18n.select_merge_target()}" (" {curr.clone()} ")"</label>
                    <select name="branch" class="form-control">{branch_options}</select>
                </div>
                <button type="submit" class="btn btn-primary">{i18n.run_merge()}</button>
            </form>
        }.into_any()
    } else {
        view! { <div style="font-size:0.875rem; color:var(--text-muted);">{i18n.single_branch_hint()}</div> }.into_any()
    };

    let conflicts_section = if conflicts.is_empty() {
        view! {
            <div class="alert alert-success" style="margin-top:1.25rem;">
                <strong>{i18n.conflicts_clean_title()}</strong>" " {i18n.conflicts_clean_desc()}
            </div>
        }
        .into_any()
    } else {
        let cards: Vec<_> = conflicts
            .iter()
            .map(|c| {
                let path = c.path.clone();
                view! {
                    <div class="conflict-card">
                        <div class="conflict-header">
                            <span class="conflict-file">{i18n.conflict_file_prefix()}" " {path.clone()}</span>
                            <div class="conflict-actions">
                                <form method="post" action=format!("/projects/{}/resolve-conflict", project_id) class="inline-form">
                                    <input type="hidden" name="file" value=path.clone() />
                                    <input type="hidden" name="choice" value="ours" />
                                    <button type="submit" class="btn btn-secondary btn-sm">{i18n.accept_ours()}</button>
                                </form>
                                <form method="post" action=format!("/projects/{}/resolve-conflict", project_id) class="inline-form">
                                    <input type="hidden" name="file" value=path />
                                    <input type="hidden" name="choice" value="theirs" />
                                    <button type="submit" class="btn btn-primary btn-sm">{i18n.accept_theirs()}</button>
                                </form>
                            </div>
                        </div>
                        <div class="diff-box">
                            <div style="margin-bottom:0.5rem;"><span class="diff-marker">"<<<<<<< " {i18n.ours_marker()}</span></div>
                            <pre class="diff-local">{c.ours_snippet.clone()}</pre>
                            <div style="margin:0.5rem 0;"><span class="diff-marker">"======="</span></div>
                            <pre class="diff-remote">{c.theirs_snippet.clone()}</pre>
                            <div style="margin-top:0.5rem;"><span class="diff-marker">">>>>>>> " {i18n.theirs_marker()}</span></div>
                        </div>
                    </div>
                }
            })
            .collect();
        view! {
            <div class="alert alert-warning" style="margin-top:1.25rem;">
                <strong>"⚠️ " {i18n.conflicts_warn_title()} " (" {conflicts.len()} "):"</strong>" " {i18n.conflicts_warn_desc()}
            </div>
            {cards}
        }.into_any()
    };

    let timeline = if snapshots.is_empty() {
        view! {
            <div class="empty-state">
                <h4 class="empty-title">"No snapshots yet"</h4>
                <p class="empty-desc">"Click 'Create Snapshot' above to record the first version."</p>
            </div>
        }.into_any()
    } else {
        let items: Vec<_> = snapshots
            .iter()
            .enumerate()
            .map(|(idx, snap)| {
                let id_str = snap.id.to_string();
                let short_id = id_str.chars().take(8).collect::<String>();
                let short_tree: String = snap.tree_hash.chars().take(8).collect();
                let time_str = snap.created_at.format("%Y-%m-%d %H:%M:%S UTC").to_string();
                let signature_badge = if project.vigilant_mode {
                    let (label, class) = if snap.gpg_signature.is_none() {
                        ("⚠ Unverified", "badge badge-idle")
                    } else {
                        match signature_statuses.get(&snap.id) {
                            Some(apich_vcs::SignatureStatus::Valid { .. }) => ("✓ Verified", "badge badge-active"),
                            _ => ("⚠ Unverified", "badge badge-idle"),
                        }
                    };
                    Some(view! { <span class=class style="font-size:0.7rem;">{label}</span> })
                } else {
                    None
                };
                view! {
                    <div class="timeline-item">
                        <div class="timeline-dot" class:active=idx == 0></div>
                        <div class="timeline-content">
                            <div class="timeline-header">
                                <span class="timeline-msg">{snap.message.clone()} " " {signature_badge}</span>
                                <span class="timeline-time">{time_str}</span>
                            </div>
                            <div class="timeline-meta">
                                <span>"Snapshot: "<code>{short_id}</code></span>
                                <span>"Tree: "<code>{short_tree}</code></span>
                            </div>
                        </div>
                    </div>
                }
            })
            .collect();
        view! { <div class="timeline-list">{items}</div> }.into_any()
    };

    let remote_desc = git
        .remote_url
        .clone()
        .unwrap_or_else(|| i18n.git_no_remote().to_string());

    view! {
        // The user's own complaint, verbatim: this page "mix[ed] git vcs and apich vcs
        // altogether" with no clear separation. There were always two genuinely different
        // systems here -- this project's own real, built-in version control (Branches, Timeline,
        // Milestones, all below) versus an *optional* bridge/export feature for people who also
        // want to sync with an external Git host -- but nothing on the page ever said so; the
        // last card just quietly sat there labeled "Git Compatibility" with no framing about how
        // it relates to everything above it. One short explainer up front, plus clearer labels on
        // the two "clone" links further down (which previously sat side by side with almost no
        // distinction beyond their button text), rather than restructuring what already is a
        // reasonable four-card layout.
        <div class="alert alert-info" style="margin-bottom:1.25rem; font-size:0.85rem;">
            "Branches, Timeline, and Milestones below are this project's own built-in history -- always on, nothing to set up. The " <strong>"Git"</strong> " section further down is optional: use it only if you also want to sync this project with GitHub, GitLab, or another Git host."
        </div>
        <div class="section-card">
            <h2 class="section-title">{i18n.merge_title()}</h2>
            <p class="text-muted" style="font-size:0.875rem; margin-bottom:1.25rem;">{i18n.merge_desc()}</p>

            <h3 class="card-subtitle">"Branches"</h3>
            <div class="passkey-list" style="margin-bottom:0.75rem;">{branch_list}</div>
            <form method="post" action=format!("/projects/{}/branch-create", project_id) class="form-row" style="align-items:flex-end; margin-bottom:1.5rem;">
                <div class="form-group" style="margin-bottom:0; flex-grow:1;">
                    <label>"New branch name"</label>
                    <input type="text" name="name" placeholder="feature-branch" required=true class="form-control" />
                </div>
                <button type="submit" class="btn btn-secondary">"Create Branch"</button>
            </form>

            <h3 class="card-subtitle">"Merge into "<code style="color:var(--primary); font-weight:600;">{curr.clone()}</code></h3>
            <div style="background:var(--bg-muted); border:1px solid var(--border-subtle); border-radius:12px; padding:1.25rem; margin-bottom:1.5rem;">
                {merge_form}
            </div>
            <h3 class="card-subtitle">{i18n.conflicts_warn_title()}</h3>
            {conflicts_section}
        </div>

        <div class="section-card">
            <div class="section-header">
                <h2 class="section-title">{i18n.tab_timeline()}</h2>
                <div style="display:flex; gap:0.5rem;">
                    <form method="post" action=format!("/projects/{}/vcs-undo", project_id) class="inline-form">
                        <button type="submit" class="btn btn-secondary btn-sm" title="Undo the last VCS operation">"↶ Undo"</button>
                    </form>
                    <form method="post" action=format!("/projects/{}/vcs-redo", project_id) class="inline-form">
                        <button type="submit" class="btn btn-secondary btn-sm" title="Redo the last undone operation">"↷ Redo"</button>
                    </form>
                    {is_owner.then(|| {
                        let action = format!("/projects/{}/vigilant-mode", project.id);
                        let (label, next_value) = if project.vigilant_mode {
                            ("🛡️ Vigilant mode: on", "false")
                        } else {
                            ("Vigilant mode: off", "true")
                        };
                        view! {
                            <form method="post" action=action class="inline-form">
                                <input type="hidden" name="enabled" value=next_value />
                                <button type="submit" class="btn btn-secondary btn-sm" title="When on, snapshots without a valid GPG signature are flagged as unverified.">{label}</button>
                            </form>
                        }
                    })}
                </div>
            </div>
            {timeline}
        </div>

        <div class="section-card">
            <h2 class="section-title">"Milestones"</h2>
            <p class="text-muted" style="font-size:0.8rem; margin-bottom:1rem;">"Named, easy-to-find points in the timeline (e.g. \"v1.0\", \"submitted-draft\")."</p>
            {if milestones.is_empty() {
                view! { <p class="text-muted" style="font-size:0.85rem;">"No milestones yet."</p> }.into_any()
            } else {
                let items: Vec<_> = milestones.iter().map(|m| {
                    let date = m.created_at.format("%Y-%m-%d").to_string();
                    view! {
                        <div class="passkey-item">
                            <div class="passkey-icon">"🏁"</div>
                            <div class="passkey-info">
                                <span class="passkey-name">{m.milestone_name.clone().unwrap_or_else(|| "unnamed".to_string())}</span>
                                <span class="passkey-meta">{format!("{} • {}", date, m.message)}</span>
                            </div>
                        </div>
                    }
                }).collect();
                view! { <div class="passkey-list" style="margin-bottom:1rem;">{items}</div> }.into_any()
            }}
            <form method="post" action=format!("/projects/{}/milestone-create", project_id) class="form-row" style="align-items:flex-end;">
                <div class="form-group" style="margin-bottom:0;">
                    <label>"Milestone name"</label>
                    <input type="text" name="name" placeholder="v1.0" required=true class="form-control" style="width:150px;" />
                </div>
                <div class="form-group" style="margin-bottom:0; flex-grow:1;">
                    <label>"Description"</label>
                    <input type="text" name="desc" placeholder="Optional description" class="form-control" />
                </div>
                <button type="submit" class="btn btn-secondary">"Mark HEAD"</button>
            </form>
        </div>

        <div class="section-card">
            <h2 class="section-title">{i18n.git_title()}</h2>
            <p class="text-muted" style="font-size:0.875rem; margin-bottom:1.5rem;">{i18n.git_desc()}</p>
            <div class="info-list" style="background:var(--bg-muted); border:1px solid var(--border-subtle); border-radius:12px; padding:1.25rem; margin-bottom:1.5rem;">
                <div class="info-row"><span class="info-label">{i18n.git_status_label()}": "</span><span class="info-val" style="color:var(--success); font-weight:600;">{if git.initialized { i18n.git_ready() } else { i18n.git_pending() }}</span></div>
                <div class="info-row"><span class="info-label">{i18n.git_remote_label()}": "</span><span class="info-val">{remote_desc}</span></div>
                <div class="info-row"><span class="info-label">{i18n.git_lfs_label()}": "</span><span class="info-val">{i18n.git_lfs_val()}</span></div>
            </div>
            <form method="post" action=format!("/projects/{}/git-sync", project_id)>
                <input type="hidden" name="message" value="APICH VCS sync to Git" />
                <button type="submit" class="btn btn-primary">{i18n.git_sync_btn()}</button>
            </form>

            <h3 class="card-subtitle" style="margin-top:1.5rem;">"Clone this project"</h3>
            <p class="text-muted" style="font-size:0.8rem; margin-bottom:0.75rem;">"Two different ways to get a copy, for two different purposes:"</p>
            <div style="background:var(--bg-muted); border:1px solid var(--border-subtle); border-radius:8px; padding:0.85rem; margin-bottom:0.75rem;">
                <div style="font-size:0.8rem; font-weight:600; margin-bottom:0.35rem;">"With a real Git client (git clone, GitHub Desktop, etc.)"</div>
                <p class="text-muted" style="font-size:0.8rem; margin-bottom:0.5rem;">
                    "Requires a " <a href="/settings#pat">"personal access token"</a> " as the password (username can be anything). Only sees whatever has been synced via \"" {i18n.git_sync_btn()} "\" above."
                </p>
                <apich_islands::CopyLinkIsland link=format!("/git/{}.git", project.slug) button_label="Copy Git URL".to_string() />
            </div>
            <div style="background:var(--bg-muted); border:1px solid var(--border-subtle); border-radius:8px; padding:0.85rem; margin-bottom:1.5rem;">
                <div style="font-size:0.8rem; font-weight:600; margin-bottom:0.35rem;">"With the apich command (full native history)"</div>
                <p class="text-muted" style="font-size:0.8rem; margin-bottom:0.5rem;">
                    "Gets this project's own real history directly -- every branch, snapshot, and milestone above, not just what's been synced to Git: " <code>"apich remote clone <url> --token <PAT>"</code>
                </p>
                <apich_islands::CopyLinkIsland link=format!("/vcs-remote/{}/bundle", project.id) button_label="Copy apich Command URL".to_string() />
            </div>

            <h3 class="card-subtitle">"Remotes"</h3>
            {if git.remotes.is_empty() {
                view! { <p class="text-muted" style="font-size:0.85rem;">"No remotes configured."</p> }.into_any()
            } else {
                let rows: Vec<_> = git.remotes.iter().map(|(name, url)| view! {
                    <div class="info-row"><span class="info-label">{name.clone()}": "</span><span class="info-val"><code>{url.clone()}</code></span></div>
                }).collect();
                view! { <div class="info-list" style="margin-bottom:1rem;">{rows}</div> }.into_any()
            }}
            <form method="post" action=format!("/projects/{}/git-remote-add", project_id) class="form-row" style="align-items:flex-end; margin-bottom:1.5rem;">
                <div class="form-group" style="margin-bottom:0;">
                    <label>"Remote name"</label>
                    <input type="text" name="name" value="origin" required=true class="form-control" style="width:120px;" />
                </div>
                <div class="form-group" style="margin-bottom:0; flex-grow:1;">
                    <label>"Remote URL:"</label>
                    <input type="text" name="url" placeholder="https://github.com/user/repo.git" required=true class="form-control" />
                </div>
                <button type="submit" class="btn btn-secondary">"Add Remote"</button>
            </form>

            <h3 class="card-subtitle">"Sync with remote"</h3>
            <div style="display:flex; gap:0.75rem; flex-wrap:wrap;">
                <form method="post" action=format!("/projects/{}/git-fetch", project_id) class="inline-form">
                    <input type="hidden" name="remote" value="origin" />
                    <button type="submit" class="btn btn-secondary btn-sm">"⬇ Fetch"</button>
                </form>
                <form method="post" action=format!("/projects/{}/git-pull", project_id) class="inline-form">
                    <input type="hidden" name="remote" value="origin" />
                    <input type="hidden" name="branch" value=curr.clone() />
                    <button type="submit" class="btn btn-secondary btn-sm">"⬇ Pull"</button>
                </form>
                <form method="post" action=format!("/projects/{}/git-push", project_id) class="inline-form">
                    <input type="hidden" name="remote" value="origin" />
                    <input type="hidden" name="branch" value=curr />
                    <button type="submit" class="btn btn-secondary btn-sm">"⬆ Push"</button>
                </form>
                <form method="post" action=format!("/projects/{}/git-rebase", project_id) class="inline-form" style="display:flex; gap:0.4rem; align-items:center;">
                    <input type="text" name="upstream" placeholder="origin/main" required=true class="form-control" style="width:150px; height:32px; font-size:0.8rem;" />
                    <button type="submit" class="btn btn-secondary btn-sm">"Rebase"</button>
                </form>
            </div>
        </div>

        {render_ignore_section(project_id, &ignore_config, &gitignore_content, &apichignore_content)}
    }
}

fn render_ignore_section(
    project_id: uuid::Uuid,
    ignore_config: &apich_vcs::IgnoreConfig,
    gitignore_content: &str,
    apichignore_content: &str,
) -> impl IntoView {
    let profile_rows: Vec<_> = apich_vcs::IgnoreProfile::all()
        .iter()
        .map(|profile| {
            let enabled = ignore_config.profile_enabled(*profile);
            let label = profile.label();
            let id = profile.id();
            let next_value = if enabled { "false" } else { "true" };
            let (badge_label, badge_class) = if enabled {
                ("On", "badge badge-active")
            } else {
                ("Off", "badge badge-idle")
            };
            view! {
                <div class="passkey-item">
                    <div class="passkey-info">
                        <span class="passkey-name">{label}</span>
                        <span class="passkey-meta">{format!("{id} smart-ignore patterns")}</span>
                    </div>
                    <span class=badge_class style="margin-right:0.6rem;">{badge_label}</span>
                    <form method="post" action=format!("/projects/{}/ignore/profile-toggle", project_id) class="inline-form">
                        <input type="hidden" name="profile" value=id />
                        <input type="hidden" name="enabled" value=next_value />
                        <button type="submit" class="btn btn-secondary btn-sm">{if enabled { "Disable" } else { "Enable" }}</button>
                    </form>
                </div>
            }
        })
        .collect();

    let custom_rule_rows: Vec<_> = ignore_config
        .custom_rules
        .iter()
        .map(|rule| {
            let rule_val = rule.clone();
            view! {
                <div class="passkey-item">
                    <div class="passkey-info"><span class="passkey-name"><code>{rule_val.clone()}</code></span></div>
                    <form method="post" action=format!("/projects/{}/ignore/rule-remove", project_id) class="inline-form">
                        <input type="hidden" name="rule" value=rule_val />
                        <button type="submit" class="btn btn-secondary btn-sm" title="Remove this rule">"✕"</button>
                    </form>
                </div>
            }
        })
        .collect();
    let custom_rules_empty = ignore_config.custom_rules.is_empty();

    view! {
        <div class="section-card">
            <h2 class="section-title">"Ignore Rules"</h2>
            <p class="text-muted" style="font-size:0.875rem; margin-bottom:1.25rem;">
                "Control what apich's \"smart ignore\" leaves out of snapshots and diffs. Toggle whole language/tool profiles, add your own patterns, or edit the raw "<code>".gitignore"</code>" / "<code>".apichignore"</code>" files directly -- all three layers combine."
            </p>

            <h3 class="card-subtitle">"Smart profiles"</h3>
            <div class="passkey-list" style="margin-bottom:1.5rem;">{profile_rows}</div>

            <h3 class="card-subtitle">"Custom rules"</h3>
            <p class="text-muted" style="font-size:0.8rem; margin-bottom:0.5rem;">"One glob pattern per rule, e.g. "<code>"*.tmp"</code>". Prefix with "<code>"!"</code>" to whitelist (un-ignore) a path that a profile or file would otherwise exclude."</p>
            {(!custom_rules_empty).then(|| view! { <div class="passkey-list" style="margin-bottom:0.75rem;">{custom_rule_rows}</div> })}
            <form method="post" action=format!("/projects/{}/ignore/rule-add", project_id) class="form-row" style="align-items:flex-end; margin-bottom:1.5rem;">
                <div class="form-group" style="margin-bottom:0; flex-grow:1;">
                    <label>"New rule"</label>
                    <input type="text" name="rule" placeholder="*.tmp or !keep-me.csv" required=true class="form-control" />
                </div>
                <button type="submit" class="btn btn-secondary">"Add Rule"</button>
            </form>

            <h3 class="card-subtitle">".gitignore"</h3>
            <form method="post" action=format!("/projects/{}/ignore/file-save", project_id) style="margin-bottom:1.5rem;">
                <input type="hidden" name="file_name" value=".gitignore" />
                <textarea name="content" rows="6" class="form-control" style="font-family:monospace; font-size:0.8rem;" placeholder="# one pattern per line, standard gitignore syntax">{gitignore_content.to_string()}</textarea>
                <button type="submit" class="btn btn-secondary" style="margin-top:0.5rem;">"Save .gitignore"</button>
            </form>

            <h3 class="card-subtitle">".apichignore"</h3>
            <p class="text-muted" style="font-size:0.8rem; margin-bottom:0.5rem;">"Same syntax as .gitignore, but only apich's own version control honors it (a real Git export/sync still follows .gitignore alone)."</p>
            <form method="post" action=format!("/projects/{}/ignore/file-save", project_id)>
                <input type="hidden" name="file_name" value=".apichignore" />
                <textarea name="content" rows="6" class="form-control" style="font-family:monospace; font-size:0.8rem;" placeholder="# one pattern per line">{apichignore_content.to_string()}</textarea>
                <button type="submit" class="btn btn-secondary" style="margin-top:0.5rem;">"Save .apichignore"</button>
            </form>
        </div>
    }
}

fn render_sharing_tab(
    project: &Project,
    members: Vec<ProjectMemberWithUser>,
    all_users: Vec<User>,
    is_owner: bool,
    i18n: I18n,
) -> impl IntoView {
    let project_id = project.id;

    let share_mode = project
        .settings
        .get("share_mode")
        .and_then(|v| v.as_str())
        .unwrap_or("private")
        .to_string();
    let share_role = project
        .settings
        .get("share_role")
        .and_then(|v| v.as_str())
        .unwrap_or("read_only")
        .to_string();
    let is_public = share_mode == "public";
    let share_link = format!("/shared/{project_id}");

    let existing_member_ids: std::collections::HashSet<_> =
        members.iter().map(|m| m.user_id).collect();

    let member_rows: Vec<_> = members
        .into_iter()
        .map(|m| {
            let role_badge = match m.role.as_str() {
                "owner" => view! { <span class="role-badge role-badge-admin">"Owner"</span> }.into_any(),
                "read_write_and_review" | "editor" => view! { <span class="role-badge role-badge-editor">{i18n.role_read_write_and_review()}</span> }.into_any(),
                "read_and_review" => view! { <span class="role-badge" style="background:#e0e7ff; color:#3730a3; border:1px solid #c7d2fe;">{i18n.role_read_and_review()}</span> }.into_any(),
                _ => view! { <span class="role-badge role-badge-viewer">{i18n.role_read_only()}</span> }.into_any(),
            };
            let remove = (is_owner && m.role != "owner").then(|| {
                view! {
                    <form method="post" action=format!("/projects/{}/members/remove", project_id) class="inline-form">
                        <input type="hidden" name="user_id" value=m.user_id.to_string() />
                        <apich_islands::ConfirmSubmitButton
                            label=i18n.remove().to_string()
                            message="Remove collaborator?".to_string()
                            button_class="btn btn-danger btn-sm".to_string()
                            button_style=String::new()
                        />
                    </form>
                }
            });
            view! {
                <div class="collaborator-item" style="justify-content:space-between;">
                    <div style="display:flex; align-items:center; gap:0.75rem;">
                        <div class="collab-avatar">{m.display_name.chars().take(2).collect::<String>()}</div>
                        <div class="collab-info">
                            <span class="collab-name">{m.display_name}" "<small style="color:var(--text-sub);">"(@"{m.username}")"</small></span>
                            <span style="font-size:0.75rem; color:var(--text-sub);">{m.email}</span>
                        </div>
                    </div>
                    <div class="row-actions">{role_badge}{remove}</div>
                </div>
            }
        })
        .collect();

    let public_link_card = is_owner.then(|| {
        view! {
            <div class="section-card" style="margin-bottom:1.5rem;">
                <h3 class="card-subtitle" style="display:flex; align-items:center; gap:0.5rem;"><span>"🔗"</span> {i18n.public_link_sharing()}</h3>
                <p style="font-size:0.85rem; color:var(--text-sub); margin:0.25rem 0 0.75rem;">
                    "Anyone with this link can view the project at the selected permission level, without signing in."
                </p>
                // The button's column used to be `1fr`, which stretched it across the whole
                // remaining width of the card -- a Save button several times wider than the
                // controls it applies to. Sized to its own content instead, with the `1fr`
                // moved to a trailing spacer column so the row still fills the card.
                <form method="post" action=format!("/projects/{}/sharing/update", project_id) style="display:grid; grid-template-columns: auto minmax(240px, auto) auto; gap:0.75rem; align-items:center; justify-content:start; max-width:720px;">
                    <label style="display:flex; align-items:center; gap:0.5rem; font-size:0.85rem; font-weight:600;">
                        <input type="checkbox" name="is_public" value="true" checked=is_public />
                        "Enabled"
                    </label>
                    <select name="default_role" class="form-control" style="min-width:240px;">
                        <option value="read_only" selected=share_role == "read_only">{i18n.role_read_only()}</option>
                        <option value="read_and_review" selected=share_role == "read_and_review">{i18n.role_read_and_review()}</option>
                        <option value="read_write_and_review" selected=share_role == "read_write_and_review">{i18n.role_read_write_and_review()}</option>
                    </select>
                    <button type="submit" class="btn btn-primary btn-sm">{i18n.save()}</button>
                </form>
                <div style="margin-top:0.85rem; display:flex; gap:0.5rem; align-items:center;">
                    <apich_islands::CopyLinkIsland link=share_link.clone() button_label=i18n.copy_link().to_string() />
                </div>
                {(!is_public).then(|| view! {
                    <p style="font-size:0.75rem; color:var(--text-sub); margin-top:0.4rem;">"Currently private — check \"Enabled\" above to activate this link."</p>
                })}
            </div>
        }
    });

    let invite_form = is_owner.then(|| {
        let exclude_user_ids: Vec<String> = existing_member_ids
            .into_iter()
            .map(|id| id.to_string())
            .collect();
        view! {
            <div class="section-card" style="margin-top:1.5rem;">
                <h3 class="card-subtitle">{i18n.add_collaborator()}</h3>
                <form method="post" action=format!("/projects/{}/sharing/update", project_id) style="display:grid; grid-template-columns: minmax(0,1.2fr) minmax(0,1fr) auto; gap:1rem; align-items:flex-end; max-width:760px;">
                    <div class="form-group" style="margin-bottom:0;">
                        <label>{i18n.select_user()}</label>
                        <apich_islands::UserSelectIsland
                            name="invite_user_id".to_string()
                            placeholder=i18n.select_user().to_string()
                            exclude_user_ids=exclude_user_ids
                        />
                    </div>
                    <div class="form-group" style="margin-bottom:0;">
                        <label>{i18n.role()}</label>
                        <select name="invite_role" class="form-control">
                            <option value="read_write_and_review">{i18n.role_read_write_and_review()}</option>
                            <option value="read_and_review">{i18n.role_read_and_review()}</option>
                            <option value="read_only">{i18n.role_read_only()}</option>
                        </select>
                    </div>
                    <button type="submit" class="btn btn-primary">{i18n.add_collaborator()}</button>
                </form>
            </div>
        }
    });

    // A ghost button buried in the page header was too easy to miss ("I cannot find where to
    // delete a project" -- the button already existed, just not where anyone would look for it).
    // A labeled, visually distinct section at the bottom of the one tab that's already about
    // project-level actions (not per-file) is where a user looking for it would actually check.
    let is_single_file = project
        .settings
        .get("is_single_file")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let single_file_name = project
        .settings
        .get("single_file_name")
        .and_then(|v| v.as_str())
        .unwrap_or(&project.name);

    let danger_zone = is_owner.then(|| {
        let (title, desc, btn_label, confirm_msg) = if is_single_file {
            (
                "Delete this file",
                "Permanently removes this standalone file and its version history from your workspace. This cannot be undone.",
                "Delete File",
                format!("Delete file \"{}\"? This cannot be undone.", single_file_name),
            )
        } else {
            (
                "Delete this project",
                "Removes this project from your workspace. This cannot be undone.",
                "Delete Project",
                format!("Delete \"{}\"? This cannot be undone.", project.name),
            )
        };
        view! {
            <div class="section-card" style="margin-top:1.5rem; border-color:var(--danger-border); background:var(--danger-bg);">
                <h3 class="card-subtitle" style="color:var(--danger);">{title}</h3>
                <p style="font-size:0.85rem; color:var(--text-sub); margin:0.25rem 0 0.75rem;">
                    {desc}
                </p>
                <form method="post" action=format!("/projects/{}/delete", project_id) class="inline-form">
                    <apich_islands::ConfirmSubmitButton
                        label=btn_label.to_string()
                        message=confirm_msg
                        button_class="btn btn-danger btn-sm".to_string()
                        button_style=String::new()
                    />
                </form>
            </div>
        }
    });

    view! {
        {public_link_card}
        <div class="section-card">
            <h2 class="section-title">{i18n.tab_members()}</h2>
            <div class="collaborator-list" style="margin-top:1rem;">{member_rows}</div>
        </div>
        {invite_form}
        {danger_zone}
    }
}
