/// Global CSS stylesheet rules embedded directly in server-rendered HTML pages.
pub const EMBEDDED_CSS: &str = r#"
/* ==========================================================================================
   DESIGN TOKENS
   ------------------------------------------------------------------------------------------
   Soft & Friendly + Clean Minimal: a warm-neutral foundation (not pure white), indigo as the
   single interactive accent, teal used sparingly for positive/highlight states, 1px borders and
   very soft shadows instead of heavy glassmorphism.

   THREE ELEVATION TIERS, used consistently:
     --bg-base      the page itself
     --bg-surface   cards/panels sitting IN the document
     --bg-elevated  overlays floating ABOVE it (menus, modals, popovers) -- always opaque

   scrim. Beyond the legibility cost of translucent surfaces, any element carrying one becomes
   a containing block for `position: fixed` descendants -- which stranded modals inside cards
   and the collapse button inside the sidebar. Cards are solid with a hairline border now.
   ========================================================================================== */
:root {
    --bg-base: #f8f9fb;
    --bg-surface: #ffffff;
    --bg-surface-elevated: #ffffff;
    --bg-elevated: #ffffff;
    --bg-card: #ffffff;
    --bg-card-hover: #fcfcfd;
    --bg-muted: #f2f4f8;
    --bg-subtle: #f8f9fb;

    --border-subtle: #e7eaf0;
    --border-strong: #d7dce6;
    --border-focus: #635bff;
    --border-glass: #e7eaf0;
    --border-glass-tint: #e7eaf0;

    --text-main: #172033;
    --text-muted: #475467;
    --text-sub: #667085;
    --text-light: #98a2b3;

    /* Indigo is the interactive color: buttons, links, focus rings, active nav. Nothing else
       should be indigo, so "indigo" reliably reads as "you can act on this". */
    --primary: #635bff;
    --primary-hover: #514bd4;
    --primary-active: #443ec0;
    --primary-light: #f0efff;
    --primary-border: #d4d1ff;
    --primary-ring: rgba(99, 91, 255, 0.18);
    --primary-gradient: linear-gradient(135deg, #635bff 0%, #7c76ff 100%);

    /* Teal: positive states and highlights only, never general decoration. */
    --accent-teal: #14b8a6;
    --accent-teal-light: #e6fffb;
    --accent-cyan: #14b8a6;
    --accent-indigo: #635bff;
    --accent-violet: #7c6bff;

    --success: #22c55e;
    --success-bg: #f0fdf4;
    --success-border: #bbf7d0;
    --warning: #f59e0b;
    --warning-bg: #fffbeb;
    --warning-border: #fde68a;
    --danger: #ef4444;
    --danger-bg: #fef2f2;
    --danger-border: #fecaca;

    /* Spacing scale -- one vocabulary for every gap/padding so rhythm stays consistent. */
    --space-1: 4px;
    --space-2: 8px;
    --space-3: 12px;
    --space-4: 16px;
    --space-5: 24px;
    --space-6: 32px;
    --space-7: 48px;

    /* Consistent radii: xs for pills/tags, sm for controls, md for cards, lg for modals. */
    --radius-xs: 6px;
    --radius-sm: 8px;
    --radius-md: 12px;
    --radius-lg: 16px;
    --radius-xl: 20px;
    --radius-pill: 9999px;

    /* Very soft, neutral shadows -- no colored glow. Depth comes from the border first. */
    --shadow-sm: 0 1px 2px rgba(16, 24, 40, 0.04);
    --shadow-md: 0 2px 6px rgba(16, 24, 40, 0.05), 0 1px 2px rgba(16, 24, 40, 0.03);
    --shadow-lg: 0 8px 24px rgba(16, 24, 40, 0.08), 0 2px 6px rgba(16, 24, 40, 0.04);
    --shadow-glass: 0 2px 6px rgba(16, 24, 40, 0.05);
    --shadow-glow: 0 0 0 3px var(--primary-ring);

    --font-sans: -apple-system, BlinkMacSystemFont, "SF Pro Display", "Inter", "Segoe UI", Roboto, "PingFang SC", "Hiragino Sans GB", "Microsoft YaHei", sans-serif;
    --font-mono: "JetBrains Mono", ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;

    /* 150-200ms: quick enough to feel responsive, slow enough to read as motion. */
    --ease-out-cubic: cubic-bezier(0.16, 1, 0.3, 1);
    --transition: 160ms var(--ease-out-cubic);
}

* { box-sizing: border-box; margin: 0; padding: 0; }

/* Every `#[island]` component's real output is wrapped in a `<leptos-island data-component="...">`
   custom element in the actual DOM (not just in the server-rendered HTML -- this persists after
   hydration too). Custom elements default to `display: inline` with no layout participation of
   their own, so any island meant to render several top-level siblings that a *parent* CSS Grid or
   Flexbox container arranges (e.g. `.editor-studio-grid`'s three columns, expected to be direct
   children `.outline-panel`/`.code-panel`/`.preview-panel`) silently breaks: the grid only ever
   sees one child -- the `<leptos-island>` -- and everything inside it collapses into normal
   document flow in whatever box that one inline element ends up occupying. Confirmed live with a
   real headless-browser screenshot: the note editor's three-column layout was crushed into a
   single ~550px-wide sliver with the entire right side of the page blank. `display: contents`
   makes the wrapper itself disappear from the box/layout tree while keeping its children in the
   DOM, so they act as if they were direct children of the real parent again -- the standard fix
   for exactly this class of "transparent wrapper element breaks CSS Grid/Flexbox" problem. */
leptos-island { display: contents; }

body {
    background-color: var(--bg-base);
    background-image: 
        radial-gradient(1200px circle at 10% 8%, rgba(186, 230, 253, 0.5) 0%, transparent 50%),
        radial-gradient(1000px circle at 90% 18%, rgba(219, 234, 254, 0.6) 0%, transparent 45%),
        radial-gradient(1400px circle at 50% 92%, rgba(224, 242, 254, 0.65) 0%, transparent 60%);
    background-attachment: fixed;
    color: var(--text-main);
    font-family: var(--font-sans);
    line-height: 1.55;
    -webkit-font-smoothing: antialiased;
}

/* Animations: Breathing Glow & Pulse */
@keyframes breathe-glow {
    0%, 100% {
        box-shadow: var(--shadow-md);
    }
    50% {
        box-shadow: var(--shadow-lg);
    }
}

@keyframes pulse-live {
    0%, 100% {
        transform: scale(1);
        box-shadow: 0 0 0 0 var(--primary-ring);
    }
    50% {
        transform: scale(1.1);
        box-shadow: 0 0 0 6px rgba(99, 91, 255, 0);
    }
}

@keyframes subtle-fade-in {
    from { opacity: 0; transform: translateY(6px); }
    to { opacity: 1; transform: translateY(0); }
}

a { color: var(--primary); text-decoration: none; transition: color 0.18s var(--ease-out-cubic); }
a:hover { color: var(--primary-hover); text-decoration: underline; }

/* Persistent Left Sidebar Layout */
.app-layout {
    display: flex;
    min-height: 100vh;
    background-color: transparent;
}

/* Solid, not frosted: "visually quiet" reads better as a plain surface with a hairline edge,
   and it keeps the sidebar from becoming a containing block for fixed-position overlays (the
   bug that stranded the collapse button inside the sidebar when it slid away). No shadow --
   the 1px border is the separation. */
.app-sidebar {
    width: 260px;
    background: var(--bg-surface);
    border-right: 1px solid var(--border-subtle);
    display: flex;
    flex-direction: column;
    position: fixed;
    top: 0;
    bottom: 0;
    left: 0;
    z-index: 90;
}

.sidebar-brand {
    height: 66px;
    padding: 0 1.25rem;
    display: flex;
    align-items: center;
    border-bottom: 1px solid var(--border-subtle);
}
.sidebar-brand .brand-link {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    text-decoration: none;
    color: var(--text-main);
}
.brand-badge {
    background: var(--primary-gradient);
    color: #fff;
    font-weight: 800;
    font-size: 0.85rem;
    padding: 3px 10px;
    border-radius: var(--radius-sm);
    letter-spacing: 0.5px;
    box-shadow: var(--shadow-sm);
}
.brand-title { font-weight: 700; font-size: 1.05rem; color: var(--text-main); letter-spacing: -0.2px; }

.sidebar-body {
    flex: 1;
    overflow-y: auto;
    padding: 1.25rem 0.85rem;
    display: flex;
    flex-direction: column;
    gap: 1.4rem;
}
.sidebar-section {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
}
.sidebar-heading {
    font-size: 0.7rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.8px;
    color: var(--text-light);
    padding: 0 0.65rem 0.35rem 0.65rem;
}
.sidebar-link {
    display: flex;
    align-items: center;
    gap: 0.7rem;
    padding: 0.55rem 0.7rem;
    border-radius: var(--radius-sm);
    font-size: 0.875rem;
    font-weight: 500;
    color: var(--text-muted);
    text-decoration: none;
    transition: background var(--transition), color var(--transition);
}
/* No translateX nudge: nav items shifting under the cursor is the kind of motion that reads as
   fidgety rather than responsive, especially on a list you scan constantly. */
.sidebar-link:hover {
    background: var(--bg-muted);
    color: var(--text-main);
    text-decoration: none;
}
/* Active state via an inset left accent rather than a border, so it can't shift the row by a
   pixel relative to its inactive siblings. */
.sidebar-link.active {
    background: var(--primary-light);
    color: var(--primary);
    font-weight: 600;
    box-shadow: inset 3px 0 0 var(--primary);
}
.sidebar-icon {
    font-size: 1.15rem;
    width: 22px;
    display: inline-flex;
    justify-content: center;
}
.sidebar-footer {
    padding: 1rem 0.85rem;
    border-top: 1px solid var(--border-subtle);
    background: var(--bg-surface);
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
}
.sidebar-user-card {
    display: flex;
    align-items: center;
    gap: 0.65rem;
}
.sidebar-user-avatar {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    background: var(--primary-gradient);
    color: #fff;
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 700;
    font-size: 0.85rem;
    flex-shrink: 0;
    box-shadow: var(--shadow-sm);
}
.sidebar-user-info {
    flex: 1;
    min-width: 0;
}
.sidebar-user-name {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text-main);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}
.sidebar-user-role {
    font-size: 0.725rem;
    color: var(--text-sub);
}
.sidebar-actions {
    display: flex;
    justify-content: space-between;
    align-items: center;
}

.app-main {
    margin-left: 260px;
    flex: 1;
    min-width: 0;
    padding: 2.25rem 3rem;
    display: flex;
    flex-direction: column;
    transition: margin-left 0.2s var(--ease-out-cubic);
}

/* Collapsed sidebar: the whole aside slides out of view and the main column reclaims its
   260px, leaving only the toggle button pinned at the screen edge to bring it back. Driven by
   a class on `.app-layout` (set by SIDEBAR_TOGGLE_JS from localStorage before first paint, so
   a collapsed sidebar never flashes open on navigation). */
.app-layout.sidebar-collapsed .app-sidebar {
    transform: translateX(-260px);
}
.app-layout.sidebar-collapsed .app-main {
    margin-left: 0;
}
.app-sidebar {
    transition: transform 0.2s var(--ease-out-cubic);
}
.sidebar-toggle {
    position: fixed;
    top: 20px;
    left: 234px;
    z-index: 95;
    width: 26px;
    height: 26px;
    border-radius: 50%;
    border: 1px solid var(--border-subtle);
    background: var(--bg-surface);
    color: var(--text-sub);
    font-size: 0.8rem;
    line-height: 1;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: var(--shadow-sm);
    transition: left 0.2s var(--ease-out-cubic), color 0.15s ease;
}
.sidebar-toggle:hover {
    color: var(--primary);
}
/* Collapsed, this button is the *only* way back to the sidebar, so it gets a solid accent
   treatment rather than the subtle edge-of-sidebar look it has when expanded -- it has to read
   as an obvious control on an otherwise empty left edge. */
.app-layout.sidebar-collapsed .sidebar-toggle {
    left: 12px;
    width: 30px;
    height: 30px;
    background: var(--primary);
    border-color: var(--primary);
    color: #fff;
    font-size: 0.9rem;
    box-shadow: var(--shadow-md);
}
.app-layout.sidebar-collapsed .sidebar-toggle:hover {
    color: #fff;
    filter: brightness(1.08);
}

/* Quick-start shortcuts: real <form> POSTs styled to sit flush with the nav links around them
   (a button, not a link, because each one creates a project -- a state change that must not be
   a GET). */
.sidebar-quick-form {
    margin: 0;
}
.sidebar-quick-btn {
    width: 100%;
    background: none;
    border: none;
    font: inherit;
    text-align: left;
    cursor: pointer;
}

