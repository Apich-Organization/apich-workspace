#![allow(clippy::trivially_copy_pass_by_ref)]

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    #[default]
    En,
    Zh,
}

impl Lang {
    pub fn parse(s: &str) -> Self {
        let clean = s.trim().to_lowercase();
        if clean.starts_with("zh") {
            Self::Zh
        } else {
            Self::En
        }
    }

    pub const fn code(&self) -> &'static str {
        match self {
            | Self::En => "en",
            | Self::Zh => "zh",
        }
    }

    pub const fn toggle_code(&self) -> &'static str {
        match self {
            | Self::En => "zh",
            | Self::Zh => "en",
        }
    }

    pub const fn toggle_label(&self) -> &'static str {
        match self {
            | Self::En => "中文",
            | Self::Zh => "English",
        }
    }
}

/// Resolve language from query parameters, Cookie header, and Accept-Language header
pub fn resolve_language(
    query_params: Option<&HashMap<String, String>>,
    cookie_header: Option<&str>,
    accept_lang: Option<&str>,
) -> Lang {
    // 1. Explicit query parameter ?lang=...
    if let Some(params) = query_params {
        if let Some(l) = params.get("lang") {
            return Lang::parse(l);
        }
    }

    // 2. Cookie header apich_lang=...
    if let Some(cookies) = cookie_header {
        for part in cookies.split(';') {
            let part = part.trim();
            if let Some(val) = part.strip_prefix("apich_lang=") {
                return Lang::parse(val);
            }
        }
    }

    // 3. Fallback: Accept-Language header
    if let Some(al) = accept_lang {
        if al.contains("zh") {
            return Lang::Zh;
        }
    }

    // Default: English (Primary language as requested)
    Lang::En
}

#[derive(Debug, Clone, Copy)]
pub struct I18n {
    pub lang: Lang,
}

impl I18n {
    pub const fn new(lang: Lang) -> Self {
        Self { lang }
    }

    pub fn is_zh(&self) -> bool {
        self.lang == Lang::Zh
    }

