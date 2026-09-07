pub const EMBEDDED_CSS: &str = r#"
:root {
    --bg-base: #f8fafc;
    --bg-surface: #ffffff;
    --bg-card: #ffffff;
    --bg-card-hover: #f8fafc;
    --bg-muted: #f1f5f9;
    --border-subtle: #e2e8f0;
    --border-strong: #cbd5e1;
    --border-focus: #2563eb;
    --text-main: #0f172a;
    --text-muted: #475569;
    --text-sub: #64748b;
    --text-light: #94a3b8;
    --primary: #2563eb;
    --primary-hover: #1d4ed8;
    --primary-light: #eff6ff;
    --primary-border: #bfdbfe;
    --accent-indigo: #4f46e5;
    --accent-violet: #7c3aed;
    --success: #16a34a;
    --success-bg: #f0fdf4;
    --success-border: #bbf7d0;
    --warning: #d97706;
    --warning-bg: #fffbeb;
    --warning-border: #fde68a;
    --danger: #dc2626;
    --danger-bg: #fef2f2;
    --danger-border: #fecaca;
    --radius-sm: 6px;
    --radius-md: 10px;
    --radius-lg: 14px;
    --radius-xl: 18px;
    --shadow-sm: 0 1px 2px 0 rgba(15, 23, 42, 0.05);
    --shadow-md: 0 4px 12px -2px rgba(15, 23, 42, 0.06), 0 2px 6px -1px rgba(15, 23, 42, 0.04);
    --shadow-lg: 0 12px 24px -4px rgba(15, 23, 42, 0.08), 0 4px 8px -2px rgba(15, 23, 42, 0.04);
    --shadow-glow: 0 0 20px -3px rgba(37, 99, 235, 0.18);
    --font-sans: -apple-system, BlinkMacSystemFont, "Inter", "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
    --font-mono: ui-monospace, SFMono-Regular, "JetBrains Mono", Menlo, Monaco, Consolas, monospace;
}

* { box-sizing: border-box; margin: 0; padding: 0; }
body {
    background-color: var(--bg-base);
    background-image: 
        radial-gradient(circle at 10% 15%, rgba(37, 99, 235, 0.04) 0%, transparent 40%),
        radial-gradient(circle at 90% 85%, rgba(124, 58, 237, 0.04) 0%, transparent 40%);
    background-attachment: fixed;
    color: var(--text-main);
    font-family: var(--font-sans);
    line-height: 1.5;
    -webkit-font-smoothing: antialiased;
}

a { color: var(--primary); text-decoration: none; transition: color 0.15s ease; }
a:hover { color: var(--primary-hover); text-decoration: underline; }

/* Navbar - Frosted Glass & Radiant Accents */
.navbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 64px;
    padding: 0 2rem;
    background: rgba(255, 255, 255, 0.88);
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
    border-bottom: 1px solid rgba(226, 232, 240, 0.8);
    box-shadow: 0 1px 3px 0 rgba(15, 23, 42, 0.03);
    position: sticky;
    top: 0;
    z-index: 100;
}

