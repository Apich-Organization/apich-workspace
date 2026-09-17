//! Real Rust replacement for the terminal page's hand-written JS (`terminal_page.rs`).
//!
//! Submits commands to the project's real sandbox container, appends output to the screen,
//! auto-scrolls to the newest output, and provides full and cropped screenshot and animation capture to project assets.

use std::fmt::Write;
use leptos::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DragHandle {
    Move,
    Nw,
    N,
    Ne,
    E,
    Se,
    S,
    Sw,
    W,
}

#[derive(Clone, Copy, Debug)]
struct DragStart {
    handle: DragHandle,
    mouse_x: f64,
    mouse_y: f64,
    init_x: i32,
    init_y: i32,
    init_w: i32,
    init_h: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct ScreenCropMetrics {
    pub crop_x: i32,
    pub crop_y: i32,
    pub crop_w: i32,
    pub crop_h: i32,
    pub scroll_top: f64,
    pub client_w: f64,
    pub client_h: f64,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct SavedAssetInfo {
    pub asset_path: String,
    pub filename: String,
    pub format: String,
    pub message: String,
    pub markdown_snippet: String,
    pub typst_snippet: String,
    pub latex_snippet: String,
}

#[island]
pub fn TerminalIsland(
    #[prop(into)] project_id: String,
    #[prop(into)] initial_screen: String,
    is_zh: bool,
) -> impl IntoView {
    let screen = RwSignal::new(initial_screen);
    let input = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let saving = RwSignal::new(false);
    let status_msg = RwSignal::new(None::<String>);
    let saved_asset = RwSignal::new(None::<SavedAssetInfo>);

    // Crop selection box state
    let crop_active = RwSignal::new(false);
    let crop_x = RwSignal::new(20i32);
    let crop_y = RwSignal::new(20i32);
    let crop_w = RwSignal::new(540i32);
    let crop_h = RwSignal::new(220i32);
    let drag_start = StoredValue::new(None::<DragStart>);

    // Live session recording state
    let is_recording = RwSignal::new(false);
    let rec_frames = StoredValue::new(Vec::<(f64, String)>::new());
    let rec_start_time = StoredValue::new(0.0f64);

    let hydrated = RwSignal::new(false);
    Effect::new(move |_| {
        hydrated.set(true);
    });

    // Auto-scroll effect: scroll to bottom whenever screen text changes
    Effect::new(move |_| {
        screen.track();
        scroll_terminal_to_bottom();
    });

    // Record frame whenever screen changes if recording is active
    Effect::new(move |_| {
        let text = screen.get();
        if is_recording.get() {
            let now = current_time_ms();
            let start = rec_start_time.get_value();
            let elapsed = if start > 0.0 { (now - start) / 1000.0 } else { 0.0 };
            rec_frames.update_value(|frames| {
                frames.push((elapsed, text));
            });
        }
    });

    let submit = {
        let project_id = project_id.clone();
        move || {
            let cmd = input.get_untracked().trim().to_string();
            if cmd.is_empty() || busy.get_untracked() {
                return;
            }
            screen.update(|s| {
                s.push_str("\n$ ");
                s.push_str(&cmd);
                s.push('\n');
            });
            input.set(String::new());
            busy.set(true);
            scroll_terminal_to_bottom();
            run_command(project_id.clone(), cmd, screen, busy);
        }
    };

    let on_clear = move |_| {
        screen.set(String::new());
        status_msg.set(Some(if is_zh { "终端屏幕已清空" } else { "Terminal screen cleared" }.to_string()));
    };

    let on_scroll_bottom = move |_| {
        scroll_terminal_to_bottom();
    };

    // Helper to get current crop metrics
    let get_crop_metrics = move || {
        let (scroll_top, client_w, client_h) = get_terminal_screen_metrics();
        ScreenCropMetrics {
            crop_x: crop_x.get_untracked(),
            crop_y: crop_y.get_untracked(),
            crop_w: crop_w.get_untracked(),
            crop_h: crop_h.get_untracked(),
            scroll_top,
            client_w,
            client_h,
        }
    };

    // Save full window
    let save_full = {
        let project_id = project_id.clone();
        move |format: &'static str| {
            let text = screen.get_untracked();
            if text.trim().is_empty() {
                status_msg.set(Some(if is_zh { "终端屏幕为空" } else { "Terminal screen is empty" }.to_string()));
                return;
            }
            saving.set(true);
            status_msg.set(Some(if is_zh { "正在生成并保存到 assets..." } else { "Generating asset and saving to assets..." }.to_string()));
            let pid = project_id.clone();

            if format == "svg" {
                let (svg, _, _) = generate_terminal_svg(&text, "apich-terminal", None);
                save_asset_to_backend(pid, svg, "svg", Some("terminal_full.svg"), saved_asset, saving, status_msg, is_zh);
            } else if format == "anim_svg" {
                let (anim_svg, _, _) = generate_animated_terminal_svg(&text, "apich-terminal", None);
                save_asset_to_backend(pid, anim_svg, "svg", Some("terminal_full_anim.svg"), saved_asset, saving, status_msg, is_zh);
            } else {
                // PNG
                let (svg, w, h) = generate_terminal_svg(&text, "apich-terminal", None);
                render_svg_to_png_and_save(pid, svg, w, h, saved_asset, saving, status_msg, is_zh);
            }
        }
    };

    // Save cropped selection box
    let save_crop = {
        let project_id = project_id.clone();
        move |format: &'static str| {
            let text = screen.get_untracked();
            if text.trim().is_empty() {
                status_msg.set(Some(if is_zh { "终端屏幕为空" } else { "Terminal screen is empty" }.to_string()));
                return;
            }
            let metrics = get_crop_metrics();
            let (cropped_text, target_w) = extract_cropped_text(&text, metrics);
            if cropped_text.trim().is_empty() {
                status_msg.set(Some(if is_zh { "选区内无文本内容" } else { "No text in selected region" }.to_string()));
                return;
            }

            saving.set(true);
            status_msg.set(Some(if is_zh { "正在保存选区截图..." } else { "Saving cropped snapshot to assets..." }.to_string()));
            let pid = project_id.clone();

            if format == "svg" {
                let (svg, _, _) = generate_terminal_svg(&cropped_text, "apich-terminal", Some(target_w));
                save_asset_to_backend(pid, svg, "svg", Some("terminal_crop.svg"), saved_asset, saving, status_msg, is_zh);
            } else if format == "anim_svg" {
                let (anim_svg, _, _) = generate_animated_terminal_svg(&cropped_text, "apich-terminal", Some(target_w));
                save_asset_to_backend(pid, anim_svg, "svg", Some("terminal_crop_anim.svg"), saved_asset, saving, status_msg, is_zh);
            } else {
                // PNG
                let (svg, w, h) = generate_terminal_svg(&cropped_text, "apich-terminal", Some(target_w));
                render_svg_to_png_and_save(pid, svg, w, h, saved_asset, saving, status_msg, is_zh);
            }
        }
    };

