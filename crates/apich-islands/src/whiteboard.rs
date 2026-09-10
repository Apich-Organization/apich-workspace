//! Real Rust replacement for the whiteboard canvas that used to be a hand-written JS `<script>`
//! string in `apich-web`'s note page. Drawing, the pen/eraser/color toolbar, PNG export, and
//! the save-to-note round trip (`POST /projects/:id/note/whiteboard`) are all implemented here
//! as compiled Rust running in the browser via wasm -- no eval, no untyped DOM string-building.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Stroke {
    color: String,
    width: f64,
    points: Vec<(f64, f64)>,
}

/// A placed text label -- the whiteboard's answer to "tables and whiteboard need to take on
/// bold, italic, colors, and background colors": freehand strokes don't really have a notion of
/// "bold" or "background", but text does, so that's where these apply. Stored as a second,
/// independent array alongside `strokes` in the saved JSON (`{"strokes": [...], "texts": [...]}`)
/// rather than folding into `Stroke`, so existing whiteboards saved before this feature (with no
/// `texts` key at all) still deserialize fine via `#[serde(default)]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TextElement {
    x: f64,
    y: f64,
    text: String,
    color: String,
    bg_color: Option<String>,
    bold: bool,
    italic: bool,
    font_size: f64,
}

#[island]
pub fn WhiteboardIsland(
    #[prop(into)] project_id: String,
    #[prop(into)] file_path: String,
    #[prop(into)] initial_strokes_json: String,
) -> impl IntoView {
    let canvas_ref = NodeRef::<leptos::html::Canvas>::new();
    let mode = RwSignal::new("pen".to_string());
    let color = RwSignal::new("#1e293b".to_string());
    let bg_color = RwSignal::new(String::new());
    let bold = RwSignal::new(false);
    let italic = RwSignal::new(false);
    let save_status = RwSignal::new(String::new());
    let strokes = StoredValue::new(Vec::<Stroke>::new());
    let texts = StoredValue::new(Vec::<TextElement>::new());
    // A pending on-canvas text input: `Some((x, y))` while the user is typing a new label,
    // positioned at the click point that opened it.
    let pending_text = RwSignal::new(None::<(f64, f64)>);
    let pending_draft = RwSignal::new(String::new());

    wire_canvas(
        canvas_ref,
        mode,
        color,
        strokes,
        texts,
        initial_strokes_json,
        pending_text,
    );

    let on_clear = clear_handler(canvas_ref, strokes, texts);
    let on_export = export_handler(canvas_ref, file_path.clone());
    let on_save = save_handler(strokes, texts, save_status, project_id, file_path);

    let commit_text = move || {
        let Some((x, y)) = pending_text.get_untracked() else { return };
        let text = pending_draft.get_untracked();
        pending_text.set(None);
        pending_draft.set(String::new());
        if text.trim().is_empty() {
            return;
        }
        let el = TextElement {
            x,
            y,
            text,
            color: color.get_untracked(),
            bg_color: {
                let bg = bg_color.get_untracked();
                (!bg.is_empty()).then_some(bg)
            },
            bold: bold.get_untracked(),
            italic: italic.get_untracked(),
            font_size: 18.0,
        };
        texts.update_value(|t| t.push(el));
        redraw_all(canvas_ref, strokes, texts);
    };

    view! {
        <div class="section-card">
            <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:0.75rem; flex-wrap:wrap; gap:0.5rem;">
                <div style="display:flex; gap:0.4rem; align-items:center;">
                    <button
                        type="button"
                        class="btn btn-sm"
                        class:btn-primary=move || mode.get() == "pen"
                        class:btn-secondary=move || mode.get() != "pen"
                        on:click=move |_| mode.set("pen".to_string())
                    >
                        "✏️ Pen"
                    </button>
                    <button
                        type="button"
                        class="btn btn-sm"
                        class:btn-primary=move || mode.get() == "eraser"
                        class:btn-secondary=move || mode.get() != "eraser"
                        on:click=move |_| mode.set("eraser".to_string())
                    >
                        "🧹 Eraser"
                    </button>
                    <button
                        type="button"
                        class="btn btn-sm"
                        class:btn-primary=move || mode.get() == "text"
                        class:btn-secondary=move || mode.get() != "text"
                        title="Click the canvas to place styled text"
                        on:click=move |_| mode.set("text".to_string())
                    >
                        "🔤 Text"
                    </button>
                    <span style="width:1px; height:22px; background:var(--border-subtle);"></span>
                    <button
                        type="button"
                        class="btn btn-sm"
                        class:btn-primary=move || bold.get()
                        class:btn-secondary=move || !bold.get()
                        title="Bold (applies to new text)"
                        on:click=move |_| bold.update(|b| *b = !*b)
                    >
                        <strong>"B"</strong>
                    </button>
                    <button
                        type="button"
                        class="btn btn-sm"
                        class:btn-primary=move || italic.get()
                        class:btn-secondary=move || !italic.get()
                        title="Italic (applies to new text)"
                        on:click=move |_| italic.update(|i| *i = !*i)
                    >
                        <em>"I"</em>
                    </button>
                    <label style="display:flex; align-items:center; gap:0.25rem; font-size:0.7rem; color:var(--text-sub);">
                        "Color"
                        <input
                            type="color"
                            prop:value=move || color.get()
                            on:input=move |ev| color.set(event_target_value(&ev))
                            style="width:28px; height:28px; border:none; padding:0; cursor:pointer;"
                        />
                    </label>
                    <label style="display:flex; align-items:center; gap:0.25rem; font-size:0.7rem; color:var(--text-sub);">
                        "Highlight"
                        <input
                            type="color"
                            prop:value=move || { let v = bg_color.get(); if v.is_empty() { "#ffffff".to_string() } else { v } }
                            on:input=move |ev| bg_color.set(event_target_value(&ev))
                            style="width:28px; height:28px; border:none; padding:0; cursor:pointer;"
                        />
                        <button type="button" class="btn btn-ghost btn-sm" style="padding:1px 5px;" title="No highlight" on:click=move |_| bg_color.set(String::new())>"✕"</button>
                    </label>
                    <button type="button" class="btn btn-secondary btn-sm" on:click=on_clear>"🗑️ Clear"</button>
                </div>
                <div style="display:flex; align-items:center; gap:0.75rem;">
                    <span style="font-size:0.8rem; color:var(--text-sub);">{move || save_status.get()}</span>
                    <button type="button" class="btn btn-secondary btn-sm" on:click=on_export>"💾 Export PNG"</button>
                    <button type="button" class="btn btn-primary btn-sm" on:click=on_save>"Save to Note"</button>
                </div>
            </div>
            <div class="whiteboard-stage" style="padding:0; min-height:600px; display:flex; justify-content:center; align-items:center; position:relative;">
                <canvas
                    node_ref=canvas_ref
                    width="1100"
                    height="600"
                    style="display:block; cursor:crosshair; background:white; border-radius:var(--radius-md); box-shadow:var(--shadow-sm);"
                ></canvas>
                {move || pending_text.get().map(|(x, y)| {
                    let commit = commit_text;
                    let bold_v = bold.get();
                    let italic_v = italic.get();
                    view! {
                        <input
                            style=format!(
                                "position:absolute; left:{}px; top:{}px; font-size:18px; font-weight:{}; font-style:{}; color:{}; background:{}; border:1px dashed var(--primary); padding:2px 4px; z-index:10; min-width:120px;",
                                x, y,
                                if bold_v { "700" } else { "400" },
                                if italic_v { "italic" } else { "normal" },
                                color.get_untracked(),
                                { let bg = bg_color.get_untracked(); if bg.is_empty() { "transparent".to_string() } else { bg } },
                            )
                            autofocus=true
                            prop:value=move || pending_draft.get()
                            on:input=move |ev| pending_draft.set(event_target_value(&ev))
                            on:keydown=move |ev| {
                                if ev.key() == "Enter" { commit(); }
                                if ev.key() == "Escape" { pending_text.set(None); pending_draft.set(String::new()); }
                            }
                            on:blur=move |_| commit()
                        />
                    }
                })}
            </div>
        </div>
    }
}

