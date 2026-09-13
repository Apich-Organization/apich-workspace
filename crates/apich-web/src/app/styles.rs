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
.role-badge-lead { background: #faf5ff; color: #7e22ce; border: 1px solid #e9d5ff; }
.role-badge-editor { background: #f0fdf4; color: #15803d; border: 1px solid #bbf7d0; }
.role-badge-viewer { background: var(--bg-subtle); color: var(--text-sub); border: 1px solid var(--border-subtle); }

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

.file-type-pill {
    display: inline-block;
    padding: 2px 7px;
    border-radius: var(--radius-xs);
    font-size: 0.7rem;
    font-weight: 600;
}
.pill-slide { background: #fdf2f8; color: #db2777; border: 1px solid #fbcfe8; }
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
.modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1.25rem;
}
.modal-title { font-size: 1.2rem; font-weight: 700; color: var(--text-main); }
.modal-close { background: none; border: none; font-size: 1.5rem; cursor: pointer; color: var(--text-sub); }
.modal-close:hover { color: var(--text-main); }

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
.terminal-screen {
    background: #090d16;
    color: #38bdf8;
    font-family: var(--font-mono);
    font-size: 0.825rem;
    line-height: 1.6;
    white-space: pre-wrap;
    padding: 1rem;
    border-radius: var(--radius-sm);
    border: 1px solid #1e293b;
    min-height: 320px;
    max-height: 480px;
    overflow-y: auto;
    margin-bottom: 0.75rem;
}
.terminal-bar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: #0f172a;
    border: 1px solid #1e293b;
    border-radius: var(--radius-sm);
    padding: 0.5rem 0.75rem;
}
.terminal-input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    color: #e2e8f0;
    font-family: var(--font-mono);
    font-size: 0.875rem;
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
}
.svg-nav-toolbar {
    padding: 0.5rem 1rem;
    background: #ffffff;
    border-bottom: 1px solid var(--border-subtle);
    display: flex;
    justify-content: space-between;
    align-items: center;
}
.svg-scroll-container {
    flex: 1;
    overflow-y: auto;
    padding: 1.5rem;
    display: flex;
    justify-content: center;
    align-items: flex-start;
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
