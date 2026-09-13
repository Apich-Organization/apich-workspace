//! Real Rust replacement for the document/slide/script editor's hand-written JS (`document_editor_page.rs`'s `build_editor_script`).
//!
//! Outline jump-to-heading, Ctrl+S save,
//! Typst/slide multi-page preview with presentation mode and click-to-jump reverse search, and
//! the script-runner console, are all implemented here. These stay coupled in one island because
//! they were coupled in the original design too -- keyboard shortcuts and slide state are
//! shared across the outline/code/preview panels, not independent widgets.

use leptos::prelude::*;
use serde::Deserialize;
use serde::Serialize;

/// Heading item parsed from document content for table of contents navigation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeadingItem {
    /// Heading level (1 for H1, 2 for H2, etc.).
    pub level: u8,
    /// Text content of the heading.
    pub text: String,
}

/// Preview mode kind for the document editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DocumentPreviewKind {
    /// Plain editor without specialized preview.
    #[default]
    None,
    /// Presentation / slide deck mode.
    Slide,
    /// Typst live preview mode.
    Typst,
    /// LaTeX live preview mode.
    Latex,
}

/// Feature flags for document editor modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DocEditorFlags {
    /// Whether script runner console is enabled.
    pub is_script: bool,
    /// The document preview mode kind.
    pub preview_kind: DocumentPreviewKind,
}

impl DocEditorFlags {
    /// Create document editor flags from preview kind and script status.
    #[must_use]
    pub const fn new(
        preview_kind: DocumentPreviewKind,
        is_script: bool,
    ) -> Self {
        Self {
            is_script,
            preview_kind,
        }
    }

    /// Whether presentation / slide deck mode is enabled.
    #[must_use]
    pub const fn is_slide(&self) -> bool {
        matches!(self.preview_kind, DocumentPreviewKind::Slide)
    }

    /// Whether Typst live preview is enabled.
    #[must_use]
    pub const fn is_typst_preview(&self) -> bool {
        matches!(self.preview_kind, DocumentPreviewKind::Typst)
    }

    /// Whether LaTeX live preview is enabled.
    #[must_use]
    pub const fn is_latex_preview(&self) -> bool {
        matches!(self.preview_kind, DocumentPreviewKind::Latex)
    }
}