    // Top buttons adapt based on whether crop box is active
    let on_snapshot_click = {
        let sf = save_full.clone();
        let sc = save_crop.clone();
        move |_| {
            if crop_active.get_untracked() {
                sc("png");
            } else {
                sf("png");
            }
        }
    };

    let on_animate_click = {
        let sf = save_full.clone();
        let sc = save_crop.clone();
        move |_| {
            if crop_active.get_untracked() {
                sc("anim_svg");
            } else {
                sf("anim_svg");
            }
        }
    };

    // Toggle live session recording (crops frames if crop_active is true)
    let toggle_recording = move |_| {
        if is_recording.get_untracked() {
                is_recording.set(false);
                let frames = rec_frames.get_value();
                if frames.is_empty() {
                    status_msg.set(Some(if is_zh { "无录制帧" } else { "No frames recorded" }.to_string()));
                    return;
                }
                saving.set(true);
                let is_crop = crop_active.get_untracked();
                let crop_opt = if is_crop { Some(get_crop_metrics()) } else { None };

                status_msg.set(Some(if is_zh {
                    if is_crop { "正在生成选区录屏动画并保存..." } else { "正在生成录屏动画并保存..." }
                } else {
                    if is_crop { "Generating cropped animation to assets..." } else { "Generating recorded animation to assets..." }
                }.to_string()));

                let pid = project_id.clone();
                let anim_svg = compile_recorded_frames_to_svg(&frames, "apich-terminal", crop_opt);
                let asset_name = if is_crop { "terminal_crop_anim.svg" } else { "terminal_live_anim.svg" };
                save_asset_to_backend(pid, anim_svg, "svg", Some(asset_name), saved_asset, saving, status_msg, is_zh);
            } else {
                // Start recording
                let text = screen.get_untracked();
                rec_frames.set_value(vec![(0.0, text)]);
                rec_start_time.set_value(current_time_ms());
                is_recording.set(true);
                let is_crop = crop_active.get_untracked();
                status_msg.set(Some(if is_zh {
                    if is_crop { "🔴 录屏已开始（仅录制选区区域），请在终端输入命令..." } else { "🔴 录屏已开始，请在终端输入命令..." }
                } else {
                    if is_crop { "🔴 Recording started (recording cropped area only)..." } else { "🔴 Recording started, run commands in terminal..." }
                }.to_string()));
            }
        };

    // Drag start handler for selection box or handles
    let start_drag = move |handle: DragHandle, ev: leptos::ev::MouseEvent| {
        ev.stop_propagation();
        ev.prevent_default();
        drag_start.set_value(Some(DragStart {
            handle,
            mouse_x: f64::from(ev.client_x()),
            mouse_y: f64::from(ev.client_y()),
            init_x: crop_x.get_untracked(),
            init_y: crop_y.get_untracked(),
            init_w: crop_w.get_untracked(),
            init_h: crop_h.get_untracked(),
        }));
    };

    // Mouse move handler on container
    let on_container_mousemove = move |ev: leptos::ev::MouseEvent| {
        if let Some(ds) = drag_start.get_value() {
            let dx = (f64::from(ev.client_x()) - ds.mouse_x) as i32;
            let dy = (f64::from(ev.client_y()) - ds.mouse_y) as i32;
            let (_, client_w, client_h) = get_terminal_screen_metrics();
            let max_bound_w = client_w as i32;
            let max_bound_h = client_h as i32;

            match ds.handle {
                DragHandle::Move => {
                    let cur_w = crop_w.get_untracked();
                    let cur_h = crop_h.get_untracked();
                    let max_x = max_bound_w.saturating_sub(cur_w).max(0);
                    let max_y = max_bound_h.saturating_sub(cur_h).max(0);
                    crop_x.set(ds.init_x.saturating_add(dx).clamp(0, max_x));
                    crop_y.set(ds.init_y.saturating_add(dy).clamp(0, max_y));
                }
                DragHandle::E => {
                    let cur_x = crop_x.get_untracked();
                    let max_w = max_bound_w.saturating_sub(cur_x).max(120);
                    crop_w.set(ds.init_w.saturating_add(dx).clamp(120, max_w));
                }
                DragHandle::S => {
                    let cur_y = crop_y.get_untracked();
                    let max_h = max_bound_h.saturating_sub(cur_y).max(60);
                    crop_h.set(ds.init_h.saturating_add(dy).clamp(60, max_h));
                }
                DragHandle::Se => {
                    let cur_x = crop_x.get_untracked();
                    let cur_y = crop_y.get_untracked();
                    let max_w = max_bound_w.saturating_sub(cur_x).max(120);
                    let max_h = max_bound_h.saturating_sub(cur_y).max(60);
                    crop_w.set(ds.init_w.saturating_add(dx).clamp(120, max_w));
                    crop_h.set(ds.init_h.saturating_add(dy).clamp(60, max_h));
                }
                DragHandle::W => {
                    let cur_right = ds.init_x.saturating_add(ds.init_w);
                    let new_x = ds.init_x.saturating_add(dx).clamp(0, cur_right.saturating_sub(120));
                    crop_x.set(new_x);
                    crop_w.set(cur_right.saturating_sub(new_x));
                }
                DragHandle::N => {
                    let cur_bottom = ds.init_y.saturating_add(ds.init_h);
                    let new_y = ds.init_y.saturating_add(dy).clamp(0, cur_bottom.saturating_sub(60));
                    crop_y.set(new_y);
                    crop_h.set(cur_bottom.saturating_sub(new_y));
                }
                DragHandle::Nw => {
                    let cur_right = ds.init_x.saturating_add(ds.init_w);
                    let cur_bottom = ds.init_y.saturating_add(ds.init_h);
                    let new_x = ds.init_x.saturating_add(dx).clamp(0, cur_right.saturating_sub(120));
                    let new_y = ds.init_y.saturating_add(dy).clamp(0, cur_bottom.saturating_sub(60));
                    crop_x.set(new_x);
                    crop_y.set(new_y);
                    crop_w.set(cur_right.saturating_sub(new_x));
                    crop_h.set(cur_bottom.saturating_sub(new_y));
                }
                DragHandle::Ne => {
                    let cur_x = crop_x.get_untracked();
                    let cur_bottom = ds.init_y.saturating_add(ds.init_h);
                    let max_w = max_bound_w.saturating_sub(cur_x).max(120);
                    let new_y = ds.init_y.saturating_add(dy).clamp(0, cur_bottom.saturating_sub(60));
                    crop_w.set(ds.init_w.saturating_add(dx).clamp(120, max_w));
                    crop_y.set(new_y);
                    crop_h.set(cur_bottom.saturating_sub(new_y));
                }
                DragHandle::Sw => {
                    let cur_right = ds.init_x.saturating_add(ds.init_w);
                    let cur_y = crop_y.get_untracked();
                    let max_h = max_bound_h.saturating_sub(cur_y).max(60);
                    let new_x = ds.init_x.saturating_add(dx).clamp(0, cur_right.saturating_sub(120));
                    crop_x.set(new_x);
                    crop_w.set(cur_right.saturating_sub(new_x));
                    crop_h.set(ds.init_h.saturating_add(dy).clamp(60, max_h));
                }
            }
        }
    };