/* Dashboard "Quick Start" dropdown. Opens on hover and on keyboard focus (`:focus-within`),
   with no JS at all, so it works identically before and after hydration. The padding-top on the
   dropdown keeps a hover bridge across the gap to the trigger -- without it the menu closes the
   moment the pointer leaves the button. */
.quick-start-menu {
    position: relative;
    display: inline-block;
}
.quick-start-dropdown {
    display: none;
    position: absolute;
    top: 100%;
    right: 0;
    padding-top: 6px;
    z-index: 80;
    min-width: 210px;
}
.quick-start-menu:hover .quick-start-dropdown,
.quick-start-menu:focus-within .quick-start-dropdown {
    display: block;
}
/* External hub links (Chat / Meeting / Drive / AI Agent). These classes previously had no CSS
   at all, so the row rendered as four bare blue anchors. Each is a card with its own accent
   tint, a tinted icon tile, and an affordance (↗) marking it as leaving the app. */
.hub-links-bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.6rem;
    margin-bottom: 1.5rem;
}
.hub-link-chip {
    --hub-tone: var(--primary);
    --hub-tint: var(--primary-light);
    display: inline-flex;
    align-items: center;
    gap: 0.55rem;
    padding: 0.5rem 0.85rem;
    background: var(--bg-surface-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-pill);
    color: var(--text-muted);
    font-size: 0.85rem;
    font-weight: 600;
    text-decoration: none;
    box-shadow: var(--shadow-sm);
    transition: border-color var(--transition), color var(--transition),
        background var(--transition);
}
.hub-link-chip:hover {
    background: var(--hub-tint);
    border-color: var(--hub-tone);
    color: var(--hub-tone);
}
.hub-link-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: var(--hub-tint);
    font-size: 0.8rem;
    line-height: 1;
}
.hub-link-arrow {
    font-size: 0.7rem;
    color: var(--text-light);
    transition: color 0.16s ease, transform 0.16s var(--ease-out-cubic);
}
.hub-link-chip:hover .hub-link-arrow {
    color: var(--hub-tone);
    transform: translate(1px, -1px);
}
.hub-link-chip.hub-chat    { --hub-tone: #0284c7; --hub-tint: #e0f2fe; }
.hub-link-chip.hub-meeting { --hub-tone: #7c3aed; --hub-tint: #f3e8ff; }
.hub-link-chip.hub-drive   { --hub-tone: var(--accent-teal); --hub-tint: var(--accent-teal-light); }
.hub-link-chip.hub-ai      { --hub-tone: #d97706; --hub-tint: #fef3c7; }
.badge-override {
    padding: 0.2rem 0.6rem;
    border-radius: var(--radius-pill);
    background: var(--warning-bg);
    border: 1px solid var(--warning-border);
    color: var(--warning);
    font-size: 0.7rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.3px;
}

/* The panel itself carries the surface -- not the individual rows. These used to be per-<form>
   backgrounds from when each item was its own POST form; the items are plain buttons now, so
   those rules matched nothing and the menu rendered fully transparent over the page. */
.quick-start-panel {
    background: var(--bg-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-lg);
    padding: 4px;
    overflow: hidden;
}
.quick-start-item {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    width: 100%;
    padding: 0.5rem 0.75rem;
    background: none;
    border: none;
    border-radius: var(--radius-sm);
    font: inherit;
    font-size: 0.875rem;
    color: var(--text-main);
    text-align: left;
    cursor: pointer;
    transition: background 0.12s ease, color 0.12s ease;
}
.quick-start-item:hover {
    background: var(--bg-muted);
    color: var(--primary);
}

/* Language Switcher Pill */
.lang-toggle {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.3rem 0.75rem;
    border-radius: var(--radius-pill);
    background: var(--bg-surface);
    color: var(--text-muted);
    font-size: 0.775rem;
    font-weight: 600;
    border: 1px solid var(--border-subtle);
    text-decoration: none;
    transition: all var(--transition);
}
.lang-toggle:hover {
    background: #ffffff;
    color: var(--primary);
    border-color: var(--primary-border);
    box-shadow: var(--shadow-md);
    text-decoration: none;
}

/* Page Header & Actions */
.page-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1.75rem;
    flex-wrap: wrap;
    gap: 1rem;
}
.page-title {
    font-size: 1.65rem;
    font-weight: 700;
    color: var(--text-main);
    margin-bottom: 0.25rem;
    letter-spacing: -0.3px;
}
.page-subtitle {
    color: var(--text-muted);
    font-size: 0.9rem;
}
.header-actions {
    display: flex;
    gap: 0.75rem;
    align-items: center;
}
.title-with-badge {
    display: flex;
    align-items: center;
    gap: 0.75rem;
}

/* Frosted Glass Cards & Sections */
/* Border first, shadow second: a hairline edge on a solid surface separates a card from the
   warm-neutral page without the heavy drop shadow the old glass treatment needed. */
.section-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    padding: var(--space-5);
    margin-bottom: var(--space-5);
    box-shadow: var(--shadow-sm);
}
.detail-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 1.5rem;
    align-items: start;
    margin-bottom: var(--space-5);
}
@media (max-width: 1024px) {
    .detail-grid {
        grid-template-columns: 1fr;
    }
    .detail-grid > .section-card {
        grid-column: span 1 !important;
    }
}
.admin-platform-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1.5rem;
    align-items: stretch;
    margin-bottom: var(--space-5);
}
@media (max-width: 1024px) {
    .admin-platform-grid {
        grid-template-columns: 1fr;
    }
}
.settings-container,
.settings-grid,
.org-teams-container {
    max-width: 880px;
    width: 100%;
}
.settings-grid {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
}
.settings-grid .form-group input.form-control,
.settings-grid .form-group select.form-control {
    max-width: 440px;
    width: 100%;
}
.settings-grid .form-group textarea.form-control {
    max-width: 640px;
    width: 100%;
}
.section-header {
    margin-bottom: 1.25rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
}
.section-title {
    font-size: 1.18rem;
    font-weight: 600;
    color: var(--text-main);
    letter-spacing: -0.2px;
}
.card-subtitle {
    font-size: 1rem;
    font-weight: 600;
    color: var(--text-main);
}

/* Templates Grid & Cards */
.templates-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: 1.25rem;
    margin-top: 1.25rem;
    margin-bottom: 2rem;
}
.template-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    padding: 1.25rem 1.35rem;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    text-decoration: none;
    color: inherit;
    transition: border-color var(--transition), box-shadow var(--transition), transform var(--transition);
    box-shadow: var(--shadow-sm);
    min-height: 180px;
}
.template-card:hover {
    border-color: var(--primary-border);
    box-shadow: var(--shadow-md);
    transform: translateY(-2px);
    text-decoration: none;
}
.template-card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.65rem;
}
.template-card-title {
    font-size: 1.08rem;
    font-weight: 600;
    color: var(--text-main);
    margin: 0 0 0.4rem 0;
    line-height: 1.35;
    letter-spacing: -0.2px;
}
.template-card-desc {
    font-size: 0.835rem;
    color: var(--text-muted);
    line-height: 1.45;
    margin: 0 0 1rem 0;
    flex: 1;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
}
.template-card-footer {
    padding-top: 0.75rem;
    border-top: 1px solid var(--border-subtle);
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.75rem;
    color: var(--text-sub);
    margin-top: auto;
}

/* Information Lists (VCS, Git Status, Remotes) */
.info-list {
    display: flex;
    flex-direction: column;
    gap: 0.65rem;
}
.info-row {
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
    font-size: 0.875rem;
}
.info-label {
    font-weight: 600;
    color: var(--text-main);
    min-width: 190px;
    flex-shrink: 0;
}
.info-val {
    color: var(--text-muted);
}

/* Projects Grid */
.projects-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(340px, 1fr));
    gap: 1.35rem;
}
.project-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    padding: var(--space-5);
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    transition: border-color var(--transition), box-shadow var(--transition),
        background var(--transition);
    box-shadow: var(--shadow-sm);
}
.project-card:hover {
    border-color: var(--primary-border);
    box-shadow: var(--shadow-md);
}
.project-card-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 0.65rem;
}
.project-name a {
    color: var(--text-main);
    font-size: 1.12rem;
    font-weight: 600;
    text-decoration: none;
    letter-spacing: -0.2px;
}
.project-name a:hover {
    color: var(--primary);
}
.team-tag {
    display: inline-block;
    font-size: 0.725rem;
    font-weight: 500;
    color: var(--accent-cyan);
    background: rgba(224, 242, 254, 0.7);
    padding: 2px 8px;
    border-radius: var(--radius-xs);
    border: 1px solid rgba(186, 230, 253, 0.8);
    margin-top: 0.35rem;
}
.project-desc {
    color: var(--text-muted);
    font-size: 0.875rem;
    margin-bottom: 1.15rem;
    line-height: 1.5;
}
.project-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-top: 1px solid var(--border-subtle);
    padding-top: 0.95rem;
}
.project-meta {
    font-size: 0.8rem;
    color: var(--text-sub);
    display: flex;
    gap: 0.5rem;
}
.project-actions {
    display: flex;
    gap: 0.5rem;
}

/* Automated Container Status Badge */
.badge-auto-env {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    padding: 0.32rem 0.85rem;
    border-radius: var(--radius-pill);
    background: rgba(239, 246, 255, 0.85);
    border: 1px solid var(--primary-border);
    color: var(--primary-hover);
    font-size: 0.775rem;
    font-weight: 600;
}
.pulse-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--primary);
    animation: pulse-live 2s infinite var(--ease-out-cubic);
}

/* Status & Role Badges */
.status-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    font-size: 0.75rem;
    font-weight: 600;
    padding: 3px 9px;
    border-radius: var(--radius-pill);
    text-transform: capitalize;
}
.badge-active { background: var(--success-bg); color: var(--success); border: 1px solid var(--success-border); }
.badge-idle { background: rgba(241, 245, 249, 0.85); color: var(--text-sub); border: 1px solid var(--border-subtle); }
.badge-warning { background: var(--warning-bg); color: var(--warning); border: 1px solid var(--warning-border); }
.badge-error { background: var(--danger-bg); color: var(--danger); border: 1px solid var(--danger-border); }
.status-dot { width: 6px; height: 6px; border-radius: 50%; background: currentColor; }

