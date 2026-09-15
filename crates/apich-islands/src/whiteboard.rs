//! Real Rust replacement for the whiteboard canvas component.
//!
//! Drawing, the toolbar (brushes, shapes, fill, pan/zoom, undo/redo, image/video import),
//! PNG export, and the save-to-note round trip (`POST /projects/:id/note/whiteboard`) are all
//! implemented here as compiled Rust running in the browser via wasm -- no eval, no untyped
//! DOM string-building.
//!
//! Per direct user feedback ("whiteboard need far more paint brushes, colors, shapes, zooms,
//! fill, image import, video imports, redo, undo, pan and so on"), everything drawn here is one
//! unified, ordered `Element` list (`Stroke`/`Shape`/`Text`/`Image`) rather than the previous
//! separate `strokes`/`texts` arrays -- shapes and images need the same z-ordering, undo, and
//! pan/zoom transform as strokes and text already had, so folding them into one model was
//! simpler than bolting parallel machinery onto each array. Old whiteboards saved before this
//! (the `{"strokes": [...], "texts": [...]}` shape) still load fine -- see `load_elements`'s doc
//! comment -- and get upgraded to the new `{"elements": [...]}` format on next save.
//!
//! Video import doesn't mean video *playback* here: a `<canvas>` can't play a video inline the
//! way an HTML5 `<video>` element does (that would need a second DOM layer positioned over the
//! canvas, with its own pan/zoom/undo/z-order story -- a real, larger feature on its own). What's
//! implemented is real and useful on its own terms: the imported video's first frame is decoded
//! and placed on the board as a real image element (same as a photo import), so a video can still
//! be dropped in as a visual reference. Said plainly in the toolbar's own title text rather than
//! silently pretending it's full playback.

#![allow(
    clippy::must_use_candidate,
    clippy::too_many_lines,
    clippy::needless_pass_by_value
)]

use leptos::prelude::*;
use serde::Deserialize;
use serde::Serialize;

const fn default_alpha() -> f64 {
    1.0
}

/// One drawable thing on the board, in a single ordered list so z-order, undo/redo, and the
/// pan/zoom transform all apply uniformly regardless of what kind of element it is.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
enum Element {
    Stroke {
        color: String,
        width: f64,
        #[serde(default = "default_alpha")]
        alpha: f64,
        points: Vec<(f64, f64)>,
    },
    Shape {
        shape: String, // "rect" | "ellipse" | "line" | "arrow"
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        color: String,
        fill: Option<String>,
        width: f64,
    },
    Text {
        x: f64,
        y: f64,
        text: String,
        color: String,
        bg_color: Option<String>,
        bold: bool,
        italic: bool,
        font_size: f64,
    },
    Image {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        data_url: String,
    },
}

/// Old, pre-unification save shapes, kept only to deserialize whiteboards saved before this
/// change -- never written anymore, see `load_elements`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OldStroke {
    color: String,
    width: f64,
    points: Vec<(f64, f64)>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OldText {
    x: f64,
    y: f64,
    text: String,
    color: String,
    bg_color: Option<String>,
    bold: bool,
    italic: bool,
    font_size: f64,
}

/// Parses a saved whiteboard's JSON into the unified `Element` list. Prefers the current
/// `{"elements": [...]}` shape; falls back to the pre-unification `{"strokes": [...], "texts":
/// [...]}` shape (strokes first, then texts, matching the z-order that version always drew in) so
/// nothing saved before this feature silently loses content. The next Save always writes the
/// current format -- this is a one-way, load-time upgrade, not an ongoing dual-format writer.
fn load_elements(json: &str) -> Vec<Element> {
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json) else {
        return Vec::new();
    };
    if let Some(els) = parsed.get("elements").cloned() {
        if let Ok(elements) = serde_json::from_value::<Vec<Element>>(els) {
            return elements;
        }
    }
    let mut out = Vec::new();
    if let Some(v) = parsed.get("strokes").cloned() {
        if let Ok(strokes) = serde_json::from_value::<Vec<OldStroke>>(v) {
            for s in strokes {
                out.push(Element::Stroke {
                    color: s.color,
                    width: s.width,
                    alpha: 1.0,
                    points: s.points,
                });
            }
        }
    }
    if let Some(v) = parsed.get("texts").cloned() {
        if let Ok(texts) = serde_json::from_value::<Vec<OldText>>(v) {
            for t in texts {
                out.push(Element::Text {
                    x: t.x,
                    y: t.y,
                    text: t.text,
                    color: t.color,
                    bg_color: t.bg_color,
                    bold: t.bold,
                    italic: t.italic,
                    font_size: t.font_size,
                });
            }
        }
    }
    out
}

const MAX_UNDO_DEPTH: usize = 50;

/// A brush preset -- width + opacity pairs a user picks by name rather than dialing in two
/// sliders every time, the same convenience a real drawing app's brush picker gives. The width
/// slider next to it still allows fine-tuning on top of whichever preset is selected.
fn brush_preset(name: &str) -> (f64, f64) {
    match name {
        | "fine" => (1.5, 1.0),
        | "marker" => (8.0, 0.9),
        | "highlighter" => (20.0, 0.35),
        | _ => (3.0, 1.0), // "pen"
    }
}