    let on_container_mouseup = move |_| {
        drag_start.set_value(None);
    };

    let quick_commands = [
        "typst --version",
        "python3 --version",
        "git status",
        "ls -la",
    ];

    view! {
        // Toolbar with Quick Commands & Capture Tools
        <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:0.75rem; flex-wrap:wrap; gap:0.6rem;">
            // Left: Quick Commands
            <div style="display:flex; gap:0.4rem; flex-wrap:wrap; align-items:center;">
                <span style="font-size:0.75rem; color:#94a3b8; margin-right:0.25rem; display:flex; align-items:center;">{crate::t(is_zh, "Quick Commands:", "快捷命令：")}</span>
                {quick_commands.into_iter().map(|cmd| {
                    view! {
                        <button
                            type="button"
                            class="btn btn-secondary btn-sm"
                            style="background:#1e293b; color:#cbd5e1; border-color:#334155; font-family:var(--font-mono); font-size:0.75rem;"
                            disabled=move || !hydrated.get()
                            on:click=move |_| input.set(cmd.to_string())
                        >
                            {cmd}
                        </button>
                    }
                }).collect::<Vec<_>>()}
                <span
                    style=move || format!(
                        "font-size:0.72rem; color:#f59e0b; align-items:center; gap:0.3rem; display:{};",
                        if hydrated.get() { "none" } else { "flex" }
                    )
                >
                    "⏳ "{crate::t(is_zh, "Loading terminal...", "终端加载中…")}
                </span>
            </div>

            // Right: Capture & Export Tools
            <div style="display:flex; gap:0.4rem; flex-wrap:wrap; align-items:center;">
                <button
                    type="button"
                    class="btn btn-secondary btn-sm"
                    style="background:#1e293b; color:#38bdf8; border-color:#334155; font-size:0.75rem; display:flex; align-items:center; gap:0.3rem;"
                    disabled=move || saving.get() || !hydrated.get()
                    title={crate::t(is_zh, "Capture to PNG for paper/docs (full or cropped if selection active)", "截图为 PNG 图片并存入 assets（选区开启时自动截取选区）")}
                    on:click=on_snapshot_click
                >
                    "📸 "{move || if crop_active.get() {
                        crate::t(is_zh, "Snapshot (Cropped)", "选区截图 PNG")
                    } else {
                        crate::t(is_zh, "Snapshot PNG", "截图 PNG")
                    }}
                </button>

                <button
                    type="button"
                    class="btn btn-secondary btn-sm"
                    style=move || format!(
                        "background:{}; color:{}; border-color:#334155; font-size:0.75rem; display:flex; align-items:center; gap:0.3rem;",
                        if crop_active.get() { "#0369a1" } else { "#1e293b" },
                        if crop_active.get() { "#ffffff" } else { "#a5f3fc" }
                    )
                    disabled=move || saving.get() || !hydrated.get()
                    title={crate::t(is_zh, "Toggle draggable & resizable box to select area for snapshot or animation", "开启/关闭可拖拽改变大小的框选截屏与动画选区")}
                    on:click=move |_| crop_active.update(|v| *v = !*v)
                >
                    "✂️ "{move || if crop_active.get() {
                        crate::t(is_zh, "Close Box", "关闭选区")
                    } else {
                        crate::t(is_zh, "Select Area", "框选区域")
                    }}
                </button>

                <button
                    type="button"
                    class="btn btn-secondary btn-sm"
                    style="background:#1e293b; color:#cbd5e1; border-color:#334155; font-size:0.75rem; display:flex; align-items:center; gap:0.3rem;"
                    disabled=move || saving.get() || !hydrated.get()
                    title={crate::t(is_zh, "Generate Animated SVG for slides (full or cropped if selection active)", "生成动画 SVG 存入 assets（选区开启时仅生成选区动画）")}
                    on:click=on_animate_click
                >
                    "🎬 "{move || if crop_active.get() {
                        crate::t(is_zh, "Animate (Cropped)", "选区动画 SVG")
                    } else {
                        crate::t(is_zh, "Animate SVG", "动画 SVG")
                    }}
                </button>

                <button
                    type="button"
                    class="btn btn-secondary btn-sm"
                    style=move || format!(
                        "background:{}; color:{}; border-color:#334155; font-size:0.75rem; display:flex; align-items:center; gap:0.3rem;",
                        if is_recording.get() { "#7f1d1d" } else { "#1e293b" },
                        if is_recording.get() { "#fca5a5" } else { "#cbd5e1" }
                    )
                    disabled=move || saving.get() || !hydrated.get()
                    title={crate::t(is_zh, "Record live terminal session into animated SVG", "实时录制终端会话为动态 SVG（开启选区时仅录制选区）")}
                    on:click=toggle_recording
                >
                    {move || if is_recording.get() {
                        view! {
                            <span class="term-rec-pulse"></span>
                            <span>{crate::t(is_zh, "Stop & Save", "停止并保存")}</span>
                        }.into_any()
                    } else {
                        view! {
                            <span>"⏺ "</span>
                            <span>{if crop_active.get() {
                                crate::t(is_zh, "Record Cropped", "录制选区")
                            } else {
                                crate::t(is_zh, "Record Live", "录制会话")
                            }}</span>
                        }.into_any()
                    }}
                </button>

                <button
                    type="button"
                    class="btn btn-secondary btn-sm"
                    style="background:#1e293b; color:#94a3b8; border-color:#334155; font-size:0.75rem;"
                    disabled=move || !hydrated.get()
                    title={crate::t(is_zh, "Clear terminal screen", "清空终端输出")}
                    on:click=on_clear
                >
                    "🧹 "{crate::t(is_zh, "Clear", "清屏")}
                </button>

                <button
                    type="button"
                    class="btn btn-secondary btn-sm"
                    style="background:#1e293b; color:#94a3b8; border-color:#334155; font-size:0.75rem;"
                    title={crate::t(is_zh, "Scroll to bottom", "滚动至最底部")}
                    on:click=on_scroll_bottom
                >
                    "⬇️"
                </button>
            </div>
        </div>

        // Status or progress notification
        {move || status_msg.get().map(|msg| {
            view! {
                <div style="font-size:0.8rem; color:#38bdf8; background:#0f172a; padding:0.4rem 0.8rem; border-radius:6px; border:1px solid #1e293b; margin-bottom:0.5rem; display:flex; justify-content:space-between; align-items:center;">
                    <span>{msg}</span>
                    <button type="button" style="background:none; border:none; color:#64748b; cursor:pointer;" on:click=move |_| status_msg.set(None)>"✕"</button>
                </div>
            }
        })}

        // Terminal Screen Wrapper with Interactive Draggable/Resizable Crop Overlay
        <div
            class="terminal-screen-wrapper"
            on:mousemove=on_container_mousemove
            on:mouseup=on_container_mouseup
        >
            <div id="term-screen" class="terminal-screen">{move || screen.get()}</div>

            // Draggable & Resizable Selection Box
            {move || if crop_active.get() {
                let sc = save_crop.clone();
                let sc_png = sc.clone();
                let sc_svg = sc.clone();
                let sc_anim = sc.clone();

                Some(view! {
                    <div class="term-crop-overlay">
                        <div
                            class="term-crop-box"
                            style=move || format!(
                                "left:{}px; top:{}px; width:{}px; height:{}px;",
                                crop_x.get(), crop_y.get(), crop_w.get(), crop_h.get()
                            )
                            on:mousedown=move |ev| start_drag(DragHandle::Move, ev)
                        >
                            // Dimension badge (flips inside if near top)
                            <div
                                class="term-crop-badge"
                                style=move || if crop_y.get() < 28 { "top:6px; left:8px;" } else { "top:-26px; left:0;" }
                            >
                                {move || format!("{} × {} px (Drag & Resize)", crop_w.get(), crop_h.get())}
                            </div>

                            // 8 Resize Handles
                            <div class="term-crop-handle handle-nw" on:mousedown=move |ev| start_drag(DragHandle::Nw, ev)></div>
                            <div class="term-crop-handle handle-n"  on:mousedown=move |ev| start_drag(DragHandle::N, ev)></div>
                            <div class="term-crop-handle handle-ne" on:mousedown=move |ev| start_drag(DragHandle::Ne, ev)></div>
                            <div class="term-crop-handle handle-e"  on:mousedown=move |ev| start_drag(DragHandle::E, ev)></div>
                            <div class="term-crop-handle handle-se" on:mousedown=move |ev| start_drag(DragHandle::Se, ev)></div>
                            <div class="term-crop-handle handle-s"  on:mousedown=move |ev| start_drag(DragHandle::S, ev)></div>
                            <div class="term-crop-handle handle-sw" on:mousedown=move |ev| start_drag(DragHandle::Sw, ev)></div>
                            <div class="term-crop-handle handle-w"  on:mousedown=move |ev| start_drag(DragHandle::W, ev)></div>

                            // Floating Crop Action Toolbar (safely placed inside crop box)
                            <div
                                class="term-crop-actions"
                                class:term-crop-actions-top=move || crop_h.get() < 95
                                style=move || if crop_h.get() < 95 { "bottom:auto; top:8px;" } else { "bottom:8px; top:auto;" }
                                on:mousedown=move |ev| ev.stop_propagation()
                            >
                                <button
                                    type="button"
                                    class="btn btn-primary btn-xs"
                                    style="font-size:0.72rem; padding:0.25rem 0.5rem;"
                                    disabled=move || saving.get()
                                    title={crate::t(is_zh, "Save cropped region to PNG in assets", "保存选区为 PNG 图片")}
                                    on:click=move |_| sc_png("png")
                                >
                                    "📸 "{crate::t(is_zh, "Save PNG", "保存 PNG")}
                                </button>
                                <button
                                    type="button"
                                    class="btn btn-secondary btn-xs"
                                    style="font-size:0.72rem; padding:0.25rem 0.5rem; background:#1e293b; color:#cbd5e1;"
                                    disabled=move || saving.get()
                                    title={crate::t(is_zh, "Save cropped region to vector SVG in assets", "保存选区为矢量 SVG")}
                                    on:click=move |_| sc_svg("svg")
                                >
                                    "📐 "{crate::t(is_zh, "Save SVG", "保存 SVG")}
                                </button>
                                <button
                                    type="button"
                                    class="btn btn-secondary btn-xs"
                                    style="font-size:0.72rem; padding:0.25rem 0.5rem; background:#1e293b; color:#cbd5e1;"
                                    disabled=move || saving.get()
                                    title={crate::t(is_zh, "Save cropped region as Animated SVG", "保存选区为动画 SVG")}
                                    on:click=move |_| sc_anim("anim_svg")
                                >
                                    "🎬 "{crate::t(is_zh, "Animate", "动画 SVG")}
                                </button>
                                <button
                                    type="button"
                                    class="btn btn-secondary btn-xs"
                                    style="font-size:0.72rem; padding:0.25rem 0.5rem; background:#1e293b; color:#94a3b8;"
                                    on:click=move |_| crop_active.set(false)
                                >
                                    "✕"
                                </button>
                            </div>
                        </div>
                    </div>
                })
            } else {
                None
            }}
        </div>

        // Input Command Bar (Taller, spacious 54px modern bar)
        <form on:submit=move |ev| { ev.prevent_default(); submit(); }>
            <div class="terminal-bar">
                <span style="color:#38bdf8; font-family:var(--font-mono); display:flex; align-items:center; font-weight:700; font-size:1.1rem;">"$"</span>
                <input
                    type="text"
                    class="terminal-input"
                    placeholder=move || if hydrated.get() {
                        crate::t(is_zh, "Type command... (e.g. typst compile main.typ paper.pdf, python3 test.py, cargo build)", "输入命令...（例如：typst compile main.typ paper.pdf, python3 test.py, cargo build）")
                    } else {
                        crate::t(is_zh, "Loading terminal, please wait...", "终端加载中，请稍候…")
                    }
                    autocomplete="off"
                    autofocus=true
                    disabled=move || !hydrated.get()
                    prop:value=move || input.get()
                    on:input=move |ev| input.set(event_target_value(&ev))
                />
                <button type="submit" class="btn btn-primary" disabled=move || busy.get() || !hydrated.get()>{crate::t(is_zh, "Run", "运行")}</button>
            </div>
        </form>

        // Saved Asset Confirmation Card with Snippets for Papers & Slides
        {move || saved_asset.get().map(|info| {
            let typst_code = info.typst_snippet.clone();
            let md_code = info.markdown_snippet.clone();
            let latex_code = info.latex_snippet.clone();

            view! {
                <div class="term-saved-card">
                    <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:0.75rem;">
                        <div style="display:flex; align-items:center; gap:0.5rem;">
                            <span style="color:#4ade80; font-size:1.1rem; font-weight:700;">"✓"</span>
                            <span style="font-weight:600; color:#f8fafc;">{info.message.clone()}</span>
                            <span class="file-type-pill pill-doc" style="font-size:0.65rem; text-transform:uppercase;">{info.format}</span>
                        </div>
                        <button
                            type="button"
                            style="background:none; border:none; color:#94a3b8; cursor:pointer; font-size:1rem;"
                            on:click=move |_| saved_asset.set(None)
                        >
                            "✕"
                        </button>
                    </div>

                    <div style="font-size:0.8rem; color:#94a3b8; margin-bottom:0.6rem;">
                        {crate::t(is_zh, "已保存到项目 assets/ 目录，可直接在论文或幻灯片中插入：", "Saved to project assets/ folder. Click to copy snippet for your paper or slides:")}
                    </div>

                    // Typst Snippet
                    <div class="term-snippet-row">
                        <div class="term-snippet-label">
                            <span>"📄 Typst (Paper / Slide):"</span>
                        </div>
                        <div class="term-snippet-box">
                            <code>{typst_code.clone()}</code>
                            <button
                                type="button"
                                class="btn btn-secondary btn-xs"
                                style="background:#1e293b; color:#cbd5e1;"
                                on:click={let c = typst_code; move |_| copy_to_clipboard(&c)}
                            >
                                {crate::t(is_zh, "复制", "Copy")}
                            </button>
                        </div>
                    </div>

                    // Markdown Snippet
                    <div class="term-snippet-row">
                        <div class="term-snippet-label">
                            <span>"📝 Markdown / Note:"</span>
                        </div>
                        <div class="term-snippet-box">
                            <code>{md_code.clone()}</code>
                            <button
                                type="button"
                                class="btn btn-secondary btn-xs"
                                style="background:#1e293b; color:#cbd5e1;"
                                on:click={let c = md_code; move |_| copy_to_clipboard(&c)}
                            >
                                {crate::t(is_zh, "复制", "Copy")}
                            </button>
                        </div>
                    </div>

                    // LaTeX Snippet
                    <div class="term-snippet-row">
                        <div class="term-snippet-label">
                            <span>"📑 LaTeX:"</span>
                        </div>
                        <div class="term-snippet-box">
                            <code>{latex_code.clone()}</code>
                            <button
                                type="button"
                                class="btn btn-secondary btn-xs"
                                style="background:#1e293b; color:#cbd5e1;"
                                on:click={let c = latex_code; move |_| copy_to_clipboard(&c)}
                            >
                                {crate::t(is_zh, "复制", "Copy")}
                            </button>
                        </div>
                    </div>
                </div>
            }
        })}
    }
}

