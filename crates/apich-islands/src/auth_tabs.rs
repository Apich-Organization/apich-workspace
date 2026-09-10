//! Real Rust replacement for the login page's password/passkey tab switcher
//! (`login.rs`'s `AUTH_TAB_SCRIPT`). Wraps the real `PasskeyLoginIsland` as a nested island --
//! islands can nest freely, each hydrates independently.

use crate::webauthn::PasskeyLoginIsland;
use leptos::prelude::*;

#[island]
#[allow(clippy::too_many_arguments)]
pub fn AuthTabsIsland(
    #[prop(into)] return_to: String,
    #[prop(into)] password_tab_label: String,
    #[prop(into)] passkey_tab_label: String,
    #[prop(into)] login_label: String,
    #[prop(into)] password_label: String,
    #[prop(into)] submit_label: String,
    #[prop(into)] passkey_login_button_label: String,
    #[prop(into)] passkey_hint_text: String,
) -> impl IntoView {
    let mode = RwSignal::new("password".to_string());

    view! {
        <div class="auth-tabs">
            <button
                type="button"
                class="auth-tab"
                class:active=move || mode.get() == "password"
                on:click=move |_| mode.set("password".to_string())
            >
                "🔑 " {password_tab_label}
            </button>
            <button
                type="button"
                class="auth-tab"
                class:active=move || mode.get() == "passkey"
                on:click=move |_| mode.set("passkey".to_string())
            >
                "⚡ " {passkey_tab_label}
            </button>
        </div>

        <div class="auth-pane" style:display=move || if mode.get() == "password" { "block" } else { "none" }>
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

        <div class="auth-pane" style:display=move || if mode.get() == "passkey" { "block" } else { "none" }>
            <PasskeyLoginIsland
                return_to=return_to
                login_button_label=passkey_login_button_label
                hint_text=passkey_hint_text
            />
        </div>
    }
}
