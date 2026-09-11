#![allow(unsafe_code)]

use crate::audio::AudioEngine;
use crate::hud::ChartInspectorState;
use crate::hud::DockAction;
use crate::hud::InkStroke;
use crate::hud::InspectorAction;
use crate::hud::PALETTE_COLORS;
use crate::hud::PresenterMode;
use crate::hud::draw_chart_inspector;
use crate::hud::draw_chart_visualizer;
use crate::hud::draw_help_overlay;
use crate::hud::draw_hotspot_highlight;
use crate::hud::draw_page_badge;
use crate::hud::draw_volume_toast;
use crate::hud::get_chart_quick_action_rects;
use crate::hud::get_dock_rects;
use crate::hud::get_inspector_layout;
use crate::hud::get_volume_slider_hover_rect;
use crate::hud::get_volume_slider_rects;
use crate::hud::hit_test_chart_inspector;
use crate::hud::hit_test_dock;
use crate::hud::hit_test_palette;
use crate::hud::hit_test_volume_slider;
use crate::hud::render_chart_hover;
use crate::hud::render_dock;
use crate::hud::render_ink_strokes;
use crate::hud::render_laser_pointer;
use crate::hud::render_laser_trail;
use crate::hud::render_palette_popup;
use crate::hud::render_volume_slider_popup;
use crate::media::MediaPlayer;
use crate::renderer::RenderMetrics;
use crate::renderer::SvgRenderer;
use crate::transition::TransitionManager;
use minifb::CursorStyle;
use minifb::Key;
use minifb::MouseButton;
use minifb::MouseMode;
use minifb::Window;
use minifb::WindowOptions;
use slide_core::animation::AnimationRegistry;
use slide_core::animation::ComponentAnimation;
use slide_core::animation::SlideAnimation;
use slide_core::animation::SlideSurface;
use slide_core::chart::ChartType;
use slide_core::compiler::SlideCompiler;
use slide_core::error::Result;
use slide_core::error::SlideError;
use slide_core::model::Hotspot;
use slide_core::model::SlideDeck;
use slide_core::watcher::SlideWatcher;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct PlayerConfig {
    pub title: String,
    pub width: usize,
    pub height: usize,
    pub fullscreen: bool,
    pub default_animation: String,
    pub watch: bool,
}

impl Default for PlayerConfig {
    fn default() -> Self {
        Self {
            title: "Cargo Slide Presentation".to_string(),
            width: 1280,
            height: 720,
            fullscreen: false,
            default_animation: "fade".to_string(),
            watch: false,
        }
    }
}

/// Slide presentation application builder and runner
pub struct SlideApp {
    source_file: Option<PathBuf>,
    deck: Option<SlideDeck>,
    config: PlayerConfig,
    registry: AnimationRegistry,
}

impl SlideApp {
    /// Create a SlideApp from a Typst file path
    pub fn new(file: impl AsRef<Path>) -> Self {
        Self {
            source_file: Some(file.as_ref().to_path_buf()),
            deck: None,
            config: PlayerConfig::default(),
            registry: AnimationRegistry::default(),
        }
    }

    /// Create a SlideApp directly from a pre-compiled or embedded `SlideDeck`
    pub fn from_deck(deck: SlideDeck) -> Self {
        let title = deck.title.clone();
        let default_animation = deck.default_animation.clone();
        Self {
            source_file: None,
            deck: Some(deck),
            config: PlayerConfig {
                title,
                default_animation,
                ..Default::default()
            },
            registry: AnimationRegistry::default(),
        }
    }

    pub fn title(
        mut self,
        title: impl Into<String>,
    ) -> Self {
        self.config.title = title.into();
        self
    }

    pub fn window_size(
        mut self,
        width: usize,
        height: usize,
    ) -> Self {
        self.config.width = width;
        self.config.height = height;
        self
    }

    pub fn fullscreen(
        mut self,
        fullscreen: bool,
    ) -> Self {
        self.config.fullscreen = fullscreen;
        self
    }

    pub fn default_animation(
        mut self,
        animation: impl Into<String>,
    ) -> Self {
        self.config.default_animation = animation.into();
        self
    }

    pub fn watch(
        mut self,
        watch: bool,
    ) -> Self {
        self.config.watch = watch;
        self
    }

    pub fn register_animation(
        mut self,
        anim: impl SlideAnimation + 'static,
    ) -> Self {
        self.registry.register(anim);
        self
    }

    pub fn register_transition(
        self,
        anim: impl SlideAnimation + 'static,
    ) -> Self {
        self.register_animation(anim)
    }

    pub fn register_component_animation(
        mut self,
        anim: impl ComponentAnimation + 'static,
    ) -> Self {
        self.registry.register_component(anim);
        self
    }

    pub fn with_registry(
        mut self,
        registry: AnimationRegistry,
    ) -> Self {
        self.registry = registry;
        self
    }

    /// Run the presentation player
    pub fn run(self) -> Result<()> {
        let deck = match self.deck {
            | Some(d) => d,
            | None => {
                let file = self.source_file.as_ref().ok_or_else(|| {
                    SlideError::Compilation("No slide file or deck provided".to_string())
                })?;
                let compiler = SlideCompiler::new()?;
                compiler.compile_file(file)?
            },
        };

        SlidePlayer::new(deck)
            .with_config(self.config)
            .with_source_file(self.source_file)
            .with_registry(self.registry)
            .run()
    }
}

pub struct SlidePlayer {
    deck: SlideDeck,
    config: PlayerConfig,
    registry: AnimationRegistry,
    source_file: Option<PathBuf>,
}

/// URL percent-decode a UTF-8 string (e.g. `%20` -> `' '`, `%2F` -> `'/'`)
#[must_use]
pub fn url_decode(s: &str) -> String {
    let parse_hex = |b: u8| {
        match b {
            | b'0'..=b'9' => Some(b.saturating_sub(b'0')),
            | b'a'..=b'f' => Some(b.saturating_sub(b'a').saturating_add(10)),
            | b'A'..=b'F' => Some(b.saturating_sub(b'A').saturating_add(10)),
            | _ => None,
        }
    };

    let bytes = s.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if let Some(&b'%') = bytes.get(i)
            && let (Some(&h1), Some(&h2)) = (
                bytes.get(i.saturating_add(1)),
                bytes.get(i.saturating_add(2)),
            )
            && let (Some(n1), Some(n2)) = (parse_hex(h1), parse_hex(h2))
        {
            let byte_val = (n1 << 4) | n2;
            decoded.push(byte_val);
            i = i.saturating_add(3);
            continue;
        }
        if let Some(&b) = bytes.get(i) {
            decoded.push(b);
        }
        i = i.saturating_add(1);
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

/// Normalize Windows verbatim extended-length path prefix (`\\?\` or `\\?\UNC\`)
/// so it can be safely passed to system shells and desktop launchers without failing.
#[must_use]
pub fn normalize_windows_path(p: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let s = p.to_string_lossy();
        if let Some(stripped) = s.strip_prefix(r"\\?\UNC\") {
            PathBuf::from(format!(r"\\{stripped}"))
        } else if let Some(stripped) = s.strip_prefix(r"\\?\") {
            PathBuf::from(stripped)
        } else {
            p
        }
    }
    #[cfg(not(windows))]
    {
        p
    }
}