// -----------------------------------------------------------------------------
// Auto Scroll & DOM Helpers
// -----------------------------------------------------------------------------

#[cfg(feature = "hydrate")]
fn scroll_terminal_to_bottom() {
    use wasm_bindgen::JsCast;
    if let Some(win) = web_sys::window() {
        let cb = wasm_bindgen::closure::Closure::once(move || {
            if let Some(win) = web_sys::window() {
                if let Some(doc) = win.document() {
                    if let Some(el) = doc.get_element_by_id("term-screen") {
                        el.set_scroll_top(el.scroll_height());
                    }
                }
            }
        });
        let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
        cb.forget();
    }
}

#[cfg(not(feature = "hydrate"))]
const fn scroll_terminal_to_bottom() {}

#[cfg(feature = "hydrate")]
fn get_terminal_screen_metrics() -> (f64, f64, f64) {
    if let Some(win) = web_sys::window() {
        if let Some(doc) = win.document() {
            if let Some(el) = doc.get_element_by_id("term-screen") {
                return (
                    el.scroll_top() as f64,
                    el.client_width() as f64,
                    el.client_height() as f64,
                );
            }
        }
    }
    (0.0, 800.0, 560.0)
}

#[cfg(not(feature = "hydrate"))]
const fn get_terminal_screen_metrics() -> (f64, f64, f64) {
    (0.0, 800.0, 560.0)
}

