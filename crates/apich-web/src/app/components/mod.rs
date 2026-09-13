use crate::app::styles::EMBEDDED_CSS;
use crate::ui::i18n::I18n;
use apich_db::User;
use leptos::prelude::*;
use uuid::Uuid;

/// Renders a Leptos view to a full HTML5 document string (server-side, no hydration for the
/// page itself -- individual `#[island]` components still ship their own small WASM bundle).
///
/// Islands that take `children` (e.g. `apich_islands::ModalIsland`) need a real
/// `hydration_context::SharedContext` to exist on the current `Owner` -- without one, Leptos's
/// internal `IslandChildren` machinery panics with `Option::unwrap()` on `None` the moment it
/// tries to register the children's hydration callback. A plain `view_fn().to_html()` call
/// never creates that context, so every render goes through a real islands-mode `Owner` root.
pub fn render_document<F, IV>(view_fn: F) -> String
where
    F: FnOnce() -> IV + 'static,
    IV: IntoView,
{
    let owner = Owner::new_root(Some(std::sync::Arc::new(
        hydration_context::SsrSharedContext::new_islands(),
    )));
    owner.with(|| format!("<!DOCTYPE html>{}", view_fn().to_html()))
}

/// Standalone page shell (auth pages): doctype + head + centered body, no sidebar.
#[component]
pub fn PageShell(
    title: String,
    i18n: I18n,
    children: Children,
) -> impl IntoView {
    let lang_attr = if i18n.is_zh() {
        "zh-CN"
    } else {
        "en"
    };
    let full_title = {
        let mut s = title;
        s.push_str(" - APICH ");
        s.push_str(i18n.brand_title());
        s
    };
    view! {
        <html lang=lang_attr>
            <head>
                <meta charset="UTF-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1.0" />
                <title>{full_title}</title>
                <style>{EMBEDDED_CSS}</style>
            </head>
            <body>
                <div id="app">{children()}</div>
            </body>
        </html>
    }
}

/// KaTeX <head> tags shared by any page shell whose content may include math -- renders
/// `$...$`/`$$...$$` delimiters that DocumentRenderer emits as `.math-inline`/`.math-block`
/// elements. Without this, math content silently shows as raw unrendered LaTeX source.
#[component]
fn KatexHead() -> impl IntoView {
    view! {
        <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/katex@0.16.8/dist/katex.min.css" />
        <script defer=true src="https://cdn.jsdelivr.net/npm/katex@0.16.8/dist/katex.min.js"></script>
        <script
            defer=true
            src="https://cdn.jsdelivr.net/npm/katex@0.16.8/dist/contrib/auto-render.min.js"
            onload="try{renderMathInElement(document.body,{delimiters:[{left:'$$',right:'$$',display:true},{left:'$',right:'$',display:false}]})}catch(e){}"
        ></script>
    }
}

/// Bootstraps Leptos islands: lazily loads the compiled `apich-islands` wasm bundle (served as
/// static files from `/pkg`, see `create_app`) only once the browser is idle, then hydrates
/// every `<leptos-island>` marker already present in the server-rendered HTML. This is Leptos's
/// own framework-provided bootstrap script (verbatim from `leptos::hydration::island_script`),
/// not hand-written page logic -- it never touches app state or does anything besides wiring
/// each island element to its compiled Rust function.
#[component]
fn IslandScript() -> impl IntoView {
    view! {
        <link rel="modulepreload" href="/pkg/apich_islands.js" />
        <link rel="preload" href="/pkg/apich_islands_bg.wasm" r#as="fetch" r#type="application/wasm" />
        <script type="module">
            {format!(
                "{ISLAND_BOOTSTRAP_JS}(\"\", \"pkg\", \"apich_islands\", \"apich_islands_bg\")",
            )}
        </script>
    }
}

const ISLAND_BOOTSTRAP_JS: &str = include_str!("island_script.js");


/// Which sidebar section is currently active, driving highlight + admin tier logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveNav {
    Projects,
    Templates,
    Settings,
    OrgAdmin,
    PlatformAdmin,
}

impl ActiveNav {
    fn is(
        self,
        other: Self,
    ) -> bool {
        self == other
    }
}

