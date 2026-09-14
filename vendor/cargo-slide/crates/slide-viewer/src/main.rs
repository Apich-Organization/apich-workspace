//! Universal presentation player binary (`slide-viewer`) with a sleek interactive GUI launcher.

use clap::Parser;
use minifb::Key;
use minifb::MouseButton;
use minifb::MouseMode;
use minifb::Window;
use minifb::WindowOptions;
use slide_core::compiler::SlideCompiler;
use slide_core::model::Rect as CoreRect;
use slide_core::model::SlideDeck;
use slide_core::package::read_package_metadata;
use slide_core::package::unpack_deck_and_assets;
use slide_core::package::unpack_deck_and_assets_from_bytes;
use slide_player::SlideApp;
use slide_player::hud::draw_text;
use slide_player::hud::draw_text_centered;
use slide_player::hud::draw_text_clipped;
use slide_player::window::apply_native_fullscreen;
use slide_player::window::get_screen_resolution;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;
use tiny_skia::*;

/// Embedded reference presentation package (`examples/geek-presentation`) with all assets (audio, charts, SVG)
const EMBEDDED_GEEK_DEMO: &[u8] = include_bytes!("../assets/geek_demo.slide");

mod installer;
use installer::install_viewer_to_system;
use installer::pick_presentation_file;

#[derive(Parser, Debug)]
#[command(
    name = "slide-viewer",
    bin_name = "slide-viewer",
    version,
    about = "Universal standalone presentation player and viewer for cargo-slide packages (.slide) and decks"
)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path to .slide package file, .typ source file, or deck.json (optional)
    file: Option<PathBuf>,

    /// Default transition animation (fade, zoom, slide-left, slide-right, particles, cut)
    #[arg(short, long, default_value = "fade")]
    animation: String,

    /// Start directly in fullscreen mode
    #[arg(long)]
    fullscreen: bool,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// Install slide-viewer binary to system and register .slide file associations
    Install,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if let Some(Commands::Install) = args.command {
        match install_viewer_to_system() {
            | Ok(msg) => {
                println!("✓ {msg}");
                return Ok(());
            },
            | Err(e) => {
                eprintln!("✕ Failed to install slide-viewer: {e}");
                std::process::exit(1);
            },
        }
    }

    if let Some(ref file_path) = args.file {
        if file_path.exists() {
            play_presentation_file(file_path, &args.animation, args.fullscreen)?;
            return Ok(());
        }
        eprintln!(
            "Error: Specified file does not exist: {}",
            file_path.display()
        );
    }

    // No file provided or file not found -> Launch the sleek Viewer GUI Launcher
    run_launcher_gui(&args.animation, args.fullscreen)?;

    Ok(())
}

/// Load and play any presentation format (.slide package, .typ source, or deck.json)
fn play_presentation_file(
    path: &Path,
    animation: &str,
    fullscreen: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let (deck, source_dir) = match extension.to_lowercase().as_str() {
        | "slide" => {
            let (d, cache) = unpack_deck_and_assets(path)?;
            (d, Some(cache))
        },
        | "json" => {
            let content = std::fs::read_to_string(path)?;
            let d: SlideDeck = serde_json::from_str(&content)?;
            let parent = path.parent().map(Path::to_path_buf);
            (d, parent)
        },
        | "typ" => {
            let compiler = SlideCompiler::new()?;
            let d = compiler.compile_file(path)?;
            let parent = path.parent().map(Path::to_path_buf);
            (d, parent)
        },
        | _ => {
            if let Ok((d, cache)) = unpack_deck_and_assets(path) {
                (d, Some(cache))
            } else {
                let compiler = SlideCompiler::new()?;
                let d = compiler.compile_file(path)?;
                let parent = path.parent().map(Path::to_path_buf);
                (d, parent)
            }
        },
    };

    let mut app = SlideApp::from_deck(deck)
        .default_animation(animation)
        .fullscreen(fullscreen);

    if let Some(ref dir) = source_dir {
        app = app.source_file(dir);
    }

    app.run()?;
    Ok(())
}

/// Launch the built-in reference showcase presentation (`examples/geek-presentation`)
fn launch_builtin_demo(
    animation: &str,
    fullscreen: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let (deck, cache_dir) =
        unpack_deck_and_assets_from_bytes(EMBEDDED_GEEK_DEMO, "geek_showcase_demo")?;
    SlideApp::from_deck(deck)
        .source_file(&cache_dir)
        .default_animation(animation)
        .fullscreen(fullscreen)
        .run()?;
    Ok(())
}

/// Discovered presentation entry in current directory
#[derive(Clone, Debug)]
struct SlideFileEntry {
    path: PathBuf,
    name: String,
    size_str: String,
    title: String,
    slides_count: usize,
    is_slide_pkg: bool,
}

