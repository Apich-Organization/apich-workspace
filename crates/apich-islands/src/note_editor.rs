//! Real Rust replacement for the unified note editor's hand-written JS (`note_page.rs`'s
//! `NOTE_EDITOR_SCRIPT`): outline click-to-jump, the task-checkbox toggle (a real POST that
//! rewrites the physical note file server-side and refreshes the textarea), the live-preview hot
//! reload, and KaTeX re-render are all real Leptos/WASM `on:click` handlers here. None of these
//! need to survive being clicked before this island's WASM has hydrated the way the AI Copilot
//! drawer's toggle button did (see `ai_drawer.rs`, now a pure CSS checkbox-hack with no JS
//! involved at all): a user can only reach a rendered note's outline/preview/checkboxes after the
//! page has already rendered them, by which point hydration has reliably finished. The
//! rendered-markdown preview's paragraphs/headings carry real `data-line` attributes from
//! `DocumentRenderer` (not `onclick="jumpToLine(...)"` strings), so one `on:click` handler on the
//! preview pane covers every clickable element inside it via `Element::closest`.

use leptos::prelude::*;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteHeadingItem {
    pub level: u8,
    pub text: String,
    pub line: u32,
}

#[island]
pub fn NoteEditorIsland(
    #[prop(into)] project_id: String,
    #[prop(into)] file_path: String,
    #[prop(into)] body_content: String,
    headings: Vec<NoteHeadingItem>,
    backlinks: Vec<String>,
    #[prop(into)] rendered_html: String,
    #[prop(into)] task_progress_label: Option<String>,
    is_zh: bool,
) -> impl IntoView {
    let t = move |en: &'static str, zh: &'static str| crate::t(is_zh, en, zh);
    let code_ref = NodeRef::<leptos::html::Textarea>::new();
    // "Hot reload": the preview used to only ever reflect the last full page load (i.e. the last
    // Save) -- there was no live re-render as you type. `preview_html` starts from the
    // server-rendered `rendered_html` prop and is then updated in place by a debounced call to
    // the already-existing `/projects/:id/render/preview` endpoint (built for exactly this, just
    // never wired up to a live editor before now).
    let preview_html = RwSignal::new(rendered_html);
    // Same "never refreshed after the initial load" gap `document_editor.rs`'s outline had, for
    // the same reason: `headings` used to be a plain prop, read once to build `outline_items`
    // below and never touched again, even though the preview right next to it already hot-
    // reloads live. Wrapped in a signal so the debounced `/render/preview` response (now
    // including recomputed headings) can refresh it the same way.
    let headings_sig = RwSignal::new(headings);
    let debounce_gen = StoredValue::new(0u32);
    let on_body_input = {
        let project_id = project_id.clone();
        let file_path = file_path.clone();
        move |_| {
            debounced_preview(
                code_ref,
                project_id.clone(),
                file_path.clone(),
                debounce_gen,
                preview_html,
                headings_sig,
            )
        }
    };
    // KaTeX's own auto-render only ever scans the DOM once, on its CDN script's `onload` (see
    // `KatexHead` in apich-web) -- fine for the initial server-rendered content, but math typed
    // after that (or arriving via this live preview) was never (re-)rendered, staying visible as
    // raw `$...$` text. Re-invoke it against just the preview pane every time its content changes.
    Effect::new(move |_| {
        preview_html.track();
        rerun_katex_on_preview();
    });

    // Grouped into a foldable tree the same way `document_editor.rs`'s outline is -- see that
    // file's comment for why (subsections were always technically visible, just with no way to
    // collapse them, which looked indistinguishable from them not existing at all).
    let outline_items = move || {
        let headings = headings_sig.get();
        if headings.is_empty() {
            view! { <p class="text-muted" style="font-size:0.8rem;">{t("No headings yet", "暂无标题")}</p> }.into_any()
        } else {
            let min_level = headings.iter().map(|h| h.level).min().unwrap_or(1);
            let mut groups: Vec<(NoteHeadingItem, Vec<NoteHeadingItem>)> = Vec::new();
            for h in headings {
                if h.level <= min_level || groups.is_empty() {
                    groups.push((h, Vec::new()));
                } else {
                    groups.last_mut().unwrap().1.push(h);
                }
            }
            groups
                .into_iter()
                .map(|(parent, children)| {
                    if children.is_empty() {
                        view! {
                            <a href="javascript:void(0)" class="outline-heading-item" data-line=parent.line.to_string()>{parent.text}</a>
                        }.into_any()
                    } else {
                        let child_items: Vec<_> = children
                            .into_iter()
                            .map(|c| {
                                let indent = format!("{}rem", 0.75 * (c.level.saturating_sub(min_level + 1)) as f64);
                                view! {
                                    <a href="javascript:void(0)" class="outline-heading-item outline-heading-h2" style=format!("margin-left:{indent};") data-line=c.line.to_string()>{c.text}</a>
                                }
                            })
                            .collect();
                        view! {
                            <details open=true class="outline-group">
                                <summary class="outline-heading-item" data-line=parent.line.to_string()>{parent.text}</summary>
                                <div class="outline-children">{child_items}</div>
                            </details>
                        }.into_any()
                    }
                })
                .collect::<Vec<_>>()
                .into_any()
        }
    };

    let on_outline_or_preview_click = {
        let project_id = project_id.clone();
        move |ev: leptos::ev::MouseEvent| handle_preview_click(ev, code_ref, project_id.clone())
    };

    // Formatting toolbar: the "somehow more WYSIWYG a bit" half of the note/wiki feedback --
    // clicking a button inserts/toggles real markdown around the current selection (see
    // `note_formatting.rs` for the actual text logic) instead of requiring it to be hand-typed,
    // then re-runs the exact same debounced-preview + syntax-highlight refresh a manual keystroke
    // would have triggered via `on_body_input`, so the live preview and highlight overlay stay in
    // sync with a toolbar-driven edit exactly as they do with a typed one.
    // `StoredValue` rather than a plain closure: this needs to be invoked from 11 independent
    // button handlers below, but it closes over owned `String`s (project_id/file_path), so it
    // isn't `Copy` and a plain `let` binding could only be moved into one of them.
    let toolbar_click = StoredValue::new({
        let project_id = project_id.clone();
        let file_path = file_path.clone();
        move |action: ToolbarAction| {
            apply_toolbar_action(code_ref, action);
            debounced_preview(
                code_ref,
                project_id.clone(),
                file_path.clone(),
                debounce_gen,
                preview_html,
                headings_sig,
            );
        }
    });
    let toolbar = view! {
        <div class="note-toolbar">
            <button type="button" class="btn btn-secondary btn-sm" title=t("Bold", "加粗") on:click=move |_| toolbar_click.with_value(|f| f(ToolbarAction::Bold))><strong>"B"</strong></button>
            <button type="button" class="btn btn-secondary btn-sm" title=t("Italic", "斜体") on:click=move |_| toolbar_click.with_value(|f| f(ToolbarAction::Italic))><em>"I"</em></button>
            <button type="button" class="btn btn-secondary btn-sm" title=t("Inline code", "行内代码") on:click=move |_| toolbar_click.with_value(|f| f(ToolbarAction::Code))>"</>"</button>
            <span class="note-toolbar-sep"></span>
            <button type="button" class="btn btn-secondary btn-sm" title=t("Heading 1", "一级标题") on:click=move |_| toolbar_click.with_value(|f| f(ToolbarAction::Heading(1)))>"H1"</button>
            <button type="button" class="btn btn-secondary btn-sm" title=t("Heading 2", "二级标题") on:click=move |_| toolbar_click.with_value(|f| f(ToolbarAction::Heading(2)))>"H2"</button>
            <button type="button" class="btn btn-secondary btn-sm" title=t("Quote", "引用") on:click=move |_| toolbar_click.with_value(|f| f(ToolbarAction::Quote))>"❝"</button>
            <span class="note-toolbar-sep"></span>
            <button type="button" class="btn btn-secondary btn-sm" title=t("Bullet list", "无序列表") on:click=move |_| toolbar_click.with_value(|f| f(ToolbarAction::BulletList))>"• ⁃"</button>
            <button type="button" class="btn btn-secondary btn-sm" title=t("Numbered list", "有序列表") on:click=move |_| toolbar_click.with_value(|f| f(ToolbarAction::NumberedList))>"1."</button>
            <button type="button" class="btn btn-secondary btn-sm" title=t("Task checkbox", "任务复选框") on:click=move |_| toolbar_click.with_value(|f| f(ToolbarAction::Task))>"☑"</button>
            <span class="note-toolbar-sep"></span>
            <button type="button" class="btn btn-secondary btn-sm" title=t("Link", "链接") on:click=move |_| toolbar_click.with_value(|f| f(ToolbarAction::Link))>"🔗"</button>
            <button type="button" class="btn btn-secondary btn-sm" title=t("Wiki-link to another note", "链接到另一篇笔记") on:click=move |_| toolbar_click.with_value(|f| f(ToolbarAction::WikiLink))>"[[ ]]"</button>
            <button type="button" class="btn btn-secondary btn-sm" title=t("Insert table", "插入表格") on:click=move |_| toolbar_click.with_value(|f| f(ToolbarAction::Table))>"⊞"</button>
        </div>
    };

    let backlink_items = if backlinks.is_empty() {
        view! { <p class="text-muted" style="font-size:0.8rem;">{t("No backlinks yet", "暂无反向链接")}</p> }.into_any()
    } else {
        backlinks
            .into_iter()
            .map(|b| {
                let href = format!("/projects/{}/note?file={}.anote", project_id, urlencoding::encode(&b));
                view! {
                    <a href=href class="wiki-link-pill" style="display:inline-block; margin:0.15rem;">
                        "[[" {b} "]]"
                    </a>
                }
            })
            .collect::<Vec<_>>()
            .into_any()
    };

    view! {
        <div class="outline-panel" on:click={let f = on_outline_or_preview_click.clone(); move |ev| f(ev)}>
            <h4 class="card-subtitle" style="font-size:0.85rem;">{t("Document Outline", "文档大纲")}</h4>
            {outline_items}
            <h4 class="card-subtitle" style="font-size:0.85rem; margin-top:1.25rem;">{t("Backlinks", "反向链接")}</h4>
            {backlink_items}
        </div>
        <div class="code-panel">
            {toolbar}
            <div class="code-editor-wrap" data-lang="markdown">
                <pre class="code-highlight-overlay" aria-hidden="true"><code></code></pre>
                <textarea node_ref=code_ref id="note-body-editor" name="body" class="code-textarea" spellcheck="false" wrap="off" on:input=on_body_input>{body_content}</textarea>
            </div>
            <script>{crate::code_highlight::CODE_HIGHLIGHT_JS}</script>
        </div>
        <div class="preview-panel">
            <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:0.75rem; font-size:0.7rem; color:var(--text-sub);">
                <span>{t("KaTeX Math • WikiLinks • Live Tasks", "KaTeX 数学公式 • 双链引用 • 实时任务")}</span>
                {task_progress_label.map(|l| view! { <span>{l}</span> })}
            </div>
            <div id="note-preview-pane" inner_html=move || preview_html.get() on:click=on_outline_or_preview_click></div>
        </div>
    }
}