#[island]
#[allow(clippy::too_many_arguments)]
pub fn DocumentEditorIsland(
    #[prop(into)] project_id: String,
    #[prop(into)] file_path: String,
    #[prop(into)] content: String,
    headings: Vec<HeadingItem>,
    flags: DocEditorFlags,
    typst_pages: Vec<String>,
    #[prop(into)] compile_error: Option<String>,
    #[prop(into)] rendered_markdown_html: Option<String>,
) -> impl IntoView {
    let is_slide = flags.is_slide();
    let is_script = flags.is_script;
    let is_typst_preview = flags.is_typst_preview();
    let is_latex_preview = flags.is_latex_preview();
    let prism_lang = {
        let ext = std::path::Path::new(&file_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        crate::code_highlight::prism_lang_for_ext(&ext).to_string()
    };
    let code_ref = NodeRef::<leptos::html::Textarea>::new();
    let form_ref = NodeRef::<leptos::html::Form>::new();
    let current_slide = RwSignal::new(1usize);
    let presenting = RwSignal::new(false);
    let pages = RwSignal::new(typst_pages);
    // "Hot loading" for Typst/slide documents: previously the preview only ever showed the
    // result of the last Save (a full page reload); nothing recompiled as you typed. `pages` and
    // `compile_error_sig` now update live via a debounced call to the already-existing
    // `/projects/:id/render/preview` endpoint (same one wired up for the markdown/note editor).
    let compile_error_sig = RwSignal::new(compile_error);
    // The outline panel used to be built once from this prop and never touched again -- every
    // other part of the hot-reload response (pages/PDF) already updates live as the user types,
    // but the outline silently didn't, leaving it showing stale (or missing) headings until the
    // next full page reload. Wrapped in a signal so `render_doc_preview_action`'s response
    // (now including recomputed headings, see its own comment) can refresh it the same way.
    let headings_sig = RwSignal::new(headings);
    let debounce_gen = StoredValue::new(0u32);
    // Bumped after each successful debounced LaTeX recompile, appended to the PDF iframe's `src`
    // as a cache-busting query param -- browsers cache an `<iframe>` by URL, so just re-fetching
    // the same URL after a save wouldn't show the new content without this.
    let latex_reload_gen = RwSignal::new(0u32);
    let latex_error = RwSignal::new(None::<String>);
    // Same "never had hot reload at all" gap as the outline, for the same reason: plain markdown
    // files opened through this island (not `.anote` notes, which go through `NoteEditorIsland`
    // instead) had no debounced call wired to `on_code_input` at all -- the preview only ever
    // showed the last Save. Wrapped in a signal for the same debounced-`/render/preview` pattern
    // Typst/LaTeX/notes already use.
    let markdown_html_sig = RwSignal::new(rendered_markdown_html.unwrap_or_default());
    // Which TeX engine to compile with. Only engines actually installed in the sandbox image are
    // offered (`texlive-xetex` + `texlive-luatex`, see `docker/Containerfile.sandbox`), rather
    // than offering an option that would just fail.
    let latex_engine = RwSignal::new("pdflatex".to_string());

    let jump_to_heading = move |title: String| {
        if let Some(ta) = code_ref.get_untracked() {
            select_substring(&ta, &title);
        }
    };

    // Grouped into a real foldable tree (native <details>/<summary> -- no JS needed for the
    // fold/unfold interaction itself, so it works immediately, no hydration wait) instead of a
    // permanently-flat list: previously every subsection was *technically* always visible
    // (small, indented, no bug in the extraction or the CSS), but with no way to collapse a
    // section's children there was also no way to *reveal* them on demand -- indistinguishable,
    // at a glance, from subsections not existing at all. A "parent" is any heading at the
    // document's own minimum level (level 1 for Typst's `=`/LaTeX's `\part`, but level 2 when a
    // LaTeX file only uses `\section` with no `\part`/`\chapter` -- using the actual minimum
    // rather than a hardcoded `1` keeps both cases grouping correctly); everything after it up to
    // the next same-or-shallower heading is nested as its child, open (expanded) by default so
    // this doesn't change what's visible on first load, only what's possible to fold shut.
    let outline_items = move || {
        let headings = headings_sig.get();
        if headings.is_empty() {
            let hint = if is_script {
                "Script Runner console active. Click \"Run Script\" to execute."
            } else {
                "No section headings detected"
            };
            view! { <div style="font-size:0.8rem; color:var(--text-sub); font-style:italic;">{hint}</div> }.into_any()
        } else {
            let min_level = headings.iter().map(|h| h.level).min().unwrap_or(1);
            let mut groups: Vec<(HeadingItem, Vec<HeadingItem>)> = Vec::new();
            for h in headings {
                if h.level <= min_level || groups.is_empty() {
                    groups.push((h, Vec::new()));
                } else if let Some(last) = groups.last_mut() {
                    last.1.push(h);
                }
            }
            groups
                .into_iter()
                .map(|(parent, children)| {
                    let parent_text = parent.text.clone();
                    let jth = jump_to_heading;
                    if children.is_empty() {
                        view! {
                            <a class="outline-heading-item outline-heading-h1" on:click=move |_| jth(parent_text.clone())>{parent.text}</a>
                        }.into_any()
                    } else {
                        let child_items: Vec<_> = children
                            .into_iter()
                            .map(|c| {
                                let class = if c.level <= min_level.saturating_add(1) { "outline-heading-item outline-heading-h2" } else { "outline-heading-item outline-heading-h3" };
                                let text = c.text.clone();
                                let jth = jump_to_heading;
                                view! {
                                    <a class=class on:click=move |_| jth(text.clone())>{c.text}</a>
                                }
                            })
                            .collect();
                        view! {
                            <details open=true class="outline-group">
                                <summary class="outline-heading-item outline-heading-h1" on:click=move |_| jth(parent_text.clone())>{parent.text}</summary>
                                <div class="outline-children">{child_items}</div>
                            </details>
                        }.into_any()
                    }
                })
                .collect::<Vec<_>>()
                .into_any()
        }
    };

    wire_keyboard_shortcuts(
        form_ref,
        current_slide,
        presenting,
        pages,
        is_slide,
        is_script,
    );
    // Only the LaTeX PDF preview needs this: PDF.js (an external JS library with no Rust/WASM
    // equivalent, see `LATEX_PDF_VIEWER_JS`'s doc comment) can't call a Rust closure directly, so
    // it dispatches a `CustomEvent` that this listener receives and forwards into the same
    // `jump_to_line_in_dom` the Typst/markdown click handlers below call directly. Those don't
    // need this bridge at all -- they're real `on:click` handlers on real Leptos-rendered
    // elements.
    crate::jump_to_line::wire_jump_to_line_listener("code-editor-input");

    let on_code_input = {
        let project_id = project_id.clone();
        let file_path = file_path.clone();
        move |_| {
            if is_typst_preview || is_slide {
                debounced_typst_preview(TypstPreviewArgs {
                    code_ref,
                    project_id: project_id.clone(),
                    file_path: file_path.clone(),
                    debounce_gen,
                    pages,
                    compile_error: compile_error_sig,
                    current_slide,
                    headings: headings_sig,
                });
            } else if is_latex_preview {
                debounced_latex_preview(LatexPreviewArgs {
                    code_ref,
                    project_id: project_id.clone(),
                    file_path: file_path.clone(),
                    debounce_gen,
                    latex_reload_gen,
                    latex_error,
                    latex_engine,
                    headings: headings_sig,
                });
            } else if !is_script {
                debounced_markdown_preview(
                    code_ref,
                    project_id.clone(),
                    file_path.clone(),
                    debounce_gen,
                    markdown_html_sig,
                    headings_sig,
                );
            }
        }
    };

    let preview = if is_script {
        render_script_console(project_id.clone(), file_path.clone()).into_any()
    } else if is_typst_preview || is_slide {
        // Cargo-slide decks compile through the exact same Typst pipeline as a plain `.typ` file
        // (see `render_doc_preview_action`, which branches on file extension, not on this flag),
        // so they need the same SVG-pages preview -- previously this branch only checked
        // `is_typst_preview` (true only for `DocumentPreviewKind::Typst`), which is mutually
        // exclusive with `DocumentPreviewKind::Slide` (see `DocEditorFlags`). Every slide file
        // fell through to the plain-markdown branch below instead, rendering an empty
        // `markdown_html_sig` (never populated for slides) -- the preview panel showed nothing at
        // all, and typing never recompiled anything (see `on_code_input`'s matching fix).
        render_typst_preview(compile_error_sig, pages, current_slide).into_any()
    } else if is_latex_preview {
        // Real `pdflatex` compilation happens inside the project's sandbox container (it has a
        // full TeX Live install; this dev host deliberately doesn't -- see
        // `ProjectManagerService::compile_latex_in_sandbox`'s doc comment). Hot reload: a
        // debounced call to `/render/preview` (which saves + recompiles) bumps `latex_reload_gen`
        // on success, which changes the iframe's `src` (a plain cache-busting query param) so the
        // browser actually re-fetches instead of showing its cached copy of the old PDF.
        let project_id_for_url = project_id.clone();
        let file_path_for_url = file_path.clone();
        let project_id_for_dl = project_id.clone();
        let file_path_for_dl = file_path.clone();
        let project_id_for_engine = project_id.clone();
        let file_path_for_engine = file_path.clone();
        let file_path_for_sync = file_path.clone();
        let download_name = std::path::Path::new(&file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("document")
            .to_string();
        view! {
            <div style="display:flex; flex-direction:column; height:100%;">
                <div style="display:flex; align-items:center; justify-content:space-between; gap:0.75rem; padding:0.4rem 0.75rem; background:var(--bg-muted); border-bottom:1px solid var(--border-subtle); font-size:0.8rem;">
                    <div style="display:flex; align-items:center; gap:0.4rem;">
                        <label for="latex-engine-select" style="color:var(--text-sub);">"Engine:"</label>
                        <select
                            id="latex-engine-select"
                            class="form-control"
                            style="width:auto; padding:0.15rem 0.4rem; font-size:0.8rem;"
                            prop:value=move || latex_engine.get()
                            on:change=move |ev| {
                                latex_engine.set(event_target_value(&ev));
                                let code_ref = code_ref;
                                let project_id = project_id_for_engine.clone();
                                let file_path = file_path_for_engine.clone();
                                debounced_latex_preview(LatexPreviewArgs {
                                    code_ref,
                                    project_id,
                                    file_path,
                                    debounce_gen,
                                    latex_reload_gen,
                                    latex_error,
                                    latex_engine,
                                    headings: headings_sig,
                                });
                            }
                        >
                            <option value="pdflatex">"pdflatex"</option>
                            <option value="xelatex">"xelatex"</option>
                            <option value="lualatex">"lualatex"</option>
                        </select>
                    </div>
                    <a
                        href=move || format!("/projects/{}/editor/latex-pdf?file={}&engine={}", project_id_for_dl, urlencoding::encode(&file_path_for_dl), latex_engine.get())
                        download=format!("{}.pdf", download_name)
                        class="btn btn-secondary btn-sm"
                    >"⬇️ Download PDF"</a>
                </div>
                <div style="position:relative; width:100%; flex:1;">
                    {move || latex_error.get().map(|err| view! {
                        <div style="position:absolute; inset:0; background:#1e1e1e; color:#f87171; font-family:var(--font-mono); font-size:0.8rem; white-space:pre-wrap; padding:1.25rem; overflow-y:auto; z-index:5;">
                            "⚠️ LaTeX compilation failed:\n\n" {err}
                        </div>
                    })}
                    // Plain `<canvas>` elements rendered by PDF.js, not an `<iframe>`: a browser's
                    // native in-iframe PDF viewer never exposes click coordinates to the parent
                    // page (no API for it, by design), so click-to-jump-to-source is simply not
                    // reachable through an iframe at all -- confirmed while building this, not a
                    // theoretical concern. Rendering onto our own canvas is what makes
                    // `LATEX_PDF_VIEWER_JS`'s click handler (which converts the click into a real
                    // `synctex edit` query, same mechanism TeXstudio/Overleaf use) possible.
                    <div
                        id="latex-pdf-viewer"
                        class="latex-pdf-viewer"
                        data-pdf-url=move || format!("/projects/{}/editor/latex-pdf?file={}&engine={}&_r={}", project_id_for_url, urlencoding::encode(&file_path_for_url), latex_engine.get(), latex_reload_gen.get())
                        data-sync-url=format!("/projects/{}/editor/latex-sync", project_id)
                        data-file=file_path_for_sync
                        data-engine=move || latex_engine.get()
                        style="width:100%; height:100%; overflow:auto; background:#525659; display:flex; flex-direction:column; align-items:center; gap:12px; padding:12px 0;"
                    ></div>
                </div>
            </div>
            <script>{LATEX_PDF_VIEWER_JS}</script>
        }.into_any()
    } else {
        view! {
            <div
                style="background:var(--bg-surface); border:1px solid var(--border-subtle); border-radius:8px; padding:2rem; max-width:760px; margin:0 auto; line-height:1.7; height:100%; overflow-y:auto;"
                inner_html=move || markdown_html_sig.get()
                on:click=move |ev: leptos::ev::MouseEvent| handle_data_line_click(ev)
            ></div>
        }.into_any()
    };

    // Real browser fullscreen for the presentation overlay. The overlay alone only covers the
    // page -- browser chrome, tabs, and the OS bar all stay visible, which is not what anyone
    // means by "present". F11 can't fill the gap either: this island's own keydown handler binds
    // F11 to *opening* the overlay and calls `prevent_default()`, so the browser's native
    // fullscreen never fires. Hence an explicit control, driving the Fullscreen API directly.
    let stage_ref = NodeRef::<leptos::html::Div>::new();
    let is_fullscreen = RwSignal::new(false);
    wire_fullscreen_listener(is_fullscreen);

    let presentation_modal = is_slide.then(|| {
        view! {
            <div
                node_ref=stage_ref
                style:display=move || if presenting.get() { "flex" } else { "none" }
                style="position:fixed; inset:0; background:#0f172a; z-index:9999; flex-direction:column; justify-content:center; align-items:center; padding:2rem;"
            >
                <div style="position:absolute; top:1.5rem; right:2rem; display:flex; align-items:center; gap:0.5rem;">
                    <button
                        type="button"
                        class="btn btn-secondary btn-sm"
                        title="Fill the whole screen (Esc or this button to exit)"
                        on:click=move |_| toggle_fullscreen(stage_ref, is_fullscreen)
                    >
                        {move || if is_fullscreen.get() { "⤢ Exit full screen" } else { "⛶ Full screen" }}
                    </button>
                    <button type="button" on:click=move |_| { exit_fullscreen_if_active(is_fullscreen); presenting.set(false); } style="background:none; border:none; color:#fff; font-size:1.75rem; cursor:pointer; line-height:1;" title="Close presentation">"×"</button>
                </div>
                <div style="background:#ffffff; width:90%; max-width:1100px; aspect-ratio:16/9; border-radius:16px; padding:2.5rem; display:flex; flex-direction:column; justify-content:center; overflow:hidden;">
                    <div
                        style="width:100%; height:100%; display:flex; justify-content:center; align-items:center;"
                        inner_html=move || pages.with(|p| p.get(current_slide.get().saturating_sub(1)).cloned().unwrap_or_default())
                    ></div>
                </div>
                <div style="display:flex; gap:1rem; align-items:center; margin-top:1.5rem; color:#94a3b8; font-size:0.9rem;">
                    <button type="button" class="btn btn-secondary btn-sm" on:click=move |_| current_slide.update(|s| if *s > 1 { *s = s.saturating_sub(1); })>"← Prev"</button>
                    <span>{move || format!("Slide {} of {}", current_slide.get(), pages.with(|p| p.len().max(1)))}</span>
                    <button type="button" class="btn btn-secondary btn-sm" on:click=move |_| { let n = pages.with(|p| p.len().max(1)); current_slide.update(|s| if *s < n { *s = s.saturating_add(1); }) }>"Next →"</button>
                </div>
            </div>
        }
    });

    view! {
        <div class="editor-studio-grid">
            <div class="outline-panel">
                <div style="font-size:0.75rem; font-weight:700; color:var(--text-sub); text-transform:uppercase; letter-spacing:0.5px; margin-bottom:0.75rem;">
                    "Document Outline"
                </div>
                {outline_items}
            </div>

            <div class="code-panel">
                <form node_ref=form_ref id="editor-form" method="post" action=format!("/projects/{}/editor/save", project_id) style="display:flex; flex-direction:column; height:100%;">
                    <input type="hidden" name="file" value=file_path />
                    <div class="code-editor-wrap" data-lang=prism_lang>
                        <pre class="code-highlight-overlay" aria-hidden="true"><code></code></pre>
                        <textarea node_ref=code_ref id="code-editor-input" name="content" class="code-textarea" spellcheck="false" wrap="off" on:input=on_code_input>{content}</textarea>
                    </div>
                    <script>{crate::code_highlight::CODE_HIGHLIGHT_JS}</script>
                    <div style="padding:0.4rem 1rem; background:var(--bg-muted); border-top:1px solid var(--border-subtle); display:flex; justify-content:space-between; font-size:0.75rem; color:var(--text-sub);">
                        <span>
                            "Press Ctrl+S to save"
                            {is_script.then_some(" • Ctrl+Enter to run script")}
                            {(!is_script && !is_typst_preview && !is_latex_preview).then_some(" • Math: $inline$ or $$block$$")}
                        </span>
                        <span>"UTF-8"</span>
                    </div>
                </form>
            </div>

            <div class="preview-panel">{preview}</div>
        </div>

        {is_slide.then(|| view! {
            <button type="button" style="display:none;" id="editor-present-trigger" on:click=move |_| { presenting.set(true); current_slide.set(1); }></button>
        })}
        {presentation_modal}
    }
}

/// Puts the presentation stage into (or out of) real browser fullscreen.
///
/// `Element::request_fullscreen()` returns a `Promise` that rejects if the call didn't originate
/// in a user gesture; it's called straight from the click handler, so that holds. The result is
/// ignored deliberately -- a rejection just means the browser declined, and the overlay is still
/// perfectly usable at page size.
#[cfg(feature = "hydrate")]
fn toggle_fullscreen(
    stage_ref: NodeRef<leptos::html::Div>,
    is_fullscreen: RwSignal<bool>,
) {
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    if doc.fullscreen_element().is_some() {
        doc.exit_fullscreen();
        is_fullscreen.set(false);
        return;
    }
    if let Some(el) = stage_ref.get_untracked() {
        let _ = el.request_fullscreen();
        is_fullscreen.set(true);
    }
}
#[cfg(not(feature = "hydrate"))]
const fn toggle_fullscreen(
    _stage_ref: NodeRef<leptos::html::Div>,
    _is_fullscreen: RwSignal<bool>,
) {
}

/// Leaves fullscreen when the presentation is closed, so dismissing the overlay never strands
/// the browser in a fullscreen state with nothing presenting in it.
#[cfg(feature = "hydrate")]
fn exit_fullscreen_if_active(is_fullscreen: RwSignal<bool>) {
    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
        if doc.fullscreen_element().is_some() {
            doc.exit_fullscreen();
        }
    }
    is_fullscreen.set(false);
}
#[cfg(not(feature = "hydrate"))]
const fn exit_fullscreen_if_active(_is_fullscreen: RwSignal<bool>) {}

/// Keeps the button's label honest when fullscreen is left by a route this component didn't
/// drive -- pressing Esc, or the browser's own exit affordance -- which fires
/// `fullscreenchange` without ever going through `toggle_fullscreen`.
#[cfg(feature = "hydrate")]
fn wire_fullscreen_listener(is_fullscreen: RwSignal<bool>) {
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;

    let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    let doc_for_cb = doc.clone();
    let closure = Closure::<dyn Fn()>::new(move || {
        is_fullscreen.set(doc_for_cb.fullscreen_element().is_some());
    });
    let _ = doc
        .add_event_listener_with_callback("fullscreenchange", closure.as_ref().unchecked_ref());
    closure.forget();
}
#[cfg(not(feature = "hydrate"))]
const fn wire_fullscreen_listener(_is_fullscreen: RwSignal<bool>) {}

/// Click-to-jump for the Typst/slide SVG preview (`<a href="sync:line:N">`, embedded by
/// `DocumentRenderer::annotate_typst_lines_for_reverse_search`) and the plain markdown preview
/// (`[data-line]` attributes): real Leptos `on:click` handlers, calling
/// `jump_to_line.rs`'s `jump_to_line_in_dom` directly -- no plain JS, no WASM round-trip via a
/// `CustomEvent`, just a normal click handler. Unlike the AI Copilot drawer's toggle button (see
/// `ai_drawer.rs`), which is visible and clickable the instant the page renders and so can't
/// tolerate any hydration delay at all, neither of these is reachable before hydration has
/// reliably already finished: a user can only get here by first waiting for a compiled preview (a
/// network round-trip) or a rendered note to appear. Live-tested to confirm: 6/6 clicks landed
/// correctly even with a fresh WASM download and no artificial wait beyond the link existing in
/// the DOM.
#[cfg(feature = "hydrate")]
fn handle_reverse_search_click(ev: leptos::ev::MouseEvent) {
    use wasm_bindgen::JsCast;
    let Some(target) = ev.target() else { return };
    let Ok(el) = target.dyn_into::<web_sys::Element>() else {
        return;
    };

    if let Some(link) = el.closest("a").ok().flatten() {
        let href = link
            .get_attribute("href")
            .or_else(|| link.get_attribute("xlink:href"));
        if let Some(line_str) = href.as_deref().and_then(|h| h.strip_prefix("sync:line:")) {
            if let Ok(line) = line_str.parse::<u32>() {
                ev.prevent_default();
                crate::jump_to_line::jump_to_line_in_dom("code-editor-input", line);
                return;
            }
        }
    }
    if let Some(with_line) = el.closest("[data-line]").ok().flatten() {
        if let Some(line_str) = with_line.get_attribute("data-line") {
            if let Ok(line) = line_str.parse::<u32>() {
                crate::jump_to_line::jump_to_line_in_dom("code-editor-input", line);
                return;
            }
        }
    }
    // Fallback for a slide with no line-level links at all (a title slide made entirely of named
    // arguments, no body markup to wrap -- see `annotate_typst_lines_for_reverse_search`'s doc
    // comment): jump to wherever that slide's own `#slide(`/`#title-slide(` call starts in the
    // source, so clicking anywhere on it still does something useful instead of silently nothing.
    if let Some(svg) = el.closest("svg[data-fallback-line]").ok().flatten() {
        if let Some(line_str) = svg.get_attribute("data-fallback-line") {
            if let Ok(line) = line_str.parse::<u32>() {
                crate::jump_to_line::jump_to_line_in_dom("code-editor-input", line);
            }
        }
    }
}
#[cfg(not(feature = "hydrate"))]
fn handle_reverse_search_click(_ev: leptos::ev::MouseEvent) {}

/// Click-to-jump for plain markdown previews (paragraphs/headings carry a real `data-line`
/// attribute from `DocumentRenderer`, not an `onclick="jumpToLine(...)"` string) -- same idea as
/// `handle_reverse_search_click`, different source markup (no `sync:line:` links here).
#[cfg(feature = "hydrate")]
fn handle_data_line_click(ev: leptos::ev::MouseEvent) {
    use wasm_bindgen::JsCast;
    let Some(target) = ev.target() else { return };
    let Ok(el) = target.dyn_into::<web_sys::Element>() else {
        return;
    };
    let Some(with_line) = el.closest("[data-line]").ok().flatten() else {
        return;
    };
    let Some(line_str) = with_line.get_attribute("data-line") else {
        return;
    };
    let Ok(line) = line_str.parse::<u32>() else {
        return;
    };
    crate::jump_to_line::jump_to_line_in_dom("code-editor-input", line);
}
#[cfg(not(feature = "hydrate"))]
fn handle_data_line_click(_ev: leptos::ev::MouseEvent) {}

/// PDF.js can't be driven from Rust/WASM the way the rest of this island is (it's a large,
/// well-established JS library -- reimplementing PDF parsing/rendering in Rust would be its own
/// multi-month project, not a reasonable ask for one feature), so the actual canvas rendering and
/// click-position detection for the LaTeX preview lives in this plain JS, loaded from the CDN
/// allowlist the same way `KatexHead` already loads `KaTeX`. Once a click is resolved to a real
/// source line (via a `synctex edit` round-trip), the click handler hands off to Rust immediately
/// by dispatching an `apich-jump-to-line` `CustomEvent` -- `jump_to_line.rs`'s
/// `wire_jump_to_line_listener` receives it and calls the exact same `jump_to_line_in_dom` the
/// Typst/markdown click handlers call directly, so there's exactly one real implementation of
/// "select this line in the textarea," not a JS copy and a Rust copy of the same logic.
///
/// Coordinate math: `synctex edit`'s `-o page:x:y:file` spec expects `x`/`y` in PDF points with
/// the origin at the page's TOP-LEFT, y increasing downward -- confirmed empirically against a
/// real compiled document (`synctex view` on a line near the top of the page returned y≈125-200;
/// the same page's later content returned y≈393, i.e. y grows going *down* the page). PDF.js's own
/// `viewport.convertToPdfPoint()` returns the PDF's native coordinates instead (origin
/// bottom-left, y increasing upward, standard since PDF's page-description coordinate system
/// comes straight from PostScript) -- so every click converts as
/// `synctexY = pageHeightPt - pdfY`, not passed through directly. Getting this backwards would
/// silently jump to the mirror-image line on every click, which would have been very easy to ship
/// un-noticed without the live round-trip test that caught it.
const LATEX_PDF_VIEWER_JS: &str = r"
(function(){
    function ensurePdfJs(){
        if (window.__apichPdfJsReady) { return window.__apichPdfJsReady; }
        window.__apichPdfJsReady = new Promise(function(resolve, reject){
            if (window.pdfjsLib) { resolve(window.pdfjsLib); return; }
            var s = document.createElement('script');
            // Pinned to the last pdf.js release still shipping a classic UMD build (a plain
            // global-exposing script, loadable via a normal <script src>) -- 4.x+ ships ESM-only
            // (`pdf.min.mjs`), which a plain `document.createElement('script')` injection can't
            // consume the way this loader works. Confirmed live: the 4.0.379 `pdf.min.js` path
            // 404s (blocked by the browser's Opaque Response Blocking before this was caught).
            s.src = 'https://cdnjs.cloudflare.com/ajax/libs/pdf.js/3.11.174/pdf.min.js';
            s.onload = function(){
                window.pdfjsLib.GlobalWorkerOptions.workerSrc = 'https://cdnjs.cloudflare.com/ajax/libs/pdf.js/3.11.174/pdf.worker.min.js';
                resolve(window.pdfjsLib);
            };
            s.onerror = function(){ reject(new Error('failed to load pdf.js')); };
            document.head.appendChild(s);
        });
        return window.__apichPdfJsReady;
    }

    function showMessage(container, text){
        container.innerHTML = '';
        var d = document.createElement('div');
        d.style.cssText = 'color:#e2e8f0; font-family:monospace; font-size:0.8rem; padding:1.5rem; white-space:pre-wrap;';
        d.textContent = text;
        container.appendChild(d);
    }

    function onPageClick(container, pageNum, viewport, pageHeightPt, canvasEl, ev){
        var rect = canvasEl.getBoundingClientRect();
        var scaleX = canvasEl.width / rect.width;
        var scaleY = canvasEl.height / rect.height;
        var cx = (ev.clientX - rect.left) * scaleX;
        var cy = (ev.clientY - rect.top) * scaleY;
        var pt = viewport.convertToPdfPoint(cx, cy);
        var synctexX = pt[0];
        var synctexY = pageHeightPt - pt[1];
        fetch(container.dataset.syncUrl, {
            method: 'POST',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify({
                file: container.dataset.file,
                engine: container.dataset.engine,
                page: pageNum,
                x: synctexX,
                y: synctexY
            })
        }).then(function(r){ return r.json(); }).then(function(data){
            if (data && data.success && typeof data.line === 'number') {
                window.dispatchEvent(new CustomEvent('apich-jump-to-line', {detail: data.line}));
            }
        }).catch(function(){});
    }

    async function renderInto(container){
        var url = container.dataset.pdfUrl;
        if (!url) { return; }
        var myGen = (container.__apichGen = (container.__apichGen || 0) + 1);
        try {
            var pdfjsLib = await ensurePdfJs();
            var resp = await fetch(url);
            var ct = resp.headers.get('content-type') || '';
            if (ct.indexOf('application/pdf') === -1) {
                if (container.__apichGen === myGen) { showMessage(container, 'Compilation failed -- see the error panel above.'); }
                return;
            }
            var bytes = await resp.arrayBuffer();
            if (container.__apichGen !== myGen) { return; }
            var pdf = await pdfjsLib.getDocument({data: bytes}).promise;
            if (container.__apichGen !== myGen) { return; }
            container.innerHTML = '';
            for (var pageNum = 1; pageNum <= pdf.numPages; pageNum++) {
                var page = await pdf.getPage(pageNum);
                var unscaled = page.getViewport({scale: 1});
                var targetWidth = Math.max(container.clientWidth - 24, 200);
                var scale = targetWidth / unscaled.width;
                var viewport = page.getViewport({scale: scale});
                var canvas = document.createElement('canvas');
                canvas.width = viewport.width;
                canvas.height = viewport.height;
                canvas.style.cssText = 'box-shadow:0 4px 16px rgba(0,0,0,0.35); cursor:text; background:#fff;';
                var ctx = canvas.getContext('2d');
                await page.render({canvasContext: ctx, viewport: viewport}).promise;
                if (container.__apichGen !== myGen) { return; }
                canvas.addEventListener('click', function(pageNum, viewport, pageHeightPt, canvasEl){
                    return function(ev){ onPageClick(container, pageNum, viewport, pageHeightPt, canvasEl, ev); };
                }(pageNum, viewport, unscaled.height, canvas));
                container.appendChild(canvas);
            }
        } catch (e) {
            if (container.__apichGen === myGen) { showMessage(container, 'Failed to render PDF preview: ' + e); }
        }
    }

    function init(){
        var container = document.getElementById('latex-pdf-viewer');
        if (!container || container.__apichWired) { return; }
        container.__apichWired = true;
        renderInto(container);
        new MutationObserver(function(){ renderInto(container); })
            .observe(container, {attributes: true, attributeFilter: ['data-pdf-url']});
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();
";

/// Reactive: re-evaluates whenever `compile_error` or `pages` change, since both now update live
/// (hot reload) via a debounced recompile as the user types, not just once at page load.
fn render_typst_preview(
    compile_error: RwSignal<Option<String>>,
    pages: RwSignal<Vec<String>>,
    current_slide: RwSignal<usize>,
) -> impl IntoView {
    view! {
        <div style="height:100%;">
            {move || {
                if let Some(err) = compile_error.get() {
                    return view! {
                        <div class="section-card" style="background:#fef2f2; border:1px solid #fecaca; padding:1.5rem; height:100%; overflow-y:auto;">
                            <h3 style="color:#b91c1c; font-size:1rem; font-weight:700; margin-bottom:0.5rem;">"⚠️ Typst Compilation Diagnostics"</h3>
                            <pre style="color:#991b1b; font-family:var(--font-mono); font-size:0.825rem; white-space:pre-wrap; line-height:1.5;">{err}</pre>
                        </div>
                    }.into_any();
                }
                let page_count = pages.with(Vec::len);
                if page_count == 0 {
                    return view! {
                        <div class="empty-state" style="padding:2.5rem; height:100%; display:flex; flex-direction:column; justify-content:center; align-items:center;">
                            <p style="color:var(--text-sub);">"No pages rendered yet. Save the document to compile."</p>
                        </div>
                    }.into_any();
                }

                let page_divs: Vec<_> = (0..page_count)
                    .map(|idx| {
                        let svg = pages.with(|p| p.get(idx).cloned().unwrap_or_default());
                        view! {
                            <div
                                style:display=move || if current_slide.get() == idx.saturating_add(1) { "block" } else { "none" }
                                class="svg-page-box"
                                inner_html=svg
                            ></div>
                        }
                    })
                    .collect();

                view! {
                    <div class="svg-preview-stage">
                        <div class="svg-nav-toolbar">
                            <div style="display:flex; align-items:center; gap:0.4rem;">
                                <button type="button" class="btn btn-secondary btn-sm" on:click=move |_| current_slide.update(|s| if *s > 1 { *s = s.saturating_sub(1); })>"← Prev"</button>
                                <span style="font-weight:600; font-size:0.825rem; min-width:90px; text-align:center;">{move || format!("Page {} of {}", current_slide.get(), pages.with(|p| p.len().max(1)))}</span>
                                <button type="button" class="btn btn-secondary btn-sm" on:click=move |_| { let n = pages.with(|p| p.len().max(1)); current_slide.update(|s| if *s < n { *s = s.saturating_add(1); }) }>"Next →"</button>
                            </div>
                            <div style="font-size:0.75rem; color:var(--text-sub);">
                                <span>"💡 Click any line in preview to jump to code"</span>
                            </div>
                        </div>
                        <div class="svg-scroll-container" on:click=move |ev: leptos::ev::MouseEvent| handle_reverse_search_click(ev)>{page_divs}</div>
                    </div>
                }.into_any()
            }}
        </div>
    }
}

fn render_script_console(
    project_id: String,
    file_path: String,
) -> impl IntoView {
    let output =
        RwSignal::new("Ready to execute. Click \"Run Script\" or press Ctrl+Enter.".to_string());
    let status = RwSignal::new(None::<(bool, i64)>);
    let time_ms = RwSignal::new(None::<f64>);
    let plots = RwSignal::new(Vec::<(String, String)>::new());
    let busy = RwSignal::new(false);
    let args = RwSignal::new(String::new());

    let run = move || {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        output.set("Executing...".to_string());
        status.set(None);
        run_script(RunScriptArgs {
            project_id: project_id.clone(),
            file_path: file_path.clone(),
            args: args.get_untracked(),
            output,
            status,
            time_ms,
            plots,
            busy,
        });
    };
    let run_click = run.clone();

    // Exposes the run action to the shared Ctrl+Enter handler via a hidden trigger button, the
    // same cross-boundary technique used elsewhere in this codebase for coordinating separate
    // reactive scopes without threading signals across island/DOM boundaries.
    let run_trigger = move |_| run_click();

    view! {
        <div class="script-console-card">
            <div class="script-console-bar">
                <div style="display:flex; align-items:center; gap:0.5rem;">
                    <button type="button" class="btn btn-primary btn-sm" on:click=move |_| run() disabled=move || busy.get()>"▶ Run Script"</button>
                    <input
                        type="text"
                        placeholder="CLI arguments..."
                        class="form-control"
                        style="width:160px; font-size:0.75rem; height:28px; background:#1e293b; color:#fff; border-color:#334155;"
                        prop:value=move || args.get()
                        on:input=move |ev| args.set(event_target_value(&ev))
                    />
                </div>
                <div style="display:flex; align-items:center; gap:0.5rem; font-size:0.75rem; color:#94a3b8;">
                    {move || status.get().map(|(success, code)| {
                        let (bg, label) = if success { ("#16a34a", format!("SUCCESS (Exit {code})")) } else { ("#dc2626", format!("FAILED (Exit {code})")) };
                        view! { <span style=format!("padding:2px 6px; border-radius:4px; font-weight:600; background:{bg}; color:#fff;")>{label}</span> }
                    })}
                    <span style="font-family:var(--font-mono);">{move || time_ms.get().map(|t| format!("{t} ms")).unwrap_or_default()}</span>
                    <button type="button" class="btn btn-ghost btn-sm" style="color:#94a3b8; padding:2px 6px;" title="Clear Output" on:click=move |_| {
                        output.set("Output cleared.".to_string());
                        plots.set(Vec::new());
                        status.set(None);
                    }>"🧹 Clear"</button>
                </div>
            </div>
            <div class="terminal-output">{move || output.get()}</div>
            {move || {
                let p = plots.get();
                (!p.is_empty()).then(|| view! {
                    <div class="script-plot-card">
                        <div style="font-size:0.75rem; font-weight:700; color:#cbd5e1; text-transform:uppercase; margin-bottom:0.5rem;">"Generated Plot Output"</div>
                        <div>
                            {p.into_iter().map(|(name, data_uri)| view! {
                                <div style="margin:0.5rem 0;">
                                    <img src=data_uri alt=name.clone() />
                                    <div style="font-size:0.75rem; color:#94a3b8; margin-top:4px;">{name}</div>
                                </div>
                            }).collect::<Vec<_>>()}
                        </div>
                    </div>
                })
            }}
            <button type="button" style="display:none;" id="script-run-trigger" on:click=run_trigger></button>
        </div>
    }
}

#[cfg(feature = "hydrate")]
fn select_substring(
    ta: &web_sys::HtmlTextAreaElement,
    needle: &str,
) {
    let value = ta.value();
    if let Some(idx) = value.find(needle) {
        let start = value[..idx].encode_utf16().count() as u32;
        let end = start + needle.encode_utf16().count() as u32;
        let _ = ta.focus();
        let _ = ta.set_selection_range(start, end);
    }
}

#[cfg(not(feature = "hydrate"))]
const fn select_substring(
    _ta: &leptos::web_sys::HtmlTextAreaElement,
    _needle: &str,
) {
}

#[cfg(feature = "hydrate")]
fn request_submit(form: &web_sys::HtmlFormElement) {
    let _ = form.request_submit();
}

#[cfg(not(feature = "hydrate"))]
const fn request_submit(_form: &leptos::web_sys::HtmlFormElement) {}

#[cfg(feature = "hydrate")]
fn wire_keyboard_shortcuts(
    form_ref: NodeRef<leptos::html::Form>,
    current_slide: RwSignal<usize>,
    presenting: RwSignal<bool>,
    pages: RwSignal<Vec<String>>,
    is_slide: bool,
    is_script: bool,
) {
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;

    let closure =
        Closure::<dyn Fn(web_sys::KeyboardEvent)>::new(move |ev: web_sys::KeyboardEvent| {
            let key = ev.key();
            if key == "F11" && is_slide {
                ev.prevent_default();
                presenting.set(true);
                current_slide.set(1);
            } else if key == "Escape" {
                // In fullscreen, the browser already consumes Esc to leave fullscreen -- closing
                // the presentation on that same keypress would drop the presenter out of the deck
                // entirely when they only meant to un-fullscreen it. Only close when there's no
                // fullscreen for Esc to have been about.
                let in_fullscreen = web_sys::window()
                    .and_then(|w| w.document())
                    .is_some_and(|d| d.fullscreen_element().is_some());
                if !in_fullscreen {
                    presenting.set(false);
                }
            } else if (key == "ArrowRight" || key == " ") && presenting.get_untracked() {
                {
                    let n = pages.with(|p| p.len().max(1));
                    current_slide.update(|s| {
                        if *s < n {
                            *s = s.saturating_add(1);
                        }
                    });
                }
            } else if key == "ArrowLeft" && presenting.get_untracked() {
                current_slide.update(|s| {
                    if *s > 1 {
                        *s = s.saturating_sub(1);
                    }
                });
            } else if (ev.ctrl_key() || ev.meta_key()) && key == "s" {
                ev.prevent_default();
                if let Some(form) = form_ref.get_untracked() {
                    request_submit(&form);
                }
            } else if (ev.ctrl_key() || ev.meta_key()) && key == "Enter" && is_script {
                ev.prevent_default();
                if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                    if let Some(btn) = doc.get_element_by_id("script-run-trigger") {
                        let _ = btn.dyn_into::<web_sys::HtmlElement>().map(|b| b.click());
                    }
                }
            }
        });
    if let Some(win) = web_sys::window() {
        let _ = win.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref());
    }
    closure.forget();
}

#[cfg(not(feature = "hydrate"))]
const fn wire_keyboard_shortcuts(
    _form_ref: NodeRef<leptos::html::Form>,
    _current_slide: RwSignal<usize>,
    _presenting: RwSignal<bool>,
    _pages: RwSignal<Vec<String>>,
    _is_slide: bool,
    _is_script: bool,
) {
}

struct RunScriptArgs {
    project_id: String,
    file_path: String,
    args: String,
    output: RwSignal<String>,
    status: RwSignal<Option<(bool, i64)>>,
    time_ms: RwSignal<Option<f64>>,
    plots: RwSignal<Vec<(String, String)>>,
    busy: RwSignal<bool>,
}

#[cfg(feature = "hydrate")]
fn run_script(args: RunScriptArgs) {
    let project_id = args.project_id;
    let file_path = args.file_path;
    let script_args = args.args;
    let output = args.output;
    let status = args.status;
    let time_ms = args.time_ms;
    let plots = args.plots;
    let busy = args.busy;
    wasm_bindgen_futures::spawn_local(async move {
        let body = serde_json::json!({ "file": file_path, "args": script_args });
        let result = gloo_net::http::Request::post(&format!("/projects/{}/script/run", project_id))
            .json(&body)
            .expect("valid json body")
            .send()
            .await;

        match result {
            | Ok(resp) => {
                let status_code = resp.status();
                match resp.json::<serde_json::Value>().await {
                    | Ok(data) => {
                        // A non-2xx response still parses as valid JSON (`{"error": "..."}"`), which
                        // silently fell through the stdout/stderr extraction below (both absent on an
                        // error body) straight to the generic "(Process exited with empty output)" --
                        // exactly the "empty output, can't see why" symptom, for every real failure
                        // (missing script, missing interpreter, spawn error), not just successful runs
                        // with nothing printed. Surface the real message first.
                        if let Some(err_msg) = data.get("error").and_then(|v| v.as_str()) {
                            output.set(format!("Server error ({status_code}): {err_msg}"));
                            status.set(Some((false, -1)));
                            busy.set(false);
                            return;
                        }
                        let stdout = data
                            .get("stdout")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default();
                        let stderr = data
                            .get("stderr")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default();
                        let mut text = stdout.to_string();
                        if !stderr.is_empty() {
                            if !text.is_empty() {
                                text.push('\n');
                            }
                            text.push_str("[STDERR]:\n");
                            text.push_str(stderr);
                        }
                        if text.is_empty() {
                            text = "(Process exited with empty output)".to_string();
                        }
                        output.set(text);

                        let success = data
                            .get("success")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        let exit_code = data
                            .get("exit_code")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(if success { 0 } else { 1 });
                        status.set(Some((success, exit_code)));
                        time_ms.set(data.get("execution_time_ms").and_then(|v| v.as_f64()));

                        let imgs: Vec<(String, String)> = data
                            .get("output_images")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|img| {
                                        let name = img.get("name")?.as_str()?.to_string();
                                        let data_uri = img.get("data_uri")?.as_str()?.to_string();
                                        Some((name, data_uri))
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();
                        plots.set(imgs);
                    },
                    | Err(e) => output.set(format!("Error parsing response: {e}")),
                }
            },
            | Err(e) => output.set(format!("Error executing script: {e}")),
        }
        busy.set(false);
    });
}

#[cfg(not(feature = "hydrate"))]
fn run_script(args: RunScriptArgs) {
    args.busy.set(false);
}

/// The outline panel's headings, recomputed by `render_doc_preview_action` on every debounced
/// call alongside whatever else that response carries (see that handler's own comment for why).
/// Shared by all three debounce functions below so the "parse a `headings` JSON array into
/// `Vec<HeadingItem>`" logic isn't triplicated.
#[cfg(feature = "hydrate")]
fn parse_headings(data: &serde_json::Value) -> Vec<HeadingItem> {
    data.get("headings")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|h| {
                    Some(HeadingItem {
                        level: h.get("level")?.as_u64()? as u8,
                        text: h.get("text")?.as_str()?.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

struct TypstPreviewArgs {
    code_ref: NodeRef<leptos::html::Textarea>,
    project_id: String,
    file_path: String,
    debounce_gen: StoredValue<u32>,
    pages: RwSignal<Vec<String>>,
    compile_error: RwSignal<Option<String>>,
    current_slide: RwSignal<usize>,
    headings: RwSignal<Vec<HeadingItem>>,
}

/// Debounced "hot loading" for Typst/slide documents: 500ms after the last keystroke, if nothing
/// newer arrived (generation-counter debounce, same pattern as the note editor's live preview),
/// POST the current unsaved content to the existing `/projects/:id/render/preview` endpoint and
/// swap in the recompiled pages (or the compile error) -- no full page reload, no explicit Save.
#[cfg(feature = "hydrate")]
fn debounced_typst_preview(args: TypstPreviewArgs) {
    let code_ref = args.code_ref;
    let project_id = args.project_id;
    let file_path = args.file_path;
    let debounce_gen = args.debounce_gen;
    let pages = args.pages;
    let compile_error = args.compile_error;
    let current_slide = args.current_slide;
    let headings = args.headings;
    let my_gen = debounce_gen.get_value().wrapping_add(1);
    debounce_gen.set_value(my_gen);
    wasm_bindgen_futures::spawn_local(async move {
        gloo_timers::future::TimeoutFuture::new(500).await;
        if debounce_gen.get_value() != my_gen {
            return;
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
        let Ok(resp) = result else { return };
        let Ok(data) = resp.json::<serde_json::Value>().await else {
            return;
        };

        let success = data
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !success {
            let err = data
                .get("error")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            compile_error.set(err.or_else(|| Some("Compilation failed".to_string())));
            return;
        }
        compile_error.set(None);
        headings.set(parse_headings(&data));
        let new_pages: Vec<String> = data
            .get("pages")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|p| p.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        let new_count = new_pages.len();
        pages.set(new_pages);
        // Keep the reader's place, but never point past the end of a shrunk deck.
        current_slide.update(|s| {
            if *s == 0 || *s > new_count {
                *s = new_count.max(1);
            }
        });
    });
}
#[cfg(not(feature = "hydrate"))]
fn debounced_typst_preview(_args: TypstPreviewArgs) {}

struct LatexPreviewArgs {
    code_ref: NodeRef<leptos::html::Textarea>,
    project_id: String,
    file_path: String,
    debounce_gen: StoredValue<u32>,
    latex_reload_gen: RwSignal<u32>,
    latex_error: RwSignal<Option<String>>,
    latex_engine: RwSignal<String>,
    headings: RwSignal<Vec<HeadingItem>>,
}

/// Debounced "hot loading" for LaTeX: 500ms after the last keystroke, save + recompile via the
/// same `/render/preview` endpoint the note/Typst editors use, then -- only on success, so a
/// failed compile never replaces a working preview with the endpoint's plain-text error page --
/// bump `latex_reload_gen`, which changes the PDF `<iframe>`'s `src` query string so the browser
/// actually re-fetches it instead of showing its cached copy of the previous PDF.
#[cfg(feature = "hydrate")]
fn debounced_latex_preview(args: LatexPreviewArgs) {
    let code_ref = args.code_ref;
    let project_id = args.project_id;
    let file_path = args.file_path;
    let debounce_gen = args.debounce_gen;
    let latex_reload_gen = args.latex_reload_gen;
    let latex_error = args.latex_error;
    let latex_engine = args.latex_engine;
    let headings = args.headings;
    let my_gen = debounce_gen.get_value().wrapping_add(1);
    debounce_gen.set_value(my_gen);
    wasm_bindgen_futures::spawn_local(async move {
        gloo_timers::future::TimeoutFuture::new(500).await;
        if debounce_gen.get_value() != my_gen {
            return;
        }
        let Some(ta) = code_ref.get_untracked() else {
            return;
        };
        let content = ta.value();
        let engine = latex_engine.get_untracked();
        let body = serde_json::json!({ "file": file_path, "content": content, "engine": engine });
        let result =
            gloo_net::http::Request::post(&format!("/projects/{}/render/preview", project_id))
                .json(&body)
                .expect("valid json body")
                .send()
                .await;
        let Ok(resp) = result else { return };
        let Ok(data) = resp.json::<serde_json::Value>().await else {
            return;
        };
        headings.set(parse_headings(&data));
        let success = data
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if success {
            latex_error.set(None);
            latex_reload_gen.update(|g| *g = g.wrapping_add(1));
        } else {
            let err = data
                .get("error")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            latex_error.set(err.or_else(|| Some("Compilation failed".to_string())));
        }
    });
}
#[cfg(not(feature = "hydrate"))]
fn debounced_latex_preview(_args: LatexPreviewArgs) {}

/// Debounced "hot loading" for plain markdown documents opened through this island (not `.anote`
/// notes, which go through `NoteEditorIsland`'s own `debounced_preview` instead) -- previously
/// `on_code_input` had no branch for this file type at all, so neither the preview body nor the
/// outline ever updated until an explicit Save + full page reload.
#[cfg(feature = "hydrate")]
fn debounced_markdown_preview(
    code_ref: NodeRef<leptos::html::Textarea>,
    project_id: String,
    file_path: String,
    debounce_gen: StoredValue<u32>,
    markdown_html: RwSignal<String>,
    headings: RwSignal<Vec<HeadingItem>>,
) {
    let my_gen = debounce_gen.get_value().wrapping_add(1);
    debounce_gen.set_value(my_gen);
    wasm_bindgen_futures::spawn_local(async move {
        gloo_timers::future::TimeoutFuture::new(500).await;
        if debounce_gen.get_value() != my_gen {
            return;
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
        let Ok(resp) = result else { return };
        let Ok(data) = resp.json::<serde_json::Value>().await else {
            return;
        };
        headings.set(parse_headings(&data));
        if let Some(html) = data.get("html").and_then(|v| v.as_str()) {
            markdown_html.set(html.to_string());
        }
    });
}
#[cfg(not(feature = "hydrate"))]
fn debounced_markdown_preview(
    _code_ref: NodeRef<leptos::html::Textarea>,
    _project_id: String,
    _file_path: String,
    _debounce_gen: StoredValue<u32>,
    _markdown_html: RwSignal<String>,
    _headings: RwSignal<Vec<HeadingItem>>,
) {
}