/// Full application shell: doctype + sidebar (with tiered admin-panel entries) + main content.
///
/// Sidebar entry count mirrors plan.md's tiering: platform admin sees platform-admin
/// panel + org-admin panel + personal settings (3); an org/team admin sees the org-admin
/// panel + personal settings (2); everyone else only sees personal settings (1).
#[component]
pub fn AppShell(
    user: User,
    is_org_or_team_admin: bool,
    active_nav: ActiveNav,
    current_path: String,
    page_title: String,
    i18n: I18n,
    children: Children,
) -> impl IntoView {
    let full_title = {
        let mut s = page_title;
        s.push_str(" - APICH ");
        s.push_str(i18n.brand_title());
        s
    };
    let lang_toggle_url = format!(
        "/set-lang?lang={}&return_to={}",
        i18n.lang.toggle_code(),
        urlencoding::encode(&current_path)
    );
    drop(current_path);
    let lang_attr = if i18n.is_zh() {
        "zh-CN"
    } else {
        "en"
    };
    let user_initial = user
        .display_name
        .chars()
        .next()
        .unwrap_or('U')
        .to_uppercase()
        .to_string();
    let role_title = if user.is_platform_admin {
        i18n.platform_admin()
    } else if is_org_or_team_admin {
        i18n.org_admin_title()
    } else {
        i18n.researcher()
    };

    let admin_section = if user.is_platform_admin {
        Some(view! {
            <div class="sidebar-section">
                <span class="sidebar-heading">"Platform"</span>
                <a href="/admin/platform" class="sidebar-link" class:active=active_nav.is(ActiveNav::PlatformAdmin)>
                    <span class="sidebar-icon">"🛡️"</span>
                    <span>{i18n.sidebar_platform_admin()}</span>
                </a>
                <a href="/admin/orgs" class="sidebar-link" class:active=active_nav.is(ActiveNav::OrgAdmin)>
                    <span class="sidebar-icon">"🏛️"</span>
                    <span>{i18n.sidebar_org_admin()}</span>
                </a>
            </div>
        }.into_any())
    } else if is_org_or_team_admin {
        Some(view! {
            <div class="sidebar-section">
                <span class="sidebar-heading">"Administration"</span>
                <a href="/admin/orgs" class="sidebar-link" class:active=active_nav.is(ActiveNav::OrgAdmin)>
                    <span class="sidebar-icon">"🏛️"</span>
                    <span>{i18n.sidebar_my_org_admin()}</span>
                </a>
            </div>
        }.into_any())
    } else {
        None
    };

    view! {
        <html lang=lang_attr>
            <head>
                <meta charset="UTF-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1.0" />
                <title>{full_title}</title>
                <KatexHead />
                <IslandScript />
                <style>{EMBEDDED_CSS}</style>
            </head>
            <body>
                <div id="app">
                    <div class="app-layout">
                        // Outside the <aside> on purpose: the collapsed state slides the sidebar
                        // away with `transform`, which moves its descendants with it (and makes
                        // it the containing block for any `position: fixed` child), so a toggle
                        // button living inside it went off-screen along with the sidebar --
                        // leaving no way to bring it back. As a sibling it stays put.
                        <button
                            type="button"
                            id="sidebar-toggle"
                            class="sidebar-toggle"
                            title="Collapse/expand sidebar (Ctrl+B)"
                            aria-label="Toggle sidebar"
                        >"«"</button>
                        <aside class="app-sidebar">
                            <div class="sidebar-brand">
                                <a href="/" class="brand-link">
                                    <span class="brand-badge">"APICH"</span>
                                    <span class="brand-title">{i18n.brand_title()}</span>
                                </a>
                            </div>
                            <div class="sidebar-body">
                                <div class="sidebar-section">
                                    <span class="sidebar-heading">"Workspace"</span>
                                    <a href="/" class="sidebar-link" class:active=active_nav.is(ActiveNav::Projects)>
                                        <span class="sidebar-icon">"📁"</span>
                                        <span>{i18n.nav_projects()}</span>
                                    </a>
                                    <a href="/templates" class="sidebar-link" class:active=active_nav.is(ActiveNav::Templates)>
                                        <span class="sidebar-icon">"📚"</span>
                                        <span>{i18n.nav_templates()}</span>
                                    </a>
                                </div>
                                // One-click "start writing X" shortcuts. The island asks where
                                // the new document should go (a new project, or an existing one)
                                // rather than creating a project the instant a button is pressed.
                                <apich_islands::QuickStartMenuIsland variant=apich_islands::QuickStartVariant::Sidebar />
                                {admin_section}
                                <div class="sidebar-section">
                                    <span class="sidebar-heading">"Account"</span>
                                    <a href="/settings" class="sidebar-link" class:active=active_nav.is(ActiveNav::Settings)>
                                        <span class="sidebar-icon">"⚙️"</span>
                                        <span>{i18n.sidebar_settings()}</span>
                                    </a>
                                </div>
                            </div>
                            <div class="sidebar-footer">
                                <div class="sidebar-user-card">
                                    <div class="sidebar-user-avatar">{user_initial}</div>
                                    <div class="sidebar-user-info">
                                        <div class="sidebar-user-name" title=user.display_name>{user.display_name.clone()}</div>
                                        <div class="sidebar-user-role">{role_title}</div>
                                    </div>
                                </div>
                                <div class="sidebar-actions">
                                    <a
                                        href=lang_toggle_url
                                        class="lang-toggle"
                                        title="Switch Language"
                                    >
                                        "🌐 " {i18n.lang.toggle_label()}
                                    </a>
                                    <form method="post" action="/logout" class="inline-form">
                                        <button type="submit" class="btn btn-ghost btn-sm" title=i18n.logout()>
                                            "🚪 " {i18n.logout()}
                                        </button>
                                    </form>
                                </div>
                            </div>
                        </aside>
                        <main class="app-main">{children()}</main>
                    </div>
                </div>
                <script>{ALERT_AUTO_DISMISS_JS}</script>
                <script>{SIDEBAR_TOGGLE_JS}</script>
            </body>
        </html>
    }
}