#[derive(Debug, Clone, Copy)]
enum ToolbarAction {
    Bold,
    Italic,
    Code,
    Heading(u8),
    Quote,
    BulletList,
    NumberedList,
    Task,
    Link,
    WikiLink,
    Table,
}

/// Thin DOM glue for the formatting toolbar: reads the textarea's current value and selection
/// (JS `selectionStart`/`selectionEnd`, i.e. UTF-16 code unit offsets -- converted to the byte
/// offsets `note_formatting`'s pure functions expect, so this is correct for notes containing CJK
/// text or emoji, not just ASCII), applies the pure transformation, writes the result back, and
/// restores a selection over whatever the user should naturally type over next (the wrapped text,
/// or a placeholder). Also refreshes the Prism highlight overlay immediately, the same as
/// `toggle_task` already does after its own programmatic `set_value` call, since that path
/// doesn't fire a native `input` event on its own.
#[cfg(feature = "hydrate")]
fn apply_toolbar_action(
    code_ref: NodeRef<leptos::html::Textarea>,
    action: ToolbarAction,
) {
    use crate::note_formatting::insert_block;
    use crate::note_formatting::numbered_list;
    use crate::note_formatting::toggle_line_prefix;
    use crate::note_formatting::wrap_selection;
    use crate::note_formatting::TABLE_SNIPPET;

    let Some(ta) = code_ref.get_untracked() else {
        return;
    };
    let value = ta.value();
    let sel_start_u16 = ta.selection_start().ok().flatten().unwrap_or(0) as usize;
    let sel_end_u16 = ta.selection_end().ok().flatten().unwrap_or(0) as usize;
    let start = utf16_offset_to_byte(&value, sel_start_u16);
    let end = utf16_offset_to_byte(&value, sel_end_u16);

    let (new_value, new_start, new_end) = match action {
        | ToolbarAction::Bold => wrap_selection(&value, start, end, "**", "**", "bold text"),
        | ToolbarAction::Italic => wrap_selection(&value, start, end, "*", "*", "italic text"),
        | ToolbarAction::Code => wrap_selection(&value, start, end, "`", "`", "code"),
        | ToolbarAction::Link => wrap_selection(&value, start, end, "[", "](url)", "link text"),
        | ToolbarAction::WikiLink => wrap_selection(&value, start, end, "[[", "]]", "Note Name"),
        | ToolbarAction::Heading(level) => {
            toggle_line_prefix(
                &value,
                start,
                end,
                &format!("{} ", "#".repeat(level as usize)),
            )
        },
        | ToolbarAction::Quote => toggle_line_prefix(&value, start, end, "> "),
        | ToolbarAction::BulletList => toggle_line_prefix(&value, start, end, "- "),
        | ToolbarAction::Task => toggle_line_prefix(&value, start, end, "- [ ] "),
        | ToolbarAction::NumberedList => numbered_list(&value, start, end),
        | ToolbarAction::Table => insert_block(&value, start, end, TABLE_SNIPPET),
    };

    ta.set_value(&new_value);
    let new_start_u16 = byte_offset_to_utf16(&new_value, new_start) as u32;
    let new_end_u16 = byte_offset_to_utf16(&new_value, new_end) as u32;
    let _ = ta.set_selection_range(new_start_u16, new_end_u16);
    let _ = ta.focus();
    call_refresh_highlight("note-body-editor");
}
#[cfg(not(feature = "hydrate"))]
fn apply_toolbar_action(
    _code_ref: NodeRef<leptos::html::Textarea>,
    _action: ToolbarAction,
) {
}