#[island]
#[must_use]
#[allow(
    clippy::too_many_lines,
    clippy::needless_pass_by_value,
    clippy::must_use_candidate
)]
pub fn WhiteboardIsland(
    #[prop(into)] project_id: String,
    #[prop(into)] file_path: String,
    #[prop(into)] initial_strokes_json: String,
) -> impl IntoView {
    let canvas_ref = NodeRef::<leptos::html::Canvas>::new();
    let image_input_ref = NodeRef::<leptos::html::Input>::new();
    let video_input_ref = NodeRef::<leptos::html::Input>::new();

    let mode = RwSignal::new("pen".to_string());
    let brush = RwSignal::new("pen".to_string());
    let color = RwSignal::new("#1e293b".to_string());
    let bg_color = RwSignal::new(String::new());
    let bold = RwSignal::new(false);
    let italic = RwSignal::new(false);
    let fill_enabled = RwSignal::new(false);
    let fill_color = RwSignal::new("#fde68a".to_string());
    let brush_width = RwSignal::new(3.0f64);
    let zoom = RwSignal::new(1.0f64);
    let view_x = RwSignal::new(0.0f64);
    let view_y = RwSignal::new(0.0f64);
    let save_status = RwSignal::new(String::new());
    let elements = StoredValue::new(load_elements(&initial_strokes_json));
    drop(initial_strokes_json);
    let undo_stack = StoredValue::new(Vec::<Vec<Element>>::new());
    let redo_stack = StoredValue::new(Vec::<Vec<Element>>::new());
    let can_undo = RwSignal::new(false);
    let can_redo = RwSignal::new(false);
    // A pending on-canvas text input: `Some((x, y))` in WORLD coordinates while the user is
    // typing a new label, positioned (on screen) via the current pan/zoom transform below.
    let pending_text = RwSignal::new(None::<(f64, f64)>);
    let pending_draft = RwSignal::new(String::new());

    let push_undo = move || {
        undo_stack.update_value(|stack| {
            elements.with_value(|els| stack.push(els.clone()));
            if stack.len() > MAX_UNDO_DEPTH {
                stack.remove(0);
            }
        });
        redo_stack.update_value(Vec::clear);
        can_undo.set(true);
        can_redo.set(false);
    };

    wire_canvas(
        canvas_ref,
        mode,
        color,
        fill_enabled,
        fill_color,
        brush_width,
        brush,
        zoom,
        view_x,
        view_y,
        elements,
        pending_text,
        Box::new(push_undo),
    );

    let on_undo = {
        move |_| {
            let Some(prev) = undo_stack.try_update_value(Vec::pop).flatten() else {
                return;
            };
            redo_stack.update_value(|s| {
                elements.with_value(|els| s.push(els.clone()));
            });
            elements.set_value(prev);
            can_undo.set(undo_stack.with_value(|s| !s.is_empty()));
            can_redo.set(true);
            redraw_all(canvas_ref, elements, zoom, view_x, view_y);
        }
    };
    let on_redo = {
        move |_| {
            let Some(next) = redo_stack.try_update_value(Vec::pop).flatten() else {
                return;
            };
            undo_stack.update_value(|s| {
                elements.with_value(|els| s.push(els.clone()));
            });
            elements.set_value(next);
            can_redo.set(redo_stack.with_value(|s| !s.is_empty()));
            can_undo.set(true);
            redraw_all(canvas_ref, elements, zoom, view_x, view_y);
        }
    };

    let on_clear = clear_handler(
        canvas_ref,
        elements,
        zoom,
        view_x,
        view_y,
        Box::new(push_undo),
        can_undo,
        can_redo,
        redo_stack,
    );
    let on_export = export_handler(canvas_ref, file_path.clone());
    let on_save_asset = save_asset_handler(
        canvas_ref,
        save_status,
        project_id.clone(),
        file_path.clone(),
    );
    let on_save = save_handler(elements, save_status, project_id, file_path);

    let on_zoom_in = move |_| zoom.update(|z| *z = (*z * 1.2).min(4.0));
    let on_zoom_out = move |_| zoom.update(|z| *z = (*z / 1.2).max(0.25));
    let on_zoom_reset = move |_| {
        zoom.set(1.0);
        view_x.set(0.0);
        view_y.set(0.0);
    };

    let commit_text = {
        move || {
            let Some((x, y)) = pending_text.get_untracked() else {
                return;
            };
            let text = pending_draft.get_untracked();
            pending_text.set(None);
            pending_draft.set(String::new());
            if text.trim().is_empty() {
                return;
            }
            push_undo();
            let el = Element::Text {
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
            elements.update_value(|e| e.push(el));
            redraw_all(canvas_ref, elements, zoom, view_x, view_y);
        }
    };

    let on_image_file = image_file_handler(
        canvas_ref,
        elements,
        zoom,
        view_x,
        view_y,
        Box::new(push_undo),
    );
    let on_video_file = video_file_handler(
        canvas_ref,
        elements,
        zoom,
        view_x,
        view_y,
        Box::new(push_undo),
    );

    let shape_btn = move |kind: &'static str, icon: &'static str, title: &'static str| {
        view! {
            <button
                type="button"
                class="btn btn-sm"
                class:btn-primary=move || mode.get() == kind
                class:btn-secondary=move || mode.get() != kind
                title=title
                on:click=move |_| mode.set(kind.to_string())
            >
                {icon}
            </button>
        }
    };

    view! {
        <div class="section-card">
            <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:0.5rem; flex-wrap:wrap; gap:0.5rem;">
                <div style="display:flex; gap:0.35rem; align-items:center; flex-wrap:wrap;">
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
                    {shape_btn("rect", "▭", "Rectangle (drag to draw)")}
                    {shape_btn("ellipse", "⬭", "Ellipse (drag to draw)")}
                    {shape_btn("line", "╱", "Line (drag to draw)")}
                    {shape_btn("arrow", "➚", "Arrow (drag to draw)")}
                    <span style="width:1px; height:22px; background:var(--border-subtle);"></span>
                    <button
                        type="button"
                        class="btn btn-sm"
                        class:btn-primary=move || mode.get() == "pan"
                        class:btn-secondary=move || mode.get() != "pan"
                        title="Pan (drag to move the view)"
                        on:click=move |_| mode.set("pan".to_string())
                    >
                        "🖐️ Pan"
                    </button>
                </div>
                <div style="display:flex; align-items:center; gap:0.5rem; flex-wrap:wrap;">
                    <span style="font-size:0.8rem; color:var(--text-sub);">{move || save_status.get()}</span>
                    <button type="button" class="btn btn-secondary btn-sm" on:click=on_export title="Download PNG to your computer">"💾 Export PNG"</button>
                    <button type="button" class="btn btn-secondary btn-sm" on:click=on_save_asset title="Save PNG to project assets folder for use in other documents">"📁 Save to Assets"</button>
                    <button type="button" class="btn btn-primary btn-sm" on:click=on_save title="Save whiteboard strokes to this note file">"Save to Note"</button>
                </div>
            </div>
            <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:0.75rem; flex-wrap:wrap; gap:0.5rem; font-size:0.7rem; color:var(--text-sub);">
                <div style="display:flex; gap:0.6rem; align-items:center; flex-wrap:wrap;">
                    <label style="display:flex; align-items:center; gap:0.3rem;">
                        "Brush"
                        <select class="form-control" style="width:auto; height:26px; padding:0 4px; font-size:0.7rem;" prop:value=move || brush.get() on:change=move |ev| {
                            let v = event_target_value(&ev);
                            let (w, _a) = brush_preset(&v);
                            brush_width.set(w);
                            brush.set(v);
                        }>
                            <option value="pen">"Pen"</option>
                            <option value="fine">"Fine liner"</option>
                            <option value="marker">"Marker"</option>
                            <option value="highlighter">"Highlighter"</option>
                        </select>
                    </label>
                    <label style="display:flex; align-items:center; gap:0.3rem;">
                        "Width"
                        <input type="range" min="1" max="40" step="1" style="width:70px;" prop:value=move || brush_width.get().to_string() on:input=move |ev| {
                            if let Ok(v) = event_target_value(&ev).parse::<f64>() { brush_width.set(v); }
                        } />
                        <span>{move || format!("{:.0}px", brush_width.get())}</span>
                    </label>
                    <label style="display:flex; align-items:center; gap:0.25rem;">
                        "Color"
                        <input type="color" prop:value=move || color.get() on:input=move |ev| color.set(event_target_value(&ev)) style="width:26px; height:26px; border:none; padding:0; cursor:pointer;" />
                    </label>
                    <label style="display:flex; align-items:center; gap:0.25rem;" title="Text highlight background">
                        "Highlight"
                        <input
                            type="color"
                            prop:value=move || { let v = bg_color.get(); if v.is_empty() { "#ffffff".to_string() } else { v } }
                            on:input=move |ev| bg_color.set(event_target_value(&ev))
                            style="width:26px; height:26px; border:none; padding:0; cursor:pointer;"
                        />
                        <button type="button" class="btn btn-ghost btn-sm" style="padding:0 4px; font-size:0.7rem;" title="No highlight" on:click=move |_| bg_color.set(String::new())>"✕"</button>
                    </label>
                    <label style="display:flex; align-items:center; gap:0.25rem;" title="Fill new shapes with this color">
                        <input type="checkbox" prop:checked=move || fill_enabled.get() on:change=move |ev| fill_enabled.set(event_target_checked(&ev)) />
                        "Fill"
                        <input type="color" prop:value=move || fill_color.get() on:input=move |ev| fill_color.set(event_target_value(&ev)) style="width:26px; height:26px; border:none; padding:0; cursor:pointer;" disabled=move || !fill_enabled.get() />
                    </label>
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
                </div>
                <div style="display:flex; gap:0.4rem; align-items:center;">
                    <button type="button" class="btn btn-secondary btn-sm" disabled=move || !can_undo.get() on:click=on_undo title="Undo">"↶ Undo"</button>
                    <button type="button" class="btn btn-secondary btn-sm" disabled=move || !can_redo.get() on:click=on_redo title="Redo">"↷ Redo"</button>
                    <span style="width:1px; height:20px; background:var(--border-subtle);"></span>
                    <button type="button" class="btn btn-secondary btn-sm" on:click=on_zoom_out title="Zoom out">"−"</button>
                    <span style="min-width:38px; text-align:center;">{move || format!("{:.0}%", zoom.get() * 100.0)}</span>
                    <button type="button" class="btn btn-secondary btn-sm" on:click=on_zoom_in title="Zoom in">"+"</button>
                    <button type="button" class="btn btn-secondary btn-sm" on:click=on_zoom_reset title="Reset view">"⟲"</button>
                    <span style="width:1px; height:20px; background:var(--border-subtle);"></span>
                    <button type="button" class="btn btn-secondary btn-sm" title="Import an image onto the board" on:click=move |_| { if let Some(el) = image_input_ref.get() { el.click(); } }>"🖼️ Image"</button>
                    <button type="button" class="btn btn-secondary btn-sm" title="Import a video's first frame as an image (this board can't play video)" on:click=move |_| { if let Some(el) = video_input_ref.get() { el.click(); } }>"🎬 Video"</button>
                    <button type="button" class="btn btn-secondary btn-sm" on:click=on_clear>"🗑️ Clear"</button>
                </div>
            </div>
            <input node_ref=image_input_ref type="file" accept="image/*" style="display:none;" on:change=on_image_file />
            <input node_ref=video_input_ref type="file" accept="video/*" style="display:none;" on:change=on_video_file />
            // The canvas's backing-store resolution (the `width`/`height` HTML attributes below,
            // NOT CSS) must actually fit within this flex-centered stage's real content width --
            // confirmed live as a real, pre-existing bug (present before this session's rewrite
            // too): at the app's normal ~874px content width (sidebar + padding already
            // subtracted from a standard viewport), the previous 1100px canvas overflowed by
            // roughly 113px on each side. Flexbox `justify-content:center` doesn't clip that
            // overflow, so the canvas kept rendering at its full width -- but an ancestor further
            // up the layout DOES clip it, making that overflowing region invisible *and*
            // unreachable by mouse/touch simultaneously, silently eating the left/right edges of
            // every stroke a user tried to draw there. 860px reliably fits within the measured
            // content width with margin to spare.
            <div class="whiteboard-stage" style="padding:0; min-height:600px; display:flex; justify-content:center; align-items:center; position:relative;">
                <canvas
                    node_ref=canvas_ref
                    width="860"
                    height="600"
                    style="display:block; background:white; border-radius:var(--radius-md); box-shadow:var(--shadow-sm); max-width:100%;"
                    style:cursor=move || if mode.get() == "pan" { "grab" } else { "crosshair" }
                ></canvas>
                {move || pending_text.get().map(|(wx, wy)| {
                    let commit = commit_text;
                    let bold_v = bold.get();
                    let italic_v = italic.get();
                    let z = zoom.get();
                    let sx = wx.mul_add(z, view_x.get());
                    let sy = wy.mul_add(z, view_y.get());
                    view! {
                        <input
                            style=format!(
                                "position:absolute; left:{}px; top:{}px; font-size:{}px; font-weight:{}; font-style:{}; color:{}; background:{}; border:1px dashed var(--primary); padding:2px 4px; z-index:10; min-width:120px;",
                                sx, sy, (18.0 * z).max(10.0),
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
fn screen_to_world(
    x: f64,
    y: f64,
    zoom: f64,
    vx: f64,
    vy: f64,
) -> (f64, f64) {
    ((x - vx) / zoom, (y - vy) / zoom)
}

/// What a mouse/touch drag is currently doing -- unified across pen/eraser (freehand), shapes
/// (rubber-band preview from a fixed start point to the live cursor), and pan (drag the view),
/// so all three share one set of start/move/end handlers instead of triplicating the event
/// wiring the way three separate tools otherwise would.
#[cfg(feature = "hydrate")]
enum DragState {
    None,
    Stroke {
        color: String,
        width: f64,
        alpha: f64,
        points: Vec<(f64, f64)>,
    },
    Shape {
        kind: String,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
    },
    Pan {
        start_screen: (f64, f64),
        start_view: (f64, f64),
    },
}

#[cfg(feature = "hydrate")]
#[allow(clippy::too_many_arguments)]
fn wire_canvas(
    canvas_ref: NodeRef<leptos::html::Canvas>,
    mode: RwSignal<String>,
    color: RwSignal<String>,
    fill_enabled: RwSignal<bool>,
    fill_color: RwSignal<String>,
    brush_width: RwSignal<f64>,
    brush: RwSignal<String>,
    zoom: RwSignal<f64>,
    view_x: RwSignal<f64>,
    view_y: RwSignal<f64>,
    elements: StoredValue<Vec<Element>>,
    pending_text: RwSignal<Option<(f64, f64)>>,
    push_undo: Box<dyn Fn()>,
) {
    use wasm_bindgen::JsCast;
    let push_undo = std::rc::Rc::new(push_undo);

    Effect::new(move |_| {
        let Some(canvas) = canvas_ref.get() else {
            return;
        };
        redraw_all(canvas_ref, elements, zoom, view_x, view_y);

        let drag = std::rc::Rc::new(std::cell::RefCell::new(DragState::None));

        let pos_from_mouse =
            |ev: &web_sys::MouseEvent, canvas: &web_sys::HtmlCanvasElement| -> (f64, f64) {
                let rect = canvas.get_bounding_client_rect();
                (
                    ev.client_x() as f64 - rect.left(),
                    ev.client_y() as f64 - rect.top(),
                )
            };
        let pos_from_touch =
            |ev: &web_sys::TouchEvent, canvas: &web_sys::HtmlCanvasElement| -> Option<(f64, f64)> {
                let touch = ev.touches().get(0)?;
                let rect = canvas.get_bounding_client_rect();
                Some((
                    touch.client_x() as f64 - rect.left(),
                    touch.client_y() as f64 - rect.top(),
                ))
            };

        let start_action = {
            let drag = drag.clone();
            move |sx: f64, sy: f64| {
                let m = mode.get_untracked();
                let z = zoom.get_untracked();
                let vx = view_x.get_untracked();
                let vy = view_y.get_untracked();
                if m == "text" {
                    let (wx, wy) = screen_to_world(sx, sy, z, vx, vy);
                    pending_text.set(Some((wx, wy)));
                    return;
                }
                if m == "pan" {
                    *drag.borrow_mut() = DragState::Pan {
                        start_screen: (sx, sy),
                        start_view: (vx, vy),
                    };
                    return;
                }
                let (wx, wy) = screen_to_world(sx, sy, z, vx, vy);
                if matches!(m.as_str(), "rect" | "ellipse" | "line" | "arrow") {
                    *drag.borrow_mut() = DragState::Shape {
                        kind: m,
                        x1: wx,
                        y1: wy,
                        x2: wx,
                        y2: wy,
                    };
                    return;
                }
                let is_eraser = m == "eraser";
                let (w, a) = if is_eraser {
                    (brush_width.get_untracked().max(12.0), 1.0)
                } else {
                    brush_preset(&brush.get_untracked())
                };
                let width = if is_eraser {
                    w
                } else {
                    brush_width.get_untracked()
                };
                *drag.borrow_mut() = DragState::Stroke {
                    color: if is_eraser {
                        "#ffffff".to_string()
                    } else {
                        color.get_untracked()
                    },
                    width,
                    alpha: if is_eraser { 1.0 } else { a },
                    points: vec![(wx, wy)],
                };
            }
        };

        let move_action = {
            let drag = drag.clone();
            let canvas = canvas.clone();
            move |sx: f64, sy: f64| {
                let z = zoom.get_untracked();
                match &mut *drag.borrow_mut() {
                    | DragState::None => {},
                    | DragState::Pan {
                        start_screen,
                        start_view,
                    } => {
                        view_x.set(start_view.0 + (sx - start_screen.0));
                        view_y.set(start_view.1 + (sy - start_screen.1));
                        redraw_all(canvas_ref, elements, zoom, view_x, view_y);
                    },
                    | DragState::Stroke {
                        points,
                        color,
                        width,
                        alpha,
                    } => {
                        let (wx, wy) = screen_to_world(
                            sx,
                            sy,
                            z,
                            view_x.get_untracked(),
                            view_y.get_untracked(),
                        );
                        points.push((wx, wy));
                        let n = points.len();
                        if n >= 2 {
                            let ctx = canvas_context(&canvas);
                            ctx.save();
                            let _ = ctx.translate(view_x.get_untracked(), view_y.get_untracked());
                            let _ = ctx.scale(z, z);
                            paint_segment(
                                &ctx,
                                color,
                                *width,
                                *alpha,
                                points[n - 2],
                                points[n - 1],
                            );
                            ctx.restore();
                        }
                    },
                    | DragState::Shape { kind, x1, y1, x2, y2 } => {
                        let (wx, wy) = screen_to_world(
                            sx,
                            sy,
                            z,
                            view_x.get_untracked(),
                            view_y.get_untracked(),
                        );
                        *x2 = wx;
                        *y2 = wy;
                        redraw_all(canvas_ref, elements, zoom, view_x, view_y);
                        let ctx = canvas_context(&canvas);
                        ctx.save();
                        let _ = ctx.translate(view_x.get_untracked(), view_y.get_untracked());
                        let _ = ctx.scale(z, z);
                        let fill = fill_enabled
                            .get_untracked()
                            .then(|| fill_color.get_untracked());
                        paint_shape(
                            &ctx,
                            kind,
                            *x1,
                            *y1,
                            *x2,
                            *y2,
                            &color.get_untracked(),
                            fill.as_deref(),
                            brush_width.get_untracked(),
                        );
                        ctx.restore();
                    },
                }
            }
        };

        let end_action = {
            let drag = drag.clone();
            let push_undo = push_undo.clone();
            move || {
                let taken = std::mem::replace(&mut *drag.borrow_mut(), DragState::None);
                match taken {
                    | DragState::None | DragState::Pan { .. } => {},
                    | DragState::Stroke {
                        color,
                        width,
                        alpha,
                        points,
                    } => {
                        if points.len() > 1 {
                            push_undo();
                            elements.update_value(|e| {
                                e.push(Element::Stroke {
                                    color,
                                    width,
                                    alpha,
                                    points,
                                })
                            });
                        }
                    },
                    | DragState::Shape { kind, x1, y1, x2, y2 } => {
                        if (x1 - x2).abs() > 1.0 || (y1 - y2).abs() > 1.0 {
                            push_undo();
                            let fill = fill_enabled
                                .get_untracked()
                                .then(|| fill_color.get_untracked());
                            elements.update_value(|e| {
                                e.push(Element::Shape {
                                    shape: kind,
                                    x1,
                                    y1,
                                    x2,
                                    y2,
                                    color: color.get_untracked(),
                                    fill,
                                    width: brush_width.get_untracked(),
                                })
                            });
                            redraw_all(canvas_ref, elements, zoom, view_x, view_y);
                        } else {
                            redraw_all(canvas_ref, elements, zoom, view_x, view_y);
                        }
                    },
                }
            }
        };

        let canvas_el: web_sys::HtmlCanvasElement = canvas.clone();

        let md_canvas = canvas_el.clone();
        let start_action_md = start_action.clone();
        let on_mousedown = wasm_bindgen::closure::Closure::<dyn Fn(web_sys::MouseEvent)>::new(
            move |ev: web_sys::MouseEvent| {
                let (x, y) = pos_from_mouse(&ev, &md_canvas);
                start_action_md(x, y);
            },
        );
        canvas_el
            .add_event_listener_with_callback("mousedown", on_mousedown.as_ref().unchecked_ref())
            .ok();
        on_mousedown.forget();

        let mm_canvas = canvas_el.clone();
        let move_action_mm = move_action.clone();
        let on_mousemove = wasm_bindgen::closure::Closure::<dyn Fn(web_sys::MouseEvent)>::new(
            move |ev: web_sys::MouseEvent| {
                let (x, y) = pos_from_mouse(&ev, &mm_canvas);
                move_action_mm(x, y);
            },
        );
        canvas_el
            .add_event_listener_with_callback("mousemove", on_mousemove.as_ref().unchecked_ref())
            .ok();
        on_mousemove.forget();

        let end_action_mu = end_action.clone();
        let on_mouseup = wasm_bindgen::closure::Closure::<dyn Fn(web_sys::MouseEvent)>::new(
            move |_ev: web_sys::MouseEvent| {
                end_action_mu();
            },
        );
        if let Some(win) = web_sys::window() {
            win.add_event_listener_with_callback("mouseup", on_mouseup.as_ref().unchecked_ref())
                .ok();
        }
        on_mouseup.forget();

        let wheel_canvas = canvas_el.clone();
        let on_wheel = wasm_bindgen::closure::Closure::<dyn Fn(web_sys::WheelEvent)>::new(
            move |ev: web_sys::WheelEvent| {
                ev.prevent_default();
                let factor = if ev.delta_y() < 0.0 {
                    1.1
                } else {
                    1.0 / 1.1
                };
                zoom.update(|z| *z = (*z * factor).clamp(0.25, 4.0));
                redraw_all(canvas_ref, elements, zoom, view_x, view_y);
            },
        );
        wheel_canvas
            .add_event_listener_with_callback("wheel", on_wheel.as_ref().unchecked_ref())
            .ok();
        on_wheel.forget();

        let ts_canvas = canvas_el.clone();
        let start_action_ts = start_action.clone();
        let on_touchstart = wasm_bindgen::closure::Closure::<dyn Fn(web_sys::TouchEvent)>::new(
            move |ev: web_sys::TouchEvent| {
                ev.prevent_default();
                if let Some((x, y)) = pos_from_touch(&ev, &ts_canvas) {
                    start_action_ts(x, y);
                }
            },
        );
        canvas_el
            .add_event_listener_with_callback("touchstart", on_touchstart.as_ref().unchecked_ref())
            .ok();
        on_touchstart.forget();

        let tm_canvas = canvas_el.clone();
        let move_action_tm = move_action.clone();
        let on_touchmove = wasm_bindgen::closure::Closure::<dyn Fn(web_sys::TouchEvent)>::new(
            move |ev: web_sys::TouchEvent| {
                ev.prevent_default();
                if let Some((x, y)) = pos_from_touch(&ev, &tm_canvas) {
                    move_action_tm(x, y);
                }
            },
        );
        canvas_el
            .add_event_listener_with_callback("touchmove", on_touchmove.as_ref().unchecked_ref())
            .ok();
        on_touchmove.forget();

        let end_action_te = end_action.clone();
        let on_touchend = wasm_bindgen::closure::Closure::<dyn Fn(web_sys::TouchEvent)>::new(
            move |_ev: web_sys::TouchEvent| {
                end_action_te();
            },
        );
        canvas_el
            .add_event_listener_with_callback("touchend", on_touchend.as_ref().unchecked_ref())
            .ok();
        on_touchend.forget();
    });
}

#[cfg(not(feature = "hydrate"))]
#[allow(clippy::too_many_arguments)]
fn wire_canvas(
    _canvas_ref: NodeRef<leptos::html::Canvas>,
    _mode: RwSignal<String>,
    _color: RwSignal<String>,
    _fill_enabled: RwSignal<bool>,
    _fill_color: RwSignal<String>,
    _brush_width: RwSignal<f64>,
    _brush: RwSignal<String>,
    _zoom: RwSignal<f64>,
    _view_x: RwSignal<f64>,
    _view_y: RwSignal<f64>,
    _elements: StoredValue<Vec<Element>>,
    _pending_text: RwSignal<Option<(f64, f64)>>,
    _push_undo: Box<dyn Fn()>,
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
fn paint_segment(
    ctx: &web_sys::CanvasRenderingContext2d,
    color: &str,
    width: f64,
    alpha: f64,
    from: (f64, f64),
    to: (f64, f64),
) {
    ctx.set_global_alpha(alpha);
    ctx.set_stroke_style_str(color);
    ctx.set_line_width(width);
    ctx.set_line_cap("round");
    ctx.set_line_join("round");
    ctx.begin_path();
    ctx.move_to(from.0, from.1);
    ctx.line_to(to.0, to.1);
    ctx.stroke();
    ctx.set_global_alpha(1.0);
}

#[cfg(feature = "hydrate")]
#[allow(clippy::too_many_arguments)]
fn paint_shape(
    ctx: &web_sys::CanvasRenderingContext2d,
    kind: &str,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    color: &str,
    fill: Option<&str>,
    width: f64,
) {
    ctx.set_stroke_style_str(color);
    ctx.set_line_width(width);
    ctx.set_line_cap("round");
    ctx.set_line_join("round");
    match kind {
        | "rect" => {
            let (rx, ry) = (x1.min(x2), y1.min(y2));
            let (rw, rh) = ((x1 - x2).abs(), (y1 - y2).abs());
            if let Some(f) = fill {
                ctx.set_fill_style_str(f);
                ctx.fill_rect(rx, ry, rw, rh);
            }
            ctx.stroke_rect(rx, ry, rw, rh);
        },
        | "ellipse" => {
            let cx = (x1 + x2) / 2.0;
            let cy = (y1 + y2) / 2.0;
            let rx = ((x1 - x2).abs() / 2.0).max(0.01);
            let ry = ((y1 - y2).abs() / 2.0).max(0.01);
            ctx.begin_path();
            let _ = ctx.ellipse(cx, cy, rx, ry, 0.0, 0.0, std::f64::consts::PI * 2.0);
            if let Some(f) = fill {
                ctx.set_fill_style_str(f);
                ctx.fill();
            }
            ctx.stroke();
        },
        | "line" => {
            ctx.begin_path();
            ctx.move_to(x1, y1);
            ctx.line_to(x2, y2);
            ctx.stroke();
        },
        | "arrow" => {
            ctx.begin_path();
            ctx.move_to(x1, y1);
            ctx.line_to(x2, y2);
            ctx.stroke();
            let angle = (y2 - y1).atan2(x2 - x1);
            let head_len = (width * 4.0).max(10.0);
            let a1 = angle + std::f64::consts::PI - 0.4;
            let a2 = angle + std::f64::consts::PI + 0.4;
            ctx.begin_path();
            ctx.move_to(x2, y2);
            ctx.line_to(x2 + head_len * a1.cos(), y2 + head_len * a1.sin());
            ctx.move_to(x2, y2);
            ctx.line_to(x2 + head_len * a2.cos(), y2 + head_len * a2.sin());
            ctx.stroke();
        },
        | _ => {},
    }
}

/// Draws every element onto the canvas in order, under the current pan/zoom transform -- called
/// on initial mount (loading whatever was last saved), after any committed change, and on every
/// pan/zoom tick, so the visible canvas never drifts from the `elements` model or the current
/// view.
#[cfg(feature = "hydrate")]
fn redraw_all(
    canvas_ref: NodeRef<leptos::html::Canvas>,
    elements: StoredValue<Vec<Element>>,
    zoom: RwSignal<f64>,
    view_x: RwSignal<f64>,
    view_y: RwSignal<f64>,
) {
    let Some(canvas) = canvas_ref.get_untracked() else {
        return;
    };
    let ctx = canvas_context(&canvas);
    ctx.clear_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);
    ctx.save();
    let z = zoom.get_untracked();
    let _ = ctx.translate(view_x.get_untracked(), view_y.get_untracked());
    let _ = ctx.scale(z, z);
    elements.with_value(|all| {
        for el in all {
            match el {
                | Element::Stroke {
                    color,
                    width,
                    alpha,
                    points,
                } => {
                    if points.len() < 2 {
                        continue;
                    }
                    ctx.set_global_alpha(*alpha);
                    ctx.set_stroke_style_str(color);
                    ctx.set_line_width(*width);
                    ctx.set_line_cap("round");
                    ctx.set_line_join("round");
                    ctx.begin_path();
                    ctx.move_to(points[0].0, points[0].1);
                    for p in &points[1..] {
                        ctx.line_to(p.0, p.1);
                    }
                    ctx.stroke();
                    ctx.set_global_alpha(1.0);
                },
                | Element::Shape {
                    shape,
                    x1,
                    y1,
                    x2,
                    y2,
                    color,
                    fill,
                    width,
                } => {
                    paint_shape(
                        &ctx,
                        shape,
                        *x1,
                        *y1,
                        *x2,
                        *y2,
                        color,
                        fill.as_deref(),
                        *width,
                    );
                },
                | Element::Text { .. } => paint_text(&ctx, el),
                | Element::Image { x, y, w, h, data_url } => {
                    paint_image(&ctx, *x, *y, *w, *h, data_url);
                },
            }
        }
    });
    ctx.restore();
}

#[cfg(not(feature = "hydrate"))]
const fn redraw_all(
    _canvas_ref: NodeRef<leptos::html::Canvas>,
    _elements: StoredValue<Vec<Element>>,
    _zoom: RwSignal<f64>,
    _view_x: RwSignal<f64>,
    _view_y: RwSignal<f64>,
) {
}

#[cfg(feature = "hydrate")]
fn paint_text(
    ctx: &web_sys::CanvasRenderingContext2d,
    el: &Element,
) {
    let Element::Text {
        x,
        y,
        text,
        color,
        bg_color,
        bold,
        italic,
        font_size,
    } = el
    else {
        return;
    };
    let weight = if *bold { "700" } else { "400" };
    let style = if *italic {
        "italic"
    } else {
        "normal"
    };
    ctx.set_font(&format!("{} {} {}px sans-serif", style, weight, font_size));
    ctx.set_text_baseline("top");
    if let Some(bg) = bg_color {
        if let Ok(metrics) = ctx.measure_text(text) {
            let w = metrics.width();
            let pad = 4.0;
            ctx.set_fill_style_str(bg);
            ctx.fill_rect(x - pad, y - pad, w + pad * 2.0, font_size + pad * 2.0);
        }
    }
    ctx.set_fill_style_str(color);
    let _ = ctx.fill_text(text, *x, *y);
}

/// Renders a saved data-URL image element onto the canvas. `HtmlImageElement::new` decodes
/// asynchronously (the browser fetches/decodes the data URL off the main thread's current call
/// stack), so this can't draw synchronously inline with the rest of `redraw_all` -- it kicks off
/// the decode and redraws just this one image once it completes, which is a harmless extra paint
/// given how infrequently this actually fires (once per image element per `redraw_all` call).
#[cfg(feature = "hydrate")]
fn paint_image(
    ctx: &web_sys::CanvasRenderingContext2d,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    data_url: &str,
) {
    use wasm_bindgen::JsCast;
    let Ok(img) = web_sys::HtmlImageElement::new() else {
        return;
    };
    img.set_src(data_url);
    let ctx = ctx.clone();
    let img_for_closure = img.clone();
    let onload = wasm_bindgen::closure::Closure::<dyn Fn()>::new(move || {
        let _ = ctx.draw_image_with_html_image_element_and_dw_and_dh(&img_for_closure, x, y, w, h);
    });
    img.set_onload(Some(onload.as_ref().unchecked_ref()));
    onload.forget();
}

#[cfg(feature = "hydrate")]
#[allow(clippy::too_many_arguments)]
fn clear_handler(
    canvas_ref: NodeRef<leptos::html::Canvas>,
    elements: StoredValue<Vec<Element>>,
    zoom: RwSignal<f64>,
    view_x: RwSignal<f64>,
    view_y: RwSignal<f64>,
    push_undo: Box<dyn Fn()>,
    can_undo: RwSignal<bool>,
    can_redo: RwSignal<bool>,
    redo_stack: StoredValue<Vec<Vec<Element>>>,
) -> impl Fn(leptos::ev::MouseEvent) + Clone + 'static {
    let push_undo = std::rc::Rc::new(push_undo);
    move |_| {
        let confirmed = web_sys::window()
            .and_then(|w| w.confirm_with_message("Clear whiteboard?").ok())
            .unwrap_or(false);
        if !confirmed {
            return;
        }
        push_undo();
        can_undo.set(true);
        redo_stack.update_value(|s| s.clear());
        can_redo.set(false);
        elements.update_value(|e| e.clear());
        redraw_all(canvas_ref, elements, zoom, view_x, view_y);
    }
}

#[cfg(not(feature = "hydrate"))]
#[allow(clippy::too_many_arguments)]
fn clear_handler(
    _canvas_ref: NodeRef<leptos::html::Canvas>,
    _elements: StoredValue<Vec<Element>>,
    _zoom: RwSignal<f64>,
    _view_x: RwSignal<f64>,
    _view_y: RwSignal<f64>,
    _push_undo: Box<dyn Fn()>,
    _can_undo: RwSignal<bool>,
    _can_redo: RwSignal<bool>,
    _redo_stack: StoredValue<Vec<Vec<Element>>>,
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
        let Some(el) = canvas_ref.get_untracked() else {
            return;
        };
        let Ok(data_url) = el.to_data_url() else {
            return;
        };
        let Some(document) = web_sys::window().and_then(|w| w.document()) else {
            return;
        };
        let Ok(anchor) = document.create_element("a") else {
            return;
        };
        let Ok(anchor) = anchor.dyn_into::<web_sys::HtmlAnchorElement>() else {
            return;
        };
        anchor.set_href(&data_url);
        anchor.set_download(&format!(
            "whiteboard-{}.png",
            file_path.replace(['/', '\\'], "_")
        ));
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
    elements: StoredValue<Vec<Element>>,
    save_status: RwSignal<String>,
    project_id: String,
    file_path: String,
) -> impl Fn(leptos::ev::MouseEvent) + Clone + 'static {
    move |_| {
        save_status.set("Saving...".to_string());
        let json = elements.with_value(|e| {
            serde_json::to_string(&serde_json::json!({ "elements": e }))
                .unwrap_or_else(|_| "{\"elements\":[]}".to_string())
        });
        let project_id = project_id.clone();
        let file_path = file_path.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let body = serde_json::json!({ "file": file_path, "whiteboard_json": json });
            let result =
                gloo_net::http::Request::post(&format!("/projects/{}/note/whiteboard", project_id))
                    .json(&body)
                    .expect("serializable body")
                    .send()
                    .await;
            match result {
                | Ok(resp) if resp.ok() => save_status.set("Saved".to_string()),
                | Ok(resp) => save_status.set(format!("Save failed ({})", resp.status())),
                | Err(e) => save_status.set(format!("Save failed: {e}")),
            }
        });
    }
}

#[cfg(not(feature = "hydrate"))]
fn save_handler(
    _elements: StoredValue<Vec<Element>>,
    _save_status: RwSignal<String>,
    _project_id: String,
    _file_path: String,
) -> impl Fn(leptos::ev::MouseEvent) + Clone + 'static {
    move |_| {}
}

#[cfg(feature = "hydrate")]
fn save_asset_handler(
    canvas_ref: NodeRef<leptos::html::Canvas>,
    save_status: RwSignal<String>,
    project_id: String,
    file_path: String,
) -> impl Fn(leptos::ev::MouseEvent) + Clone + 'static {
    move |_| {
        let Some(canvas) = canvas_ref.get_untracked() else {
            return;
        };
        let Ok(data_url) = canvas.to_data_url() else {
            save_status.set("Failed to capture whiteboard image".to_string());
            return;
        };

        let default_name = {
            let stem = std::path::Path::new(&file_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("notes");
            format!("whiteboard-{stem}.png")
        };

        let asset_name = web_sys::window()
            .and_then(|w| {
                w.prompt_with_message_and_default(
                    "Save whiteboard image to assets folder as:\n(Can be referenced in notes, Typst, LaTeX, etc.)",
                    &default_name,
                )
                .ok()
            })
            .flatten();

        let Some(name) = asset_name else {
            return;
        };

        let name = name.trim().to_string();
        if name.is_empty() {
            return;
        }

        save_status.set("Saving to assets...".to_string());
        let project_id = project_id.clone();
        let file_path = file_path.clone();

        wasm_bindgen_futures::spawn_local(async move {
            let body = serde_json::json!({
                "file": file_path,
                "asset_name": name,
                "data_url": data_url
            });
            let result = gloo_net::http::Request::post(&format!(
                "/projects/{}/note/whiteboard/save-asset",
                project_id
            ))
            .json(&body)
            .expect("serializable body")
            .send()
            .await;

            match result {
                | Ok(resp) if resp.ok() => {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        let path = data
                            .get("asset_path")
                            .and_then(|v| v.as_str())
                            .unwrap_or("assets/");
                        save_status.set(format!("✓ Saved to {path}"));
                    } else {
                        save_status.set("✓ Saved to assets".to_string());
                    }
                }
                | Ok(resp) => save_status.set(format!("Asset save failed ({})", resp.status())),
                | Err(e) => save_status.set(format!("Asset save failed: {e}")),
            }
        });
    }
}

#[cfg(not(feature = "hydrate"))]
fn save_asset_handler(
    _canvas_ref: NodeRef<leptos::html::Canvas>,
    _save_status: RwSignal<String>,
    _project_id: String,
    _file_path: String,
) -> impl Fn(leptos::ev::MouseEvent) + Clone + 'static {
    move |_| {}
}