/// Detached, non-blocking opener for URLs and external files across Linux, macOS, and Windows.
/// Spawns a background thread with standard streams detached so the main render loop never freezes.
pub fn open_external_target_detached(target: &str) {
    let target_str = target.to_string();
    let _ = std::thread::spawn(move || {
        #[cfg(target_os = "linux")]
        {
            let child = std::process::Command::new("xdg-open")
                .arg(&target_str)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
            if let Ok(mut c) = child {
                let _ = c.wait();
                return;
            }
            let child = std::process::Command::new("gio")
                .args(["open", &target_str])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
            if let Ok(mut c) = child {
                let _ = c.wait();
            }
        }

        #[cfg(target_os = "macos")]
        {
            let child = std::process::Command::new("open")
                .arg(&target_str)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
            if let Ok(mut c) = child {
                let _ = c.wait();
            }
        }

        #[cfg(windows)]
        {
            let child = std::process::Command::new("cmd")
                .args(["/C", "start", "", &target_str])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
            if let Ok(mut c) = child {
                let _ = c.wait();
            }
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
        {
            let _ = target_str;
        }
    });
}

/// Robust multi-tier local file resolver supporting:
/// 1. `file://` scheme stripping and RFC 8089 Windows leading slash fix (`/C:/` -> `C:/`)
/// 2. URL percent-decoding (`%20` -> `' '`)
/// 3. Direct absolute path resolution & canonicalization
/// 4. Parent directory traversal relative to Typst presentation source file (up to 6 levels)
/// 5. Current Working Directory (CWD) and ancestor traversal (up to 4 levels)
/// 6. Executable directory traversal for standalone binary mode
/// 7. Windows verbatim path prefix (`\\?\`) normalization
#[must_use]
pub fn resolve_local_file_path(
    raw_path: &str,
    source_file: Option<&Path>,
) -> Option<PathBuf> {
    // 1. Strip file:// scheme if present
    let path_no_scheme = raw_path.strip_prefix("file://").unwrap_or(raw_path);

    // 2. URL percent-decode
    let decoded = url_decode(path_no_scheme);

    // 3. Normalize RFC 8089 Windows drive paths: `/C:/foo` -> `C:/foo`
    let clean_str = if decoded.starts_with('/') && decoded.len() >= 3 {
        let bytes = decoded.as_bytes();
        if let (Some(&b1), Some(&b2)) = (bytes.get(1), bytes.get(2)) {
            if b1.is_ascii_alphabetic() && b2 == b':' {
                decoded.strip_prefix('/').unwrap_or(&decoded)
            } else {
                &decoded
            }
        } else {
            &decoded
        }
    } else {
        &decoded
    };

    let p = Path::new(clean_str);

    // 4. If absolute path and exists on disk
    if p.is_absolute() && p.exists() {
        return Some(
            p.canonicalize()
                .ok()
                .map(normalize_windows_path)
                .unwrap_or_else(|| p.to_path_buf()),
        );
    }

    // 5. Traverse ancestors relative to source_file parent
    if let Some(sf) = source_file {
        let mut cur_dir = sf.parent();
        let mut depth = 0usize;
        while let Some(dir) = cur_dir {
            if depth >= 6 {
                break;
            }
            let candidate = dir.join(p);
            if candidate.exists() {
                return Some(
                    candidate
                        .canonicalize()
                        .ok()
                        .map(normalize_windows_path)
                        .unwrap_or(candidate),
                );
            }
            cur_dir = dir.parent();
            depth = depth.saturating_add(1);
        }
    }

    // 6. Check relative to CWD and ancestors
    if let Ok(cwd) = std::env::current_dir() {
        let mut cur_dir: Option<&Path> = Some(&cwd);
        let mut depth = 0usize;
        while let Some(dir) = cur_dir {
            if depth >= 4 {
                break;
            }
            let candidate = dir.join(p);
            if candidate.exists() {
                return Some(
                    candidate
                        .canonicalize()
                        .ok()
                        .map(normalize_windows_path)
                        .unwrap_or(candidate),
                );
            }
            cur_dir = dir.parent();
            depth = depth.saturating_add(1);
        }
    }

    // 7. Check relative to current executable (crucial for standalone packaged binary mode)
    if let Ok(exe_path) = std::env::current_exe() {
        let mut cur_dir = exe_path.parent();
        let mut depth = 0usize;
        while let Some(dir) = cur_dir {
            if depth >= 4 {
                break;
            }
            let candidate = dir.join(p);
            if candidate.exists() {
                return Some(
                    candidate
                        .canonicalize()
                        .ok()
                        .map(normalize_windows_path)
                        .unwrap_or(candidate),
                );
            }
            cur_dir = dir.parent();
            depth = depth.saturating_add(1);
        }
    }

    None
}

fn resolve_media_path(
    path: &str,
    source_file: Option<&Path>,
) -> String {
    if let Some(resolved) = resolve_local_file_path(path, source_file) {
        resolved.to_string_lossy().to_string()
    } else {
        path.to_string()
    }
}

/// Query the screen resolution of the active display across platforms
pub fn get_screen_resolution() -> Option<(usize, usize)> {
    #[cfg(target_os = "linux")]
    {
        // 1. Try xrandr first to get active/primary display resolution on multi-monitor setups
        if let Ok(output) = Command::new("xrandr").output()
            && output.status.success()
            && let Ok(text) = String::from_utf8(output.stdout)
        {
            let mut primary_res = None;
            let mut first_connected = None;
            for line in text.lines() {
                if line.contains(" connected ") {
                    let is_primary = line.contains(" primary ");
                    for part in line.split_whitespace() {
                        if let Some((w_s, rest)) = part.split_once('x')
                            && let Some((h_s, _)) = rest.split_once('+')
                            && let (Ok(w), Ok(h)) = (w_s.parse::<usize>(), h_s.parse::<usize>())
                            && w > 0
                            && h > 0
                        {
                            if is_primary {
                                primary_res = Some((w, h));
                                break;
                            } else if first_connected.is_none() {
                                first_connected = Some((w, h));
                            }
                        }
                    }
                }
            }
            if let Some(res) = primary_res.or(first_connected) {
                return Some(res);
            }
        }

        // 2. Query X11 screen dimensions via Xlib
        if let Ok(xlib) = x11_dl::xlib::Xlib::open() {
            unsafe {
                let display = (xlib.XOpenDisplay)(std::ptr::null());
                if !display.is_null() {
                    let screen = (xlib.XDefaultScreen)(display);
                    let w = (xlib.XDisplayWidth)(display, screen) as usize;
                    let h = (xlib.XDisplayHeight)(display, screen) as usize;
                    (xlib.XCloseDisplay)(display);
                    if w > 0 && h > 0 {
                        return Some((w, h));
                    }
                }
            }
        }
        // 3. Fallback: /sys/class/graphics/fb0/virtual_size
        if let Ok(content) = std::fs::read_to_string("/sys/class/graphics/fb0/virtual_size")
            && let Some((w_s, h_s)) = content.trim().split_once(',')
            && let (Ok(w), Ok(h)) = (w_s.parse::<usize>(), h_s.parse::<usize>())
            && w > 0
            && h > 0
        {
            return Some((w, h));
        }
    }

    #[cfg(windows)]
    {
        unsafe extern "system" {
            fn GetSystemMetrics(nIndex: i32) -> i32;
        }
        let w = unsafe { GetSystemMetrics(0) }; // SM_CXSCREEN
        let h = unsafe { GetSystemMetrics(1) }; // SM_CYSCREEN
        if w > 0 && h > 0 {
            return Some((w as usize, h as usize));
        }
    }

    #[cfg(target_os = "macos")]
    {
        #[link(name = "CoreGraphics", kind = "framework")]
        unsafe extern "C" {
            fn CGMainDisplayID() -> u32;
            fn CGDisplayPixelsWide(display: u32) -> usize;
            fn CGDisplayPixelsHigh(display: u32) -> usize;
        }
        unsafe {
            let display = CGMainDisplayID();
            let w = CGDisplayPixelsWide(display);
            let h = CGDisplayPixelsHigh(display);
            if w > 0 && h > 0 {
                return Some((w, h));
            }
        }
    }

    None
}

#[cfg(windows)]
#[allow(clippy::single_call_fn)]
fn init_dpi_awareness() {
    unsafe extern "system" {
        fn SetProcessDpiAwarenessContext(value: isize) -> i32;
    }
    unsafe {
        let _ = SetProcessDpiAwarenessContext(-4);
    }
}

#[cfg(not(windows))]
#[allow(clippy::single_call_fn)]
const fn init_dpi_awareness() {}

#[cfg(target_os = "linux")]
#[allow(clippy::similar_names, clippy::single_call_fn, unsafe_code)]
fn set_x11_fullscreen(
    window_handle: *mut std::ffi::c_void,
    fullscreen: bool,
    width: usize,
    height: usize,
) {
    if window_handle.is_null() {
        return;
    }
    let xid = window_handle as std::os::raw::c_ulong;
    if let Ok(xlib) = x11_dl::xlib::Xlib::open() {
        unsafe {
            let display = (xlib.XOpenDisplay)(std::ptr::null());
            if display.is_null() {
                return;
            }
            let root = (xlib.XDefaultRootWindow)(display);
            let net_wm_state =
                (xlib.XInternAtom)(display, b"_NET_WM_STATE\0".as_ptr() as *const _, 0);
            let net_wm_state_fullscreen = (xlib.XInternAtom)(
                display,
                b"_NET_WM_STATE_FULLSCREEN\0".as_ptr() as *const _,
                0,
            );
            let atom_type = (xlib.XInternAtom)(display, b"ATOM\0".as_ptr() as *const _, 0);

            // 1. Direct property setting on client window (required for unmapped state per EWMH spec)
            if fullscreen {
                (xlib.XChangeProperty)(
                    display,
                    xid,
                    net_wm_state,
                    atom_type,
                    32,
                    x11_dl::xlib::PropModeReplace,
                    &net_wm_state_fullscreen as *const _ as *const _,
                    1,
                );
            } else {
                (xlib.XDeleteProperty)(display, xid, net_wm_state);
            }

            // 2. Clear Motif decorations to eliminate any title bars or window borders
            let motif_atom =
                (xlib.XInternAtom)(display, b"_MOTIF_WM_HINTS\0".as_ptr() as *const _, 0);
            if motif_atom != 0 {
                #[repr(C)]
                struct MwmHints {
                    flags: std::os::raw::c_ulong,
                    functions: std::os::raw::c_ulong,
                    decorations: std::os::raw::c_ulong,
                    input_mode: std::os::raw::c_long,
                    status: std::os::raw::c_ulong,
                }
                let hints = MwmHints {
                    flags: 2, // MWM_HINTS_DECORATIONS
                    functions: 0,
                    decorations: if fullscreen { 0 } else { 1 },
                    input_mode: 0,
                    status: 0,
                };
                (xlib.XChangeProperty)(
                    display,
                    xid,
                    motif_atom,
                    motif_atom,
                    32,
                    x11_dl::xlib::PropModeReplace,
                    &hints as *const _ as *const _,
                    5,
                );
            }

            // 3. Send ClientMessage to root window (for mapped state per EWMH spec)
            let mut data = x11_dl::xlib::ClientMessageData::new();
            data.set_long(0, if fullscreen { 1 } else { 0 }); // 1 = ADD, 0 = REMOVE
            data.set_long(1, net_wm_state_fullscreen as _);
            data.set_long(2, 0);
            data.set_long(3, 1); // 1 = normal application
            data.set_long(4, 0);

            let xev = x11_dl::xlib::XClientMessageEvent {
                type_: x11_dl::xlib::ClientMessage,
                serial: 0,
                send_event: 1, // True
                display,
                window: xid,
                message_type: net_wm_state,
                format: 32,
                data,
            };

            let mut ev: x11_dl::xlib::XEvent = std::mem::zeroed();
            ev.client_message = xev;

            let mask =
                x11_dl::xlib::SubstructureRedirectMask | x11_dl::xlib::SubstructureNotifyMask;
            (xlib.XSendEvent)(display, root, 0, mask, &mut ev);

            if fullscreen {
                (xlib.XSetWindowBorderWidth)(display, xid, 0);
                (xlib.XMoveResizeWindow)(display, xid, 0, 0, width as u32, height as u32);
                (xlib.XRaiseWindow)(display, xid);
            }

            (xlib.XFlush)(display);
            (xlib.XCloseDisplay)(display);
        }
    }
}

#[cfg(windows)]
#[allow(clippy::single_call_fn, unsafe_code)]
fn set_windows_fullscreen(
    window_handle: *mut std::ffi::c_void,
    fullscreen: bool,
    width: usize,
    height: usize,
) {
    if window_handle.is_null() {
        return;
    }
    unsafe extern "system" {
        fn SetWindowLongW(
            hWnd: *mut std::ffi::c_void,
            nIndex: i32,
            dwNewLong: i32,
        ) -> i32;
        fn SetWindowPos(
            hWnd: *mut std::ffi::c_void,
            hWndInsertAfter: *mut std::ffi::c_void,
            X: i32,
            Y: i32,
            cx: i32,
            cy: i32,
            uFlags: u32,
        ) -> i32;
    }
    const GWL_STYLE: i32 = -16;
    const WS_POPUP: i32 = 0x8000_0000u32 as i32;
    const WS_OVERLAPPEDWINDOW: i32 = 0x00CF_0000;
    const WS_VISIBLE: i32 = 0x1000_0000;
    const HWND_TOP: *mut std::ffi::c_void = 0isize as *mut std::ffi::c_void;
    const HWND_NOTOPMOST: *mut std::ffi::c_void = -2isize as *mut std::ffi::c_void;
    const SWP_FRAMECHANGED: u32 = 0x0020;
    const SWP_SHOWWINDOW: u32 = 0x0040;

    unsafe {
        if fullscreen {
            SetWindowLongW(window_handle, GWL_STYLE, WS_POPUP | WS_VISIBLE);
            SetWindowPos(
                window_handle,
                HWND_TOP,
                0,
                0,
                width as i32,
                height as i32,
                SWP_FRAMECHANGED | SWP_SHOWWINDOW,
            );
        } else {
            SetWindowLongW(window_handle, GWL_STYLE, WS_OVERLAPPEDWINDOW | WS_VISIBLE);
            SetWindowPos(
                window_handle,
                HWND_NOTOPMOST,
                100,
                100,
                width as i32,
                height as i32,
                SWP_FRAMECHANGED | SWP_SHOWWINDOW,
            );
        }
    }
}

#[cfg(target_os = "macos")]
#[allow(clippy::single_call_fn, unsafe_code)]
fn set_macos_fullscreen(
    window_handle: *mut std::ffi::c_void,
    fullscreen: bool,
) {
    if window_handle.is_null() {
        return;
    }
    unsafe {
        #[link(name = "objc", kind = "dylib")]
        extern "C" {
            fn sel_registerName(name: *const std::os::raw::c_char) -> *mut std::ffi::c_void;
            fn objc_getClass(name: *const std::os::raw::c_char) -> *mut std::ffi::c_void;
            fn objc_msgSend(
                receiver: *mut std::ffi::c_void,
                selector: *mut std::ffi::c_void,
                ...
            ) -> *mut std::ffi::c_void;
        }

        let nswindow = window_handle;
        let set_collection_behavior =
            sel_registerName(b"setCollectionBehavior:\0".as_ptr() as *const _);
        let _ = objc_msgSend(nswindow, set_collection_behavior, 128usize);

        let style_mask_sel = sel_registerName(b"styleMask\0".as_ptr() as *const _);
        let mask = objc_msgSend(nswindow, style_mask_sel) as usize;
        let is_currently_fullscreen = (mask & 16384) != 0;

        if is_currently_fullscreen != fullscreen {
            let toggle_full_screen = sel_registerName(b"toggleFullScreen:\0".as_ptr() as *const _);
            let _ = objc_msgSend(
                nswindow,
                toggle_full_screen,
                std::ptr::null_mut::<std::ffi::c_void>(),
            );
        }

        let nsapp_class = objc_getClass(b"NSApplication\0".as_ptr() as *const _);
        let shared_app_sel = sel_registerName(b"sharedApplication\0".as_ptr() as *const _);
        let app = objc_msgSend(nsapp_class, shared_app_sel);
        if !app.is_null() {
            let set_presentation_opts_sel =
                sel_registerName(b"setPresentationOptions:\0".as_ptr() as *const _);
            let options: usize = if fullscreen {
                1 | 4 | 1024
            } else {
                0
            };
            let _ = objc_msgSend(app, set_presentation_opts_sel, options);
        }
    }
}

#[cfg(target_os = "linux")]
#[allow(unsafe_code, clippy::single_call_fn)]
fn is_mouse_left_physically_down() -> bool {
    if let Ok(xlib) = x11_dl::xlib::Xlib::open() {
        unsafe {
            let display = (xlib.XOpenDisplay)(std::ptr::null());
            if !display.is_null() {
                let root = (xlib.XDefaultRootWindow)(display);
                let mut root_return = 0;
                let mut child_return = 0;
                let mut root_x = 0;
                let mut root_y = 0;
                let mut win_x = 0;
                let mut win_y = 0;
                let mut mask_return = 0;
                let res = (xlib.XQueryPointer)(
                    display,
                    root,
                    &mut root_return,
                    &mut child_return,
                    &mut root_x,
                    &mut root_y,
                    &mut win_x,
                    &mut win_y,
                    &mut mask_return,
                );
                (xlib.XCloseDisplay)(display);
                if res != 0 {
                    return (mask_return & (1 << 8)) != 0; // Button1Mask = 1 << 8
                }
            }
        }
    }
    false
}

#[cfg(windows)]
#[allow(unsafe_code, clippy::single_call_fn)]
fn is_mouse_left_physically_down() -> bool {
    unsafe extern "system" {
        fn GetAsyncKeyState(vKey: i32) -> i16;
    }
    unsafe { (GetAsyncKeyState(0x01) as u16 & 0x8000) != 0 }
}

#[cfg(target_os = "macos")]
#[allow(unsafe_code, clippy::single_call_fn)]
fn is_mouse_left_physically_down() -> bool {
    unsafe {
        #[link(name = "objc", kind = "dylib")]
        extern "C" {
            fn sel_registerName(name: *const std::os::raw::c_char) -> *mut std::ffi::c_void;
            fn objc_getClass(name: *const std::os::raw::c_char) -> *mut std::ffi::c_void;
            fn objc_msgSend(
                receiver: *mut std::ffi::c_void,
                selector: *mut std::ffi::c_void,
                ...
            ) -> *mut std::ffi::c_void;
        }
        let nsevent_class = objc_getClass(b"NSEvent\0".as_ptr() as *const _);
        if !nsevent_class.is_null() {
            let pressed_buttons_sel =
                sel_registerName(b"pressedMouseButtons\0".as_ptr() as *const _);
            let buttons = objc_msgSend(nsevent_class, pressed_buttons_sel) as usize;
            return (buttons & 1) != 0;
        }
    }
    false
}

#[cfg(not(any(target_os = "linux", windows, target_os = "macos")))]
fn is_mouse_left_physically_down() -> bool {
    false
}

fn apply_native_fullscreen(
    window_handle: *mut std::ffi::c_void,
    fullscreen: bool,
    width: usize,
    height: usize,
) {
    if window_handle.is_null() {
        return;
    }
    #[cfg(target_os = "linux")]
    set_x11_fullscreen(window_handle, fullscreen, width, height);

    #[cfg(windows)]
    set_windows_fullscreen(window_handle, fullscreen, width, height);

    #[cfg(target_os = "macos")]
    set_macos_fullscreen(window_handle, fullscreen);

    #[cfg(not(any(target_os = "linux", windows, target_os = "macos")))]
    {
        let _ = (fullscreen, width, height);
    }
}

fn create_window(
    title: &str,
    width: usize,
    height: usize,
    fullscreen: bool,
) -> Result<Window> {
    let options = if fullscreen {
        WindowOptions {
            borderless: true,
            title: false,
            resize: true,
            scale: minifb::Scale::X1,
            topmost: false,
            ..WindowOptions::default()
        }
    } else {
        WindowOptions {
            borderless: false,
            title: true,
            resize: true,
            scale: minifb::Scale::X1,
            topmost: false,
            ..WindowOptions::default()
        }
    };

    let mut window = Window::new(title, width, height, options)
        .map_err(|e| SlideError::Format(format!("Failed to create window: {}", e)))?;
    window.set_target_fps(60);

    if fullscreen {
        window.set_position(0, 0);
    }
    apply_native_fullscreen(window.get_window_handle(), fullscreen, width, height);

    Ok(window)
}

impl SlidePlayer {
    pub fn new(deck: SlideDeck) -> Self {
        let title = deck.title.clone();
        let default_anim = deck.default_animation.clone();
        Self {
            deck,
            config: PlayerConfig {
                title,
                default_animation: default_anim,
                ..Default::default()
            },
            registry: AnimationRegistry::default(),
            source_file: None,
        }
    }

    pub fn with_config(
        mut self,
        config: PlayerConfig,
    ) -> Self {
        self.config = config;
        self
    }

    pub fn with_registry(
        mut self,
        registry: AnimationRegistry,
    ) -> Self {
        self.registry = registry;
        self
    }

    pub fn with_source_file(
        mut self,
        file: Option<PathBuf>,
    ) -> Self {
        self.source_file = file;
        self
    }

    pub fn with_watch(
        mut self,
        watch: bool,
    ) -> Self {
        self.config.watch = watch;
        self
    }

    pub fn register_animation(
        mut self,
        anim: impl SlideAnimation + 'static,
    ) -> Self {
        self.registry.register(anim);
        self
    }

    pub fn register_transition(
        self,
        anim: impl SlideAnimation + 'static,
    ) -> Self {
        self.register_animation(anim)
    }

    pub fn register_component_animation(
        mut self,
        anim: impl ComponentAnimation + 'static,
    ) -> Self {
        self.registry.register_component(anim);
        self
    }

    /// Run the presentation player event loop
    pub fn run(mut self) -> Result<()> {
        init_dpi_awareness();
        if self.deck.slides.is_empty() {
            return Err(SlideError::Format(
                "Slide deck has no slides to display".to_string(),
            ));
        }

        let mut is_fullscreen = self.config.fullscreen;
        let (screen_w, screen_h) = get_screen_resolution().unwrap_or((1920, 1080));
        let (mut width, mut height) = if is_fullscreen {
            (screen_w, screen_h)
        } else {
            (self.config.width, self.config.height)
        };

        let mut window = create_window(&self.config.title, width, height, is_fullscreen)?;

        let renderer = SvgRenderer::new();
        let mut transition_mgr = TransitionManager::new(self.registry);

        let mut current_idx = 0;
        let mut total_slides = self.deck.total_slides();

        // File watcher for hot reloading
        let mut watcher = if self.config.watch {
            if let Some(ref file) = self.source_file {
                match SlideWatcher::new(file) {
                    | Ok(w) => {
                        let watch_dir = w.watch_dir();
                        println!(
                            "👀 Live hot reload active. Watching {} for changes...",
                            watch_dir.display()
                        );
                        slide_core::logger::log_event(
                            "info",
                            &format!(
                                "👀 Live hot reload active. Watching {} for changes...",
                                watch_dir.display()
                            ),
                            Some(serde_json::json!({
                                "stage": "hot_reload_active",
                                "watch_dir": watch_dir.display().to_string(),
                                "source_file": file.display().to_string(),
                            })),
                        );
                        Some(w)
                    },
                    | Err(e) => {
                        eprintln!("⚠️ Failed to initialize slide watcher: {}", e);
                        None
                    },
                }
            } else {
                None
            }
        } else {
            None
        };

        // Audio engine
        let mut audio_engine = AudioEngine::new();
        let mut last_volume_change: Option<Instant> = None;

        // Presenter interactive tools state
        let mut presenter_mode = PresenterMode::Normal;
        let mut active_color_idx = 0usize; // Cyan by default (index 0)
        let mut palette_open = false;
        let mut volume_slider_open = false;
        let mut volume_dragging = false;
        let mut slide_ink: HashMap<usize, Vec<InkStroke>> = HashMap::new();
        let mut active_pen_stroke: Option<InkStroke> = None;
        let mut laser_trail: VecDeque<(usize, usize, Instant)> = VecDeque::new();
        let mut laser_trail_scratch: Vec<(usize, usize, f32)> = Vec::with_capacity(64);

        // In-slide component step state
        let get_max_step = |deck: &SlideDeck, idx: usize| -> usize {
            deck.get_slide(idx).map(|s| s.max_step()).unwrap_or(0)
        };
        let mut current_step: usize = if get_max_step(&self.deck, 0) > 0 {
            1
        } else {
            0
        };

        // Chart interactivity and inspector state
        let mut chart_type_overrides: HashMap<(usize, usize), ChartType> = HashMap::new();
        let mut active_chart_inspector: Option<ChartInspectorState> = None;

        // Surface cache mapped by slide_index
        let mut cache: HashMap<usize, (Arc<SlideSurface>, RenderMetrics)> = HashMap::new();

        let mut buffer = vec![0u32; width * height];
        let mut prev_pressed_keys = Vec::new();
        let mut prev_mouse_down = false;
        let mut prev_mouse_right_down = false;
        let mut was_window_active = true;
        let mut mouse_stuck_suppressed = false;
        let mut prev_mouse_pos: Option<(f32, f32)> = None;
        let mut last_mouse_activity = Instant::now();
        let mut hovered_hotspot_key: Option<(usize, usize)> = None;
        let mut hotspot_hover_alpha: f32 = 0.0;
        let mut last_frame_time = Instant::now();
        let mut cursor_hidden = false;
        let mut show_help = false;
        let mut exit_requested = false;

        let trigger_slide_audio =
            |deck: &SlideDeck, idx: usize, audio: &mut AudioEngine, sf: Option<&Path>| {
                if let Some(slide) = deck.get_slide(idx) {
                    for hs in &slide.hotspots {
                        if let Hotspot::Audio {
                            source,
                            autoplay,
                            loop_audio,
                            volume,
                            ..
                        } = hs
                            && *autoplay
                        {
                            let resolved = resolve_media_path(source, sf);
                            let _ = audio.play_track(
                                &resolved,
                                *loop_audio,
                                *volume,
                                Duration::from_millis(600),
                            );
                            return;
                        }
                    }
                }
                audio.stop_with_fade(Duration::from_millis(600));
            };

        trigger_slide_audio(
            &self.deck,
            current_idx,
            &mut audio_engine,
            self.source_file.as_deref(),
        );

        let mut first_frame = true;

        while window.is_open() && !exit_requested && !window.is_key_down(Key::Q) {
            let is_window_active = window.is_active();
            if !is_window_active {
                was_window_active = false;
                volume_dragging = false;
                active_pen_stroke = None;
                prev_pressed_keys.clear();
            } else if !was_window_active {
                was_window_active = true;
                if window.get_mouse_down(MouseButton::Left) && !is_mouse_left_physically_down() {
                    mouse_stuck_suppressed = true;
                }
            }

            let (new_w, new_h) = window.get_size();
            if (new_w != width || new_h != height) && new_w >= 100 && new_h >= 100 {
                width = new_w;
                height = new_h;
                buffer.resize(width * height, 0);
                cache.clear(); // Invalidate cache on resize
            }

            let dt = last_frame_time.elapsed().as_secs_f32().clamp(0.001, 0.1);
            last_frame_time = Instant::now();

            // Update smooth volume curves in audio engine
            audio_engine.update();

            // Handle mouse scroll wheel for volume adjustment or Data Inspector table scrolling
            if let Some((_scroll_x, scroll_y)) = window.get_scroll_wheel()
                && scroll_y != 0.0
            {
                if let Some(ref mut inspector) = active_chart_inspector {
                    let (_, _, _, _, _, _, _, right_pane) = get_inspector_layout(width, height);
                    let header_h = 26.0;
                    let row_h = 24.0;
                    let visible_rows =
                        ((right_pane.height - header_h - 2.0).max(0.0) / row_h) as usize;
                    let max_scroll = inspector
                        .chart_data
                        .categories
                        .len()
                        .saturating_sub(visible_rows);
                    if scroll_y < 0.0 {
                        inspector.table_scroll = (inspector.table_scroll + 1).min(max_scroll);
                    } else {
                        inspector.table_scroll = inspector.table_scroll.saturating_sub(1);
                    }
                } else {
                    let delta = if scroll_y > 0.0 {
                        0.05
                    } else {
                        -0.05
                    };
                    audio_engine.adjust_volume(delta);
                    last_volume_change = Some(Instant::now());
                }
            }

            // Ensure current slide surface is rendered
            let (current_surface, current_metrics) = match cache.get(&current_idx) {
                | Some(cached) => (cached.0.clone(), cached.1),
                | None => {
                    let slide = self.deck.get_slide(current_idx).unwrap();
                    let (surf, metrics) = renderer.render_svg(&slide.svg_data, width, height)?;
                    let surf_arc = Arc::new(surf);
                    cache.insert(current_idx, (surf_arc.clone(), metrics));
                    (surf_arc, metrics)
                },
            };

            // Handle transition animation or render static slide
            let is_animating = transition_mgr.render_frame(width, height, &mut buffer);
            if !is_animating {
                let copy_len = buffer.len().min(current_surface.pixels.len());
                buffer[..copy_len].copy_from_slice(&current_surface.pixels[..copy_len]);

                // Blank / mask component steps that have not yet been revealed
                if let Some(slide) = self.deck.get_slide(current_idx) {
                    let bg_color = current_surface.bg_color;
                    for step in &slide.steps {
                        if step.order > current_step {
                            let screen_rect = current_metrics.svg_to_screen_rect(&step.rect);
                            let sx1 = (screen_rect.x.max(0.0) as usize).min(width);
                            let sy1 = (screen_rect.y.max(0.0) as usize).min(height);
                            let sx2 =
                                ((screen_rect.x + screen_rect.width).max(0.0) as usize).min(width);
                            let sy2 = ((screen_rect.y + screen_rect.height).max(0.0) as usize)
                                .min(height);

                            for y in sy1..sy2 {
                                let row = y * width;
                                for x in sx1..sx2 {
                                    buffer[row + x] = bg_color;
                                }
                            }
                        }
                    }

                    // Render active in-slide component reveal animation if running
                    transition_mgr.render_component_animation(
                        width,
                        height,
                        &current_surface,
                        &mut buffer,
                        bg_color,
                    );
                }

                // Render in-slide chart type overrides if any
                if let Some(slide) = self.deck.get_slide(current_idx) {
                    for (hs_idx, hs) in slide.hotspots.iter().enumerate() {
                        if let Hotspot::Chart { rect, data } = hs
                            && let Some(&override_type) =
                                chart_type_overrides.get(&(current_idx, hs_idx))
                            && override_type != data.chart_type
                        {
                            let screen_rect = current_metrics.svg_to_screen_rect(rect);
                            let empty_set = std::collections::HashSet::new();
                            draw_chart_visualizer(
                                &mut buffer,
                                width,
                                height,
                                screen_rect,
                                data,
                                override_type,
                                &empty_set,
                                None,
                                None,
                                None,
                            );
                        }
                    }
                }
            }

            // Mouse handling and cursor style
            let raw_mouse_pos = window.get_mouse_pos(MouseMode::Pass);
            let mouse_pos = if is_window_active {
                raw_mouse_pos
            } else {
                None
            };
            if mouse_pos != prev_mouse_pos {
                last_mouse_activity = Instant::now();
                if cursor_hidden && presenter_mode != PresenterMode::Laser {
                    window.set_cursor_visibility(true);
                    cursor_hidden = false;
                }
                prev_mouse_pos = mouse_pos;
            } else if !cursor_hidden && last_mouse_activity.elapsed() > Duration::from_secs(3) {
                // Auto-hide cursor during presentation if idle for 3 seconds
                window.set_cursor_visibility(false);
                cursor_hidden = true;
            }

            // Synchronize mouse hovering within Data Inspector panes if open
            if let Some(ref mut inspector) = active_chart_inspector
                && let Some((mx, my)) = mouse_pos
            {
                let (_, _, _, _, _, _, left_pane, right_pane) = get_inspector_layout(width, height);
                if left_pane.contains(mx, my) {
                    let px = left_pane.x + 44.0;
                    let pw = left_pane.width - 60.0;
                    let cat_count = inspector.chart_data.categories.len().max(1);
                    let col_w = pw / cat_count as f32;
                    if mx >= px && mx <= (px + pw) {
                        let c_idx = (((mx - px) / col_w) as usize).min(cat_count.saturating_sub(1));
                        inspector.hovered_category = Some(c_idx);
                        if window.get_mouse_down(MouseButton::Left) {
                            let old_range = inspector.marquee_range;
                            if let Some(start) = inspector.marquee_drag_start {
                                inspector.marquee_range = Some((start, c_idx));
                            } else {
                                inspector.marquee_drag_start = Some(c_idx);
                                inspector.marquee_range = Some((c_idx, c_idx));
                            }
                            if inspector.marquee_range != old_range {
                                inspector.recompute_cache();
                            }
                        }
                    } else {
                        inspector.hovered_category = None;
                    }
                } else if right_pane.contains(mx, my) {
                    let header_h = 56.0;
                    let row_h = 24.0;
                    let table_y = right_pane.y + header_h;
                    let visible_rows =
                        ((right_pane.height - header_h - 2.0).max(0.0) / row_h) as usize;
                    if my >= table_y && my < (right_pane.y + right_pane.height) {
                        let row_slot = ((my - table_y) / row_h) as usize;
                        if row_slot < visible_rows {
                            let filtered_indices = inspector.get_filtered_category_indices();
                            let list_idx = row_slot.saturating_add(inspector.table_scroll);
                            if let Some(&cat_idx) = filtered_indices.get(list_idx) {
                                inspector.hovered_category = Some(cat_idx);
                            } else {
                                inspector.hovered_category = None;
                            }
                        } else {
                            inspector.hovered_category = None;
                        }
                    } else {
                        inspector.hovered_category = None;
                    }
                } else {
                    inspector.hovered_category = None;
                }
                if !window.get_mouse_down(MouseButton::Left) {
                    inspector.marquee_drag_start = None;
                }
            }

            let (dock_rect, _) = get_dock_rects(width, height, is_fullscreen);
            let mouse_in_dock = mouse_pos
                .map(|(mx, my)| dock_rect.contains(mx, my))
                .unwrap_or(false);
            let mouse_near_dock = mouse_pos
                .map(|(_, my)| my >= (height as f32 - 70.0))
                .unwrap_or(false);
            let hovered_dock_action =
                mouse_pos.and_then(|(mx, my)| hit_test_dock(width, height, is_fullscreen, mx, my));

            let (slider_popup_rect, track_rect) =
                get_volume_slider_rects(width, height, is_fullscreen);
            let vol_hover_rect = get_volume_slider_hover_rect(width, height, is_fullscreen);
            let is_vol_btn_hovered = hovered_dock_action == Some(DockAction::ToggleMute);
            let is_vol_zone_hovered = mouse_pos
                .map(|(mx, my)| vol_hover_rect.contains(mx, my))
                .unwrap_or(false);

            if palette_open || active_chart_inspector.is_some() {
                volume_slider_open = false;
                volume_dragging = false;
            } else if volume_dragging || is_vol_btn_hovered {
                volume_slider_open = true;
            } else if volume_slider_open && !is_vol_zone_hovered {
                volume_slider_open = false;
            }
            let show_volume_slider = volume_slider_open;
            let mouse_in_slider = show_volume_slider
                && mouse_pos
                    .map(|(mx, my)| slider_popup_rect.contains(mx, my))
                    .unwrap_or(false);

            let hovered_palette_idx = mouse_pos
                .and_then(|(mx, my)| hit_test_palette(width, height, is_fullscreen, mx, my));

            let mut hovered_hotspot_idx: Option<usize> = None;
            if active_chart_inspector.is_none()
                && !is_animating
                && let Some((mx, my)) = mouse_pos
                && !mouse_in_dock
                && !mouse_in_slider
                && (!palette_open || hovered_palette_idx.is_none())
                && let Some((svg_x, svg_y)) = current_metrics.screen_to_svg(mx, my)
                && let Some(slide) = self.deck.get_slide(current_idx)
            {
                for (idx, hotspot) in slide.hotspots.iter().enumerate() {
                    if hotspot.contains(svg_x, svg_y) {
                        hovered_hotspot_idx = Some(idx);
                        break;
                    }
                }
            }

            // Smooth animation: lerp hotspot hover alpha (fades in ~80ms, fades out ~100ms)
            let current_target_key = if !is_animating {
                hovered_hotspot_idx.map(|hs_i| (current_idx, hs_i))
            } else {
                None
            };

            if current_target_key != hovered_hotspot_key {
                if current_target_key.is_none() {
                    hotspot_hover_alpha = (hotspot_hover_alpha - dt * 10.0).max(0.0);
                    if hotspot_hover_alpha <= 0.01 {
                        hovered_hotspot_key = None;
                    }
                } else {
                    if hovered_hotspot_key.is_none()
                        || hovered_hotspot_key.unwrap().0 != current_idx
                    {
                        hotspot_hover_alpha = 0.0;
                    }
                    hovered_hotspot_key = current_target_key;
                    hotspot_hover_alpha = (hotspot_hover_alpha + dt * 12.0).min(1.0);
                }
            } else if hovered_hotspot_key.is_some() {
                hotspot_hover_alpha = (hotspot_hover_alpha + dt * 12.0).min(1.0);
            } else {
                hotspot_hover_alpha = (hotspot_hover_alpha - dt * 10.0).max(0.0);
            }

            // Set cursor style based on mode and hover
            if presenter_mode == PresenterMode::Laser {
                if !cursor_hidden {
                    window.set_cursor_visibility(false);
                    cursor_hidden = true;
                }
            } else if active_chart_inspector.is_some() {
                window.set_cursor_style(CursorStyle::Arrow);
            } else {
                if hovered_dock_action.is_some()
                    || hovered_hotspot_idx.is_some()
                    || hovered_palette_idx.is_some()
                    || mouse_in_slider
                {
                    window.set_cursor_style(CursorStyle::OpenHand);
                } else if presenter_mode == PresenterMode::Pen {
                    window.set_cursor_style(CursorStyle::Crosshair);
                } else {
                    window.set_cursor_style(CursorStyle::Arrow);
                }
            }

            // Highlight hovered hotspot (in Normal mode when inspector is closed, not animating transition)
            if presenter_mode == PresenterMode::Normal
                && active_chart_inspector.is_none()
                && !is_animating
                && hotspot_hover_alpha > 0.01
                && let Some((slide_i, hs_idx)) = hovered_hotspot_key
                && slide_i == current_idx
                && let Some(slide) = self.deck.get_slide(current_idx)
                && let Some(hotspot) = slide.hotspots.get(hs_idx)
            {
                let rect = hotspot.rect();
                let screen_rect = current_metrics.svg_to_screen_rect(&rect);

                let color = match hotspot {
                    | Hotspot::Link { .. } => 0xFF58a6ff,  // Blue outline
                    | Hotspot::Video { .. } => 0xFFf0883e, // Orange outline
                    | Hotspot::Audio { .. } => 0xFF3fb950, // Green outline
                    | Hotspot::Chart { .. } => 0xFF38bdf8, // Cyan outline
                };
                draw_hotspot_highlight(
                    &mut buffer,
                    width,
                    height,
                    screen_rect,
                    color,
                    hotspot_hover_alpha,
                );

                if let Hotspot::Chart { rect, data } = hotspot {
                    let active_t = chart_type_overrides
                        .get(&(current_idx, hs_idx))
                        .copied()
                        .unwrap_or(data.chart_type);
                    if let Some((mx, my)) = mouse_pos {
                        render_chart_hover(
                            &mut buffer,
                            width,
                            height,
                            &current_metrics,
                            *rect,
                            data,
                            active_t,
                            mx,
                            my,
                        );
                    }
                }
            }

            // Mouse click handling
            let raw_mouse_down = window.get_mouse_down(MouseButton::Left);
            if !raw_mouse_down {
                mouse_stuck_suppressed = false;
            }
            let mouse_down = is_window_active && raw_mouse_down && !mouse_stuck_suppressed;
            let mouse_right = is_window_active && window.get_mouse_down(MouseButton::Right);
            let left_clicked = mouse_down && !prev_mouse_down;
            let right_clicked = mouse_right && !prev_mouse_right_down;

            let mut trigger_forward = false;
            let mut trigger_backward = false;
            let mut jump_target = None;
            let mut toggle_fullscreen_requested = false;

            if right_clicked {
                if active_chart_inspector.is_some() {
                    active_chart_inspector = None;
                } else {
                    // Right click steps backward
                    trigger_backward = true;
                }
            }

            // Interactive dragging on volume slider track
            if mouse_down && active_chart_inspector.is_none() {
                if volume_dragging {
                    if let Some((_, my)) = mouse_pos {
                        let norm = 1.0 - ((my - track_rect.y) / track_rect.height).clamp(0.0, 1.0);
                        audio_engine.set_volume(norm);
                        last_volume_change = Some(Instant::now());
                    }
                } else if show_volume_slider
                    && let Some((mx, my)) = mouse_pos
                    && let Some(vol) = hit_test_volume_slider(width, height, is_fullscreen, mx, my)
                {
                    volume_dragging = true;
                    audio_engine.set_volume(vol);
                    last_volume_change = Some(Instant::now());
                }
            } else {
                volume_dragging = false;
            }

            if left_clicked {
                if let Some(ref mut inspector) = active_chart_inspector {
                    if let Some((mx, my)) = mouse_pos
                        && let Some(action) =
                            hit_test_chart_inspector(inspector, width, height, mx, my)
                    {
                        match action {
                            | InspectorAction::Close => {
                                if let Some(hs_idx) = hovered_hotspot_idx {
                                    chart_type_overrides
                                        .insert((current_idx, hs_idx), inspector.active_type);
                                }
                                active_chart_inspector = None;
                            },
                            | InspectorAction::SetType(t) => {
                                inspector.active_type = t;
                                inspector.search_active = false;
                            },
                            | InspectorAction::SetTransform(t) => {
                                inspector.set_transform(t);
                                inspector.search_active = false;
                            },
                            | InspectorAction::ToggleSeries(s_idx) => {
                                inspector.toggle_series(s_idx);
                                inspector.search_active = false;
                            },
                            | InspectorAction::ExportCsv => {
                                inspector.search_active = false;
                                let csv = inspector.chart_data.export_csv(&inspector.hidden_series);
                                if let Err(e) = std::fs::write("chart_export.csv", csv) {
                                    eprintln!("Failed to export CSV: {}", e);
                                    inspector.show_toast("Export failed!");
                                } else {
                                    inspector.show_toast("Exported to chart_export.csv");
                                }
                            },
                            | InspectorAction::SelectCategory(cat_idx) => {
                                inspector.hovered_category = Some(cat_idx);
                                inspector.search_active = false;
                            },
                            | InspectorAction::ScrollTable(delta) => {
                                inspector.search_active = false;
                                let (_, _, _, _, _, _, _, right_pane) =
                                    get_inspector_layout(width, height);
                                let header_h = 56.0;
                                let row_h = 24.0;
                                let visible_rows = ((right_pane.height - header_h - 2.0).max(0.0)
                                    / row_h)
                                    as usize;
                                let filtered_len = inspector.get_filtered_category_indices().len();
                                let max_scroll = filtered_len.saturating_sub(visible_rows);
                                if delta > 0 {
                                    inspector.table_scroll =
                                        (inspector.table_scroll + delta as usize).min(max_scroll);
                                } else {
                                    inspector.table_scroll =
                                        inspector.table_scroll.saturating_sub((-delta) as usize);
                                }
                            },
                            | InspectorAction::FocusSearch => {
                                inspector.search_active = true;
                            },
                            | InspectorAction::ClearFilter => {
                                inspector.clear_filter();
                            },
                            | InspectorAction::CycleFormat => {
                                inspector.cycle_format();
                                inspector.search_active = false;
                            },
                            | InspectorAction::SortColumn(col) => {
                                inspector.search_active = false;
                                if inspector.sort_column == Some(col) {
                                    inspector.sort_ascending = !inspector.sort_ascending;
                                } else {
                                    inspector.sort_column = Some(col);
                                    inspector.sort_ascending = true;
                                }
                                inspector.recompute_cache();
                                let label = if col == 0 {
                                    format!(
                                        "Sorted by Category {}",
                                        if inspector.sort_ascending {
                                            "Ascending"
                                        } else {
                                            "Descending"
                                        }
                                    )
                                } else {
                                    let s_name = inspector
                                        .chart_data
                                        .series
                                        .get(col.saturating_sub(1))
                                        .map(|s| s.name.as_str())
                                        .unwrap_or("Series");
                                    format!(
                                        "Sorted by {} {}",
                                        s_name,
                                        if inspector.sort_ascending {
                                            "Ascending"
                                        } else {
                                            "Descending"
                                        }
                                    )
                                };
                                inspector.show_toast(&label);
                            },
                        }
                    }
                } else if let Some(pal_idx) = hovered_palette_idx {
                    active_color_idx = pal_idx;
                } else if let Some(dock_action) = hovered_dock_action {
                    match dock_action {
                        | DockAction::Prev => trigger_backward = true,
                        | DockAction::Next => trigger_forward = true,
                        | DockAction::ModeNormal => presenter_mode = PresenterMode::Normal,
                        | DockAction::ModeLaser => presenter_mode = PresenterMode::Laser,
                        | DockAction::ModePen => presenter_mode = PresenterMode::Pen,
                        | DockAction::TogglePalette => palette_open = !palette_open,
                        | DockAction::ClearInk => {
                            slide_ink.remove(&current_idx);
                            active_pen_stroke = None;
                        },
                        | DockAction::ToggleMute => {
                            if !volume_slider_open {
                                volume_slider_open = true;
                            } else {
                                audio_engine.toggle_mute();
                                last_volume_change = Some(Instant::now());
                            }
                        },
                        | DockAction::ToggleFullscreen => toggle_fullscreen_requested = true,
                        | DockAction::ToggleHelp => show_help = !show_help,
                    }
                } else if mouse_in_slider {
                    if let Some((mx, my)) = mouse_pos
                        && let Some(vol) =
                            hit_test_volume_slider(width, height, is_fullscreen, mx, my)
                    {
                        volume_dragging = true;
                        audio_engine.set_volume(vol);
                        last_volume_change = Some(Instant::now());
                    }
                } else if presenter_mode != PresenterMode::Pen {
                    if let Some(hs_idx) = hovered_hotspot_idx {
                        if let Some(slide) = self.deck.get_slide(current_idx)
                            && let Some(hotspot) = slide.hotspots.get(hs_idx)
                        {
                            match hotspot {
                                | Hotspot::Link { target, rect } => {
                                    let clean_target = target.strip_prefix('#').unwrap_or(target);
                                    if clean_target.starts_with("chart:") {
                                        // Fallback guard: if a chart hotspot was parsed as Link, open Inspector directly
                                        if let Some(payload) = clean_target.strip_prefix("chart:")
                                            && let Some(Hotspot::Chart { data, .. }) =
                                                slide_core::svg::parse_chart_href(payload, *rect)
                                        {
                                            let curr_t = chart_type_overrides
                                                .get(&(current_idx, hs_idx))
                                                .copied()
                                                .unwrap_or(data.chart_type);
                                            active_chart_inspector =
                                                Some(ChartInspectorState::new(data, curr_t));
                                        }
                                    } else if clean_target.starts_with("video:")
                                        || clean_target.starts_with("audio:")
                                        || clean_target.starts_with("step:")
                                        || clean_target.starts_with("transition:")
                                    {
                                        // Internal metadata markers; never invoke OS open
                                    } else if target.starts_with("mailto:")
                                        || (target.contains("://")
                                            && !target.starts_with("file://"))
                                    {
                                        open_external_target_detached(target);
                                        volume_dragging = false;
                                        active_pen_stroke = None;
                                        mouse_stuck_suppressed = true;
                                    } else if let Some(page_str) = target
                                        .strip_prefix("#page=")
                                        .or_else(|| target.strip_prefix("#slide="))
                                        .or_else(|| target.strip_prefix("#"))
                                    {
                                        if let Ok(page) = page_str.parse::<usize>()
                                            && page >= 1
                                            && page <= total_slides
                                            && page.saturating_sub(1) != current_idx
                                        {
                                            jump_target = Some(page.saturating_sub(1));
                                        }
                                    } else {
                                        // Local document or file path relative to presentation
                                        if let Some(resolved_path) = resolve_local_file_path(
                                            target,
                                            self.source_file.as_deref(),
                                        ) {
                                            open_external_target_detached(
                                                &resolved_path.to_string_lossy(),
                                            );
                                        } else {
                                            let clean =
                                                target.strip_prefix("file://").unwrap_or(target);
                                            let decoded = url_decode(clean);
                                            open_external_target_detached(&decoded);
                                        }
                                        volume_dragging = false;
                                        active_pen_stroke = None;
                                        mouse_stuck_suppressed = true;
                                    }
                                },
                                | Hotspot::Video { source, .. } => {
                                    let resolved =
                                        resolve_media_path(source, self.source_file.as_deref());
                                    let _ = MediaPlayer::play_video(&resolved);
                                    volume_dragging = false;
                                    active_pen_stroke = None;
                                    mouse_stuck_suppressed = true;
                                },
                                | Hotspot::Audio {
                                    source,
                                    loop_audio,
                                    volume,
                                    ..
                                } => {
                                    let resolved =
                                        resolve_media_path(source, self.source_file.as_deref());
                                    if audio_engine.current_source() == Some(&resolved) {
                                        audio_engine.toggle_pause();
                                    } else {
                                        let _ = audio_engine.play_track(
                                            &resolved,
                                            *loop_audio,
                                            *volume,
                                            Duration::from_millis(500),
                                        );
                                    }
                                },
                                | Hotspot::Chart { rect, data } => {
                                    let (type_btn, _) =
                                        get_chart_quick_action_rects(&current_metrics, *rect);
                                    let clicked_type_btn = mouse_pos
                                        .map(|(mx, my)| type_btn.contains(mx, my))
                                        .unwrap_or(false);

                                    let curr_t = chart_type_overrides
                                        .get(&(current_idx, hs_idx))
                                        .copied()
                                        .unwrap_or(data.chart_type);

                                    if clicked_type_btn {
                                        let next_t = match curr_t {
                                            | ChartType::Bar => ChartType::Line,
                                            | ChartType::Line => ChartType::Area,
                                            | ChartType::Area => ChartType::Pie,
                                            | ChartType::Pie => ChartType::Donut,
                                            | ChartType::Donut => ChartType::Bar,
                                            | ChartType::Scatter => ChartType::Bar,
                                        };
                                        chart_type_overrides.insert((current_idx, hs_idx), next_t);
                                    } else {
                                        // Open Data Inspector Modal!
                                        active_chart_inspector =
                                            Some(ChartInspectorState::new(data.clone(), curr_t));
                                    }
                                },
                            }
                        }
                    } else {
                        // Canvas click steps forward
                        trigger_forward = true;
                    }
                }
            }

            let active_color = PALETTE_COLORS[active_color_idx].1;

            // Whiteboard pen stroke tracking
            if presenter_mode == PresenterMode::Pen {
                if mouse_down
                    && !mouse_in_dock
                    && !mouse_in_slider
                    && (!palette_open || hovered_palette_idx.is_none())
                {
                    if let Some((mx, my)) = mouse_pos {
                        let pt = (mx as usize, my as usize);
                        if let Some(ref mut stroke) = active_pen_stroke {
                            if stroke.points.last() != Some(&pt) {
                                stroke.points.push(pt);
                            }
                        } else {
                            active_pen_stroke = Some(InkStroke {
                                points: vec![pt],
                                color: active_color,
                                width: 4,
                            });
                        }
                    }
                } else if !mouse_down
                    && prev_mouse_down
                    && let Some(stroke) = active_pen_stroke.take()
                {
                    slide_ink.entry(current_idx).or_default().push(stroke);
                }
            }

            // Laser pointer position recording for trailing effect
            if presenter_mode == PresenterMode::Laser
                && let Some((mx, my)) = mouse_pos
                && mx >= 0.0
                && my >= 0.0
                && (mx as usize) < width
                && (my as usize) < height
            {
                laser_trail.push_back((mx as usize, my as usize, Instant::now()));
            }
            // Trim old laser trail points (decay over 220ms)
            let now = Instant::now();
            while let Some(front) = laser_trail.front() {
                if now.duration_since(front.2) > Duration::from_millis(220) {
                    laser_trail.pop_front();
                } else {
                    break;
                }
            }

            prev_mouse_down = mouse_down;
            prev_mouse_right_down = mouse_right;

            // Keyboard navigation & tools
            let keys = if is_window_active {
                window.get_keys()
            } else {
                Vec::new()
            };
            let mut reload_triggered = false;

            for key in &keys {
                if !prev_pressed_keys.contains(key) {
                    if let Some(ref mut inspector) = active_chart_inspector {
                        let is_shift =
                            keys.contains(&Key::LeftShift) || keys.contains(&Key::RightShift);
                        if inspector.search_active {
                            match key {
                                | Key::Escape | Key::Enter => {
                                    inspector.search_active = false;
                                },
                                | Key::Backspace => {
                                    inspector.search_query.pop();
                                },
                                | Key::Space => {
                                    inspector.search_query.push(' ');
                                },
                                | Key::Period => {
                                    inspector
                                        .search_query
                                        .push(if is_shift { '>' } else { '.' });
                                },
                                | Key::Comma => {
                                    inspector
                                        .search_query
                                        .push(if is_shift { '<' } else { ',' });
                                },
                                | Key::Equal => {
                                    inspector
                                        .search_query
                                        .push(if is_shift { '+' } else { '=' });
                                },
                                | Key::Minus => {
                                    inspector
                                        .search_query
                                        .push(if is_shift { '_' } else { '-' });
                                },
                                | Key::Slash => {
                                    inspector
                                        .search_query
                                        .push(if is_shift { '?' } else { '/' });
                                },
                                | Key::Key1 if is_shift => {
                                    inspector.search_query.push('!');
                                },
                                | Key::Key0 | Key::NumPad0 => inspector.search_query.push('0'),
                                | Key::Key1 | Key::NumPad1 => inspector.search_query.push('1'),
                                | Key::Key2 | Key::NumPad2 => inspector.search_query.push('2'),
                                | Key::Key3 | Key::NumPad3 => inspector.search_query.push('3'),
                                | Key::Key4 | Key::NumPad4 => inspector.search_query.push('4'),
                                | Key::Key5 | Key::NumPad5 => inspector.search_query.push('5'),
                                | Key::Key6 | Key::NumPad6 => inspector.search_query.push('6'),
                                | Key::Key7 | Key::NumPad7 => inspector.search_query.push('7'),
                                | Key::Key8 | Key::NumPad8 => inspector.search_query.push('8'),
                                | Key::Key9 | Key::NumPad9 => inspector.search_query.push('9'),
                                | Key::A => {
                                    inspector
                                        .search_query
                                        .push(if is_shift { 'A' } else { 'a' })
                                },
                                | Key::B => {
                                    inspector
                                        .search_query
                                        .push(if is_shift { 'B' } else { 'b' })
                                },
                                | Key::C => {
                                    inspector
                                        .search_query
                                        .push(if is_shift { 'C' } else { 'c' })
                                },
                                | Key::D => {
                                    inspector
                                        .search_query
                                        .push(if is_shift { 'D' } else { 'd' })
                                },
                                | Key::E => {
                                    inspector
                                        .search_query
                                        .push(if is_shift { 'E' } else { 'e' })
                                },
                                | Key::F => {
                                    inspector
                                        .search_query
                                        .push(if is_shift { 'F' } else { 'f' })
                                },
                                | Key::G => {
                                    inspector
                                        .search_query
                                        .push(if is_shift { 'G' } else { 'g' })
                                },
                                | Key::H => {
                                    inspector
                                        .search_query
                                        .push(if is_shift { 'H' } else { 'h' })
                                },
                                | Key::I => {
                                    inspector
                                        .search_query
                                        .push(if is_shift { 'I' } else { 'i' })
                                },
                                | Key::J => {
                                    inspector
                                        .search_query
                                        .push(if is_shift { 'J' } else { 'j' })
                                },
                                | Key::K => {
                                    inspector
                                        .search_query
                                        .push(if is_shift { 'K' } else { 'k' })
                                },
                                | Key::L => {
                                    inspector
                                        .search_query
                                        .push(if is_shift { 'L' } else { 'l' })
                                },
                                | Key::M => {
                                    inspector
                                        .search_query
                                        .push(if is_shift { 'M' } else { 'm' })
                                },
                                | Key::N => {
                                    inspector
                                        .search_query
                                        .push(if is_shift { 'N' } else { 'n' })
                                },
                                | Key::O => {
                                    inspector
                                        .search_query
                                        .push(if is_shift { 'O' } else { 'o' })
                                },
                                | Key::P => {
                                    inspector
                                        .search_query
                                        .push(if is_shift { 'P' } else { 'p' })
                                },
                                | Key::Q => {
                                    inspector
                                        .search_query
                                        .push(if is_shift { 'Q' } else { 'q' })
                                },
                                | Key::R => {
                                    inspector
                                        .search_query
                                        .push(if is_shift { 'R' } else { 'r' })
                                },
                                | Key::S => {
                                    inspector
                                        .search_query
                                        .push(if is_shift { 'S' } else { 's' })
                                },
                                | Key::T => {
                                    inspector
                                        .search_query
                                        .push(if is_shift { 'T' } else { 't' })
                                },
                                | Key::U => {
                                    inspector
                                        .search_query
                                        .push(if is_shift { 'U' } else { 'u' })
                                },
                                | Key::V => {
                                    inspector
                                        .search_query
                                        .push(if is_shift { 'V' } else { 'v' })
                                },
                                | Key::W => {
                                    inspector
                                        .search_query
                                        .push(if is_shift { 'W' } else { 'w' })
                                },
                                | Key::X => {
                                    inspector
                                        .search_query
                                        .push(if is_shift { 'X' } else { 'x' })
                                },
                                | Key::Y => {
                                    inspector
                                        .search_query
                                        .push(if is_shift { 'Y' } else { 'y' })
                                },
                                | Key::Z => {
                                    inspector
                                        .search_query
                                        .push(if is_shift { 'Z' } else { 'z' })
                                },
                                | _ => {},
                            }
                            inspector.recompute_cache();
                        } else {
                            match key {
                                | Key::Escape => {
                                    if !inspector.search_query.is_empty()
                                        || inspector.marquee_range.is_some()
                                    {
                                        inspector.clear_filter();
                                    } else {
                                        if let Some(hs_idx) = hovered_hotspot_idx {
                                            chart_type_overrides.insert(
                                                (current_idx, hs_idx),
                                                inspector.active_type,
                                            );
                                        }
                                        active_chart_inspector = None;
                                    }
                                },
                                | Key::Slash => {
                                    inspector.search_active = true;
                                },
                                | Key::F => {
                                    inspector.cycle_format();
                                },
                                | Key::X => {
                                    inspector.clear_filter();
                                },
                                | Key::Tab => {
                                    inspector.cycle_type();
                                },
                                | Key::Key1 | Key::NumPad1 => inspector.toggle_series(0),
                                | Key::Key2 | Key::NumPad2 => inspector.toggle_series(1),
                                | Key::Key3 | Key::NumPad3 => inspector.toggle_series(2),
                                | Key::Key4 | Key::NumPad4 => inspector.toggle_series(3),
                                | Key::Key5 | Key::NumPad5 => inspector.toggle_series(4),
                                | Key::Key6 | Key::NumPad6 => inspector.toggle_series(5),
                                | Key::Key7 | Key::NumPad7 => inspector.toggle_series(6),
                                | Key::Key8 | Key::NumPad8 => inspector.toggle_series(7),
                                | Key::Key9 | Key::NumPad9 => inspector.toggle_series(8),
                                | Key::C | Key::E => {
                                    let csv =
                                        inspector.chart_data.export_csv(&inspector.hidden_series);
                                    if let Err(e) = std::fs::write("chart_export.csv", csv) {
                                        eprintln!("Failed to export CSV: {}", e);
                                        inspector.show_toast("Export failed!");
                                    } else {
                                        inspector.show_toast("Exported to chart_export.csv");
                                    }
                                },
                                | Key::Up => {
                                    inspector.table_scroll =
                                        inspector.table_scroll.saturating_sub(1);
                                },
                                | Key::Down => {
                                    let (_, _, _, _, _, _, _, right_pane) =
                                        get_inspector_layout(width, height);
                                    let header_h = 56.0;
                                    let row_h = 24.0;
                                    let visible_rows =
                                        ((right_pane.height - header_h - 2.0).max(0.0) / row_h)
                                            as usize;
                                    let filtered_len =
                                        inspector.get_filtered_category_indices().len();
                                    let max_scroll = filtered_len.saturating_sub(visible_rows);
                                    inspector.table_scroll =
                                        (inspector.table_scroll + 1).min(max_scroll);
                                },
                                | Key::O => {
                                    inspector.set_transform(slide_core::chart::ChartTransform::None)
                                },
                                | Key::T => {
                                    inspector
                                        .set_transform(slide_core::chart::ChartTransform::TopK(5))
                                },
                                | Key::S => {
                                    inspector
                                        .set_transform(slide_core::chart::ChartTransform::SortDesc)
                                },
                                | Key::A => {
                                    inspector
                                        .set_transform(slide_core::chart::ChartTransform::SortAsc)
                                },
                                | Key::U => {
                                    inspector.set_transform(
                                        slide_core::chart::ChartTransform::Cumulative,
                                    )
                                },
                                | Key::P => {
                                    inspector.set_transform(
                                        slide_core::chart::ChartTransform::Percent100,
                                    )
                                },
                                | Key::M => {
                                    inspector.set_transform(
                                        slide_core::chart::ChartTransform::MovingAvg(3),
                                    )
                                },
                                | _ => {},
                            }
                        }
                    } else {
                        match key {
                            | Key::Right | Key::Down | Key::Space | Key::PageDown | Key::Enter => {
                                trigger_forward = true;
                            },
                            | Key::Left | Key::Up | Key::Backspace | Key::PageUp => {
                                trigger_backward = true;
                            },
                            | Key::Home => jump_target = Some(0),
                            | Key::End => jump_target = Some(total_slides.saturating_sub(1)),
                            | Key::NumPad1 | Key::Key1 => {
                                if palette_open {
                                    active_color_idx = 0;
                                } else {
                                    jump_target = Some(0);
                                }
                            },
                            | Key::NumPad2 | Key::Key2 => {
                                if palette_open {
                                    active_color_idx = 1;
                                } else {
                                    jump_target = Some(1);
                                }
                            },
                            | Key::NumPad3 | Key::Key3 => {
                                if palette_open {
                                    active_color_idx = 2;
                                } else {
                                    jump_target = Some(2);
                                }
                            },
                            | Key::NumPad4 | Key::Key4 => {
                                if palette_open {
                                    active_color_idx = 3;
                                } else {
                                    jump_target = Some(3);
                                }
                            },
                            | Key::NumPad5 | Key::Key5 => {
                                if palette_open {
                                    active_color_idx = 4;
                                } else {
                                    jump_target = Some(4);
                                }
                            },
                            | Key::NumPad6 | Key::Key6 => {
                                if palette_open {
                                    active_color_idx = 5;
                                } else {
                                    jump_target = Some(5);
                                }
                            },
                            | Key::NumPad7 | Key::Key7 => {
                                if palette_open {
                                    active_color_idx = 6;
                                } else {
                                    jump_target = Some(6);
                                }
                            },
                            | Key::NumPad8 | Key::Key8 => jump_target = Some(7),
                            | Key::NumPad9 | Key::Key9 => jump_target = Some(8),
                            | Key::F11 | Key::F => {
                                toggle_fullscreen_requested = true;
                            },
                            | Key::L => {
                                presenter_mode = if presenter_mode == PresenterMode::Laser {
                                    PresenterMode::Normal
                                } else {
                                    PresenterMode::Laser
                                };
                            },
                            | Key::P => {
                                presenter_mode = if presenter_mode == PresenterMode::Pen {
                                    PresenterMode::Normal
                                } else {
                                    PresenterMode::Pen
                                };
                            },
                            | Key::K => {
                                palette_open = !palette_open;
                            },
                            | Key::C | Key::X => {
                                slide_ink.remove(&current_idx);
                                active_pen_stroke = None;
                            },
                            | Key::Equal | Key::NumPadPlus => {
                                audio_engine.adjust_volume(0.05);
                                last_volume_change = Some(Instant::now());
                            },
                            | Key::Minus | Key::NumPadMinus => {
                                audio_engine.adjust_volume(-0.05);
                                last_volume_change = Some(Instant::now());
                            },
                            | Key::M => {
                                audio_engine.toggle_mute();
                                last_volume_change = Some(Instant::now());
                            },
                            | Key::Escape => {
                                if is_fullscreen {
                                    toggle_fullscreen_requested = true;
                                } else {
                                    exit_requested = true;
                                }
                            },
                            | Key::H | Key::Slash => show_help = !show_help,
                            | Key::R => reload_triggered = true,
                            | _ => {},
                        }
                    }
                }
            }
            prev_pressed_keys = keys;

            // Fullscreen toggle
            if toggle_fullscreen_requested {
                is_fullscreen = !is_fullscreen;
                let (target_w, target_h) = if is_fullscreen {
                    get_screen_resolution().unwrap_or((1920, 1080))
                } else {
                    (self.config.width, self.config.height)
                };
                width = target_w;
                height = target_h;
                window = create_window(&self.config.title, width, height, is_fullscreen)?;
                buffer.resize(width * height, 0);
                cache.clear();
                first_frame = true;
            }

            // Hot reload check from file watcher
            let mut hot_reload_detected = false;
            if let Some(ref mut w) = watcher
                && let Some(changed_path) = w.poll_change()
            {
                println!("📝 Detected change in: {}", changed_path.display());
                hot_reload_detected = true;
            }

            // Live reload (watcher trigger or manual R key)
            if reload_triggered || hot_reload_detected {
                active_chart_inspector = None;
                if let Some(ref file) = self.source_file {
                    match SlideCompiler::new().and_then(|c| c.compile_file(file)) {
                        | Ok(new_deck) => {
                            self.deck = new_deck;
                            total_slides = self.deck.total_slides();
                            current_idx = current_idx.min(total_slides.saturating_sub(1));
                            let max_step = get_max_step(&self.deck, current_idx);
                            current_step = current_step.min(max_step);
                            if current_step == 0 && max_step > 0 {
                                current_step = 1;
                            }
                            cache.clear();
                            println!("🔥 Slides live-reloaded! ({} slides)", total_slides);
                            slide_core::logger::log_event(
                                "success",
                                &format!("✨ Slides live-reloaded! ({} slides)", total_slides),
                                Some(serde_json::json!({
                                    "stage": "hot_reload_success",
                                    "total_slides": total_slides,
                                    "current_slide": current_idx + 1,
                                })),
                            );
                            trigger_slide_audio(
                                &self.deck,
                                current_idx,
                                &mut audio_engine,
                                self.source_file.as_deref(),
                            );
                        },
                        | Err(e) => {
                            eprintln!("⚠️ [cargo-slide] Hot reload compilation error:\n{}", e);
                            slide_core::logger::log_event(
                                "error",
                                &format!("Hot reload compilation error: {}", e),
                                Some(serde_json::json!({
                                    "stage": "hot_reload_error",
                                    "error": e.to_string(),
                                })),
                            );
                        },
                    }
                    if let Some(ref mut w) = watcher {
                        w.drain();
                    }
                }
            }

            // Compute step builds or slide progression
            let max_substep = get_max_step(&self.deck, current_idx);
            let mut next_slide = false;
            let mut prev_slide = false;

            if trigger_forward {
                if current_step < max_substep {
                    current_step += 1;
                    // Trigger in-slide component animation
                    if let Some(slide) = self.deck.get_slide(current_idx)
                        && let Some(fragment) = slide.steps.iter().find(|s| s.order == current_step)
                    {
                        let screen_rect = current_metrics.svg_to_screen_rect(&fragment.rect);
                        transition_mgr.start_component_step(&fragment.effect, screen_rect);
                    }
                } else if current_idx + 1 < total_slides {
                    next_slide = true;
                }
            } else if trigger_backward {
                if current_step > 1 {
                    current_step -= 1;
                } else if current_idx > 0 {
                    prev_slide = true;
                }
            }

            // Compute new slide index
            let mut new_slide_idx = current_idx;
            if next_slide && current_idx + 1 < total_slides {
                new_slide_idx = current_idx + 1;
            } else if prev_slide && current_idx > 0 {
                new_slide_idx = current_idx - 1;
            } else if let Some(target) = jump_target
                && target < total_slides
            {
                new_slide_idx = target;
            }

            if new_slide_idx != current_idx {
                active_chart_inspector = None;
                hovered_hotspot_key = None;
                hotspot_hover_alpha = 0.0;
                // If slide specified transition override in Typst (e.g. transition: "glitch"), use it!
                let anim_name = self
                    .deck
                    .get_slide(new_slide_idx)
                    .and_then(|s| s.animation.as_deref())
                    .unwrap_or(&self.config.default_animation);

                let (target_surf, target_metrics) = match cache.get(&new_slide_idx) {
                    | Some(cached) => (cached.0.clone(), cached.1),
                    | None => {
                        let s = self.deck.get_slide(new_slide_idx).unwrap();
                        let (sf, met) = renderer.render_svg(&s.svg_data, width, height)?;
                        let sf_arc = Arc::new(sf);
                        cache.insert(new_slide_idx, (sf_arc.clone(), met));
                        (sf_arc, met)
                    },
                };

                let from_slide = self.deck.get_slide(current_idx).unwrap();
                let to_slide = self.deck.get_slide(new_slide_idx).unwrap();
                let new_max_step = get_max_step(&self.deck, new_slide_idx);
                let next_step = if prev_slide {
                    new_max_step
                } else if new_max_step > 0 {
                    1
                } else {
                    0
                };

                let from_bg = current_surface.bg_color;
                let mut from_stepped = (*current_surface).clone();
                from_stepped.mask_steps(&from_slide.steps, current_step, &current_metrics, from_bg);

                let to_bg = target_surf.bg_color;
                let mut to_stepped = (*target_surf).clone();
                to_stepped.mask_steps(&to_slide.steps, next_step, &target_metrics, to_bg);

                transition_mgr.start_transition(
                    anim_name,
                    Arc::new(from_stepped),
                    Arc::new(to_stepped),
                );
                current_idx = new_slide_idx;
                current_step = next_step;
                trigger_slide_audio(
                    &self.deck,
                    current_idx,
                    &mut audio_engine,
                    self.source_file.as_deref(),
                );
            }

            // Draw whiteboard ink strokes for current slide
            if let Some(strokes) = slide_ink.get(&current_idx) {
                render_ink_strokes(&mut buffer, width, height, strokes);
            }
            if let Some(ref current_stroke) = active_pen_stroke {
                render_ink_strokes(
                    &mut buffer,
                    width,
                    height,
                    std::slice::from_ref(current_stroke),
                );
            }

            // Draw laser pointer and trailing effect if in Laser mode
            if presenter_mode == PresenterMode::Laser {
                laser_trail_scratch.clear();
                laser_trail_scratch.extend(laser_trail.iter().map(|(x, y, t)| {
                    let age_ms = now.duration_since(*t).as_millis() as f32;
                    let freshness = (1.0 - age_ms / 220.0).clamp(0.0, 1.0);
                    (*x, *y, freshness)
                }));
                render_laser_trail(
                    &mut buffer,
                    width,
                    height,
                    &laser_trail_scratch,
                    active_color,
                );

                if let Some((mx, my)) = mouse_pos
                    && mx >= 0.0
                    && my >= 0.0
                    && (mx as usize) < width
                    && (my as usize) < height
                {
                    render_laser_pointer(
                        &mut buffer,
                        width,
                        height,
                        mx as usize,
                        my as usize,
                        active_color,
                    );
                }
            }

            // Draw floating interactive dock
            render_dock(
                &mut buffer,
                width,
                height,
                presenter_mode,
                audio_engine.is_muted(),
                audio_engine.volume_percent(),
                hovered_dock_action,
                mouse_near_dock,
                is_fullscreen,
                active_color,
                palette_open,
            );

            // Draw Color Palette popup if open
            if palette_open {
                render_palette_popup(
                    &mut buffer,
                    width,
                    height,
                    is_fullscreen,
                    active_color_idx,
                    hovered_palette_idx,
                );
            }

            // Draw Volume Slider popup if hovering volume
            if show_volume_slider {
                render_volume_slider_popup(
                    &mut buffer,
                    width,
                    height,
                    is_fullscreen,
                    audio_engine.volume(),
                    audio_engine.is_muted(),
                );
            }

            // Draw temporary Volume HUD Toast overlay if volume recently adjusted
            if let Some(t) = last_volume_change {
                let elapsed = t.elapsed().as_secs_f32();
                if elapsed < 1.5 {
                    let toast_alpha = if elapsed < 1.0 {
                        1.0
                    } else {
                        1.0 - (elapsed - 1.0) / 0.5
                    };
                    draw_volume_toast(
                        &mut buffer,
                        width,
                        height,
                        audio_engine.volume_percent(),
                        audio_engine.is_muted(),
                        toast_alpha,
                    );
                }
            }

            // Draw HUD slide & step badges
            draw_page_badge(
                &mut buffer,
                width,
                height,
                current_idx + 1,
                total_slides,
                current_step,
                max_substep,
            );

            if show_help {
                draw_help_overlay(&mut buffer, width, height);
            }

            // Render active Chart Data Inspector modal
            if let Some(ref inspector) = active_chart_inspector {
                let (mx, my) = mouse_pos.unwrap_or((-1.0, -1.0));
                draw_chart_inspector(&mut buffer, width, height, inspector, mx, my);
            }

            window
                .update_with_buffer(&buffer, width, height)
                .map_err(|e| SlideError::Format(format!("Window update error: {}", e)))?;

            if first_frame {
                first_frame = false;
                if is_fullscreen {
                    apply_native_fullscreen(window.get_window_handle(), true, width, height);
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_decode() {
        assert_eq!(url_decode("hello%20world"), "hello world");
        assert_eq!(url_decode("path%2Fto%2Ffile.pdf"), "path/to/file.pdf");
        assert_eq!(url_decode("no_percent"), "no_percent");
        assert_eq!(url_decode("%E4%BD%A0%E5%A5%BD"), "你好");
        // Malformed or trailing percents
        assert_eq!(url_decode("bad%"), "bad%");
        assert_eq!(url_decode("bad%2"), "bad%2");
        assert_eq!(url_decode("bad%ZZ"), "bad%ZZ");
    }

    #[test]
    fn test_normalize_windows_path() {
        let p = PathBuf::from("normal/path/file.txt");
        assert_eq!(normalize_windows_path(p.clone()), p);
    }

    #[test]
    fn test_resolve_local_file_path() {
        // Resolve README.md from repo root
        let root_readme = resolve_local_file_path("README.md", None);
        assert!(
            root_readme.is_some(),
            "Should find README.md in CWD or ancestors"
        );

        // Resolve with file:// and percent encoding
        let encoded_file = resolve_local_file_path("file://README.md", None);
        assert!(encoded_file.is_some());

        // Resolve relative to a nested hypothetical source file
        let fake_slide = PathBuf::from("examples/geek-presentation/slides.typ");
        let found_from_nested = resolve_local_file_path("README.md", Some(&fake_slide));
        assert!(
            found_from_nested.is_some(),
            "Should find README.md via ancestor traversal"
        );

        // Nonexistent file should return None
        let not_found = resolve_local_file_path("non_existent_file_xyz_12345.typ", None);
        assert!(not_found.is_none());
    }
}