.role-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.25rem 0.65rem;
    border-radius: var(--radius-pill);
    font-size: 0.775rem;
    font-weight: 600;
    line-height: 1;
}
.role-badge-admin { background: var(--primary-light); color: var(--primary-hover); border: 1px solid var(--primary-border); }
.role-badge-owner { background: #faf5ff; color: #7e22ce; border: 1px solid #e9d5ff; }
.role-badge-lead { background: #faf5ff; color: #7e22ce; border: 1px solid #e9d5ff; }
.role-badge-editor { background: #f0fdf4; color: #15803d; border: 1px solid #bbf7d0; }
.role-badge-viewer { background: var(--bg-subtle); color: var(--text-sub); border: 1px solid var(--border-subtle); }
.role-badge-member { background: var(--bg-subtle); color: var(--text-sub); border: 1px solid var(--border-subtle); }

/* Buttons with Smooth Bezier Elevation */
.btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    font-weight: 500;
    font-size: 0.875rem;
    padding: 0.5rem 0.95rem;
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    cursor: pointer;
    /* Only the properties that actually change -- `all` also animates layout properties, which
       is what makes hover states feel sluggish. */
    transition: background var(--transition), border-color var(--transition),
        color var(--transition), box-shadow var(--transition);
    text-decoration: none;
    line-height: 1.25;
    white-space: nowrap;
    /* Floor on the hit target: several call sites pass their own tighter padding inline, which
       was producing ~26px-tall buttons that are genuinely fiddly to hit. */
    min-height: 32px;
}
/* Keyboard focus is visible on every control, and only for keyboard users (`:focus-visible`),
   so pointer clicks don't leave a ring behind. */
.btn:focus-visible,
.form-control:focus-visible,
.sidebar-link:focus-visible,
.tab-item:focus-visible {
    outline: none;
    box-shadow: 0 0 0 3px var(--primary-ring);
}
.btn:disabled,
.btn[disabled] {
    opacity: 0.55;
    cursor: not-allowed;
}
/* Flat indigo, not a gradient: one solid interactive color is calmer and keeps the accent
   reading as "action" rather than decoration. Depth on hover comes from a darker shade, not a
   lift -- transforms on buttons make dense toolbars feel jittery. */
.btn-primary {
    background: var(--primary);
    color: #fff;
    border: 1px solid var(--primary);
}
.btn-primary:hover {
    background: var(--primary-hover);
    border-color: var(--primary-hover);
    color: #fff;
    text-decoration: none;
}
.btn-primary:active {
    background: var(--primary-active);
    border-color: var(--primary-active);
}
.btn-secondary {
    background: var(--bg-surface);
    color: var(--text-main);
    border: 1px solid var(--border-strong);
}
.btn-secondary:hover {
    background: #ffffff;
    border-color: #94a3b8;
    box-shadow: 0 2px 8px rgba(15, 23, 42, 0.05);
    text-decoration: none;
}
.btn-ghost {
    background: transparent;
    color: var(--text-muted);
}
.btn-ghost:hover {
    background: var(--bg-muted);
    color: var(--text-main);
    text-decoration: none;
}
.btn-danger {
    background: #ffffff;
    color: var(--danger);
    border: 1px solid var(--danger-border);
}
.btn-danger:hover {
    background: var(--danger-bg);
}
.btn-sm { font-size: 0.775rem; padding: 0.32rem 0.7rem; border-radius: var(--radius-xs); min-height: 30px; }
.btn-lg { font-size: 0.95rem; padding: 0.65rem 1.35rem; border-radius: var(--radius-md); }
.btn-block { width: 100%; }

/* Forms & Inputs */
.form-group { margin-bottom: 1.15rem; display: flex; flex-direction: column; gap: 0.35rem; }
.form-group label { font-size: 0.825rem; font-weight: 600; color: var(--text-main); }
.form-control {
    background: var(--bg-surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    padding: 0.55rem 0.8rem;
    color: var(--text-main);
    /* 0.9rem+ everywhere -- no tiny text in form controls. */
    font-size: 0.9rem;
    font-family: inherit;
    line-height: 1.4;
    transition: border-color var(--transition), box-shadow var(--transition);
}
.form-control::placeholder {
    color: var(--text-light);
}
.form-control:focus {
    outline: none;
    border-color: var(--primary);
    background: #ffffff;
    box-shadow: 0 0 0 3px var(--primary-ring);
}
.form-control.readonly { background: var(--bg-muted); cursor: not-allowed; color: var(--text-muted); }
select,
select.form-control {
    -webkit-appearance: none;
    -moz-appearance: none;
    appearance: none;
    cursor: pointer;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='16' height='16' viewBox='0 0 24 24' fill='none' stroke='%2364748b' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='m6 9 6 6 6-6'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 0.75rem center;
    background-size: 14px 14px;
    padding-right: 2.25rem;
}
select::-ms-expand {
    display: none;
}
/* Every use of this is an "input + submit button" row (branch create, merge, milestone, git
   remote, ignore rule, knowledge search). The columns used to be `1fr 1fr`, which on a wide
   screen gave the *button* half the card -- a huge stretched button marooned at the right edge,
   far from the field it submits. The button now hugs its own content, and the row is capped at a
   readable measure instead of spanning the full width of an ultrawide display. */
/* Flex, not a fixed grid: these rows have two OR three children (Milestones and Add Remote
   both have two fields plus the button), and a two-column grid wrapped the third onto its own
   line -- which is what put a gap between the fields and stranded the button below them.
   Flex sizes each child by its own content, so the same rule serves every arity. */
.form-row {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: var(--space-3);
    max-width: 720px;
}
/* Fields share the free space; a field with an explicit inline width keeps it. */
.form-row > .form-group {
    flex: 1 1 180px;
    min-width: 0;
    margin-bottom: 0;
}
/* The submit button hugs its own label instead of being stretched by the row. */
.form-row > .btn,
.form-row > button {
    flex: 0 0 auto;
}
@media (max-width: 640px) {
    .form-row > .form-group { flex-basis: 100%; }
    .form-row > .btn,
    .form-row > button { width: 100%; }
}
.inline-form { display: inline-block; margin: 0; }

/* Alerts */
.alert {
    padding: 0.75rem 1.1rem;
    border-radius: var(--radius-md);
    font-size: 0.85rem;
    margin-bottom: 1.25rem;
    border: 1px solid transparent;
}
.alert-warning { background: var(--warning-bg); border-color: var(--warning-border); color: #92400e; }
.alert-danger { background: var(--danger-bg); border-color: var(--danger-border); color: #991b1b; }
.alert-success { background: var(--success-bg); border-color: var(--success-border); color: #15803d; box-shadow: inset 3px 0 0 var(--accent-teal); }
.alert-info { background: var(--primary-light); border-color: var(--primary-border); color: #1e40af; }
.pat-token-banner {
    background: var(--success-bg);
    border: 1px solid var(--success-border);
    box-shadow: inset 4px 0 0 var(--accent-teal);
    border-radius: var(--radius-md);
    padding: 1.1rem 1.25rem;
    margin-bottom: 1.5rem;
}

/* Auth Pages - Tabbed Login & Registration */
.auth-page {
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 2rem;
    background: transparent;
}
.auth-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-glass);
    border-radius: var(--radius-xl);
    padding: 2.5rem;
    width: 100%;
    max-width: 460px;
    box-shadow: var(--shadow-lg);
    animation: breathe-glow 6s infinite ease-in-out;
}
.auth-header { text-align: center; margin-bottom: 1.5rem; }
.auth-logo {
    font-size: 1.6rem;
    font-weight: 800;
    color: var(--primary);
    letter-spacing: 1.5px;
    margin-bottom: 0.35rem;
    background: var(--primary-gradient);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
}
.auth-title { font-size: 1.35rem; font-weight: 700; color: var(--text-main); margin-bottom: 0.35rem; }
.auth-subtitle { font-size: 0.825rem; color: var(--text-muted); }

.auth-tabs {
    display: flex;
    border-bottom: 1px solid var(--border-subtle);
    margin-bottom: 1.5rem;
    gap: 0.5rem;
}
.auth-tab {
    flex: 1;
    text-align: center;
    padding: 0.7rem 0.5rem;
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text-muted);
    border-bottom: 2px solid transparent;
    cursor: pointer;
    background: none;
    border-top: none;
    border-left: none;
    border-right: none;
    transition: all var(--transition);
}
.auth-tab:hover { color: var(--primary); }
.auth-tab.active { color: var(--primary); border-bottom-color: var(--primary); font-weight: 700; }

.passkey-box {
    text-align: center;
    padding: 1.85rem 1rem;
    background: rgba(248, 250, 252, 0.8);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    margin-bottom: 1.25rem;
}
.passkey-icon-circle {
    width: 64px;
    height: 64px;
    margin: 0 auto 1.15rem;
    border-radius: 50%;
    background: var(--primary-light);
    color: var(--primary);
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: var(--shadow-md);
}

.auth-footer { text-align: center; margin-top: 1.35rem; font-size: 0.825rem; color: var(--text-muted); }
.hint-text { font-size: 0.75rem; color: var(--text-sub); text-align: center; margin-top: 0.5rem; }

/* Expandable Org Registration Box */
.org-expand-box {
    background: rgba(240, 249, 255, 0.6);
    border: 1px dashed var(--primary-border);
    border-radius: var(--radius-md);
    padding: 1.1rem;
    margin-top: 0.75rem;
    margin-bottom: 1.1rem;
    animation: subtle-fade-in 0.2s var(--ease-out-cubic);
}

/* Tabs Navigation for Project View */
.tab-bar {
    display: flex;
    gap: 0.5rem;
    border-bottom: 1px solid var(--border-subtle);
    margin-bottom: 1.6rem;
    overflow-x: auto;
}
.tab-item {
    padding: 0.65rem 1.15rem;
    font-size: 0.875rem;
    font-weight: 500;
    color: var(--text-muted);
    border-bottom: 2px solid transparent;
    text-decoration: none;
    cursor: pointer;
    white-space: nowrap;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    transition: all var(--transition);
}
.tab-item:hover { color: var(--primary); text-decoration: none; }
.tab-item.active { color: var(--primary); border-bottom-color: var(--primary); font-weight: 600; }

/* File Explorer */
.file-explorer-toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1.1rem;
    flex-wrap: wrap;
    gap: 0.75rem;
}
.file-table-wrap {
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    overflow: hidden;
    box-shadow: var(--shadow-sm);
}
.file-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.875rem;
}
.file-table th {
    background: rgba(248, 250, 252, 0.9);
    padding: 0.75rem 1rem;
    text-align: left;
    font-weight: 600;
    color: var(--text-muted);
    border-bottom: 1px solid var(--border-subtle);
    font-size: 0.775rem;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}
.file-table td {
    padding: 0.8rem 1rem;
    border-bottom: 1px solid var(--border-subtle);
    vertical-align: middle;
}
.file-table tr:hover td {
    background: rgba(241, 245, 249, 0.5);
}
.file-name-cell {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    font-weight: 500;
}
.file-name-cell a {
    color: var(--text-main);
    text-decoration: none;
}
.file-name-cell a:hover {
    color: var(--primary);
    text-decoration: underline;
}

.file-table th.sortable-th {
    cursor: pointer;
    user-select: none;
    transition: background 0.15s ease, color 0.15s ease;
}
.file-table th.sortable-th:hover {
    background: rgba(226, 232, 240, 0.7);
    color: var(--text-main);
}
.sort-icon {
    display: inline-block;
    margin-left: 4px;
    font-size: 0.72rem;
    opacity: 0.45;
    transition: opacity 0.15s ease, color 0.15s ease;
}
.file-table th.sorted-asc .sort-icon,
.file-table th.sorted-desc .sort-icon {
    opacity: 1;
    color: var(--primary);
    font-weight: 700;
}
.file-table th.sortable-th:hover .sort-icon {
    opacity: 0.85;
}

.file-more-wrap {
    position: relative;
    display: inline-flex;
    align-items: center;
}
.file-more-btn {
    opacity: 0.55;
    background: transparent;
    border: none;
    border-radius: var(--radius-xs);
    cursor: pointer;
    padding: 1px 5px;
    font-size: 1rem;
    font-weight: 700;
    line-height: 1;
    color: var(--text-muted);
    transition: opacity 0.15s ease, background 0.15s ease, color 0.15s ease;
    display: inline-flex;
    align-items: center;
    justify-content: center;
}
.file-name-cell:hover .file-more-btn,
.project-card:hover .file-more-btn,
.file-more-btn:focus,
.file-more-btn.is-active {
    opacity: 1;
}
.file-more-btn:hover,
.file-more-btn.is-active {
    background: var(--bg-muted, rgba(148, 163, 184, 0.2));
    color: var(--text-main);
}

.file-action-menu {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    z-index: 1050;
    min-width: 210px;
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.25), 0 8px 10px -6px rgba(0, 0, 0, 0.15);
    backdrop-filter: blur(14px);
    padding: 0.35rem 0;
    display: flex;
    flex-direction: column;
    animation: fadeIn 0.12s ease-out;
}
.file-action-item {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    padding: 0.48rem 0.9rem;
    color: var(--text-main);
    font-size: 0.825rem;
    text-decoration: none;
    background: transparent;
    border: none;
    width: 100%;
    text-align: left;
    cursor: pointer;
    transition: background 0.12s ease, color 0.12s ease;
}
.file-action-item .action-icon {
    font-size: 0.95rem;
    width: 18px;
    text-align: center;
    flex-shrink: 0;
}
.file-action-item:hover {
    background: var(--primary-light, rgba(99, 102, 241, 0.12));
    color: var(--primary);
}
.file-action-item.item-danger {
    color: var(--danger, #ef4444);
}
.file-action-item.item-danger:hover {
    background: rgba(239, 68, 68, 0.1);
    color: #dc2626;
}
.file-action-divider {
    height: 1px;
    background: var(--border-subtle);
    margin: 0.3rem 0;
}

.file-type-pill {
    display: inline-block;
    padding: 2px 7px;
    border-radius: var(--radius-xs);
    font-size: 0.7rem;
    font-weight: 600;
}
.pill-slide,
.pill-slides { background: #fdf2f8; color: #db2777; border: 1px solid #fbcfe8; }
.pill-kanban { background: #eff6ff; color: #2563eb; border: 1px solid #bfdbfe; }
.pill-typst { background: var(--primary-light); color: var(--primary); border: 1px solid var(--primary-border); }
.pill-latex { background: #f0fdf4; color: #16a34a; border: 1px solid #bbf7d0; }
.pill-table { background: #ecfdf5; color: #059669; border: 1px solid #a7f3d0; }
.pill-note { background: #fefce8; color: #ca8a04; border: 1px solid #fef08a; }
.pill-code { background: #f5f3ff; color: #7c3aed; border: 1px solid #ddd6fe; }
.pill-asset { background: var(--bg-subtle); color: var(--text-sub); border: 1px solid var(--border-subtle); }

/* Modals */
.modal-backdrop {
    display: none;
    position: fixed;
    inset: 0;
    background: rgba(15, 23, 42, 0.45);
    backdrop-filter: blur(4px);
    -webkit-backdrop-filter: blur(4px);
    align-items: center;
    justify-content: center;
    z-index: 999;
}
.modal-card {
    background: var(--bg-surface);
    border-radius: var(--radius-lg);
    border: 1px solid var(--border-glass);
    padding: 2rem;
    width: 100%;
    max-width: 520px;
    /* A dialog taller than the viewport (the "+ New File" form, with its name/folder/type/
       template fields, gets close on a short window) scrolls within itself rather than
       overflowing past the top and bottom edges with no way to reach either end. */
    max-height: 90vh;
    overflow-y: auto;
    box-shadow: var(--shadow-lg);
    animation: subtle-fade-in 0.2s var(--ease-out-cubic);
}
.modal-card.modal-wide,
.modal-card:has(.modal-wide) {
    max-width: 860px;
    width: min(860px, 95vw);
}
.modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1.25rem;
}
.modal-title { font-size: 1.2rem; font-weight: 700; color: var(--text-main); }
.modal-close { background: none; border: none; font-size: 1.5rem; cursor: pointer; color: var(--text-sub); }
.modal-close:hover { color: var(--text-main); }

/* Attach & Upload Modal */
.attach-modal-backdrop {
    z-index: 1000;
}
.attach-modal-card {
    max-width: 740px;
    width: min(740px, 95vw);
    max-height: 88vh;
    height: min(670px, 88vh);
    display: flex;
    flex-direction: column;
    padding: 1.6rem 2rem;
    border-radius: 16px;
    box-shadow: 0 25px 60px -12px rgba(0, 0, 0, 0.3);
    border: 1px solid var(--border-glass, var(--border-subtle));
    overflow: hidden;
}
.attach-modal-header {
    flex-shrink: 0;
    margin-bottom: 1rem;
}
.attach-modal-tabs {
    display: flex;
    gap: 0.6rem;
    border-bottom: 1.5px solid var(--border-subtle);
    padding-bottom: 0.85rem;
    margin-bottom: 1.15rem;
    flex-shrink: 0;
}
.attach-modal-tab {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.55rem 1.1rem;
    font-size: 0.92rem;
    font-weight: 500;
    border-radius: 10px;
    border: 1px solid transparent;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.15s ease;
}
.attach-modal-tab:hover {
    background: var(--bg-hover);
    color: var(--text-main);
}
.attach-modal-tab.active {
    background: var(--bg-subtle);
    color: var(--primary);
    border-color: var(--border-subtle);
    font-weight: 600;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.04);
}
.attach-badge {
    display: inline-block;
    padding: 0.15rem 0.55rem;
    font-size: 0.75rem;
    font-weight: 600;
    border-radius: 999px;
    background: var(--bg-hover);
    color: var(--text-sub);
}
.attach-tab-pane {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
}
.attach-filter-bar {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    margin-bottom: 0.95rem;
    flex-shrink: 0;
}
.attach-search-row {
    display: flex;
    gap: 0.75rem;
    align-items: center;
    width: 100%;
}
.attach-search-wrap {
    flex: 1;
    position: relative;
    display: flex;
    align-items: center;
}
.attach-search-icon {
    position: absolute;
    left: 1rem;
    font-size: 1.15rem;
    pointer-events: none;
    opacity: 0.6;
}
.attach-modern-input {
    width: 100%;
    height: 46px;
    padding: 0.65rem 1.15rem;
    font-size: 0.95rem;
    color: var(--text-main);
    background: var(--bg-surface);
    border: 1.5px solid var(--border-subtle);
    border-radius: 12px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.04);
    outline: none;
    transition: border-color 0.2s ease, box-shadow 0.2s ease, background 0.2s ease;
}
.attach-modern-input:focus {
    border-color: var(--primary);
    box-shadow: 0 0 0 3.5px var(--primary-ring, rgba(99, 91, 255, 0.2));
}
.attach-modern-input::placeholder {
    color: var(--text-muted);
    opacity: 0.85;
}
.attach-search-input {
    padding-left: 2.85rem;
    padding-right: 2.5rem;
}
.attach-search-clear {
    position: absolute;
    right: 0.75rem;
    background: var(--bg-subtle);
    border: 1px solid var(--border-subtle);
    width: 24px;
    height: 24px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 0.95rem;
    cursor: pointer;
    color: var(--text-muted);
    padding: 0;
    transition: all 0.15s ease;
}
.attach-search-clear:hover {
    background: var(--bg-hover);
    color: var(--text-main);
}
.attach-dropdown-wrap {
    min-width: 230px;
    flex-shrink: 0;
}
.attach-modern-select {
    width: 100%;
    height: 46px;
    padding: 0.65rem 2.5rem 0.65rem 1.15rem;
    font-size: 0.95rem;
    font-weight: 500;
    color: var(--text-main);
    background-color: var(--bg-surface);
    border: 1.5px solid var(--border-subtle);
    border-radius: 12px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.04);
    cursor: pointer;
    outline: none;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' fill='none' viewBox='0 0 24 24' stroke='%2364748b' stroke-width='2.5'%3E%3Cpath stroke-linecap='round' stroke-linejoin='round' d='M19 9l-7 7-7-7'%3E%3C/path%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 1rem center;
    background-size: 16px;
    appearance: none;
    -webkit-appearance: none;
    transition: border-color 0.2s ease, box-shadow 0.2s ease;
}
.attach-modern-select:focus {
    border-color: var(--primary);
    box-shadow: 0 0 0 3.5px var(--primary-ring, rgba(99, 91, 255, 0.2));
}
.attach-category-pills {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
}
.attach-pill {
    padding: 0.3rem 0.75rem;
    font-size: 0.8rem;
    font-weight: 500;
    border-radius: 999px;
    border: 1px solid var(--border-subtle);
    background: var(--bg-subtle);
    color: var(--text-sub);
    cursor: pointer;
    transition: all 0.15s ease;
}
.attach-pill:hover {
    background: var(--bg-hover);
    color: var(--text-main);
}
.attach-pill.active {
    background: var(--primary);
    color: #fff;
    border-color: var(--primary);
    box-shadow: 0 2px 8px rgba(99, 91, 255, 0.3);
}
.attach-file-list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    border: 1.5px solid var(--border-subtle);
    border-radius: 12px;
    padding: 0.6rem;
    background: var(--bg-subtle);
}
.attach-items-grid {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
}
.attach-file-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.65rem 0.85rem;
    border-radius: 10px;
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    transition: all 0.15s ease;
}
.attach-file-item:hover {
    border-color: var(--primary);
    box-shadow: 0 3px 10px rgba(0, 0, 0, 0.06);
}
.attach-file-icon {
    font-size: 1.45rem;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2.2rem;
    flex-shrink: 0;
}
.attach-file-info {
    flex: 1;
    min-width: 0;
}
.attach-file-name {
    font-size: 0.9rem;
    font-weight: 600;
    color: var(--text-main);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}
.attach-file-sub {
    display: flex;
    gap: 0.3rem;
    font-size: 0.78rem;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    margin-top: 0.15rem;
}
.attach-file-path {
    font-family: var(--font-mono, monospace);
}
.attach-file-actions {
    display: flex;
    gap: 0.4rem;
    flex-shrink: 0;
}
.btn-xs {
    padding: 0.25rem 0.65rem;
    font-size: 0.78rem;
    border-radius: 7px;
    font-weight: 500;
}
.attach-empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 3rem 1.5rem;
    text-align: center;
    color: var(--text-muted);
}
.attach-upload-form {
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 0.25rem 0.1rem;
}
.attach-dropzone {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    border: 2.5px dashed var(--border-subtle);
    border-radius: 16px;
    padding: 2.25rem 1.5rem;
    text-align: center;
    background: var(--bg-subtle);
    cursor: pointer;
    transition: all 0.2s ease;
}
.attach-dropzone:hover {
    border-color: var(--primary);
    background: var(--bg-hover);
}
.attach-dropzone-icon {
    font-size: 2.8rem;
    margin-bottom: 0.5rem;
    line-height: 1;
    transition: transform 0.2s ease;
}
.attach-dropzone:hover .attach-dropzone-icon {
    transform: scale(1.1);
}
.attach-dropzone-title {
    font-weight: 600;
    font-size: 1.05rem;
    color: var(--text-main);
    margin-bottom: 0.35rem;
}
.attach-dropzone-sub {
    font-size: 0.85rem;
    color: var(--text-muted);
    margin-bottom: 1.2rem;
    max-width: 440px;
    line-height: 1.4;
}
.attach-browse-btn {
    height: 38px;
    padding: 0.45rem 1.25rem;
    font-size: 0.88rem;
    font-weight: 600;
    border-radius: 999px;
}
.attach-selected-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.85rem 1.25rem;
    background: var(--bg-surface);
    border: 1.5px solid var(--primary);
    border-radius: 12px;
    box-shadow: 0 4px 14px rgba(99, 91, 255, 0.1);
    animation: subtle-fade-in 0.2s ease-out;
}
.attach-selected-name {
    font-weight: 600;
    font-size: 0.95rem;
    color: var(--text-main);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}