/// Placed images cap at this on-canvas size (scaled down, aspect preserved) -- an imported photo
/// straight off a phone can be 4000px+ on a side, which would swamp a 1100x600 board and make the
/// data URL (embedded directly in the saved JSON, no separate asset store for whiteboards exists
/// yet) unnecessarily large.
#[cfg(feature = "hydrate")]
const MAX_PLACED_DIM: f64 = 400.0;

#[cfg(feature = "hydrate")]
fn image_file_handler(
    canvas_ref: NodeRef<leptos::html::Canvas>,
    elements: StoredValue<Vec<Element>>,
    zoom: RwSignal<f64>,
    view_x: RwSignal<f64>,
    view_y: RwSignal<f64>,
    push_undo: Box<dyn Fn()>,
) -> impl Fn(leptos::ev::Event) + 'static {
    use wasm_bindgen::JsCast;
    let push_undo = std::rc::Rc::new(push_undo);
    move |ev: leptos::ev::Event| {
        let Some(input) = ev
            .target()
            .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
        else {
            return;
        };
        let Some(files) = input.files() else { return };
        let Some(file) = files.get(0) else { return };
        let Ok(reader) = web_sys::FileReader::new() else {
            return;
        };
        let reader_for_closure = reader.clone();
        let push_undo = push_undo.clone();
        let onload = wasm_bindgen::closure::Closure::<dyn Fn()>::new(move || {
            let Ok(result) = reader_for_closure.result() else {
                return;
            };
            let Some(data_url) = result.as_string() else {
                return;
            };
            place_image_from_data_url(
                canvas_ref,
                elements,
                zoom,
                view_x,
                view_y,
                push_undo.clone(),
                data_url,
            );
        });
        reader.set_onload(Some(onload.as_ref().unchecked_ref()));
        onload.forget();
        let _ = reader.read_as_data_url(&file);
        input.set_value("");
    }
}
#[cfg(not(feature = "hydrate"))]
fn image_file_handler(
    _canvas_ref: NodeRef<leptos::html::Canvas>,
    _elements: StoredValue<Vec<Element>>,
    _zoom: RwSignal<f64>,
    _view_x: RwSignal<f64>,
    _view_y: RwSignal<f64>,
    _push_undo: Box<dyn Fn()>,
) -> impl Fn(leptos::ev::Event) + 'static {
    move |_ev: leptos::ev::Event| {}
}