/// Scan current directory for `.slide` and `slides.typ` presentation files
fn scan_local_presentations() -> Vec<SlideFileEntry> {
    let mut entries = Vec::new();

    if let Ok(cwd) = std::env::current_dir() {
        if let Ok(dir_entries) = std::fs::read_dir(&cwd) {
            for entry in dir_entries.flatten() {
                let p = entry.path();
                if p.extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("slide"))
                {
                    let size = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
                    let size_str = format!("{:.1} KB", size as f64 / 1024.0);
                    let (title, total_slides) = read_package_metadata(&p)
                        .map(|m| (m.title, m.total_slides))
                        .unwrap_or_else(|_| {
                            (
                                p.file_stem()
                                    .map(|s| s.to_string_lossy().to_string())
                                    .unwrap_or_default(),
                                0,
                            )
                        });

                    entries.push(SlideFileEntry {
                        name: p
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string(),
                        path: p,
                        size_str,
                        title,
                        slides_count: total_slides,
                        is_slide_pkg: true,
                    });
                }
            }
        }

        // Also check if slides.typ exists
        let typ_path = cwd.join("slides.typ");
        if typ_path.exists() {
            let size = std::fs::metadata(&typ_path).map(|m| m.len()).unwrap_or(0);
            entries.push(SlideFileEntry {
                name: "slides.typ".to_string(),
                path: typ_path,
                size_str: format!("{:.1} KB", size as f64 / 1024.0),
                title: "Local Typst Source Deck".to_string(),
                slides_count: 0,
                is_slide_pkg: false,
            });
        }
    }

    entries
}

///// Simple bounding rectangle for mouse interaction
#[derive(Clone, Copy, Debug)]
struct GuiRect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl GuiRect {
    fn new(
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    ) -> Self {
        Self { x, y, w, h }
    }

    fn contains(
        &self,
        px: f32,
        py: f32,
    ) -> bool {
        px >= self.x && px <= (self.x + self.w) && py >= self.y && py <= (self.y + self.h)
    }
}

/// Dynamically computed layout for responsive launcher window sizing
#[derive(Clone, Debug)]
struct LauncherLayout {
    width: usize,
    height: usize,
    content_top: f32,
    content_bottom: f32,
    btn_open: GuiRect,
    btn_install: GuiRect,
    btn_demo: GuiRect,
    btn_fullscreen: GuiRect,
    btn_refresh: GuiRect,
    btn_quit: GuiRect,
    left_panel: Rect,
    right_panel: Rect,
    card_bounds: Vec<GuiRect>,
    details_box: Rect,
    toast_rect: Option<GuiRect>,
    launch_btn: GuiRect,
    shortcuts_box: Rect,
}

fn compute_launcher_layout(
    width: usize,
    height: usize,
    total_items: usize,
    scroll_offset: f32,
    has_toast: bool,
) -> LauncherLayout {
    let fw = width as f32;
    let fh = height as f32;
    let margin_x = 24.0f32;
    let header_h = 56.0f32;
    let footer_h = 32.0f32;
    let gap = 16.0f32;

    // Header buttons arranged from right to left:
    let btn_y = 12.0f32;
    let btn_h = 32.0f32;
    let mut rx = fw - margin_x;

    let btn_quit = GuiRect::new(rx - 70.0, btn_y, 70.0, btn_h);
    rx -= 70.0 + 8.0;
    let btn_refresh = GuiRect::new(rx - 88.0, btn_y, 88.0, btn_h);
    rx -= 88.0 + 8.0;
    let btn_fullscreen = GuiRect::new(rx - 110.0, btn_y, 110.0, btn_h);
    rx -= 110.0 + 8.0;
    let btn_demo = GuiRect::new(rx - 84.0, btn_y, 84.0, btn_h);
    rx -= 84.0 + 8.0;
    let btn_install = GuiRect::new(rx - 102.0, btn_y, 102.0, btn_h);
    rx -= 102.0 + 8.0;
    let btn_open = GuiRect::new(rx - 116.0, btn_y, 116.0, btn_h);

    // Two column body
    let content_top = header_h + 12.0f32;
    let content_bottom = (fh - footer_h - 10.0f32).max(content_top + 150.0);
    let content_h = content_bottom - content_top;
    let avail_w = (fw - margin_x * 2.0).max(400.0);

    let left_w = (avail_w * 0.56)
        .clamp(420.0, (avail_w - 360.0).max(420.0))
        .min(avail_w - 200.0);
    let right_x = margin_x + left_w + gap;
    let right_w = (avail_w - left_w - gap).max(200.0);

    let left_panel = Rect::from_xywh(margin_x, content_top, left_w, content_h)
        .unwrap_or_else(|| Rect::from_xywh(0.0, 0.0, 10.0, 10.0).expect("Rect"));
    let right_panel = Rect::from_xywh(right_x, content_top, right_w, content_h)
        .unwrap_or_else(|| Rect::from_xywh(0.0, 0.0, 10.0, 10.0).expect("Rect"));

    // Left cards
    let card_x = margin_x + 14.0;
    let card_w = (left_w - 28.0).max(100.0);
    let card_h = 72.0f32;
    let item_h = 80.0f32;
    let start_y = content_top + 46.0 - scroll_offset;

    let mut card_bounds = Vec::with_capacity(total_items);
    for idx in 0..total_items {
        let cy = start_y + (idx as f32) * item_h;
        card_bounds.push(GuiRect::new(card_x, cy, card_w, card_h));
    }

    // Right details & actions
    let inner_x = right_x + 14.0;
    let inner_w = (right_w - 28.0).max(100.0);
    let details_y = content_top + 44.0;
    let details_h = 160.0f32;
    let details_box = Rect::from_xywh(inner_x, details_y, inner_w, details_h)
        .unwrap_or_else(|| Rect::from_xywh(0.0, 0.0, 10.0, 10.0).expect("Rect"));

    let mut current_y = details_y + details_h + 10.0;
    let toast_rect = if has_toast {
        let tr = GuiRect::new(inner_x, current_y, inner_w, 36.0);
        current_y += 44.0;
        Some(tr)
    } else {
        None
    };

    let launch_btn = GuiRect::new(inner_x, current_y, inner_w, 42.0);
    current_y += 50.0;

    let shortcuts_y = current_y;
    let shortcuts_h = (content_bottom - 12.0 - shortcuts_y).max(100.0);
    let shortcuts_box = Rect::from_xywh(inner_x, shortcuts_y, inner_w, shortcuts_h)
        .unwrap_or_else(|| Rect::from_xywh(0.0, 0.0, 10.0, 10.0).expect("Rect"));

    LauncherLayout {
        width,
        height,
        content_top,
        content_bottom,
        btn_open,
        btn_install,
        btn_demo,
        btn_fullscreen,
        btn_refresh,
        btn_quit,
        left_panel,
        right_panel,
        card_bounds,
        details_box,
        toast_rect,
        launch_btn,
        shortcuts_box,
    }
}

