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
    totp_setup: Option<crate::auth::TotpSetupData>,
    github_cred: Option<apich_db::UserGitCredential>,
    notice: Option<String>,
    error: Option<String>,
    i18n: I18n,
    current_path: String,
) -> impl IntoView {
    let alert = notice.map_or_else(
        || error.map(|e| view! { <div class="alert alert-danger">{e}</div> }.into_any()),
        |n| Some(view! { <div class="alert alert-success">{n}</div> }.into_any()),
    );

    let role_display = if user.is_platform_admin {
        i18n.platform_admin().to_string()
    } else if is_org_or_team_admin {
        i18n.org_admin_title().to_string()
    } else {
        i18n.researcher().to_string()
    };

    let is_zh = i18n.is_zh();
    let no_passkeys_text = if is_zh {
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
                    .last_used_at.map_or_else(|| if is_zh { "从未使用".to_string() } else { "Never".to_string() }, |t| t.format("%Y-%m-%d %H:%M").to_string());
                let created = cred.created_at.format("%Y-%m-%d").to_string();
                let status_label = if is_zh { "有效" } else { "Active" };
                view! {
                    <div class="passkey-item">
                        <div class="passkey-icon">"🔑"</div>
                        <div class="passkey-info">
                            <span class="passkey-name">{cred.device_name}</span>
                            <span class="passkey-meta">{format!("{} • {}", if is_zh { format!("已登记 {created}") } else { format!("Enrolled {created}") }, if is_zh { format!("上次使用 {last_used}") } else { format!("Last used {last_used}") })}</span>
                        </div>
                        <span class="badge badge-active">{status_label}</span>
                        <form method="post" action=format!("/settings/passkey/{}/delete", cred.id) class="inline-form">
                            <button type="submit" class="btn btn-danger btn-sm">{i18n.remove()}</button>
                        </form>
                    </div>
                }
            })
            .collect::<Vec<_>>();
        view! { <div class="passkey-list">{items}</div> }.into_any()
    };

    let (initial_pat_display, initial_pat_token) = match new_pat_token {
        | Some(token) => ("display:block;", token),
        | None => ("display:none;", String::new()),
    };

    let new_pat_banner = view! {
        <div id="new-pat-container" class="pat-token-banner" style=initial_pat_display>
            <div style="display:flex; justify-content:space-between; align-items:flex-start; margin-bottom:0.75rem;">
                <div>
                    <div style="font-weight:600; font-size:0.95rem; color:#15803d; display:flex; align-items:center; gap:0.4rem;">
                        <span>"🔑"</span>
                        <span>{if i18n.is_zh() { "个人访问令牌已生成" } else { "New Personal Access Token Generated" }}</span>
                    </div>
                    <p style="margin:0.25rem 0 0 0; font-size:0.83rem; color:var(--text-muted);">
                        {if i18n.is_zh() {
                            "请务必立即复制并妥善保存。在您离开本页面之前，该令牌会一直显示；一旦离开页面，将无法再次查看。"
                        } else {
                            "Make sure to copy your personal access token now. It will remain on this page until you leave, but cannot be shown again."
                        }}
                    </p>
                </div>
                <button
                    type="button"
                    id="dismiss-pat-token-btn"
                    title=if i18n.is_zh() { "关闭" } else { "Dismiss" }
                    style="background:none; border:none; color:var(--text-muted); font-size:1.25rem; cursor:pointer; line-height:1; padding:2px 6px; border-radius:4px; opacity:0.7;"
                >
                    "×"
                </button>
            </div>
            <div style="display:flex; gap:0.6rem; align-items:center;">
                <input
                    type="text"
                    id="new-pat-token-input"
                    readonly=true
                    value=initial_pat_token
                    class="form-control"
                    style="font-family:var(--font-mono); font-size:0.88rem; font-weight:600; background:var(--bg-surface); color:var(--text-main); letter-spacing:0.3px; padding:0.5rem 0.75rem; border:1px solid var(--border-subtle); flex:1; border-radius:6px;"
                    onfocus="this.select()"
                    onclick="this.select()"
                />
                <button
                    type="button"
                    id="copy-pat-token-btn"
                    class="btn btn-primary btn-sm"
                    style="display:inline-flex; align-items:center; gap:0.35rem; font-weight:600; padding:0.5rem 0.95rem; white-space:nowrap;"
                >
                    <span id="copy-pat-icon">"📋"</span>
                    <span id="copy-pat-text">{if i18n.is_zh() { "复制令牌" } else { "Copy Token" }}</span>
                </button>
            </div>
            <div style="margin-top:0.6rem; font-size:0.8rem; color:var(--text-muted);">
                {if i18n.is_zh() {
                    "💡 提示：使用 Git 通过 HTTPS 克隆或推送项目时，请使用此令牌作为密码。"
                } else {
                    "💡 Tip: Use this token as your password when cloning or pushing with Git over HTTPS."
                }}
            </div>
        </div>
        <script>
        {r"
        (function() {
            const box = document.getElementById('new-pat-container');
            const input = document.getElementById('new-pat-token-input');
            const copyBtn = document.getElementById('copy-pat-token-btn');
            const copyText = document.getElementById('copy-pat-text');
            const copyIcon = document.getElementById('copy-pat-icon');
            const dismissBtn = document.getElementById('dismiss-pat-token-btn');
            const storageKey = 'apich_active_pat_token';

            if (!box || !input) return;

            if (input.value && input.value.trim().length > 0) {
                sessionStorage.setItem(storageKey, input.value.trim());
                if (window.history && window.history.replaceState) {
                    window.history.replaceState({}, document.title, window.location.pathname + (window.location.hash || '#pat'));
                }
            } else {
                const stored = sessionStorage.getItem(storageKey);
                if (stored && stored.trim().length > 0) {
                    input.value = stored.trim();
                    box.style.display = 'block';
                }
            }

            if (copyBtn) {
                copyBtn.onclick = function() {
                    if (!input.value) return;
                    const textToCopy = input.value;
                    const onCopied = function() {
                        if (copyText) copyText.textContent = 'Copied!';
                        if (copyIcon) copyIcon.textContent = '✓';
                        copyBtn.style.background = 'var(--accent-teal, #10b981)';
                        copyBtn.style.borderColor = 'var(--accent-teal, #10b981)';
                        setTimeout(function() {
                            if (copyText) copyText.textContent = 'Copy Token';
                            if (copyIcon) copyIcon.textContent = '📋';
                            copyBtn.style.background = '';
                            copyBtn.style.borderColor = '';
                        }, 2500);
                    };

                    if (navigator.clipboard && navigator.clipboard.writeText) {
                        navigator.clipboard.writeText(textToCopy).then(onCopied).catch(function() {
                            input.select();
                            document.execCommand('copy');
                            onCopied();
                        });
                    } else {
                        input.select();
                        document.execCommand('copy');
                        onCopied();
                    }
                };
            }

            if (dismissBtn) {
                dismissBtn.onclick = function() {
                    sessionStorage.removeItem(storageKey);
                    box.style.display = 'none';
                };
            }

            document.addEventListener('click', function(e) {
                const anchor = e.target && e.target.closest ? e.target.closest('a') : null;
                if (anchor && anchor.href) {
                    try {
                        const targetUrl = new URL(anchor.href, window.location.origin);
                        if (targetUrl.origin === window.location.origin && targetUrl.pathname !== '/settings') {
                            sessionStorage.removeItem(storageKey);
                        }
                    } catch (_) {}
                }
            });
        })();
        "}
        </script>
    };

    let no_pats_text = if is_zh { "暂无个人访问令牌。" } else { "No personal access tokens yet." };
    let pat_list = if pats.is_empty() {
        view! { <p class="text-muted" style="font-size:0.85rem;">{no_pats_text}</p> }.into_any()
    } else {
        let items = pats.into_iter().map(|t| {
            let created = t.created_at.format("%Y-%m-%d").to_string();
            let last_used = t.last_used_at.map_or_else(|| if is_zh { "从未使用".to_string() } else { "Never".to_string() }, |d| d.format("%Y-%m-%d %H:%M").to_string());
            let is_active = t.is_active();
            let expires_info = t.expires_at.map_or_else(
                || if is_zh { "永不过期".to_string() } else { "No expiration".to_string() },
                |exp| {
                    if exp < chrono::Utc::now() {
                        format!("{} {}", if is_zh { "已过期" } else { "Expired" }, exp.format("%Y-%m-%d"))
                    } else {
                        format!("{} {}", if is_zh { "过期时间" } else { "Expires" }, exp.format("%Y-%m-%d"))
                    }
                },
            );
            let status_label = if is_active {
                if is_zh { "有效" } else { "Active" }
            } else {
                if is_zh { "已撤销/已过期" } else { "Revoked/Expired" }
            };
            let status_class = if is_active { "badge badge-active" } else { "badge badge-idle" };
            view! {
                <div class="passkey-item">
                    <div class="passkey-icon">"🔑"</div>
                    <div class="passkey-info">
                        <span class="passkey-name">{t.name} " (" {t.token_prefix} "…)"</span>
                        <span class="passkey-meta">{format!("{} • {} • {}", if is_zh { format!("创建于 {created}") } else { format!("Created {created}") }, expires_info, if is_zh { format!("上次使用 {last_used}") } else { format!("Last used {last_used}") })}</span>
                    </div>
                    <span class=status_class>{status_label}</span>
                    {is_active.then(|| view! {
                        <form method="post" action=format!("/settings/pat/{}/revoke", t.id) class="inline-form">
                            <button type="submit" class="btn btn-danger btn-sm">{if is_zh { "撤销" } else { "Revoke" }}</button>
                        </form>
                    })}
                </div>
            }
        }).collect::<Vec<_>>();
        view! { <div class="passkey-list">{items}</div> }.into_any()
    };

    let no_ssh_text = if is_zh { "尚未添加任何 SSH 密钥。" } else { "No SSH keys added yet." };
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
                        <span class="passkey-meta">{format!("{} • {}", k.fingerprint, if is_zh { format!("添加于 {created}") } else { format!("Added {created}") })}</span>
                    </div>
                    <form method="post" action=format!("/settings/ssh/{}/delete", k.id) class="inline-form">
                        <button type="submit" class="btn btn-danger btn-sm">{i18n.delete()}</button>
                    </form>
                </div>
            }
        }).collect::<Vec<_>>();
        view! { <div class="passkey-list">{items}</div> }.into_any()
    };

    let no_gpg_text = if is_zh { "尚未添加任何 GPG 密钥。" } else { "No GPG keys added yet." };
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
                        <span class="passkey-meta">{format!("{} • {}", k.fingerprint, if is_zh { format!("添加于 {created}") } else { format!("Added {created}") })}</span>
                    </div>
                    <form method="post" action=format!("/settings/gpg/{}/delete", k.id) class="inline-form">
                        <button type="submit" class="btn btn-danger btn-sm">{i18n.delete()}</button>
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
            <div class="settings-container" style="max-width:880px; width:100%;">
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
                                <input type="text" id="display_name" name="display_name" value=user.display_name.clone() required=true class="form-control" style="max-width:440px;" />
                            </div>
                            <div class="form-group">
                                <label for="email">{i18n.email()}</label>
                                <input type="email" id="email" name="email" value=user.email.clone() required=true class="form-control" style="max-width:440px;" />
                            </div>
                            <div class="form-group">
                                <label for="avatar_url">{i18n.avatar_url()}</label>
                                <input type="url" id="avatar_url" name="avatar_url" value=user.avatar_url.clone().unwrap_or_default() placeholder="https://example.com/avatar.png" class="form-control" style="max-width:440px;" />
                            </div>
                            <div class="form-group">
                                <label>{if is_zh { "角色" } else { "Role" }}</label>
                                <input type="text" value=role_display readonly=true class="form-control readonly" style="max-width:440px;" />
                            </div>
                            <button type="submit" class="btn btn-primary">{i18n.save_profile()}</button>
                        </form>
                    </div>

                    <div class="section-card">
                        <h2 class="section-title">{i18n.change_password()}</h2>
                        <form method="post" action="/settings/password">
                            <div class="form-group">
                                <label for="current_password">{i18n.current_password()}</label>
                                <input type="password" id="current_password" name="current_password" required=true class="form-control" style="max-width:380px;" />
                            </div>
                            <div class="form-group">
                                <label for="new_password">{i18n.new_password()}</label>
                                <input type="password" id="new_password" name="new_password" required=true class="form-control" style="max-width:380px;" />
                            </div>
                            <div class="form-group">
                                <label for="confirm_password">{i18n.confirm_password()}</label>
                                <input type="password" id="confirm_password" name="confirm_password" required=true class="form-control" style="max-width:380px;" />
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

                <div class="section-card" id="totp">
                    <div class="section-header">
                        <div>
                            <h2 class="section-title">{i18n.totp_title()}</h2>
                            <p class="text-muted">{i18n.totp_desc()}</p>
                        </div>
                        <div>
                            {if user.totp_enabled {
                                view! { <span class="badge badge-active">{i18n.totp_status_active()}</span> }.into_any()
                            } else {
                                view! { <span class="badge badge-idle">{i18n.totp_status_disabled()}</span> }.into_any()
                            }}
                        </div>
                    </div>

                    {if user.totp_enabled {
                        view! {
                            <div style="background:var(--bg-muted); border:1px solid var(--border-subtle); border-radius:8px; padding:1.25rem; display:flex; justify-content:space-between; align-items:center; flex-wrap:wrap; gap:1rem;">
                                <div>
                                    <div style="font-weight:600; font-size:0.95rem; color:var(--text-main); margin-bottom:0.25rem;">
                                        "✅ " {i18n.totp_status_active()}
                                    </div>
                                    <p class="text-muted" style="margin:0; font-size:0.85rem;">
                                        {if is_zh { "您的账号已启用身份验证器双因素安全认证保护。" } else { "Your account is protected with two-factor authentication via an authenticator application." }}
                                    </p>
                                </div>
                                <form method="post" action="/settings/totp/disable" onsubmit=format!("return confirm('{}');", if is_zh { "确定要为您的账号禁用双因素身份验证 (2FA) 吗？" } else { "Are you sure you want to disable Two-Factor Authentication for your account?" })>
                                    <button type="submit" class="btn btn-danger btn-sm">{i18n.totp_disable_btn()}</button>
                                </form>
                            </div>
                        }.into_any()
                    } else if let Some(setup) = totp_setup {
                        let secret = setup.secret.clone();
                        let formatted_secret = setup.formatted_secret.clone();
                        let qr_url = setup.qr_data_url.clone();
                        view! {
                            <div style="background:var(--bg-muted); border:1px solid var(--border-subtle); border-radius:10px; padding:1.25rem;">
                                <div style="display:flex; flex-wrap:wrap; gap:1.75rem; align-items:center;">
                                    // Left: Scannable QR Code
                                    <div style="text-align:center;">
                                        <img
                                            src=qr_url
                                            alt="TOTP QR Code"
                                            style="width:160px; height:160px; border-radius:8px; border:1px solid var(--border-subtle); background:#ffffff; padding:6px; display:block;"
                                        />
                                        <span class="text-muted" style="font-size:0.75rem; margin-top:0.4rem; display:block;">
                                            {i18n.totp_scan_qr_prompt()}
                                        </span>
                                    </div>

                                    // Right: Manual entry hash & activation form
                                    <div style="flex:1; min-width:280px;">
                                        <div style="margin-bottom:1.15rem;">
                                            <label style="font-size:0.8rem; font-weight:600; color:var(--text-sub); display:block; margin-bottom:0.35rem;">
                                                {i18n.totp_manual_key_prompt()}
                                            </label>
                                            <div style="display:flex; align-items:center; gap:0.5rem; max-width:390px;">
                                                <input
                                                    type="text"
                                                    id="totp-secret-input"
                                                    value=formatted_secret
                                                    readonly=true
                                                    class="form-control readonly"
                                                    style="font-family:var(--font-mono); font-size:0.85rem; font-weight:600; letter-spacing:0.05em; background:var(--bg-surface); text-align:center;"
                                                />
                                                <button
                                                    type="button"
                                                    class="btn btn-secondary btn-sm"
                                                    onclick="const i=document.getElementById('totp-secret-input');if(i){navigator.clipboard.writeText(i.value.replace(/\\s/g,''));this.innerText='✓ Copied';setTimeout(()=>this.innerText='Copy Key',2000);}"
                                                    style="white-space:nowrap;"
                                                >
                                                    {i18n.totp_copy_key()}
                                                </button>
                                            </div>
                                        </div>

                                        // Verification Form
                                        <form method="post" action="/settings/totp/activate" style="margin:0;">
                                            <input type="hidden" name="secret" value=secret />
                                            <div style="display:flex; flex-wrap:wrap; gap:0.6rem; align-items:flex-end;">
                                                <div class="form-group" style="margin-bottom:0;">
                                                    <label style="font-size:0.8rem; font-weight:600; color:var(--text-sub); display:block; margin-bottom:0.35rem;">
                                                        {i18n.totp_verify_code_label()}
                                                    </label>
                                                    <input
                                                        type="text"
                                                        name="code"
                                                        required=true
                                                        placeholder="00000000"
                                                        maxlength="8"
                                                        class="form-control"
                                                        style="width:160px; font-family:var(--font-mono); font-size:1.1rem; font-weight:700; text-align:center; letter-spacing:0.2em;"
                                                    />
                                                </div>
                                                <button type="submit" class="btn btn-primary" style="height:38px; padding:0 1.1rem;">
                                                    {i18n.totp_activate_btn()}
                                                </button>
                                            </div>
                                        </form>
                                    </div>
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        view! { <div></div> }.into_any()
                    }}
                </div>

                <div class="section-card" id="pat">
                    <h2 class="section-title">{i18n.pat_heading()}</h2>
                    <p class="text-muted">{i18n.pat_subtitle()}</p>
                    {new_pat_banner}
                    <form method="post" action="/settings/pat/create" class="pat-create-form" style="display:flex; flex-wrap:wrap; gap:0.75rem; align-items:flex-end; margin-bottom:1.25rem;">
                        <div class="form-group" style="margin-bottom:0;">
                            <label style="font-size:0.8rem; font-weight:600; margin-bottom:0.25rem; display:block;">{i18n.token_name_label()}</label>
                            <input type="text" name="name" placeholder="e.g. \"cli-laptop\"" required=true class="form-control" style="width:220px;" />
                        </div>
                        <div class="form-group" style="margin-bottom:0;">
                            <label style="font-size:0.8rem; font-weight:600; margin-bottom:0.25rem; display:block;">{i18n.expiration_label()}</label>
                            <select
                                name="expiration"
                                id="pat-expiration-select"
                                class="form-control"
                                style="width:170px;"
                                onchange="const c = document.getElementById('pat-custom-date'); if (c) c.style.display = this.value === 'custom' ? 'block' : 'none';"
                            >
                                <option value="30" selected=true>{if is_zh { "30 天" } else { "30 days" }}</option>
                                <option value="60">{if is_zh { "60 天" } else { "60 days" }}</option>
                                <option value="90">{if is_zh { "90 天" } else { "90 days" }}</option>
                                <option value="7">{if is_zh { "7 天" } else { "7 days" }}</option>
                                <option value="365">{if is_zh { "1 年" } else { "1 year" }}</option>
                                <option value="never">{if is_zh { "永不过期" } else { "No expiration" }}</option>
                                <option value="custom">{if is_zh { "自定义日期..." } else { "Custom date..." }}</option>
                            </select>
                        </div>
                        <div class="form-group" id="pat-custom-date" style="display:none; margin-bottom:0;">
                            <label style="font-size:0.8rem; font-weight:600; margin-bottom:0.25rem; display:block;">{if is_zh { "过期于" } else { "Expire on" }}</label>
                            <input type="date" name="custom_date" class="form-control" style="width:160px;" />
                        </div>
                        <button type="submit" class="btn btn-primary btn-sm" style="height:36px; padding:0 1rem;">{i18n.generate_token_btn()}</button>
                    </form>
                    {pat_list}
                </div>

                <div class="section-card" id="github">
                    <h2 class="section-title">{i18n.github_integration_title()}</h2>
                    <p class="text-muted" style="font-size:0.875rem; margin-bottom:1.25rem;">
                        {i18n.github_integration_subtitle()}
                    </p>
                    {if let Some(cred) = github_cred {
                        let ends_with = cred.access_token.chars().rev().take(4).collect::<String>().chars().rev().collect::<String>();
                        view! {
                            <div style="background:var(--bg-muted); border:1px solid var(--border-subtle); border-radius:10px; padding:1.25rem; margin-bottom:1.25rem;">
                                <div style="display:flex; justify-content:space-between; align-items:center; flex-wrap:wrap; gap:0.75rem;">
                                    <div>
                                        <div style="font-weight:600; font-size:0.95rem; margin-bottom:0.25rem;">
                                            <span style="color:var(--success); margin-right:0.4rem;">"●"</span>
                                            {if is_zh { "已关联 @" } else { "Connected as @" }} {cred.account_username.clone()}
                                        </div>
                                        <div class="text-muted" style="font-size:0.8rem;">
                                            {if is_zh { "令牌有效（尾号 ..." } else { "Token active (ends with ..." }} {ends_with} ")"
                                        </div>
                                    </div>
                                    <form method="post" action="/settings/github/disconnect" onsubmit=format!("return confirm('{}');", if is_zh { "确定要解绑您的 GitHub 账号吗？" } else { "Are you sure you want to disconnect your GitHub account?" })>
                                        <button type="submit" class="btn btn-secondary btn-sm" style="color:var(--danger); border-color:var(--danger);">{if is_zh { "解除关联" } else { "Disconnect" }}</button>
                                    </form>
                                </div>
                            </div>
                            <h3 class="card-subtitle" style="font-size:0.9rem; margin-bottom:0.5rem;">{if is_zh { "更新 GitHub 访问令牌" } else { "Update GitHub Token" }}</h3>
                            <form method="post" action="/settings/github/save" class="form-row" style="align-items:flex-end; max-width:600px;">
                                <input type="hidden" name="username" value=cred.account_username.clone() />
                                <div class="form-group" style="margin-bottom:0; flex-grow:1;">
                                    <label>{if is_zh { "新个人访问令牌 (PAT)" } else { "New Personal Access Token (PAT)" }}</label>
                                    <input type="password" name="token" required=true placeholder="ghp_..." class="form-control" />
                                </div>
                                <button type="submit" class="btn btn-primary btn-sm" style="height:36px;">{if is_zh { "更新令牌" } else { "Update Token" }}</button>
                            </form>
                        }.into_any()
                    } else {
                        view! {
                            <form method="post" action="/settings/github/save" style="max-width:540px;">
                                <div class="form-group">
                                    <label>{if is_zh { "GitHub 用户名" } else { "GitHub Username" }}</label>
                                    <input type="text" name="username" placeholder="e.g. octocat" required=true class="form-control" />
                                </div>
                                <div class="form-group">
                                    <label>{if is_zh { "个人访问令牌 (PAT)" } else { "Personal Access Token (PAT)" }}</label>
                                    <input type="password" name="token" placeholder="ghp_..." required=true class="form-control" />
                                    <p class="text-muted" style="font-size:0.75rem; margin-top:0.35rem;">
                                        {if is_zh { "需要令牌？" } else { "Need a token? " }} <a href="https://github.com/settings/tokens/new?scopes=repo" target="_blank" rel="noopener noreferrer">{if is_zh { "前往 GitHub 生成带有 'repo' 权限的令牌 →" } else { "Generate a token with 'repo' scope on GitHub →" }}</a>
                                    </p>
                                </div>
                                <button type="submit" class="btn btn-primary">{if is_zh { "关联 GitHub 账号" } else { "Connect GitHub Account" }}</button>
                            </form>
                        }.into_any()
                    }}
                </div>

                <div class="section-card" id="ssh">
                    <h2 class="section-title">{i18n.ssh_keys_title()}</h2>
                    <p class="text-muted">{i18n.ssh_keys_subtitle()}</p>
                    <form method="post" action="/settings/ssh/add" style="margin-bottom:1rem;">
                        <div class="form-group">
                            <label>{i18n.key_name_label()}</label>
                            <input type="text" name="name" placeholder="e.g. \"laptop\"" required=true class="form-control" style="max-width:260px;" />
                        </div>
                        <div class="form-group">
                            <label>{i18n.public_key_label()}</label>
                            <textarea name="public_key" placeholder="ssh-ed25519 AAAA..." required=true class="form-control" rows="2" style="font-family:var(--font-mono); font-size:0.8rem; max-width:640px;"></textarea>
                        </div>
                        <button type="submit" class="btn btn-primary btn-sm">{i18n.add_key_btn()}</button>
                    </form>
                    {ssh_list}
                </div>

                <div class="section-card" id="gpg">
                    <h2 class="section-title">{i18n.gpg_keys_title()}</h2>
                    <p class="text-muted">{i18n.gpg_keys_subtitle()}</p>
                    <form method="post" action="/settings/gpg/add" style="margin-bottom:1rem;">
                        <div class="form-group">
                            <label>{i18n.key_name_label()}</label>
                            <input type="text" name="name" placeholder="e.g. \"work key\"" required=true class="form-control" style="max-width:260px;" />
                        </div>
                        <div class="form-group">
                            <label>{if is_zh { "公钥内容 (ASCII-armored)" } else { "Public key (ASCII-armored)" }}</label>
                            <textarea name="public_key" placeholder="-----BEGIN PGP PUBLIC KEY BLOCK-----..." required=true class="form-control" rows="4" style="font-family:var(--font-mono); font-size:0.75rem; max-width:640px;"></textarea>
                        </div>
                        <button type="submit" class="btn btn-primary btn-sm">{i18n.add_key_btn()}</button>
                    </form>
                    {gpg_list}
                </div>
            </div>
            </div>
        </AppShell>
    }
}
