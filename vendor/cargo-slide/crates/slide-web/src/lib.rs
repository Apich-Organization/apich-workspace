//! Standalone Leptos CSR presentation player with full feature parity with native slide-viewer:
//! - Pure Rust WebAssembly (Zero inline JavaScript)
//! - True Fullscreen & letterboxed viewport
//! - 13 Slide Transitions & In-Slide Component Steps
//! - Interactive Hotspots (Internal Links, External Links, Audio, Video, Charts)
//! - Full Interactive HUD Data Inspector (Bar/Line/Area/Scatter, Transforms, Table, CSV Export)
//! - Presenter Tools (Laser Pointer, 7-Color Whiteboard Pen, Audio Engine, Volume Toast)
//! - Modern Glassmorphism Dock, Overview Grid Modal, and Keyboard Help Modal.

pub mod chart;
pub mod highlight;
pub mod model;

use chart::TransformPreset;
use chart::apply_transform;
use chart::calculate_stats;
use chart::export_csv_string;
use chart::parse_filter_condition;
use chart::render_svg_chart;
use leptos::prelude::*;
use model::ChartData;
use model::ChartType;
use model::Hotspot;
use model::Rect;
use model::SlideDeck;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use web_sys::HtmlAudioElement;

pub const PEN_COLORS: [&str; 14] = [
    "#00E5FF", // Cyan
    "#FF3366", // Red
    "#00E676", // Green
    "#FFD600", // Yellow
    "#D500F9", // Purple
    "#FFFFFF", // White
    "#FF6D00", // Orange
    "#FF4081", // Pink
    "#38BDF8", // Sky
    "#76FF03", // Lime
    "#64FFDA", // Mint
    "#FFAB00", // Gold
    "#7C4DFF", // Violet
    "#263238", // Charcoal
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrushType {
    Pen,
    Highlighter,
    Neon,
    Arrow,
}

impl BrushType {
    pub const ALL: [BrushType; 4] = [
        BrushType::Pen,
        BrushType::Highlighter,
        BrushType::Neon,
        BrushType::Arrow,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            | Self::Pen => "Pen",
            | Self::Highlighter => "Highlighter",
            | Self::Neon => "Neon",
            | Self::Arrow => "Arrow",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            | Self::Pen => "✏️ Pen",
            | Self::Highlighter => "🖍️ Highlighter",
            | Self::Neon => "⚡ Neon",
            | Self::Arrow => "➔ Arrow",
        }
    }

    pub fn cycle(&self) -> Self {
        match self {
            | Self::Pen => Self::Highlighter,
            | Self::Highlighter => Self::Neon,
            | Self::Neon => Self::Arrow,
            | Self::Arrow => Self::Pen,
        }
    }
}

#[derive(Clone, Debug)]
struct PenStroke {
    points: Vec<(f64, f64)>,
    color: String,
    brush_type: BrushType,
    width: f64,
}

fn render_stroke_path(s: PenStroke) -> AnyView {
    let d = s
        .points
        .iter()
        .enumerate()
        .map(|(i, (x, y))| {
            if i == 0 {
                format!("M {x:.1} {y:.1}")
            } else {
                format!(" L {x:.1} {y:.1}")
            }
        })
        .collect::<Vec<_>>()
        .join("");

    let w = s.width;
    let color = s.color;

    match s.brush_type {
        BrushType::Pen => {
            view! {
                <g>
                    <path d=d fill="none" stroke=color stroke-width=w.to_string() stroke-linecap="round" stroke-linejoin="round"/>
                </g>
            }.into_any()
        }
        BrushType::Highlighter => {
            let hw = (w * 3.5).max(12.0);
            view! {
                <g>
                    <path d=d fill="none" stroke=color stroke-width=hw.to_string() stroke-opacity="0.38" stroke-linecap="butt" stroke-linejoin="miter" style="mix-blend-mode: multiply;"/>
                </g>
            }.into_any()
        }
        BrushType::Neon => {
            let glow_w = (w * 2.8).max(8.0);
            let core_w = (w * 0.9).max(2.0);
            view! {
                <g>
                    <path d=d.clone() fill="none" stroke=color.clone() stroke-width=glow_w.to_string() stroke-opacity="0.45" stroke-linecap="round" stroke-linejoin="round" style="filter: drop-shadow(0 0 6px currentColor);"/>
                    <path d=d fill="none" stroke="#ffffff" stroke-width=core_w.to_string() stroke-linecap="round" stroke-linejoin="round"/>
                </g>
            }.into_any()
        }
        BrushType::Arrow => {
            let len = s.points.len();
            let mut arrow_head = None;
            if len >= 2 {
                let (x0, y0) = s.points[len.saturating_sub(4)];
                let (x1, y1) = s.points[len - 1];
                let dx = x1 - x0;
                let dy = y1 - y0;
                let angle = dy.atan2(dx);
                let head_len = (w * 4.0).clamp(14.0, 38.0);
                let p1_x = x1 - head_len * (angle - 0.45).cos();
                let p1_y = y1 - head_len * (angle - 0.45).sin();
                let p2_x = x1 - head_len * (angle + 0.45).cos();
                let p2_y = y1 - head_len * (angle + 0.45).sin();
                let poly = format!("{x1:.1},{y1:.1} {p1_x:.1},{p1_y:.1} {p2_x:.1},{p2_y:.1}");
                arrow_head = Some(poly);
            }
            view! {
                <g>
                    <path d=d fill="none" stroke=color.clone() stroke-width=w.to_string() stroke-linecap="round" stroke-linejoin="round"/>
                    {arrow_head.map(|points| {
                        view! {
                            <polygon points=points fill=color />
                        }
                    })}
                </g>
            }.into_any()
        }
    }
}

fn create_fallback_deck() -> SlideDeck {
    SlideDeck {
        title: "Cargo Slide Presentation".to_string(),
        default_animation: "fade".to_string(),
        slides: vec![
            model::Slide {
                page_number: 1,
                view_box: Rect::new(0.0, 0.0, 1920.0, 1080.0),
                hotspots: Vec::new(),
                animation: Some("fade".to_string()),
                steps: Vec::new(),
                svg_data: r##"<svg viewBox="0 0 1920 1080" xmlns="http://www.w3.org/2000/svg">
                    <rect width="100%" height="100%" fill="#0f172a"/>
                    <rect x="160" y="120" width="1600" height="840" rx="32" fill="#1e293b" stroke="#334155" stroke-width="4"/>
                    <text x="960" y="440" font-family="system-ui, sans-serif" font-size="72" font-weight="bold" fill="#38bdf8" text-anchor="middle">Cargo Slide Web</text>
                    <text x="960" y="540" font-family="system-ui, sans-serif" font-size="36" fill="#94a3b8" text-anchor="middle">Pure Rust Leptos CSR Presentation Engine (Zero Inline JS)</text>
                    <text x="960" y="680" font-family="system-ui, sans-serif" font-size="24" fill="#64748b" text-anchor="middle">Loading deck.json... Use Arrow Keys or Space to Navigate • Press F for Fullscreen</text>
                </svg>"##.to_string(),
            },
        ],
    }
}

async fn fetch_deck_json() -> Result<SlideDeck, String> {
    use gloo_net::http::Request;
    let resp = Request::get("deck.json")
        .send()
        .await
        .map_err(|e| format!("Network request failed: {e}"))?;
    if !resp.ok() {
        return Err(format!("HTTP error {}", resp.status()));
    }
    let deck: SlideDeck = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse deck JSON: {e}"))?;
    Ok(deck)
}

fn toggle_fullscreen() {
    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
        if doc.fullscreen_element().is_some() {
            doc.exit_fullscreen();
        } else if let Some(el) = doc.get_element_by_id("presentation-root") {
            let _ = el.request_fullscreen();
        } else if let Some(el) = doc.document_element() {
            let _ = el.request_fullscreen();
        }
    }
}

fn trigger_file_download(
    filename: &str,
    content: &str,
    mime_type: &str,
) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(doc) = window.document() else {
        return;
    };
    let array = js_sys::Array::new();
    array.push(&JsValue::from_str(content));
    let options = web_sys::BlobPropertyBag::new();
    options.set_type(mime_type);
    let Ok(blob) = web_sys::Blob::new_with_str_sequence_and_options(&array, &options) else {
        return;
    };
    let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) else {
        return;
    };
    let Ok(el) = doc.create_element("a") else {
        return;
    };
    let a: web_sys::HtmlAnchorElement = el.unchecked_into();
    a.set_href(&url);
    a.set_download(filename);
    a.click();
    let _ = web_sys::Url::revoke_object_url(&url);
}