    // --- Navigation & Header ---
    pub const fn brand_title(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Research Workspace",
            | Lang::Zh => "科研工作区",
        }
    }

    pub const fn nav_projects(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Projects",
            | Lang::Zh => "项目列表",
        }
    }

    pub const fn nav_templates(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Templates",
            | Lang::Zh => "模板库",
        }
    }

    // --- Template Library ---

    pub const fn template_gallery_title(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Template Library",
            | Lang::Zh => "模板库",
        }
    }

    pub fn template_kind_label(
        &self,
        kind: &str,
    ) -> &'static str {
        match (self.lang, kind) {
            | (Lang::En, "note") => "Note",
            | (Lang::Zh, "note") => "笔记",
            | (_, "latex") => "LaTeX",
            | (_, "typst") => "Typst",
            | (Lang::En, "slides") => "Slides",
            | (Lang::Zh, "slides") => "幻灯片",
            | (Lang::En, "kanban") => "Kanban",
            | (Lang::Zh, "kanban") => "看板",
            | (Lang::En, _) => "All",
            | (Lang::Zh, _) => "全部",
        }
    }

    pub const fn template_owner(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Owner",
            | Lang::Zh => "所有者",
        }
    }

    pub fn template_visibility_label(
        &self,
        visibility: &str,
    ) -> &'static str {
        match (self.lang, visibility) {
            | (Lang::En, "public") => "Public",
            | (Lang::Zh, "public") => "公开",
            | (Lang::En, "shared") => "Shared",
            | (Lang::Zh, "shared") => "共享",
            | (Lang::En, _) => "Private",
            | (Lang::Zh, _) => "私有",
        }
    }

    pub const fn template_latest_version(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Latest version",
            | Lang::Zh => "最新版本",
        }
    }

    pub const fn template_view(&self) -> &'static str {
        match self.lang {
            | Lang::En => "View",
            | Lang::Zh => "查看",
        }
    }

    pub const fn template_no_templates(&self) -> &'static str {
        match self.lang {
            | Lang::En => "No templates here yet.",
            | Lang::Zh => "此处暂无模板。",
        }
    }

    pub const fn template_versions_heading(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Versions",
            | Lang::Zh => "版本记录",
        }
    }

    pub const fn template_version_label_field(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Version label (e.g. 1.0.0)",
            | Lang::Zh => "版本号（如 1.0.0）",
        }
    }

    pub const fn template_changelog_field(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Changelog (optional)",
            | Lang::Zh => "更新说明（可选）",
        }
    }

    pub const fn template_publish(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Publish",
            | Lang::Zh => "发布",
        }
    }

    pub const fn template_sharing_heading(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Sharing",
            | Lang::Zh => "共享管理",
        }
    }

    /// Placeholder for the share-target `<select>` (org/team picker) -- distinct from a real
    /// option so `required=true` + this staying `disabled` forces an actual choice before submit.
    pub const fn template_share_target_placeholder(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Select an org or team to share with...",
            | Lang::Zh => "选择要共享的组织或团队…",
        }
    }

    pub const fn template_no_share_targets(&self) -> &'static str {
        match self.lang {
            | Lang::En => "No orgs or teams exist on this instance yet to share with.",
            | Lang::Zh => "当前实例中还没有可共享的组织或团队。",
        }
    }

    pub const fn template_add_share(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Add share",
            | Lang::Zh => "添加共享",
        }
    }

    pub const fn template_remove(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Remove",
            | Lang::Zh => "移除",
        }
    }

    pub const fn template_change_visibility(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Change visibility",
            | Lang::Zh => "更改可见范围",
        }
    }

    pub const fn template_delete(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Delete template",
            | Lang::Zh => "删除模板",
        }
    }

    pub const fn template_description_field(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Description (optional)",
            | Lang::Zh => "描述（可选）",
        }
    }

    pub const fn template_create_new(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Create new template",
            | Lang::Zh => "新建模板",
        }
    }

    pub const fn template_name_field(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Name",
            | Lang::Zh => "名称",
        }
    }

    pub const fn template_slug_field(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Slug (short id, e.g. sprint-board)",
            | Lang::Zh => "标识 slug（如 sprint-board）",
        }
    }

    pub const fn template_publish_new_version_to(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Publish new version to an existing template of mine:",
            | Lang::Zh => "发布新版本到我已有的模板：",
        }
    }

    pub const fn template_apply(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Apply",
            | Lang::Zh => "应用",
        }
    }

    pub const fn template_apply_to_project(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Apply to project",
            | Lang::Zh => "应用到项目",
        }
    }

    pub const fn template_select_project(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Select a project...",
            | Lang::Zh => "选择项目…",
        }
    }

    pub const fn template_select_version(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Select a template...",
            | Lang::Zh => "选择模板…",
        }
    }

    pub const fn template_publish_as_template(&self) -> &'static str {
        match self.lang {
            | Lang::En => "📚 Publish as Template",
            | Lang::Zh => "📚 发布为模板",
        }
    }

    pub const fn template_apply_template(&self) -> &'static str {
        match self.lang {
            | Lang::En => "📚 Apply a Template",
            | Lang::Zh => "📚 应用模板",
        }
    }

    pub const fn template_preview_heading(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Preview",
            | Lang::Zh => "预览",
        }
    }

    pub const fn template_source_only_preview_note(&self) -> &'static str {
        match self.lang {
            Lang::En => "Compiled PDF preview isn't available here for LaTeX (it needs a real project sandbox to run pdflatex in) -- showing source. Applying it still writes the real file into your project, where the project's own preview compiles it normally.",
            Lang::Zh => "此处暂不支持 LaTeX 的编译 PDF 预览（需要在真实的项目沙箱中运行 pdflatex）——以下为源码显示。应用后会将真实文件写入你的项目，届时可通过项目自身的预览正常编译查看。",
        }
    }

    pub const fn template_new_file_name_field(&self) -> &'static str {
        match self.lang {
            | Lang::En => "New file name",
            | Lang::Zh => "新文件名",
        }
    }

    pub const fn template_published_by(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Published by",
            | Lang::Zh => "发布者",
        }
    }

    pub const fn template_dest_folder_field(&self) -> &'static str {
        match self.lang {
            | Lang::En => "destination folder (optional)",
            | Lang::Zh => "目标文件夹（可选）",
        }
    }

    pub const fn template_no_projects_to_apply(&self) -> &'static str {
        match self.lang {
            | Lang::En => "You don't have any projects to apply this to yet.",
            | Lang::Zh => "你还没有可应用此模板的项目。",
        }
    }

    pub const fn template_no_versions(&self) -> &'static str {
        match self.lang {
            | Lang::En => "No versions published yet.",
            | Lang::Zh => "尚未发布任何版本。",
        }
    }

    pub const fn nav_teams_orgs(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Teams & Orgs",
            | Lang::Zh => "团队与组织",
        }
    }

    pub const fn nav_admin(&self) -> &'static str {
        match self.lang {
            | Lang::En => "System Admin",
            | Lang::Zh => "系统管理",
        }
    }

    pub const fn nav_settings(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Settings",
            | Lang::Zh => "个人设置",
        }
    }

    pub const fn welcome_prefix(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Welcome,",
            | Lang::Zh => "欢迎，",
        }
    }

    pub const fn logout(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Sign Out",
            | Lang::Zh => "退出登录",
        }
    }

    // --- Dashboard ---
    pub const fn dashboard_title(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Projects & Sandboxes",
            | Lang::Zh => "科研项目与计算沙箱",
        }
    }

    pub const fn dashboard_subtitle(&self) -> &'static str {
        match self.lang {
            // Deliberately says nothing about isolated/cloud environments -- this is a docs and
            // collaboration tool, not a cloud dev platform, and per direct user feedback the
            // wording shouldn't read like one (and per plan.md, the per-project sandbox this app
            // runs commands in is meant to be a silent implementation detail, never something the
            // page advertises).
            | Lang::En => "Your projects, with full version history and real-time collaboration",
            | Lang::Zh => "您的项目，具备完整版本历史与实时协作",
        }
    }

    pub const fn new_project(&self) -> &'static str {
        match self.lang {
            | Lang::En => "New Project",
            | Lang::Zh => "新建项目",
        }
    }

    pub const fn active_projects_section(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Active Projects",
            | Lang::Zh => "活跃项目工作区",
        }
    }

    pub const fn active_projects_hint(&self) -> &'static str {
        match self.lang {
            Lang::En => "Click a project card to inspect code, resolve merge conflicts, and manage sandboxes",
            Lang::Zh => "点击卡片直接进入项目工作区执行 Merge 冲突合并、快照及沙箱管理",
        }
    }

    pub const fn enter_workspace(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Enter Project",
            | Lang::Zh => "进入工作区",
        }
    }

    pub const fn launch_sandbox(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Start Sandbox",
            | Lang::Zh => "启动沙箱",
        }
    }

    pub const fn stop_sandbox(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Pause",
            | Lang::Zh => "暂停",
        }
    }

    // Per direct user feedback, nothing on the page should say "sandbox" or "container" --
    // the underlying per-project environment is meant to be a silent implementation detail,
    // not something the UI names. `sandbox_running`/`sandbox_stopped` are the Rust method
    // names (internal, not user-visible); only the returned strings changed here.
    pub const fn sandbox_running(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Ready",
            | Lang::Zh => "已就绪",
        }
    }

    pub const fn sandbox_stopped(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Paused",
            | Lang::Zh => "已暂停",
        }
    }

    pub const fn no_projects(&self) -> &'static str {
        match self.lang {
            | Lang::En => "No projects found",
            | Lang::Zh => "暂无项目",
        }
    }

    pub const fn no_projects_desc(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Click 'New Project' above to create your first versioned workspace.",
            | Lang::Zh => "点击上方“新建项目”以开启版本控制与隔离沙箱计算环境。",
        }
    }

    // --- Project Detail Tabs ---
    pub const fn tab_overview(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Overview & Sandbox",
            | Lang::Zh => "概览与沙箱",
        }
    }

    pub const fn tab_merge(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Branches & Merge Conflicts",
            | Lang::Zh => "分支与 Merge 冲突解决",
        }
    }

    pub const fn tab_timeline(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Timeline Snapshots",
            | Lang::Zh => "时间线快照",
        }
    }

    pub const fn tab_git(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Git Bridge",
            | Lang::Zh => "Git 兼容同步",
        }
    }

    pub const fn tab_members(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Collaborators",
            | Lang::Zh => "协作者成员",
        }
    }

    // --- Merge & Conflicts ---
    pub const fn merge_title(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Branches & Merging",
            | Lang::Zh => "分支与合并",
        }
    }

    pub const fn merge_desc(&self) -> &'static str {
        match self.lang {
            Lang::En => "Merge one branch into another. Each merge produces a single, consistent snapshot of your files.",
            Lang::Zh => "将一个分支合并到另一个分支。每次合并都会生成一份完整且一致的文件快照。",
        }
    }

    pub const fn current_branch(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Current Branch:",
            | Lang::Zh => "当前所在分支:",
        }
    }

    pub const fn available_branches(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Available branches:",
            | Lang::Zh => "可用分支:",
        }
    }

    pub const fn select_merge_target(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Select target branch to merge into current:",
            | Lang::Zh => "选择要合并入当前分支的目标分支：",
        }
    }

    pub const fn run_merge(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Merge Branches",
            | Lang::Zh => "合并分支",
        }
    }

    pub const fn single_branch_hint(&self) -> &'static str {
        match self.lang {
            Lang::En => "This project currently has only the main branch. Create another branch from the terminal to merge branches here.",
            Lang::Zh => "当前项目仅有 main 一个分支。请先在终端中创建其他分支，才能在此进行合并。",
        }
    }

    pub const fn conflicts_clean_title(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Clean Working Tree:",
            | Lang::Zh => "状态良好：",
        }
    }

    pub const fn conflicts_clean_desc(&self) -> &'static str {
        match self.lang {
            | Lang::En => "No merge conflicts detected. All files are fully reconciled.",
            | Lang::Zh => "工作区当前无合并冲突。所有文件均已完全对齐。",
        }
    }

    pub const fn conflicts_warn_title(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Unresolved Code/Document Conflicts:",
            | Lang::Zh => "存在未解决的代码/文档合并冲突：",
        }
    }

    pub const fn conflicts_warn_desc(&self) -> &'static str {
        match self.lang {
            Lang::En => "Select a resolution strategy below (keep local, accept incoming, or edit custom content).",
            Lang::Zh => "请选择以下文件的解决策略（保留本地版本、采纳传入版本或在线编辑保存）。",
        }
    }

    pub const fn conflict_file_prefix(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Conflict in:",
            | Lang::Zh => "冲突文件：",
        }
    }

    pub const fn accept_ours(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Keep Local (Ours)",
            | Lang::Zh => "采纳本地版本（Ours）",
        }
    }

    pub const fn accept_theirs(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Accept Incoming (Theirs)",
            | Lang::Zh => "采纳传入版本（Theirs）",
        }
    }

    pub const fn ours_marker(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Local Changes (Ours)",
            | Lang::Zh => "本地修改 (Ours)",
        }
    }

    pub const fn theirs_marker(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Incoming Changes (Theirs)",
            | Lang::Zh => "传入修改 (Theirs)",
        }
    }

    // --- Git Tab ---
    pub const fn git_title(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Git Compatibility",
            | Lang::Zh => "Git 兼容性",
        }
    }

    pub const fn git_desc(&self) -> &'static str {
        match self.lang {
            Lang::En => "Every project can also be used as a standard Git repository, so it works with GitHub, GitLab, or your own Git remote.",
            Lang::Zh => "每个项目也可作为标准 Git 仓库使用，可与 GitHub、GitLab 或您自己的 Git 远程仓库配合使用。",
        }
    }

    pub const fn git_status_label(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Git Repository Status",
            | Lang::Zh => "Git 仓库初始化状态",
        }
    }

    pub const fn git_ready(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Ready (.git initialized)",
            | Lang::Zh => "已就绪 (.git 存在)",
        }
    }

    pub const fn git_pending(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Pending Sync",
            | Lang::Zh => "自动同步待触发",
        }
    }

    pub const fn git_remote_label(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Remote URL",
            | Lang::Zh => "远程 Remote URL",
        }
    }

    pub const fn git_no_remote(&self) -> &'static str {
        match self.lang {
            | Lang::En => "No remote configured",
            | Lang::Zh => "未绑定远程仓库",
        }
    }

    pub const fn git_lfs_label(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Large File LFS Policy",
            | Lang::Zh => "大文件 LFS 策略",
        }
    }

    pub const fn git_lfs_val(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Large files are stored efficiently and automatically",
            | Lang::Zh => "大文件自动高效存储",
        }
    }

    pub const fn git_sync_btn(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Sync to Git",
            | Lang::Zh => "同步至 Git",
        }
    }

    // --- Snapshots & Timeline ---
    pub const fn create_snapshot(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Create Snapshot",
            | Lang::Zh => "创建快照",
        }
    }

    pub const fn modal_snapshot_title(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Record VCS Snapshot",
            | Lang::Zh => "记录版本快照",
        }
    }

    pub const fn modal_snapshot_desc(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Save the current state of your files as a point you can return to later",
            | Lang::Zh => "将当前文件状态保存为一个可以随时回退的版本",
        }
    }

    pub const fn snapshot_message_label(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Snapshot Message",
            | Lang::Zh => "快照说明",
        }
    }

    pub const fn snapshot_submit(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Save Snapshot Now",
            | Lang::Zh => "立即保存快照",
        }
    }

    // --- Common Form & Action ---
    pub const fn cancel(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Cancel",
            | Lang::Zh => "取消",
        }
    }

    pub const fn save(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Save",
            | Lang::Zh => "保存",
        }
    }

    pub const fn upload(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Upload",
            | Lang::Zh => "上传",
        }
    }

    pub const fn attach(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Attach",
            | Lang::Zh => "附件",
        }
    }

    pub const fn delete(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Delete",
            | Lang::Zh => "删除",
        }
    }

    pub const fn remove(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Remove",
            | Lang::Zh => "移除",
        }
    }

    pub const fn back(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Back",
            | Lang::Zh => "返回",
        }
    }

    pub const fn actions(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Actions",
            | Lang::Zh => "操作",
        }
    }

    pub const fn role(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Role",
            | Lang::Zh => "角色",
        }
    }

    // --- Collaborators Management ---
    pub const fn add_collaborator(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Add Collaborator",
            | Lang::Zh => "添加协作者",
        }
    }

    pub const fn select_user(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Select User",
            | Lang::Zh => "选择用户",
        }
    }

    // --- Organizations & Teams Admin ---
    pub const fn org_admin_title(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Organizations & Teams",
            | Lang::Zh => "组织与团队层级树",
        }
    }

    pub const fn org_admin_subtitle(&self) -> &'static str {
        match self.lang {
            | Lang::En => {
                "Manage institutional hierarchy, laboratory teams, and member access roles"
            },
            | Lang::Zh => "构建机构与实验室团队层级，统一权限管控",
        }
    }

    pub const fn new_organization(&self) -> &'static str {
        match self.lang {
            | Lang::En => "New Organization",
            | Lang::Zh => "新建机构",
        }
    }

    pub const fn teams_and_groups(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Teams & Groups",
            | Lang::Zh => "团队与小组",
        }
    }

    pub const fn no_subteams(&self) -> &'static str {
        match self.lang {
            | Lang::En => "No teams yet",
            | Lang::Zh => "暂无团队",
        }
    }

    pub const fn org_members_heading(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Organization Members",
            | Lang::Zh => "机构成员",
        }
    }

    pub const fn create_team(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Create Team",
            | Lang::Zh => "创建新团队",
        }
    }

    pub const fn team_name(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Team Name",
            | Lang::Zh => "团队名称",
        }
    }

    pub const fn team_slug(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Team Slug",
            | Lang::Zh => "团队英文标识",
        }
    }

    pub const fn parent_team(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Parent Team",
            | Lang::Zh => "上级所属团队",
        }
    }

    pub const fn no_parent_root(&self) -> &'static str {
        match self.lang {
            | Lang::En => "None (Root Team)",
            | Lang::Zh => "无（作为顶层团队）",
        }
    }

    pub const fn description(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Description",
            | Lang::Zh => "简要描述",
        }
    }

    pub const fn add_member(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Add Member",
            | Lang::Zh => "添加成员",
        }
    }

    // --- Settings & Profile ---
    pub const fn settings_title(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Account & Security Settings",
            | Lang::Zh => "个人账号与安全设置",
        }
    }

    pub const fn settings_subtitle(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Manage your profile, hardware passkeys, and multi-factor credentials",
            | Lang::Zh => "管理个人基本信息、硬件 Passkeys 以及多因子安全凭据",
        }
    }

    pub const fn profile_card(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Personal Profile",
            | Lang::Zh => "个人资料",
        }
    }

    pub const fn username(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Username",
            | Lang::Zh => "用户名",
        }
    }

    pub const fn display_name(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Display Name",
            | Lang::Zh => "显示名称",
        }
    }

    pub const fn email(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Email Address",
            | Lang::Zh => "邮箱地址",
        }
    }

    pub const fn system_role(&self) -> &'static str {
        match self.lang {
            | Lang::En => "System Role",
            | Lang::Zh => "系统角色",
        }
    }

    pub const fn platform_admin(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Platform Administrator",
            | Lang::Zh => "平台超级管理员",
        }
    }

    pub const fn researcher(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Researcher",
            | Lang::Zh => "科研人员",
        }
    }

    pub const fn passkeys_title(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Passkeys & Hardware Keys",
            | Lang::Zh => "Passkey 与硬件密钥",
        }
    }

    pub const fn passkeys_desc(&self) -> &'static str {
        match self.lang {
            Lang::En => "FIDO2 / WebAuthn Level 3 phishing-resistant cryptographic authentication.",
            Lang::Zh => "基于 WebAuthn / FIDO2 Level 3 安全标准，提供免疫钓鱼攻击的生物特征或硬件密钥登录能力。",
        }
    }

    pub const fn register_passkey_btn(&self) -> &'static str {
        match self.lang {
            | Lang::En => "+ Register New Passkey",
            | Lang::Zh => "+ 注册新 Passkey 硬件密钥",
        }
    }

    // --- Auth (Login & Register) ---
    pub const fn sign_in_title(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Sign In to Workspace",
            | Lang::Zh => "登录科研工作区",
        }
    }

    pub const fn sign_in_subtitle(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Projects, version history, and collaboration in one place",
            | Lang::Zh => "项目、版本历史与协作，一站式管理",
        }
    }

    pub const fn passkey_login_btn(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Sign In with Passkey / Hardware Key",
            | Lang::Zh => "使用 Passkey / 指纹面容硬件密钥快捷登录",
        }
    }

    pub const fn passkey_hint(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Supports YubiKey, Touch ID, Windows Hello, and FIDO2 keys",
            | Lang::Zh => "支持 YubiKey、Touch ID、Windows Hello 及 FIDO2 标准认证",
        }
    }

    pub const fn divider_or(&self) -> &'static str {
        match self.lang {
            | Lang::En => "OR SIGN IN WITH PASSWORD",
            | Lang::Zh => "或使用账号密码登录",
        }
    }

    pub const fn login_label(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Username or Email",
            | Lang::Zh => "用户名或邮箱",
        }
    }

    pub const fn password_label(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Password",
            | Lang::Zh => "密码",
        }
    }

    pub const fn confirm_password_label(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Confirm Password",
            | Lang::Zh => "确认密码",
        }
    }

    pub const fn passwords_mismatch(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Passwords do not match",
            | Lang::Zh => "两次输入的密码不一致",
        }
    }

    pub const fn invitation_codes_title(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Invitation Codes",
            | Lang::Zh => "邀请码管理",
        }
    }

    pub const fn invitation_codes_desc(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Generate and manage multi-use invitation codes with expiration dates and usage limits",
            | Lang::Zh => "生成并管理支持多人员使用次数与有效期的注册邀请码",
        }
    }

    pub const fn generate_code_btn(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Generate Invitation Code",
            | Lang::Zh => "生成新邀请码",
        }
    }

    pub const fn max_uses_label(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Max Uses",
            | Lang::Zh => "最大使用次数",
        }
    }

    pub const fn expires_in_days_label(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Validity (Days)",
            | Lang::Zh => "有效天数",
        }
    }

    pub const fn code_optional_hint(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Custom Code (Leave empty to auto-generate)",
            | Lang::Zh => "自定义邀请码（留空则自动生成）",
        }
    }

    pub const fn email_restriction_hint(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Restrict to Email (Optional, leave empty for open sharing)",
            | Lang::Zh => "指定邮箱（选填，留空则允许任何人凭码注册）",
        }
    }

    pub const fn revoke_btn(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Revoke",
            | Lang::Zh => "作废",
        }
    }

    pub const fn submit_login(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Sign In",
            | Lang::Zh => "登录",
        }
    }

    pub const fn no_account_prompt(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Don't have an account? Register with invitation",
            | Lang::Zh => "还没有账号？使用邀请码注册新账号",
        }
    }

    pub const fn register_title(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Create Research Account",
            | Lang::Zh => "注册科研账号",
        }
    }

    pub const fn register_subtitle(&self) -> &'static str {
        match self.lang {
            | Lang::En => {
                "Join your laboratory to access shared documents, notes, and versioned projects"
            },
            | Lang::Zh => "加入组织，共享文档、笔记与版本化项目",
        }
    }

    pub const fn invite_code_label(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Invitation Code",
            | Lang::Zh => "邀请码",
        }
    }

    pub const fn submit_register(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Create Account",
            | Lang::Zh => "立即注册",
        }
    }

    pub const fn have_account_prompt(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Already have an account? Sign in",
            | Lang::Zh => "已有账号？直接登录",
        }
    }

    pub const fn session_expired_notice(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Your session has expired. Please sign in again.",
            | Lang::Zh => "您的登录会话已过期，请重新登录。",
        }
    }

    pub const fn registration_mode_invite(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Invite Code Mode (Valid invitation code required)",
            | Lang::Zh => "邀请码注册模式（需要填写有效邀请码）",
        }
    }

    pub const fn registration_mode_open(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Open Registration Mode (Email Sign-up)",
            | Lang::Zh => "开放注册模式（支持邮箱注册）",
        }
    }

    pub const fn registration_mode_closed(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Self-registration is currently closed by the platform administrator.",
            | Lang::Zh => "当前自主注册已由平台管理员关闭。",
        }
    }

    pub const fn invite_code_optional(&self) -> &'static str {
        match self.lang {
            | Lang::En => "(Optional) Invitation Code",
            | Lang::Zh => "邀请码（选填）",
        }
    }

    // --- Profile & Password Management ---
    pub const fn edit_profile(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Edit Profile",
            | Lang::Zh => "编辑个人资料",
        }
    }

    pub const fn save_profile(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Save Profile",
            | Lang::Zh => "保存资料",
        }
    }

    pub const fn avatar_url(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Avatar URL",
            | Lang::Zh => "头像链接",
        }
    }

    pub const fn change_password(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Change Password",
            | Lang::Zh => "修改密码",
        }
    }

    pub const fn current_password(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Current Password",
            | Lang::Zh => "当前密码",
        }
    }

    pub const fn new_password(&self) -> &'static str {
        match self.lang {
            | Lang::En => "New Password",
            | Lang::Zh => "新密码",
        }
    }

    pub const fn confirm_password(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Confirm New Password",
            | Lang::Zh => "确认新密码",
        }
    }

    pub const fn update_password_btn(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Update Password",
            | Lang::Zh => "更新密码",
        }
    }

    // --- Platform & SMTP Management ---
    pub const fn registration_policy_title(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Registration & Admission Policy",
            | Lang::Zh => "注册准入策略",
        }
    }

    pub const fn registration_policy_subtitle(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Controls how new researchers can sign up and join workspace projects",
            | Lang::Zh => "控制新用户如何注册并加入工作区项目",
        }
    }

    pub const fn registration_mode_label(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Registration Admission Mode",
            | Lang::Zh => "注册准入模式",
        }
    }

    pub const fn save_policy(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Save Policy",
            | Lang::Zh => "保存策略",
        }
    }

    pub const fn sso_clients_title(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Single Sign-On Clients",
            | Lang::Zh => "单点登录客户端",
        }
    }

    pub const fn sso_clients_subtitle(&self) -> &'static str {
        match self.lang {
            | Lang::En => {
                "APICH acts as an OpenID Connect provider for these registered applications"
            },
            | Lang::Zh => "APICH 作为这些已注册应用的 OpenID Connect 身份提供方",
        }
    }

    pub const fn no_sso_clients(&self) -> &'static str {
        match self.lang {
            | Lang::En => "No SSO clients registered yet",
            | Lang::Zh => "尚未注册任何 SSO 客户端",
        }
    }

    pub const fn view_openid_discovery(&self) -> &'static str {
        match self.lang {
            | Lang::En => "View OpenID Discovery Document",
            | Lang::Zh => "查看 OpenID 发现文档",
        }
    }

    pub const fn smtp_config_title(&self) -> &'static str {
        match self.lang {
            | Lang::En => "SMTP Outbound Mail Server",
            | Lang::Zh => "SMTP 外发邮件服务器配置",
        }
    }

    pub const fn smtp_config_subtitle(&self) -> &'static str {
        match self.lang {
            Lang::En => "Full configuration for transactional delivery, invitation dispatches, and system alerts",
            Lang::Zh => "配置外发邮件网关，用于发送组织邀请链接、密码找回和容器告警通知",
        }
    }

    pub const fn smtp_enabled(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Enable Outbound SMTP Delivery",
            | Lang::Zh => "启用真实 SMTP 邮件外发服务",
        }
    }

    pub const fn smtp_host(&self) -> &'static str {
        match self.lang {
            | Lang::En => "SMTP Host",
            | Lang::Zh => "SMTP 服务器主机",
        }
    }

    pub const fn smtp_port(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Port",
            | Lang::Zh => "端口号",
        }
    }

    pub const fn smtp_username(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Username / Account",
            | Lang::Zh => "认证用户名 / 账号",
        }
    }

    pub const fn smtp_password(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Password / App Token",
            | Lang::Zh => "认证密码 / 授权码",
        }
    }

    pub const fn smtp_from_email(&self) -> &'static str {
        match self.lang {
            | Lang::En => "From Email Address",
            | Lang::Zh => "发件人邮箱",
        }
    }

    pub const fn smtp_from_name(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Sender Display Name",
            | Lang::Zh => "发件人显示名",
        }
    }

    pub const fn smtp_security(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Encryption Security (TLS / STARTTLS)",
            | Lang::Zh => "加密安全连接 (TLS / STARTTLS)",
        }
    }

    pub const fn smtp_encryption_mode(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Transport Security / Encryption",
            | Lang::Zh => "传输加密与安全协议",
        }
    }

    pub const fn smtp_sec_force_tls(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Force TLS / SMTPS (Implicit TLS - Port 465)",
            | Lang::Zh => "强制 TLS / SMTPS (隐式 TLS 直连 - 端口 465)",
        }
    }

    pub const fn smtp_sec_starttls(&self) -> &'static str {
        match self.lang {
            | Lang::En => "STARTTLS (Explicit TLS Upgrade - Port 587)",
            | Lang::Zh => "STARTTLS (明文升级 TLS - 端口 587)",
        }
    }

    pub const fn smtp_sec_none(&self) -> &'static str {
        match self.lang {
            | Lang::En => "None / Plaintext (Unencrypted - Port 25)",
            | Lang::Zh => "无加密 / 明文传输 (端口 25)",
        }
    }

    pub const fn save_smtp_settings(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Save SMTP Configuration",
            | Lang::Zh => "保存 SMTP 设置",
        }
    }

    pub const fn test_smtp_title(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Test SMTP Delivery",
            | Lang::Zh => "测试邮件外发",
        }
    }

    pub const fn test_recipient(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Test Recipient Address",
            | Lang::Zh => "测试收件人邮箱",
        }
    }

    pub const fn send_test_email(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Send Test Email",
            | Lang::Zh => "发送测试邮件",
        }
    }

    // --- Organization & Team Management ---
    pub const fn create_organization(&self) -> &'static str {
        match self.lang {
            | Lang::En => "+ Create Organization",
            | Lang::Zh => "+ 创建新机构",
        }
    }

    pub const fn edit_organization(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Edit Organization",
            | Lang::Zh => "编辑机构信息",
        }
    }

    pub const fn delete_organization(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Delete Organization",
            | Lang::Zh => "删除机构",
        }
    }

    pub const fn org_name(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Organization Name",
            | Lang::Zh => "机构名称",
        }
    }

    pub const fn org_slug(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Organization Slug",
            | Lang::Zh => "机构标识 (Slug)",
        }
    }

    pub const fn org_desc(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Description",
            | Lang::Zh => "机构描述",
        }
    }

    pub const fn edit_team(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Edit Team",
            | Lang::Zh => "编辑团队",
        }
    }

    pub const fn delete_team(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Delete Team",
            | Lang::Zh => "删除团队",
        }
    }

    pub const fn team_desc(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Team Description",
            | Lang::Zh => "团队描述",
        }
    }

    pub const fn top_level_team(&self) -> &'static str {
        match self.lang {
            | Lang::En => "-- Top Level (Root of Org) --",
            | Lang::Zh => "-- 顶级团队 (直属机构根节点) --",
        }
    }

    pub const fn tab_tables(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Tables (SQLite)",
            | Lang::Zh => "SQLite 数据库表格",
        }
    }

    /// The table page's own file switcher -- kept deliberately less "database"-sounding than
    /// `tab_tables` above (a one-time nav label vs. text sitting right above the grid on every
    /// visit): every table here really is a physical SQLite file, but that fact doesn't need to
    /// dominate the day-to-day spreadsheet-editing experience.
    pub const fn table_active_file_label(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Table file:",
            | Lang::Zh => "表格文件：",
        }
    }

    pub const fn table_new_file(&self) -> &'static str {
        match self.lang {
            | Lang::En => "New table file",
            | Lang::Zh => "新建表格文件",
        }
    }

    pub const fn table_no_tables_title(&self) -> &'static str {
        match self.lang {
            | Lang::En => "No tables yet",
            | Lang::Zh => "暂无表格",
        }
    }

    pub const fn table_no_tables_desc(&self) -> &'static str {
        match self.lang {
            Lang::En => "Tables are stored as real, version-controlled SQLite files -- create the first one to get started.",
            Lang::Zh => "表格以真实的、受版本控制的 SQLite 文件形式存储——创建第一个表格即可开始使用。",
        }
    }

    pub const fn table_init_first(&self) -> &'static str {
        match self.lang {
            | Lang::En => "+ Create first table",
            | Lang::Zh => "+ 创建第一个表格",
        }
    }

    pub const fn tab_knowledge(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Knowledge Hub",
            | Lang::Zh => "知识图谱与看板",
        }
    }

    pub const fn tab_terminal(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Terminal",
            | Lang::Zh => "交互终端",
        }
    }

    pub const fn terminal_idle_status(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Starts automatically on first command",
            | Lang::Zh => "首次运行命令时自动启动",
        }
    }

    pub const fn terminal_subtitle(&self) -> &'static str {
        match self.lang {
            Lang::En => "A real shell for this project -- run any command, build tool, or script directly against your files",
            Lang::Zh => "这是本项目的真实终端——可直接对你的项目文件运行任意命令、构建工具或脚本",
        }
    }

    /// The rest of `TerminalPage`'s `initial_screen` (username/slug line) stays built in
    /// `terminal_page.rs` itself -- no other `I18n` method takes formatting arguments, so this
    /// keeps that convention rather than being the one exception.
    pub const fn terminal_hint(&self) -> &'static str {
        match self.lang {
            Lang::En => "cargo, git, python3, and R are all available here -- e.g. `cargo new my_crate && cd my_crate && cargo run` for a real multi-file Rust project.\n\nType a command below and press Enter or click 'Run'.",
            Lang::Zh => "这里已提供 cargo、git、python3 和 R——例如执行 `cargo new my_crate && cd my_crate && cargo run` 即可创建并运行一个真正的多文件 Rust 项目。\n\n在下方输入命令，按回车或点击“运行”。",
        }
    }

    pub const fn view_kanban(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Kanban Board",
            | Lang::Zh => "任务看板",
        }
    }

    /// Default titles a Kanban board's 3 columns start out with, before anyone customizes them
    /// (see `KnowledgeSyncService::default_kanban_columns`) -- distinct from `view_kanban` (the
    /// tab label), these are the individual column names.
    pub const fn kanban_col_todo(&self) -> &'static str {
        match self.lang {
            | Lang::En => "To Do",
            | Lang::Zh => "待办",
        }
    }

    pub const fn kanban_col_in_progress(&self) -> &'static str {
        match self.lang {
            | Lang::En => "In Progress",
            | Lang::Zh => "进行中",
        }
    }

    pub const fn kanban_col_done(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Completed",
            | Lang::Zh => "已完成",
        }
    }

    /// The catch-all column for a task whose resolved status matches none of the board's
    /// currently configured columns (e.g. its column was deleted or renamed after the task was
    /// tagged for it) -- see `KANBAN_UNSORTED_COLUMN_ID`.
    pub const fn kanban_col_unsorted(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Unsorted",
            | Lang::Zh => "未分类",
        }
    }

    pub const fn kanban_customize_columns(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Customize columns",
            | Lang::Zh => "自定义列",
        }
    }

    pub const fn kanban_add_column(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Add column",
            | Lang::Zh => "添加列",
        }
    }

    pub const fn kanban_column_name_placeholder(&self) -> &'static str {
        match self.lang {
            | Lang::En => "New column name",
            | Lang::Zh => "新列名称",
        }
    }

    pub const fn kanban_mark_as_done_column(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Counts as \"done\"",
            | Lang::Zh => "计为“已完成”",
        }
    }

    pub const fn kanban_rename(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Rename",
            | Lang::Zh => "重命名",
        }
    }

    pub const fn kanban_delete_column(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Delete column",
            | Lang::Zh => "删除列",
        }
    }

    pub const fn kanban_move_left(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Move left",
            | Lang::Zh => "左移",
        }
    }

    pub const fn kanban_move_right(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Move right",
            | Lang::Zh => "右移",
        }
    }

    pub const fn kanban_no_tasks(&self) -> &'static str {
        match self.lang {
            | Lang::En => "No tasks",
            | Lang::Zh => "暂无任务",
        }
    }

    pub const fn kanban_toggle_task(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Toggle task",
            | Lang::Zh => "切换任务状态",
        }
    }

    pub const fn kanban_completion_rate(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Tasks Completion Rate:",
            | Lang::Zh => "任务完成率：",
        }
    }

    pub const fn view_wiki(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Wiki & Graph",
            | Lang::Zh => "双链图谱",
        }
    }

    pub const fn view_calendar(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Calendar",
            | Lang::Zh => "日程日历",
        }
    }

    pub const fn visual_grid(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Visual Grid",
            | Lang::Zh => "可视化表格",
        }
    }

    pub const fn sql_console(&self) -> &'static str {
        match self.lang {
            | Lang::En => "SQL Console",
            | Lang::Zh => "原生 SQL 控制台",
        }
    }

    pub const fn external_hub(&self) -> &'static str {
        match self.lang {
            | Lang::En => "External Hub",
            | Lang::Zh => "协同中心",
        }
    }

    pub const fn run_query(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Run Query",
            | Lang::Zh => "执行查询",
        }
    }

    pub const fn hub_links_settings(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Hub Integrations & External Links",
            | Lang::Zh => "协同中心与外部系统链接",
        }
    }

    pub const fn chat_url_label(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Instant Messaging (Chat) URL",
            | Lang::Zh => "即时通讯 / 群聊链接",
        }
    }

    pub const fn meeting_url_label(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Video Meeting URL",
            | Lang::Zh => "视频会议链接",
        }
    }

    pub const fn drive_url_label(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Cloud Drive URL",
            | Lang::Zh => "云盘 / 资料库链接",
        }
    }

    pub const fn ai_agent_url_label(&self) -> &'static str {
        match self.lang {
            | Lang::En => "AI Assistant URL",
            | Lang::Zh => "AI 助手 / 智能体链接",
        }
    }

    pub const fn allow_team_override_label(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Allow teams to override hub links",
            | Lang::Zh => "允许下属团队自定义覆盖链接",
        }
    }

    pub const fn tab_password(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Password",
            | Lang::Zh => "密码登录",
        }
    }

    pub const fn tab_passkey(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Passkey (FIDO2)",
            | Lang::Zh => "通行密钥 (FIDO2)",
        }
    }

    pub const fn create_org_prompt(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Create an Organization for my research lab / team",
            | Lang::Zh => "同时创建科研实验室/课题组机构 (Organization)",
        }
    }

    pub const fn load_demo_project(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Load Demo Project",
            | Lang::Zh => "加载全功能演示项目",
        }
    }

    pub const fn tab_files(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Files",
            | Lang::Zh => "文件列表",
        }
    }

    pub const fn tab_vcs(&self) -> &'static str {
        match self.lang {
            | Lang::En => "VCS & History",
            | Lang::Zh => "版本与快照",
        }
    }

    pub const fn tab_sharing(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Sharing & Access",
            | Lang::Zh => "共享与权限",
        }
    }

    pub const fn role_read_only(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Read Only",
            | Lang::Zh => "仅可阅读 (Read Only)",
        }
    }

    pub const fn role_read_and_review(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Read & Review",
            | Lang::Zh => "阅读与审阅 (Read & Review)",
        }
    }

    pub const fn role_read_write_and_review(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Read, Write & Review",
            | Lang::Zh => "读写与审阅合并 (Read, Write & Review)",
        }
    }

    pub const fn public_link_sharing(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Public Link Sharing",
            | Lang::Zh => "公开链接访问",
        }
    }

    pub const fn open_in_app(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Open",
            | Lang::Zh => "打开",
        }
    }

    pub const fn formula_bar(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Formula",
            | Lang::Zh => "公式",
        }
    }

    pub const fn add_row(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Add Row",
            | Lang::Zh => "添加行",
        }
    }

    pub const fn add_col(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Add Column",
            | Lang::Zh => "添加列",
        }
    }

    pub const fn delete_row(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Delete Row",
            | Lang::Zh => "删除行",
        }
    }

    pub const fn raw_sql_layer(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Advanced: Raw SQLite Database Layer",
            | Lang::Zh => "底层 SQLite 数据库控制台与 SQL",
        }
    }

    pub const fn doc_slide_studio(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Document & Slide Studio",
            | Lang::Zh => "学术文档与幻灯片工作台",
        }
    }

    pub const fn spreadsheet_studio(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Spreadsheet Studio",
            | Lang::Zh => "结构化表格工作台",
        }
    }

    pub const fn unified_note_studio(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Unified Note Studio",
            | Lang::Zh => "全能笔记与知识工作台",
        }
    }

    pub const fn present_mode(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Present (F11)",
            | Lang::Zh => "全屏演示 (F11)",
        }
    }

    pub const fn sidebar_platform_admin(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Platform Admin",
            | Lang::Zh => "平台系统管理",
        }
    }

    pub const fn sidebar_org_admin(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Org & Teams",
            | Lang::Zh => "机构与团队管理",
        }
    }

    pub const fn sidebar_my_org_admin(&self) -> &'static str {
        match self.lang {
            | Lang::En => "My Org & Teams",
            | Lang::Zh => "我管理的机构与团队",
        }
    }

    pub const fn sidebar_settings(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Personal Settings",
            | Lang::Zh => "个人设置",
        }
    }

    pub const fn project_name(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Project Name",
            | Lang::Zh => "项目名称",
        }
    }

    pub const fn project_slug(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Project Slug",
            | Lang::Zh => "项目标识 (Slug)",
        }
    }

    pub const fn description_label(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Description",
            | Lang::Zh => "描述",
        }
    }

    pub const fn copy_link(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Copy Share Link",
            | Lang::Zh => "复制分享链接",
        }
    }

    pub const fn sidebar_user_admin(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Users",
            | Lang::Zh => "用户管理",
        }
    }

    pub const fn user_admin_title(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Platform Users",
            | Lang::Zh => "平台用户总览",
        }
    }

    pub const fn user_admin_subtitle(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Provision users, monitor allocated vs used storage space, manage locks, reset passwords, and assign organizations & teams.",
            | Lang::Zh => "管理平台账号、监控空间配额与已用容量、设置账号锁定、重置密码及分配所属机构和团队。",
        }
    }

    pub const fn new_user(&self) -> &'static str {
        match self.lang {
            | Lang::En => "New User",
            | Lang::Zh => "新建用户",
        }
    }

    pub const fn create_user_title(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Create New User Account",
            | Lang::Zh => "创建新用户账号",
        }
    }

    pub const fn user_storage_quota(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Storage Quota",
            | Lang::Zh => "空间配额",
        }
    }

    pub const fn user_storage_used(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Storage Used",
            | Lang::Zh => "已用容量",
        }
    }

    pub const fn user_storage_quota_mb(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Storage Quota (MB)",
            | Lang::Zh => "存储配额 (MB)",
        }
    }

    pub const fn update_storage_quota(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Update Storage Quota",
            | Lang::Zh => "修改存储配额",
        }
    }

    pub const fn lock_user(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Lock User",
            | Lang::Zh => "锁定用户",
        }
    }

    pub const fn unlock_user(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Unlock User",
            | Lang::Zh => "解锁用户",
        }
    }

    pub const fn reset_password(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Reset Password",
            | Lang::Zh => "重置密码",
        }
    }

    pub const fn delete_user(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Delete User",
            | Lang::Zh => "删除用户",
        }
    }

    pub const fn assign_org_teams(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Assign Org / Teams",
            | Lang::Zh => "分配机构 / 团队",
        }
    }

    pub const fn account_status(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Status",
            | Lang::Zh => "状态",
        }
    }

    pub const fn active(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Active",
            | Lang::Zh => "正常活跃",
        }
    }

    pub const fn locked(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Locked",
            | Lang::Zh => "已锁定",
        }
    }

    // --- Two-Factor Authentication (2FA) & TOTP ---
    pub const fn global_2fa_title(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Two-Factor Authentication (2FA) Policy",
            | Lang::Zh => "双因素认证 (2FA) 全局策略",
        }
    }

    pub const fn global_2fa_enforce(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Enforce 2FA globally for all accounts",
            | Lang::Zh => "全局强制所有账户启用 2FA 双因素认证",
        }
    }

    pub const fn global_2fa_desc(&self) -> &'static str {
        match self.lang {
            | Lang::En => "When enabled, every user must authenticate with a second factor (Passkey, TOTP authenticator app, or email code) after password verification.",
            | Lang::Zh => "启用后，所有用户在输入密码后必须通过第二重验证（通行密钥、身份验证器 TOTP 或邮件验证码）。",
        }
    }

    pub const fn totp_title(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Two-Factor Authentication (TOTP)",
            | Lang::Zh => "两步验证 (TOTP 动态验证码)",
        }
    }

    pub const fn totp_desc(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Protect your account with an authenticator app (Google Authenticator, Microsoft Authenticator, Authy, etc.).",
            | Lang::Zh => "使用身份验证器应用（如 Google Authenticator、Authy 等）为您的账户提供二次安全保护。",
        }
    }

    pub const fn totp_status_active(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Active & Protected",
            | Lang::Zh => "已启用保护",
        }
    }

    pub const fn totp_status_disabled(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Not Configured",
            | Lang::Zh => "未启用",
        }
    }

    pub const fn totp_setup_btn(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Set Up Authenticator App",
            | Lang::Zh => "设置身份验证器",
        }
    }

    pub const fn totp_disable_btn(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Disable 2FA",
            | Lang::Zh => "停用两步验证",
        }
    }

    pub const fn totp_scan_qr_prompt(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Scan this QR code with your authenticator app:",
            | Lang::Zh => "使用您的身份验证器应用扫描下方二维码：",
        }
    }

    pub const fn totp_manual_key_prompt(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Unable to scan the QR code? Enter this secret key manually into your app:",
            | Lang::Zh => "无法扫描二维码？请手动将以下密钥添加至验证器应用：",
        }
    }

    pub const fn totp_copy_key(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Copy Key",
            | Lang::Zh => "复制密钥",
        }
    }

    pub const fn totp_verify_code_label(&self) -> &'static str {
        match self.lang {
            | Lang::En => "8-Digit Verification Code",
            | Lang::Zh => "8 位动态验证码",
        }
    }

    pub const fn totp_activate_btn(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Verify & Activate 2FA",
            | Lang::Zh => "验证并启用两步验证",
        }
    }

    pub const fn two_factor_login_title(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Two-Factor Authentication",
            | Lang::Zh => "双因素身份验证",
        }
    }

    pub const fn two_factor_login_subtitle(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Confirm your identity with a verification code or passkey to finish signing in.",
            | Lang::Zh => "请输入验证码或使用通行密钥完成登录。",
        }
    }

    pub const fn two_factor_code_placeholder(&self) -> &'static str {
        match self.lang {
            | Lang::En => "8-digit code",
            | Lang::Zh => "8 位验证码",
        }
    }

    pub const fn send_code_email_btn(&self) -> &'static str {
        match self.lang {
            | Lang::En => "✉️ Send code to my email",
            | Lang::Zh => "✉️ 发送验证码到我的邮箱",
        }
    }

    pub const fn passkey_2fa_btn(&self) -> &'static str {
        match self.lang {
            | Lang::En => "🔑 Verify with Passkey (FIDO2)",
            | Lang::Zh => "🔑 使用通行密钥验证 (FIDO2)",
        }
    }

    pub const fn verify_and_login_btn(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Verify & Sign In",
            | Lang::Zh => "验证并登录",
        }
    }

    // --- Toolbar, Navigation, and Action Buttons ---
    pub const fn back_to_files(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Back to Files",
            | Lang::Zh => "返回文件列表",
        }
    }

    pub const fn back_to_dashboard(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Back to Dashboard",
            | Lang::Zh => "返回主控台",
        }
    }

    pub const fn history(&self) -> &'static str {
        match self.lang {
            | Lang::En => "History",
            | Lang::Zh => "历史版本",
        }
    }

    pub const fn ai_copilot(&self) -> &'static str {
        match self.lang {
            | Lang::En => "AI Copilot",
            | Lang::Zh => "AI 助手",
        }
    }

    pub const fn download_pdf(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Download PDF",
            | Lang::Zh => "下载 PDF",
        }
    }

    pub const fn new_file(&self) -> &'static str {
        match self.lang {
            | Lang::En => "+ New File",
            | Lang::Zh => "+ 新建文件",
        }
    }

    pub const fn new_folder(&self) -> &'static str {
        match self.lang {
            | Lang::En => "+ New Folder",
            | Lang::Zh => "+ 新建文件夹",
        }
    }

    pub const fn create_file(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Create File",
            | Lang::Zh => "创建文件",
        }
    }

    pub const fn create_folder(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Create Folder",
            | Lang::Zh => "创建文件夹",
        }
    }

    pub const fn open_file(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Open File",
            | Lang::Zh => "打开文件",
        }
    }

    pub const fn rename(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Rename",
            | Lang::Zh => "重命名",
        }
    }

    pub const fn quick_start(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Quick Start",
            | Lang::Zh => "快速开始",
        }
    }

    pub const fn sidebar_heading_workspace(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Workspace",
            | Lang::Zh => "工作区",
        }
    }

    pub const fn sidebar_heading_platform(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Platform",
            | Lang::Zh => "平台管理",
        }
    }

    pub const fn sidebar_heading_admin(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Administration",
            | Lang::Zh => "组织管理",
        }
    }

    pub const fn sidebar_heading_account(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Account",
            | Lang::Zh => "账户设置",
        }
    }

    pub const fn columns(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Columns",
            | Lang::Zh => "列管理",
        }
    }

    pub const fn export(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Export",
            | Lang::Zh => "导出",
        }
    }

    pub const fn share(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Share",
            | Lang::Zh => "分享",
        }
    }

    pub const fn search(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Search",
            | Lang::Zh => "搜索",
        }
    }

    pub const fn switch_branch(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Switch",
            | Lang::Zh => "切换分支",
        }
    }

    pub const fn create_branch(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Create Branch",
            | Lang::Zh => "创建分支",
        }
    }

    pub const fn undo(&self) -> &'static str {
        match self.lang {
            | Lang::En => "↶ Undo",
            | Lang::Zh => "↶ 撤销",
        }
    }

    pub const fn redo(&self) -> &'static str {
        match self.lang {
            | Lang::En => "↷ Redo",
            | Lang::Zh => "↷ 重做",
        }
    }

    pub const fn save_remote(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Save Remote",
            | Lang::Zh => "保存远程仓库",
        }
    }

    pub const fn git_fetch(&self) -> &'static str {
        match self.lang {
            | Lang::En => "⬇ Fetch",
            | Lang::Zh => "⬇ 拉取分支",
        }
    }

    pub const fn git_pull(&self) -> &'static str {
        match self.lang {
            | Lang::En => "⬇ Pull",
            | Lang::Zh => "⬇ 拉取代码 (Pull)",
        }
    }

    pub const fn git_push(&self) -> &'static str {
        match self.lang {
            | Lang::En => "⬆ Push",
            | Lang::Zh => "⬆ 推送代码 (Push)",
        }
    }

    pub const fn git_rebase(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Rebase",
            | Lang::Zh => "变基 (Rebase)",
        }
    }

    pub const fn add_rule(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Add Rule",
            | Lang::Zh => "添加规则",
        }
    }

    pub const fn save_gitignore(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Save .gitignore",
            | Lang::Zh => "保存 .gitignore",
        }
    }

    pub const fn save_apichignore(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Save .apichignore",
            | Lang::Zh => "保存 .apichignore",
        }
    }

    pub const fn notes_and_wiki(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Notes & Wiki",
            | Lang::Zh => "笔记与知识网",
        }
    }

    pub const fn terminal_btn(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Terminal",
            | Lang::Zh => "交互终端",
        }
    }

    pub const fn folders_view(&self) -> &'static str {
        match self.lang {
            | Lang::En => "📂 Folders",
            | Lang::Zh => "📂 文件夹模式",
        }
    }

    pub const fn all_files_view(&self) -> &'static str {
        match self.lang {
            | Lang::En => "☰ All files",
            | Lang::Zh => "☰ 所有文件",
        }
    }

    pub fn share_public(&self, role: &str) -> String {
        match self.lang {
            | Lang::En => format!("Public ({role})"),
            | Lang::Zh => format!("公开 ({role})"),
        }
    }

    pub fn share_specific(&self, role: &str) -> String {
        match self.lang {
            | Lang::En => format!("Specific ({role})"),
            | Lang::Zh => format!("指定成员 ({role})"),
        }
    }

    pub const fn share_private(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Private",
            | Lang::Zh => "私有",
        }
    }

    pub const fn delete_file(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Delete File",
            | Lang::Zh => "删除文件",
        }
    }

    pub const fn delete_folder_confirm(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Delete this folder and everything in it?",
            | Lang::Zh => "确定要删除此文件夹及其中的全部内容吗？",
        }
    }

    pub fn delete_file_confirm(&self, filename: &str) -> String {
        match self.lang {
            | Lang::En => format!("Are you sure you want to permanently delete \"{filename}\"? This cannot be undone."),
            | Lang::Zh => format!("确定要永久删除 \"{filename}\" 吗？此操作无法撤销。"),
        }
    }

    pub const fn user_account(&self) -> &'static str {
        match self.lang {
            | Lang::En => "User Account",
            | Lang::Zh => "用户账号",
        }
    }

    pub const fn storage_space(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Storage Space (Used / Quota)",
            | Lang::Zh => "存储空间（已用 / 配额）",
        }
    }

    pub const fn affiliations(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Affiliations",
            | Lang::Zh => "组织与团队",
        }
    }

    pub const fn quota_btn(&self) -> &'static str {
        match self.lang {
            | Lang::En => "⚙️ Quota",
            | Lang::Zh => "⚙️ 配额与角色",
        }
    }

    pub const fn affiliations_btn(&self) -> &'static str {
        match self.lang {
            | Lang::En => "🏛️ Affiliations",
            | Lang::Zh => "🏛️ 隶属机构",
        }
    }

    pub const fn reset_btn(&self) -> &'static str {
        match self.lang {
            | Lang::En => "🔑 Reset",
            | Lang::Zh => "🔑 重置密码",
        }
    }

    pub const fn lock_btn(&self) -> &'static str {
        match self.lang {
            | Lang::En => "🔒 Lock",
            | Lang::Zh => "🔒 锁定",
        }
    }

    pub const fn unlock_btn(&self) -> &'static str {
        match self.lang {
            | Lang::En => "🔓 Unlock",
            | Lang::Zh => "🔓 解锁",
        }
    }

    pub const fn add_team(&self) -> &'static str {
        match self.lang {
            | Lang::En => "+ Team",
            | Lang::Zh => "+ 新建团队",
        }
    }

    pub const fn create_team_title(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Create New Team",
            | Lang::Zh => "创建新团队",
        }
    }

    pub const fn parent_org(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Parent Organization",
            | Lang::Zh => "所属组织",
        }
    }

    pub const fn team_name_label(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Team Name",
            | Lang::Zh => "团队名称",
        }
    }

    pub const fn team_slug_label(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Team Slug",
            | Lang::Zh => "团队标识",
        }
    }

    pub const fn remove_member_confirm(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Remove member?",
            | Lang::Zh => "确定要移除该成员吗？",
        }
    }

    pub const fn delete_org_confirm(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Delete this organization and everything under it?",
            | Lang::Zh => "确定要删除此组织及其包含的全部内容吗？",
        }
    }

    pub const fn delete_team_confirm(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Delete this team and its subteams?",
            | Lang::Zh => "确定要删除此团队及其子团队吗？",
        }
    }

    pub const fn reset_password_confirm(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Generate and dispatch a temporary password to this user? Existing sessions will be terminated.",
            | Lang::Zh => "是否为此用户生成临时密码并通过邮件发送？现有会话将被终止。",
        }
    }

    pub const fn lock_user_confirm(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Lock this user account? The user will be immediately logged out.",
            | Lang::Zh => "确定要锁定此用户账号吗？该用户将被立即强制登出。",
        }
    }

    pub const fn unlock_user_confirm(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Unlock this user account?",
            | Lang::Zh => "确定要解锁此用户账号吗？",
        }
    }

    pub const fn delete_user_confirm(&self) -> &'static str {
        match self.lang {
            | Lang::En => "PERMANENTLY delete this user account? All owned workspaces, personal access tokens, and sessions will be destroyed. This cannot be undone.",
            | Lang::Zh => "确定要永久删除此用户账号吗？其拥有的全部工作区、个人访问令牌和会话将被销毁，此操作无法撤销。",
        }
    }

    pub const fn pat_heading(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Personal Access Tokens",
            | Lang::Zh => "个人访问令牌",
        }
    }

    pub const fn pat_subtitle(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Used to authenticate the apich CLI and any external git client against your projects.",
            | Lang::Zh => "用于通过 apich CLI 或外部 Git 客户端对您的项目进行身份验证。",
        }
    }

    pub const fn token_name_label(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Token name",
            | Lang::Zh => "令牌名称",
        }
    }

    pub const fn expiration_label(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Expiration",
            | Lang::Zh => "有效期",
        }
    }

    pub const fn generate_token_btn(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Generate Token",
            | Lang::Zh => "生成令牌",
        }
    }

    pub const fn github_integration_title(&self) -> &'static str {
        match self.lang {
            | Lang::En => "GitHub Integration",
            | Lang::Zh => "GitHub 关联整合",
        }
    }

    pub const fn github_integration_subtitle(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Connect your GitHub account to enable automated authentication and one-click remote syncing across all your projects without having to paste tokens into individual repository URLs.",
            | Lang::Zh => "关联您的 GitHub 账号，即可在所有项目中启用自动化身份验证与一键远程同步，无需再为每个仓库手动粘贴令牌。",
        }
    }

    pub const fn ssh_keys_title(&self) -> &'static str {
        match self.lang {
            | Lang::En => "SSH Keys",
            | Lang::Zh => "SSH 公钥",
        }
    }

    pub const fn ssh_keys_subtitle(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Stored for identity/compatibility. SSH-based clone/push transport is not available yet — use a Personal Access Token over HTTPS instead.",
            | Lang::Zh => "用于身份标识与兼容性。目前建议通过 HTTPS 配合个人访问令牌进行克隆与推送。",
        }
    }

    pub const fn gpg_keys_title(&self) -> &'static str {
        match self.lang {
            | Lang::En => "GPG Keys",
            | Lang::Zh => "GPG 公钥",
        }
    }

    pub const fn gpg_keys_subtitle(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Used to verify signed VCS snapshots. Signing itself always happens locally with your own keyring — only your public key is stored here.",
            | Lang::Zh => "用于验证带签名的版本快照。签名操作在本地通过密钥环完成，此处仅存储公钥。",
        }
    }

    pub const fn key_name_label(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Key name",
            | Lang::Zh => "密钥名称",
        }
    }

    pub const fn public_key_label(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Public key",
            | Lang::Zh => "公钥内容",
        }
    }

    pub const fn add_key_btn(&self) -> &'static str {
        match self.lang {
            | Lang::En => "Add Key",
            | Lang::Zh => "添加公钥",
        }
    }
}


