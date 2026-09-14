use crate::app::components::ActiveNav;
use crate::app::components::AppShell;
use crate::ui::i18n::I18n;
use apich_db::Invitation;
use apich_db::OAuthClient;
use apich_db::SystemSettings;
use apich_db::User;
use leptos::prelude::*;

#[component]
pub fn AdminPlatformPage(
    user: User,
    settings: SystemSettings,
    sso_clients: Vec<OAuthClient>,
    invitations: Vec<Invitation>,
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

    let current_sec = if settings.smtp_force_tls
        || (settings.smtp_port == Some(465) && settings.smtp_use_tls)
    {
        "force_tls"
    } else if settings.smtp_use_tls {
        "starttls"
    } else {
        "none"
    };

    let now = chrono::Utc::now();
    let invite_rows = if invitations.is_empty() {
        view! { <p class="text-muted" style="font-size:0.85rem; padding:1rem 0;">"No invitation codes created yet."</p> }.into_any()
    } else {
        let rows = invitations
            .into_iter()
            .map(|inv| {
                let is_expired = now > inv.expires_at;
                let is_exhausted = inv.used_count >= inv.max_uses;
                let (badge_cls, status_text) = if is_expired {
                    ("badge badge-viewer", "Expired")
                } else if is_exhausted {
                    ("badge badge-viewer", "Exhausted")
                } else {
                    ("badge badge-active", "Active")
                };

                let recipient_text = inv
                    .email
                    .filter(|e| !e.trim().is_empty())
                    .unwrap_or_else(|| "Open (Anyone)".to_string());
                let expires_str = inv.expires_at.format("%Y-%m-%d %H:%M").to_string();
                let invite_id = inv.id.to_string();

                view! {
                    <tr style="border-bottom:1px solid var(--border-subtle);">
                        <td style="padding:10px 8px;">
                            <code style="font-weight:600; font-size:0.95rem; color:var(--primary); background:var(--bg-muted); padding:3px 8px; border-radius:4px; border:1px solid var(--border-subtle);">
                                {inv.token.clone()}
                            </code>
                        </td>
                        <td style="padding:10px 8px;">
                            <span class=badge_cls>{status_text}</span>
                        </td>
                        <td style="padding:10px 8px; font-weight:600;">
                            {format!("{} / {}", inv.used_count, inv.max_uses)}
                        </td>
                        <td style="padding:10px 8px;">
                            <span class="text-muted" style="font-size:0.85rem;">{recipient_text}</span>
                        </td>
                        <td style="padding:10px 8px;">
                            <span style="text-transform:capitalize; font-size:0.85rem;">{inv.role}</span>
                        </td>
                        <td style="padding:10px 8px; font-size:0.85rem;" class="text-muted">
                            {expires_str}
                        </td>
                        <td style="padding:10px 8px; text-align:right;">
                            <form method="post" action="/admin/invitations/delete" style="display:inline;">
                                <input type="hidden" name="id" value=invite_id />
                                <button type="submit" class="btn btn-danger btn-sm" style="padding:2px 8px; font-size:0.75rem;">
                                    {i18n.revoke_btn()}
                                </button>
                            </form>
                        </td>
                    </tr>
                }
            })
            .collect::<Vec<_>>();

        view! {
            <div style="overflow-x:auto; margin-top:1rem;">
                <table class="table" style="width:100%; border-collapse:collapse;">
                    <thead>
                        <tr style="border-bottom:1px solid var(--border-subtle); text-align:left; font-size:0.8rem; color:var(--text-muted);">
                            <th style="padding:8px;">"Code"</th>
                            <th style="padding:8px;">"Status"</th>
                            <th style="padding:8px;">"Uses"</th>
                            <th style="padding:8px;">"Recipient"</th>
                            <th style="padding:8px;">"Role"</th>
                            <th style="padding:8px;">"Expires At"</th>
                            <th style="padding:8px; text-align:right;">"Action"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {rows}
                    </tbody>
                </table>
            </div>
        }.into_any()
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

            <div class="admin-platform-grid">
                // Left Card: Platform Access & Security Policy
                <div class="section-card" style="display:flex; flex-direction:column; justify-content:space-between; margin-bottom:0;">
                    <div>
                        <div style="display:flex; justify-content:space-between; align-items:flex-start; border-bottom:1px solid var(--border-subtle); padding-bottom:1rem; margin-bottom:1.25rem;">
                            <div>
                                <h2 class="section-title" style="margin-bottom:0.25rem;">
                                    "🛡️ " {i18n.registration_policy_title()}
                                </h2>
                                <p class="text-muted" style="font-size:0.85rem; margin:0;">
                                    {i18n.registration_policy_subtitle()}
                                </p>
                            </div>
                            <div>
                                {match settings.registration_mode.as_str() {
                                    "open" => view! { <span class="badge badge-active">"Open Registration"</span> }.into_any(),
                                    "admin_only" => view! { <span class="badge badge-viewer">"Admin Only"</span> }.into_any(),
                                    _ => view! { <span class="badge badge-idle">"Invite Code Required"</span> }.into_any(),
                                }}
                            </div>
                        </div>

                        <form method="post" action="/admin/platform/settings">
                            <input type="hidden" name="section" value="registration" />

                            // Registration Mode Selection
                            <div class="form-group" style="margin-bottom:1.25rem;">
                                <label for="registration_mode" style="font-weight:600; font-size:0.875rem; display:block; margin-bottom:0.4rem;">
                                    {i18n.registration_mode_label()}
                                </label>
                                <select
                                    id="registration_mode"
                                    name="registration_mode"
                                    class="form-control"
                                    style="width:100%; padding:0.5rem 0.75rem; font-size:0.875rem;"
                                >
                                    <option value="invite_only" selected=settings.registration_mode == "invite_only">
                                        {i18n.registration_mode_invite()}
                                    </option>
                                    <option value="open" selected=settings.registration_mode == "open">
                                        {i18n.registration_mode_open()}
                                    </option>
                                    <option value="admin_only" selected=settings.registration_mode == "admin_only">
                                        "Platform Administrator Only"
                                    </option>
                                </select>
                                <div style="margin-top:0.6rem; padding:0.65rem 0.85rem; background:var(--bg-muted); border:1px solid var(--border-subtle); border-radius:6px; font-size:0.8rem; color:var(--text-muted); line-height:1.45;">
                                    {match settings.registration_mode.as_str() {
                                        "open" => "🌐 Anyone with an email address can create an account directly on the sign-up page.",
                                        "admin_only" => "🔒 Self-registration is disabled. Only platform administrators can provision accounts.",
                                        _ => "🎫 New users must present a valid, unexpired invitation code during registration.",
                                    }}
                                </div>
                            </div>

                            // Global 2FA Policy Section
                            <div class="form-group" style="margin-bottom:1.25rem; padding-top:1.15rem; border-top:1px solid var(--border-subtle);">
                                <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:0.5rem;">
                                    <label style="font-weight:600; font-size:0.875rem; margin:0;">
                                        {i18n.global_2fa_title()}
                                    </label>
                                    {if settings.require_2fa {
                                        view! { <span class="badge badge-active" style="font-size:0.75rem;">"2FA Enforced"</span> }.into_any()
                                    } else {
                                        view! { <span class="badge badge-viewer" style="font-size:0.75rem;">"2FA Optional"</span> }.into_any()
                                    }}
                                </div>

                                <div style="background:var(--bg-muted); border:1px solid var(--border-subtle); border-radius:8px; padding:0.85rem 1rem; margin-bottom:0.65rem;">
                                    <label style="display:flex; align-items:flex-start; gap:0.65rem; cursor:pointer; font-weight:600; font-size:0.875rem; margin:0;">
                                        <input
                                            type="checkbox"
                                            name="require_2fa"
                                            value="true"
                                            checked=settings.require_2fa
                                            style="width:18px; height:18px; margin-top:0.1rem; accent-color:var(--primary); cursor:pointer;"
                                        />
                                        <div>
                                            <span>{i18n.global_2fa_enforce()}</span>
                                            <p class="text-muted" style="font-weight:400; font-size:0.8rem; margin:0.25rem 0 0 0; line-height:1.45;">
                                                {i18n.global_2fa_desc()}
                                            </p>
                                        </div>
                                    </label>
                                </div>

                                <div style="display:flex; flex-wrap:wrap; gap:0.4rem; margin-top:0.6rem;">
                                    <span style="font-size:0.75rem; background:var(--bg-muted); border:1px solid var(--border-subtle); border-radius:12px; padding:2px 8px; color:var(--text-sub);">
                                        "📱 Authenticator (SHA-512, 8-digit)"
                                    </span>
                                    <span style="font-size:0.75rem; background:var(--bg-muted); border:1px solid var(--border-subtle); border-radius:12px; padding:2px 8px; color:var(--text-sub);">
                                        "✉️ Email Code (Zero-Lockout)"
                                    </span>
                                    <span style="font-size:0.75rem; background:var(--bg-muted); border:1px solid var(--border-subtle); border-radius:12px; padding:2px 8px; color:var(--text-sub);">
                                        "🔑 Passkey (FIDO2)"
                                    </span>
                                </div>
                            </div>

                            <button type="submit" class="btn btn-primary" style="margin-top:0.5rem; padding:0.5rem 1.25rem;">
                                {i18n.save_policy()}
                            </button>
                        </form>
                    </div>
                </div>

                // Right Card: Outbound Email (SMTP) Configuration
                <div class="section-card" style="display:flex; flex-direction:column; justify-content:space-between; margin-bottom:0;">
                    <div>
                        <div style="display:flex; justify-content:space-between; align-items:flex-start; border-bottom:1px solid var(--border-subtle); padding-bottom:1rem; margin-bottom:1.25rem;">
                            <div>
                                <h2 class="section-title" style="margin-bottom:0.25rem;">
                                    "✉️ " {i18n.smtp_config_title()}
                                </h2>
                                <p class="text-muted" style="font-size:0.85rem; margin:0;">
                                    {i18n.smtp_config_subtitle()}
                                </p>
                            </div>
                            <div>
                                {if settings.smtp_enabled {
                                    view! { <span class="role-badge role-badge-admin">"Active / Real Delivery"</span> }.into_any()
                                } else {
                                    view! { <span class="role-badge role-badge-viewer">"Simulated Mode"</span> }.into_any()
                                }}
                            </div>
                        </div>

                        <form method="post" action="/admin/platform/settings">
                            <input type="hidden" name="section" value="smtp" />
                            <div class="form-group" style="margin-bottom:1.25rem; background:var(--bg-muted); padding:0.85rem 1rem; border-radius:8px; border:1px solid var(--border-subtle);">
                                <label style="display:flex; align-items:center; gap:0.6rem; cursor:pointer; font-weight:600; font-size:0.875rem; margin:0;">
                                    <input type="checkbox" name="smtp_enabled" value="true" checked=settings.smtp_enabled style="width:18px; height:18px; accent-color:var(--primary); cursor:pointer;" />
                                    <span>{i18n.smtp_enabled()}</span>
                                </label>
                            </div>

                            <div style="display:grid; grid-template-columns: 2fr 1fr; gap:0.85rem;">
                                <div class="form-group">
                                    <label style="font-size:0.85rem; font-weight:500;">{i18n.smtp_host()}</label>
                                    <input type="text" name="smtp_host" value=settings.smtp_host.clone().unwrap_or_default() placeholder="smtp.institution.edu" class="form-control" />
                                </div>
                                <div class="form-group">
                                    <label style="font-size:0.85rem; font-weight:500;">{i18n.smtp_port()}</label>
                                    <input type="number" id="smtp_port" name="smtp_port" value=settings.smtp_port.map_or_else(|| "587".to_string(), |p| p.to_string()) placeholder="587" class="form-control" />
                                </div>
                            </div>

                            <div style="display:grid; grid-template-columns: 1fr 1fr; gap:0.85rem;">
                                <div class="form-group">
                                    <label style="font-size:0.85rem; font-weight:500;">{i18n.smtp_username()}</label>
                                    <input type="text" name="smtp_username" value=settings.smtp_username.clone().unwrap_or_default() placeholder="mailer@institution.edu" class="form-control" />
                                </div>
                                <div class="form-group">
                                    <label style="font-size:0.85rem; font-weight:500;">{i18n.smtp_password()}</label>
                                    <input type="password" name="smtp_password" placeholder=pwd_placeholder class="form-control" />
                                </div>
                            </div>

                            <div style="display:grid; grid-template-columns: 1fr 1fr; gap:0.85rem;">
                                <div class="form-group">
                                    <label style="font-size:0.85rem; font-weight:500;">{i18n.smtp_from_email()}</label>
                                    <input type="email" name="smtp_from_email" value=settings.smtp_from_email.clone().unwrap_or_default() placeholder="notifications@apich.org" class="form-control" />
                                </div>
                                <div class="form-group">
                                    <label style="font-size:0.85rem; font-weight:500;">{i18n.smtp_from_name()}</label>
                                    <input type="text" name="smtp_from_name" value=settings.smtp_from_name.clone().unwrap_or_default() placeholder="APICH Platform" class="form-control" />
                                </div>
                            </div>

                            <div class="form-group" style="margin-bottom:1.25rem;">
                                <label for="smtp_security" style="display:block; margin-bottom:0.4rem; font-size:0.85rem; font-weight:500;">
                                    {i18n.smtp_encryption_mode()}
                                </label>
                                <select
                                    id="smtp_security"
                                    name="smtp_security"
                                    class="form-control"
                                    style="width:100%;"
                                    onchange="var p=document.getElementById('smtp_port');if(p){if(this.value==='force_tls'&&(p.value==='587'||p.value==='25'||!p.value))p.value='465';else if(this.value==='starttls'&&(p.value==='465'||p.value==='25'||!p.value))p.value='587';else if(this.value==='none'&&(p.value==='465'||p.value==='587'||!p.value))p.value='25';}"
                                >
                                    <option value="force_tls" selected={current_sec == "force_tls"}>{i18n.smtp_sec_force_tls()}</option>
                                    <option value="starttls" selected={current_sec == "starttls"}>{i18n.smtp_sec_starttls()}</option>
                                    <option value="none" selected={current_sec == "none"}>{i18n.smtp_sec_none()}</option>
                                </select>
                                <small class="text-muted" style="display:block; margin-top:0.35rem; font-size:0.78rem; line-height:1.4;">
                                    "Force TLS (Port 465) connects with TLS immediately. STARTTLS (Port 587) negotiates TLS over plain connection."
                                </small>
                            </div>

                            <button type="submit" class="btn btn-primary" style="padding:0.5rem 1.25rem;">
                                {i18n.save_smtp_settings()}
                            </button>
                        </form>
                    </div>

                    <div style="margin-top:1.5rem; padding-top:1.25rem; border-top:1px solid var(--border-subtle);">
                        <h3 style="font-size:0.95rem; font-weight:600; margin-bottom:0.5rem; color:var(--text-main);">
                            {i18n.test_smtp_title()}
                        </h3>
                        <form method="post" action="/admin/platform/smtp-test" style="display:flex; gap:0.6rem; align-items:flex-end;">
                            <div class="form-group" style="flex:1; margin-bottom:0;">
                                <input type="email" name="test_email" required=true placeholder="test-admin@lab.org" class="form-control" style="font-size:0.875rem;" />
                            </div>
                            <button type="submit" class="btn btn-secondary" style="white-space:nowrap; padding:0.45rem 0.9rem; font-size:0.85rem;">
                                {i18n.send_test_email()}
                            </button>
                        </form>
                    </div>
                </div>
            </div>

            // Card 3: Invitation Codes (Full-width)
            <div class="section-card" style="margin-top:1.5rem;">
                <div style="display:flex; justify-content:space-between; align-items:center; border-bottom:1px solid var(--border-subtle); padding-bottom:1rem; margin-bottom:1.25rem;">
                    <div>
                        <h2 class="section-title" style="margin-bottom:0.25rem;">
                            "🎫 " {i18n.invitation_codes_title()}
                        </h2>
                        <p class="text-muted" style="font-size:0.85rem; margin:0;">
                            {i18n.invitation_codes_desc()}
                        </p>
                    </div>
                </div>

                <form method="post" action="/admin/invitations/new" style="background:var(--bg-muted); padding:1.25rem; border-radius:8px; border:1px solid var(--border-subtle); margin-bottom:1.5rem;">
                    <h3 style="font-size:0.95rem; font-weight:600; margin-bottom:1rem; color:var(--text-main);">"Create New Invitation Code"</h3>
                    <div style="display:grid; grid-template-columns: 2fr 1fr 1fr 2fr 1fr; gap:0.75rem; align-items:flex-end;">
                        <div class="form-group" style="margin-bottom:0;">
                            <label style="font-size:0.8rem;">{i18n.code_optional_hint()}</label>
                            <input type="text" name="code" placeholder="e.g. LAB-2026-FALL" class="form-control" />
                        </div>
                        <div class="form-group" style="margin-bottom:0;">
                            <label style="font-size:0.8rem;">{i18n.max_uses_label()}</label>
                            <input type="number" name="max_uses" min="1" value="1" required=true class="form-control" />
                        </div>
                        <div class="form-group" style="margin-bottom:0;">
                            <label style="font-size:0.8rem;">{i18n.expires_in_days_label()}</label>
                            <input type="number" name="expires_in_days" min="1" max="365" value="7" required=true class="form-control" />
                        </div>
                        <div class="form-group" style="margin-bottom:0;">
                            <label style="font-size:0.8rem;">{i18n.email_restriction_hint()}</label>
                            <input type="email" name="email" placeholder="researcher@lab.org" class="form-control" />
                        </div>
                        <div class="form-group" style="margin-bottom:0;">
                            <label style="font-size:0.8rem;">"Role"</label>
                            <select name="role" class="form-control">
                                <option value="member">"Member"</option>
                                <option value="guest">"Guest"</option>
                                <option value="admin">"Admin"</option>
                            </select>
                        </div>
                    </div>
                    <button type="submit" class="btn btn-primary btn-sm" style="margin-top:1rem;">
                        "+ " {i18n.generate_code_btn()}
                    </button>
                </form>

                <h3 style="font-size:0.95rem; font-weight:600; margin-bottom:0.5rem; color:var(--text-main);">"Active & Past Invitation Codes"</h3>
                {invite_rows}
            </div>

            // Card 4: SSO / OAuth Clients (Full-width)
            <div class="section-card">
                <div style="display:flex; justify-content:space-between; align-items:center; border-bottom:1px solid var(--border-subtle); padding-bottom:1rem; margin-bottom:1.25rem;">
                    <div>
                        <h2 class="section-title" style="margin-bottom:0.25rem;">
                            "🔗 " {i18n.sso_clients_title()}
                        </h2>
                        <p class="text-muted" style="font-size:0.85rem; margin:0;">
                            {i18n.sso_clients_subtitle()}
                        </p>
                    </div>
                    <a href="/.well-known/openid-configuration" target="_blank" class="btn btn-ghost btn-sm">
                        {i18n.view_openid_discovery()}
                    </a>
                </div>
                {sso_rows}
            </div>
        </AppShell>
    }
}
