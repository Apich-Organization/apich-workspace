use crate::app::components::PageShell;
use crate::ui::i18n::I18n;
use apich_islands::AuthTabsIsland;
use leptos::prelude::*;

#[component]
pub fn LoginPage(
    error: Option<String>,
    success: Option<String>,
    notice: Option<String>,
    return_to: String,
    i18n: I18n,
    current_path: String,
) -> impl IntoView {
    let notice_text = notice.map(|n| {
        if n == "session_expired" {
            i18n.session_expired_notice().to_string()
        } else {
            n
        }
    });

    view! {
        <PageShell title=i18n.submit_login().to_string() i18n=i18n>
            <div class="auth-page">
                <div class="auth-card">
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
                        <h1 class="auth-title">{i18n.sign_in_title()}</h1>
                        <p class="auth-subtitle">{i18n.sign_in_subtitle()}</p>
                    </div>

                    {notice_text.map(|n| view! {
                        <div class="alert alert-info" style="display:flex; align-items:center; gap:0.5rem; margin-bottom:1rem;">
                            <span>"⏳"</span><span>{n}</span>
                        </div>
                    })}
                    {error.map(|e| view! { <div class="alert alert-danger">{e}</div> })}
                    {success.map(|s| view! { <div class="alert alert-success">{s}</div> })}

                    <AuthTabsIsland
                        return_to=return_to
                        password_tab_label=i18n.tab_password().to_string()
                        passkey_tab_label=i18n.tab_passkey().to_string()
                        login_label=i18n.login_label().to_string()
                        password_label=i18n.password_label().to_string()
                        submit_label=i18n.submit_login().to_string()
                        passkey_login_button_label=i18n.passkey_login_btn().to_string()
                        passkey_hint_text=i18n.passkey_hint().to_string()
                    />

                    <div class="auth-footer">
                        <p><a href="/register">{i18n.no_account_prompt()}</a></p>
                    </div>
                </div>
            </div>
        </PageShell>
    }
}