fn create_launcher_window(
    title: &str,
    width: usize,
    height: usize,
    fullscreen: bool,
) -> Result<Window, minifb::Error> {
    let options = if fullscreen {
        WindowOptions {
            borderless: true,
            title: false,
            resize: true,
            scale: minifb::Scale::X1,
            scale_mode: minifb::ScaleMode::AspectRatioStretch,
            topmost: false,
            ..WindowOptions::default()
        }
    } else {
        WindowOptions {
            borderless: false,
            title: true,
            resize: true,
            scale: minifb::Scale::X1,
            scale_mode: minifb::ScaleMode::AspectRatioStretch,
            topmost: false,
            ..WindowOptions::default()
        }
    };

    let mut window = Window::new(title, width, height, options)?;
    window.set_target_fps(60);
    if fullscreen {
        apply_native_fullscreen(window.get_window_handle(), true, width, height);
    }
    Ok(window)
}

/// Run the modern, interactive launcher GUI using minifb + tiny-skia with full mouse & keyboard controls
fn run_launcher_gui(
    animation: &str,
    initial_fullscreen: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut is_fullscreen = initial_fullscreen;
    let (mut width, mut height) = if is_fullscreen {
        get_screen_resolution().unwrap_or((1920, 1080))
    } else {
        (1180usize, 760usize)
    };

    let mut window = create_launcher_window(
        "Cargo Slide Viewer - Presentation Launcher",
        width,
        height,
        is_fullscreen,
    )?;

    let mut files = scan_local_presentations();
    let mut selected_index = 0usize; // 0 is Demo, 1..=N are local files
    let mut buffer: Vec<u32> = vec![0; width * height];
    let mut last_refresh = Instant::now();
    let mut prev_mouse_down = false;
    let mut last_click_time = Instant::now();
    let mut last_clicked_index: Option<usize> = None;
    let mut scroll_offset = 0.0f32;
    let mut toast_message: Option<(String, Instant)> = None;

    while window.is_open() && !window.is_key_down(Key::Escape) && !window.is_key_down(Key::Q) {
        if last_refresh.elapsed() > Duration::from_secs(3) {
            files = scan_local_presentations();
            last_refresh = Instant::now();
        }

        let total_items = files.len() + 1; // 0 = Built-in Demo, 1..=files.len() = local files
        if selected_index >= total_items {
            selected_index = total_items.saturating_sub(1);
        }

        // Window resize support
        let (new_w, new_h) = window.get_size();
        if (new_w != width || new_h != height) && new_w >= 500 && new_h >= 400 {
            width = new_w;
            height = new_h;
            buffer.resize(width * height, 0);
        }

        let has_toast = toast_message
            .as_ref()
            .is_some_and(|(_, ts)| ts.elapsed() < Duration::from_secs(5));

        let layout = compute_launcher_layout(width, height, total_items, scroll_offset, has_toast);

        // Mouse inputs
        let mouse_pos = window.get_mouse_pos(MouseMode::Pass);
        let mouse_down = window.get_mouse_down(MouseButton::Left);
        let mouse_clicked = mouse_down && !prev_mouse_down;
        prev_mouse_down = mouse_down;

        if let Some((_, scroll_y)) = window.get_scroll_wheel() {
            scroll_offset = (scroll_offset - scroll_y * 36.0).max(0.0);
        }

        let mut toggle_fullscreen = false;

        // Check clicks (header buttons and deck cards)
        if let (true, Some((mx, my))) = (mouse_clicked, mouse_pos) {
            if layout.btn_open.contains(mx, my) {
                let maybe_path = pick_presentation_file();
                if let Some(path) = maybe_path {
                    drop(window);
                    play_presentation_file(&path, animation, is_fullscreen)?;
                    return Ok(());
                }
            }
            if layout.btn_install.contains(mx, my) {
                match install_viewer_to_system() {
                    | Ok(msg) => {
                        toast_message = Some((msg, Instant::now()));
                    },
                    | Err(e) => {
                        toast_message = Some((format!("Install failed: {e}"), Instant::now()));
                    },
                }
            }
            if layout.btn_demo.contains(mx, my) {
                drop(window);
                launch_builtin_demo(animation, is_fullscreen)?;
                return Ok(());
            }
            if layout.btn_fullscreen.contains(mx, my) {
                toggle_fullscreen = true;
            }
            if layout.btn_refresh.contains(mx, my) {
                files = scan_local_presentations();
                last_refresh = Instant::now();
            }
            if layout.btn_quit.contains(mx, my) {
                break;
            }
            if layout.launch_btn.contains(mx, my) {
                if selected_index == 0 {
                    drop(window);
                    launch_builtin_demo(animation, is_fullscreen)?;
                    return Ok(());
                } else if let Some(target) = files.get(selected_index - 1) {
                    let path = target.path.clone();
                    drop(window);
                    play_presentation_file(&path, animation, is_fullscreen)?;
                    return Ok(());
                }
            }

            // Card items hit-testing
            for (idx, card_rect) in layout.card_bounds.iter().enumerate() {
                if card_rect.y + card_rect.h >= layout.content_top + 44.0
                    && card_rect.y <= layout.content_bottom - 8.0
                    && card_rect.contains(mx, my)
                {
                    let now = Instant::now();
                    let is_double_click = last_clicked_index == Some(idx)
                        && now.duration_since(last_click_time) < Duration::from_millis(400);
                    selected_index = idx;
                    last_click_time = now;
                    last_clicked_index = Some(idx);

                    if is_double_click {
                        if idx == 0 {
                            drop(window);
                            launch_builtin_demo(animation, is_fullscreen)?;
                            return Ok(());
                        } else if let Some(target) = files.get(idx - 1) {
                            let path = target.path.clone();
                            drop(window);
                            play_presentation_file(&path, animation, is_fullscreen)?;
                            return Ok(());
                        }
                    }
                }
            }
        }

        // Fullscreen toggle shortcut [F] or [F11]
        if window.is_key_pressed(Key::F, minifb::KeyRepeat::No)
            || window.is_key_pressed(Key::F11, minifb::KeyRepeat::No)
        {
            toggle_fullscreen = true;
        }

        if toggle_fullscreen {
            is_fullscreen = !is_fullscreen;
            let (target_w, target_h) = if is_fullscreen {
                get_screen_resolution().unwrap_or((1920, 1080))
            } else {
                (1180, 760)
            };
            width = target_w;
            height = target_h;
            drop(window);
            window = create_launcher_window(
                "Cargo Slide Viewer - Presentation Launcher",
                width,
                height,
                is_fullscreen,
            )?;
            buffer.resize(width * height, 0);
            toast_message = Some((
                if is_fullscreen {
                    "Entered Fullscreen (Press F / F11 to restore)".to_string()
                } else {
                    "Restored Windowed Mode".to_string()
                },
                Instant::now(),
            ));
        }

        // Keyboard navigation
        if window.is_key_pressed(Key::Down, minifb::KeyRepeat::No) {
            selected_index = (selected_index + 1) % total_items;
        }
        if window.is_key_pressed(Key::Up, minifb::KeyRepeat::No) {
            if selected_index == 0 {
                selected_index = total_items.saturating_sub(1);
            } else {
                selected_index = selected_index.saturating_sub(1);
            }
        }
        if window.is_key_pressed(Key::R, minifb::KeyRepeat::No) {
            files = scan_local_presentations();
            last_refresh = Instant::now();
        }

        // File picker shortcut [O]
        if window.is_key_pressed(Key::O, minifb::KeyRepeat::No) {
            let maybe_path = pick_presentation_file();
            if let Some(path) = maybe_path {
                drop(window);
                play_presentation_file(&path, animation, is_fullscreen)?;
                return Ok(());
            }
        }

        // Install shortcut [I]
        if window.is_key_pressed(Key::I, minifb::KeyRepeat::No) {
            match install_viewer_to_system() {
                | Ok(msg) => {
                    toast_message = Some((msg, Instant::now()));
                },
                | Err(e) => {
                    toast_message = Some((format!("Install failed: {e}"), Instant::now()));
                },
            }
        }

        // Instant demo shortcut [D]
        if window.is_key_pressed(Key::D, minifb::KeyRepeat::No) {
            drop(window);
            launch_builtin_demo(animation, is_fullscreen)?;
            return Ok(());
        }

        // Open selected [Enter] or [Space]
        if window.is_key_pressed(Key::Enter, minifb::KeyRepeat::No)
            || window.is_key_pressed(Key::Space, minifb::KeyRepeat::No)
        {
            if selected_index == 0 {
                drop(window);
                launch_builtin_demo(animation, is_fullscreen)?;
                return Ok(());
            } else if let Some(target) = files.get(selected_index - 1) {
                let path = target.path.clone();
                drop(window);
                play_presentation_file(&path, animation, is_fullscreen)?;
                return Ok(());
            }
        }

        let toast = toast_message.as_ref().and_then(|(msg, ts)| {
            if ts.elapsed() < Duration::from_secs(5) {
                Some(msg.as_str())
            } else {
                None
            }
        });

        // Render GUI frame
        render_launcher_screen(
            &mut buffer,
            &layout,
            &files,
            selected_index,
            mouse_pos,
            toast,
            is_fullscreen,
        );

        window.update_with_buffer(&buffer, width, height)?;
    }

    Ok(())
}

