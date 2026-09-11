//! "Integrate python and r into the tables: just like a python notebook and r notebook" (direct
//! user request). Real Rust island for the per-table notebook (see
//! `crates/apich-web/src/services/sqlite_table.rs`'s `NotebookCell` doc comment for the full
//! storage/execution design -- this file is just the client-side editor/run/output UI).
//!
//! Each cell is its own fresh Python or R process (no persistent kernel, no shared variables
//! between cells) run via the server's existing script-execution + plot-capture infrastructure,
//! against the real table's SQLite file (handed to the cell via the `APICH_TABLE_DB` env var --
//! see the starter code a new cell is created with). Code editing here is a plain `<textarea>`,
//! not the syntax-highlighted overlay the document/note editors use -- that overlay's bootstrap
//! script (`code_highlight.rs`) scans the DOM once, on page load, for `.code-editor-wrap`
//! elements; cells created after that (this island's own "+ New Cell" buttons trigger a real
//! page reload today, but a future live-add-without-reload version wouldn't get picked up)
//! would need that scanner taught to watch for dynamically-added cells first. Left as a real,
//! named scope decision rather than silently half-wiring highlighting that would only work for
//! cells present at initial page load.

use leptos::prelude::*;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookCellImageData {
    pub name: String,
    pub data_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookCellData {
    pub id: i64,
    pub language: String,
    pub code: String,
    pub output: String,
    pub output_images: Vec<NotebookCellImageData>,
}

#[derive(Clone)]
struct CellRuntime {
    id: i64,
    language: String,
    code: RwSignal<String>,
    output: RwSignal<String>,
    images: RwSignal<Vec<NotebookCellImageData>>,
    running: RwSignal<bool>,
}

#[island]
pub fn NotebookIsland(
    #[prop(into)] project_id: String,
    #[prop(into)] file_path: String,
    #[prop(into)] table_name: String,
    cells: Vec<NotebookCellData>,
) -> impl IntoView {
    let runtimes: Vec<CellRuntime> = cells
        .into_iter()
        .map(|c| {
            CellRuntime {
                id: c.id,
                language: c.language,
                code: RwSignal::new(c.code),
                output: RwSignal::new(c.output),
                images: RwSignal::new(c.output_images),
                running: RwSignal::new(false),
            }
        })
        .collect();
    let runtimes = RwSignal::new(runtimes);

    let run_cell = {
        let project_id = project_id.clone();
        let file_path = file_path.clone();
        move |cell: CellRuntime| {
            let project_id = project_id.clone();
            let file_path = file_path.clone();
            cell.running.set(true);
            cell.output.set("Running...".to_string());
            run_notebook_cell_request(
                project_id,
                file_path,
                cell.id,
                cell.language.clone(),
                cell.code.get_untracked(),
                cell.output,
                cell.images,
                cell.running,
            );
        }
    };

    let run_all = {
        let project_id = project_id.clone();
        let file_path = file_path.clone();
        move |_| {
            let project_id = project_id.clone();
            let file_path = file_path.clone();
            let cells = runtimes.get_untracked();
            spawn_local_run_all(project_id, file_path, cells);
        }
    };
    let any_running = move || runtimes.get().iter().any(|c| c.running.get());

    let project_id_new = project_id.clone();
    let file_path_new = file_path.clone();
    let table_name_new = table_name.clone();

    let cell_views = move || {
        let all_cells = runtimes.get();
        let n_cells = all_cells.len();
        all_cells
            .into_iter()
            .enumerate()
            .map(|(idx, cell)| {
                let lang_label = if cell.language == "r" { "R" } else { "Python" };
                let lang_class = if cell.language == "r" { "pill-latex" } else { "pill-script" };
                let run = run_cell.clone();
                let cell_for_run = cell.clone();
                let cell_for_delete = cell.clone();
                let project_id_del = project_id.clone();
                let file_path_del = file_path.clone();
                let table_name_del = table_name.clone();
                let project_id_move = project_id.clone();
                let file_path_move = file_path.clone();
                let table_name_move = table_name.clone();
                let is_first = idx == 0;
                let is_last = idx + 1 == n_cells;
                view! {
                    <div style="border:1px solid var(--border-subtle); border-radius:8px; margin-bottom:0.85rem; overflow:hidden;">
                        <div style="display:flex; justify-content:space-between; align-items:center; padding:0.5rem 0.75rem; background:var(--bg-muted); border-bottom:1px solid var(--border-subtle);">
                            <span class=format!("file-type-pill {}", lang_class) style="font-size:0.7rem;">{lang_label}</span>
                            <div style="display:flex; gap:0.4rem; align-items:center;">
                                <form method="post" action=format!("/projects/{}/table/notebook/cell-move", project_id_move)>
                                    <input type="hidden" name="file" value=file_path_move.clone() />
                                    <input type="hidden" name="table" value=table_name_move.clone() />
                                    <input type="hidden" name="cell_id" value=cell.id.to_string() />
                                    <input type="hidden" name="direction" value="up" />
                                    <button type="submit" class="btn btn-ghost btn-sm" disabled=is_first title="Move cell up">"▲"</button>
                                </form>
                                <form method="post" action=format!("/projects/{}/table/notebook/cell-move", project_id_move.clone())>
                                    <input type="hidden" name="file" value=file_path_move.clone() />
                                    <input type="hidden" name="table" value=table_name_move.clone() />
                                    <input type="hidden" name="cell_id" value=cell.id.to_string() />
                                    <input type="hidden" name="direction" value="down" />
                                    <button type="submit" class="btn btn-ghost btn-sm" disabled=is_last title="Move cell down">"▼"</button>
                                </form>
                                <button
                                    type="button"
                                    class="btn btn-primary btn-sm"
                                    disabled=move || cell_for_run.running.get()
                                    on:click=move |_| run(cell_for_run.clone())
                                >
                                    {move || if cell_for_delete.running.get() { "Running..." } else { "▶ Run" }}
                                </button>
                                <form method="post" action=format!("/projects/{}/table/notebook/cell-delete", project_id_del)>
                                    <input type="hidden" name="file" value=file_path_del.clone() />
                                    <input type="hidden" name="table" value=table_name_del.clone() />
                                    <input type="hidden" name="cell_id" value=cell.id.to_string() />
                                    <button type="submit" class="btn btn-ghost btn-sm" title="Delete cell">"🗑️"</button>
                                </form>
                            </div>
                        </div>
                        <textarea
                            class="form-control"
                            style="width:100%; min-height:120px; font-family:var(--font-mono); font-size:0.82rem; border:none; border-radius:0; resize:vertical; padding:0.75rem;"
                            spellcheck="false"
                            prop:value=move || cell.code.get()
                            on:input=move |ev| cell.code.set(event_target_value(&ev))
                        ></textarea>
                        {
                            let output = cell.output;
                            let images = cell.images;
                            move || {
                                let out = output.get();
                                let imgs = images.get();
                                if out.is_empty() && imgs.is_empty() {
                                    return view! { <div></div> }.into_any();
                                }
                                let image_views: Vec<_> = imgs
                                    .iter()
                                    .map(|img| view! {
                                        <img src=img.data_uri.clone() alt=img.name.clone() style="max-width:100%; margin-top:0.5rem; border-radius:6px; border:1px solid var(--border-subtle);" />
                                    })
                                    .collect();
                                view! {
                                    <div style="padding:0.75rem; background:#0f172a; color:#e2e8f0; font-family:var(--font-mono); font-size:0.8rem; white-space:pre-wrap; max-height:320px; overflow-y:auto;">
                                        {out}
                                        {image_views}
                                    </div>
                                }.into_any()
                            }
                        }
                    </div>
                }
            })
            .collect::<Vec<_>>()
    };

    view! {
        <div>
            {cell_views}
            <div style="display:flex; gap:0.5rem;">
                <button type="button" class="btn btn-primary btn-sm" disabled=any_running on:click=run_all>
                    {move || if any_running() { "Running...".to_string() } else { "▶▶ Run All".to_string() }}
                </button>
                <form method="post" action=format!("/projects/{}/table/notebook/cell-create", project_id_new)>
                    <input type="hidden" name="file" value=file_path_new.clone() />
                    <input type="hidden" name="table" value=table_name_new.clone() />
                    <input type="hidden" name="language" value="python" />
                    <button type="submit" class="btn btn-secondary btn-sm">"+ New Python Cell"</button>
                </form>
                <form method="post" action=format!("/projects/{}/table/notebook/cell-create", project_id_new)>
                    <input type="hidden" name="file" value=file_path_new.clone() />
                    <input type="hidden" name="table" value=table_name_new.clone() />
                    <input type="hidden" name="language" value="r" />
                    <button type="submit" class="btn btn-secondary btn-sm">"+ New R Cell"</button>
                </form>
            </div>
        </div>
    }
}

#[cfg(feature = "hydrate")]
#[allow(clippy::too_many_arguments)]
async fn run_one_cell(
    project_id: &str,
    file_path: &str,
    cell_id: i64,
    language: &str,
    code: String,
    output: RwSignal<String>,
    images: RwSignal<Vec<NotebookCellImageData>>,
) {
    let body = serde_json::json!({ "file": file_path, "cell_id": cell_id, "language": language, "code": code });
    let result =
        gloo_net::http::Request::post(&format!("/projects/{}/table/notebook/run-cell", project_id))
            .json(&body)
            .expect("valid json body")
            .send()
            .await;
    match result {
        | Ok(resp) => {
            match resp.json::<serde_json::Value>().await {
                | Ok(data) => {
                    if let Some(err) = data.get("error").and_then(|v| v.as_str()) {
                        output.set(format!("Error: {err}"));
                    } else {
                        let out = data
                            .get("output")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string();
                        output.set(out);
                        let imgs: Vec<NotebookCellImageData> = data
                            .get("output_images")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|i| {
                                        Some(NotebookCellImageData {
                                            name: i.get("name")?.as_str()?.to_string(),
                                            data_uri: i.get("data_uri")?.as_str()?.to_string(),
                                        })
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();
                        images.set(imgs);
                    }
                },
                | Err(e) => output.set(format!("Request failed: {e}")),
            }
        },
        | Err(e) => output.set(format!("Request failed: {e}")),
    }
}

#[cfg(feature = "hydrate")]
#[allow(clippy::too_many_arguments)]
fn run_notebook_cell_request(
    project_id: String,
    file_path: String,
    cell_id: i64,
    language: String,
    code: String,
    output: RwSignal<String>,
    images: RwSignal<Vec<NotebookCellImageData>>,
    running: RwSignal<bool>,
) {
    wasm_bindgen_futures::spawn_local(async move {
        run_one_cell(
            &project_id,
            &file_path,
            cell_id,
            &language,
            code,
            output,
            images,
        )
        .await;
        running.set(false);
    });
}
#[cfg(not(feature = "hydrate"))]
#[allow(clippy::too_many_arguments)]
fn run_notebook_cell_request(
    _project_id: String,
    _file_path: String,
    _cell_id: i64,
    _language: String,
    _code: String,
    _output: RwSignal<String>,
    _images: RwSignal<Vec<NotebookCellImageData>>,
    _running: RwSignal<bool>,
) {
}

/// "Run All": executes every cell's current code in top-to-bottom order, one at a time (never in
/// parallel -- each cell is its own fresh interpreter process in the project's sandbox, and
/// running them all at once would needlessly pile up concurrent processes there for no benefit,
/// since cells don't share state anyway).
#[cfg(feature = "hydrate")]
fn spawn_local_run_all(
    project_id: String,
    file_path: String,
    cells: Vec<CellRuntime>,
) {
    wasm_bindgen_futures::spawn_local(async move {
        for cell in cells {
            cell.running.set(true);
            cell.output.set("Running...".to_string());
            run_one_cell(
                &project_id,
                &file_path,
                cell.id,
                &cell.language,
                cell.code.get_untracked(),
                cell.output,
                cell.images,
            )
            .await;
            cell.running.set(false);
        }
    });
}
#[cfg(not(feature = "hydrate"))]
fn spawn_local_run_all(
    _project_id: String,
    _file_path: String,
    _cells: Vec<CellRuntime>,
) {
}
