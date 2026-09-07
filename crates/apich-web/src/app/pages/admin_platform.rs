use crate::app::components::Navbar;
use leptos::prelude::*;

#[component]
pub fn AdminPlatformPage() -> impl IntoView {
    view! {
        <div class="admin-platform-page">
            <Navbar user_name="Platform Administrator".to_string() is_admin=true />

            <div class="main-content">
                <div class="page-header">
                    <div>
                        <h1 class="page-title">"Platform Administration"</h1>
                        <p class="page-subtitle">"Manage global registration policies, outbound SMTP communication, and Single Sign-On integrations"</p>
                    </div>
                </div>

                <div class="admin-grid">
                    // Card 1: Registration Mode Policy
                    <div class="section-card">
                        <h2 class="section-title">"Account Registration Policy"</h2>
                        <p class="text-muted">"Control how new researchers and academic collaborators gain access to the system"</p>

                        <form method="post" action="/api/admin/settings" class="admin-form">
                            <div class="radio-group">
                                <label class="radio-label">
                                    <input type="radio" name="registration_mode" value="invite_only" checked />
                                    <div class="radio-content">
                                        <span class="radio-title">"Invitation Only (Recommended)"</span>
                                        <span class="radio-desc">"Users must receive an email invitation or enter an administrator-issued invite token"</span>
                                    </div>
                                </label>

                                <label class="radio-label">
                                    <input type="radio" name="registration_mode" value="open" />
                                    <div class="radio-content">
                                        <span class="radio-title">"Open Self-Registration"</span>
                                        <span class="radio-desc">"Anyone with institutional access can register an account without prior invitation"</span>
                                    </div>
                                </label>

                                <label class="radio-label">
                                    <input type="radio" name="registration_mode" value="admin_only" />
                                    <div class="radio-content">
                                        <span class="radio-title">"Administrator Only"</span>
                                        <span class="radio-desc">"Self-serve signup is disabled; accounts can only be created via administrator provisioning"</span>
                                    </div>
                                </label>
                            </div>
                            <button type="submit" class="btn btn-secondary">"Save Registration Policy"</button>
                        </form>
                    </div>

                    // Card 2: Outbound Email & SMTP Settings
                    <div class="section-card">
                        <h2 class="section-title">"SMTP Mail Server Configuration"</h2>
                        <p class="text-muted">"Enables automated invitation dispatch, password recovery, and container status notifications"</p>

                        <form method="post" action="/api/admin/settings" class="admin-form">
                            <div class="form-row">
                                <div class="form-group flex-2">
                                    <label for="smtp_host">"SMTP Host"</label>
                                    <input type="text" id="smtp_host" name="smtp_host" placeholder="smtp.institution.org" class="form-control" />
                                </div>
                                <div class="form-group flex-1">
                                    <label for="smtp_port">"Port"</label>
                                    <input type="number" id="smtp_port" name="smtp_port" placeholder="587" class="form-control" />
                                </div>
                            </div>

                            <div class="form-row">
                                <div class="form-group flex-1">
                                    <label for="smtp_user">"Username"</label>
                                    <input type="text" id="smtp_user" name="smtp_username" placeholder="mailer@institution.org" class="form-control" />
                                </div>
                                <div class="form-group flex-1">
                                    <label for="smtp_pwd">"Password"</label>
                                    <input type="password" id="smtp_pwd" name="smtp_password" placeholder="••••••••" class="form-control" />
                                </div>
                            </div>

                            <div class="form-row">
                                <div class="form-group flex-1">
                                    <label for="from_email">"Sender Address"</label>
                                    <input type="email" id="from_email" name="smtp_from_email" placeholder="noreply@apich.org" class="form-control" />
                                </div>
                                <div class="form-group flex-1">
                                    <label for="from_name">"Display Sender Name"</label>
                                    <input type="text" id="from_name" name="smtp_from_name" placeholder="APICH Research Suite" class="form-control" />
                                </div>
                            </div>

                            <div class="form-actions">
                                <button type="submit" class="btn btn-primary">"Update SMTP Settings"</button>
                                <button type="button" class="btn btn-ghost" id="btn-test-email">"Send Test Email"</button>
                            </div>
                        </form>
                    </div>

                    // Card 3: Single Sign-On (SSO) Platform Provider
                    <div class="section-card">
                        <div class="section-header">
                            <div>
                                <h2 class="section-title">"Single Sign-On (SSO) Platform Provider"</h2>
                                <p class="text-muted">"APICH acts as an OpenID Connect & OAuth2 identity provider for JupyterLab, Grafana, and cluster tools"</p>
                            </div>
                            <a href="/.well-known/openid-configuration" target="_blank" class="btn btn-ghost btn-sm">
                                "View OpenID Discovery Document"
                            </a>
                        </div>

                        <div class="client-list">
                            <div class="client-item">
                                <div class="client-info">
                                    <span class="client-name">"JupyterLab Academic Cluster Hub"</span>
                                    <span class="client-id">"client_id: jupyterlab-hub"</span>
                                </div>
                                <span class="badge badge-active">"Authorized"</span>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}