.attach-selected-size {
    font-size: 0.8rem;
    color: var(--text-muted);
    margin-top: 0.15rem;
}
.attach-form-group {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
}
.attach-form-label {
    font-size: 0.92rem;
    font-weight: 600;
    color: var(--text-main);
    display: flex;
    align-items: center;
    gap: 0.35rem;
}
.attach-form-sublabel {
    font-size: 0.82rem;
    font-weight: 500;
    color: var(--text-muted);
    margin-bottom: 0.25rem;
    display: block;
}
.attach-folder-row {
    width: 100%;
}
.attach-custom-folder-row {
    margin-top: 0.5rem;
    animation: subtle-fade-in 0.15s ease-out;
}
.attach-status-banner {
    padding: 0.75rem 1rem;
    border-radius: 10px;
    font-size: 0.88rem;
    font-weight: 500;
}
.attach-status-banner.success {
    background: rgba(34, 197, 94, 0.12);
    color: var(--success, #16a34a);
    border: 1px solid rgba(34, 197, 94, 0.25);
}
.attach-status-banner.error {
    background: rgba(239, 68, 68, 0.12);
    color: var(--danger, #dc2626);
    border: 1px solid rgba(239, 68, 68, 0.25);
}
.attach-upload-footer {
    display: flex;
    justify-content: flex-end;
    margin-top: auto;
    padding-top: 0.75rem;
}
.attach-submit-btn {
    height: 44px;
    padding: 0 1.75rem;
    font-size: 0.95rem;
    font-weight: 600;
    border-radius: 10px;
    box-shadow: 0 4px 12px rgba(99, 91, 255, 0.25);
}

/* Document & Slide Editor Studio */
.editor-studio-wrap {
    display: flex;
    flex-direction: column;
    height: calc(100vh - 120px);
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
    overflow: hidden;
    box-shadow: var(--shadow-md);
}
.editor-top-bar {
    min-height: 52px;
    background: var(--bg-surface);
    border-bottom: 1px solid var(--border-subtle);
    padding: var(--space-2) var(--space-4);
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--space-4);
    flex-wrap: wrap;
}
.editor-top-left,
.editor-top-right {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
}
/* Back button reads as navigation, not another tool: it keeps the secondary button chrome but
   picks up the accent on hover, and its arrow nudges left to reinforce the direction. */
.editor-back-btn {
    font-weight: 600;
}
.editor-back-btn:hover {
    border-color: var(--primary);
    color: var(--primary);
}
.editor-back-btn span:first-child {
    transition: transform var(--transition);
}
.editor-back-btn:hover span:first-child {
    transform: translateX(-2px);
}
/* Hairline group separator -- cheaper visually than boxing each toolbar cluster. */
.editor-top-divider {
    width: 1px;
    align-self: stretch;
    margin: var(--space-1) var(--space-2);
    background: var(--border-subtle);
}
.editor-file-name {
    font-weight: 600;
    font-size: 0.9rem;
    color: var(--text-main);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 40ch;
}
.editor-studio-grid {
    display: grid;
    grid-template-columns: 240px 1fr 1fr;
    flex: 1;
    overflow: hidden;
}
.outline-panel {
    background: var(--bg-subtle);
    border-right: 1px solid var(--border-subtle);
    overflow-y: auto;
    padding: 1rem;
}
.code-panel {
    border-right: 1px solid var(--border-subtle);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: #ffffff;
}
.preview-panel {
    background: rgba(241, 245, 249, 0.5);
    overflow-y: auto;
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
}
/* Note editor formatting toolbar (note_editor.rs's ToolbarAction buttons) -- gives the note
   editor a "somehow more WYSIWYG a bit" way to format text (click Bold instead of typing `**`)
   without turning it into a hidden-document-model rich text editor; the underlying textarea
   stays plain markdown. */
.note-toolbar {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    flex-wrap: wrap;
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid var(--border-subtle);
    background: rgba(248, 250, 252, 0.7);
}
.note-toolbar-sep {
    width: 1px;
    height: 1.1rem;
    background: var(--border-subtle);
    margin: 0 0.15rem;
}
.code-textarea {
    flex: 1;
    width: 100%;
    padding: 1.25rem;
    border: none;
    outline: none;
    font-family: var(--font-mono);
    font-size: 0.88rem;
    line-height: 1.6;
    color: #0f172a;
    background: #ffffff;
    resize: none;
}

/* Syntax-highlighted code editor: a `.code-editor-wrap` holds two exactly-overlapping layers --
   the real `<textarea>` on top with its own glyphs made invisible (only its caret shows), and a
   `<pre><code>` behind it showing Prism-tokenized colored text (see apich-islands'
   `code_highlight.rs`). Padding/font here must stay identical to `.code-textarea` above, or the
   highlighted text drifts out of alignment with what the invisible textarea is actually doing --
   a real, live-reproduced bug: `Prism.highlightElement()` stamps a `language-*` class onto both
   the `<code>` AND its `<pre>` parent, and Prism's own theme CSS targets exactly that
   `pre[class*="language-"]` pattern with its own padding/margin/font at higher specificity than a
   plain `.code-highlight-overlay` class selector, silently overriding this block's padding/font
   and breaking pixel alignment (clicks landing on the wrong character, jump-to-line selections
   appearing shifted from the text they were selecting). Fixed two ways: `code_highlight.rs` no
   longer loads Prism's theme CSS at all (token colors are hand-written below instead, scoped
   under `.code-highlight-overlay` so nothing else can match them either), and every selector here
   is qualified with `.code-editor-wrap` to out-specificity `pre[class*="language-"]` /
   `code[class*="language-"]` even if a future change ever reintroduces a rule like that.

   A second, separate alignment bug (also live-reproduced): a `<textarea>` is a native, browser-
   rendered widget with its *own* internal line-wrapping algorithm, controlled by the HTML `wrap`
   attribute -- it does not actually run the CSS `white-space`/`word-wrap` text-layout algorithm a
   `<pre>` uses, even though both elements *accept* those properties without erroring. On any line
   too long to fit unbroken, the two engines can choose different wrap points, so the overlay and
   the real textarea silently drift out of line-for-line sync the moment such a line appears
   earlier in the document -- confirmed live: clicking at the visual position of line 15 landed
   the edit on the textarea's real line 14, and by line 30 the drift had grown to 5 lines, purely
   from how many long lines happened to wrap differently before that point. Real code editors
   don't soft-wrap long lines for exactly this class of reason (VS Code, CodeMirror, Monaco all
   horizontal-scroll instead); doing the same here removes the wrapping decision from both engines
   entirely rather than trying to keep two different wrap algorithms in sync. The textarea's own
   `wrap="off"` HTML attribute is set at each call site (`document_editor.rs`/`note_editor.rs`). */
.code-editor-wrap {
    position: relative;
    flex: 1;
    min-height: 0;
    overflow: hidden;
}
.code-editor-wrap .code-textarea {
    position: absolute;
    inset: 0;
    background: transparent;
    color: transparent;
    caret-color: #0f172a;
    -webkit-text-fill-color: transparent;
    z-index: 2;
}
.code-editor-wrap .code-highlight-overlay {
    position: absolute;
    inset: 0;
    margin: 0;
    padding: 1.25rem;
    font-family: var(--font-mono);
    font-size: 0.88rem;
    line-height: 1.6;
    white-space: pre;
    overflow: auto;
    pointer-events: none;
    background: #ffffff;
    color: #0f172a;
    z-index: 1;
}
.code-editor-wrap .code-highlight-overlay code {
    font-family: inherit;
    font-size: inherit;
    line-height: inherit;
    white-space: inherit;
    background: none;
    text-shadow: none;
}
/* Hand-written token palette (replaces Prism's own theme CSS, deliberately not loaded -- see the
   comment above and `code_highlight.rs`). Scoped under `.code-highlight-overlay` so it can only
   ever affect this overlay, never anything else Prism's generic `.token.*` classes might appear
   in. Includes Typst's alias from `code_highlight.rs`'s hand-written grammar (`heading` ->
   `title`) alongside Prism's own standard token names. */
.code-highlight-overlay .token.comment,
.code-highlight-overlay .token.prolog,
.code-highlight-overlay .token.doctype,
.code-highlight-overlay .token.cdata {
    color: var(--text-sub);
    font-style: italic;
}
.code-highlight-overlay .token.punctuation {
    color: var(--text-sub);
}
.code-highlight-overlay .token.namespace {
    opacity: 0.7;
}
.code-highlight-overlay .token.property,
.code-highlight-overlay .token.tag,
.code-highlight-overlay .token.boolean,
.code-highlight-overlay .token.number,
.code-highlight-overlay .token.constant,
.code-highlight-overlay .token.symbol,
.code-highlight-overlay .token.deleted {
    color: var(--danger);
}
.code-highlight-overlay .token.selector,
.code-highlight-overlay .token.attr-name,
.code-highlight-overlay .token.string,
.code-highlight-overlay .token.char,
.code-highlight-overlay .token.builtin,
.code-highlight-overlay .token.inserted {
    color: var(--success);
}
.code-highlight-overlay .token.operator,
.code-highlight-overlay .token.entity,
.code-highlight-overlay .token.url {
    color: var(--warning);
}
.code-highlight-overlay .token.atrule,
.code-highlight-overlay .token.attr-value,
.code-highlight-overlay .token.keyword {
    color: var(--primary);
    font-weight: 600;
}
.code-highlight-overlay .token.function,
.code-highlight-overlay .token.class-name {
    color: var(--accent-violet);
}
.code-highlight-overlay .token.regex,
.code-highlight-overlay .token.important,
.code-highlight-overlay .token.variable {
    color: #b45309;
}
.code-highlight-overlay .token.title {
    color: var(--primary);
    font-weight: 700;
}
.code-highlight-overlay .token.bold {
    font-weight: 700;
}
.code-highlight-overlay .token.italic {
    font-style: italic;
}
.outline-heading-item {
    display: block;
    padding: 0.35rem 0.5rem;
    border-radius: var(--radius-xs);
    font-size: 0.825rem;
    color: var(--text-muted);
    text-decoration: none;
    cursor: pointer;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    transition: background 0.15s;
}
.outline-heading-item:hover {
    background: var(--primary-light);
    color: var(--primary);
}
.outline-heading-h1 { font-weight: 700; color: var(--text-main); }
.outline-heading-h2 { padding-left: 1rem; font-size: 0.8rem; }
.outline-heading-h3 { padding-left: 1.5rem; font-size: 0.775rem; color: var(--text-sub); }

/* Foldable outline tree (native <details>/<summary> -- the fold/unfold interaction needs no JS
   and works the instant the page renders, not after WASM hydration). `.outline-heading-item` on
   the <summary> already gives it the same look/hover as a standalone top-level entry; this only
   adds the disclosure marker and the children's collapsible container. */
.outline-group > summary {
    list-style: none;
}
.outline-group > summary::-webkit-details-marker {
    display: none;
}
.outline-group > summary::before {
    content: "▸";
    display: inline-block;
    width: 0.9em;
    margin-right: 0.15rem;
    color: var(--text-sub);
    transition: transform 0.15s var(--ease-out-cubic);
}
.outline-group[open] > summary::before {
    transform: rotate(90deg);
}
.outline-children {
    display: flex;
    flex-direction: column;
}

/* Slide Presentation Component */
.slide-presentation-card {
    background: #0f172a;
    color: #f8fafc;
    border-radius: var(--radius-md);
    aspect-ratio: 16 / 9;
    padding: 2rem 2.5rem;
    display: flex;
    flex-direction: column;
    justify-content: center;
    box-shadow: var(--shadow-lg);
    position: relative;
    overflow: hidden;
}
.slide-nav-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    padding: 0.4rem 0.8rem;
    margin-bottom: 0.75rem;
}

