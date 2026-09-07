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
            Lang::Zh
        } else {
            Lang::En
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Zh => "zh",
        }
    }

    pub fn toggle_code(&self) -> &'static str {
        match self {
            Lang::En => "zh",
            Lang::Zh => "en",
        }
    }

    pub fn toggle_label(&self) -> &'static str {
        match self {
            Lang::En => "中文",
            Lang::Zh => "English",
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
    pub fn new(lang: Lang) -> Self {
        Self { lang }
    }

    pub fn is_zh(&self) -> bool {
        self.lang == Lang::Zh
    }

    // --- Navigation & Header ---
    pub fn brand_title(&self) -> &'static str {
        match self.lang {
            Lang::En => "Research Workspace",
            Lang::Zh => "科研工作区",
        }
    }

    pub fn nav_projects(&self) -> &'static str {
        match self.lang {
            Lang::En => "Projects",
            Lang::Zh => "项目列表",
        }
    }

    pub fn nav_teams_orgs(&self) -> &'static str {
        match self.lang {
            Lang::En => "Teams & Orgs",
            Lang::Zh => "团队与组织",
        }
    }

    pub fn nav_admin(&self) -> &'static str {
        match self.lang {
            Lang::En => "System Admin",
            Lang::Zh => "系统管理",
        }
    }

    pub fn nav_settings(&self) -> &'static str {
        match self.lang {
            Lang::En => "Settings",
            Lang::Zh => "个人设置",
        }
    }

    pub fn welcome_prefix(&self) -> &'static str {
        match self.lang {
            Lang::En => "Welcome,",
            Lang::Zh => "欢迎，",
        }
    }

    pub fn logout(&self) -> &'static str {
        match self.lang {
            Lang::En => "Sign Out",
            Lang::Zh => "退出登录",
        }
    }

    // --- Dashboard ---
    pub fn dashboard_title(&self) -> &'static str {
        match self.lang {
            Lang::En => "Projects & Sandboxes",
            Lang::Zh => "科研项目与计算沙箱",
        }
    }

    pub fn dashboard_subtitle(&self) -> &'static str {
        match self.lang {
            Lang::En => "Workspace repositories with FastCDC deduplication and Podman isolated containers",
            Lang::Zh => "管理基于 FastCDC 增量去重版本控制与 Podman 独立容器隔离的计算环境",
        }
    }

    pub fn new_project(&self) -> &'static str {
        match self.lang {
            Lang::En => "New Project",
            Lang::Zh => "新建项目",
        }
    }

    pub fn stat_total_projects(&self) -> &'static str {
        match self.lang {
            Lang::En => "Total Projects",
            Lang::Zh => "我的项目总数",
        }
    }

    pub fn stat_total_projects_sub(&self) -> &'static str {
        match self.lang {
            Lang::En => "Owned or shared repositories",
            Lang::Zh => "具备所有者或协作者权限",
        }
    }

    pub fn stat_active_sandboxes(&self) -> &'static str {
        match self.lang {
            Lang::En => "Active Sandboxes",
            Lang::Zh => "运行中的沙箱",
        }
    }

    pub fn stat_active_sandboxes_sub(&self) -> &'static str {
        match self.lang {
            Lang::En => "Dedicated rootless container instances",
            Lang::Zh => "独立 cgroup 资源隔离计算实例",
        }
    }

    pub fn stat_vcs_storage(&self) -> &'static str {
        match self.lang {
            Lang::En => "Storage Engine",
            Lang::Zh => "存储引擎",
        }
    }

    pub fn stat_vcs_storage_sub(&self) -> &'static str {
        match self.lang {
            Lang::En => "FastCDC content-addressed deduplication",
            Lang::Zh => "FastCDC 内容自适应分块去重",
        }
    }

    pub fn stat_git_bridge(&self) -> &'static str {
        match self.lang {
            Lang::En => "Git Compatibility",
            Lang::Zh => "Git 兼容层",
        }
    }

    pub fn stat_git_bridge_sub(&self) -> &'static str {
        match self.lang {
            Lang::En => "Dual-engine bidirectional sync ready",
            Lang::Zh => "双向就绪，支持与远程 Git 仓库同步",
        }
    }

    pub fn active_projects_section(&self) -> &'static str {
        match self.lang {
            Lang::En => "Active Projects",
            Lang::Zh => "活跃项目工作区",
        }
    }

    pub fn active_projects_hint(&self) -> &'static str {
        match self.lang {
            Lang::En => "Click a project card to inspect code, resolve merge conflicts, and manage sandboxes",
            Lang::Zh => "点击卡片直接进入项目工作区执行 Merge 冲突合并、快照及沙箱管理",
        }
    }

    pub fn enter_workspace(&self) -> &'static str {
        match self.lang {
            Lang::En => "Enter Project",
            Lang::Zh => "进入工作区",
        }
    }

    pub fn launch_sandbox(&self) -> &'static str {
        match self.lang {
            Lang::En => "Start Sandbox",
            Lang::Zh => "启动沙箱",
        }
    }

    pub fn stop_sandbox(&self) -> &'static str {
        match self.lang {
            Lang::En => "Pause Sandbox",
            Lang::Zh => "暂停沙箱",
        }
    }

    pub fn sandbox_running(&self) -> &'static str {
        match self.lang {
            Lang::En => "Sandbox Running",
            Lang::Zh => "计算沙箱正在运行",
        }
    }

    pub fn sandbox_stopped(&self) -> &'static str {
        match self.lang {
            Lang::En => "Sandbox Stopped",
            Lang::Zh => "计算沙箱已暂停",
        }
    }

    pub fn no_projects(&self) -> &'static str {
        match self.lang {
            Lang::En => "No projects found",
            Lang::Zh => "暂无项目",
        }
    }

    pub fn no_projects_desc(&self) -> &'static str {
        match self.lang {
            Lang::En => "Click 'New Project' above to create your first versioned workspace.",
            Lang::Zh => "点击上方“新建项目”以开启版本控制与隔离沙箱计算环境。",
        }
    }

    // --- Project Detail Tabs ---
    pub fn tab_overview(&self) -> &'static str {
        match self.lang {
            Lang::En => "Overview & Sandbox",
            Lang::Zh => "概览与沙箱",
        }
    }

    pub fn tab_merge(&self) -> &'static str {
        match self.lang {
            Lang::En => "Branches & Merge Conflicts",
            Lang::Zh => "分支与 Merge 冲突解决",
        }
    }

    pub fn tab_timeline(&self) -> &'static str {
        match self.lang {
            Lang::En => "Timeline Snapshots",
            Lang::Zh => "时间线快照",
        }
    }

    pub fn tab_git(&self) -> &'static str {
        match self.lang {
            Lang::En => "Git Bridge",
            Lang::Zh => "Git 兼容同步",
        }
    }

    pub fn tab_members(&self) -> &'static str {
        match self.lang {
            Lang::En => "Collaborators",
            Lang::Zh => "协作者成员",
        }
    }

    // --- Merge & Conflicts ---
    pub fn merge_title(&self) -> &'static str {
        match self.lang {
            Lang::En => "Weave-Free Branch & 3-Way Merge Engine",
            Lang::Zh => "Weave-Free 分支与 3-Way 冲突合并引擎",
        }
    }

    pub fn merge_desc(&self) -> &'static str {
        match self.lang {
            Lang::En => "Based on the APICH VCS weave-free history model, branch merges produce a deterministic snapshot without criss-cross merge locks.",
            Lang::Zh => "基于 APICH VCS 无指针交织模型（Weave-Free History），分支合并产生单一确定性快照，彻底消灭 Git 常见的交织历史死锁与重复冲突。",
        }
    }

    pub fn current_branch(&self) -> &'static str {
        match self.lang {
            Lang::En => "Current Branch:",
            Lang::Zh => "当前所在分支:",
        }
    }

    pub fn available_branches(&self) -> &'static str {
        match self.lang {
            Lang::En => "Available branches:",
            Lang::Zh => "可用分支:",
        }
    }

    pub fn select_merge_target(&self) -> &'static str {
        match self.lang {
            Lang::En => "Select target branch to merge into current:",
            Lang::Zh => "选择要合并入当前分支的目标分支：",
        }
    }

    pub fn run_merge(&self) -> &'static str {
        match self.lang {
            Lang::En => "Execute Weave-free 3-Way Reconcile",
            Lang::Zh => "执行 Weave-free 3-Way Reconcile",
        }
    }

    pub fn single_branch_hint(&self) -> &'static str {
        match self.lang {
            Lang::En => "Project currently has only the main branch. Create a feature branch in your sandbox terminal to test 3-way merge reconciliation.",
            Lang::Zh => "当前项目仅有单个分支 main。在沙箱终端中创建功能分支后即可在此进行无指针交织 3-Way 合并。",
        }
    }

    pub fn conflicts_clean_title(&self) -> &'static str {
        match self.lang {
            Lang::En => "Clean Working Tree:",
            Lang::Zh => "状态良好：",
        }
    }

    pub fn conflicts_clean_desc(&self) -> &'static str {
        match self.lang {
            Lang::En => "No merge conflicts detected. All files are fully reconciled.",
            Lang::Zh => "工作区当前无合并冲突。所有文件均已完全对齐。",
        }
    }

    pub fn conflicts_warn_title(&self) -> &'static str {
        match self.lang {
            Lang::En => "Unresolved Code/Document Conflicts:",
            Lang::Zh => "存在未解决的代码/文档合并冲突：",
        }
    }

    pub fn conflicts_warn_desc(&self) -> &'static str {
        match self.lang {
            Lang::En => "Select a resolution strategy below (keep local, accept incoming, or edit custom content).",
            Lang::Zh => "请选择以下文件的解决策略（保留本地版本、采纳传入版本或在线编辑保存）。",
        }
    }

    pub fn conflict_file_prefix(&self) -> &'static str {
        match self.lang {
            Lang::En => "Conflict in:",
            Lang::Zh => "冲突文件：",
        }
    }

    pub fn accept_ours(&self) -> &'static str {
        match self.lang {
            Lang::En => "Keep Local (Ours)",
            Lang::Zh => "采纳本地版本（Ours）",
        }
    }

    pub fn accept_theirs(&self) -> &'static str {
        match self.lang {
            Lang::En => "Accept Incoming (Theirs)",
            Lang::Zh => "采纳传入版本（Theirs）",
        }
    }

    pub fn ours_marker(&self) -> &'static str {
        match self.lang {
            Lang::En => "Local Changes (Ours)",
            Lang::Zh => "本地修改 (Ours)",
        }
    }

    pub fn theirs_marker(&self) -> &'static str {
        match self.lang {
            Lang::En => "Incoming Changes (Theirs)",
            Lang::Zh => "传入修改 (Theirs)",
        }
    }

    // --- Git Tab ---
    pub fn git_title(&self) -> &'static str {
        match self.lang {
            Lang::En => "Git Dual-Engine Compatibility Bridge",
            Lang::Zh => "Git 双向兼容桥接系统",
        }
    }

    pub fn git_desc(&self) -> &'static str {
        match self.lang {
            Lang::En => "Every APICH project is both a native APICH-VCS workspace and a standard Git repository, seamlessly interoperating with GitHub, GitLab, or private Git remotes.",
            Lang::Zh => "APICH 每一个项目既是原生的 APICH-VCS 高性能工作区，同时也作为纯正的 Git 仓库无缝与 GitHub、GitLab 或私有 Git 平台互联。",
        }
    }

    pub fn git_status_label(&self) -> &'static str {
        match self.lang {
            Lang::En => "Git Repository Status",
            Lang::Zh => "Git 仓库初始化状态",
        }
    }

    pub fn git_ready(&self) -> &'static str {
        match self.lang {
            Lang::En => "Ready (.git initialized)",
            Lang::Zh => "已就绪 (.git 存在)",
        }
    }

    pub fn git_pending(&self) -> &'static str {
        match self.lang {
            Lang::En => "Pending Sync",
            Lang::Zh => "自动同步待触发",
        }
    }

    pub fn git_remote_label(&self) -> &'static str {
        match self.lang {
            Lang::En => "Remote URL",
            Lang::Zh => "远程 Remote URL",
        }
    }

    pub fn git_no_remote(&self) -> &'static str {
        match self.lang {
            Lang::En => "No remote configured",
            Lang::Zh => "未绑定远程仓库",
        }
    }

    pub fn git_lfs_label(&self) -> &'static str {
        match self.lang {
            Lang::En => "Large File LFS Policy",
            Lang::Zh => "大文件 LFS 策略",
        }
    }

    pub fn git_lfs_val(&self) -> &'static str {
        match self.lang {
            Lang::En => "Automatic pointer synthesis via FastCDC CAS",
            Lang::Zh => "智能自动合成（FastCDC 指针化）",
        }
    }

    pub fn git_sync_btn(&self) -> &'static str {
        match self.lang {
            Lang::En => "Export & Sync to Local Git Commit",
            Lang::Zh => "一键导出并同步至本地 Git 提交",
        }
    }

    // --- Snapshots & Timeline ---
    pub fn create_snapshot(&self) -> &'static str {
        match self.lang {
            Lang::En => "Create Snapshot",
            Lang::Zh => "创建快照",
        }
    }

    pub fn modal_snapshot_title(&self) -> &'static str {
        match self.lang {
            Lang::En => "Record VCS Snapshot",
            Lang::Zh => "记录版本快照",
        }
    }

    pub fn modal_snapshot_desc(&self) -> &'static str {
        match self.lang {
            Lang::En => "FastCDC content-addressed incremental snapshot with zero pointer weaving",
            Lang::Zh => "基于 FastCDC 变长分块引擎执行增量去重快照，零指针交织",
        }
    }

    pub fn snapshot_message_label(&self) -> &'static str {
        match self.lang {
            Lang::En => "Snapshot Message",
            Lang::Zh => "快照说明",
        }
    }

    pub fn snapshot_submit(&self) -> &'static str {
        match self.lang {
            Lang::En => "Save Snapshot Now",
            Lang::Zh => "立即保存快照",
        }
    }

    // --- Common Form & Action ---
    pub fn cancel(&self) -> &'static str {
        match self.lang {
            Lang::En => "Cancel",
            Lang::Zh => "取消",
        }
    }

    pub fn save(&self) -> &'static str {
        match self.lang {
            Lang::En => "Save",
            Lang::Zh => "保存",
        }
    }

    pub fn delete(&self) -> &'static str {
        match self.lang {
            Lang::En => "Delete",
            Lang::Zh => "删除",
        }
    }

    pub fn remove(&self) -> &'static str {
        match self.lang {
            Lang::En => "Remove",
            Lang::Zh => "移除",
        }
    }

    pub fn back(&self) -> &'static str {
        match self.lang {
            Lang::En => "Back",
            Lang::Zh => "返回",
        }
    }

    pub fn actions(&self) -> &'static str {
        match self.lang {
            Lang::En => "Actions",
            Lang::Zh => "操作",
        }
    }

    pub fn role(&self) -> &'static str {
        match self.lang {
            Lang::En => "Role",
            Lang::Zh => "角色",
        }
    }

    // --- Collaborators Management ---
    pub fn add_collaborator(&self) -> &'static str {
        match self.lang {
            Lang::En => "Add Collaborator",
            Lang::Zh => "添加协作者",
        }
    }

    pub fn select_user(&self) -> &'static str {
        match self.lang {
            Lang::En => "Select User",
            Lang::Zh => "选择用户",
        }
    }

    // --- Organizations & Teams Admin ---
    pub fn org_admin_title(&self) -> &'static str {
        match self.lang {
            Lang::En => "Organizations & Teams",
            Lang::Zh => "组织与团队层级树",
        }
    }

    pub fn org_admin_subtitle(&self) -> &'static str {
        match self.lang {
            Lang::En => "Manage institutional hierarchy, laboratory teams, and member access roles",
            Lang::Zh => "构建机构与实验室团队层级，统一权限管控",
        }
    }

    pub fn create_team(&self) -> &'static str {
        match self.lang {
            Lang::En => "Create Team",
            Lang::Zh => "创建新团队",
        }
    }

    pub fn team_name(&self) -> &'static str {
        match self.lang {
            Lang::En => "Team Name",
            Lang::Zh => "团队名称",
        }
    }

    pub fn team_slug(&self) -> &'static str {
        match self.lang {
            Lang::En => "Team Slug",
            Lang::Zh => "团队英文标识",
        }
    }

    pub fn parent_team(&self) -> &'static str {
        match self.lang {
            Lang::En => "Parent Team",
            Lang::Zh => "上级所属团队",
        }
    }

    pub fn no_parent_root(&self) -> &'static str {
        match self.lang {
            Lang::En => "None (Root Team)",
            Lang::Zh => "无（作为顶层团队）",
        }
    }

    pub fn description(&self) -> &'static str {
        match self.lang {
            Lang::En => "Description",
            Lang::Zh => "简要描述",
        }
    }

    pub fn add_member(&self) -> &'static str {
        match self.lang {
            Lang::En => "Add Member",
            Lang::Zh => "添加成员",
        }
    }

    // --- Settings & Profile ---
    pub fn settings_title(&self) -> &'static str {
        match self.lang {
            Lang::En => "Account & Security Settings",
            Lang::Zh => "个人账号与安全设置",
        }
    }

    pub fn settings_subtitle(&self) -> &'static str {
        match self.lang {
            Lang::En => "Manage your profile, hardware passkeys, and multi-factor credentials",
            Lang::Zh => "管理个人基本信息、硬件 Passkeys 以及多因子安全凭据",
        }
    }

    pub fn profile_card(&self) -> &'static str {
        match self.lang {
            Lang::En => "Personal Profile",
            Lang::Zh => "个人资料",
        }
    }

    pub fn username(&self) -> &'static str {
        match self.lang {
            Lang::En => "Username",
            Lang::Zh => "用户名",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self.lang {
            Lang::En => "Display Name",
            Lang::Zh => "显示名称",
        }
    }

    pub fn email(&self) -> &'static str {
        match self.lang {
            Lang::En => "Email Address",
            Lang::Zh => "邮箱地址",
        }
    }

    pub fn system_role(&self) -> &'static str {
        match self.lang {
            Lang::En => "System Role",
            Lang::Zh => "系统角色",
        }
    }

    pub fn platform_admin(&self) -> &'static str {
        match self.lang {
            Lang::En => "Platform Administrator",
            Lang::Zh => "平台超级管理员",
        }
    }

    pub fn researcher(&self) -> &'static str {
        match self.lang {
            Lang::En => "Researcher",
            Lang::Zh => "科研人员",
        }
    }

    pub fn passkeys_title(&self) -> &'static str {
        match self.lang {
            Lang::En => "Passkeys & Hardware Keys",
            Lang::Zh => "Passkey 与硬件密钥",
        }
    }

    pub fn passkeys_desc(&self) -> &'static str {
        match self.lang {
            Lang::En => "FIDO2 / WebAuthn Level 3 phishing-resistant cryptographic authentication.",
            Lang::Zh => "基于 WebAuthn / FIDO2 Level 3 安全标准，提供免疫钓鱼攻击的生物特征或硬件密钥登录能力。",
        }
    }

    pub fn register_passkey_btn(&self) -> &'static str {
        match self.lang {
            Lang::En => "+ Register New Passkey",
            Lang::Zh => "+ 注册新 Passkey 硬件密钥",
        }
    }

    // --- Auth (Login & Register) ---
    pub fn sign_in_title(&self) -> &'static str {
        match self.lang {
            Lang::En => "Sign In to Workspace",
            Lang::Zh => "登录科研工作区",
        }
    }

    pub fn sign_in_subtitle(&self) -> &'static str {
        match self.lang {
            Lang::En => "Unified sandboxes, FastCDC versioning & Git collaboration",
            Lang::Zh => "统一计算沙箱环境、FastCDC 版本控制与 Git 协同系统",
        }
    }

    pub fn passkey_login_btn(&self) -> &'static str {
        match self.lang {
            Lang::En => "Sign In with Passkey / Hardware Key",
            Lang::Zh => "使用 Passkey / 指纹面容硬件密钥快捷登录",
        }
    }

    pub fn passkey_hint(&self) -> &'static str {
        match self.lang {
            Lang::En => "Supports YubiKey, Touch ID, Windows Hello, and FIDO2 keys",
            Lang::Zh => "支持 YubiKey、Touch ID、Windows Hello 及 FIDO2 标准认证",
        }
    }

    pub fn divider_or(&self) -> &'static str {
        match self.lang {
            Lang::En => "OR SIGN IN WITH PASSWORD",
            Lang::Zh => "或使用账号密码登录",
        }
    }

    pub fn login_label(&self) -> &'static str {
        match self.lang {
            Lang::En => "Username or Email",
            Lang::Zh => "用户名或邮箱",
        }
    }

    pub fn password_label(&self) -> &'static str {
        match self.lang {
            Lang::En => "Password",
            Lang::Zh => "密码",
        }
    }

    pub fn submit_login(&self) -> &'static str {
        match self.lang {
            Lang::En => "Sign In",
            Lang::Zh => "登录",
        }
    }

    pub fn no_account_prompt(&self) -> &'static str {
        match self.lang {
            Lang::En => "Don't have an account? Register with invitation",
            Lang::Zh => "还没有账号？使用邀请码注册新账号",
        }
    }

    pub fn register_title(&self) -> &'static str {
        match self.lang {
            Lang::En => "Create Research Account",
            Lang::Zh => "注册科研账号",
        }
    }

    pub fn register_subtitle(&self) -> &'static str {
        match self.lang {
            Lang::En => "Join your laboratory to access isolated sandboxes and versioned projects",
            Lang::Zh => "加入组织并开始使用隔离计算沙箱与版本化工作区",
        }
    }

    pub fn invite_code_label(&self) -> &'static str {
        match self.lang {
            Lang::En => "Invitation Code",
            Lang::Zh => "邀请码",
        }
    }

    pub fn submit_register(&self) -> &'static str {
        match self.lang {
            Lang::En => "Create Account",
            Lang::Zh => "立即注册",
        }
    }

    pub fn have_account_prompt(&self) -> &'static str {
        match self.lang {
            Lang::En => "Already have an account? Sign in",
            Lang::Zh => "已有账号？直接登录",
        }
    }

    // --- Profile & Password Management ---
    pub fn edit_profile(&self) -> &'static str {
        match self.lang {
            Lang::En => "Edit Profile",
            Lang::Zh => "编辑个人资料",
        }
    }

    pub fn save_profile(&self) -> &'static str {
        match self.lang {
            Lang::En => "Save Profile",
            Lang::Zh => "保存资料",
        }
    }

    pub fn avatar_url(&self) -> &'static str {
        match self.lang {
            Lang::En => "Avatar URL",
            Lang::Zh => "头像链接",
        }
    }

    pub fn change_password(&self) -> &'static str {
        match self.lang {
            Lang::En => "Change Password",
            Lang::Zh => "修改密码",
        }
    }

    pub fn current_password(&self) -> &'static str {
        match self.lang {
            Lang::En => "Current Password",
            Lang::Zh => "当前密码",
        }
    }

    pub fn new_password(&self) -> &'static str {
        match self.lang {
            Lang::En => "New Password",
            Lang::Zh => "新密码",
        }
    }

    pub fn confirm_password(&self) -> &'static str {
        match self.lang {
            Lang::En => "Confirm New Password",
            Lang::Zh => "确认新密码",
        }
    }

    pub fn update_password_btn(&self) -> &'static str {
        match self.lang {
            Lang::En => "Update Password",
            Lang::Zh => "更新密码",
        }
    }

    // --- Platform & SMTP Management ---
    pub fn smtp_config_title(&self) -> &'static str {
        match self.lang {
            Lang::En => "SMTP Outbound Mail Server",
            Lang::Zh => "SMTP 外发邮件服务器配置",
        }
    }

    pub fn smtp_config_subtitle(&self) -> &'static str {
        match self.lang {
            Lang::En => "Full configuration for transactional delivery, invitation dispatches, and system alerts",
            Lang::Zh => "配置外发邮件网关，用于发送组织邀请链接、密码找回和容器告警通知",
        }
    }

    pub fn smtp_enabled(&self) -> &'static str {
        match self.lang {
            Lang::En => "Enable Outbound SMTP Delivery",
            Lang::Zh => "启用真实 SMTP 邮件外发服务",
        }
    }

    pub fn smtp_host(&self) -> &'static str {
        match self.lang {
            Lang::En => "SMTP Host",
            Lang::Zh => "SMTP 服务器主机",
        }
    }

    pub fn smtp_port(&self) -> &'static str {
        match self.lang {
            Lang::En => "Port",
            Lang::Zh => "端口号",
        }
    }

    pub fn smtp_username(&self) -> &'static str {
        match self.lang {
            Lang::En => "Username / Account",
            Lang::Zh => "认证用户名 / 账号",
        }
    }

    pub fn smtp_password(&self) -> &'static str {
        match self.lang {
            Lang::En => "Password / App Token",
            Lang::Zh => "认证密码 / 授权码",
        }
    }

    pub fn smtp_from_email(&self) -> &'static str {
        match self.lang {
            Lang::En => "From Email Address",
            Lang::Zh => "发件人邮箱",
        }
    }

    pub fn smtp_from_name(&self) -> &'static str {
        match self.lang {
            Lang::En => "Sender Display Name",
            Lang::Zh => "发件人显示名",
        }
    }

    pub fn smtp_security(&self) -> &'static str {
        match self.lang {
            Lang::En => "Encryption Security (TLS / STARTTLS)",
            Lang::Zh => "加密安全连接 (TLS / STARTTLS)",
        }
    }

    pub fn save_smtp_settings(&self) -> &'static str {
        match self.lang {
            Lang::En => "Save SMTP Configuration",
            Lang::Zh => "保存 SMTP 设置",
        }
    }

    pub fn test_smtp_title(&self) -> &'static str {
        match self.lang {
            Lang::En => "Test SMTP Delivery",
            Lang::Zh => "测试邮件外发",
        }
    }

    pub fn test_recipient(&self) -> &'static str {
        match self.lang {
            Lang::En => "Test Recipient Address",
            Lang::Zh => "测试收件人邮箱",
        }
    }

    pub fn send_test_email(&self) -> &'static str {
        match self.lang {
            Lang::En => "Send Test Email",
            Lang::Zh => "发送测试邮件",
        }
    }

    // --- Organization & Team Management ---
    pub fn create_organization(&self) -> &'static str {
        match self.lang {
            Lang::En => "+ Create Organization",
            Lang::Zh => "+ 创建新机构",
        }
    }

    pub fn edit_organization(&self) -> &'static str {
        match self.lang {
            Lang::En => "Edit Organization",
            Lang::Zh => "编辑机构信息",
        }
    }

    pub fn delete_organization(&self) -> &'static str {
        match self.lang {
            Lang::En => "Delete Organization",
            Lang::Zh => "删除机构",
        }
    }

    pub fn org_name(&self) -> &'static str {
        match self.lang {
            Lang::En => "Organization Name",
            Lang::Zh => "机构名称",
        }
    }

    pub fn org_slug(&self) -> &'static str {
        match self.lang {
            Lang::En => "Organization Slug",
            Lang::Zh => "机构标识 (Slug)",
        }
    }

    pub fn org_desc(&self) -> &'static str {
        match self.lang {
            Lang::En => "Description",
            Lang::Zh => "机构描述",
        }
    }

    pub fn edit_team(&self) -> &'static str {
        match self.lang {
            Lang::En => "Edit Team",
            Lang::Zh => "编辑团队",
        }
    }

    pub fn delete_team(&self) -> &'static str {
        match self.lang {
            Lang::En => "Delete Team",
            Lang::Zh => "删除团队",
        }
    }

    pub fn team_desc(&self) -> &'static str {
        match self.lang {
            Lang::En => "Team Description",
            Lang::Zh => "团队描述",
        }
    }

    pub fn top_level_team(&self) -> &'static str {
        match self.lang {
            Lang::En => "-- Top Level (Root of Org) --",
            Lang::Zh => "-- 顶级团队 (直属机构根节点) --",
        }
    }
}


