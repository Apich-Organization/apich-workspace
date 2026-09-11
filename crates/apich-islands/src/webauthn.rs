//! Real Rust replacement for the two hand-written WebAuthn (FIDO2/passkey) ceremony scripts
//! that used to live as `<script>` strings in `login.rs` and `settings.rs`. Both talk to the
//! same real backend endpoints (`/api/auth/passkey/...`) with the exact same payload shapes the
//! original JS used -- only how the ceremony is driven in the browser changed, not the wire
//! protocol.

use leptos::prelude::*;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Deserialize)]
struct AuthStartResponse {
    challenge: String,
    timeout: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
struct AuthFinishRequest {
    credential_id: String,
    challenge: String,
    client_data_json_base64: String,
    auth_data_base64: String,
    signature_base64: String,
}

#[derive(Debug, Clone, Deserialize)]
struct RegisterStartResponse {
    challenge: String,
    rp: RpEntity,
    user: UserEntity,
}

#[derive(Debug, Clone, Deserialize)]
struct RpEntity {
    name: String,
    id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct UserEntity {
    id: String,
    name: String,
    display_name: String,
}

#[derive(Debug, Clone, Serialize)]
struct RegisterFinishRequest {
    credential_id: String,
    public_key_base64: String,
    device_name: Option<String>,
}

/// The passkey login pane on the login page. `login_button_label` and `hint_text` are passed in
/// already-i18n-resolved from the server rather than baking i18n into the island.
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
                // Matches the original script's behavior: reload so the new passkey shows up
                // in the enrolled-credentials list rendered server-side.
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
fn run_passkey_enroll(
    _status: RwSignal<String>,
    _is_error: RwSignal<bool>,
    _busy: RwSignal<bool>,
) {
}

#[cfg(feature = "hydrate")]
async fn passkey_login_ceremony(return_to: String) -> Result<(), String> {
    use wasm_bindgen::JsCast;

    if web_sys::window()
        .and_then(|w| js_sys::Reflect::get(&w, &"PublicKeyCredential".into()).ok())
        .is_none()
    {
        return Err(
            "WebAuthn is not supported in this browser. Please use Password login.".to_string(),
        );
    }

    let start: AuthStartResponse = gloo_net::http::Request::post("/api/auth/passkey/auth/start")
        .send()
        .await
        .map_err(|e| format!("Could not retrieve login challenge: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Could not retrieve login challenge: {e}"))?;

    let challenge_bytes =
        b64url_decode(&start.challenge).map_err(|e| format!("Bad challenge from server: {e}"))?;
    let challenge_array = js_sys::Uint8Array::from(challenge_bytes.as_slice());

    let req_opts = web_sys::PublicKeyCredentialRequestOptions::new(&challenge_array);
    req_opts.set_timeout(start.timeout.unwrap_or(60000) as u32);
    let hostname = web_sys::window()
        .and_then(|w| w.location().hostname().ok())
        .unwrap_or_default();
    req_opts.set_rp_id(&hostname);

    let cred_opts = web_sys::CredentialRequestOptions::new();
    cred_opts.set_public_key(&req_opts);

    let navigator = web_sys::window().ok_or("No window")?.navigator();
    let promise = navigator
        .credentials()
        .get_with_options(&cred_opts)
        .map_err(|e| format!("Passkey request failed: {}", js_error_string(&e)))?;
    let credential = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|e| format!("{}", js_error_string(&e)))?;
    let credential: web_sys::PublicKeyCredential = credential
        .dyn_into()
        .map_err(|_| "No credential selected.".to_string())?;

    let auth_response = credential.response();
    let assertion: web_sys::AuthenticatorAssertionResponse = auth_response
        .dyn_into()
        .map_err(|_| "Unexpected credential response type.".to_string())?;

    let client_data_json = array_buffer_to_vec(&assertion.client_data_json());
    let auth_data = array_buffer_to_vec(&assertion.authenticator_data());
    let signature = array_buffer_to_vec(&assertion.signature());

    let cred_id: web_sys::Credential = credential.unchecked_into();
    let credential_id = cred_id.id();

    let finish_body = AuthFinishRequest {
        credential_id,
        challenge: start.challenge,
        client_data_json_base64: b64_standard_encode(&client_data_json),
        auth_data_base64: b64_standard_encode(&auth_data),
        signature_base64: b64_standard_encode(&signature),
    };

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
    use wasm_bindgen::JsCast;