/// A `<textarea>`'s `selectionStart`/`selectionEnd` count UTF-16 code units (real JS string
/// semantics, exposed as-is through web-sys), not the UTF-8 byte offsets Rust string slicing
/// needs -- these two convert at that boundary so a CJK-heavy note's selection maps onto the
/// right bytes instead of silently drifting or panicking on a non-char-boundary slice.
#[cfg(feature = "hydrate")]
fn utf16_offset_to_byte(
    s: &str,
    utf16_offset: usize,
) -> usize {
    let mut units = 0usize;
    for (byte_idx, ch) in s.char_indices() {
        if units >= utf16_offset {
            return byte_idx;
        }
        units += ch.len_utf16();
    }
    s.len()
}
#[cfg(feature = "hydrate")]
fn byte_offset_to_utf16(
    s: &str,
    byte_offset: usize,
) -> usize {
    s.get(..byte_offset)
        .map(|s| s.encode_utf16().count())
        .unwrap_or_else(|| s.encode_utf16().count())
}

/// Handles both the task-checkbox toggle (a real POST that rewrites the physical note file
/// server-side, then refreshes the textarea from the response) and outline/preview click-to-jump
/// -- one real Leptos `on:click` handler bound to both `.outline-panel` and `#note-preview-pane`
/// (see `NoteEditorIsland`'s view), using `Element::closest` the same way the removed JS version
/// did, just as normal Rust control flow instead of a `document`-level delegated listener.
#[cfg(feature = "hydrate")]
fn handle_preview_click(
    ev: leptos::ev::MouseEvent,
    code_ref: NodeRef<leptos::html::Textarea>,
    project_id: String,
) {
    use wasm_bindgen::JsCast;
    let Some(target) = ev.target() else { return };
    let Ok(el) = target.dyn_into::<web_sys::Element>() else {
        return;
    };

    if let Some(checkbox) = el.closest("input.task-live-checkbox").ok().flatten() {
        let Some(row) = checkbox.closest("[data-line][data-file]").ok().flatten() else {
            return;
        };
        let Some(line) = row.get_attribute("data-line") else {
            return;
        };
        let Some(file) = row.get_attribute("data-file") else {
            return;
        };
        let checked = checkbox
            .dyn_ref::<web_sys::HtmlInputElement>()
            .map(|c| c.checked())
            .unwrap_or(false);
        toggle_task(project_id, file, line, checked, code_ref);
        return;
    }

    if let Some(with_line) = el.closest("[data-line]").ok().flatten() {
        if let Some(line_str) = with_line.get_attribute("data-line") {
            if let Ok(line) = line_str.parse::<u32>() {
                crate::jump_to_line::jump_to_line_in_dom("note-body-editor", line);
            }
        }
    }
}
#[cfg(not(feature = "hydrate"))]
fn handle_preview_click(
    _ev: leptos::ev::MouseEvent,
    _code_ref: NodeRef<leptos::html::Textarea>,
    _project_id: String,
) {
}