#[cfg(feature = "hydrate")]
fn current_time_ms() -> f64 {
    js_sys::Date::now()
}

#[cfg(not(feature = "hydrate"))]
const fn current_time_ms() -> f64 {
    0.0
}

#[cfg(feature = "hydrate")]
fn copy_to_clipboard(text: &str) {
    if let Some(win) = web_sys::window() {
        let nav = win.navigator();
        let clip = nav.clipboard();
        let _ = clip.write_text(text);
    }
}

#[cfg(not(feature = "hydrate"))]
const fn copy_to_clipboard(_text: &str) {}

// -----------------------------------------------------------------------------
// Crop Text Extraction Logic
// -----------------------------------------------------------------------------

/// Extracts precisely the text lines and character columns that are visible inside
/// the user's draggable/resizable crop box, taking scroll position into account.
#[allow(clippy::cast_precision_loss)]
fn extract_cropped_text(text: &str, crop: ScreenCropMetrics) -> (String, u32) {
    let all_lines: Vec<&str> = text.lines().collect();
    if all_lines.is_empty() {
        return (String::new(), (crop.crop_w as u32).clamp(480, 1100));
    }

    // Parameters matching #term-screen CSS (padding: 1.25rem = 20px, font: 0.85rem ~13.6px * 1.6 = 21.76px)
    let line_height_px = 21.76f64;
    let padding_top_px = 20.0f64;
    let padding_left_px = 20.0f64;
    let char_width_px = 8.16f64;

    let content_top = (f64::from(crop.crop_y) + crop.scroll_top - padding_top_px).max(0.0);
    let content_bottom = content_top + f64::from(crop.crop_h);

    let start_line = (content_top / line_height_px).floor() as usize;
    let end_line = ((content_bottom / line_height_px).ceil() as usize).max(start_line.saturating_add(1));

    let total_lines = all_lines.len();
    let s_line = start_line.min(total_lines.saturating_sub(1));
    let e_line = end_line.min(total_lines).max(s_line.saturating_add(1));

    let left_in_content = (f64::from(crop.crop_x) - padding_left_px).max(0.0);
    let start_col = (left_in_content / char_width_px).floor() as usize;
    let col_count = (f64::from(crop.crop_w) / char_width_px).ceil() as usize;
    let end_col = start_col.saturating_add(col_count);

    let mut cropped_lines = Vec::new();
    if let Some(slice) = all_lines.get(s_line..e_line) {
        for line in slice {
            let chars: Vec<char> = line.chars().collect();
            if chars.is_empty() {
                cropped_lines.push(String::new());
                continue;
            }

            // If user positioned the box horizontally past the first 4 columns, slice columns
            if start_col > 4 && start_col < chars.len() {
                let sc = start_col.min(chars.len());
                let ec = end_col.min(chars.len());
                if ec > sc {
                    if let Some(sub) = chars.get(sc..ec) {
                        cropped_lines.push(sub.iter().collect());
                    } else {
                        cropped_lines.push(String::new());
                    }
                } else {
                    cropped_lines.push(String::new());
                }
            } else if end_col < chars.len() && f64::from(crop.crop_w) < crop.client_w * 0.85 {
                let ec = end_col.min(chars.len());
                if let Some(sub) = chars.get(..ec) {
                    cropped_lines.push(sub.iter().collect());
                } else {
                    cropped_lines.push(chars.iter().collect());
                }
            } else {
                // Full line
                cropped_lines.push((*line).to_string());
            }
        }
    }

    let target_width = (crop.crop_w as u32).clamp(480, 1100);
    (cropped_lines.join("\n"), target_width)
}