// --- Everything below only makes sense once this is actually running in a browser (the
// `hydrate` feature, which pulls in web-sys/wasm-bindgen/gloo-net). Under `ssr` these become
// no-ops so the server can still render the static HTML shell above without those deps.

#[cfg(feature = "hydrate")]
fn wire_canvas(
    canvas_ref: NodeRef<leptos::html::Canvas>,
    mode: RwSignal<String>,
    color: RwSignal<String>,
    strokes: StoredValue<Vec<Stroke>>,
    texts: StoredValue<Vec<TextElement>>,
    initial_strokes_json: String,
    pending_text: RwSignal<Option<(f64, f64)>>,
) {
    use wasm_bindgen::JsCast;

    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&initial_strokes_json) {
        if let Some(points) = parsed.get("strokes").cloned() {
            if let Ok(parsed_strokes) = serde_json::from_value::<Vec<Stroke>>(points) {
                strokes.set_value(parsed_strokes);
            }
        }
        if let Some(t) = parsed.get("texts").cloned() {
            if let Ok(parsed_texts) = serde_json::from_value::<Vec<TextElement>>(t) {
                texts.set_value(parsed_texts);
            }
        }
    }

    Effect::new(move |_| {
        let Some(canvas) = canvas_ref.get() else { return };
        let ctx = canvas_context(&canvas);
        redraw_all(canvas_ref, strokes, texts);

        let drawing = std::rc::Rc::new(std::cell::Cell::new(false));
        let current = std::rc::Rc::new(std::cell::RefCell::new(None::<Stroke>));

        let pos_from_mouse = |ev: &web_sys::MouseEvent, canvas: &web_sys::HtmlCanvasElement| -> (f64, f64) {
            let rect = canvas.get_bounding_client_rect();
            (ev.client_x() as f64 - rect.left(), ev.client_y() as f64 - rect.top())
        };
        let pos_from_touch = |ev: &web_sys::TouchEvent, canvas: &web_sys::HtmlCanvasElement| -> Option<(f64, f64)> {
            let touch = ev.touches().get(0)?;
            let rect = canvas.get_bounding_client_rect();
            Some((touch.client_x() as f64 - rect.left(), touch.client_y() as f64 - rect.top()))
        };

        let start_stroke = {
            let mode = mode;
            let color = color;
            let drawing = drawing.clone();
            let current = current.clone();
            move |x: f64, y: f64| {
                if mode.get_untracked() == "text" {
                    pending_text.set(Some((x, y)));
                    return;
                }
                drawing.set(true);
                let is_eraser = mode.get_untracked() == "eraser";
                *current.borrow_mut() = Some(Stroke {
                    color: if is_eraser { "#ffffff".to_string() } else { color.get_untracked() },
                    width: if is_eraser { 18.0 } else { 3.0 },
                    points: vec![(x, y)],
                });
            }
        };

        let canvas_for_move = canvas.clone();
        let ctx_for_move = ctx.clone();
        let move_stroke = {
            let drawing = drawing.clone();
            let current = current.clone();
            move |x: f64, y: f64| {
                if !drawing.get() {
                    return;
                }
                let mut c = current.borrow_mut();
                let Some(stroke) = c.as_mut() else { return };
                stroke.points.push((x, y));
                let n = stroke.points.len();
                if n < 2 {
                    return;
                }
                paint_segment(&ctx_for_move, &stroke.color, stroke.width, stroke.points[n - 2], stroke.points[n - 1]);
            }
        };
        let _ = &canvas_for_move;

        let end_stroke = {
            let drawing = drawing.clone();
            let current = current.clone();
            move || {
                if !drawing.get() {
                    return;
                }
                drawing.set(false);
                if let Some(stroke) = current.borrow_mut().take() {
                    if stroke.points.len() > 1 {
                        strokes.update_value(|s| s.push(stroke));
                    }
                }
            }
        };

        let canvas_el: web_sys::HtmlCanvasElement = canvas.clone();

        let md_canvas = canvas_el.clone();
        let start_stroke_md = start_stroke.clone();
        let on_mousedown = wasm_bindgen::closure::Closure::<dyn Fn(web_sys::MouseEvent)>::new(move |ev: web_sys::MouseEvent| {
            let (x, y) = pos_from_mouse(&ev, &md_canvas);
            start_stroke_md(x, y);
        });
        canvas_el.add_event_listener_with_callback("mousedown", on_mousedown.as_ref().unchecked_ref()).ok();
        on_mousedown.forget();

        let mm_canvas = canvas_el.clone();
        let move_stroke_mm = move_stroke.clone();
        let on_mousemove = wasm_bindgen::closure::Closure::<dyn Fn(web_sys::MouseEvent)>::new(move |ev: web_sys::MouseEvent| {
            let (x, y) = pos_from_mouse(&ev, &mm_canvas);
            move_stroke_mm(x, y);
        });
        canvas_el.add_event_listener_with_callback("mousemove", on_mousemove.as_ref().unchecked_ref()).ok();
        on_mousemove.forget();

        let end_stroke_mu = end_stroke.clone();
        let on_mouseup = wasm_bindgen::closure::Closure::<dyn Fn(web_sys::MouseEvent)>::new(move |_ev: web_sys::MouseEvent| {
            end_stroke_mu();
        });
        if let Some(win) = web_sys::window() {
            win.add_event_listener_with_callback("mouseup", on_mouseup.as_ref().unchecked_ref()).ok();
        }
        on_mouseup.forget();

        let ts_canvas = canvas_el.clone();
        let start_stroke_ts = start_stroke.clone();
        let on_touchstart = wasm_bindgen::closure::Closure::<dyn Fn(web_sys::TouchEvent)>::new(move |ev: web_sys::TouchEvent| {
            ev.prevent_default();
            if let Some((x, y)) = pos_from_touch(&ev, &ts_canvas) {
                start_stroke_ts(x, y);
            }
        });
        canvas_el.add_event_listener_with_callback("touchstart", on_touchstart.as_ref().unchecked_ref()).ok();
        on_touchstart.forget();

        let tm_canvas = canvas_el.clone();
        let move_stroke_tm = move_stroke.clone();
        let on_touchmove = wasm_bindgen::closure::Closure::<dyn Fn(web_sys::TouchEvent)>::new(move |ev: web_sys::TouchEvent| {
            ev.prevent_default();
            if let Some((x, y)) = pos_from_touch(&ev, &tm_canvas) {
                move_stroke_tm(x, y);
            }
        });
        canvas_el.add_event_listener_with_callback("touchmove", on_touchmove.as_ref().unchecked_ref()).ok();
        on_touchmove.forget();

        let end_stroke_te = end_stroke.clone();
        let on_touchend = wasm_bindgen::closure::Closure::<dyn Fn(web_sys::TouchEvent)>::new(move |_ev: web_sys::TouchEvent| {
            end_stroke_te();
        });
        canvas_el.add_event_listener_with_callback("touchend", on_touchend.as_ref().unchecked_ref()).ok();
        on_touchend.forget();
    });
}