#[cfg(feature = "hydrate")]
fn toggle_task(
    project_id: String,
    file: String,
    line: String,
    checked: bool,
    code_ref: NodeRef<leptos::html::Textarea>,
) {
    wasm_bindgen_futures::spawn_local(async move {
        let status = if checked { "done" } else { "todo" };
        let body = format!(
            "file={}&line_number={}&status={}",
            urlencode(&file),
            urlencode(&line),
            status
        );
        let result = gloo_net::http::Request::post(&format!(
            "/projects/{}/knowledge/toggle-task-ajax",
            project_id
        ))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .expect("valid form body")
        .send()
        .await;
        if let Ok(resp) = result {
            if let Ok(data) = resp.json::<serde_json::Value>().await {
                if let Some(updated) = data.get("updated_body").and_then(|v| v.as_str()) {
                    if let Some(ta) = code_ref.get_untracked() {
                        let scroll = ta.scroll_top();
                        ta.set_value(updated);
                        ta.set_scroll_top(scroll);
                        // `set_value` doesn't fire a native `input` event, so the syntax-highlight
                        // overlay (`code_highlight.rs`, driven by Prism.js -- no realistic
                        // Rust/WASM equivalent) would otherwise keep showing stale content until
                        // the next keystroke. Calling its exposed refresh function directly (real
                        // FFI from Rust, same pattern `rerun_katex_on_preview` already uses for
                        // KaTeX) avoids dispatching a real `input` event, which would *also* fire
                        // this same textarea's own `on:input` handler for the live-preview
                        // debounce -- confirmed live, that caused an unwanted extra remote preview
                        // re-render + DOM replacement after every checkbox click, destroying the
                        // very checkbox the user had just clicked.
                        call_refresh_highlight("note-body-editor");
                    }
                }
            }
        }
    });
}
#[cfg(not(feature = "hydrate"))]
fn toggle_task(
    _project_id: String,
    _file: String,
    _line: String,
    _checked: bool,
    _code_ref: NodeRef<leptos::html::Textarea>,
) {
}

