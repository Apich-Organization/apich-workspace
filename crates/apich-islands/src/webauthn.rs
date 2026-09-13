//! WebAuthn (FIDO2 / Passkey) client ceremonies powered by `webauthn-rs-proto`.
//!
//! Provides interactive client-side enrollment and discoverable passwordless login ceremonies.
//! Uses standard W3C WebAuthn Credential Management API mappings provided by `webauthn-rs-proto`
//! with the `wasm` feature, eliminating browser incompatibility and Level 3 API gaps.

use leptos::prelude::*;

/// Best-effort extraction of the real DOMException / Error name and message.
#[cfg(feature = "hydrate")]
fn describe_js_error(e: &wasm_bindgen::JsValue) -> String {
    use wasm_bindgen::JsCast;
    if let Some(err) = e.dyn_ref::<web_sys::DomException>() {
        format!("{}: {}", err.name(), err.message())
    } else if let Some(err) = e.dyn_ref::<js_sys::Error>() {
        format!("{}: {}", err.name(), err.message())
    } else {
        format!("{e:?}")
    }
}

/// The passkey login pane on the login page.
#[island]
pub fn PasskeyLoginIsland(
    #[prop(into)] return_to: String,
    #[prop(into)] login_button_label: String,
    #[prop(into)] hint_text: String,
) -> impl IntoView {
    let status = RwSignal::new(String::new());
    let busy = RwSignal::new(false);

    let on_click = move |_| {
        if busy.get_untracked() {
            return;
        }
        status.set(String::new());
        busy.set(true);
        run_passkey_login(status, busy, return_to.clone());
    };

    view! {
        <div class="passkey-box">
            <div class="passkey-icon-circle">
                <svg viewBox="0 0 24 24" width="32" height="32" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M12 2a5 5 0 0 0-5 5v3H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-8a2 2 0 0 0-2-2h-1V7a5 5 0 0 0-5-5zm-3 5a3 3 0 1 1 6 0v3H9V7z"></path>
                </svg>
            </div>
            <button type="button" class="btn btn-primary btn-block btn-lg" on:click=on_click disabled=move || busy.get()>
                {login_button_label}
            </button>
            <p class="hint-text">{hint_text}</p>
        </div>
        <p
            style:display=move || if status.get().is_empty() { "none" } else { "block" }
            style="font-size:0.8rem; color:#dc2626; text-align:center;"
        >
            {move || status.get()}
        </p>
    }
}

/// The "Register Passkey" button + status message on the settings page.
#[island]
pub fn PasskeyEnrollIsland(#[prop(into)] register_button_label: String) -> impl IntoView {
    let status = RwSignal::new(String::new());
    let is_error = RwSignal::new(true);
    let busy = RwSignal::new(false);

    let on_click = move |_| {
        if busy.get_untracked() {
            return;
        }
        status.set(String::new());
        busy.set(true);
        run_passkey_enroll(status, is_error, busy);
    };

    view! {
        <button type="button" class="btn btn-primary btn-sm" on:click=on_click disabled=move || busy.get()>
            {register_button_label}
        </button>
        <p
            style:display=move || if status.get().is_empty() { "none" } else { "block" }
            style:color=move || if is_error.get() { "#dc2626" } else { "var(--text-sub)" }
            style="font-size:0.8rem; margin-top:0.5rem;"
        >
            {move || status.get()}
        </p>
    }
}

#[cfg(feature = "hydrate")]
fn run_passkey_login(
    status: RwSignal<String>,
    busy: RwSignal<bool>,
    return_to: String,
) {
    wasm_bindgen_futures::spawn_local(async move {
        let result = passkey_login_ceremony(return_to).await;
        if let Err(e) = result {
            status.set(e);
        }
        busy.set(false);
    });
}

#[cfg(not(feature = "hydrate"))]
fn run_passkey_login(
    _status: RwSignal<String>,
    _busy: RwSignal<bool>,
    _return_to: String,
) {
}

#[cfg(feature = "hydrate")]
fn run_passkey_enroll(
    status: RwSignal<String>,
    is_error: RwSignal<bool>,
    busy: RwSignal<bool>,
) {
    wasm_bindgen_futures::spawn_local(async move {
        match passkey_enroll_ceremony().await {
            | Ok(()) => {
                if let Some(win) = web_sys::window() {
                    let _ = win.location().reload();
                }
            },
            | Err(e) => {
                is_error.set(true);
                status.set(e);
            },
        }
        busy.set(false);
    });
}

#[cfg(not(feature = "hydrate"))]
const fn run_passkey_enroll(
    _status: RwSignal<String>,
    _is_error: RwSignal<bool>,
    _busy: RwSignal<bool>,
) {
}

