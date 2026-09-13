use crate::app::components::PageShell;
use crate::ui::i18n::I18n;
use apich_islands::PasskeyLoginIsland;
use leptos::prelude::*;

#[component]
pub fn TwoFactorLoginPage(
    challenge_id: String,
    username: String,
    email: String,
    return_to: String,
    has_passkeys: bool,
    notice: Option<String>,
    error: Option<String>,
    i18n: I18n,
    current_path: String,
) -> impl IntoView {
    let passkey_login_btn = i18n.passkey_2fa_btn().to_string();
    let passkey_hint = i18n.passkey_hint().to_string();

    view! {
        <PageShell title=i18n.two_factor_login_title().to_string() i18n=i18n>
            <div class="auth-page">
                <div class="auth-card" style="max-width:440px;">
                    <div style="display:flex; justify-content:flex-end; margin-bottom:1rem;">
                        <a
                            href=format!("/set-lang?lang={}&return_to={}", i18n.lang.toggle_code(), urlencoding::encode(&current_path))
                            class="lang-toggle"
                        >
                            "🌐 " {i18n.lang.toggle_label()}
                        </a>
                    </div>
                    <div class="auth-header">
                        <div class="auth-logo">"APICH"</div>
                        <h1 class="auth-title" style="font-size:1.35rem;">{i18n.two_factor_login_title()}</h1>
                        <p class="auth-subtitle">{i18n.two_factor_login_subtitle()}</p>
                    </div>

                    // User Identity context
                    <div style="background:var(--bg-muted); border:1px solid var(--border-subtle); border-radius:8px; padding:0.6rem 0.85rem; margin-bottom:1.25rem; display:flex; align-items:center; gap:0.6rem; font-size:0.85rem;">
                        <span style="font-size:1.1rem;">"👤"</span>
                        <div style="overflow:hidden; text-overflow:ellipsis; white-space:nowrap;">
                            <strong>{format!("@{}", username)}</strong>
                            <span class="text-muted" style="margin-left:0.4rem;">{format!("({})", email)}</span>
                        </div>
                    </div>

                    {notice.map(|n| view! {
                        <div class="alert alert-info" style="display:flex; align-items:center; gap:0.5rem; margin-bottom:1rem; font-size:0.85rem;">
                            <span>"✉️"</span><span>{n}</span>
                        </div>
                    })}
                    {error.map(|e| view! {
                        <div class="alert alert-danger" style="margin-bottom:1rem; font-size:0.85rem;">{e}</div>
                    })}

                    // 6-digit Code verification form
                    <form method="post" action="/login/2fa/verify" class="auth-form">
                        <input type="hidden" name="challenge_id" value=challenge_id.clone() />
                        <input type="hidden" name="return_to" value=return_to.clone() />
                        <div class="form-group" style="text-align:center; margin-bottom:1.25rem;">
                            <label style="font-size:0.85rem; font-weight:600; color:var(--text-sub); display:block; margin-bottom:0.5rem;">
                                {i18n.totp_verify_code_label()}
                            </label>
                            <input
                                type="text"
                                id="2fa-code"
                                name="code"
                                required=true
                                autofocus=true
                                maxlength="8"
                                placeholder="00000000"
                                class="form-control"
                                style="font-family:var(--font-mono); font-size:1.5rem; font-weight:700; letter-spacing:0.35em; text-align:center; width:100%; max-width:240px; margin:0 auto; padding:0.6rem 0.75rem;"
                            />
                            <small class="text-muted" style="display:block; margin-top:0.4rem; font-size:0.78rem;">
                                "Enter 8-digit code from authenticator app or email."
                            </small>
                        </div>
                        <button type="submit" class="btn btn-primary btn-block btn-lg" style="height:44px;">
                            {i18n.verify_and_login_btn()}
                        </button>
                    </form>

                    // Alternative options divider
                    <div style="display:flex; align-items:center; margin:1.5rem 0; gap:0.75rem;">
                        <div style="flex:1; height:1px; background:var(--border-subtle);"></div>
                        <span style="font-size:0.75rem; font-weight:600; color:var(--text-muted); text-transform:uppercase;">
                            {i18n.divider_or()}
                        </span>
                        <div style="flex:1; height:1px; background:var(--border-subtle);"></div>
                    </div>

                    <div style="display:flex; flex-direction:column; gap:0.75rem;">
                        // Request email code
                        <form method="post" action="/login/2fa/send-email" style="margin:0;">
                            <input type="hidden" name="challenge_id" value=challenge_id.clone() />
                            <input type="hidden" name="return_to" value=return_to.clone() />
                            <button
                                type="submit"
                                class="btn btn-secondary btn-block"
                                style="display:flex; align-items:center; justify-content:center; gap:0.4rem; font-size:0.85rem;"
                            >
                                {i18n.send_code_email_btn()}
                            </button>
                        </form>

                        // Passkey option if registered
                        {if has_passkeys {
                            view! {
                                <div style="margin-top:0.25rem;">
                                    <PasskeyLoginIsland
                                        return_to=return_to.clone()
                                        login_button_label=passkey_login_btn
                                        hint_text=passkey_hint
                                    />
                                </div>
                            }.into_any()
                        } else {
                            view! { <div></div> }.into_any()
                        }}
                    </div>

                    <div style="text-align:center; margin-top:1.5rem; padding-top:1rem; border-top:1px solid var(--border-subtle);">
                        <a href="/login" style="font-size:0.825rem; color:var(--text-muted); text-decoration:none;">
                            "← " {i18n.back()}
                        </a>
                    </div>
                </div>
            </div>
        </PageShell>
    }
}
