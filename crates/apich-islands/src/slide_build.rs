//! Cross-compilation target picker and build-progress bar for cargo-slide presentations.
//!
//! Replaces a plain synchronous download link (which blocked the whole request for as long as
//! the build took, with no feedback) with an async job: start the build, then poll its real
//! percentage -- sourced from actual `cargo build` compiler-artifact events, not a simulated
//! animation -- until it's done, then offer the finished binary as a download.

use leptos::prelude::*;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BuildStatusResponse {
    status: String,
    percent: Option<u8>,
    message: Option<String>,
    error: Option<String>,
}

#[island]
pub fn SlideBuildIsland(
    #[prop(into)] project_id: String,
    #[prop(into)] file_path: String,
) -> impl IntoView {
    // Defaults to "host" (dynamically linked against the sandbox image's own glibc/X11), NOT one
    // of the musl targets -- confirmed live: a `x86_64-unknown-linux-musl` build is *fully static*
    // (Rust's musl targets default `crt-static` on), and the player's window (`minifb`) opens its
    // X11/Wayland backend via `dlopen()` at runtime, not a compile-time link. A fully static
    // binary has no dynamic linker segment for `dlopen()` to work at all -- musl's own libc stub
    // for it unconditionally returns "Dynamic loading not supported", on every machine, X server
    // or not. So the musl targets can never open a presentation window; they were briefly this
    // picker's default under the mistaken assumption that "static" meant "more portable" here,
    // which made the download strictly worse (guaranteed failure everywhere, vs. sometimes-works).
    // They're kept in the list below only for someone building a *headless* consumer of the
    // deck's data, not for anyone who plans to actually watch the presentation run.
    // "host" being dynamically linked against glibc/X11 means the binary can still fail at
    // *runtime* on a sufficiently different Linux machine (mismatched shared library versions --
    // e.g. the `BadWindow`/`X_DeleteProperty` report that started this) -- there's no target this
    // picker can offer that's both statically self-contained and able to open a real window, since
    // `dlopen`-based windowing fundamentally requires a dynamic linker to exist at runtime.
    let target = RwSignal::new("host".to_string());
    let job_id = RwSignal::new(None::<String>);
    let percent = RwSignal::new(0u8);
    let status_text = RwSignal::new(String::new());
    let is_running = RwSignal::new(false);
    let is_done = RwSignal::new(false);
    let error_msg = RwSignal::new(None::<String>);

    let project_id = StoredValue::new(project_id);
    let file_path = StoredValue::new(file_path);

    let start_build = move |_| {
        is_running.set(true);
        is_done.set(false);
        error_msg.set(None);
        percent.set(0);
        status_text.set("Starting build...".to_string());
        job_id.set(None);
        start_build_request(
            project_id.with_value(Clone::clone),
            file_path.with_value(Clone::clone),
            target.get_untracked(),
            job_id,
            status_text,
            percent,
            is_running,
            is_done,
            error_msg,
        );
    };

    // Gated on `is_done`, not just `job_id` being known -- a job id exists from the moment the
    // build *starts* (needed for status polling), well before it's actually finished, so a link
    // that appeared as soon as the id was known would be clickable while the build is still
    // running and 409 against `slide_binary_download_action` (confirmed live: this was a real
    // bug here, not hypothetical).
    let download_href = move || {
        if !is_done.get() {
            return None;
        }
        job_id.get().map(|jid| {
            format!(
                "/projects/{}/editor/slide-binary/download?job={}",
                project_id.with_value(Clone::clone),
                jid
            )
        })
    };

    view! {
        <div style="display:flex; flex-direction:column; gap:0.5rem;">
            <div style="display:flex; gap:0.4rem; align-items:center; flex-wrap:wrap;">
                <select
                    class="form-control"
                    style="width:auto; font-size:0.8rem; padding:0.3rem 0.5rem;"
                    disabled=move || is_running.get()
                    prop:value=move || target.get()
                    on:change=move |ev| target.set(event_target_value(&ev))
                >
                    <option value="host">"Linux (native, recommended -- opens a real window)"</option>
                    <option value="x86_64-pc-windows-gnu">"Windows x86_64"</option>
                    <option value="aarch64-pc-windows-gnullvm">"Windows ARM64"</option>
                    <option value="x86_64-unknown-linux-musl">"Linux x86_64 (musl, static -- cannot open a window, headless use only)"</option>
                    <option value="aarch64-unknown-linux-musl">"Linux ARM64 (musl, static -- cannot open a window, headless use only)"</option>
                </select>
                <button
                    type="button"
                    class="btn btn-secondary btn-sm"
                    disabled=move || is_running.get()
                    on:click=start_build
                    title="Compiles a standalone presentation binary for the chosen platform. Cross-compiled targets can take several minutes on first build."
                >
                    {move || if is_running.get() { "⏳ Building...".to_string() } else { "⬇️ Build & Download Binary".to_string() }}
                </button>
                {move || download_href().map(|href| view! {
                    <a href=href class="btn btn-primary btn-sm">"✅ Download binary"</a>
                })}
            </div>
            {move || is_running.get().then(|| view! {
                <div style="display:flex; align-items:center; gap:0.5rem;">
                    <div style="background:var(--bg-muted); border-radius:6px; overflow:hidden; height:6px; width:220px; border:1px solid var(--border-subtle);">
                        <div style=move || format!("background:var(--primary); height:100%; width:{}%; transition:width 0.4s ease;", percent.get())></div>
                    </div>
                    <span style="font-size:0.75rem; color:var(--text-sub); white-space:nowrap;">{move || format!("{}%", percent.get())}</span>
                </div>
            })}
            {move || (is_running.get()).then(|| view! {
                <span style="font-size:0.75rem; color:var(--text-sub);">{move || status_text.get()}</span>
            })}
            {move || error_msg.get().map(|msg| view! {
                <div class="alert alert-danger" style="font-size:0.8rem; padding:0.5rem 0.75rem; max-width:480px; white-space:pre-wrap;">{msg}</div>
            })}
        </div>
    }
}