.nav-brand .brand-link {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    text-decoration: none;
    color: var(--text-main);
}
.brand-badge {
    background: linear-gradient(135deg, #2563eb 0%, #4f46e5 50%, #7c3aed 100%);
    color: #fff;
    font-weight: 800;
    font-size: 0.825rem;
    padding: 3px 9px;
    border-radius: var(--radius-sm);
    letter-spacing: 0.8px;
    box-shadow: 0 2px 8px rgba(37, 99, 235, 0.28);
}
.brand-title { font-weight: 700; font-size: 1.05rem; color: var(--text-main); letter-spacing: -0.2px; }

.nav-links { display: flex; gap: 1.5rem; align-items: center; }
.nav-item {
    color: var(--text-muted);
    font-size: 0.9rem;
    font-weight: 500;
    text-decoration: none;
    padding: 0.4rem 0.2rem;
    position: relative;
    transition: color 0.15s ease;
}
.nav-item:hover { color: var(--primary); text-decoration: none; }
.nav-item.active { color: var(--primary); font-weight: 600; }
.nav-item.active::after {
    content: "";
    position: absolute;
    bottom: -19px;
    left: 0;
    right: 0;
    height: 2px;
    background: linear-gradient(90deg, var(--primary), var(--accent-indigo));
    border-radius: 2px 2px 0 0;
}
.nav-admin { color: #2563eb; }

.nav-user { display: flex; gap: 0.85rem; align-items: center; }
.user-greeting { font-size: 0.85rem; color: var(--text-muted); font-weight: 500; }
.logout-form { display: inline; margin: 0; }

/* Language Switcher Button */
.lang-toggle {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.28rem 0.65rem;
    border-radius: 9999px;
    background: #f1f5f9;
    color: var(--text-muted);
    font-size: 0.775rem;
    font-weight: 600;
    border: 1px solid var(--border-subtle);
    text-decoration: none;
    transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
}
.lang-toggle:hover {
    background: #ffffff;
    color: var(--primary);
    border-color: var(--primary-border);
    box-shadow: 0 2px 8px rgba(37, 99, 235, 0.15);
    text-decoration: none;
}

/* Layout & Main Content */
.main-content { max-width: 1200px; margin: 2rem auto; padding: 0 1.5rem; }
.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 1.75rem; flex-wrap: wrap; gap: 1rem; }
.page-title { font-size: 1.65rem; font-weight: 700; color: var(--text-main); margin-bottom: 0.25rem; }
.page-subtitle { color: var(--text-muted); font-size: 0.9rem; }
.header-actions { display: flex; gap: 0.75rem; align-items: center; }

/* Office Metric / Stat Cards */
.stats-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 1rem; margin-bottom: 1.75rem; }
.stat-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    padding: 1.25rem;
    display: flex;
    flex-direction: column;
    box-shadow: var(--shadow-sm);
}
.stat-title { font-size: 0.8rem; font-weight: 600; color: var(--text-sub); margin-bottom: 0.4rem; text-transform: uppercase; letter-spacing: 0.5px; }
.stat-value { font-size: 1.85rem; font-weight: 700; color: var(--text-main); }
.stat-subtitle { font-size: 0.8rem; color: var(--text-sub); margin-top: 0.25rem; }

/* Cards & Sections */
.section-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
    padding: 1.5rem;
    margin-bottom: 1.75rem;
    box-shadow: var(--shadow-sm);
}
.section-header { margin-bottom: 1.25rem; display: flex; justify-content: space-between; align-items: center; }
.section-title { font-size: 1.2rem; font-weight: 600; color: var(--text-main); }
.card-subtitle { font-size: 1.05rem; font-weight: 600; margin-bottom: 0.75rem; color: var(--text-main); }

/* Project Cards Grid */
.projects-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(350px, 1fr)); gap: 1.25rem; }
.project-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    padding: 1.25rem;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    transition: all 0.15s ease;
    box-shadow: var(--shadow-sm);
}
.project-card:hover {
    border-color: var(--primary-border);
    box-shadow: var(--shadow-md);
    transform: translateY(-1px);
}
.project-card-header { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 0.5rem; }
.project-name a { color: var(--text-main); font-size: 1.1rem; font-weight: 600; text-decoration: none; }
.project-name a:hover { color: var(--primary); }
.team-tag { display: inline-block; font-size: 0.75rem; color: var(--primary); background: var(--primary-light); padding: 2px 8px; border-radius: var(--radius-sm); border: 1px solid var(--primary-border); margin-top: 0.35rem; }
.project-desc { color: var(--text-muted); font-size: 0.875rem; margin-bottom: 1rem; line-height: 1.45; }
.project-footer { display: flex; justify-content: space-between; align-items: center; border-top: 1px solid var(--border-subtle); padding-top: 0.85rem; }
.project-meta { font-size: 0.8rem; color: var(--text-sub); display: flex; gap: 0.5rem; }
.project-actions { display: flex; gap: 0.5rem; }

