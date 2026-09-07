use leptos::prelude::*;

#[component]
pub fn Navbar(
    #[prop(optional)] user_name: Option<String>,
    #[prop(optional)] is_admin: Option<bool>,
) -> impl IntoView {
    let name = user_name.unwrap_or_else(|| "Researcher".to_string());
    let admin = is_admin.unwrap_or(false);

    view! {
        <header class="navbar">
            <div class="nav-brand">
                <a href="/" class="brand-link">
                    <span class="brand-badge">"APICH"</span>
                    <span class="brand-title">"Research Workspace"</span>
                </a>
            </div>
            <nav class="nav-links">
                <a href="/" class="nav-item">"Projects"</a>
                <a href="/admin/orgs" class="nav-item">"Teams & Directory"</a>
                {if admin {
                    view! { <a href="/admin/platform" class="nav-item nav-admin">"Platform Administration"</a> }.into_any()
                } else {
                    view! { <span /> }.into_any()
                }}
                <a href="/settings" class="nav-item">"Settings"</a>
            </nav>
            <div class="nav-user">
                <span class="user-greeting">{format!("Signed in as {}", name)}</span>
                <form method="post" action="/api/auth/logout" class="logout-form">
                    <button type="submit" class="btn btn-ghost btn-sm">"Sign Out"</button>
                </form>
            </div>
        </header>
    }
}

#[component]
pub fn StatCard(
    title: &'static str,
    value: String,
    #[prop(optional)] subtitle: Option<&'static str>,
) -> impl IntoView {
    view! {
        <div class="stat-card">
            <span class="stat-title">{title}</span>
            <span class="stat-value">{value}</span>
            {if let Some(sub) = subtitle {
                view! { <span class="stat-subtitle">{sub}</span> }.into_any()
            } else {
                view! { <span /> }.into_any()
            }}
        </div>
    }
}

#[component]
pub fn StatusBadge(status: &'static str, #[prop(optional)] is_running: Option<bool>) -> impl IntoView {
    let running = is_running.unwrap_or(status == "running" || status == "active");
    let class_name = if running {
        "status-badge badge-active"
    } else {
        "status-badge badge-idle"
    };

    view! {
        <span class={class_name}>
            <span class="status-dot"></span>
            {status}
        </span>
    }
}