/// Decodes the data URL once (to get real natural dimensions to scale from -- placing at a
/// hardcoded size would stretch/squash non-square imports), then adds it as a real `Element` and
/// redraws.
#[cfg(feature = "hydrate")]
fn place_image_from_data_url(
    canvas_ref: NodeRef<leptos::html::Canvas>,
    elements: StoredValue<Vec<Element>>,
    zoom: RwSignal<f64>,
    view_x: RwSignal<f64>,
    view_y: RwSignal<f64>,
    push_undo: std::rc::Rc<Box<dyn Fn()>>,
    data_url: String,
) {
    use wasm_bindgen::JsCast;
    let Ok(img) = web_sys::HtmlImageElement::new() else {
        return;
    };
    img.set_src(&data_url);
    let img_for_closure = img.clone();
    let onload = wasm_bindgen::closure::Closure::<dyn Fn()>::new(move || {
        let natural_w = img_for_closure.natural_width().max(1) as f64;
        let natural_h = img_for_closure.natural_height().max(1) as f64;
        let scale = (MAX_PLACED_DIM / natural_w)
            .min(MAX_PLACED_DIM / natural_h)
            .min(1.0);
        let (w, h) = (natural_w * scale, natural_h * scale);
        let (x, y) = screen_to_world(
            120.0,
            80.0,
            zoom.get_untracked(),
            view_x.get_untracked(),
            view_y.get_untracked(),
        );
        push_undo();
        elements.update_value(|e| {
            e.push(Element::Image {
                x,
                y,
                w,
                h,
                data_url: data_url.clone(),
            })
        });
        redraw_all(canvas_ref, elements, zoom, view_x, view_y);
    });
    img.set_onload(Some(onload.as_ref().unchecked_ref()));
    onload.forget();
}