/* SPREADSHEET TABLE APP */
.spreadsheet-studio-wrap {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    margin-bottom: 1.75rem;
}
.spreadsheet-layout {
    display: grid;
    grid-template-columns: 2fr 1fr;
    gap: 1.25rem;
    align-items: flex-start;
}
.spreadsheet-card {
    background: var(--bg-card);
    border: 1px solid var(--border-glass);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-md);
    overflow: hidden;
}
.spreadsheet-toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.65rem 1rem;
    background: rgba(248, 250, 252, 0.95);
    border-bottom: 1px solid var(--border-subtle);
    flex-wrap: wrap;
    gap: 0.5rem;
}
.formula-bar-container {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.45rem 1rem;
    background: #ffffff;
    border-bottom: 1px solid var(--border-subtle);
}
.formula-name-box {
    width: 60px;
    font-weight: 700;
    font-family: var(--font-mono);
    font-size: 0.85rem;
    text-align: center;
    color: var(--primary);
    background: var(--primary-light);
    border: 1px solid var(--primary-border);
    border-radius: var(--radius-xs);
    padding: 3px 6px;
}
.formula-fx {
    font-weight: 800;
    font-style: italic;
    font-family: serif;
    color: var(--text-sub);
}
.formula-input {
    flex: 1;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-xs);
    padding: 0.35rem 0.65rem;
    font-family: var(--font-mono);
    font-size: 0.85rem;
    color: var(--text-main);
}
.formula-input:focus {
    outline: none;
    border-color: var(--primary);
}
/* Spreadsheet grid -- deliberately styled to read as a polished spreadsheet (Google
   Sheets/Excel-like: muted grid lines, zebra striping, right-aligned numbers, a header that
   states the column name first and its SQL type second and quietly) rather than a raw database
   query result grid, even though every table here really is backed by a physical SQLite file --
   that fact doesn't need to dominate the visual design of the thing people actually read and
   edit day to day. */
.spreadsheet-grid-wrap {
    overflow-x: auto;
    background: #ffffff;
    max-height: 480px;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
}
.spreadsheet-grid {
    width: 100%;
    border-collapse: separate;
    border-spacing: 0;
    font-size: 0.85rem;
    font-family: var(--font-sans);
}
.spreadsheet-grid th, .spreadsheet-grid td {
    border-bottom: 1px solid var(--border-subtle);
    border-right: 1px solid rgba(226, 232, 240, 0.6);
    padding: 0.5rem 0.75rem;
    height: 34px;
}
.spreadsheet-grid th:last-child, .spreadsheet-grid td:last-child {
    border-right: none;
}
.spreadsheet-grid th {
    background: var(--bg-subtle);
    text-align: left;
    position: sticky;
    top: 0;
    z-index: 10;
    border-bottom: 2px solid var(--border-subtle);
}
.spreadsheet-grid .col-letter {
    font-size: 0.7rem;
    color: var(--text-sub);
    font-weight: 500;
}
.spreadsheet-grid .col-name {
    font-weight: 600;
    color: var(--text-main);
    white-space: nowrap;
}
.spreadsheet-grid .col-type {
    font-size: 0.68rem;
    color: var(--text-sub);
    font-weight: 400;
    text-transform: lowercase;
    margin-top: 1px;
}
.spreadsheet-grid .col-pk-badge {
    font-size: 0.62rem;
    background: var(--primary-light);
    color: var(--primary);
    padding: 1px 5px;
    border-radius: var(--radius-xs);
    margin-left: 4px;
    font-weight: 700;
    vertical-align: middle;
}
.spreadsheet-grid th.col-numeric, .spreadsheet-grid td.col-numeric {
    text-align: right;
}
.spreadsheet-grid .row-index-cell {
    background: var(--bg-subtle);
    color: var(--text-sub);
    font-weight: 500;
    text-align: center;
    width: 45px;
    min-width: 45px;
    cursor: pointer;
}
.spreadsheet-grid tbody tr:nth-child(even) td.cell-data {
    background: #fbfcfd;
}
.spreadsheet-grid tbody tr:hover td.cell-data {
    background: var(--bg-muted);
}
.spreadsheet-grid td.cell-data {
    cursor: cell;
    background: #ffffff;
    min-width: 110px;
    font-variant-numeric: tabular-nums;
}
.spreadsheet-grid td.cell-data .cell-display {
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 5.6em;
    overflow-y: auto;
    line-height: 1.4;
}
.spreadsheet-grid td.cell-data textarea.cell-edit-textarea {
    width: 100%;
    box-sizing: border-box;
    padding: 2px 4px;
    font-size: 0.85rem;
    font-family: inherit;
    resize: vertical;
    min-height: 1.6em;
}
.spreadsheet-grid td.cell-selected {
    outline: 2px solid var(--primary) !important;
    outline-offset: -2px;
    background: var(--primary-light) !important;
}
.spreadsheet-grid .summary-row td {
    background: var(--bg-subtle);
    font-weight: 600;
    border-top: 2px solid var(--border-subtle);
}
.dropdown-menu-wrap summary::-webkit-details-marker {
    display: none;
}
.dropdown-menu-wrap summary::marker {
    content: "";
}
.dropdown-item:hover {
    background: var(--bg-muted);
}
.summary-dropdown {
    font-size: 0.75rem;
    border: 1px solid #cbd5e1;
    border-radius: var(--radius-xs);
    padding: 2px 4px;
    background: #ffffff;
}

