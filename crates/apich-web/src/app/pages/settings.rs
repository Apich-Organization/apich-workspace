use crate::app::components::ActiveNav;
use crate::app::components::AppShell;
use crate::ui::i18n::I18n;
use apich_db::Fido2Credential;
use apich_db::GpgPublicKey;
use apich_db::PersonalAccessToken;
use apich_db::SshPublicKey;
use apich_db::User;
use apich_islands::PasskeyEnrollIsland;
use leptos::prelude::*;

#[allow(clippy::too_many_arguments)]
#[component]
pub fn SettingsPage(
    user: User,
    is_org_or_team_admin: bool,
    passkeys: Vec<Fido2Credential>,
    pats: Vec<PersonalAccessToken>,
    ssh_keys: Vec<SshPublicKey>,
    gpg_keys: Vec<GpgPublicKey>,
    new_pat_token: Option<String>,
    notice: Option<String>,
    error: Option<String>,
    i18n: I18n,
    current_path: String,
) -> impl IntoView {
    let alert = if let Some(n) = notice {
        Some(view! { <div class="alert alert-success">{n}</div> }.into_any())
    } else {
        error.map(|e| view! { <div class="alert alert-danger">{e}</div> }.into_any())
    };

    let role_display = if user.is_platform_admin {
        i18n.platform_admin().to_string()
    } else if is_org_or_team_admin {
        i18n.org_admin_title().to_string()
    } else {
        i18n.researcher().to_string()
    };

    let no_passkeys_text = if i18n.is_zh() {
        "尚未注册任何通行密钥"
    } else {
        "No passkeys registered yet"
    };
    let passkey_list = if passkeys.is_empty() {
        view! {
            <p class="text-muted" style="font-size:0.85rem;">{no_passkeys_text}</p>
        }
        .into_any()
    } else {
        let items = passkeys
            .into_iter()
            .map(|cred| {
                let last_used = cred
                    .last_used_at
                    .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| "Never".to_string());
                let created = cred.created_at.format("%Y-%m-%d").to_string();
                view! {
                    <div class="passkey-item">
                        <div class="passkey-icon">"🔑"</div>
                        <div class="passkey-info">
                            <span class="passkey-name">{cred.device_name}</span>
                            <span class="passkey-meta">{format!("Enrolled {} • Last used {}", created, last_used)}</span>
                        </div>
                        <span class="badge badge-active">"Active"</span>
                    </div>
                }
            })
            .collect::<Vec<_>>();
        view! { <div class="passkey-list">{items}</div> }.into_any()
    };

    let new_pat_banner = new_pat_token.map(|token| view! {
        <div class="alert alert-success" style="font-family:var(--font-mono); word-break:break-all;">
            <strong>"New token created. Copy it now — it will not be shown again:"</strong><br/>
            {token}
        </div>
    });

    let no_pats_text = "No personal access tokens yet.";
    let pat_list = if pats.is_empty() {
        view! { <p class="text-muted" style="font-size:0.85rem;">{no_pats_text}</p> }.into_any()
    } else {
        let items = pats.into_iter().map(|t| {
            let created = t.created_at.format("%Y-%m-%d").to_string();
            let last_used = t.last_used_at.map(|d| d.format("%Y-%m-%d %H:%M").to_string()).unwrap_or_else(|| "Never".to_string());
            let is_active = t.is_active();
            let status_label = if is_active { "Active" } else { "Revoked/Expired" };
            let status_class = if is_active { "badge badge-active" } else { "badge badge-idle" };
            view! {
                <div class="passkey-item">
                    <div class="passkey-icon">"🔑"</div>
                    <div class="passkey-info">
                        <span class="passkey-name">{t.name} " (" {t.token_prefix} "…)"</span>
                        <span class="passkey-meta">{format!("Created {} • Last used {}", created, last_used)}</span>
                    </div>
                    <span class=status_class>{status_label}</span>
                    {is_active.then(|| view! {
                        <form method="post" action=format!("/settings/pat/{}/revoke", t.id) class="inline-form">
                            <button type="submit" class="btn btn-danger btn-sm">"Revoke"</button>
                        </form>
                    })}
                </div>
            }
        }).collect::<Vec<_>>();
        view! { <div class="passkey-list">{items}</div> }.into_any()
    };

    let no_ssh_text = "No SSH keys added yet.";
    let ssh_list = if ssh_keys.is_empty() {
        view! { <p class="text-muted" style="font-size:0.85rem;">{no_ssh_text}</p> }.into_any()
    } else {
        let items = ssh_keys.into_iter().map(|k| {
            let created = k.created_at.format("%Y-%m-%d").to_string();
            view! {
                <div class="passkey-item">
                    <div class="passkey-icon">"🗝️"</div>
                    <div class="passkey-info">
                        <span class="passkey-name">{k.name} " (" {k.key_type} ")"</span>
                        <span class="passkey-meta">{format!("{} • Added {}", k.fingerprint, created)}</span>
                    </div>
                    <form method="post" action=format!("/settings/ssh/{}/delete", k.id) class="inline-form">
                        <button type="submit" class="btn btn-danger btn-sm">"Delete"</button>
                    </form>
                </div>
            }
        }).collect::<Vec<_>>();
        view! { <div class="passkey-list">{items}</div> }.into_any()
    };

    let no_gpg_text = "No GPG keys added yet.";
    let gpg_list = if gpg_keys.is_empty() {
        view! { <p class="text-muted" style="font-size:0.85rem;">{no_gpg_text}</p> }.into_any()
    } else {
        let items = gpg_keys.into_iter().map(|k| {
            let created = k.created_at.format("%Y-%m-%d").to_string();
            view! {
                <div class="passkey-item">
                    <div class="passkey-icon">"🔏"</div>
                    <div class="passkey-info">
                        <span class="passkey-name">{k.name}</span>
                        <span class="passkey-meta">{format!("{} • Added {}", k.fingerprint, created)}</span>
                    </div>
                    <form method="post" action=format!("/settings/gpg/{}/delete", k.id) class="inline-form">
                        <button type="submit" class="btn btn-danger btn-sm">"Delete"</button>
                    </form>
                </div>
            }
        }).collect::<Vec<_>>();
        view! { <div class="passkey-list">{items}</div> }.into_any()
    };

    view! {
        <AppShell
            user=user.clone()
            is_org_or_team_admin=is_org_or_team_admin
            active_nav=ActiveNav::Settings
            current_path=current_path
            page_title=i18n.settings_title().to_string()
            i18n=i18n
        >
            <div class="page-header">
                <div>
                    <h1 class="page-title">{i18n.settings_title()}</h1>
                </div>
            </div>
            {alert}

            <div class="settings-grid">
                <div class="section-card">
                    <h2 class="section-title">{i18n.profile_card()}</h2>
                    <form method="post" action="/settings/profile">
                        <div class="form-group">
                            <label for="display_name">{i18n.display_name()}</label>
                            <input type="text" id="display_name" name="display_name" value=user.display_name.clone() required=true class="form-control" />
                        </div>
                        <div class="form-group">
                            <label for="email">{i18n.email()}</label>
                            <input type="email" id="email" name="email" value=user.email.clone() required=true class="form-control" />
                        </div>
                        <div class="form-group">
                            <label for="avatar_url">{i18n.avatar_url()}</label>
                            <input type="url" id="avatar_url" name="avatar_url" value=user.avatar_url.clone().unwrap_or_default() placeholder="https://example.com/avatar.png" class="form-control" />
                        </div>
                        <div class="form-group">
                            <label>"Role"</label>
                            <input type="text" value=role_display readonly=true class="form-control readonly" />
                        </div>
                        <button type="submit" class="btn btn-primary">{i18n.save_profile()}</button>
                    </form>
                </div>

                <div class="section-card">
                    <h2 class="section-title">{i18n.change_password()}</h2>
                    <form method="post" action="/settings/password">
                        <div class="form-group">
                            <label for="current_password">{i18n.current_password()}</label>
                            <input type="password" id="current_password" name="current_password" required=true class="form-control" />
                        </div>
                        <div class="form-group">
                            <label for="new_password">{i18n.new_password()}</label>
                            <input type="password" id="new_password" name="new_password" required=true class="form-control" />
                        </div>
                        <div class="form-group">
                            <label for="confirm_password">{i18n.confirm_password()}</label>
                            <input type="password" id="confirm_password" name="confirm_password" required=true class="form-control" />
                        </div>
                        <button type="submit" class="btn btn-primary">{i18n.update_password_btn()}</button>
                    </form>
                </div>

                <div class="section-card">
                    <div class="section-header">
                        <div>
                            <h2 class="section-title">{i18n.passkeys_title()}</h2>
                            <p class="text-muted">{i18n.passkeys_desc()}</p>
                        </div>
                        <PasskeyEnrollIsland register_button_label=i18n.register_passkey_btn().to_string() />
                    </div>
                    {passkey_list}
                </div>

                <div class="section-card" id="pat">
                    <h2 class="section-title">"Personal Access Tokens"</h2>
                    <p class="text-muted">"Used to authenticate the apich CLI and any external git client against your projects."</p>
                    {new_pat_banner}
                    <form method="post" action="/settings/pat/create" class="inline-form" style="margin-bottom:1rem;">
                        <input type="text" name="name" placeholder="Token name (e.g. \"laptop\")" required=true class="form-control" style="max-width:260px;" />
                        <button type="submit" class="btn btn-primary btn-sm">"Generate Token"</button>
                    </form>
                    {pat_list}
                </div>

                <div class="section-card" id="ssh">
                    <h2 class="section-title">"SSH Keys"</h2>
                    <p class="text-muted">"Stored for identity/compatibility. SSH-based clone/push transport is not available yet — use a Personal Access Token over HTTPS instead."</p>
                    <form method="post" action="/settings/ssh/add" style="margin-bottom:1rem;">
                        <div class="form-group">
                            <label>"Key name"</label>
                            <input type="text" name="name" placeholder="e.g. \"laptop\"" required=true class="form-control" style="max-width:260px;" />
                        </div>
                        <div class="form-group">
                            <label>"Public key"</label>
                            <textarea name="public_key" placeholder="ssh-ed25519 AAAA..." required=true class="form-control" rows="2" style="font-family:var(--font-mono); font-size:0.8rem;"></textarea>
                        </div>
                        <button type="submit" class="btn btn-primary btn-sm">"Add Key"</button>
                    </form>
                    {ssh_list}
                </div>

                <div class="section-card" id="gpg">
                    <h2 class="section-title">"GPG Keys"</h2>
                    <p class="text-muted">"Used to verify signed VCS snapshots. Signing itself always happens locally with your own keyring — only your public key is stored here."</p>
                    <form method="post" action="/settings/gpg/add" style="margin-bottom:1rem;">
                        <div class="form-group">
                            <label>"Key name"</label>
                            <input type="text" name="name" placeholder="e.g. \"work key\"" required=true class="form-control" style="max-width:260px;" />
                        </div>
                        <div class="form-group">
                            <label>"Public key (ASCII-armored)"</label>
                            <textarea name="public_key" placeholder="-----BEGIN PGP PUBLIC KEY BLOCK-----..." required=true class="form-control" rows="4" style="font-family:var(--font-mono); font-size:0.75rem;"></textarea>
                        </div>
                        <button type="submit" class="btn btn-primary btn-sm">"Add Key"</button>
                    </form>
                    {gpg_list}
                </div>
            </div>
        </AppShell>
    }
}