#[cfg(not(feature = "hydrate"))]
fn wire_canvas(
    _canvas_ref: NodeRef<leptos::html::Canvas>,
    _mode: RwSignal<String>,
    _color: RwSignal<String>,
    _strokes: StoredValue<Vec<Stroke>>,
    _texts: StoredValue<Vec<TextElement>>,
    _initial_strokes_json: String,
    _pending_text: RwSignal<Option<(f64, f64)>>,
) {
}

#[cfg(feature = "hydrate")]
fn canvas_context(canvas: &web_sys::HtmlCanvasElement) -> web_sys::CanvasRenderingContext2d {
    use wasm_bindgen::JsCast;
    canvas
        .get_context("2d")
        .ok()
        .flatten()
        .and_then(|c| c.dyn_into::<web_sys::CanvasRenderingContext2d>().ok())
        .expect("2d canvas context")
}

#[cfg(feature = "hydrate")]
fn paint_segment(ctx: &web_sys::CanvasRenderingContext2d, color: &str, width: f64, from: (f64, f64), to: (f64, f64)) {
    ctx.set_stroke_style_str(color);
    ctx.set_line_width(width);
    ctx.set_line_cap("round");
    ctx.set_line_join("round");
    ctx.begin_path();
    ctx.move_to(from.0, from.1);
    ctx.line_to(to.0, to.1);
    ctx.stroke();
}

