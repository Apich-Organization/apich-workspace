use crate::app::components::Navbar;
use leptos::prelude::*;

#[component]
pub fn SettingsPage() -> impl IntoView {
    view! {
        <div class="settings-page">
            <Navbar user_name="Dr. Researcher".to_string() is_admin=true />

            <div class="main-content">
                <div class="page-header">
                    <div>
                        <h1 class="page-title">"Account & Security Settings"</h1>
                        <p class="page-subtitle">"Manage your personal identity profile, hardware passkeys, and authentication preferences"</p>
                    </div>
                </div>

                <div class="settings-grid">
                    // Profile Card
                    <div class="section-card">
                        <h2 class="section-title">"Profile Information"</h2>
                        <div class="form-group">
                            <label>"Display Name"</label>
                            <input type="text" value="Dr. Researcher" readonly class="form-control readonly" />
                        </div>
                        <div class="form-group">
                            <label>"Email Address"</label>
                            <input type="email" value="researcher@lab.org" readonly class="form-control readonly" />
                        </div>
                        <div class="form-group">
                            <label>"Role"</label>
                            <input type="text" value="Platform Administrator" readonly class="form-control readonly" />
                        </div>
                    </div>

                    // Passkeys / FIDO2 Card
                    <div class="section-card">
                        <div class="section-header">
                            <div>
                                <h2 class="section-title">"Passkeys & Hardware Security Keys"</h2>
                                <p class="text-muted">"Phishing-resistant authentication via WebAuthn / FIDO2 Level 3 standards"</p>
                            </div>
                            <button class="btn btn-primary btn-sm" id="btn-add-passkey">
                                "+ Register New Passkey"
                            </button>
                        </div>

                        <div class="passkey-list">
                            <div class="passkey-item">
                                <div class="passkey-icon">"🔑"</div>
                                <div class="passkey-info">
                                    <span class="passkey-name">"YubiKey 5C NFC (Primary)"</span>
                                    <span class="passkey-meta">"Enrolled 2 weeks ago • Last used today"</span>
                                </div>
                                <span class="badge badge-active">"Active"</span>
                            </div>

                            <div class="passkey-item">
                                <div class="passkey-icon">"💻"</div>
                                <div class="passkey-info">
                                    <span class="passkey-name">"Touch ID / Platform Biometrics"</span>
                                    <span class="passkey-meta">"Enrolled 1 month ago"</span>
                                </div>
                                <span class="badge badge-active">"Active"</span>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}