#[cfg(feature = "hydrate")]
fn call_refresh_highlight(textarea_id: &str) {
    use wasm_bindgen::JsCast;
    if let Some(window) = web_sys::window() {
        if let Ok(func) = js_sys::Reflect::get(
            &window,
            &wasm_bindgen::JsValue::from_str("__apichRefreshHighlight"),
        ) {
            if let Ok(func) = func.dyn_into::<js_sys::Function>() {
                let _ = func.call1(
                    &wasm_bindgen::JsValue::NULL,
                    &wasm_bindgen::JsValue::from_str(textarea_id),
                );
            }
        }
    }
}

#[cfg(feature = "hydrate")]
fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            | b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            },
            | _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

/// Debounced live preview: waits 500ms after the last keystroke, then -- only if no newer
/// keystroke arrived in the meantime (checked via a generation counter, the standard
/// cancel-the-stale-one debounce pattern) -- POSTs the current textarea content to the
/// already-existing `/projects/:id/render/preview` endpoint and swaps the rendered HTML in.
#[cfg(feature = "hydrate")]
fn debounced_preview(
    code_ref: NodeRef<leptos::html::Textarea>,
    project_id: String,
    file_path: String,
    debounce_gen: StoredValue<u32>,
    preview_html: RwSignal<String>,
    headings: RwSignal<Vec<NoteHeadingItem>>,
) {
    let my_gen = debounce_gen.get_value().wrapping_add(1);
    debounce_gen.set_value(my_gen);
    wasm_bindgen_futures::spawn_local(async move {
        gloo_timers::future::TimeoutFuture::new(500).await;
        if debounce_gen.get_value() != my_gen {
            return; // a newer keystroke superseded this one
        }
        let Some(ta) = code_ref.get_untracked() else {
            return;
        };
        let content = ta.value();
        let body = serde_json::json!({ "file": file_path, "content": content });
        let result =
            gloo_net::http::Request::post(&format!("/projects/{}/render/preview", project_id))
                .json(&body)
                .expect("valid json body")
                .send()
                .await;
        if let Ok(resp) = result {
            if let Ok(data) = resp.json::<serde_json::Value>().await {
                if let Some(html) = data.get("html").and_then(|v| v.as_str()) {
                    preview_html.set(html.to_string());
                }
                // Outline used to be a plain prop, read once and never refreshed -- see
                // `NoteEditorIsland`'s own `headings_sig` comment. Recomputed here on every
                // debounced call, alongside the preview HTML it already refreshes.
                if let Some(arr) = data.get("headings").and_then(|v| v.as_array()) {
                    let new_headings: Vec<NoteHeadingItem> = arr
                        .iter()
                        .filter_map(|h| {
                            Some(NoteHeadingItem {
                                level: h.get("level")?.as_u64()? as u8,
                                text: h.get("text")?.as_str()?.to_string(),
                                line: h.get("line")?.as_u64()? as u32,
                            })
                        })
                        .collect();
                    headings.set(new_headings);
                }
            }
        }
    });
}
#[cfg(not(feature = "hydrate"))]
fn debounced_preview(
    _code_ref: NodeRef<leptos::html::Textarea>,
    _project_id: String,
    _file_path: String,
    _debounce_gen: StoredValue<u32>,
    _preview_html: RwSignal<String>,
    _headings: RwSignal<Vec<NoteHeadingItem>>,
) {
}

