use crate::app::components::PageShell;
use crate::ui::i18n::I18n;
use apich_islands::PasskeyLoginIsland;
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

    let password_tab_label = i18n.tab_password().to_string();
    let passkey_tab_label = i18n.tab_passkey().to_string();
    let login_label = i18n.login_label().to_string();
    let password_label = i18n.password_label().to_string();
    let submit_label = i18n.submit_login().to_string();
    let passkey_login_btn = i18n.passkey_login_btn().to_string();
    let passkey_hint = i18n.passkey_hint().to_string();

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

                    <div class="auth-tabs">
                        <button
                            type="button"
                            id="tab-btn-password"
                            class="auth-tab active"
                            onclick="switchAuthTab('password')"
                        >
                            "🔑 " {password_tab_label}
                        </button>
                        <button
                            type="button"
                            id="tab-btn-passkey"
                            class="auth-tab"
                            onclick="switchAuthTab('passkey')"
                        >
                            "⚡ " {passkey_tab_label}
                        </button>
                    </div>

                    <div id="pane-password" class="auth-pane" style="display:block;">
                        <form method="post" action="/login" class="auth-form">
                            <input type="hidden" name="return_to" value=return_to.clone() />
                            <div class="form-group">
                                <label for="login">{login_label}</label>
                                <input type="text" id="login" name="login" required=true placeholder="researcher@lab.org" class="form-control" autofocus=true />
                            </div>
                            <div class="form-group">
                                <label for="password">{password_label}</label>
                                <input type="password" id="password" name="password" required=true placeholder="••••••••••••" class="form-control" />
                            </div>
                            <button type="submit" class="btn btn-primary btn-block btn-lg" style="margin-top:0.75rem;">{submit_label}</button>
                        </form>
                    </div>

                    <div id="pane-passkey" class="auth-pane" style="display:none;">
                        <PasskeyLoginIsland
                            return_to=return_to
                            login_button_label=passkey_login_btn
                            hint_text=passkey_hint
                        />
                    </div>

                    <script>
                    {r#"
                    function switchAuthTab(mode) {
                        const panePw = document.getElementById('pane-password');
                        const panePk = document.getElementById('pane-passkey');
                        const btnPw = document.getElementById('tab-btn-password');
                        const btnPk = document.getElementById('tab-btn-passkey');
                        if (panePw && panePk && btnPw && btnPk) {
                            panePw.style.display = mode === 'password' ? 'block' : 'none';
                            panePk.style.display = mode === 'passkey' ? 'block' : 'none';
                            btnPw.classList.toggle('active', mode === 'password');
                            btnPk.classList.toggle('active', mode === 'passkey');
                        }
                    }
                    "#}
                    </script>

                    <div class="auth-footer">
                        <p><a href="/register">{i18n.no_account_prompt()}</a></p>
                    </div>
                </div>
            </div>
        </PageShell>
    }
}