/* UNIFIED NOTE STUDIO */
.note-studio-wrap {
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
    margin-bottom: 1.75rem;
}
.note-nav-tabs {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
    align-items: center;
    margin-bottom: 0.5rem;
}
.kanban-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 1.25rem;
}
.kanban-col {
    background: var(--bg-card);
    border: 1px solid var(--border-glass);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-glass);
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    min-height: 400px;
}
.kanban-col-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-weight: 700;
    font-size: 0.85rem;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    color: var(--text-muted);
    padding-bottom: 0.6rem;
    border-bottom: 1px solid var(--border-subtle);
}
.kanban-count {
    background: var(--primary-gradient);
    color: #ffffff;
    font-size: 0.7rem;
    font-weight: 700;
    padding: 1px 8px;
    border-radius: var(--radius-pill);
}

/* Wiki graph (unified note space) */
.wiki-container {
    display: grid;
    grid-template-columns: 1.1fr 1fr;
    gap: 1.25rem;
}
.wiki-card {
    background: var(--bg-card);
    border: 1px solid var(--border-glass);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-glass);
    padding: 1.25rem 1.5rem;
}
.wiki-node-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    max-height: 420px;
    overflow-y: auto;
}
.wiki-node-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.75rem;
    padding: 0.6rem 0.85rem;
    background: rgba(248, 250, 252, 0.75);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    transition: all var(--transition);
}
.wiki-node-item:hover {
    border-color: var(--primary-border);
    background: var(--primary-light);
}
.wiki-link-title {
    font-weight: 600;
    font-size: 0.85rem;
    color: var(--accent-indigo);
}

/* Calendar (unified note space) */
.calendar-wrap {
    display: flex;
    flex-direction: column;
    gap: 0.85rem;
}
.calendar-day-card {
    background: var(--bg-card);
    border: 1px solid var(--border-glass);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-glass);
    overflow: hidden;
}
.calendar-date-header {
    font-weight: 700;
    font-size: 0.825rem;
    color: #ffffff;
    background: var(--primary-gradient);
    padding: 0.6rem 1.1rem;
}
.calendar-event-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.75rem;
    padding: 0.7rem 1.1rem;
    border-top: 1px solid var(--border-subtle);
    font-size: 0.85rem;
}
.calendar-event-item:first-of-type {
    border-top: none;
}
.kanban-card {
    background: #ffffff;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    padding: 0.85rem;
    box-shadow: var(--shadow-sm);
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
    transition: all var(--transition);
}
.kanban-card:hover {
    transform: translateY(-1px);
    box-shadow: var(--shadow-md);
    border-color: var(--primary-border);
}
.kanban-card-title {
    display: flex;
    align-items: flex-start;
    gap: 0.45rem;
    font-size: 0.875rem;
    font-weight: 600;
}
.kanban-meta {
    display: flex;
    gap: 0.4rem;
    align-items: center;
    flex-wrap: wrap;
    font-size: 0.75rem;
    color: var(--text-sub);
}
.tag-badge {
    background: rgba(239, 246, 255, 0.9);
    color: var(--primary);
    border: 1px solid var(--primary-border);
    padding: 1px 6px;
    border-radius: var(--radius-xs);
    font-size: 0.725rem;
    font-weight: 600;
}
.date-badge {
    background: rgba(254, 243, 199, 0.8);
    color: #b45309;
    border: 1px solid #fde68a;
    padding: 1px 6px;
    border-radius: var(--radius-xs);
    font-size: 0.725rem;
    font-weight: 500;
}
.file-link-tag {
    color: var(--text-sub);
    font-family: var(--font-mono);
    font-size: 0.725rem;
}

/* Whiteboard Stage in Unified Note */
.whiteboard-stage {
    min-height: 480px;
    background-color: var(--bg-surface);
    background-image: radial-gradient(rgba(203, 213, 225, 0.8) 1.2px, transparent 1.2px);
    background-size: 22px 22px;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
    padding: 2rem;
    position: relative;
    overflow: hidden;
}

/* Timeline & Collaborators */
.timeline-list { display: flex; flex-direction: column; gap: 0.75rem; }
.timeline-item {
    display: flex;
    align-items: flex-start;
    gap: 1rem;
    padding: 0.85rem;
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
}
/* Capped so a snapshot's own metadata and any action on it stay visually associated instead of
   sitting at opposite ends of a very wide card. */
.timeline-list { max-width: 860px; }
.collaborator-list { display: flex; flex-direction: column; gap: 0.65rem; max-width: 860px; }
/* Action cluster at the end of a list row. `flex-shrink: 0` keeps the buttons at full size when
   a long name/email pushes on them, which is what made them awkward to hit. */
.row-actions {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex-shrink: 0;
}
.collaborator-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    padding: 0.75rem 1rem;
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
}
.collab-avatar {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background: var(--primary-light);
    color: var(--primary);
    font-weight: 700;
    font-size: 0.8rem;
    display: flex;
    align-items: center;
    justify-content: center;
}
.collab-info { display: flex; flex-direction: column; }
.collab-name { font-weight: 600; font-size: 0.875rem; color: var(--text-main); }

/* Empty States */
.empty-state {
    text-align: center;
    padding: 3.5rem 1.5rem;
    background: var(--bg-card);
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius-lg);
}
.empty-title { font-size: 1.25rem; font-weight: 700; color: var(--text-main); margin-bottom: 0.45rem; }
.empty-desc { color: var(--text-muted); font-size: 0.9rem; max-width: 480px; margin: 0 auto 1.25rem; }

/* AI Copilot & Agent Drawer -- a pure CSS "checkbox hack": `#ai-drawer-toggle-cb` (a hidden
   checkbox, see `ai_drawer.rs`) is a preceding sibling of both `.ai-drawer-backdrop` and
   `.ai-drawer-panel`, so `:checked ~` alone drives the whole open/close/backdrop-dismiss
   interaction -- no JavaScript, no WASM, and it works the instant the page parses, not only once
   an island has hydrated. `display: contents` on `<leptos-island>` wrappers (see the reset rule
   near the top of this file) doesn't affect DOM-tree sibling relationships, only box generation,
   so this keeps working with the checkbox/backdrop/panel each wrapped in their own island output. */
.ai-drawer-toggle-cb {
    position: absolute;
    opacity: 0;
    pointer-events: none;
    width: 0;
    height: 0;
}
.ai-drawer-backdrop {
    display: none;
    position: fixed;
    inset: 0;
    background: rgba(15, 23, 42, 0.4);
    z-index: 1000;
    cursor: pointer;
}
.ai-drawer-toggle-cb:checked ~ .ai-drawer-backdrop {
    display: block;
}
.ai-drawer-panel {
    position: fixed;
    top: 0;
    right: -480px;
    width: 460px;
    max-width: 95vw;
    height: 100vh;
    background: #ffffff;
    box-shadow: -8px 0 32px rgba(15, 23, 42, 0.15);
    z-index: 1001;
    display: flex;
    flex-direction: column;
    transition: right 0.28s cubic-bezier(0.16, 1, 0.3, 1);
}
.ai-drawer-toggle-cb:checked ~ .ai-drawer-panel {
    right: 0;
}
.ai-drawer-header {
    padding: 1rem 1.25rem;
    border-bottom: 1px solid var(--border-subtle);
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: var(--bg-subtle);
}
.ai-drawer-body {
    flex: 1;
    overflow-y: auto;
    padding: 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
}
.ai-drawer-footer {
    padding: 1rem 1.25rem;
    border-top: 1px solid var(--border-subtle);
    background: var(--bg-subtle);
}
.ai-msg {
    padding: 0.85rem 1rem;
    border-radius: var(--radius-md);
    font-size: 0.875rem;
    line-height: 1.6;
}
.ai-msg-user {
    background: var(--primary-light);
    color: var(--text-main);
    border: 1px solid var(--primary-border);
    align-self: flex-end;
    max-width: 85%;
}
.ai-msg-assistant {
    background: #ffffff;
    border: 1px solid var(--border-subtle);
    color: var(--text-main);
    align-self: flex-start;
    box-shadow: var(--shadow-sm);
    max-width: 98%;
}

/* Script Runner Terminal & Visualizer */
.script-console-card {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #090d16;
    border-radius: var(--radius-md);
    overflow: hidden;
    border: 1px solid #1e293b;
    box-shadow: var(--shadow-md);
}
.script-console-bar {
    padding: 0.6rem 1rem;
    background: #0f172a;
    border-bottom: 1px solid #1e293b;
    display: flex;
    justify-content: space-between;
    align-items: center;
}
.terminal-output {
    flex: 1;
    overflow-y: auto;
    padding: 1rem;
    font-family: var(--font-mono);
    font-size: 0.825rem;
    color: #38bdf8;
    background: #090d16;
    line-height: 1.6;
    white-space: pre-wrap;
    min-height: 240px;
}
.script-plot-card {
    background: #1e293b;
    border-top: 1px solid #334155;
    padding: 1rem;
    text-align: center;
}

/* Project Terminal (apich-islands TerminalIsland) -- this dark-on-dark pairing (near-black
   text on a near-black `.section-card` background in terminal_page.rs) is exactly what made the
   output unreadable before these rules existed: `--text-main` is #0f172a and the wrapping card is
   also #0f172a, so with no color of its own `.terminal-screen`/`.terminal-input` rendered text
   the same color as the background. Mirrors `.terminal-output`'s cyan-on-navy scheme above. */
.terminal-screen-wrapper {
    position: relative;
    width: 100%;
    margin-bottom: 0.75rem;
}
.terminal-screen {
    background: #090d16;
    color: #38bdf8;
    font-family: var(--font-mono);
    font-size: 0.85rem;
    line-height: 1.6;
    white-space: pre-wrap;
    padding: 1.25rem;
    border-radius: var(--radius-sm);
    border: 1px solid #1e293b;
    min-height: 560px;
    max-height: 800px;
    height: clamp(560px, 68vh, 850px);
    overflow-y: auto;
    scroll-behavior: smooth;
}
.terminal-screen::-webkit-scrollbar {
    width: 8px;
    height: 8px;
}
.terminal-screen::-webkit-scrollbar-track {
    background: #090d16;
}
.terminal-screen::-webkit-scrollbar-thumb {
    background: #1e293b;
    border-radius: 4px;
}
.terminal-screen::-webkit-scrollbar-thumb:hover {
    background: #334155;
}

/* Draggable & Resizable Selection/Crop Box */
.term-crop-overlay {
    position: absolute;
    inset: 0;
    pointer-events: none;
    z-index: 20;
    border-radius: var(--radius-sm);
    overflow: hidden;
}
.term-crop-box {
    position: absolute;
    border: 2px dashed #38bdf8;
    background: rgba(56, 189, 248, 0.08);
    box-shadow: 0 0 0 9999px rgba(0, 0, 0, 0.55), 0 0 16px rgba(56, 189, 248, 0.35);
    cursor: move;
    pointer-events: auto;
    z-index: 21;
    box-sizing: border-box;
    min-width: 120px;
    min-height: 60px;
    user-select: none;
    touch-action: none;
}
.term-crop-handle {
    position: absolute;
    width: 10px;
    height: 10px;
    background: #38bdf8;
    border: 1.5px solid #ffffff;
    border-radius: 2px;
    z-index: 22;
    pointer-events: auto;
    transition: transform 0.1s ease;
}
.term-crop-handle:hover {
    transform: scale(1.3);
    background: #7dd3fc;
}
.handle-nw { top: -5px; left: -5px; cursor: nwse-resize; }
.handle-n  { top: -5px; left: calc(50% - 5px); cursor: ns-resize; }
.handle-ne { top: -5px; right: -5px; cursor: nesw-resize; }
.handle-e  { top: calc(50% - 5px); right: -5px; cursor: ew-resize; }
.handle-se { bottom: -5px; right: -5px; cursor: nwse-resize; }
.handle-s  { bottom: -5px; left: calc(50% - 5px); cursor: ns-resize; }
.handle-sw { bottom: -5px; left: -5px; cursor: nesw-resize; }
.handle-w  { top: calc(50% - 5px); left: -5px; cursor: ew-resize; }