/* Status Badges */
.status-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    font-size: 0.75rem;
    font-weight: 600;
    padding: 3px 9px;
    border-radius: 9999px;
    text-transform: capitalize;
}
.badge-active { background: var(--success-bg); color: var(--success); border: 1px solid var(--success-border); }
.badge-idle { background: #f1f5f9; color: var(--text-sub); border: 1px solid var(--border-subtle); }
.badge-warning { background: var(--warning-bg); color: var(--warning); border: 1px solid var(--warning-border); }
.badge-error { background: var(--danger-bg); color: var(--danger); border: 1px solid var(--danger-border); }
.status-dot { width: 6px; height: 6px; border-radius: 50%; background: currentColor; }

@keyframes pulse-live {
    0%, 100% { transform: scale(1); box-shadow: 0 0 0 0 rgba(22, 163, 74, 0.4); }
    50% { transform: scale(1.2); box-shadow: 0 0 0 5px rgba(22, 163, 74, 0); }
}
.badge-active .status-dot {
    animation: pulse-live 2s infinite ease-in-out;
}

/* Role Badges / Chips */
.role-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.25rem 0.65rem;
    border-radius: 9999px;
    font-size: 0.775rem;
    font-weight: 600;
    line-height: 1;
}
.role-badge-admin { background: #eff6ff; color: #1d4ed8; border: 1px solid #bfdbfe; }
.role-badge-lead { background: #faf5ff; color: #7e22ce; border: 1px solid #e9d5ff; }
.role-badge-editor { background: #f0fdf4; color: #15803d; border: 1px solid #bbf7d0; }
.role-badge-viewer { background: #f8fafc; color: #64748b; border: 1px solid #e2e8f0; }

/* Profile Info Cards (Non-input display) */
.profile-info-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 1.25rem;
    margin-top: 1rem;
}
.profile-field {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    background: #f8fafc;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    padding: 0.85rem 1rem;
}
.profile-label {
    font-size: 0.725rem;
    font-weight: 600;
    color: var(--text-sub);
    text-transform: uppercase;
    letter-spacing: 0.5px;
}
.profile-val {
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--text-main);
    display: flex;
    align-items: center;
    gap: 0.5rem;
}

/* Team Tree & Admin Management */
.team-tree { display: flex; flex-direction: column; gap: 0.85rem; margin-top: 1rem; }
.team-node {
    background: #ffffff;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    padding: 1.15rem 1.25rem;
    transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
}
.team-node:hover {
    border-color: #cbd5e1;
    box-shadow: var(--shadow-sm);
    transform: translateY(-1px);
}
.team-node-sub {
    margin-left: 2rem;
    border-left: 3px solid var(--primary);
    background: #fcfdfe;
}
.team-node-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 0.75rem;
}
.team-node-title {
    font-weight: 600;
    font-size: 1rem;
    color: var(--text-main);
    display: flex;
    align-items: center;
    gap: 0.5rem;
}
.team-node-slug { font-family: var(--font-mono); font-size: 0.775rem; color: var(--text-sub); }
.team-members-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: 0.85rem;
    padding-top: 0.85rem;
    border-top: 1px solid var(--border-subtle);
    flex-wrap: wrap;
    gap: 0.5rem;
}
.member-chips { display: flex; gap: 0.4rem; flex-wrap: wrap; align-items: center; }
.member-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.2rem 0.6rem;
    background: #f1f5f9;
    border-radius: 9999px;
    font-size: 0.75rem;
    color: var(--text-main);
    border: 1px solid var(--border-subtle);
}
.member-chip-remove {
    cursor: pointer;
    color: var(--text-sub);
    font-weight: 700;
    margin-left: 2px;
}
.member-chip-remove:hover { color: var(--danger); }

