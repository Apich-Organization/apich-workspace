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

    // The build is a real server-side background job (`SlideBuildRegistry`, which keeps a
    // finished job available for two hours) -- the only thing that was ever lost by navigating
    // away was the *client's* memory of the job id, leaving a build that was still running with
    // nothing watching it. Stashing the id in `localStorage`, keyed per project+file, lets this
    // component re-attach to a build already in flight when the user comes back, whether it's
    // still running (resume the progress bar) or has since finished (offer the download).
    resume_build(ResumeArgs {
        project_id: project_id.with_value(Clone::clone),
        file_path: file_path.with_value(Clone::clone),
        job_id,
        status_text,
        percent,
        is_running,
        is_done,
        error_msg,
    });

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

    let build_btn_label = move || {
        if is_running.get() {
            "⏳ Processing...".to_string()
        } else {
            match target.get().as_str() {
                | "slide_package" => "📦 Package (.slide)".to_string(),
                | "wasm_bundle" => "🌐 Export Web (.zip)".to_string(),
                | _ => "⬇️ Build binary".to_string(),
            }
        }
    };

    let download_btn_label = move || match target.get().as_str() {
        | "slide_package" => "✅ Ready — download .slide",
        | "wasm_bundle" => "✅ Ready — download .zip",
        | _ => "✅ Ready — download binary",
    };

    let build_btn_title = move || match target.get().as_str() {
        | "slide_package" => {
            "Packages the presentation into an ultra-compressed .slide archive that can be opened locally with slide-viewer."
        },
        | "wasm_bundle" => {
            "Exports the full presentation into a standalone WebAssembly static bundle (.zip) ready to host on any web server."
        },
        | _ => {
            "Builds a standalone executable presentation you can run directly on the chosen platform. Keeps running in the background if you leave the page."
        },
    };

    view! {
        <div style="display:flex; flex-direction:column; gap:0.5rem;">
            <div style="display:flex; gap:0.4rem; align-items:center; flex-wrap:wrap;">
                <select
                    class="form-control"
                    style="width:auto; max-width:210px; font-size:0.8rem; padding:0.3rem 0.5rem;"
                    title="Choose target platform binary or packaging format"
                    disabled=move || is_running.get()
                    prop:value=move || target.get()
                    on:change=move |ev| target.set(event_target_value(&ev))
                >
                    <optgroup label="Binary Executable">
                        <option value="host">"Linux x86_64"</option>
                        <option value="aarch64-unknown-linux-gnu">"Linux ARM64"</option>
                        <option value="x86_64-pc-windows-gnu">"Windows x86_64 (.exe)"</option>
                        <option value="aarch64-pc-windows-gnullvm">"Windows ARM64 (.exe)"</option>
                        <option value="x86_64-unknown-linux-musl">"Linux x86_64 (musl static)"</option>
                        <option value="aarch64-unknown-linux-musl">"Linux ARM64 (musl static)"</option>
                    </optgroup>
                    <optgroup label="Package & Web">
                        <option value="slide_package">"📦 Package (.slide for viewer)"</option>
                        <option value="wasm_bundle">"🌐 WebAssembly Web Bundle (.zip)"</option>
                    </optgroup>
                </select>
                <button
                    type="button"
                    class="btn btn-secondary btn-sm"
                    disabled=move || is_running.get()
                    on:click=start_build
                    title=build_btn_title
                >
                    {build_btn_label}
                </button>
                {move || download_href().map(|href| view! {
                    <a href=href class="btn btn-primary btn-sm">{download_btn_label()}</a>
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
            // Reassurance that navigating away is safe -- the build really does continue, and
            // this component picks it back up on return (see `resume_build`).
            {move || is_running.get().then(|| view! {
                <span style="font-size:0.72rem; color:var(--text-light);">
                    "You can keep working or leave this page — the build carries on."
                </span>
            })}
            {move || error_msg.get().map(|msg| view! {
                <div class="alert alert-danger" style="font-size:0.8rem; padding:0.5rem 0.75rem; max-width:480px; white-space:pre-wrap;">{msg}</div>
            })}
        </div>
    }
}

struct ResumeArgs {
    project_id: String,
    file_path: String,
    job_id: RwSignal<Option<String>>,
    status_text: RwSignal<String>,
    percent: RwSignal<u8>,
    is_running: RwSignal<bool>,
    is_done: RwSignal<bool>,
    error_msg: RwSignal<Option<String>>,
}

/// `localStorage` key for the in-flight build of one specific file. Per project+file, so two
/// decks in the same project (or the same file name across projects) never adopt each other's
/// build.
#[cfg(feature = "hydrate")]
fn job_storage_key(
    project_id: &str,
    file_path: &str,
) -> String {
    format!("apich-slide-build:{project_id}:{file_path}")
}

#[cfg(feature = "hydrate")]
fn local_storage() -> Option<web_sys::Storage> {
    // Wrapped because `local_storage()` itself throws (not just returns `None`) when site data
    // is blocked, e.g. in some private-browsing modes.
    web_sys::window().and_then(|w| w.local_storage().ok().flatten())
}

#[cfg(feature = "hydrate")]
fn remember_job(
    project_id: &str,
    file_path: &str,
    job: &str,
) {
    if let Some(store) = local_storage() {
        let _ = store.set_item(&job_storage_key(project_id, file_path), job);
    }
}

#[cfg(feature = "hydrate")]
fn forget_job(
    project_id: &str,
    file_path: &str,
) {
    if let Some(store) = local_storage() {
        let _ = store.remove_item(&job_storage_key(project_id, file_path));
    }
}

/// Re-attaches to a build already in flight (or already finished) for this file, so leaving the
/// page and coming back shows live progress -- or a ready download -- instead of a reset button
/// with no sign the build ever happened. Called once as the island mounts.
#[cfg(feature = "hydrate")]
fn resume_build(args: ResumeArgs) {
    let ResumeArgs {
        project_id,
        file_path,
        job_id,
        status_text,
        percent,
        is_running,
        is_done,
        error_msg,
    } = args;

    // Must run from an `Effect`, not straight from the island body. The body executes *during*
    // hydration, when Leptos is matching its reactive graph onto the server-rendered DOM and
    // takes that markup to be already current -- signal writes made there never get patched
    // into the existing nodes, so the component kept rendering the idle "Build presentation"
    // state even with a live job id in storage and the server reporting it as running
    // (reproduced exactly that way). An `Effect` runs only after hydration has finished, so
    // these writes land as real updates. Same reason `terminal.rs` flips its `hydrated` flag
    // from an `Effect` rather than inline.
    Effect::new(move |_| {
        let Some(job) = local_storage()
            .and_then(|s| {
                s.get_item(&job_storage_key(&project_id, &file_path))
                    .ok()
                    .flatten()
            })
            .filter(|j| !j.is_empty())
        else {
            return;
        };

        job_id.set(Some(job.clone()));
        is_running.set(true);
        status_text.set("Reconnecting to build...".to_string());

        let (project_id, file_path) = (project_id.clone(), file_path.clone());
        wasm_bindgen_futures::spawn_local(async move {
            poll_until_finished(PollArgs {
                project_id,
                file_path,
                job,
                job_id,
                status_text,
                percent,
                is_running,
                is_done,
                error_msg,
            })
            .await;
        });
    });
}

#[cfg(not(feature = "hydrate"))]
fn resume_build(args: ResumeArgs) {
    args.is_running.set(false);
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
        remember_job(&project_id, &file_path, &job);

        poll_until_finished(PollArgs {
            project_id,
            file_path,
            job,
            job_id,
            status_text,
            percent,
            is_running,
            is_done,
            error_msg,
        })
        .await;
    });
}