/// Re-invokes KaTeX's own globally-exposed `renderMathInElement` (from the `auto-render` CDN
/// script `KatexHead` loads) against just the preview pane, not the whole document -- its own
/// `onload` handler only ever scans the DOM once, so math arriving later (typed, or swapped in by
/// the live preview above) is otherwise left as raw, unrendered `$...$` text. Defensive: the
/// function may not be defined yet if this runs before that CDN script's own `onload` fires, so
/// this checks for it rather than assuming.
#[cfg(feature = "hydrate")]
fn rerun_katex_on_preview() {
    use wasm_bindgen::JsCast;
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let Some(el) = document.get_element_by_id("note-preview-pane") else {
        return;
    };
    let Ok(func) = js_sys::Reflect::get(
        &window,
        &wasm_bindgen::JsValue::from_str("renderMathInElement"),
    ) else {
        return;
    };
    let Ok(func) = func.dyn_into::<js_sys::Function>() else {
        return;
    };
    let options = js_sys::Object::new();
    let delimiters = js_sys::Array::new();
    let mk = |left: &str, right: &str, display: bool| {
        let d = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&d, &"left".into(), &left.into());
        let _ = js_sys::Reflect::set(&d, &"right".into(), &right.into());
        let _ = js_sys::Reflect::set(
            &d,
            &"display".into(),
            &wasm_bindgen::JsValue::from_bool(display),
        );
        d
    };
    delimiters.push(&mk("$$", "$$", true));
    delimiters.push(&mk("$", "$", false));
    let _ = js_sys::Reflect::set(&options, &"delimiters".into(), &delimiters);
    let _ = func.call2(&wasm_bindgen::JsValue::NULL, &el, &options);
}
#[cfg(not(feature = "hydrate"))]
fn rerun_katex_on_preview() {}
