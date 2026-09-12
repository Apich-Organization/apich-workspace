use crate::app::components::ActiveNav;
use crate::app::components::AppShell;
use crate::ui::i18n::I18n;
use apich_db::OAuthClient;
use apich_db::SystemSettings;
use apich_db::User;
use leptos::prelude::*;

#[component]
pub fn AdminPlatformPage(
    user: User,
    settings: SystemSettings,
    sso_clients: Vec<OAuthClient>,
    notice: Option<String>,
    error: Option<String>,
    i18n: I18n,
    current_path: String,
) -> impl IntoView {
    let alert = notice.map_or_else(
        || {
            error.map(|e| {
                view! { <div class="alert alert-danger" style="margin-bottom:1.5rem;">{e}</div> }
                    .into_any()
            })
        },
        |n| {
            Some(
                view! { <div class="alert alert-success" style="margin-bottom:1.5rem;">{n}</div> }
                    .into_any(),
            )
        },
    );

    let pwd_placeholder = if settings.smtp_password.is_some() {
        "•••••••• (Password configured - leave empty to keep unchanged)".to_string()
    } else {
        "Enter SMTP password".to_string()
    };

    let sso_rows = if sso_clients.is_empty() {
        view! { <p class="text-muted" style="font-size:0.85rem;">{i18n.no_sso_clients()}</p> }
            .into_any()
    } else {
        let rows = sso_clients
            .into_iter()
            .map(|c| {
                view! {
                    <div class="client-item">
                        <div class="client-info">
                            <span class="client-name">{c.name}</span>
                            <span class="client-id">{format!("client_id: {}", c.client_id)}</span>
                        </div>
                        <span class="badge badge-active">"Registered"</span>
                    </div>
                }
            })
            .collect::<Vec<_>>();
        view! { <div class="client-list">{rows}</div> }.into_any()
    };

    view! {
        <AppShell
            user=user
            is_org_or_team_admin=true
            active_nav=ActiveNav::PlatformAdmin
            current_path=current_path
            page_title=i18n.nav_admin().to_string()
            i18n=i18n
        >
            <div class="page-header">
                <div>
                    <h1 class="page-title">{i18n.nav_admin()}</h1>
                </div>
            </div>
            {alert}

            <div class="detail-grid">
                <div class="section-card">
                    <h2 class="section-title">{i18n.registration_policy_title()}</h2>
                    <p class="text-muted" style="font-size:0.85rem; margin-top:0.25rem; margin-bottom:1rem;">
                        {i18n.registration_policy_subtitle()}
                    </p>
                    <form method="post" action="/admin/platform/settings">
                        <input type="hidden" name="section" value="registration" />
                        <div class="form-group">
                            <label>{i18n.registration_mode_label()}</label>
                            <select name="registration_mode" class="form-control" style="margin-top:0.5rem;">
                                <option value="invite_only" selected=settings.registration_mode == "invite_only">{i18n.registration_mode_invite()}</option>
                                <option value="open" selected=settings.registration_mode == "open">{i18n.registration_mode_open()}</option>
                                <option value="admin_only" selected=settings.registration_mode == "admin_only">"Platform Administrator Only"</option>
                            </select>
                        </div>
                        <button type="submit" class="btn btn-primary" style="margin-top:0.5rem;">{i18n.save_policy()}</button>
                    </form>
                </div>

                <div class="section-card" style="grid-column: span 2;">
                    <div style="display:flex; justify-content:space-between; align-items:center; border-bottom:1px solid var(--border-subtle); padding-bottom:1rem; margin-bottom:1.25rem;">
                        <div>
                            <h2 class="section-title" style="margin-bottom:0.25rem;">{i18n.smtp_config_title()}</h2>
                            <p class="text-muted" style="font-size:0.85rem;">{i18n.smtp_config_subtitle()}</p>
                        </div>
                        <div>
                            {if settings.smtp_enabled {
                                view! { <span class="role-badge role-badge-admin">"Active / Real Delivery"</span> }.into_any()
                            } else {
                                view! { <span class="role-badge role-badge-viewer">"Simulated Mode (In-Memory Queue)"</span> }.into_any()
                            }}
                        </div>
                    </div>

                    <form method="post" action="/admin/platform/settings">
                        <input type="hidden" name="section" value="smtp" />
                        <div class="form-group" style="margin-bottom:1.25rem; background:var(--bg-muted); padding:1rem; border-radius:8px; border:1px solid var(--border-subtle);">
                            <label style="display:flex; align-items:center; gap:0.6rem; cursor:pointer; font-weight:600;">
                                <input type="checkbox" name="smtp_enabled" value="true" checked=settings.smtp_enabled style="width:18px; height:18px;" />
                                <span>{i18n.smtp_enabled()}</span>
                            </label>
                        </div>

                        <div style="display:grid; grid-template-columns: 2fr 1fr; gap:1rem;">
                            <div class="form-group">
                                <label>{i18n.smtp_host()}</label>
                                <input type="text" name="smtp_host" value=settings.smtp_host.clone().unwrap_or_default() placeholder="smtp.institution.edu" class="form-control" />
                            </div>
                            <div class="form-group">
                                <label>{i18n.smtp_port()}</label>
                                <input type="number" name="smtp_port" value=settings.smtp_port.map_or_else(|| "587".to_string(), |p| p.to_string()) placeholder="587" class="form-control" />
                            </div>
                        </div>

                        <div style="display:grid; grid-template-columns: 1fr 1fr; gap:1rem;">
                            <div class="form-group">
                                <label>{i18n.smtp_username()}</label>
                                <input type="text" name="smtp_username" value=settings.smtp_username.clone().unwrap_or_default() placeholder="mailer@institution.edu" class="form-control" />
                            </div>
                            <div class="form-group">
                                <label>{i18n.smtp_password()}</label>
                                <input type="password" name="smtp_password" placeholder=pwd_placeholder class="form-control" />
                            </div>
                        </div>

                        <div style="display:grid; grid-template-columns: 1fr 1fr; gap:1rem;">
                            <div class="form-group">
                                <label>{i18n.smtp_from_email()}</label>
                                <input type="email" name="smtp_from_email" value=settings.smtp_from_email.clone().unwrap_or_default() placeholder="notifications@apich.org" class="form-control" />
                            </div>
                            <div class="form-group">
                                <label>{i18n.smtp_from_name()}</label>
                                <input type="text" name="smtp_from_name" value=settings.smtp_from_name.clone().unwrap_or_default() placeholder="APICH Platform" class="form-control" />
                            </div>
                        </div>

                        <div class="form-group" style="margin-bottom:1.25rem;">
                            <label style="display:flex; align-items:center; gap:0.6rem; cursor:pointer;">
                                <input type="checkbox" name="smtp_use_tls" value="true" checked=settings.smtp_use_tls style="width:16px; height:16px;" />
                                <span>{i18n.smtp_security()}</span>
                            </label>
                        </div>

                        <button type="submit" class="btn btn-primary">{i18n.save_smtp_settings()}</button>
                    </form>

                    <div style="margin-top:2rem; padding-top:1.5rem; border-top:1px solid var(--border-subtle);">
                        <h3 style="font-size:1.05rem; font-weight:600; margin-bottom:0.5rem; color:var(--text-main);">{i18n.test_smtp_title()}</h3>
                        <form method="post" action="/admin/platform/smtp-test" style="display:flex; gap:0.75rem; align-items:flex-end;">
                            <div class="form-group" style="flex:1; margin-bottom:0;">
                                <label>{i18n.test_recipient()}</label>
                                <input type="email" name="test_email" required=true placeholder="admin@lab.org" class="form-control" />
                            </div>
                            <button type="submit" class="btn btn-secondary">{i18n.send_test_email()}</button>
                        </form>
                    </div>
                </div>

                <div class="section-card" style="grid-column: span 3;">
                    <div style="display:flex; justify-content:space-between; align-items:center; border-bottom:1px solid var(--border-subtle); padding-bottom:1rem; margin-bottom:1.25rem;">
                        <div>
                            <h2 class="section-title" style="margin-bottom:0.25rem;">{i18n.sso_clients_title()}</h2>
                            <p class="text-muted" style="font-size:0.85rem;">{i18n.sso_clients_subtitle()}</p>
                        </div>
                        <a href="/.well-known/openid-configuration" target="_blank" class="btn btn-ghost btn-sm">
                            {i18n.view_openid_discovery()}
                        </a>
                    </div>
                    {sso_rows}
                </div>
            </div>
        </AppShell>
    }
}