.term-crop-badge {
    position: absolute;
    top: -26px;
    left: 0;
    background: #0f172a;
    color: #38bdf8;
    font-family: var(--font-mono);
    font-size: 0.72rem;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: 4px;
    border: 1px solid #38bdf8;
    white-space: nowrap;
    pointer-events: none;
    box-shadow: 0 2px 6px rgba(0,0,0,0.4);
}
.term-crop-actions {
    position: absolute;
    bottom: 8px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    gap: 0.45rem;
    background: rgba(15, 23, 42, 0.96);
    backdrop-filter: blur(8px);
    padding: 5px 10px;
    border-radius: 8px;
    border: 1.5px solid #38bdf8;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.7);
    z-index: 35;
    white-space: nowrap;
    pointer-events: auto;
}
.term-crop-actions-top {
    bottom: auto !important;
    top: 8px !important;
}

/* Terminal Saved Asset Card & Snippets */
.term-saved-card {
    background: #0f172a;
    border: 1px solid #334155;
    border-radius: 10px;
    padding: 1rem 1.25rem;
    margin-top: 1rem;
    box-shadow: 0 4px 16px rgba(0,0,0,0.3);
}
.term-snippet-row {
    margin-top: 0.6rem;
}
.term-snippet-label {
    font-size: 0.75rem;
    font-weight: 600;
    color: #94a3b8;
    display: flex;
    align-items: center;
    gap: 0.35rem;
    margin-bottom: 0.2rem;
}
.term-snippet-box {
    background: #090d16;
    border: 1px solid #1e293b;
    border-radius: 6px;
    padding: 0.45rem 0.75rem;
    font-family: var(--font-mono);
    font-size: 0.8rem;
    color: #38bdf8;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
    word-break: break-all;
}
.term-rec-pulse {
    width: 8px;
    height: 8px;
    background: #ef4444;
    border-radius: 50%;
    display: inline-block;
    animation: termPulse 1s infinite alternate;
}
@keyframes termPulse {
    0% { transform: scale(0.85); opacity: 0.6; }
    100% { transform: scale(1.3); opacity: 1; }
}
.terminal-bar {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    background: #0f172a;
    border: 1.5px solid #1e293b;
    border-radius: var(--radius-sm);
    padding: 0.75rem 1.15rem;
    min-height: 54px;
    transition: border-color 0.2s ease, box-shadow 0.2s ease;
}
.terminal-bar:focus-within {
    border-color: #38bdf8;
    box-shadow: 0 0 0 3px rgba(56, 189, 248, 0.15);
}
.terminal-input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    color: #e2e8f0;
    font-family: var(--font-mono);
    font-size: 0.95rem;
}
.terminal-input::placeholder {
    color: var(--text-sub);
}
.terminal-input:disabled {
    /* Disabled only while the island's wasm hasn't hydrated yet (see terminal.rs's `hydrated`
       signal) -- an italic placeholder is a much clearer "still loading" signal here than the
       browser's own barely-visible default disabled-input dimming would be against this dark
       background. */
    font-style: italic;
    cursor: wait;
}
.script-plot-card img {
    max-width: 100%;
    border-radius: var(--radius-sm);
    box-shadow: 0 4px 12px rgba(0,0,0,0.3);
}

/* SVG Live Preview & Reverse Search Glow */
.svg-preview-stage {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-muted);
    border-radius: var(--radius-md);
    overflow: hidden;
    position: relative;
}
.svg-nav-toolbar {
    padding: 0.5rem 1rem;
    background: #ffffff;
    border-bottom: 1px solid var(--border-subtle);
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-shrink: 0;
}
.svg-viewport-wrapper {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
    overflow: hidden;
    width: 100%;
    height: 100%;
}
.svg-scroll-container {
    flex: 1;
    overflow-y: auto;
    padding: 1.5rem 3.5rem;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    width: 100%;
    height: 100%;
    scroll-behavior: smooth;
}

/* Edge navigation strips (< and >) spanning top to bottom */
.svg-page-nav-edge {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 64px;
    z-index: 20;
    display: flex;
    align-items: center;
    background: transparent;
    border: none;
    padding: 0;
    cursor: pointer;
    transition: background 0.2s ease, opacity 0.2s ease;
    outline: none;
    -webkit-tap-highlight-color: transparent;
}
.svg-page-nav-prev {
    left: 0;
    justify-content: flex-start;
    padding-left: 0.85rem;
}
.svg-page-nav-prev:hover {
    background: linear-gradient(to right, rgba(0, 0, 0, 0.08), rgba(0, 0, 0, 0.001));
}
.svg-page-nav-next {
    right: 0;
    justify-content: flex-end;
    padding-right: 0.85rem;
}
.svg-page-nav-next:hover {
    background: linear-gradient(to left, rgba(0, 0, 0, 0.08), rgba(0, 0, 0, 0.001));
}

[data-theme="dark"] .svg-page-nav-prev:hover,
.presentation-nav-edge.svg-page-nav-prev:hover {
    background: linear-gradient(to right, rgba(255, 255, 255, 0.12), rgba(255, 255, 255, 0.001));
}
[data-theme="dark"] .svg-page-nav-next:hover,
.presentation-nav-edge.svg-page-nav-next:hover {
    background: linear-gradient(to left, rgba(255, 255, 255, 0.12), rgba(255, 255, 255, 0.001));
}

.svg-page-nav-edge.hidden {
    opacity: 0 !important;
    pointer-events: none !important;
}

/* Floating < and > arrow button inside edge strip */
.svg-page-nav-arrow {
    width: 40px;
    height: 40px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 1.6rem;
    line-height: 1;
    font-weight: 700;
    background: var(--bg-surface, #ffffff);
    color: var(--text-main, #1e293b);
    box-shadow: 0 4px 14px rgba(0, 0, 0, 0.16);
    border: 1px solid var(--border-subtle, rgba(0, 0, 0, 0.1));
    transition: transform 0.2s cubic-bezier(0.34, 1.56, 0.64, 1), background 0.2s ease, box-shadow 0.2s ease, color 0.2s ease;
    user-select: none;
    pointer-events: none;
}
.presentation-nav-edge .svg-page-nav-arrow {
    background: rgba(30, 41, 59, 0.85);
    color: #ffffff;
    border-color: rgba(255, 255, 255, 0.2);
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.4);
}
.svg-page-nav-edge:hover .svg-page-nav-arrow {
    transform: scale(1.15);
    background: var(--primary, #635bff);
    color: #ffffff;
    border-color: transparent;
    box-shadow: 0 6px 20px rgba(99, 91, 255, 0.45);
}
.svg-page-nav-edge:active .svg-page-nav-arrow {
    transform: scale(0.92);
}
.svg-page-box {
    background: #ffffff;
    box-shadow: 0 8px 30px rgba(0,0,0,0.12);
    border-radius: var(--radius-xs);
    max-width: 100%;
}
.svg-page-box svg {
    display: block;
    max-width: 100%;
    height: auto;
}
.svg-page-box a {
    cursor: pointer;
    transition: opacity 0.15s;
}
.svg-page-box a:hover {
    outline: 2px solid var(--primary);
    outline-offset: 1px;
    background: var(--primary-ring);
}

/* Reverse search highlight flash */
@keyframes line-glow {
    0% { background-color: rgba(99, 91, 255, 0.35); }
    100% { background-color: transparent; }
}
.line-highlight-flash {
    animation: line-glow 2.5s ease-out;
}

/* Interactive Markdown Elements */
.interactive-task-list {
    list-style: none;
    padding-left: 0;
    margin: 0.75rem 0;
}
.interactive-task-row {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    padding: 0.4rem 0.6rem;
    border-radius: var(--radius-xs);
    transition: background 0.15s;
}
.interactive-task-row:hover {
    background: var(--bg-subtle);
}
.interactive-task-row.task-completed .task-text {
    text-decoration: line-through;
    color: var(--text-sub);
}
.task-live-checkbox {
    cursor: pointer;
    width: 16px;
    height: 16px;
    accent-color: var(--primary);
}
.task-tag-badge {
    font-size: 0.7rem;
    color: var(--accent-cyan);
    background: rgba(2, 132, 199, 0.1);
    padding: 1px 6px;
    border-radius: var(--radius-xs);
    font-weight: 600;
}
.task-date-badge {
    font-size: 0.7rem;
    color: var(--warning);
    background: var(--warning-bg);
    padding: 1px 6px;
    border-radius: var(--radius-xs);
    font-weight: 600;
}
.wiki-link-pill {
    display: inline-block;
    color: var(--accent-purple);
    background: rgba(124, 58, 237, 0.08);
    border: 1px solid rgba(124, 58, 237, 0.2);
    border-radius: var(--radius-xs);
    padding: 1px 6px;
    font-weight: 600;
    font-size: 0.85em;
    text-decoration: none;
}
.wiki-link-pill:hover {
    background: rgba(124, 58, 237, 0.18);
    text-decoration: none;
}
.math-block {
    background: var(--bg-subtle);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    padding: 0.75rem 1rem;
    margin: 0.75rem 0;
    font-family: serif;
    font-size: 1.05rem;
    text-align: center;
}
.math-inline {
    font-family: serif;
    background: rgba(241, 245, 249, 0.6);
    padding: 1px 4px;
    border-radius: var(--radius-xs);
}
.doc-heading {
    position: relative;
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: 1.25rem;
    margin-bottom: 0.5rem;
    font-weight: 700;
}
.line-sync-anchor {
    font-size: 0.75rem;
    color: #94a3b8;
    text-decoration: none;
    padding: 1px 5px;
    border-radius: var(--radius-xs);
}
.line-sync-anchor:hover {
    background: var(--border-subtle);
    color: #1e293b;
}

/* File Sharing Badges */
.share-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    font-size: 0.725rem;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: var(--radius-pill);
}
.share-badge-public {
    background: rgba(22, 163, 74, 0.1);
    color: var(--success);
    border: 1px solid rgba(22, 163, 74, 0.25);
}
.share-badge-specific {
    background: var(--primary-light);
    color: var(--primary);
    border: 1px solid var(--primary-border);
}
.share-badge-private {
    background: rgba(148, 163, 184, 0.15);
    color: var(--text-sub);
    border: 1px solid rgba(148, 163, 184, 0.3);
}

/* Responsive Queries */
@media (max-width: 900px) {
    /* The sidebar becomes a slide-over rather than `display: none`. It used to be removed
       outright on small screens, which took the entire primary navigation with it and left no
       way to reach Projects/Templates/Settings at all. It now uses the same collapse mechanism
       as desktop (SIDEBAR_TOGGLE_JS defaults it closed below this width), so the toggle button
       is a real mobile nav control. */
    .app-main { margin-left: 0; padding: var(--space-4); }
    .app-sidebar { box-shadow: var(--shadow-lg); }
    .app-layout:not(.sidebar-collapsed) .app-sidebar { transform: translateX(0); }
    .page-header { flex-direction: column; align-items: flex-start; gap: var(--space-4); }
    .header-actions { width: 100%; flex-wrap: wrap; }
    .editor-studio-grid { grid-template-columns: 1fr; }
    .spreadsheet-layout { grid-template-columns: 1fr; }
    .kanban-grid { grid-template-columns: 1fr; }
    .wiki-container { grid-template-columns: 1fr; }
    .projects-grid { grid-template-columns: 1fr; }
    /* Comfortable touch targets. */
    .btn { min-height: 40px; }
    .sidebar-link { padding: 0.7rem; }
}

/* User autocomplete dropdown in sharing dialogs */
.user-autocomplete-dropdown {
    background: var(--bg-elevated);
    border: 1px solid var(--border-subtle);
    box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.12), 0 8px 10px -6px rgba(0, 0, 0, 0.08);
}
.user-dropdown-item:hover {
    background: var(--bg-muted);
}
[data-theme="dark"] .user-autocomplete-dropdown,
html.dark .user-autocomplete-dropdown {
    background: var(--bg-surface-elevated, #1e293b);
    border-color: var(--border-strong, #334155);
}
[data-theme="dark"] .user-dropdown-item:hover,
html.dark .user-dropdown-item:hover {
    background: rgba(255, 255, 255, 0.06);
}
[data-theme="dark"] select,
html.dark select,
[data-theme="dark"] select.form-control,
html.dark select.form-control {
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='16' height='16' viewBox='0 0 24 24' fill='none' stroke='%2394a3b8' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='m6 9 6 6 6-6'/%3E%3C/svg%3E");
}

/* Enhanced File Upload Modal & Metrics */
.upload-dropzone {
    border: 2px dashed var(--border-subtle, #424758);
    border-radius: 10px;
    padding: 2rem 1.5rem;
    text-align: center;
    cursor: pointer;
    background: var(--bg-elevated, rgba(255,255,255,0.02));
    transition: all 0.2s ease-in-out;
}
.upload-dropzone:hover,
.upload-dropzone.is-dragover {
    border-color: var(--primary, #3b82f6) !important;
    background: rgba(59, 130, 246, 0.08) !important;
    transform: translateY(-1px);
    box-shadow: 0 4px 14px rgba(59, 130, 246, 0.15);
}
.upload-selected-card {
    transition: all 0.2s ease;
}
.upload-selected-card:hover {
    border-color: var(--border-strong, #4f566b) !important;
}
.upload-progress-container {
    background: var(--bg-elevated, #282c37);
    border-radius: 10px;
    overflow: hidden;
    position: relative;
    box-shadow: inset 0 1px 3px rgba(0,0,0,0.3);
}
.upload-progress-fill {
    height: 100%;
    background: linear-gradient(90deg, #3b82f6, #60a5fa, #38bdf8);
    transition: width 0.15s ease-out;
    border-radius: 10px;
    position: relative;
}
.upload-progress-shine {
    position: absolute;
    inset: 0;
    background: linear-gradient(90deg, transparent, rgba(255,255,255,0.35), transparent);
    animation: uploadShine 1.8s infinite linear;
}
@keyframes uploadShine {
    0% { transform: translateX(-100%); }
    100% { transform: translateX(100%); }
}
@keyframes uploadSpin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
}
.upload-metric-card {
    transition: transform 0.15s ease, border-color 0.15s ease;
}
.upload-metric-card:hover {
    transform: translateY(-1px);
    border-color: var(--primary, #3b82f6) !important;
}

/* Interactive VCS Timeline & Diff Viewer */
.vcs-timeline-container {
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
    margin-top: 0.5rem;
}

.vcs-timeline-toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
    padding: 0.85rem 1.25rem;
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.04);
    flex-wrap: wrap;
}

.vcs-toolbar-search {
    display: flex;
    align-items: center;
    position: relative;
    flex: 1;
    min-width: 260px;
    max-width: 480px;
}

.vcs-search-icon {
    position: absolute;
    left: 0.75rem;
    font-size: 0.9rem;
    color: var(--text-muted);
    pointer-events: none;
}

.vcs-search-input {
    padding-left: 2.25rem !important;
    padding-right: 2rem !important;
    border-radius: 999px !important;
    border: 1.5px solid var(--border-subtle) !important;
    background: var(--bg-subtle) !important;
    transition: all 0.15s ease;
}

.vcs-search-input:focus {
    border-color: var(--primary) !important;
    background: var(--bg-surface) !important;
    box-shadow: 0 0 0 3px var(--primary-light, rgba(99, 91, 255, 0.15)) !important;
}

.vcs-clear-btn {
    position: absolute;
    right: 0.75rem;
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 0.8rem;
    padding: 2px 5px;
    border-radius: 50%;
}

.vcs-clear-btn:hover {
    color: var(--text-main);
}

.vcs-toolbar-actions {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
}

.vcs-file-filter-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.3rem 0.75rem;
    border-radius: 999px;
    background: var(--primary-light);
    color: var(--primary);
    font-size: 0.8rem;
    font-weight: 600;
    border: 1px solid var(--primary-border, rgba(99, 91, 255, 0.25));
}

.vcs-badge-close {
    background: none;
    border: none;
    color: var(--primary);
    cursor: pointer;
    font-size: 0.8rem;
    padding: 0 2px;
    line-height: 1;
}

.vcs-status-banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 1.25rem;
    border-radius: var(--radius-sm);
    font-size: 0.9rem;
    animation: subtle-fade-in 0.2s ease-out;
}

.vcs-banner-close {
    background: none;
    border: none;
    font-size: 1rem;
    cursor: pointer;
    opacity: 0.7;
}

.vcs-banner-close:hover {
    opacity: 1;
}

.vcs-main-split {
    display: grid;
    grid-template-columns: 360px 1fr;
    gap: 1.25rem;
    align-items: flex-start;
}

@media (max-width: 960px) {
    .vcs-main-split {
        grid-template-columns: 1fr;
    }
}

/* Timeline Rail (Left) */
.vcs-timeline-rail {
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.05);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    max-height: 720px;
}

.vcs-rail-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.85rem 1.25rem;
    border-bottom: 1px solid var(--border-subtle);
    background: var(--bg-subtle);
}

.vcs-rail-title {
    font-weight: 700;
    font-size: 0.92rem;
    color: var(--text-main);
}

.vcs-rail-count {
    font-size: 0.78rem;
    color: var(--text-muted);
    font-weight: 500;
}

.vcs-rail-scroll {
    overflow-y: auto;
    padding: 0.75rem 0.5rem;
    display: flex;
    flex-direction: column;
}

.vcs-nodes-list {
    display: flex;
    flex-direction: column;
}

.vcs-node-item {
    display: flex;
    gap: 0.75rem;
    padding: 0.75rem 0.65rem;
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.15s ease;
    border: 1px solid transparent;
}

.vcs-node-item:hover {
    background: var(--bg-hover);
    border-color: var(--border-subtle);
}

.vcs-node-item.selected {
    background: var(--primary-light, rgba(99, 91, 255, 0.08));
    border-color: var(--primary);
    box-shadow: 0 2px 8px rgba(99, 91, 255, 0.15);
}

.vcs-node-track {
    display: flex;
    flex-direction: column;
    align-items: center;
    width: 24px;
    flex-shrink: 0;
    position: relative;
}

.vcs-track-line-top,
.vcs-track-line-bottom {
    width: 2px;
    background: var(--border-subtle);
    flex: 1;
}

.vcs-track-line-top.hidden,
.vcs-track-line-bottom.hidden {
    opacity: 0;
}

.vcs-node-dot {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: var(--bg-surface);
    border: 2px solid var(--border-strong, #94a3b8);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 0.7rem;
    color: var(--text-muted);
    z-index: 2;
    flex-shrink: 0;
    transition: all 0.15s ease;
}

.vcs-node-dot.head {
    border-color: var(--primary);
    background: var(--primary);
    color: #fff;
    box-shadow: 0 0 0 3px var(--primary-light, rgba(99, 91, 255, 0.25));
}

.vcs-node-dot.milestone {
    border-color: #f59e0b;
    background: #fef3c7;
    font-size: 0.8rem;
}

.vcs-node-item.selected .vcs-node-dot {
    border-color: var(--primary);
    transform: scale(1.1);
}

.vcs-node-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
}

.vcs-node-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
}