#[cfg(feature = "hydrate")]
#[allow(clippy::too_many_arguments)]
fn start_build_request(
    project_id: String,
    file_path: String,
    target: String,
    job_id: RwSignal<Option<String>>,
    status_text: RwSignal<String>,
    percent: RwSignal<u8>,
    is_running: RwSignal<bool>,
    is_done: RwSignal<bool>,
    error_msg: RwSignal<Option<String>>,
) {
    wasm_bindgen_futures::spawn_local(async move {
        let body = serde_json::json!({ "file": file_path, "target": target });
        let start_result = gloo_net::http::Request::post(&format!(
            "/projects/{}/editor/slide-binary/start",
            project_id
        ))
        .json(&body)
        .expect("valid json body")
        .send()
        .await;

        let job = match start_result {
            | Ok(resp) => {
                match resp.json::<serde_json::Value>().await {
                    | Ok(data) => {
                        data.get("job_id")
                            .and_then(|v| v.as_str())
                            .map(str::to_string)
                    },
                    | Err(_) => None,
                }
            },
            | Err(_) => None,
        };
        let Some(job) = job else {
            error_msg.set(Some("Failed to start build".to_string()));
            is_running.set(false);
            return;
        };
        job_id.set(Some(job.clone()));

        loop {
            gloo_timers::future::TimeoutFuture::new(1500).await;
            let poll_result = gloo_net::http::Request::get(&format!(
                "/projects/{}/editor/slide-binary/status?job={}",
                project_id, job
            ))
            .send()
            .await;

            let Ok(resp) = poll_result else { continue };
            let Ok(data) = resp.json::<BuildStatusResponse>().await else {
                continue;
            };

            match data.status.as_str() {
                | "running" => {
                    percent.set(data.percent.unwrap_or(0));
                    status_text.set(data.message.unwrap_or_default());
                },
                | "done" => {
                    percent.set(100);
                    is_running.set(false);
                    is_done.set(true);
                    break;
                },
                | "failed" => {
                    error_msg.set(Some(
                        data.error.unwrap_or_else(|| "Build failed".to_string()),
                    ));
                    is_running.set(false);
                    break;
                },
                | _ => {},
            }
        }
    });
}

#[cfg(not(feature = "hydrate"))]
#[allow(clippy::too_many_arguments)]
fn start_build_request(
    _project_id: String,
    _file_path: String,
    _target: String,
    _job_id: RwSignal<Option<String>>,
    _status_text: RwSignal<String>,
    _percent: RwSignal<u8>,
    _is_running: RwSignal<bool>,
    _is_done: RwSignal<bool>,
    _error_msg: RwSignal<Option<String>>,
) {
}