    if web_sys::window()
        .and_then(|w| js_sys::Reflect::get(&w, &"PublicKeyCredential".into()).ok())
        .is_none()
    {
        return Err("WebAuthn is not supported in this browser.".to_string());
    }

    let start: RegisterStartResponse =
        gloo_net::http::Request::post("/api/auth/passkey/register/start")
            .send()
            .await
            .map_err(|e| format!("Could not start passkey registration: {e}"))?
            .json()
            .await
            .map_err(|e| format!("Could not start passkey registration: {e}"))?;

    let device_name = web_sys::window()
        .and_then(|w| {
            w.prompt_with_message_and_default(
                "Name this passkey (e.g. \"YubiKey\", \"Touch ID\"):",
                "Security Key",
            )
            .ok()
        })
        .flatten()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "Security Key".to_string());

    let challenge_bytes =
        b64url_decode(&start.challenge).map_err(|e| format!("Bad challenge from server: {e}"))?;
    let challenge_array = js_sys::Uint8Array::from(challenge_bytes.as_slice());

    let rp = web_sys::PublicKeyCredentialRpEntity::new(&start.rp.name);
    rp.set_id(&start.rp.id);

    let user_id_bytes = start.user.id.as_bytes();
    let user_id_array = js_sys::Uint8Array::from(user_id_bytes);
    let user = web_sys::PublicKeyCredentialUserEntity::new_with_u8_array(
        &start.user.name,
        &start.user.display_name,
        &user_id_array,
    );

    let pub_key_param = web_sys::PublicKeyCredentialParameters::new(
        -7,
        web_sys::PublicKeyCredentialType::PublicKey,
    );
    let params = js_sys::Array::new();
    params.push(&pub_key_param);

    let create_opts =
        web_sys::PublicKeyCredentialCreationOptions::new(&challenge_array, &params, &rp, &user);
    create_opts.set_timeout(60000);

    let cred_opts = web_sys::CredentialCreationOptions::new();
    cred_opts.set_public_key(&create_opts);

    let navigator = web_sys::window().ok_or("No window")?.navigator();
    let promise = navigator
        .credentials()
        .create_with_options(&cred_opts)
        .map_err(|e| format!("Passkey creation failed: {}", js_error_string(&e)))?;
    let credential = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|e| format!("{}", js_error_string(&e)))?;
    let credential: web_sys::PublicKeyCredential = credential
        .dyn_into()
        .map_err(|_| "No credential created.".to_string())?;

    let auth_response = credential.response();
    let attestation: web_sys::AuthenticatorAttestationResponse = auth_response
        .dyn_into()
        .map_err(|_| "Unexpected credential response type.".to_string())?;

    let public_key = attestation
        .get_public_key()
        .map_err(|e| format!("{}", js_error_string(&e)))?
        .ok_or("Browser did not return a public key (getPublicKey unsupported).".to_string())?;
    let public_key_bytes = array_buffer_to_vec(&public_key);

    let cred_id: web_sys::Credential = credential.unchecked_into();
    let credential_id = cred_id.id();

    let finish_body = RegisterFinishRequest {
        credential_id,
        public_key_base64: b64_standard_encode(&public_key_bytes),
        device_name: Some(device_name),
    };

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

#[cfg(feature = "hydrate")]
fn array_buffer_to_vec(buf: &js_sys::ArrayBuffer) -> Vec<u8> {
    js_sys::Uint8Array::new(buf).to_vec()
}

#[cfg(feature = "hydrate")]
fn b64_standard_encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

#[cfg(feature = "hydrate")]
fn b64url_decode(s: &str) -> Result<Vec<u8>, String> {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(s)
        .or_else(|_| base64::engine::general_purpose::STANDARD.decode(s))
        .map_err(|e| e.to_string())
}

#[cfg(feature = "hydrate")]
fn js_error_string(e: &wasm_bindgen::JsValue) -> String {
    e.as_string()
        .or_else(|| {
            js_sys::Reflect::get(e, &"message".into())
                .ok()
                .and_then(|v| v.as_string())
        })
        .unwrap_or_else(|| "Unknown error".to_string())
}