#[component]
pub fn App() -> impl IntoView {
    let deck = RwSignal::new(create_fallback_deck());
    let current_index = RwSignal::new(0usize);
    let current_step = RwSignal::new(0usize);
    let active_transition = RwSignal::new("fade".to_string());
    let transition_counter = RwSignal::new(0usize);

    // Modals
    let overview_open = RwSignal::new(false);
    let help_open = RwSignal::new(false);
    let is_blank = RwSignal::new(false);
    let active_video_modal = RwSignal::new(None::<(String, Option<String>)>);
    let active_chart_inspector = RwSignal::new(None::<ChartData>);
    let active_text_modal = RwSignal::new(None::<(String, String, bool)>); // (file_path, content, is_loading)

    // Presenter Tools: Laser & Pen
    let laser_active = RwSignal::new(false);
    let laser_coords = RwSignal::new((0.0f64, 0.0f64));
    let laser_trail = RwSignal::new(Vec::<(f64, f64, f64)>::new()); // (x, y, timestamp_ms)
    let pen_active = RwSignal::new(false);
    let pen_color_idx = RwSignal::new(0usize); // Default Cyan
    let pen_custom_color = RwSignal::new(String::from(PEN_COLORS[0]));
    let pen_brush_type = RwSignal::new(BrushType::Pen);
    let pen_width = RwSignal::new(4.0f64);
    let pen_strokes = RwSignal::new(HashMap::<usize, Vec<PenStroke>>::new());
    let is_drawing = RwSignal::new(false);
    let current_drawing_stroke = RwSignal::new(Vec::<(f64, f64)>::new());

    // Audio State
    let audio_element = RwSignal::new(None::<HtmlAudioElement>);
    let global_volume = RwSignal::new(0.8f64);
    let is_muted = RwSignal::new(false);
    let volume_toast = RwSignal::new(None::<(String, f64)>); // Message, timestamp
    let is_audio_playing = RwSignal::new(false);

    let trigger_toast = move |msg: String| {
        let now = js_sys::Date::now();
        volume_toast.set(Some((msg, now)));
        if let Some(w) = web_sys::window() {
            let cb = wasm_bindgen::closure::Closure::once(move || {
                if let Some((_, ts)) = volume_toast.get_untracked()
                    && (ts - now).abs() < 5.0
                {
                    volume_toast.set(None);
                }
            });
            let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                2200,
            );
            cb.forget();
        }
    };

    let resume_audio_if_needed = move || {
        if let Some(ref a) = audio_element
            .get()
            .filter(|a| a.paused() && is_audio_playing.get())
        {
            let _ = a.play();
        }
    };

    // Asynchronously fetch deck.json when mounted
    Effect::new(move |_| {
        wasm_bindgen_futures::spawn_local(async move {
            let Ok(loaded_deck) = fetch_deck_json().await else {
                return;
            };
            if !loaded_deck.slides.is_empty() {
                let first_max_step = loaded_deck
                    .slides
                    .first()
                    .map(|s| s.max_step())
                    .unwrap_or(0);
                current_step.set(if first_max_step > 0 { 1 } else { 0 });
                active_transition.set(loaded_deck.default_animation.clone());
                deck.set(loaded_deck);
            }
        });
    });

    let total_slides = Signal::derive(move || deck.get().slides.len());

    // Audio Engine updater
    let trigger_audio_for_slide = move |slide_idx: usize| {
        let d = deck.get();
        if let Some(slide) = d.slides.get(slide_idx) {
            let mut found_autoplay = false;
            for hs in &slide.hotspots {
                if let Hotspot::Audio {
                    source,
                    autoplay: true,
                    loop_audio,
                    volume,
                    ..
                } = hs
                {
                    found_autoplay = true;
                    // Play track
                    if let Ok(audio) = HtmlAudioElement::new_with_src(source) {
                        audio.set_loop(*loop_audio);
                        let vol = if is_muted.get() {
                            0.0
                        } else {
                            global_volume.get() * (*volume as f64)
                        };
                        audio.set_volume(vol.clamp(0.0, 1.0));
                        let _ = audio.play();
                        audio_element.set(Some(audio));
                        is_audio_playing.set(true);
                    }
                    break;
                }
            }
            if let (false, Some(prev)) = (found_autoplay, audio_element.get()) {
                let _ = prev.pause();
                audio_element.set(None);
                is_audio_playing.set(false);
            }
        }
    };

    // Navigation logic with Steps Support
    let next_step_or_slide = move || {
        let d = deck.get();
        let cur_idx = current_index.get();
        let cur_st = current_step.get();
        if let Some(slide) = d.slides.get(cur_idx) {
            let max_st = slide.max_step();
            if cur_st < max_st {
                current_step.set(cur_st + 1);
                return;
            }
        }
        // Advance slide
        let total = total_slides.get();
        if cur_idx + 1 < total {
            let next_idx = cur_idx + 1;
            let anim = d
                .slides
                .get(next_idx)
                .and_then(|s| s.animation.clone())
                .unwrap_or_else(|| d.default_animation.clone());
            active_transition.set(anim);
            current_index.set(next_idx);
            transition_counter.update(|c| *c += 1);
            let next_max = d.slides.get(next_idx).map(|s| s.max_step()).unwrap_or(0);
            current_step.set(if next_max > 0 { 1 } else { 0 });
            trigger_audio_for_slide(next_idx);
        }
    };

    let prev_step_or_slide = move || {
        let cur_st = current_step.get();
        if cur_st > 1 {
            current_step.set(cur_st - 1);
            return;
        }
        // Go back slide
        let cur_idx = current_index.get();
        if cur_idx > 0 {
            let prev_idx = cur_idx - 1;
            let d = deck.get();
            let anim = d
                .slides
                .get(prev_idx)
                .and_then(|s| s.animation.clone())
                .unwrap_or_else(|| d.default_animation.clone());
            active_transition.set(anim);
            current_index.set(prev_idx);
            transition_counter.update(|c| *c += 1);
            let prev_max = d.slides.get(prev_idx).map(|s| s.max_step()).unwrap_or(0);
            current_step.set(prev_max);
            trigger_audio_for_slide(prev_idx);
        }
    };

    let jump_to_slide = move |target_idx: usize| {
        let total = total_slides.get();
        if target_idx < total {
            let d = deck.get();
            let anim = d
                .slides
                .get(target_idx)
                .and_then(|s| s.animation.clone())
                .unwrap_or_else(|| d.default_animation.clone());
            active_transition.set(anim);
            current_index.set(target_idx);
            transition_counter.update(|c| *c += 1);
            let max_st = d.slides.get(target_idx).map(|s| s.max_step()).unwrap_or(0);
            current_step.set(if max_st > 0 { 1 } else { 0 });
            overview_open.set(false);
            trigger_audio_for_slide(target_idx);
        }
    };

    let adjust_vol = move |delta: f64| {
        let new_v = (global_volume.get() + delta).clamp(0.0, 1.0);
        global_volume.set(new_v);
        if is_muted.get() && new_v > 0.0 {
            is_muted.set(false);
        }
        if let Some(ref a) = audio_element.get() {
            a.set_volume(if is_muted.get() { 0.0 } else { new_v });
        }
        let pct = (new_v * 100.0).round() as u32;
        trigger_toast(format!("Volume: {pct}%"));
    };

    let toggle_mute = move || {
        let muted = !is_muted.get();
        is_muted.set(muted);
        if let Some(ref a) = audio_element.get() {
            a.set_volume(if muted {
                0.0
            } else {
                global_volume.get()
            });
        }
        let txt = if muted { "Muted" } else { "Unmuted" };
        trigger_toast(txt.to_string());
    };

    // Keyboard events
    window_event_listener(leptos::ev::keydown, move |ev: web_sys::KeyboardEvent| {
        resume_audio_if_needed();
        if active_chart_inspector.get().is_some()
            || active_video_modal.get().is_some()
            || active_text_modal.get().is_some()
        {
            if ev.key() == "Escape" {
                ev.prevent_default();
                active_chart_inspector.set(None);
                active_video_modal.set(None);
                active_text_modal.set(None);
            }
            return;
        }

        match ev.key().as_str() {
            | "ArrowRight" | "ArrowDown" | " " | "Enter" | "PageDown" | "n" => {
                ev.prevent_default();
                next_step_or_slide();
            },
            | "ArrowLeft" | "ArrowUp" | "PageUp" | "Backspace" => {
                ev.prevent_default();
                prev_step_or_slide();
            },
            | "Home" => {
                ev.prevent_default();
                jump_to_slide(0);
            },
            | "End" => {
                ev.prevent_default();
                jump_to_slide(total_slides.get().saturating_sub(1));
            },
            | "f" | "F" => {
                ev.prevent_default();
                toggle_fullscreen();
            },
            | "o" | "O" | "g" | "G" => {
                ev.prevent_default();
                overview_open.update(|v| *v = !*v);
            },
            | "b" | "B" | "." => {
                ev.prevent_default();
                is_blank.update(|v| *v = !*v);
            },
            | "l" | "L" => {
                ev.prevent_default();
                laser_active.update(|v| *v = !*v);
                if laser_active.get() {
                    pen_active.set(false);
                }
            },
            | "p" | "P" => {
                ev.prevent_default();
                pen_active.update(|v| *v = !*v);
                if pen_active.get() {
                    laser_active.set(false);
                }
                let on = pen_active.get();
                trigger_toast(if on {
                    "Whiteboard Pen Activated".to_string()
                } else {
                    "Whiteboard Pen Closed".to_string()
                });
            },
            | "t" | "T" => {
                ev.prevent_default();
                pen_brush_type.update(|b| *b = b.cycle());
                let b = pen_brush_type.get();
                trigger_toast(format!("Brush: {}", b.name()));
            },
            | "[" => {
                ev.prevent_default();
                pen_width.update(|w| {
                    *w = match *w as usize {
                        | 0..=2 => 2.0,
                        | 3..=4 => 2.0,
                        | 5..=8 => 4.0,
                        | _ => 8.0,
                    };
                });
                let w = pen_width.get();
                trigger_toast(format!("Brush Width: {w:.0}px"));
            },
            | "]" => {
                ev.prevent_default();
                pen_width.update(|w| {
                    *w = match *w as usize {
                        | 0..=2 => 4.0,
                        | 3..=4 => 8.0,
                        | _ => 14.0,
                    };
                });
                let w = pen_width.get();
                trigger_toast(format!("Brush Width: {w:.0}px"));
            },
            | "u" | "U" => {
                ev.prevent_default();
                let cur = current_index.get();
                let mut popped = false;
                pen_strokes.update(|m| {
                    if let Some(v) = m.get_mut(&cur) {
                        popped = v.pop().is_some();
                    }
                });
                if popped {
                    trigger_toast("Undid last stroke".to_string());
                }
            },
            | "z" | "Z" if ev.ctrl_key() || ev.meta_key() => {
                ev.prevent_default();
                let cur = current_index.get();
                let mut popped = false;
                pen_strokes.update(|m| {
                    if let Some(v) = m.get_mut(&cur) {
                        popped = v.pop().is_some();
                    }
                });
                if popped {
                    trigger_toast("Undid last stroke".to_string());
                }
            },
            | "c" | "C" | "x" | "X" => {
                ev.prevent_default();
                let cur = current_index.get();
                pen_strokes.update(|m| {
                    m.remove(&cur);
                });
                trigger_toast("Cleared whiteboard annotations".to_string());
            },
            | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => {
                if let Ok(num) = ev.key().parse::<usize>() {
                    let idx = num.saturating_sub(1);
                    if idx < PEN_COLORS.len() {
                        pen_color_idx.set(idx);
                        pen_custom_color.set(PEN_COLORS[idx].to_string());
                        trigger_toast(format!("Color: {}", PEN_COLORS[idx]));
                    }
                }
            },
            | "0" => {
                if 9 < PEN_COLORS.len() {
                    pen_color_idx.set(9);
                    pen_custom_color.set(PEN_COLORS[9].to_string());
                    trigger_toast(format!("Color: {}", PEN_COLORS[9]));
                }
            },
            | "+" | "=" => {
                ev.prevent_default();
                adjust_vol(0.05);
            },
            | "-" | "_" => {
                ev.prevent_default();
                adjust_vol(-0.05);
            },
            | "m" | "M" => {
                ev.prevent_default();
                toggle_mute();
            },
            | "?" => {
                ev.prevent_default();
                help_open.update(|v| *v = !*v);
            },
            | "Escape" => {
                if help_open.get() {
                    help_open.set(false);
                } else if overview_open.get() {
                    overview_open.set(false);
                } else if is_blank.get() {
                    is_blank.set(false);
                }
            },
            | _ => {},
        }
    });

    let current_slide = Signal::derive(move || {
        let d = deck.get();
        let idx = current_index.get();
        d.slides.get(idx).cloned()
    });

    let current_view_box = Signal::derive(move || {
        current_slide
            .get()
            .map(|s| s.view_box)
            .unwrap_or_else(|| Rect::new(0.0, 0.0, 793.7008, 446.4567))
    });

    let current_view_box_str = Signal::derive(move || {
        let vb = current_view_box.get();
        format!("{} {} {} {}", vb.x, vb.y, vb.width, vb.height)
    });

    let current_svg =
        Signal::derive(move || current_slide.get().map(|s| s.svg_data).unwrap_or_default());

    let current_hotspots =
        Signal::derive(move || current_slide.get().map(|s| s.hotspots).unwrap_or_default());

    let current_steps =
        Signal::derive(move || current_slide.get().map(|s| s.steps).unwrap_or_default());

    let get_slide_coords = move |ev: &web_sys::MouseEvent| -> Option<(f64, f64)> {
        let doc = web_sys::window()?.document()?;
        let el = doc.get_element_by_id("slide-canvas")?;
        let rect = el.get_bounding_client_rect();
        if rect.width() <= 0.0 || rect.height() <= 0.0 {
            return None;
        }
        let vb = current_view_box.get();
        let rel_x =
            (ev.client_x() as f64 - rect.left()) / rect.width() * vb.width as f64 + vb.x as f64;
        let rel_y =
            (ev.client_y() as f64 - rect.top()) / rect.height() * vb.height as f64 + vb.y as f64;
        Some((rel_x, rel_y))
    };

    // Mouse movement for Laser Pointer & Pen
    let on_mousemove = move |ev: web_sys::MouseEvent| {
        let (mx, my) = (ev.client_x() as f64, ev.client_y() as f64);
        if laser_active.get() {
            let now = js_sys::Date::now();
            laser_coords.set((mx, my));
            laser_trail.update(|trail| {
                trail.retain(|(_, _, t)| now - *t <= 260.0);
                trail.push((mx, my, now));
            });
        }
        if let (true, true, Some(pt)) = (pen_active.get(), is_drawing.get(), get_slide_coords(&ev))
        {
            current_drawing_stroke.update(|pts| {
                pts.push(pt);
            });
        }
    };

    let on_mousedown = move |ev: web_sys::MouseEvent| {
        resume_audio_if_needed();
        if let (true, 0, Some(pt)) = (pen_active.get(), ev.button(), get_slide_coords(&ev)) {
            is_drawing.set(true);
            current_drawing_stroke.set(vec![pt]);
        }
    };

    let on_mouseup = move |_: web_sys::MouseEvent| {
        if pen_active.get() && is_drawing.get() {
            is_drawing.set(false);
            let pts = current_drawing_stroke.get();
            if pts.len() >= 2 {
                let color = pen_custom_color.get();
                let brush_type = pen_brush_type.get();
                let width = pen_width.get();
                let cur = current_index.get();
                pen_strokes.update(|map| {
                    map.entry(cur).or_default().push(PenStroke {
                        points: pts,
                        color,
                        brush_type,
                        width,
                    });
                });
            }
            current_drawing_stroke.set(Vec::new());
        }
    };

    // Scroll wheel for volume
    let on_wheel = move |ev: web_sys::WheelEvent| {
        let dy = ev.delta_y();
        if dy < 0.0 {
            adjust_vol(0.04);
        } else if dy > 0.0 {
            adjust_vol(-0.04);
        }
    };

    let progress_percent = Signal::derive(move || {
        let total = total_slides.get();
        if total <= 1 {
            100.0
        } else {
            let cur = current_index.get() as f64;
            let tot = (total - 1) as f64;
            (cur / tot * 100.0).clamp(0.0, 100.0)
        }
    });

    view! {
        <div
            id="presentation-root"
            class="slide-app"
            on:mousemove=on_mousemove
            on:mousedown=on_mousedown
            on:mouseup=on_mouseup
            on:wheel=on_wheel
        >
            // Blank screen mode
            {move || {
                if is_blank.get() {
                    view! {
                        <div class="blank-screen" on:click=move |_| is_blank.set(false)>
                            <div class="blank-hint">"Screen paused. Press 'B' or click anywhere to resume."</div>
                        </div>
                    }.into_any()
                } else {
                    view! { <div/> }.into_any()
                }
            }}

            // Main Presentation Viewport
            <div
                class="slide-viewport"
                on:click=move |ev| {
                    resume_audio_if_needed();
                    // Left click advances if not drawing pen
                    if !pen_active.get() && ev.button() == 0 {
                        next_step_or_slide();
                    }
                }
                on:contextmenu=move |ev| {
                    ev.prevent_default();
                    if !pen_active.get() {
                        prev_step_or_slide();
                    }
                }
            >
                <div id="slide-canvas" class="canvas-container">
                    <div
                        class=move || format!("slide-transition-frame anim-{}", active_transition.get())
                        attr:data-turn=move || transition_counter.get().to_string()
                    >
                        // SVG Slide Base
                        <div class="svg-wrapper" inner_html=move || current_svg.get() />

                        // In-Slide Step Masking Overlay
                        <svg class="step-overlay" viewBox=move || current_view_box_str.get()>
                            {move || {
                                let cur_st = current_step.get();
                                current_steps.get().into_iter().map(|st| {
                                    if st.order > cur_st {
                                        view! {
                                            <rect
                                                x=st.rect.x
                                                y=st.rect.y
                                                width=st.rect.width
                                                height=st.rect.height
                                                fill="#0f111a"
                                            />
                                        }.into_any()
                                    } else {
                                        view! { <g/> }.into_any()
                                    }
                                }).collect::<Vec<_>>()
                            }}
                        </svg>

                    // Interactive Hotspots Layer
                    <svg class="hotspot-overlay" viewBox=move || current_view_box_str.get()>
                        {move || {
                            current_hotspots.get().into_iter().map(|hs| {
                                let rect = hs.rect();
                                match hs {
                                    Hotspot::Link { target, .. } => {
                                        let tgt = target.clone();
                                        view! {
                                            <rect
                                                class="hotspot link"
                                                x=rect.x
                                                y=rect.y
                                                width=rect.width
                                                height=rect.height
                                                on:click=move |ev| {
                                                    ev.stop_propagation();
                                                    if tgt.starts_with("#page=") {
                                                        if let Ok(p) = tgt.trim_start_matches("#page=").parse::<usize>() {
                                                            jump_to_slide(p.saturating_sub(1));
                                                        }
                                                    } else if tgt.starts_with("#slide=") {
                                                        if let Ok(p) = tgt.trim_start_matches("#slide=").parse::<usize>() {
                                                            jump_to_slide(p.saturating_sub(1));
                                                        }
                                                    } else if tgt.starts_with('#') && tgt[1..].chars().all(|c| c.is_ascii_digit()) {
                                                        if let Ok(p) = tgt[1..].parse::<usize>() {
                                                            jump_to_slide(p.saturating_sub(1));
                                                        }
                                                    } else if tgt.starts_with("http://") || tgt.starts_with("https://") || tgt.starts_with("mailto:") {
                                                        if let Some(w) = web_sys::window() {
                                                            let _ = w.open_with_url_and_target(&tgt, "_blank");
                                                        }
                                                    } else {
                                                        let clean = tgt.strip_prefix("file://").unwrap_or(&tgt).to_string();
                                                        active_text_modal.set(Some((clean.clone(), "Loading document...".to_string(), true)));
                                                        let fetch_target = clean.clone();
                                                        wasm_bindgen_futures::spawn_local(async move {
                                                            match gloo_net::http::Request::get(&fetch_target).send().await {
                                                                Ok(resp) if resp.ok() => {
                                                                    match resp.text().await {
                                                                        Ok(text) => {
                                                                            active_text_modal.set(Some((clean, text, false)));
                                                                        }
                                                                        Err(e) => {
                                                                            active_text_modal.set(Some((clean, format!("Error reading document: {e}"), false)));
                                                                        }
                                                                    }
                                                                }
                                                                Ok(resp) => {
                                                                    active_text_modal.set(Some((clean, format!("HTTP error: {}", resp.status()), false)));
                                                                }
                                                                Err(e) => {
                                                                    active_text_modal.set(Some((clean, format!("Network error: {e}"), false)));
                                                                }
                                                            }
                                                        });
                                                    }
                                                }
                                            >
                                                <title>{target}</title>
                                            </rect>
                                        }.into_any()
                                    }
                                    Hotspot::Audio { source, volume, .. } => {
                                        view! {
                                            <rect
                                                class="hotspot audio"
                                                x=rect.x
                                                y=rect.y
                                                width=rect.width
                                                height=rect.height
                                                on:click=move |ev| {
                                                    ev.stop_propagation();
                                                    if let Some(ref a) = audio_element.get() {
                                                        if a.paused() {
                                                            let _ = a.play();
                                                            is_audio_playing.set(true);
                                                        } else {
                                                            let _ = a.pause();
                                                            is_audio_playing.set(false);
                                                        }
                                                    } else if let Ok(audio) = HtmlAudioElement::new_with_src(&source) {
                                                        audio.set_volume(global_volume.get() * volume as f64);
                                                        let _ = audio.play();
                                                        audio_element.set(Some(audio));
                                                        is_audio_playing.set(true);
                                                    }
                                                }
                                            >
                                                <title>"Click to toggle audio"</title>
                                            </rect>
                                        }.into_any()
                                    }
                                    Hotspot::Video { source, caption, .. } => {
                                        view! {
                                            <rect
                                                class="hotspot video"
                                                x=rect.x
                                                y=rect.y
                                                width=rect.width
                                                height=rect.height
                                                on:click=move |ev| {
                                                    ev.stop_propagation();
                                                    active_video_modal.set(Some((source.clone(), caption.clone())));
                                                }
                                            >
                                                <title>"Click to play video"</title>
                                            </rect>
                                        }.into_any()
                                    }
                                    Hotspot::Chart { data, .. } => {
                                        view! {
                                            <rect
                                                class="hotspot chart"
                                                x=rect.x
                                                y=rect.y
                                                width=rect.width
                                                height=rect.height
                                                on:click=move |ev| {
                                                    ev.stop_propagation();
                                                    active_chart_inspector.set(Some(data.clone()));
                                                }
                                            >
                                                <title>"Click to open HUD Data Inspector"</title>
                                            </rect>
                                        }.into_any()
                                    }
                                }
                            }).collect::<Vec<_>>()
                        }}
                    </svg>

                    // Whiteboard Drawing Canvas Layer
                    <svg class="pen-layer" viewBox=move || current_view_box_str.get()>
                        {move || {
                            let cur = current_index.get();
                            let map = pen_strokes.get();
                            let strokes = map.get(&cur).cloned().unwrap_or_default();
                            let current_pts = current_drawing_stroke.get();

                            let live_stroke = if current_pts.len() >= 2 {
                                Some(PenStroke {
                                    points: current_pts,
                                    color: pen_custom_color.get(),
                                    brush_type: pen_brush_type.get(),
                                    width: pen_width.get(),
                                })
                            } else {
                                None
                            };

                            view! {
                                <g>
                                    {strokes.into_iter().map(render_stroke_path).collect::<Vec<_>>()}
                                    {live_stroke.map(render_stroke_path)}
                                </g>
                            }
                        }}
                    </svg>

                    </div>
                </div>
            </div>

            // Laser Pointer Overlay with animated motion blur trail
            {move || {
                if laser_active.get() {
                    let (lx, ly) = laser_coords.get();
                    let now = js_sys::Date::now();
                    let pts = laser_trail.get();
                    let valid_pts: Vec<(f64, f64, f64)> = pts
                        .into_iter()
                        .filter(|(_, _, t)| now - *t <= 260.0)
                        .collect();

                    view! {
                        <svg class="laser-trail-overlay">
                            <defs>
                                <radialGradient id="laserGlow" cx="50%" cy="50%" r="50%">
                                    <stop offset="0%" stop-color="#ffffff" stop-opacity="1"/>
                                    <stop offset="25%" stop-color="#ff3366" stop-opacity="0.95"/>
                                    <stop offset="65%" stop-color="#ff3366" stop-opacity="0.45"/>
                                    <stop offset="100%" stop-color="#ff3366" stop-opacity="0"/>
                                </radialGradient>
                                <radialGradient id="particleGlow" cx="50%" cy="50%" r="50%">
                                    <stop offset="0%" stop-color="#ff3366" stop-opacity="0.8"/>
                                    <stop offset="100%" stop-color="#ff3366" stop-opacity="0"/>
                                </radialGradient>
                            </defs>
                            {valid_pts.windows(2).map(|w| {
                                let (x0, y0, t0) = w[0];
                                let (x1, y1, t1) = w[1];
                                let age = now - (t0 + t1) * 0.5;
                                let progress = (1.0 - (age / 260.0)).clamp(0.0, 1.0);
                                let stroke_w = 2.0 + progress * 6.0;
                                let opacity = progress * progress * 0.85;
                                view! {
                                    <line
                                        x1=x0 y1=y0 x2=x1 y2=y1
                                        stroke="#ff3366"
                                        stroke-width=stroke_w
                                        stroke-linecap="round"
                                        opacity=opacity
                                    />
                                }
                            }).collect::<Vec<_>>()}
                            {valid_pts.iter().map(|&(px, py, pt)| {
                                let age = now - pt;
                                let progress = (1.0 - (age / 260.0)).clamp(0.0, 1.0);
                                let r = 2.5 + progress * 4.5;
                                let opacity = progress * 0.75;
                                view! {
                                    <circle cx=px cy=py r=r fill="url(#particleGlow)" opacity=opacity />
                                }
                            }).collect::<Vec<_>>()}
                            <circle cx=lx cy=ly r="14" fill="url(#laserGlow)"/>
                            <circle cx=lx cy=ly r="3.5" fill="#ffffff"/>
                        </svg>
                    }.into_any()
                } else {
                    view! { <div/> }.into_any()
                }
            }}

            // Volume Toast Alert
            {move || {
                if let Some((ref msg, _)) = volume_toast.get() {
                    view! {
                        <div class="hud-toast">
                            <span>{msg.clone()}</span>
                        </div>
                    }.into_any()
                } else {
                    view! { <div/> }.into_any()
                }
            }}

            // Progress bar
            <div class="progress-container">
                <div
                    class="progress-bar"
                    style=move || format!("width: {}%;", progress_percent.get())
                />
            </div>

            // Floating Pen / Brush Toolbox Overlay
            {move || {
                if pen_active.get() {
                    view! {
                        <div class="pen-toolbox-container" on:mousedown=move |ev| ev.stop_propagation()>
                            <div class="pen-toolbox-card">
                                // Brush Mode Selection
                                <div class="pen-group">
                                    {BrushType::ALL.into_iter().map(|b| {
                                        let is_sel = move || pen_brush_type.get() == b;
                                        view! {
                                            <button
                                                class=move || if is_sel() { "pen-tool-btn active" } else { "pen-tool-btn" }
                                                title=format!("Switch to {} Brush (T)", b.name())
                                                on:click=move |_| {
                                                    pen_brush_type.set(b);
                                                    trigger_toast(format!("Brush: {}", b.name()));
                                                }
                                            >
                                                {b.icon()}
                                            </button>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>

                                <div class="pen-divider"/>

                                // Stroke Width Selection
                                <div class="pen-group">
                                    {[2.0, 4.0, 8.0, 14.0].into_iter().map(|w| {
                                        let is_sel = move || (pen_width.get() - w).abs() < 0.1;
                                        view! {
                                            <button
                                                class=move || if is_sel() { "pen-size-btn active" } else { "pen-size-btn" }
                                                title=format!("Stroke Width {w:.0}px ([ / ])")
                                                on:click=move |_| {
                                                    pen_width.set(w);
                                                    trigger_toast(format!("Brush Width: {w:.0}px"));
                                                }
                                            >
                                                {format!("{w:.0}p")}
                                            </button>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>

                                <div class="pen-divider"/>

                                // 14 Color Swatches
                                <div class="pen-swatches-grid">
                                    {PEN_COLORS.iter().enumerate().map(|(idx, &c)| {
                                        let is_sel = move || pen_custom_color.get().eq_ignore_ascii_case(c);
                                        view! {
                                            <button
                                                class=move || if is_sel() { "pen-swatch active" } else { "pen-swatch" }
                                                style=format!("background-color: {c};")
                                                title=format!("Palette Color {} ({c})", idx + 1)
                                                on:click=move |_| {
                                                    pen_color_idx.set(idx);
                                                    pen_custom_color.set(c.to_string());
                                                    trigger_toast(format!("Color: {c}"));
                                                }
                                            />
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>

                                // Custom Color Picker Native Input
                                <div class="pen-picker-wrap" title="Custom color picker">
                                    <input
                                        type="color"
                                        class="pen-color-native"
                                        prop:value=move || pen_custom_color.get()
                                        on:input=move |ev| {
                                            let val = event_target_value(&ev);
                                            pen_custom_color.set(val);
                                        }
                                    />
                                </div>

                                <div class="pen-divider"/>

                                // Quick Actions: Undo & Clear
                                <div class="pen-group">
                                    <button
                                        class="pen-action-btn"
                                        title="Undo last stroke (U / Ctrl+Z)"
                                        on:click=move |_| {
                                            let cur = current_index.get();
                                            let mut popped = false;
                                            pen_strokes.update(|m| {
                                                if let Some(v) = m.get_mut(&cur) {
                                                    popped = v.pop().is_some();
                                                }
                                            });
                                            if popped {
                                                trigger_toast("Undid last stroke".to_string());
                                            }
                                        }
                                    >"↩ Undo"</button>
                                    <button
                                        class="pen-action-btn"
                                        title="Clear all annotations on this slide (C / X)"
                                        on:click=move |_| {
                                            let cur = current_index.get();
                                            pen_strokes.update(|m| { m.remove(&cur); });
                                            trigger_toast("Cleared whiteboard annotations".to_string());
                                        }
                                    >"🧹 Clear"</button>
                                </div>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! { <div/> }.into_any()
                }
            }}

            // Bottom Navigation & Presenter Tools Dock
            <div class="hud-container">
                <div class="hud-card">
                    <button class="hud-btn" title="First slide (Home)" on:click=move |_| jump_to_slide(0)>"⏮"</button>
                    <button class="hud-btn" title="Previous (Left arrow / Backspace)" on:click=move |_| prev_step_or_slide()>"◀"</button>
                    <div class="hud-counter">
                        {move || {
                            let cur_st = current_step.get();
                            let max_st = current_slide.get().map(|s| s.max_step()).unwrap_or(0);
                            if max_st > 0 {
                                format!("{} / {} (Step {})", current_index.get() + 1, total_slides.get(), cur_st)
                            } else {
                                format!("{} / {}", current_index.get() + 1, total_slides.get())
                            }
                        }}
                    </div>
                    <button class="hud-btn" title="Next (Right arrow / Space / Enter)" on:click=move |_| next_step_or_slide()>"▶"</button>
                    <button class="hud-btn" title="Last slide (End)" on:click=move |_| jump_to_slide(total_slides.get().saturating_sub(1))>"⏭"</button>
                    <div class="hud-divider"/>
                    <button
                        class=move || if laser_active.get() { "hud-btn active" } else { "hud-btn" }
                        title="Laser Pointer (L)"
                        on:click=move |_| {
                            laser_active.update(|v| *v = !*v);
                            if laser_active.get() { pen_active.set(false); }
                        }
                    >"🔦"</button>
                    <button
                        class=move || if pen_active.get() { "hud-btn active" } else { "hud-btn" }
                        title="Whiteboard Drawing Pen (P)"
                        on:click=move |_| {
                            pen_active.update(|v| *v = !*v);
                            if pen_active.get() { laser_active.set(false); }
                        }
                    >"✏️"</button>
                    <button
                        class="hud-btn"
                        title="Clear whiteboard pen annotations (C / X)"
                        on:click=move |_| {
                            let cur = current_index.get();
                            pen_strokes.update(|m| { m.remove(&cur); });
                        }
                    >"🧹"</button>
                    <div class="hud-divider"/>
                    <button
                        class=move || if is_muted.get() { "hud-btn active" } else { "hud-btn" }
                        title="Mute / Unmute Audio (M)"
                        on:click=move |_| toggle_mute()
                    >{move || if is_muted.get() { "🔇" } else { "🔊" }}</button>
                    <div class="hud-divider"/>
                    <button class="hud-btn" title="Slide Overview Grid (O)" on:click=move |_| overview_open.update(|v| *v = !*v)>"▦"</button>
                    <button class="hud-btn" title="Toggle Fullscreen (F)" on:click=move |_| toggle_fullscreen()>"⛶"</button>
                    <button class="hud-btn" title="Help & Shortcuts (?)" on:click=move |_| help_open.update(|v| *v = !*v)>"?"</button>
                </div>
            </div>

            // HUD Data Inspector Modal
            {move || {
                if let Some(initial_data) = active_chart_inspector.get() {
                    view! {
                        <ChartInspectorModal
                            initial_data=initial_data
                            on_close=move || active_chart_inspector.set(None)
                        />
                    }.into_any()
                } else {
                    view! { <div/> }.into_any()
                }
            }}

            // Video Player Modal
            {move || {
                if let Some((src, caption)) = active_video_modal.get() {
                    view! {
                        <div class="modal-overlay" on:click=move |_| active_video_modal.set(None)>
                            <div class="video-modal" id="modal-video-container" on:click=move |ev| ev.stop_propagation()>
                                <div class="modal-header">
                                    <div class="video-title-box">
                                        <span class="modal-icon">"🎬"</span>
                                        <h2>{caption.unwrap_or_else(|| "Video Player".to_string())}</h2>
                                    </div>
                                    <div class="modal-actions">
                                        <button
                                            class="btn-action"
                                            title="Toggle True Fullscreen (or double-click video)"
                                            on:click=move |_| {
                                                if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                                                    if doc.fullscreen_element().is_some() {
                                                        doc.exit_fullscreen();
                                                    } else if let Some(v_el) = doc.get_element_by_id("modal-video-element") {
                                                        let _ = v_el.request_fullscreen();
                                                    } else if let Some(m_el) = doc.get_element_by_id("modal-video-container") {
                                                        let _ = m_el.request_fullscreen();
                                                    }
                                                }
                                            }
                                        >"⛶ Fullscreen"</button>
                                        <button class="close-btn" on:click=move |_| active_video_modal.set(None)>"✕"</button>
                                    </div>
                                </div>
                                <div class="video-body">
                                    <video
                                        id="modal-video-element"
                                        src=src
                                        controls
                                        autoplay
                                        playsinline
                                        class="video-player"
                                        on:dblclick=move |_| {
                                            if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                                                if doc.fullscreen_element().is_some() {
                                                    doc.exit_fullscreen();
                                                } else if let Some(v_el) = doc.get_element_by_id("modal-video-element") {
                                                    let _ = v_el.request_fullscreen();
                                                }
                                            }
                                        }
                                    />
                                </div>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! { <div/> }.into_any()
                }
            }}

            // Text / Code Preview Modal
            {move || {
                if let Some((file_path, content, is_loading)) = active_text_modal.get() {
                    view! {
                        <TextPreviewModal
                            file_path=file_path
                            content=content
                            is_loading=is_loading
                            on_close=move || active_text_modal.set(None)
                        />
                    }.into_any()
                } else {
                    view! { <div/> }.into_any()
                }
            }}

            // Overview Modal
            {move || {
                if overview_open.get() {
                    let d = deck.get();
                    let cur_idx = current_index.get();
                    view! {
                        <div class="modal-overlay" on:click=move |_| overview_open.set(false)>
                            <div class="overview-modal" on:click=move |ev| ev.stop_propagation()>
                                <div class="modal-header">
                                    <h2>"Slide Overview"</h2>
                                    <button class="close-btn" on:click=move |_| overview_open.set(false)>"✕"</button>
                                </div>
                                <div class="grid-container">
                                    {d.slides.into_iter().enumerate().map(|(idx, slide)| {
                                        let is_active = idx == cur_idx;
                                        view! {
                                            <div
                                                class=if is_active { "slide-card active" } else { "slide-card" }
                                                on:click=move |_| jump_to_slide(idx)
                                            >
                                                <div class="card-thumbnail" inner_html=slide.svg_data/>
                                                <div class="card-label">
                                                    <span class="card-page">{format!("#{}", idx + 1)}</span>
                                                    <span class="card-title">{format!("Slide {}", idx + 1)}</span>
                                                </div>
                                            </div>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! { <div/> }.into_any()
                }
            }}

            // Help Shortcuts Modal
            {move || {
                if help_open.get() {
                    view! {
                        <div class="modal-overlay" on:click=move |_| help_open.set(false)>
                            <div class="help-modal" on:click=move |ev| ev.stop_propagation()>
                                <div class="modal-header">
                                    <h2>"Presenter Shortcuts & Controls"</h2>
                                    <button class="close-btn" on:click=move |_| help_open.set(false)>"✕"</button>
                                </div>
                                <div class="help-content">
                                    <table class="shortcuts-table">
                                        <tbody>
                                            <tr><td><kbd>"Space"</kbd> / <kbd>"Enter"</kbd> / <kbd>"→"</kbd></td><td>Next step or slide</td></tr>
                                            <tr><td><kbd>"Backspace"</kbd> / <kbd>"←"</kbd></td><td>Previous step or slide</td></tr>
                                            <tr><td><kbd>"Home"</kbd> / <kbd>"End"</kbd></td><td>Jump to first / last slide</td></tr>
                                            <tr><td><kbd>"F"</kbd></td><td>Toggle True Fullscreen</td></tr>
                                            <tr><td><kbd>"L"</kbd></td><td>Toggle Laser Pointer trail</td></tr>
                                            <tr><td><kbd>"P"</kbd></td><td>Toggle Whiteboard Drawing Pen</td></tr>
                                            <tr><td><kbd>"T"</kbd></td><td>Cycle Brush (Pen / Highlighter / Neon / Arrow)</td></tr>
                                            <tr><td><kbd>"["</kbd> / <kbd>"]"</kbd></td><td>Decrease / Increase brush width</td></tr>
                                            <tr><td><kbd>"1 - 0"</kbd></td><td>Select pen palette color</td></tr>
                                            <tr><td><kbd>"U"</kbd> / <kbd>"Ctrl+Z"</kbd></td><td>Undo last pen stroke</td></tr>
                                            <tr><td><kbd>"C"</kbd> / <kbd>"X"</kbd></td><td>Clear slide whiteboard pen annotations</td></tr>
                                            <tr><td><kbd>"+"</kbd> / <kbd>"-"</kbd> / <kbd>"Wheel"</kbd></td><td>Adjust master audio volume</td></tr>
                                            <tr><td><kbd>"M"</kbd></td><td>Mute / Unmute audio</td></tr>
                                            <tr><td><kbd>"O"</kbd> / <kbd>"G"</kbd></td><td>Toggle Overview Grid</td></tr>
                                            <tr><td><kbd>"B"</kbd> / <kbd>"."</kbd></td><td>Blank / Pause screen</td></tr>
                                            <tr><td><kbd>"?"</kbd></td><td>Toggle Help Modal</td></tr>
                                            <tr><td><kbd>"Esc"</kbd></td><td>Close Active Modal</td></tr>
                                        </tbody>
                                    </table>
                                </div>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! { <div/> }.into_any()
                }
            }}
        </div>
    }
}

#[component]
fn ChartInspectorModal(
    initial_data: ChartData,
    on_close: impl Fn() + 'static + Copy,
) -> impl IntoView {
    let base_data = RwSignal::new(initial_data.clone());
    let active_type = RwSignal::new(initial_data.chart_type);
    let active_transform = RwSignal::new(TransformPreset::None);
    let search_query = RwSignal::new(String::new());
    let visible_series = RwSignal::new(
        initial_data
            .series
            .iter()
            .map(|s| s.name.clone())
            .collect::<Vec<_>>(),
    );
    let sort_col = RwSignal::new(None::<(usize, bool)>); // Column index, asc

    let transformed_data = Signal::derive(move || {
        let raw = base_data.get();
        let tr = active_transform.get();
        apply_transform(&raw, tr)
    });

    let filtered_indices = Signal::derive(move || {
        let d = transformed_data.get();
        let vs = visible_series.get();
        let query = search_query.get();
        let q_trim = query.trim();

        let mut indices: Vec<usize> = (0..d.categories.len()).collect();

        if !q_trim.is_empty() {
            if let Some((op, num_str)) = parse_filter_condition(q_trim) {
                if let Ok(target_num) = num_str.parse::<f64>() {
                    indices.retain(|&cat_i| {
                        d.series.iter().filter(|s| vs.contains(&s.name)).any(|s| {
                            if let Some(&val) = s.values.get(cat_i) {
                                match op {
                                    | ">" => val > target_num,
                                    | ">=" => val >= target_num,
                                    | "<" => val < target_num,
                                    | "<=" => val <= target_num,
                                    | "==" | "=" => (val - target_num).abs() < 1e-6,
                                    | "!=" => (val - target_num).abs() >= 1e-6,
                                    | _ => true,
                                }
                            } else {
                                false
                            }
                        })
                    });
                }
            } else {
                let q_lower = q_trim.to_lowercase();
                indices.retain(|&cat_i| {
                    if let Some(cat_name) = d.categories.get(cat_i)
                        && cat_name.to_lowercase().contains(&q_lower)
                    {
                        return true;
                    }
                    d.series.iter().filter(|s| vs.contains(&s.name)).any(|s| {
                        if let Some(&val) = s.values.get(cat_i) {
                            let formatted = d.format_number(val).to_lowercase();
                            if formatted.contains(&q_lower) || val.to_string().contains(&q_lower) {
                                return true;
                            }
                        }
                        false
                    })
                });
            }
        }

        if let Some((col, asc)) = sort_col.get() {
            indices.sort_by(|&a, &b| {
                let ord = if col == 0 {
                    let cat_a = d.categories.get(a).map(String::as_str).unwrap_or("");
                    let cat_b = d.categories.get(b).map(String::as_str).unwrap_or("");
                    cat_a.cmp(cat_b)
                } else {
                    let s_idx = col.saturating_sub(1);
                    let val_a = d
                        .series
                        .iter()
                        .filter(|s| vs.contains(&s.name))
                        .nth(s_idx)
                        .and_then(|s| s.values.get(a))
                        .copied()
                        .unwrap_or(0.0);
                    let val_b = d
                        .series
                        .iter()
                        .filter(|s| vs.contains(&s.name))
                        .nth(s_idx)
                        .and_then(|s| s.values.get(b))
                        .copied()
                        .unwrap_or(0.0);
                    val_a
                        .partial_cmp(&val_b)
                        .unwrap_or(std::cmp::Ordering::Equal)
                };
                if asc { ord } else { ord.reverse() }
            });
        }

        indices
    });

    let stats = Signal::derive(move || {
        let d = transformed_data.get();
        let vs = visible_series.get();
        let f_indices = filtered_indices.get();
        calculate_stats(&d, &vs, &f_indices)
    });

    let rendered_chart_svg = Signal::derive(move || {
        let d = transformed_data.get();
        let ct = active_type.get();
        let vs = visible_series.get();
        let f_indices = filtered_indices.get();
        render_svg_chart(&d, ct, &vs, &f_indices, 750.0, 370.0)
    });

    let on_export_csv = move |_| {
        let d = transformed_data.get();
        let vs = visible_series.get();
        let f_indices = filtered_indices.get();
        let csv = export_csv_string(&d, &vs, &f_indices);
        trigger_file_download("chart_data.csv", &csv, "text/csv");
    };

    let on_cycle_format = move |_| {
        base_data.update(|d| {
            d.format = d.format.cycle();
        });
    };

    view! {
        <div class="modal-overlay" on:click=move |_| on_close()>
            <div class="inspector-modal" on:click=move |ev| ev.stop_propagation()>
                // Modal Header
                <div class="modal-header">
                    <div class="inspector-title">
                        <h2>{move || transformed_data.get().title.unwrap_or_else(|| "HUD Data Inspector".to_string())}</h2>
                        <span class="inspector-badge">"Interactive SQL & Data Engine"</span>
                    </div>
                    <button class="close-btn" on:click=move |_| on_close()>"✕"</button>
                </div>

                // KPI Strip (6 HUD KPI Cards)
                <div class="kpi-strip">
                    <div class="kpi-card">
                        <span class="kpi-label">"TOTAL SUM"</span>
                        <span class="kpi-value">{move || transformed_data.get().format_number(stats.get().total_sum)}</span>
                        <span class="kpi-sub">"All visible"</span>
                    </div>
                    <div class="kpi-card">
                        <span class="kpi-label">"MEAN / AVG"</span>
                        <span class="kpi-value">{move || transformed_data.get().format_number(stats.get().avg)}</span>
                        <span class="kpi-sub">"Per record"</span>
                    </div>
                    <div class="kpi-card">
                        <span class="kpi-label">"MEDIAN"</span>
                        <span class="kpi-value">{move || transformed_data.get().format_number(stats.get().median)}</span>
                        <span class="kpi-sub">"Middle 50%"</span>
                    </div>
                    <div class="kpi-card">
                        <span class="kpi-label">"STD DEV"</span>
                        <span class="kpi-value">{move || transformed_data.get().format_number(stats.get().std_dev)}</span>
                        <span class="kpi-sub">"Dispersion"</span>
                    </div>
                    <div class="kpi-card">
                        <span class="kpi-label">"PEAK / MAX"</span>
                        <span class="kpi-value">{move || transformed_data.get().format_number(stats.get().max_val)}</span>
                        <span class="kpi-sub">{move || stats.get().max_cat}</span>
                    </div>
                    <div class="kpi-card">
                        <span class="kpi-label">"MIN / LOW"</span>
                        <span class="kpi-value">{move || transformed_data.get().format_number(stats.get().min_val)}</span>
                        <span class="kpi-sub">{move || stats.get().min_cat}</span>
                    </div>
                </div>

                // Toolbar Controls
                <div class="inspector-toolbar">
                    // Chart Type Switcher
                    <div class="tool-group">
                        <button
                            class=move || if active_type.get() == ChartType::Bar { "btn-chip active" } else { "btn-chip" }
                            on:click=move |_| active_type.set(ChartType::Bar)
                        >"📊 Bar"</button>
                        <button
                            class=move || if active_type.get() == ChartType::Line { "btn-chip active" } else { "btn-chip" }
                            on:click=move |_| active_type.set(ChartType::Line)
                        >"📈 Line"</button>
                        <button
                            class=move || if active_type.get() == ChartType::Area { "btn-chip active" } else { "btn-chip" }
                            on:click=move |_| active_type.set(ChartType::Area)
                        >"📉 Area"</button>
                        <button
                            class=move || if active_type.get() == ChartType::Pie { "btn-chip active" } else { "btn-chip" }
                            on:click=move |_| active_type.set(ChartType::Pie)
                        >"🥧 Pie"</button>
                        <button
                            class=move || if active_type.get() == ChartType::Donut { "btn-chip active" } else { "btn-chip" }
                            on:click=move |_| active_type.set(ChartType::Donut)
                        >"🍩 Donut"</button>
                        <button
                            class=move || if active_type.get() == ChartType::Scatter { "btn-chip active" } else { "btn-chip" }
                            on:click=move |_| active_type.set(ChartType::Scatter)
                        >"⁖ Scatter"</button>
                    </div>

                    // Transform Presets
                    <div class="tool-group">
                        <button
                            class=move || if active_transform.get() == TransformPreset::None { "btn-chip active" } else { "btn-chip" }
                            on:click=move |_| active_transform.set(TransformPreset::None)
                        >"ORIG"</button>
                        <button
                            class=move || if active_transform.get() == TransformPreset::Top5 { "btn-chip active" } else { "btn-chip" }
                            on:click=move |_| active_transform.set(TransformPreset::Top5)
                        >"TOP 5"</button>
                        <button
                            class=move || if active_transform.get() == TransformPreset::SortDesc { "btn-chip active" } else { "btn-chip" }
                            on:click=move |_| active_transform.set(TransformPreset::SortDesc)
                        >"SORT ▼"</button>
                        <button
                            class=move || if active_transform.get() == TransformPreset::SortAsc { "btn-chip active" } else { "btn-chip" }
                            on:click=move |_| active_transform.set(TransformPreset::SortAsc)
                        >"SORT ▲"</button>
                        <button
                            class=move || if active_transform.get() == TransformPreset::Cumulative { "btn-chip active" } else { "btn-chip" }
                            on:click=move |_| active_transform.set(TransformPreset::Cumulative)
                        >"CUM"</button>
                        <button
                            class=move || if active_transform.get() == TransformPreset::Percent100 { "btn-chip active" } else { "btn-chip" }
                            on:click=move |_| active_transform.set(TransformPreset::Percent100)
                        >"% SHARE"</button>
                        <button
                            class=move || if active_transform.get() == TransformPreset::MovingAvg3 { "btn-chip active" } else { "btn-chip" }
                            on:click=move |_| active_transform.set(TransformPreset::MovingAvg3)
                        >"MA3"</button>
                    </div>

                    // Format Toggle Button
                    <button class="btn-action" on:click=on_cycle_format>
                        {move || format!("🔣 {}", transformed_data.get().format.display_name())}
                    </button>

                    // Export CSV Button
                    <button class="btn-action" on:click=on_export_csv>"💾 Export CSV"</button>
                </div>

                // Series Filter Chips
                <div class="series-strip">
                    <span class="series-label">"Series:"</span>
                    {move || {
                        let d = base_data.get();
                        let current_vs = visible_series.get();
                        d.series.into_iter().enumerate().map(|(idx, s)| {
                            let name = s.name.clone();
                            let is_visible = current_vs.contains(&name);
                            let color = s.color.unwrap_or_else(|| chart::PALETTE[idx % chart::PALETTE.len()].to_string());
                            view! {
                                <button
                                    class=if is_visible { "series-chip active" } else { "series-chip" }
                                    on:click=move |_| {
                                        let n = name.clone();
                                        visible_series.update(|list| {
                                            if list.contains(&n) {
                                                if list.len() > 1 {
                                                    list.retain(|x| x != &n);
                                                }
                                            } else {
                                                list.push(n);
                                            }
                                        });
                                    }
                                >
                                    <span class="series-dot" style=format!("background: {color};") />
                                    {s.name}
                                </button>
                            }
                        }).collect::<Vec<_>>()
                    }}
                </div>

                // Inspector Body: Left Chart SVG, Right Data Table
                <div class="inspector-body">
                    <div class="chart-display" inner_html=move || rendered_chart_svg.get() />

                    // Interactive Tabular Data Grid
                    <div class="table-container">
                        <div class="table-controls-bar">
                            <div class="table-search">
                                <input
                                    type="text"
                                    placeholder="Filter rows or condition (e.g. >50 or Q3)..."
                                    prop:value=move || search_query.get()
                                    on:input=move |ev| search_query.set(event_target_value(&ev))
                                />
                                {move || {
                                    if !search_query.get().is_empty() {
                                        view! {
                                            <button class="clear-filter-btn" on:click=move |_| search_query.set(String::new())>
                                                "CLEAR"
                                            </button>
                                        }.into_any()
                                    } else {
                                        view! { <span/> }.into_any()
                                    }
                                }}
                            </div>
                            <div class="row-count-badge">
                                {move || format!("Showing {} of {} rows", filtered_indices.get().len(), transformed_data.get().categories.len())}
                            </div>
                        </div>
                        <table class="inspector-table">
                            <thead>
                                <tr>
                                    <th on:click=move |_| {
                                        sort_col.update(|sc| {
                                            *sc = match sc {
                                                Some((0, true)) => Some((0, false)),
                                                _ => Some((0, true)),
                                            };
                                        });
                                    }>"Category ↕"</th>
                                    {move || {
                                        let d = transformed_data.get();
                                        let vs = visible_series.get();
                                        d.series.into_iter().enumerate().filter(|(_, s)| vs.contains(&s.name)).map(|(idx, s)| {
                                            let col_idx = idx + 1;
                                            view! {
                                                <th on:click=move |_| {
                                                    sort_col.update(|sc| {
                                                        *sc = match sc {
                                                            Some((c, true)) if *c == col_idx => Some((col_idx, false)),
                                                            _ => Some((col_idx, true)),
                                                        };
                                                    });
                                                }>{format!("{} ↕", s.name)}</th>
                                            }
                                        }).collect::<Vec<_>>()
                                    }}
                                </tr>
                            </thead>
                            <tbody>
                                {move || {
                                    let d = transformed_data.get();
                                    let f_indices = filtered_indices.get();
                                    let vs = visible_series.get();

                                    f_indices.into_iter().map(|cat_i| {
                                        let cat = d.categories.get(cat_i).cloned().unwrap_or_default();
                                        let vals: Vec<f64> = d.series.iter().filter(|s| vs.contains(&s.name)).map(|s| {
                                            s.values.get(cat_i).copied().unwrap_or(0.0)
                                        }).collect();

                                        view! {
                                            <tr>
                                                <td class="cat-cell">{cat}</td>
                                                {vals.into_iter().map(|v| {
                                                    view! {
                                                        <td class="num-cell">{d.format_number(v)}</td>
                                                    }
                                                }).collect::<Vec<_>>()}
                                            </tr>
                                        }
                                    }).collect::<Vec<_>>()
                                }}
                            </tbody>
                        </table>
                    </div>
                </div>
            </div>
        </div>
    }
}

#[component]
fn TextPreviewModal(
    file_path: String,
    content: String,
    is_loading: bool,
    on_close: impl Fn() + 'static + Copy,
) -> impl IntoView {
    let copy_feedback = RwSignal::new(false);
    let raw_text = content.clone();
    let highlighted_lines = highlight::highlight_content(&content, &file_path);
    let total_lines = highlighted_lines.len();
    let total_bytes = content.len();

    let ext_tag = file_path
        .rsplit('.')
        .next()
        .unwrap_or("FILE")
        .to_uppercase();

    let on_copy = move |_| {
        let text_to_copy = raw_text.clone();
        if let Some(w) = web_sys::window() {
            let clipboard = w.navigator().clipboard();
            let _ = clipboard.write_text(&text_to_copy);
            copy_feedback.set(true);
            let cb = wasm_bindgen::closure::Closure::once(move || {
                copy_feedback.set(false);
            });
            let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                2000,
            );
            cb.forget();
        }
    };

    let on_download = {
        let text_to_dl = content.clone();
        let filename = file_path
            .rsplit('/')
            .next()
            .unwrap_or(&file_path)
            .to_string();
        move |_| {
            trigger_file_download(&filename, &text_to_dl, "text/plain; charset=utf-8");
        }
    };

    view! {
        <div class="modal-overlay" on:click=move |_| on_close()>
            <div class="text-preview-modal" on:click=move |ev| ev.stop_propagation()>
                // Modal Header
                <div class="modal-header">
                    <div class="text-preview-title">
                        <span class="file-icon">"📄"</span>
                        <h2>{file_path.clone()}</h2>
                        <span class="format-badge">{ext_tag}</span>
                        <span class="file-meta">{format!("{total_lines} lines • {total_bytes} bytes")}</span>
                    </div>
                    <div class="modal-actions">
                        <button
                            class="btn-action"
                            title="Copy file contents to clipboard"
                            on:click=on_copy
                        >
                            {move || if copy_feedback.get() { "✓ Copied!" } else { "📋 Copy" }}
                        </button>
                        <button
                            class="btn-action"
                            title="Download file"
                            on:click=on_download
                        >
                            "💾 Download"
                        </button>
                        <button class="close-btn" on:click=move |_| on_close()>"✕"</button>
                    </div>
                </div>

                // Modal Body
                <div class="text-preview-body">
                    {if is_loading {
                        view! {
                            <div class="loading-state">
                                <div class="spinner" />
                                <span>"Fetching document..."</span>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div class="code-viewer-container">
                                <table class="code-table">
                                    <tbody>
                                        {highlighted_lines.into_iter().map(|(num, line_html)| {
                                            view! {
                                                <tr>
                                                    <td class="line-num">{num}</td>
                                                    <td class="line-code" inner_html=line_html />
                                                </tr>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </tbody>
                                </table>
                            </div>
                        }.into_any()
                    }}
                </div>
            </div>
        </div>
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}