/* Office Buttons */
.btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-weight: 500;
    font-size: 0.875rem;
    padding: 0.45rem 0.95rem;
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s;
    text-decoration: none;
    line-height: 1.25;
}
.btn-primary { background: var(--primary); color: #fff; border-color: var(--primary); }
.btn-primary:hover { background: var(--primary-hover); text-decoration: none; }
.btn-secondary { background: #ffffff; color: var(--text-main); border-color: var(--border-strong); }
.btn-secondary:hover { background: #f8fafc; border-color: #94a3b8; text-decoration: none; }
.btn-ghost { background: transparent; color: var(--text-muted); }
.btn-ghost:hover { background: #f1f5f9; color: var(--text-main); text-decoration: none; }
.btn-danger { background: #ffffff; color: var(--danger); border-color: var(--danger-border); }
.btn-danger:hover { background: var(--danger-bg); }
.btn-sm { font-size: 0.775rem; padding: 0.3rem 0.65rem; }
.btn-lg { font-size: 0.95rem; padding: 0.65rem 1.25rem; }
.btn-block { width: 100%; }
.btn-icon { margin-right: 0.35rem; font-weight: 700; }

/* Forms & Inputs */
.form-group { margin-bottom: 1.1rem; display: flex; flex-direction: column; gap: 0.35rem; }
.form-group label { font-size: 0.825rem; font-weight: 600; color: var(--text-main); }
.form-control {
    background: #ffffff;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    padding: 0.55rem 0.8rem;
    color: var(--text-main);
    font-size: 0.9rem;
    transition: border-color 0.15s, box-shadow 0.15s;
}
.form-control:focus { outline: none; border-color: var(--primary); box-shadow: 0 0 0 3px rgba(37, 99, 235, 0.1); }
.form-control.readonly { background: #f8fafc; cursor: not-allowed; color: var(--text-muted); }
.form-row { display: grid; grid-template-columns: 1fr 1fr; gap: 1rem; }
.inline-form { display: inline-block; margin: 0; }

/* Alerts */
.alert { padding: 0.75rem 1rem; border-radius: var(--radius-md); font-size: 0.85rem; margin-bottom: 1rem; border: 1px solid transparent; }
.alert-warning { background: var(--warning-bg); border-color: var(--warning-border); color: #92400e; }
.alert-danger { background: var(--danger-bg); border-color: var(--danger-border); color: #991b1b; }
.alert-success { background: var(--success-bg); border-color: var(--success-border); color: #166534; }
.alert-info { background: var(--primary-light); border-color: var(--primary-border); color: #1e40af; }

/* Auth Pages - Clean Office Login & Registration */
.auth-page { min-height: 100vh; display: flex; align-items: center; justify-content: center; padding: 2rem; background: #f8fafc; }
.auth-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
    padding: 2.25rem;
    width: 100%;
    max-width: 440px;
    box-shadow: var(--shadow-md);
}
.auth-header { text-align: center; margin-bottom: 1.75rem; }
.auth-logo { font-size: 1.4rem; font-weight: 800; color: var(--primary); letter-spacing: 1px; margin-bottom: 0.35rem; }
.auth-title { font-size: 1.3rem; font-weight: 700; color: var(--text-main); margin-bottom: 0.35rem; }
.auth-subtitle { font-size: 0.825rem; color: var(--text-muted); }
.divider { text-align: center; margin: 1.25rem 0; position: relative; }
.divider:before { content: ""; position: absolute; left: 0; top: 50%; width: 100%; height: 1px; background: var(--border-subtle); }
.divider span { background: var(--bg-surface); padding: 0 0.75rem; color: var(--text-sub); font-size: 0.725rem; font-weight: 600; position: relative; z-index: 1; }
.auth-footer { text-align: center; margin-top: 1.25rem; font-size: 0.825rem; color: var(--text-muted); }
.hint-text { font-size: 0.75rem; color: var(--text-sub); text-align: center; margin-top: 0.35rem; }

/* Breadcrumb & Project Details */
.breadcrumb { font-size: 0.85rem; color: var(--text-sub); margin-bottom: 0.85rem; }
.breadcrumb a { color: var(--text-muted); }
.breadcrumb a:hover { color: var(--primary); }
.breadcrumb .active { color: var(--text-main); font-weight: 600; }
.title-with-badge { display: flex; align-items: center; gap: 0.85rem; flex-wrap: wrap; }
.detail-grid { display: grid; grid-template-columns: 2fr 1fr; gap: 1.5rem; }
@media (max-width: 900px) {
    .detail-grid { grid-template-columns: 1fr; }
    .form-row { grid-template-columns: 1fr; }
}

/* Tabs Navigation for Project View */
.tab-bar { display: flex; gap: 0.5rem; border-bottom: 1px solid var(--border-subtle); margin-bottom: 1.5rem; overflow-x: auto; }
.tab-item {
    padding: 0.6rem 1rem;
    font-size: 0.875rem;
    font-weight: 500;
    color: var(--text-muted);
    border-bottom: 2px solid transparent;
    text-decoration: none;
    cursor: pointer;
    white-space: nowrap;
}
.tab-item:hover { color: var(--primary); text-decoration: none; }
.tab-item.active { color: var(--primary); border-bottom-color: var(--primary); font-weight: 600; }

/* Merge & Conflict Resolution View */
.merge-panel { background: var(--bg-surface); border: 1px solid var(--border-subtle); border-radius: var(--radius-lg); padding: 1.5rem; margin-bottom: 1.5rem; box-shadow: var(--shadow-sm); }
.conflict-card { background: #ffffff; border: 1px solid var(--warning-border); border-radius: var(--radius-md); padding: 1.25rem; margin-bottom: 1.25rem; }
.conflict-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.75rem; }
.conflict-file { font-family: var(--font-mono); font-size: 0.85rem; font-weight: 600; color: #92400e; }
.diff-box {
    background: #f8fafc;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    padding: 0.85rem;
    font-family: var(--font-mono);
    font-size: 0.8rem;
    overflow-x: auto;
    line-height: 1.5;
    margin-bottom: 0.85rem;
}
.diff-local { background: rgba(22, 163, 74, 0.08); color: #166534; padding: 2px 4px; border-radius: 2px; }
.diff-remote { background: rgba(37, 99, 235, 0.08); color: #1e40af; padding: 2px 4px; border-radius: 2px; }
.diff-marker { background: #fef3c7; color: #92400e; font-weight: 700; padding: 2px 4px; }
.conflict-actions { display: flex; gap: 0.5rem; flex-wrap: wrap; }

/* Timeline & VCS Snapshots */
.timeline-list { display: flex; flex-direction: column; gap: 0.85rem; margin-top: 0.75rem; }
.timeline-item { display: flex; gap: 0.85rem; align-items: flex-start; position: relative; }
.timeline-dot { width: 10px; height: 10px; border-radius: 50%; background: var(--border-strong); margin-top: 6px; flex-shrink: 0; }
.timeline-dot.active { background: var(--primary); box-shadow: 0 0 0 3px rgba(37, 99, 235, 0.15); }
.timeline-content { background: #ffffff; border: 1px solid var(--border-subtle); border-radius: var(--radius-md); padding: 0.85rem 1rem; flex-grow: 1; box-shadow: var(--shadow-sm); }
.timeline-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.25rem; }
.timeline-msg { font-weight: 600; font-size: 0.9rem; color: var(--text-main); }
.timeline-time { font-size: 0.75rem; color: var(--text-sub); }
.timeline-meta { font-size: 0.775rem; color: var(--text-muted); display: flex; gap: 0.5rem; }

/* Info & Collaborator Lists */
.info-list { display: flex; flex-direction: column; gap: 0.65rem; }
.info-row { display: flex; justify-content: space-between; align-items: center; font-size: 0.825rem; }
.info-label { color: var(--text-muted); }
.info-val { font-weight: 500; color: var(--text-main); font-family: var(--font-mono); font-size: 0.775rem; }

.collaborator-list { display: flex; flex-direction: column; gap: 0.65rem; }
.collaborator-item { display: flex; align-items: center; gap: 0.75rem; background: #ffffff; padding: 0.65rem 0.85rem; border-radius: var(--radius-sm); border: 1px solid var(--border-subtle); }
.collab-avatar { width: 30px; height: 30px; border-radius: 50%; background: var(--primary); color: #fff; font-weight: 600; font-size: 0.75rem; display: flex; align-items: center; justify-content: center; }
.collab-info { display: flex; flex-direction: column; }
.collab-name { font-weight: 600; font-size: 0.85rem; color: var(--text-main); }
.collab-role { font-size: 0.725rem; color: var(--text-sub); text-transform: capitalize; }

/* Empty state */
.empty-state { text-align: center; padding: 3rem 1.5rem; background: #ffffff; border: 1px dashed var(--border-strong); border-radius: var(--radius-lg); }
.empty-title { font-size: 1.15rem; font-weight: 600; margin-bottom: 0.4rem; color: var(--text-main); }
.empty-desc { color: var(--text-muted); font-size: 0.875rem; margin-bottom: 1.25rem; max-width: 480px; margin-left: auto; margin-right: auto; }
"#;