/// Draws every stroke, then every text element, onto the canvas -- called both on initial mount
/// (loading whatever was last saved) and after any change (a new stroke, a new text label,
/// clearing), so the visible canvas and the `strokes`/`texts` model never drift apart.
#[cfg(feature = "hydrate")]
fn redraw_all(canvas_ref: NodeRef<leptos::html::Canvas>, strokes: StoredValue<Vec<Stroke>>, texts: StoredValue<Vec<TextElement>>) {
    let Some(canvas) = canvas_ref.get_untracked() else { return };
    let ctx = canvas_context(&canvas);
    ctx.clear_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);
    strokes.with_value(|all| {
        for s in all {
            if s.points.len() < 2 {
                continue;
            }
            ctx.set_stroke_style_str(&s.color);
            ctx.set_line_width(s.width);
            ctx.set_line_cap("round");
            ctx.set_line_join("round");
            ctx.begin_path();
            ctx.move_to(s.points[0].0, s.points[0].1);
            for p in &s.points[1..] {
                ctx.line_to(p.0, p.1);
            }
            ctx.stroke();
        }
    });
    texts.with_value(|all| {
        for t in all {
            paint_text(&ctx, t);
        }
    });
}

#[cfg(not(feature = "hydrate"))]
fn redraw_all(_canvas_ref: NodeRef<leptos::html::Canvas>, _strokes: StoredValue<Vec<Stroke>>, _texts: StoredValue<Vec<TextElement>>) {}