#[cfg(feature = "hydrate")]
async fn passkey_login_ceremony(return_to: String) -> Result<(), String> {
    use wasm_bindgen_futures::JsFuture;
    use webauthn_rs_proto::{PublicKeyCredential as ProtoPublicKeyCredential, RequestChallengeResponse};

    if web_sys::window()
        .and_then(|w| js_sys::Reflect::get(&w, &"PublicKeyCredential".into()).ok())
        .is_none()
    {
        return Err(
            "WebAuthn is not supported in this browser. Please use Password login.".to_string(),
        );
    }

    let mut rcr: RequestChallengeResponse = gloo_net::http::Request::post("/api/auth/passkey/auth/start")
        .send()
        .await
        .map_err(|e| format!("Could not retrieve login challenge: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Could not parse login challenge: {e}"))?;

    rcr.mediation = None;
    let c_options: web_sys::CredentialRequestOptions = rcr.into();

    let window = web_sys::window().ok_or_else(|| "No window".to_string())?;
    let promise = window
        .navigator()
        .credentials()
        .get_with_options(&c_options)
        .map_err(|e| format!("Could not start passkey sign-in: {}", describe_js_error(&e)))?;
    let jsval = JsFuture::from(promise)
        .await
        .map_err(|e| format!("Passkey sign-in was cancelled or failed: {}", describe_js_error(&e)))?;
    let w_pkc = web_sys::PublicKeyCredential::from(jsval);
    let pkc = ProtoPublicKeyCredential::from(w_pkc);
    let cred_json = serde_json::to_string(&pkc).map_err(|e| e.to_string())?;

    let finish_body = serde_json::json!({
        "credential_json": cred_json,
    });

    let resp = gloo_net::http::Request::post("/api/auth/passkey/auth/finish")
        .json(&finish_body)
        .map_err(|e| format!("{e}"))?
        .send()
        .await
        .map_err(|e| format!("{e}"))?;

    if resp.ok() {
        let target = if return_to.starts_with('/') && !return_to.starts_with("//") {
            return_to
        } else {
            "/".to_string()
        };
        if let Some(win) = web_sys::window() {
            let _ = win.location().set_href(&target);
        }
        Ok(())
    } else {
        let msg = resp
            .json::<serde_json::Value>()
            .await
            .ok()
            .and_then(|v| {
                v.get("error")
                    .and_then(|e| e.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "Passkey verification failed. Please try password.".to_string());
        Err(msg)
    }
}

#[cfg(feature = "hydrate")]
async fn passkey_enroll_ceremony() -> Result<(), String> {
    use wasm_bindgen_futures::JsFuture;
    use webauthn_rs_proto::{CreationChallengeResponse, RegisterPublicKeyCredential, ResidentKeyRequirement};

    if web_sys::window()
        .and_then(|w| js_sys::Reflect::get(&w, &"PublicKeyCredential".into()).ok())
        .is_none()
    {
        return Err("WebAuthn is not supported in this browser.".to_string());
    }

    let ccr: CreationChallengeResponse =
        gloo_net::http::Request::post("/api/auth/passkey/register/start")
            .send()
            .await
            .map_err(|e| format!("Could not start passkey registration: {e}"))?
            .json()
            .await
            .map_err(|e| format!("Could not parse registration challenge: {e}"))?;

    let device_name = web_sys::window()
        .and_then(|w| {
            w.prompt_with_message_and_default(
                "Name this passkey (e.g. \"YubiKey\", \"Touch ID\", \"Windows Hello\"):",
                "Security Key",
            )
            .ok()
        })
        .flatten()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "Security Key".to_string());

    let mut ccr = ccr;
    if let Some(selection) = ccr.public_key.authenticator_selection.as_mut() {
        selection.resident_key = Some(ResidentKeyRequirement::Required);
        selection.require_resident_key = true;
    }
    let c_options: web_sys::CredentialCreationOptions = ccr.into();

    let window = web_sys::window().ok_or_else(|| "No window".to_string())?;
    let promise = window
        .navigator()
        .credentials()
        .create_with_options(&c_options)
        .map_err(|e| format!("Could not start passkey creation: {}", describe_js_error(&e)))?;
    let jsval = JsFuture::from(promise)
        .await
        .map_err(|e| format!("Passkey creation was cancelled or failed: {}", describe_js_error(&e)))?;
    let w_rpkc = web_sys::PublicKeyCredential::from(jsval);
    let rpkc = RegisterPublicKeyCredential::from(w_rpkc);
    let cred_json = serde_json::to_string(&rpkc).map_err(|e| e.to_string())?;

    let finish_body = serde_json::json!({
        "device_name": device_name,
        "credential_json": cred_json,
    });

    let resp = gloo_net::http::Request::post("/api/auth/passkey/register/finish")
        .json(&finish_body)
        .map_err(|e| format!("{e}"))?
        .send()
        .await
        .map_err(|e| format!("{e}"))?;

    if resp.ok() {
        Ok(())
    } else {
        let msg = resp
            .json::<serde_json::Value>()
            .await
            .ok()
            .and_then(|v| {
                v.get("error")
                    .and_then(|e| e.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "Passkey enrollment failed.".to_string());
        Err(msg)
    }
}
