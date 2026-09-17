//! Multi-File Document Rendering Modal Island for Typst and LaTeX.
//!
//! Allows users to:
//! - Select multiple project files (.typ or .tex)
//! - Reorder them sequentially (who is first, who is second, etc.)
//! - Choose page break and engine settings
//! - Render them into a single unified PDF file
//! - Preview and download the combined single file directly

use leptos::prelude::*;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderFileItem {
    pub path: String,
    pub selected: bool,
}

#[derive(Debug, Clone, Serialize)]
struct MultiRenderApiPayload {
    files: Vec<String>,
    pagebreaks: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    engine: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    save_as: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct MultiRenderApiResponse {
    success: bool,
    #[serde(default)]
    pdf_url: Option<String>,
    #[serde(default)]
    download_url: Option<String>,
    #[serde(default)]
    file_count: Option<usize>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MultiRenderResult {
    pub success: bool,
    pub pdf_url: String,
    pub download_url: String,
    pub file_count: usize,
    pub error: Option<String>,
}

#[island]
pub fn MultiFileRenderModalIsland(
    #[prop(into)] project_id: String,
    #[prop(into)] active_file: String,
    #[prop(into)] doc_type: String, // "typst" | "latex"
    #[prop(into)] available_files: Vec<String>,
    #[prop(into)] is_zh: bool,
) -> impl IntoView {
    let is_latex = doc_type == "latex";
    let open = RwSignal::new(false);
    let backdrop_ref = NodeRef::<leptos::html::Div>::new();
    crate::modal::reparent_to_body(backdrop_ref);

    // Initial items: active_file is first and selected; other available files are listed
    let initial_items: Vec<RenderFileItem> = {
        let mut list = Vec::new();
        // Add active file first if in available list
        if available_files.contains(&active_file) {
            list.push(RenderFileItem {
                path: active_file.clone(),
                selected: true,
            });
        }
        for f in &available_files {
            if f != &active_file {
                list.push(RenderFileItem {
                    path: f.clone(),
                    selected: false,
                });
            }
        }
        list
    };

    let items = RwSignal::new(initial_items);
    let pagebreaks = RwSignal::new(true);
    let latex_engine = RwSignal::new("pdflatex".to_string());
    let save_source = RwSignal::new(String::new());
    let is_rendering = RwSignal::new(false);
    let render_result = RwSignal::new(None::<MultiRenderResult>);

    // Select all / Deselect all
    let select_all = move |val: bool| {
        items.update(|list| {
            for it in list.iter_mut() {
                it.selected = val;
            }
        });
    };

    // Execute multi-file render via fetch
    let on_render_click = {
        let doc_type = doc_type.clone();
        move |_| {
            let cur_items = items.get();
            let selected_files: Vec<String> = cur_items
                .into_iter()
                .filter(|it| it.selected)
                .map(|it| it.path)
                .collect();

            if selected_files.is_empty() {
                render_result.set(Some(MultiRenderResult {
                    success: false,
                    pdf_url: String::new(),
                    download_url: String::new(),
                    file_count: 0,
                    error: Some(if is_zh {
                        "请至少勾选一个文件进行联合渲染。".to_string()
                    } else {
                        "Please select at least one file to render.".to_string()
                    }),
                }));
                return;
            }

            is_rendering.set(true);
            render_result.set(None);

            let engine_opt = if doc_type == "latex" {
                Some(latex_engine.get())
            } else {
                None
            };
            let save_val = save_source.get().trim().to_string();
            let save_opt = if save_val.is_empty() {
                None
            } else {
                Some(save_val)
            };

            let payload = MultiRenderApiPayload {
                files: selected_files,
                pagebreaks: pagebreaks.get(),
                engine: engine_opt,
                save_as: save_opt,
            };

            let endpoint = format!("/projects/{project_id}/editor/render-multiple-{doc_type}");

            #[cfg(feature = "hydrate")]
            {
                let is_rendering = is_rendering;
                let render_result = render_result;
                let is_zh = is_zh;
                leptos::task::spawn_local(async move {
                    let json_body = match serde_json::to_string(&payload) {
                        Ok(s) => s,
                        Err(e) => {
                            is_rendering.set(false);
                            render_result.set(Some(MultiRenderResult {
                                success: false,
                                pdf_url: String::new(),
                                download_url: String::new(),
                                file_count: 0,
                                error: Some(format!("Failed to serialize request: {e}")),
                            }));
                            return;
                        }
                    };

                    let res = gloo_net::http::Request::post(&endpoint)
                        .header("Content-Type", "application/json")
                        .body(json_body)
                        .map_err(|e| e.to_string());

                    let resp = match res {
                        Ok(req) => req.send().await.map_err(|e| e.to_string()),
                        Err(e) => Err(e),
                    };

                    is_rendering.set(false);
                    match resp {
                        Ok(response) => {
                            if let Ok(data) = response.json::<MultiRenderApiResponse>().await {
                                if data.success {
                                    render_result.set(Some(MultiRenderResult {
                                        success: true,
                                        pdf_url: data.pdf_url.unwrap_or_default(),
                                        download_url: data.download_url.unwrap_or_default(),
                                        file_count: data.file_count.unwrap_or(payload.files.len()),
                                        error: None,
                                    }));
                                } else {
                                    render_result.set(Some(MultiRenderResult {
                                        success: false,
                                        pdf_url: String::new(),
                                        download_url: String::new(),
                                        file_count: 0,
                                        error: data.error.or_else(|| Some("Compilation failed".to_string())),
                                    }));
                                }
                            } else {
                                render_result.set(Some(MultiRenderResult {
                                    success: false,
                                    pdf_url: String::new(),
                                    download_url: String::new(),
                                    file_count: 0,
                                    error: Some(if is_zh { "服务器响应解析失败" } else { "Failed to parse server response" }.to_string()),
                                }));
                            }
                        }
                        Err(e) => {
                            render_result.set(Some(MultiRenderResult {
                                success: false,
                                pdf_url: String::new(),
                                download_url: String::new(),
                                file_count: 0,
                                error: Some(format!("Network request failed: {e}")),
                            }));
                        }
                    }
                });
            }
            #[cfg(not(feature = "hydrate"))]
            {
                let _ = (endpoint, payload);
            }
        }
    };

    let trigger_label = if is_zh {
        "📑 多文件联合渲染".to_string()
    } else {
        "📑 Render Multiple Files".to_string()
    };

    let title = if is_zh {
        if is_latex {
            "LaTeX 多文件联合渲染与合并".to_string()
        } else {
            "Typst 多文件联合渲染与合并".to_string()
        }
    } else if is_latex {
        "LaTeX Multi-File Document Compiler".to_string()
    } else {
        "Typst Multi-File Document Compiler".to_string()
    };

    let desc = if is_zh {
        "勾选需要合并的文件，点击 ⬆ / ⬇ 自由调整前后顺序，系统将按照您的指定顺序将其联合渲染为单一完整 PDF 文档。"
    } else {
        "Select files and reorder them with ⬆ / ⬇ to define who comes first and second. The compiler will merge and render them into a single unified PDF."
    };

    let btn_label = trigger_label.clone();
    let btn_title = trigger_label;

    view! {
        <button
            type="button"
            class="btn btn-secondary btn-sm"
            on:click=move |_| open.set(true)
            title=btn_title
        >
            {btn_label}
        </button>

        <div
            node_ref=backdrop_ref
            class="modal-backdrop"
            style:display=move || if open.get() { "flex" } else { "none" }
        >
            <div class="modal-card" style="max-width:760px; width:92%; max-height:90vh; display:flex; flex-direction:column; padding:1.25rem;">
                // Header
                <div class="modal-header" style="padding-bottom:0.75rem; border-bottom:1px solid var(--border-subtle); margin-bottom:1rem;">
                    <div>
                        <h3 class="modal-title" style="display:flex; align-items:center; gap:0.5rem; margin:0;">
                            <span>"📚"</span>
                            <span>{title}</span>
                            <span class="file-type-pill pill-doc" style="font-size:0.72rem; padding:0.15rem 0.5rem;">
                                {if is_latex { "LaTeX" } else { "Typst" }}
                            </span>
                        </h3>
                        <p style="font-size:0.825rem; color:var(--text-sub); margin:0.35rem 0 0 0;">
                            {desc}
                        </p>
                    </div>
                    <button type="button" class="modal-close" on:click=move |_| open.set(false)>"×"</button>
                </div>

                // Modal Body - Scrollable
                <div style="flex:1; overflow-y:auto; padding-right:0.25rem;">
                    // Toolbar: Select All / Deselect All
                    <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:0.75rem;">
                        <span style="font-size:0.8rem; font-weight:600; color:var(--text-main);">
                            {if is_zh { "文件清单与顺序编排（从上到下即为合并顺序）：" } else { "Document Sequence (Top to Bottom):" }}
                        </span>
                        <div style="display:flex; gap:0.4rem;">
                            <button
                                type="button"
                                class="btn btn-ghost btn-sm"
                                style="font-size:0.75rem; padding:0.15rem 0.5rem;"
                                on:click=move |_| select_all(true)
                            >
                                {if is_zh { "全选" } else { "Select All" }}
                            </button>
                            <button
                                type="button"
                                class="btn btn-ghost btn-sm"
                                style="font-size:0.75rem; padding:0.15rem 0.5rem;"
                                on:click=move |_| select_all(false)
                            >
                                {if is_zh { "清空选择" } else { "Deselect All" }}
                            </button>
                        </div>
                    </div>

                    // File List with ordering controls
                    <div style="display:flex; flex-direction:column; gap:0.5rem; margin-bottom:1.25rem;">
                        {move || {
                            let cur_items = items.get();
                            // Captured once here, NOT read from the signal again inside the rows
                            // below -- see the `prop:disabled` comment on the "move down" button.
                            let total = cur_items.len();
                            let mut order_counter = 0usize;

                            cur_items.into_iter().enumerate().map(|(idx, item)| {
                                let is_sel = item.selected;
                                let order_badge = if is_sel {
                                    order_counter = order_counter.saturating_add(1);
                                    format!("#{order_counter}")
                                } else {
                                    "-".to_string()
                                };

                                let bg_style = if is_sel {
                                    "background:var(--bg-surface); border:1px solid var(--primary); box-shadow:0 1px 3px rgba(0,0,0,0.08);"
                                } else {
                                    "background:var(--bg-muted); border:1px solid var(--border-subtle); opacity:0.65;"
                                };

                                let badge_style = if is_sel {
                                    "background:var(--primary); color:white; font-weight:700;"
                                } else {
                                    "background:var(--border-subtle); color:var(--text-sub); font-weight:500;"
                                };

                                view! {
                                    <div
                                        style=format!("display:flex; align-items:center; justify-content:space-between; padding:0.55rem 0.85rem; border-radius:8px; transition:all 0.15s ease; {bg_style}")
                                    >
                                        <div style="display:flex; align-items:center; gap:0.75rem; flex:1; min-width:0;">
                                            // Checkbox
                                            <input
                                                type="checkbox"
                                                prop:checked=is_sel
                                                on:change=move |_| {
                                                    items.update(|list| {
                                                        if let Some(it) = list.get_mut(idx) {
                                                            it.selected = !it.selected;
                                                        }
                                                    });
                                                }
                                                style="cursor:pointer; width:16px; height:16px; accent-color:var(--primary);"
                                            />
                                            // Order indicator
                                            <span
                                                style=format!("min-width:28px; height:22px; border-radius:12px; font-size:0.75rem; display:flex; align-items:center; justify-content:center; padding:0 0.35rem; {badge_style}")
                                            >
                                                {order_badge}
                                            </span>
                                            // File Name & Icon
                                            <div style="display:flex; align-items:center; gap:0.4rem; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;">
                                                <span style="font-size:0.95rem;">{if is_latex { "📝" } else { "📄" }}</span>
                                                <span style="font-size:0.85rem; font-family:var(--font-mono); font-weight:500; color:var(--text-main); overflow:hidden; text-overflow:ellipsis;">
                                                    {item.path}
                                                </span>
                                            </div>
                                        </div>

                                        // Move Up / Move Down buttons
                                        <div style="display:flex; align-items:center; gap:0.3rem; margin-left:0.75rem;">
                                            <button
                                                type="button"
                                                class="btn btn-secondary btn-sm"
                                                style="padding:0.2rem 0.5rem; font-size:0.75rem;"
                                                prop:disabled=idx == 0
                                                on:click=move |_| {
                                                    if idx > 0 {
                                                        items.update(|list| {
                                                            list.swap(idx, idx.saturating_sub(1));
                                                        });
                                                    }
                                                }
                                                title={if is_zh { "上移（优先渲染）" } else { "Move Earlier" }}
                                            >
                                                "⬆️"
                                            </button>
                                            <button
                                                type="button"
                                                class="btn btn-secondary btn-sm"
                                                style="padding:0.2rem 0.5rem; font-size:0.75rem;"
                                                // Deliberately a plain value, not `move || ...
                                                // items.get() ...`. This button is rendered inside
                                                // the enclosing `move ||` block that already reads
                                                // `items`, so a nested closure reading the same
                                                // signal registers a second reactive subscriber
                                                // whose owner is the very DOM subtree that the
                                                // outer block tears down and rebuilds on every
                                                // `items` change. Updating `items` (ticking a
                                                // checkbox, pressing ⬆/⬇) then invoked that inner
                                                // closure against a scope being disposed in the
                                                // same update, which traps the whole wasm module
                                                // with "closure invoked recursively or after being
                                                // dropped" -- and because that kills the shared
                                                // module, *every* island on the page stops
                                                // responding, not just this modal. `total` comes
                                                // from the same snapshot the row was built from,
                                                // so it is always correct for this render pass.
                                                // Parenthesised on purpose: `view!`'s parser ends
                                                // an opening tag at the first `>`, so an
                                                // unbracketed `>=` here silently truncates the
                                                // attribute instead of comparing.
                                                prop:disabled={idx.saturating_add(1) >= total}
                                                on:click=move |_| {
                                                    items.update(|list| {
                                                        if idx.saturating_add(1) < list.len() {
                                                            list.swap(idx, idx.saturating_add(1));
                                                        }
                                                    });
                                                }
                                                title={if is_zh { "下移（靠后渲染）" } else { "Move Later" }}
                                            >
                                                "⬇️"
                                            </button>
                                        </div>
                                    </div>
                                }
                            }).collect::<Vec<_>>()
                        }}
                    </div>

                    // Options Grid
                    <div style="display:grid; grid-template-columns:repeat(auto-fit, minmax(220px, 1fr)); gap:0.75rem; background:var(--bg-muted); border:1px solid var(--border-subtle); border-radius:8px; padding:0.85rem; margin-bottom:1.25rem;">
                        // Page breaks option
                        <label style="display:flex; align-items:center; gap:0.5rem; cursor:pointer; font-size:0.825rem; font-weight:500; color:var(--text-main); margin:0;">
                            <input
                                type="checkbox"
                                prop:checked=move || pagebreaks.get()
                                on:change=move |ev| pagebreaks.set(event_target_checked(&ev))
                                style="width:16px; height:16px; accent-color:var(--primary);"
                            />
                            <span>{if is_zh { "文件之间自动插入分页符" } else { "Separate files with page breaks" }}</span>
                        </label>

                        // Engine select (LaTeX only)
                        {is_latex.then(|| view! {
                            <div style="display:flex; align-items:center; gap:0.4rem;">
                                <span style="font-size:0.8rem; color:var(--text-sub);">{if is_zh { "编译引擎：" } else { "Engine:" }}</span>
                                <select
                                    class="form-control"
                                    style="width:auto; padding:0.2rem 0.5rem; font-size:0.8rem;"
                                    prop:value=move || latex_engine.get()
                                    on:change=move |ev| latex_engine.set(event_target_value(&ev))
                                >
                                    <option value="pdflatex">"pdflatex"</option>
                                    <option value="xelatex">"xelatex"</option>
                                    <option value="lualatex">"lualatex"</option>
                                </select>
                            </div>
                        })}

                        // Save merged source name (Optional)
                        <div style="display:flex; align-items:center; gap:0.4rem;">
                            <span style="font-size:0.8rem; color:var(--text-sub); white-space:nowrap;">
                                {if is_zh { "保存合并源文件：" } else { "Save merged as:" }}
                            </span>
                            <input
                                type="text"
                                class="form-control"
                                style="padding:0.2rem 0.5rem; font-size:0.8rem;"
                                placeholder={if is_zh { "可选，如 combined.typ" } else { "Optional, e.g. combined.typ" }}
                                prop:value=move || save_source.get()
                                on:input=move |ev| save_source.set(event_target_value(&ev))
                            />
                        </div>
                    </div>

                    // Result Panel: Success or Error
                    {move || render_result.get().map(|res| {
                        if res.success {
                            let pdf_url = res.pdf_url.clone();
                            let dl_url = res.download_url.clone();
                            let count = res.file_count;
                            view! {
                                <div style="margin-bottom:1.25rem; background:rgba(34,197,94,0.08); border:1px solid rgba(34,197,94,0.3); border-radius:8px; padding:1rem;">
                                    <div style="display:flex; align-items:center; justify-content:space-between; flex-wrap:wrap; gap:0.5rem; margin-bottom:0.75rem;">
                                        <div style="display:flex; align-items:center; gap:0.5rem; color:#16a34a; font-weight:600; font-size:0.9rem;">
                                            <span>"✅"</span>
                                            <span>
                                                {if is_zh {
                                                    format!("成功将 {count} 个文件联合渲染为单一文档！")
                                                } else {
                                                    format!("Successfully rendered {count} files into a single document!")
                                                }}
                                            </span>
                                        </div>
                                        <div style="display:flex; gap:0.5rem;">
                                            <a
                                                href=dl_url
                                                class="btn btn-primary btn-sm"
                                                style="display:flex; align-items:center; gap:0.35rem;"
                                                download=format!("combined_{doc_type}.pdf")
                                            >
                                                <span>"⬇️"</span>
                                                <span>{if is_zh { "下载合并后 PDF" } else { "Download Combined PDF" }}</span>
                                            </a>
                                            <a
                                                href=pdf_url.clone()
                                                target="_blank"
                                                class="btn btn-secondary btn-sm"
                                            >
                                                <span>"↗️"</span>
                                                <span>{if is_zh { "新标签页打开" } else { "Open in New Tab" }}</span>
                                            </a>
                                        </div>
                                    </div>
                                    // Embedded preview frame
                                    <div style="border-radius:6px; overflow:hidden; border:1px solid var(--border-subtle); background:#525659; height:320px;">
                                        <iframe
                                            src=pdf_url
                                            style="width:100%; height:100%; border:none;"
                                            title="Combined Document Preview"
                                        ></iframe>
                                    </div>
                                </div>
                            }.into_any()
                        } else {
                            let err_msg = res.error.unwrap_or_else(|| "Unknown compilation error".to_string());
                            view! {
                                <div style="margin-bottom:1.25rem; background:rgba(239,68,68,0.08); border:1px solid rgba(239,68,68,0.3); border-radius:8px; padding:1rem;">
                                    <div style="color:#dc2626; font-weight:600; font-size:0.875rem; margin-bottom:0.5rem; display:flex; align-items:center; gap:0.4rem;">
                                        <span>"⚠️"</span>
                                        <span>{if is_zh { "联合渲染失败：" } else { "Multi-File Compilation Failed:" }}</span>
                                    </div>
                                    <pre style="background:#1e1e1e; color:#f87171; font-family:var(--font-mono); font-size:0.78rem; padding:0.75rem; border-radius:6px; overflow:auto; max-height:180px; margin:0; white-space:pre-wrap;">
                                        {err_msg}
                                    </pre>
                                </div>
                            }.into_any()
                        }
                    })}
                </div>

                // Modal Footer
                <div style="display:flex; justify-content:flex-end; align-items:center; gap:0.75rem; border-top:1px solid var(--border-subtle); padding-top:0.85rem; margin-top:0.5rem;">
                    <button
                        type="button"
                        class="btn btn-secondary"
                        on:click=move |_| open.set(false)
                    >
                        {if is_zh { "关闭" } else { "Close" }}
                    </button>
                    <button
                        type="button"
                        class="btn btn-primary"
                        prop:disabled=move || is_rendering.get()
                        on:click=on_render_click
                        style="display:flex; align-items:center; gap:0.5rem;"
                    >
                        {move || if is_rendering.get() {
                            view! {
                                <span class="spinner" style="width:14px; height:14px; border:2px solid rgba(255,255,255,0.3); border-top-color:white; border-radius:50%; animation:uploadSpin 0.7s linear infinite;"></span>
                                <span>{if is_zh { "正在联合渲染..." } else { "Rendering to Single File..." }}</span>
                            }.into_any()
                        } else {
                            view! {
                                <span>"🚀"</span>
                                <span>{if is_zh { "渲染为单一文件" } else { "Render to Single File" }}</span>
                            }.into_any()
                        }}
                    </button>
                </div>
            </div>
        </div>
    }
}
