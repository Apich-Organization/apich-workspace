pub mod components;
pub mod pages;
pub mod styles;

pub use styles::EMBEDDED_CSS;

use leptos::prelude::*;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::path;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <style>{EMBEDDED_CSS}</style>
        <Router>
            <div class="app-container">
                <main>
                    <Routes fallback=|| view! { <p>"Page not found"</p> }>
                        <Route path=path!("/") view=pages::dashboard::DashboardPage />
                        <Route path=path!("/login") view=pages::login::LoginPage />
                        <Route path=path!("/register") view=pages::register::RegisterPage />
                        <Route path=path!("/settings") view=pages::settings::SettingsPage />
                        <Route path=path!("/projects/:id") view=pages::project_detail::ProjectDetailPage />
                        <Route path=path!("/admin/orgs") view=pages::org_teams::OrgTeamsPage />
                        <Route path=path!("/admin/platform") view=pages::admin_platform::AdminPlatformPage />
                    </Routes>
                </main>
            </div>
        </Router>
    }
}