// -----------------------------------------------------------------------------
// SVG & PNG Generation Logic
// -----------------------------------------------------------------------------

/// Generates a clean, standalone terminal vector SVG with macOS title bar and syntax coloring.
/// If `target_width` is specified, the SVG adapts to that width.
#[allow(clippy::cast_precision_loss)]
fn generate_terminal_svg(
    text: &str,
    title: &str,
    target_width: Option<u32>,
) -> (String, u32, u32) {
    let raw_lines: Vec<&str> = text.lines().collect();
    let font_size = 13u32;
    let line_height = 21u32;
    let char_width = 8.0f64;
    let padding_x = 24u32;
    let header_height = 42u32;
    let padding_bottom = 24u32;

    let max_line_len = raw_lines
        .iter()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(40)
        .max(45);

    let natural_width = (((max_line_len as f64) * char_width) as u32)
        .saturating_add(padding_x.saturating_mul(2))
        .clamp(520, 1100);
    let full_width = target_width.unwrap_or(natural_width).clamp(480, 1100);
    let full_height = header_height
        .saturating_add((raw_lines.len() as u32).saturating_mul(line_height).max(60))
        .saturating_add(padding_bottom);

    let mut svg = String::with_capacity(4096);
    let _ = write!(
        svg,
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {full_width} {full_height}" width="{full_width}" height="{full_height}">
  <rect x="0" y="0" width="{full_width}" height="{full_height}" rx="10" ry="10" fill="#090d16" stroke="#1e293b" stroke-width="1"/>
  <!-- Window Header -->
  <path d="M 0 10 C 0 4.477 4.477 0 10 0 L {w_minus_10} 0 C {w_minus_0} 0 {full_width} 4.477 {full_width} 10 L {full_width} {header_height} L 0 {header_height} Z" fill="#0f172a"/>
  <line x1="0" y1="{header_height}" x2="{full_width}" y2="{header_height}" stroke="#1e293b" stroke-width="1"/>
  <circle cx="20" cy="21" r="6" fill="#ff5f56"/>
  <circle cx="38" cy="21" r="6" fill="#ffbd2e"/>
  <circle cx="56" cy="21" r="6" fill="#27c93f"/>
  <text x="{center_x}" y="25" fill="#94a3b8" font-family="ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace" font-size="12" font-weight="600" text-anchor="middle">{escaped_title}</text>
  <g font-family="ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace" font-size="{font_size}">
"##,
        full_width = full_width,
        full_height = full_height,
        w_minus_10 = full_width.saturating_sub(10),
        w_minus_0 = full_width,
        header_height = header_height,
        center_x = full_width / 2,
        escaped_title = xml_escape(title),
        font_size = font_size,
    );

    for (i, line) in raw_lines.iter().enumerate() {
        let y = header_height.saturating_add(24).saturating_add((i as u32).saturating_mul(line_height));
        let trimmed = line.trim_start();
        let (fill, font_weight) = if trimmed.starts_with('$') {
            ("#38bdf8", "700")
        } else if trimmed.starts_with("[Error]") || trimmed.contains("error:") || trimmed.contains("Error:") {
            ("#f87171", "600")
        } else if trimmed.starts_with('✓') || trimmed.starts_with("[Success]") {
            ("#4ade80", "600")
        } else if trimmed.starts_with('#') {
            ("#94a3b8", "400")
        } else {
            ("#cbd5e1", "400")
        };

        let _ = writeln!(
            svg,
            r#"    <text x="{padding_x}" y="{y}" fill="{fill}" font-weight="{font_weight}" xml:space="preserve">{text}</text>"#,
            padding_x = padding_x,
            y = y,
            fill = fill,
            font_weight = font_weight,
            text = xml_escape(line)
        );
    }

    svg.push_str("  </g>\n</svg>");
    (svg, full_width, full_height)
}