#[cfg(feature = "hydrate")]
fn video_file_handler(
    canvas_ref: NodeRef<leptos::html::Canvas>,
    elements: StoredValue<Vec<Element>>,
    zoom: RwSignal<f64>,
    view_x: RwSignal<f64>,
    view_y: RwSignal<f64>,
    push_undo: Box<dyn Fn()>,
) -> impl Fn(leptos::ev::Event) + 'static {
    use wasm_bindgen::JsCast;
    let push_undo = std::rc::Rc::new(push_undo);
    move |ev: leptos::ev::Event| {
        let Some(input) = ev
            .target()
            .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
        else {
            return;
        };
        let Some(files) = input.files() else { return };
        let Some(file) = files.get(0) else { return };
        let Ok(object_url) = web_sys::Url::create_object_url_with_blob(&file) else {
            return;
        };
        let Some(document) = web_sys::window().and_then(|w| w.document()) else {
            return;
        };
        let Ok(video) = document.create_element("video") else {
            return;
        };
        let Ok(video) = video.dyn_into::<web_sys::HtmlVideoElement>() else {
            return;
        };
        video.set_src(&object_url);
        video.set_muted(true);
        let video_for_seek = video.clone();
        let object_url_for_cleanup = object_url.clone();
        let on_loadeddata = wasm_bindgen::closure::Closure::<dyn Fn()>::new(move || {
            // Seeking (even to 0, its already-loaded position) forces a `seeked` event once a
            // real decoded frame is actually available to draw -- `loadeddata` alone can fire
            // before the first frame is paintable on some codecs, confirmed a blank/black frame
            // without this extra step.
            video_for_seek.set_current_time(0.01);
        });
        video.set_onloadeddata(Some(on_loadeddata.as_ref().unchecked_ref()));
        on_loadeddata.forget();

        let video_for_seeked = video.clone();
        let canvas_ref = canvas_ref;
        let push_undo = push_undo.clone();
        let on_seeked = wasm_bindgen::closure::Closure::<dyn Fn()>::new(move || {
            let vw = video_for_seeked.video_width().max(1) as f64;
            let vh = video_for_seeked.video_height().max(1) as f64;
            let Some(document) = web_sys::window().and_then(|w| w.document()) else {
                return;
            };
            let Ok(off_canvas) = document.create_element("canvas") else {
                return;
            };
            let Ok(off_canvas) = off_canvas.dyn_into::<web_sys::HtmlCanvasElement>() else {
                return;
            };
            off_canvas.set_width(vw as u32);
            off_canvas.set_height(vh as u32);
            let off_ctx = canvas_context(&off_canvas);
            if off_ctx
                .draw_image_with_html_video_element(&video_for_seeked, 0.0, 0.0)
                .is_ok()
            {
                if let Ok(data_url) = off_canvas.to_data_url() {
                    place_image_from_data_url(
                        canvas_ref,
                        elements,
                        zoom,
                        view_x,
                        view_y,
                        push_undo.clone(),
                        data_url,
                    );
                }
            }
            let _ = web_sys::Url::revoke_object_url(&object_url_for_cleanup);
        });
        video.set_onseeked(Some(on_seeked.as_ref().unchecked_ref()));
        on_seeked.forget();

        input.set_value("");
    }
}
#[cfg(not(feature = "hydrate"))]
fn video_file_handler(
    _canvas_ref: NodeRef<leptos::html::Canvas>,
    _elements: StoredValue<Vec<Element>>,
    _zoom: RwSignal<f64>,
    _view_x: RwSignal<f64>,
    _view_y: RwSignal<f64>,
    _push_undo: Box<dyn Fn()>,
) -> impl Fn(leptos::ev::Event) + 'static {
    move |_ev: leptos::ev::Event| {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_elements_reads_current_format() {
        let json = r##"{"elements":[{"kind":"Text","x":1.0,"y":2.0,"text":"hi","color":"#000","bg_color":null,"bold":false,"italic":false,"font_size":18.0}]}"##;
        let els = load_elements(json);
        assert_eq!(els.len(), 1);
        assert!(matches!(els.first(), Some(Element::Text { .. })));
    }

    #[test]
    fn test_load_elements_upgrades_old_strokes_and_texts_format() {
        let json = r##"{"strokes":[{"color":"#111","width":3.0,"points":[[0.0,0.0],[1.0,1.0]]}],"texts":[{"x":5.0,"y":5.0,"text":"note","color":"#222","bg_color":null,"bold":true,"italic":false,"font_size":18.0}]}"##;
        let els = load_elements(json);
        assert_eq!(els.len(), 2);
        assert!(matches!(els.first(), Some(Element::Stroke { color, .. }) if color == "#111"));
        assert!(
            matches!(els.get(1), Some(Element::Text { text, bold: true, .. }) if text == "note")
        );
    }

    #[test]
    fn test_load_elements_empty_or_malformed_is_empty() {
        assert!(load_elements("").is_empty());
        assert!(load_elements("not json").is_empty());
        assert!(load_elements("{}").is_empty());
    }

    #[test]
    fn test_brush_presets_have_distinct_widths() {
        let (pen_w, _) = brush_preset("pen");
        let (marker_w, _) = brush_preset("marker");
        let (highlighter_w, highlighter_a) = brush_preset("highlighter");
        assert!(pen_w < marker_w);
        assert!(marker_w < highlighter_w);
        assert!(highlighter_a < 1.0, "highlighter should be translucent");
    }
}