/// Collapses/expands the left sidebar, remembering the choice across navigations.
///
/// Plain JS rather than an island on purpose: the toggle has to apply the *stored* state before
/// first paint, and an island only hydrates after its WASM bundle downloads -- an expanded
/// sidebar would visibly flash and re-collapse on every page load for a user who'd chosen
/// collapsed. This script runs inline at parse time, so the layout is already correct when the
/// page first renders. `localStorage` access is wrapped because it throws outright in some
/// privacy modes rather than just returning null.
const SIDEBAR_TOGGLE_JS: &str = r"
(function(){
    var KEY = 'apich-sidebar-collapsed';
    // Below the responsive breakpoint the sidebar is a slide-over, so it starts closed unless
    // the user has explicitly opened it -- matching the `@media (max-width: 900px)` rules.
    function stored(){
        try {
            var v = localStorage.getItem(KEY);
            if (v === null) { return window.innerWidth <= 900; }
            return v === '1';
        } catch (e) { return window.innerWidth <= 900; }
    }
    function save(v){
        try { localStorage.setItem(KEY, v ? '1' : '0'); } catch (e) {}
    }
    function apply(collapsed){
        var layout = document.querySelector('.app-layout');
        if (layout) { layout.classList.toggle('sidebar-collapsed', collapsed); }
        var btn = document.getElementById('sidebar-toggle');
        if (btn) { btn.textContent = collapsed ? '»' : '«'; }
    }
    function init(){
        apply(stored());
        var btn = document.getElementById('sidebar-toggle');
        if (btn && !btn.__apichWired) {
            btn.__apichWired = true;
            btn.addEventListener('click', function(){
                var next = !stored();
                save(next);
                apply(next);
            });
        }
    }
    document.addEventListener('keydown', function(ev){
        if ((ev.ctrlKey || ev.metaKey) && (ev.key === 'b' || ev.key === 'B')) {
            ev.preventDefault();
            var next = !stored();
            save(next);
            apply(next);
        }
    });
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();
";

/// A server-rendered `.alert-success`/`.alert-danger` (a "File created", "Saved", etc. notice
/// from a `?notice=`/`?error=` query param baked into the initial HTML -- see `project_detail.rs`
/// and `document_editor_page.rs`) used to just sit on screen forever: it was static markup with
/// no dismiss affordance and no timer, so it only ever went away on a full navigation that
/// dropped the query param. A `MutationObserver` (not a one-shot scan on load) is required, not
/// optional, because pages built as Leptos islands mutate their own DOM after initial render --
/// e.g. `ConfirmSubmitButton`'s own success path, or any island that inserts a fresh alert client
/// side -- and a plain `querySelectorAll` at page-load time would silently miss all of those.
const ALERT_AUTO_DISMISS_JS: &str = r"
(function(){
    function dismiss(el){
        if (el.__apichDismissing) { return; }
        el.__apichDismissing = true;
        el.style.transition = 'opacity 0.3s ease, margin 0.3s ease, padding 0.3s ease, max-height 0.3s ease';
        el.style.maxHeight = el.offsetHeight + 'px';
        el.style.overflow = 'hidden';
        requestAnimationFrame(function(){
            el.style.opacity = '0';
            el.style.maxHeight = '0';
            el.style.marginTop = '0';
            el.style.marginBottom = '0';
            el.style.paddingTop = '0';
            el.style.paddingBottom = '0';
        });
        setTimeout(function(){ el.remove(); }, 320);
    }

    function wire(el){
        if (el.__apichWired) { return; }
        el.__apichWired = true;
        var close = document.createElement('button');
        close.type = 'button';
        close.setAttribute('aria-label', 'Dismiss');
        close.textContent = '×';
        close.style.cssText = 'float:right; background:none; border:none; cursor:pointer; font-size:1.1rem; line-height:1; color:inherit; opacity:0.6; padding:0 0 0 0.75rem; margin:-2px 0 0 0;';
        close.addEventListener('click', function(){ dismiss(el); });
        el.appendChild(close);
        setTimeout(function(){ dismiss(el); }, 5000);
    }

    function scan(root){
        root.querySelectorAll('.alert-success, .alert-danger').forEach(wire);
    }

    scan(document);
    new MutationObserver(function(mutations){
        mutations.forEach(function(m){
            m.addedNodes.forEach(function(node){
                if (node.nodeType !== 1) { return; }
                if (node.matches && (node.matches('.alert-success') || node.matches('.alert-danger'))) { wire(node); }
                if (node.querySelectorAll) { scan(node); }
            });
        });
    }).observe(document.body, {childList: true, subtree: true});
})();
";

/// Per-file sharing modal shared by every file-type studio (files tab, note editor,
/// document/slide editor). Real component now: `apich_islands::FileShareModalIsland`. Trigger
/// it from any "Share" button by dispatching the `apich-open-share-modal` window CustomEvent,
/// e.g. `onclick="window.dispatchEvent(new CustomEvent('apich-open-share-modal', {detail:
/// {path: 'foo.typ', mode: 'private', role: 'read', users: 'alice,bob'}}))"`.
#[component]
pub fn FileShareModal(
    project_id: Uuid,
    redirect_to: String,
    all_users: Vec<User>,
    owner_id: Uuid,
    i18n: I18n,
) -> impl IntoView {
    let _ = i18n;
    let shareable_users: Vec<apich_islands::ShareableUser> = all_users
        .into_iter()
        .filter(|u| u.id != owner_id)
        .map(|u| {
            apich_islands::ShareableUser {
                username: u.username,
                display_name: u.display_name,
            }
        })
        .collect();

    view! {
        <apich_islands::FileShareModalIsland
            project_id=project_id.to_string()
            redirect_to=redirect_to
            all_users=shareable_users
        />
    }
}

/// Thin wrapper around the real `AiDrawerIsland` (chat + real in-container agent runs + real
/// agent account login, all compiled Rust) -- kept as a plain server component so call sites
/// don't need to depend on `apich_islands` directly. `file_path` is optional: the chat tab uses
/// it to answer questions about a specific open file, but agent runs are always project-scoped,
/// so the drawer works the same with no file open (e.g. mounted on the project dashboard).
#[component]
pub fn AiDrawer(
    project_id: Uuid,
    #[prop(optional)] file_path: Option<String>,
) -> impl IntoView {
    view! { <apich_islands::AiDrawerIsland project_id=project_id.to_string() file_path=file_path /> }
}