/// Generates an animated SVG where lines stream out sequentially and loop cleanly.
#[allow(clippy::cast_precision_loss)]
fn generate_animated_terminal_svg(
    text: &str,
    title: &str,
    target_width: Option<u32>,
) -> (String, u32, u32) {
    let raw_lines: Vec<&str> = text.lines().collect();
    let n = raw_lines.len().max(1);
    let font_size = 13u32;
    let line_height = 21u32;
    let char_width = 8.0f64;
    let padding_x = 24u32;
    let header_height = 42u32;
    let padding_bottom = 24u32;

    let max_line_len = raw_lines
        .iter()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(40)
        .max(45);

    let natural_width = (((max_line_len as f64) * char_width) as u32)
        .saturating_add(padding_x.saturating_mul(2))
        .clamp(520, 1100);
    let full_width = target_width.unwrap_or(natural_width).clamp(480, 1100);
    let full_height = header_height
        .saturating_add((raw_lines.len() as u32).saturating_mul(line_height).max(60))
        .saturating_add(padding_bottom);

    let total_dur = ((n as f64 * 0.4).max(3.5) + 3.0).min(18.0);
    let hold_ratio = ((total_dur - 1.5) / total_dur).clamp(0.65, 0.92);

    let mut svg = String::with_capacity(4096);
    let _ = write!(
        svg,
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {full_width} {full_height}" width="{full_width}" height="{full_height}">
  <rect x="0" y="0" width="{full_width}" height="{full_height}" rx="10" ry="10" fill="#090d16" stroke="#1e293b" stroke-width="1"/>
  <path d="M 0 10 C 0 4.477 4.477 0 10 0 L {w_minus_10} 0 C {w_minus_0} 0 {full_width} 4.477 {full_width} 10 L {full_width} {header_height} L 0 {header_height} Z" fill="#0f172a"/>
  <line x1="0" y1="{header_height}" x2="{full_width}" y2="{header_height}" stroke="#1e293b" stroke-width="1"/>
  <circle cx="20" cy="21" r="6" fill="#ff5f56"/>
  <circle cx="38" cy="21" r="6" fill="#ffbd2e"/>
  <circle cx="56" cy="21" r="6" fill="#27c93f"/>
  <text x="{center_x}" y="25" fill="#94a3b8" font-family="ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace" font-size="12" font-weight="600" text-anchor="middle">{escaped_title}</text>
  <g font-family="ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace" font-size="{font_size}">
"##,
        full_width = full_width,
        full_height = full_height,
        w_minus_10 = full_width.saturating_sub(10),
        w_minus_0 = full_width,
        header_height = header_height,
        center_x = full_width / 2,
        escaped_title = xml_escape(title),
        font_size = font_size,
    );

    for (i, line) in raw_lines.iter().enumerate() {
        let y = header_height.saturating_add(24).saturating_add((i as u32).saturating_mul(line_height));
        let trimmed = line.trim_start();
        let (fill, font_weight) = if trimmed.starts_with('$') {
            ("#38bdf8", "700")
        } else if trimmed.starts_with("[Error]") || trimmed.contains("error:") {
            ("#f87171", "600")
        } else if trimmed.starts_with('✓') {
            ("#4ade80", "600")
        } else {
            ("#cbd5e1", "400")
        };

        let start_ratio = ((i as f64 / n as f64) * (hold_ratio - 0.15)).clamp(0.0, 0.85);
        let appear_ratio = (start_ratio + 0.04).min(hold_ratio);

        let _ = writeln!(
            svg,
            r#"    <text x="{padding_x}" y="{y}" fill="{fill}" font-weight="{font_weight}" xml:space="preserve" opacity="0">
      <animate attributeName="opacity" dur="{total_dur:.1}s" values="0;0;1;1;0" keyTimes="0;{start_ratio:.3};{appear_ratio:.3};{hold_ratio:.3};1" repeatCount="indefinite"/>
      {text}
    </text>"#,
            padding_x = padding_x,
            y = y,
            fill = fill,
            font_weight = font_weight,
            total_dur = total_dur,
            start_ratio = start_ratio,
            appear_ratio = appear_ratio,
            hold_ratio = hold_ratio,
            text = xml_escape(line)
        );
    }

    svg.push_str("  </g>\n</svg>");
    (svg, full_width, full_height)
}

/// Compiles timestamped recorded frames into an animated SVG.
/// If `crop` is specified, each frame's text is cropped to that window!
fn compile_recorded_frames_to_svg(
    frames: &[(f64, String)],
    title: &str,
    crop: Option<ScreenCropMetrics>,
) -> String {
    if frames.is_empty() {
        return generate_terminal_svg("", title, None).0;
    }

    let mut combined_text = String::new();
    for (_, text) in frames {
        if text.len() > combined_text.len() {
            combined_text.clone_from(text);
        }
    }

    crop.map_or_else(
        || generate_animated_terminal_svg(&combined_text, title, None).0,
        |metrics| {
            let (cropped_text, target_w) = extract_cropped_text(&combined_text, metrics);
            generate_animated_terminal_svg(&cropped_text, title, Some(target_w)).0
        },
    )
}

fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            | '&' => out.push_str("&amp;"),
            | '<' => out.push_str("&lt;"),
            | '>' => out.push_str("&gt;"),
            | '"' => out.push_str("&quot;"),
            | '\'' => out.push_str("&apos;"),
            | _ => out.push(c),
        }
    }
    out
}

// -----------------------------------------------------------------------------
// Backend Saving Helpers
// -----------------------------------------------------------------------------