struct PollArgs {
    project_id: String,
    file_path: String,
    job: String,
    job_id: RwSignal<Option<String>>,
    status_text: RwSignal<String>,
    percent: RwSignal<u8>,
    is_running: RwSignal<bool>,
    is_done: RwSignal<bool>,
    error_msg: RwSignal<Option<String>>,
}

/// Polls one job to completion, driving the progress signals. Shared by a freshly started build
/// and by one re-attached to on page load (`resume_build`), so both behave identically.
///
/// Not `Send`, and never needs to be: it only ever runs under `spawn_local` on the browser's
/// single-threaded event loop, and it awaits `TimeoutFuture`, which isn't `Send` either.
#[cfg(feature = "hydrate")]
#[allow(clippy::future_not_send)]
async fn poll_until_finished(args: PollArgs) {
    let PollArgs {
        project_id,
        file_path,
        job,
        job_id,
        status_text,
        percent,
        is_running,
        is_done,
        error_msg,
    } = args;
    loop {
        let poll_result = gloo_net::http::Request::get(&format!(
            "/projects/{}/editor/slide-binary/status?job={}",
            project_id, job
        ))
        .send()
        .await;

        let Ok(resp) = poll_result else {
            gloo_timers::future::TimeoutFuture::new(1500).await;
            continue;
        };
        // A job the server no longer knows about (expired past its retention window, or a
        // stale id left in storage from a previous session) must clear the stored id, or this
        // component would retry it forever on every page load.
        if resp.status() == 404 {
            forget_job(&project_id, &file_path);
            job_id.set(None);
            is_running.set(false);
            return;
        }
        let Ok(data) = resp.json::<BuildStatusResponse>().await else {
            gloo_timers::future::TimeoutFuture::new(1500).await;
            continue;
        };

        match data.status.as_str() {
            | "running" => {
                is_running.set(true);
                percent.set(data.percent.unwrap_or(0));
                status_text.set(data.message.unwrap_or_default());
            },
            | "done" => {
                percent.set(100);
                is_running.set(false);
                is_done.set(true);
                return;
            },
            | "failed" => {
                error_msg.set(Some(
                    data.error.unwrap_or_else(|| "Build failed".to_string()),
                ));
                is_running.set(false);
                forget_job(&project_id, &file_path);
                return;
            },
            | _ => {},
        }
        gloo_timers::future::TimeoutFuture::new(1500).await;
    }
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