/// Render the clean, minimal, professional GUI launcher screen into ARGB u32 buffer
fn render_launcher_screen(
    buffer: &mut [u32],
    layout: &LauncherLayout,
    files: &[SlideFileEntry],
    selected_index: usize,
    mouse_pos: Option<(f32, f32)>,
    toast: Option<&str>,
    is_fullscreen: bool,
) {
    let width = layout.width;
    let height = layout.height;
    let total_items = files.len() + 1;
    let (mx, my) = mouse_pos.unwrap_or((-1.0, -1.0));

    {
        let mut pixmap =
            PixmapMut::from_bytes(bytemuck_cast_slice_mut(buffer), width as u32, height as u32)
                .expect("PixmapMut");

        // Clean neutral dark background #16181D
        pixmap.fill(Color::from_rgba8(22, 24, 29, 255));

        // Header bar (#1F232B)
        let mut paint = Paint::default();
        paint.set_color_rgba8(31, 35, 43, 255);
        pixmap.fill_rect(
            Rect::from_xywh(0.0, 0.0, width as f32, 56.0).unwrap(),
            &paint,
            Transform::identity(),
            None,
        );

        // Header bottom divider line (#2D333F)
        paint.set_color_rgba8(45, 51, 63, 255);
        pixmap.fill_rect(
            Rect::from_xywh(0.0, 55.0, width as f32, 1.0).unwrap(),
            &paint,
            Transform::identity(),
            None,
        );

        // Left brand badge in header (#1E293B)
        let badge_rect = Rect::from_xywh(24.0, 14.0, 108.0, 28.0).unwrap();
        paint.set_color_rgba8(30, 41, 59, 255);
        pixmap.fill_rect(badge_rect, &paint, Transform::identity(), None);
        stroke_rect(
            &mut pixmap,
            badge_rect,
            1.0,
            Color::from_rgba8(56, 189, 248, 200),
        );

        // Draw Open Button
        let open_hover = layout.btn_open.contains(mx, my);
        draw_button(
            &mut pixmap,
            layout.btn_open,
            if open_hover {
                Color::from_rgba8(30, 58, 138, 255)
            } else {
                Color::from_rgba8(30, 41, 59, 255)
            },
            if open_hover {
                Color::from_rgba8(96, 165, 250, 255)
            } else {
                Color::from_rgba8(59, 130, 246, 200)
            },
        );

        // Draw Install Button
        let install_hover = layout.btn_install.contains(mx, my);
        draw_button(
            &mut pixmap,
            layout.btn_install,
            if install_hover {
                Color::from_rgba8(6, 78, 59, 255)
            } else {
                Color::from_rgba8(20, 45, 40, 255)
            },
            if install_hover {
                Color::from_rgba8(52, 211, 153, 255)
            } else {
                Color::from_rgba8(16, 185, 129, 200)
            },
        );

        // Draw Demo Button
        let demo_hover = layout.btn_demo.contains(mx, my);
        draw_button(
            &mut pixmap,
            layout.btn_demo,
            if demo_hover {
                Color::from_rgba8(120, 53, 15, 255)
            } else {
                Color::from_rgba8(45, 30, 20, 255)
            },
            if demo_hover {
                Color::from_rgba8(251, 191, 36, 255)
            } else {
                Color::from_rgba8(245, 158, 11, 200)
            },
        );

        // Draw Fullscreen Button
        let full_hover = layout.btn_fullscreen.contains(mx, my);
        draw_button(
            &mut pixmap,
            layout.btn_fullscreen,
            if full_hover {
                Color::from_rgba8(51, 65, 85, 255)
            } else {
                Color::from_rgba8(35, 45, 60, 255)
            },
            if full_hover {
                Color::from_rgba8(148, 163, 184, 255)
            } else {
                Color::from_rgba8(100, 116, 139, 200)
            },
        );

        // Draw Refresh Button
        let refresh_hover = layout.btn_refresh.contains(mx, my);
        draw_button(
            &mut pixmap,
            layout.btn_refresh,
            if refresh_hover {
                Color::from_rgba8(44, 50, 62, 255)
            } else {
                Color::from_rgba8(35, 40, 50, 255)
            },
            if refresh_hover {
                Color::from_rgba8(75, 85, 105, 255)
            } else {
                Color::from_rgba8(52, 59, 74, 255)
            },
        );

        // Draw Quit Button
        let quit_hover = layout.btn_quit.contains(mx, my);
        draw_button(
            &mut pixmap,
            layout.btn_quit,
            if quit_hover {
                Color::from_rgba8(127, 29, 29, 200)
            } else {
                Color::from_rgba8(35, 40, 50, 255)
            },
            if quit_hover {
                Color::from_rgba8(239, 68, 68, 200)
            } else {
                Color::from_rgba8(52, 59, 74, 255)
            },
        );

        // Left Panel Container
        paint.set_color_rgba8(26, 29, 36, 255);
        pixmap.fill_rect(layout.left_panel, &paint, Transform::identity(), None);
        stroke_rect(
            &mut pixmap,
            layout.left_panel,
            1.0,
            Color::from_rgba8(43, 49, 60, 255),
        );

        // Right Panel Container
        pixmap.fill_rect(layout.right_panel, &paint, Transform::identity(), None);
        stroke_rect(
            &mut pixmap,
            layout.right_panel,
            1.0,
            Color::from_rgba8(43, 49, 60, 255),
        );

        // Render Cards in Left Panel
        for (idx, card_bound) in layout.card_bounds.iter().enumerate() {
            if card_bound.y + card_bound.h < layout.content_top + 44.0
                || card_bound.y > layout.content_bottom - 8.0
            {
                continue;
            }

            let is_selected = idx == selected_index;
            let card_rect =
                Rect::from_xywh(card_bound.x, card_bound.y, card_bound.w, card_bound.h).unwrap();
            let is_hovered = card_bound.contains(mx, my);

            // Card background
            if is_selected {
                paint.set_color_rgba8(37, 50, 77, 255);
                pixmap.fill_rect(card_rect, &paint, Transform::identity(), None);
                stroke_rect(
                    &mut pixmap,
                    card_rect,
                    1.5,
                    Color::from_rgba8(59, 130, 246, 255),
                );
            } else if is_hovered {
                paint.set_color_rgba8(35, 40, 51, 255);
                pixmap.fill_rect(card_rect, &paint, Transform::identity(), None);
                stroke_rect(
                    &mut pixmap,
                    card_rect,
                    1.0,
                    Color::from_rgba8(75, 85, 105, 255),
                );
            } else {
                paint.set_color_rgba8(30, 34, 43, 255);
                pixmap.fill_rect(card_rect, &paint, Transform::identity(), None);
                stroke_rect(
                    &mut pixmap,
                    card_rect,
                    1.0,
                    Color::from_rgba8(40, 46, 58, 255),
                );
            }

            // Left accent indicator bar
            let bar_rect = Rect::from_xywh(card_bound.x, card_bound.y, 4.0, card_bound.h).unwrap();
            if is_selected {
                paint.set_color_rgba8(59, 130, 246, 255);
            } else if idx == 0 {
                paint.set_color_rgba8(245, 158, 11, 200);
            } else {
                paint.set_color_rgba8(75, 85, 105, 150);
            }
            pixmap.fill_rect(bar_rect, &paint, Transform::identity(), None);
        }

        // If files is empty, draw a friendly hint card below Demo
        if files.is_empty() {
            let guide_y = layout.content_top + 46.0 + 84.0;
            if guide_y + 60.0 <= layout.content_bottom - 8.0 {
                let guide_rect = Rect::from_xywh(
                    layout.left_panel.x() + 14.0,
                    guide_y,
                    layout.left_panel.width() - 28.0,
                    60.0,
                )
                .unwrap();
                paint.set_color_rgba8(24, 27, 34, 200);
                pixmap.fill_rect(guide_rect, &paint, Transform::identity(), None);
                stroke_rect(
                    &mut pixmap,
                    guide_rect,
                    1.0,
                    Color::from_rgba8(45, 52, 65, 200),
                );
            }
        }

        // Right Panel: Details Box
        paint.set_color_rgba8(30, 34, 43, 255);
        pixmap.fill_rect(layout.details_box, &paint, Transform::identity(), None);
        stroke_rect(
            &mut pixmap,
            layout.details_box,
            1.0,
            Color::from_rgba8(43, 49, 60, 255),
        );

        // Toast Notification Banner (if any)
        if let Some(ref tr) = layout.toast_rect {
            draw_button(
                &mut pixmap,
                *tr,
                Color::from_rgba8(6, 78, 59, 245),
                Color::from_rgba8(16, 185, 129, 255),
            );
        }

        // Right Panel: Big Launch Button
        let launch_hover = layout.launch_btn.contains(mx, my);
        draw_button(
            &mut pixmap,
            layout.launch_btn,
            if launch_hover {
                Color::from_rgba8(29, 78, 216, 255)
            } else {
                Color::from_rgba8(37, 99, 235, 255)
            },
            Color::from_rgba8(59, 130, 246, 255),
        );

        // Right Panel: Shortcuts Card Background
        paint.set_color_rgba8(30, 34, 43, 255);
        pixmap.fill_rect(layout.shortcuts_box, &paint, Transform::identity(), None);
        stroke_rect(
            &mut pixmap,
            layout.shortcuts_box,
            1.0,
            Color::from_rgba8(43, 49, 60, 255),
        );

        // Bottom Status Bar
        paint.set_color_rgba8(22, 24, 29, 255);
        let footer_rect = Rect::from_xywh(0.0, height as f32 - 32.0, width as f32, 32.0).unwrap();
        pixmap.fill_rect(footer_rect, &paint, Transform::identity(), None);
        paint.set_color_rgba8(43, 49, 60, 255);
        pixmap.fill_rect(
            Rect::from_xywh(0.0, height as f32 - 32.0, width as f32, 1.0).unwrap(),
            &paint,
            Transform::identity(),
            None,
        );
    }

    // Render Text Elements
    draw_text_centered(
        buffer,
        width,
        height,
        CoreRect::new(24.0, 14.0, 108.0, 28.0),
        "CARGO SLIDE",
        0xFF38BDF8,
    );
    draw_text(buffer, width, height, 144, 22, "Viewer", 0xFFFFFFFF);
    draw_text(
        buffer,
        width,
        height,
        196,
        22,
        "•  Universal Presentation Player",
        0xFF9CA3AF,
    );

    // Header Button Labels
    draw_text_centered(
        buffer,
        width,
        height,
        CoreRect::new(
            layout.btn_open.x,
            layout.btn_open.y,
            layout.btn_open.w,
            layout.btn_open.h,
        ),
        "Open (O)",
        0xFF93C5FD,
    );
    draw_text_centered(
        buffer,
        width,
        height,
        CoreRect::new(
            layout.btn_install.x,
            layout.btn_install.y,
            layout.btn_install.w,
            layout.btn_install.h,
        ),
        "Install (I)",
        0xFF6EE7B7,
    );
    draw_text_centered(
        buffer,
        width,
        height,
        CoreRect::new(
            layout.btn_demo.x,
            layout.btn_demo.y,
            layout.btn_demo.w,
            layout.btn_demo.h,
        ),
        "Demo (D)",
        0xFFFDE68A,
    );
    let full_label = if is_fullscreen {
        "Windowed (F)"
    } else {
        "Fullscreen (F)"
    };
    draw_text_centered(
        buffer,
        width,
        height,
        CoreRect::new(
            layout.btn_fullscreen.x,
            layout.btn_fullscreen.y,
            layout.btn_fullscreen.w,
            layout.btn_fullscreen.h,
        ),
        full_label,
        0xFFE2E8F0,
    );
    draw_text_centered(
        buffer,
        width,
        height,
        CoreRect::new(
            layout.btn_refresh.x,
            layout.btn_refresh.y,
            layout.btn_refresh.w,
            layout.btn_refresh.h,
        ),
        "Refresh (R)",
        0xFFE5E7EB,
    );
    draw_text_centered(
        buffer,
        width,
        height,
        CoreRect::new(
            layout.btn_quit.x,
            layout.btn_quit.y,
            layout.btn_quit.w,
            layout.btn_quit.h,
        ),
        "Quit (Esc)",
        0xFFE5E7EB,
    );

    // Panel Headers
    let lp_x = layout.left_panel.x() as usize + 16;
    let lp_y = layout.left_panel.y() as usize + 16;
    let avail_title = format!("Available Presentations  ({total_items} found)");
    draw_text(buffer, width, height, lp_x, lp_y, &avail_title, 0xFFD1D5DB);

    let rp_x = layout.right_panel.x() as usize + 16;
    let rp_y = layout.right_panel.y() as usize + 16;
    draw_text(
        buffer,
        width,
        height,
        rp_x,
        rp_y,
        "Selected Presentation Details",
        0xFFD1D5DB,
    );

    // Left List Cards Text
    for (idx, card_bound) in layout.card_bounds.iter().enumerate() {
        if card_bound.y + card_bound.h < layout.content_top + 44.0
            || card_bound.y > layout.content_bottom - 8.0
        {
            continue;
        }
        let ux = card_bound.x as usize + 16;
        let uy = card_bound.y as usize;
        let max_text_w = (card_bound.w as usize).saturating_sub(90);
        let tag_x = (card_bound.x + card_bound.w - 68.0) as usize;
        let is_selected = idx == selected_index;
        let title_col = if is_selected {
            0xFFFFFFFF
        } else {
            0xFFE5E7EB
        };

        if idx == 0 {
            draw_text_clipped(
                buffer,
                width,
                height,
                ux,
                uy + 16,
                "Reference Showcase (Geek Presentation)",
                title_col,
                max_text_w,
            );
            draw_text_clipped(
                buffer,
                width,
                height,
                ux,
                uy + 40,
                "22 slides • Interactive charts, audio engine, 4-brush ink, step fragments",
                0xFF9CA3AF,
                max_text_w,
            );
            draw_text(buffer, width, height, tag_x, uy + 28, "DEMO", 0xFFF59E0B);
        } else if let Some(entry) = files.get(idx - 1) {
            let label = if entry.title.is_empty() {
                &entry.name
            } else {
                &entry.title
            };
            draw_text_clipped(
                buffer,
                width,
                height,
                ux,
                uy + 16,
                label,
                title_col,
                max_text_w,
            );
            let sub = if entry.slides_count > 0 {
                format!(
                    "{} • {} slides • {}",
                    entry.name, entry.slides_count, entry.size_str
                )
            } else {
                format!("{} • {}", entry.name, entry.size_str)
            };
            draw_text_clipped(
                buffer,
                width,
                height,
                ux,
                uy + 40,
                &sub,
                0xFF9CA3AF,
                max_text_w,
            );
            let tag = if entry.is_slide_pkg {
                "SLIDE"
            } else {
                "TYP"
            };
            draw_text(buffer, width, height, tag_x, uy + 28, tag, 0xFF38BDF8);
        }
    }

    if files.is_empty() {
        let guide_y = (layout.content_top + 46.0 + 84.0) as usize;
        if guide_y + 40 < layout.content_bottom as usize {
            let gx = layout.left_panel.x() as usize + 24;
            draw_text(
                buffer,
                width,
                height,
                gx,
                guide_y + 14,
                "No local presentation files found in current directory.",
                0xFF9CA3AF,
            );
            draw_text(
                buffer,
                width,
                height,
                gx,
                guide_y + 34,
                "Click [Open (O)] to browse files, or press [Demo (D)] to view showcase.",
                0xFF6B7280,
            );
        }
    }

    // Right Panel Text: Selected Deck Details
    let det_x = layout.details_box.x() as usize + 16;
    let det_y = layout.details_box.y() as usize + 16;
    let max_det_w = (layout.details_box.width() as usize).saturating_sub(32);

    if selected_index == 0 {
        draw_text_clipped(
            buffer,
            width,
            height,
            det_x,
            det_y,
            "Title:   Geek Presentation Showcase",
            0xFFFFFFFF,
            max_det_w,
        );
        draw_text_clipped(
            buffer,
            width,
            height,
            det_x,
            det_y + 26,
            "Source:  Built-in Reference Deck (examples/geek-presentation)",
            0xFF9CA3AF,
            max_det_w,
        );
        draw_text_clipped(
            buffer,
            width,
            height,
            det_x,
            det_y + 52,
            "Format:  Standalone Package (.slide) LZMA2 extreme",
            0xFFD1D5DB,
            max_det_w,
        );
        draw_text_clipped(
            buffer,
            width,
            height,
            det_x,
            det_y + 78,
            "Slides:  22 Slides (Charts, Media, Steps, Brushes)",
            0xFFD1D5DB,
            max_det_w,
        );
        draw_text(
            buffer,
            width,
            height,
            det_x,
            det_y + 106,
            "Status:  Ready to present",
            0xFF10B981,
        );
    } else if let Some(entry) = files.get(selected_index - 1) {
        let label = if entry.title.is_empty() {
            &entry.name
        } else {
            &entry.title
        };
        draw_text_clipped(
            buffer,
            width,
            height,
            det_x,
            det_y,
            &format!("Title:   {label}"),
            0xFFFFFFFF,
            max_det_w,
        );
        draw_text_clipped(
            buffer,
            width,
            height,
            det_x,
            det_y + 26,
            &format!("Path:    {}", entry.path.display()),
            0xFF9CA3AF,
            max_det_w,
        );
        let fmt = if entry.is_slide_pkg {
            "Standalone Package (.slide)"
        } else {
            "Typst Source (.typ)"
        };
        draw_text_clipped(
            buffer,
            width,
            height,
            det_x,
            det_y + 52,
            &format!("Format:  {fmt}"),
            0xFFD1D5DB,
            max_det_w,
        );
        let slides_info = if entry.slides_count > 0 {
            format!("{} slides", entry.slides_count)
        } else {
            "Dynamic compile".to_string()
        };
        draw_text_clipped(
            buffer,
            width,
            height,
            det_x,
            det_y + 78,
            &format!("Slides:  {slides_info} • Size: {}", entry.size_str),
            0xFFD1D5DB,
            max_det_w,
        );
        draw_text(
            buffer,
            width,
            height,
            det_x,
            det_y + 106,
            "Status:  Ready to present",
            0xFF10B981,
        );
    }

    // Toast Notification Banner Text
    if let (Some(tr), Some(msg)) = (layout.toast_rect, toast) {
        draw_text_centered(
            buffer,
            width,
            height,
            CoreRect::new(tr.x, tr.y, tr.w, tr.h),
            msg,
            0xFFECFDF5,
        );
    }

    // Launch Button Text - Centered
    draw_text_centered(
        buffer,
        width,
        height,
        CoreRect::new(
            layout.launch_btn.x,
            layout.launch_btn.y,
            layout.launch_btn.w,
            layout.launch_btn.h,
        ),
        "▶ Start Presentation (Enter / Double-Click)",
        0xFFFFFFFF,
    );

    // Shortcuts Box Text
    let sc_x = layout.shortcuts_box.x() as usize + 16;
    let sc_box_y = layout.shortcuts_box.y() as usize + 14;
    draw_text(
        buffer,
        width,
        height,
        sc_x,
        sc_box_y,
        "Controls & Keyboard Shortcuts",
        0xFFD1D5DB,
    );

    let shortcuts = [
        ("Space / Enter", "Next slide or step"),
        ("Backspace", "Previous slide or step"),
        ("F / F11", "Toggle true fullscreen mode"),
        ("O", "Browse presentation file (.slide/.typ)"),
        ("I", "Install viewer to system PATH"),
        ("D", "Launch reference demo showcase"),
        ("L", "Laser pointer with trail"),
        ("P / T", "Whiteboard pen & cycle brush mode"),
        ("1 - 0", "Palette colors & stroke width"),
        ("U / Ctrl+Z", "Undo last whiteboard stroke"),
        ("C / X", "Clear slide annotations"),
        ("Esc / Q", "Exit presentation or quit launcher"),
    ];

    let sc_desc_x = sc_x + 116;
    let mut sc_y = sc_box_y + 24;
    let max_sc_y = (layout.shortcuts_box.y() + layout.shortcuts_box.height() - 16.0) as usize;

    for (key, desc) in shortcuts {
        if sc_y + 16 > max_sc_y {
            break;
        }
        draw_text(buffer, width, height, sc_x, sc_y, key, 0xFF93C5FD);
        draw_text(buffer, width, height, sc_desc_x, sc_y, desc, 0xFFE5E7EB);
        sc_y += 19;
    }

    // Bottom Status Bar Text
    draw_text(
        buffer,
        width,
        height,
        24,
        height - 21,
        "Double-click or Enter to start • Scroll wheel to browse • F/F11 for Fullscreen",
        0xFF9CA3AF,
    );
    draw_text(
        buffer,
        width,
        height,
        width - 140,
        height - 21,
        "cargo-slide v0.1.3",
        0xFF6B7280,
    );
}

fn draw_button(
    pixmap: &mut PixmapMut,
    rect: GuiRect,
    bg: Color,
    border: Color,
) {
    let r = Rect::from_xywh(rect.x, rect.y, rect.w, rect.h).unwrap();
    let mut paint = Paint::default();
    paint.set_color(bg);
    pixmap.fill_rect(r, &paint, Transform::identity(), None);
    stroke_rect(pixmap, r, 1.5, border);
}

fn stroke_rect(
    pixmap: &mut PixmapMut,
    rect: Rect,
    stroke_width: f32,
    color: Color,
) {
    let stroke = Stroke {
        width: stroke_width,
        ..Stroke::default()
    };
    let mut paint = Paint::default();
    paint.set_color(color);
    let path = PathBuilder::from_rect(rect);
    pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
}

fn bytemuck_cast_slice_mut(slice: &mut [u32]) -> &mut [u8] {
    let len = slice.len() * 4;
    let ptr = slice.as_mut_ptr() as *mut u8;
    unsafe { std::slice::from_raw_parts_mut(ptr, len) }
}