#[cfg(feature = "hydrate")]
#[allow(clippy::too_many_arguments)]
fn save_asset_to_backend(
    project_id: String,
    data_content: String,
    format: &'static str,
    asset_name: Option<&'static str>,
    saved_asset: RwSignal<Option<SavedAssetInfo>>,
    saving: RwSignal<bool>,
    status_msg: RwSignal<Option<String>>,
    is_zh: bool,
) {
    wasm_bindgen_futures::spawn_local(async move {
        let body = serde_json::json!({
            "asset_name": asset_name,
            "data_url": data_content,
            "format": format
        });

        let result = gloo_net::http::Request::post(&format!(
            "/projects/{}/terminal/save-asset",
            project_id
        ))
        .json(&body)
        .expect("serializable body")
        .send()
        .await;

        match result {
            | Ok(resp) if resp.ok() => {
                if let Ok(info) = resp.json::<SavedAssetInfo>().await {
                    status_msg.set(Some(if is_zh {
                        format!("✓ 已保存至 {}", info.asset_path)
                    } else {
                        format!("✓ Successfully saved to {}", info.asset_path)
                    }));
                    saved_asset.set(Some(info));
                } else {
                    status_msg.set(Some(if is_zh { "✓ 已保存到 assets" } else { "✓ Saved to assets" }.to_string()));
                }
            }
            | Ok(resp) => {
                status_msg.set(Some(format!("Save failed ({})", resp.status())));
            }
            | Err(e) => {
                status_msg.set(Some(format!("Save error: {e}")));
            }
        }
        saving.set(false);
    });
}

#[cfg(not(feature = "hydrate"))]
#[allow(clippy::too_many_arguments)]
fn save_asset_to_backend(
    _project_id: String,
    _data_content: String,
    _format: &'static str,
    _asset_name: Option<&'static str>,
    _saved_asset: RwSignal<Option<SavedAssetInfo>>,
    saving: RwSignal<bool>,
    _status_msg: RwSignal<Option<String>>,
    _is_zh: bool,
) {
    saving.set(false);
}

#[cfg(feature = "hydrate")]
#[allow(clippy::too_many_arguments)]
fn render_svg_to_png_and_save(
    project_id: String,
    svg_content: String,
    width: u32,
    height: u32,
    saved_asset: RwSignal<Option<SavedAssetInfo>>,
    saving: RwSignal<bool>,
    status_msg: RwSignal<Option<String>>,
    is_zh: bool,
) {
    use wasm_bindgen::JsCast;

    let encoded_svg = js_sys::encode_uri_component(&svg_content);
    let data_url = format!("data:image/svg+xml;charset=utf-8,{}", encoded_svg);

    let Ok(img) = web_sys::HtmlImageElement::new() else {
        save_asset_to_backend(project_id, svg_content, "svg", None, saved_asset, saving, status_msg, is_zh);
        return;
    };

    let doc = web_sys::window().and_then(|w| w.document());
    let canvas = doc.and_then(|d| d.create_element("canvas").ok()).and_then(|c| c.dyn_into::<web_sys::HtmlCanvasElement>().ok());

    let Some(canvas) = canvas else {
        save_asset_to_backend(project_id, svg_content, "svg", None, saved_asset, saving, status_msg, is_zh);
        return;
    };

    let scale = 2u32; // 2x Retina scaling for crisp paper output
    canvas.set_width(width * scale);
    canvas.set_height(height * scale);

    let ctx = canvas.get_context("2d").ok().flatten().and_then(|c| c.dyn_into::<web_sys::CanvasRenderingContext2d>().ok());
    let Some(ctx) = ctx else {
        save_asset_to_backend(project_id, svg_content, "svg", None, saved_asset, saving, status_msg, is_zh);
        return;
    };
    let _ = ctx.scale(scale as f64, scale as f64);

    let img_clone = img.clone();
    let pid = project_id.clone();
    let canvas_clone = canvas.clone();

    let onload = wasm_bindgen::closure::Closure::<dyn Fn()>::new(move || {
        let _ = ctx.draw_image_with_html_image_element_and_dw_and_dh(
            &img_clone,
            0.0,
            0.0,
            width as f64,
            height as f64,
        );
        let png_data_url = canvas_clone.to_data_url().unwrap_or_default();
        if png_data_url.starts_with("data:image/png") {
            save_asset_to_backend(pid.clone(), png_data_url, "png", None, saved_asset, saving, status_msg, is_zh);
        } else {
            save_asset_to_backend(pid.clone(), svg_content.clone(), "svg", None, saved_asset, saving, status_msg, is_zh);
        }
    });

    img.set_onload(Some(onload.as_ref().unchecked_ref()));
    onload.forget();
    img.set_src(&data_url);
}

#[cfg(not(feature = "hydrate"))]
#[allow(clippy::too_many_arguments)]
fn render_svg_to_png_and_save(
    _project_id: String,
    _svg_content: String,
    _width: u32,
    _height: u32,
    _saved_asset: RwSignal<Option<SavedAssetInfo>>,
    saving: RwSignal<bool>,
    _status_msg: RwSignal<Option<String>>,
    _is_zh: bool,
) {
    saving.set(false);
}

// -----------------------------------------------------------------------------
// Terminal Exec Network Request
// -----------------------------------------------------------------------------

#[cfg(feature = "hydrate")]
fn run_command(
    project_id: String,
    cmd: String,
    screen: RwSignal<String>,
    busy: RwSignal<bool>,
) {
    wasm_bindgen_futures::spawn_local(async move {
        let body = format!("command={}", urlencode(&cmd));
        let result =
            gloo_net::http::Request::post(&format!("/projects/{}/terminal/exec", project_id))
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(body)
                .expect("valid form body")
                .send()
                .await;

        match result {
            | Ok(resp) => {
                match resp.json::<serde_json::Value>().await {
                    | Ok(data) => {
                        if let Some(out) = data.get("output").and_then(|v| v.as_str()) {
                            screen.update(|s| s.push_str(out));
                        } else if let Some(err) = data.get("error").and_then(|v| v.as_str()) {
                            screen.update(|s| {
                                s.push_str("[Error]: ");
                                s.push_str(err);
                                s.push('\n');
                            });
                        }
                    },
                    | Err(e) => screen.update(|s| s.push_str(&format!("[Response Error]: {e}\n"))),
                }
            },
            | Err(e) => screen.update(|s| s.push_str(&format!("[Network Error]: {e}\n"))),
        }
        busy.set(false);
        scroll_terminal_to_bottom();
    });
}

#[cfg(not(feature = "hydrate"))]
fn run_command(
    _project_id: String,
    _cmd: String,
    _screen: RwSignal<String>,
    busy: RwSignal<bool>,
) {
    busy.set(false);
}

#[cfg(feature = "hydrate")]
fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            | b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            },
            | b' ' => out.push('+'),
            | _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}