#[cfg(feature = "hydrate")]
fn paint_text(ctx: &web_sys::CanvasRenderingContext2d, t: &TextElement) {
    let weight = if t.bold { "700" } else { "400" };
    let style = if t.italic { "italic" } else { "normal" };
    ctx.set_font(&format!("{} {} {}px sans-serif", style, weight, t.font_size));
    ctx.set_text_baseline("top");
    if let Some(bg) = &t.bg_color {
        // `measure_text` needs the font already set (done above) to size the highlight box
        // correctly for this element's own weight/size, not whatever the context last had.
        if let Ok(metrics) = ctx.measure_text(&t.text) {
            let w = metrics.width();
            let pad = 4.0;
            ctx.set_fill_style_str(bg);
            ctx.fill_rect(t.x - pad, t.y - pad, w + pad * 2.0, t.font_size + pad * 2.0);
        }
    }
    ctx.set_fill_style_str(&t.color);
    let _ = ctx.fill_text(&t.text, t.x, t.y);
}

#[cfg(feature = "hydrate")]
fn clear_handler(
    canvas_ref: NodeRef<leptos::html::Canvas>,
    strokes: StoredValue<Vec<Stroke>>,
    texts: StoredValue<Vec<TextElement>>,
) -> impl Fn(leptos::ev::MouseEvent) + Clone + 'static {
    move |_| {
        let confirmed = web_sys::window()
            .and_then(|w| w.confirm_with_message("Clear whiteboard?").ok())
            .unwrap_or(false);
        if !confirmed {
            return;
        }
        strokes.update_value(|s| s.clear());
        texts.update_value(|t| t.clear());
        redraw_all(canvas_ref, strokes, texts);
    }
}

#[cfg(not(feature = "hydrate"))]
fn clear_handler(
    _canvas_ref: NodeRef<leptos::html::Canvas>,
    _strokes: StoredValue<Vec<Stroke>>,
    _texts: StoredValue<Vec<TextElement>>,
) -> impl Fn(leptos::ev::MouseEvent) + Clone + 'static {
    move |_| {}
}

#[cfg(feature = "hydrate")]
fn export_handler(
    canvas_ref: NodeRef<leptos::html::Canvas>,
    file_path: String,
) -> impl Fn(leptos::ev::MouseEvent) + Clone + 'static {
    use wasm_bindgen::JsCast;
    move |_| {
        let Some(el) = canvas_ref.get_untracked() else { return };
        let Ok(data_url) = el.to_data_url() else { return };
        let Some(document) = web_sys::window().and_then(|w| w.document()) else { return };
        let Ok(anchor) = document.create_element("a") else { return };
        let Ok(anchor) = anchor.dyn_into::<web_sys::HtmlAnchorElement>() else { return };
        anchor.set_href(&data_url);
        anchor.set_download(&format!("whiteboard-{}.png", file_path.replace(['/', '\\'], "_")));
        anchor.click();
    }
}

#[cfg(not(feature = "hydrate"))]
fn export_handler(
    _canvas_ref: NodeRef<leptos::html::Canvas>,
    _file_path: String,
) -> impl Fn(leptos::ev::MouseEvent) + Clone + 'static {
    move |_| {}
}

#[cfg(feature = "hydrate")]
fn save_handler(
    strokes: StoredValue<Vec<Stroke>>,
    texts: StoredValue<Vec<TextElement>>,
    save_status: RwSignal<String>,
    project_id: String,
    file_path: String,
) -> impl Fn(leptos::ev::MouseEvent) + Clone + 'static {
    move |_| {
        save_status.set("Saving...".to_string());
        let strokes_json = strokes.with_value(|s| {
            texts.with_value(|t| {
                serde_json::to_string(&serde_json::json!({ "strokes": s, "texts": t })).unwrap_or_else(|_| "{\"strokes\":[],\"texts\":[]}".to_string())
            })
        });
        let project_id = project_id.clone();
        let file_path = file_path.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let body = serde_json::json!({ "file": file_path, "whiteboard_json": strokes_json });
            let result = gloo_net::http::Request::post(&format!("/projects/{}/note/whiteboard", project_id))
                .json(&body)
                .expect("serializable body")
                .send()
                .await;
            match result {
                Ok(resp) if resp.ok() => save_status.set("Saved".to_string()),
                Ok(resp) => save_status.set(format!("Save failed ({})", resp.status())),
                Err(e) => save_status.set(format!("Save failed: {e}")),
            }
        });
    }
}

#[cfg(not(feature = "hydrate"))]
fn save_handler(
    _strokes: StoredValue<Vec<Stroke>>,
    _texts: StoredValue<Vec<TextElement>>,
    _save_status: RwSignal<String>,
    _project_id: String,
    _file_path: String,
) -> impl Fn(leptos::ev::MouseEvent) + Clone + 'static {
    move |_| {}
}