.vcs-node-msg {
    font-size: 0.88rem;
    font-weight: 600;
    color: var(--text-main);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.vcs-verified-badge {
    font-size: 0.8rem;
}

.vcs-node-meta {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.78rem;
    color: var(--text-muted);
    flex-wrap: wrap;
}

.vcs-short-hash {
    font-family: var(--font-mono);
    padding: 1px 4px;
    background: var(--bg-subtle);
    border: 1px solid var(--border-subtle);
    border-radius: 4px;
    font-size: 0.74rem;
    color: var(--primary);
}

.vcs-ms-badge {
    font-size: 0.68rem;
}

/* Snapshot Inspector (Right) */
.vcs-inspector-column {
    display: flex;
    flex-direction: column;
    min-width: 0;
}

.vcs-inspector-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.05);
    overflow: hidden;
    display: flex;
    flex-direction: column;
}

.vcs-inspector-loading {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.75rem;
    padding: 5rem 2rem;
    color: var(--text-muted);
    font-size: 0.95rem;
}

.vcs-snap-header {
    padding: 1.25rem 1.5rem;
    border-bottom: 1px solid var(--border-subtle);
    background: var(--bg-surface);
}

.vcs-snap-title-row {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
    flex-wrap: wrap;
    margin-bottom: 1rem;
}

.vcs-snap-message {
    font-size: 1.2rem;
    font-weight: 700;
    color: var(--text-main);
    margin: 0;
    line-height: 1.35;
    flex: 1;
    min-width: 240px;
}

.vcs-snap-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-shrink: 0;
    flex-wrap: wrap;
}

.vcs-snap-meta-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 0.75rem;
    padding: 0.75rem 1rem;
    background: var(--bg-subtle);
    border-radius: 8px;
    border: 1px solid var(--border-subtle);
}

.vcs-meta-item {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
}

.vcs-meta-label {
    font-size: 0.72rem;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.03em;
}

.vcs-meta-val {
    font-size: 0.84rem;
    color: var(--text-main);
    font-weight: 500;
}

/* Changed Files Section */
.vcs-files-section {
    padding: 1rem 1.5rem;
    border-bottom: 1px solid var(--border-subtle);
    background: var(--bg-subtle);
}

.vcs-files-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.75rem;
}

.vcs-files-title {
    font-weight: 700;
    font-size: 0.9rem;
    color: var(--text-main);
}

.vcs-files-stats {
    display: flex;
    gap: 0.75rem;
    font-size: 0.78rem;
    font-weight: 600;
}

.vcs-stat-added { color: #16a34a; }
.vcs-stat-mod { color: #d97706; }
.vcs-stat-rem { color: #dc2626; }

.vcs-file-pills-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
}

.vcs-file-pill {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.35rem 0.8rem;
    border-radius: 6px;
    font-size: 0.82rem;
    font-weight: 500;
    cursor: pointer;
    border: 1px solid var(--border-subtle);
    background: var(--bg-surface);
    color: var(--text-main);
    transition: all 0.15s ease;
}

.vcs-file-pill:hover {
    border-color: var(--primary);
    transform: translateY(-1px);
}

.vcs-file-pill.active {
    border-color: var(--primary);
    background: var(--primary-light);
    color: var(--primary);
    font-weight: 600;
    box-shadow: 0 2px 6px rgba(99, 91, 255, 0.15);
}

.pill-badge {
    font-weight: 700;
    font-size: 0.85rem;
}

.pill-added .pill-badge { color: #16a34a; }
.pill-mod .pill-badge { color: #d97706; }
.pill-rem .pill-badge { color: #dc2626; }

/* Diff Viewer Section */
.vcs-diff-section {
    padding: 0;
    background: var(--bg-surface);
}

.vcs-diff-loading {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.75rem;
    padding: 4rem 2rem;
    color: var(--text-muted);
}

.vcs-diff-placeholder {
    padding: 4rem 2rem;
    text-align: center;
    color: var(--text-muted);
    font-size: 0.9rem;
}

.vcs-diff-viewer {
    display: flex;
    flex-direction: column;
}

.vcs-diff-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.65rem 1.25rem;
    background: var(--bg-subtle);
    border-bottom: 1px solid var(--border-subtle);
}

.vcs-diff-filepath {
    font-family: var(--font-mono);
    font-weight: 600;
    font-size: 0.85rem;
    color: var(--text-main);
}

.vcs-diff-counts {
    display: flex;
    gap: 0.6rem;
    font-family: var(--font-mono);
    font-size: 0.8rem;
    font-weight: 700;
}

.diff-count-add { color: #16a34a; }
.diff-count-del { color: #dc2626; }

.vcs-diff-table-container {
    overflow-x: auto;
    max-height: 520px;
}

.vcs-diff-table {
    width: 100%;
    border-collapse: collapse;
    font-family: var(--font-mono);
    font-size: 0.82rem;
    line-height: 1.5;
}

.diff-row {
    border-bottom: 1px solid rgba(226, 232, 240, 0.3);
}

.diff-row-add {
    background: rgba(34, 197, 94, 0.12);
}

.diff-row-add:hover {
    background: rgba(34, 197, 94, 0.18);
}

.diff-row-del {
    background: rgba(239, 68, 68, 0.12);
}

.diff-row-del:hover {
    background: rgba(239, 68, 68, 0.18);
}

.diff-row-ctx:hover {
    background: var(--bg-hover);
}

.diff-lineno {
    width: 48px;
    padding: 2px 8px;
    text-align: right;
    color: var(--text-muted);
    user-select: none;
    font-size: 0.75rem;
    border-right: 1px solid var(--border-subtle);
    opacity: 0.7;
}

.diff-sign {
    width: 24px;
    padding: 2px 6px;
    text-align: center;
    user-select: none;
    font-weight: 700;
}

.diff-row-add .diff-sign { color: #16a34a; }
.diff-row-del .diff-sign { color: #dc2626; }

.diff-code {
    padding: 2px 8px;
    white-space: pre-wrap;
    word-break: break-all;
}

.diff-code code {
    font-family: inherit;
    font-size: inherit;
    background: none;
    padding: 0;
    color: inherit;
}

/* Honour the OS "reduce motion" setting: transitions collapse to effectively instant rather
   than being removed, so state changes still land in the right place. */
@media (prefers-reduced-motion: reduce) {
    *,
    *::before,
    *::after {
        animation-duration: 0.01ms !important;
        animation-iteration-count: 1 !important;
        transition-duration: 0.01ms !important;
        scroll-behavior: auto !important;
    }
}
"#;
