use slide_core::animation::blend_pixel_fast;
use slide_core::chart::ChartData;
use slide_core::chart::ChartTransform;
use slide_core::chart::ChartType;
use slide_core::chart::DEFAULT_CHART_COLORS;
use slide_core::chart::SeriesData;
use slide_core::model::Rect;
use slide_core::model::RenderMetrics;

pub const PALETTE_COLORS: [(&str, u32); 7] = [
    ("Cyan", 0xFF00E5FF),   // Neon Cyan (default for pen)
    ("Red", 0xFFFF3366),    // Crimson Neon Red (default for laser)
    ("Green", 0xFF00E676),  // Bright Emerald Green
    ("Yellow", 0xFFFFD600), // Solar Yellow
    ("Purple", 0xFFD500F9), // Electric Purple
    ("White", 0xFFFFFFFF),  // Crisp White
    ("Orange", 0xFFFF9100), // Vivid Coral Orange
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresenterMode {
    Normal,
    Laser,
    Pen,
}

#[derive(Debug, Clone)]
pub struct InkStroke {
    pub points: Vec<(usize, usize)>,
    pub color: u32,
    pub width: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockAction {
    Prev,
    Next,
    ModeNormal,
    ModeLaser,
    ModePen,
    TogglePalette,
    ClearInk,
    ToggleMute,
    ToggleFullscreen,
    ToggleHelp,
}

/// Draw a smoothly animated highlight rectangle around active/hovered hotspot
pub fn draw_hotspot_highlight(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    screen_rect: Rect,
    color: u32,
    alpha: f32,
) {
    if alpha <= 0.005 {
        return;
    }
    let a = alpha.clamp(0.0, 1.0);

    let x1 = (screen_rect.x as usize).min(width.saturating_sub(1));
    let y1 = (screen_rect.y as usize).min(height.saturating_sub(1));
    let x2 = ((screen_rect.x + screen_rect.width) as usize).min(width.saturating_sub(1));
    let y2 = ((screen_rect.y + screen_rect.height) as usize).min(height.saturating_sub(1));

    if x2 <= x1 || y2 <= y1 {
        return;
    }

    let cr = (color >> 16) & 0xFF;
    let cg = (color >> 8) & 0xFF;
    let cb = color & 0xFF;

    #[inline]
    fn blend_px(
        bg: u32,
        r: u32,
        g: u32,
        b: u32,
        alpha: f32,
    ) -> u32 {
        let bg_r = (bg >> 16) & 0xFF;
        let bg_g = (bg >> 8) & 0xFF;
        let bg_b = bg & 0xFF;
        let out_r = ((r as f32 * alpha + bg_r as f32 * (1.0 - alpha)) as u32).min(255);
        let out_g = ((g as f32 * alpha + bg_g as f32 * (1.0 - alpha)) as u32).min(255);
        let out_b = ((b as f32 * alpha + bg_b as f32 * (1.0 - alpha)) as u32).min(255);
        (out_r << 16) | (out_g << 8) | out_b
    }

    // 1. Subtle interior card tint fill (opacity = a * 0.08)
    let fill_alpha = a * 0.08;
    for y in (y1 + 2)..(y2.saturating_sub(1)) {
        let row = y * width;
        for x in (x1 + 2)..(x2.saturating_sub(1)) {
            let idx = row + x;
            buffer[idx] = blend_px(buffer[idx], cr, cg, cb, fill_alpha);
        }
    }

    // 2. 1px soft outer glow fringe (opacity = a * 0.35)
    let glow_alpha = a * 0.35;
    if y1 > 0 {
        let row = (y1 - 1) * width;
        for x in x1..=x2 {
            buffer[row + x] = blend_px(buffer[row + x], cr, cg, cb, glow_alpha);
        }
    }
    if y2 + 1 < height {
        let row = (y2 + 1) * width;
        for x in x1..=x2 {
            buffer[row + x] = blend_px(buffer[row + x], cr, cg, cb, glow_alpha);
        }
    }
    if x1 > 0 {
        for y in y1..=y2 {
            let idx = y * width + (x1 - 1);
            buffer[idx] = blend_px(buffer[idx], cr, cg, cb, glow_alpha);
        }
    }
    if x2 + 1 < width {
        for y in y1..=y2 {
            let idx = y * width + (x2 + 1);
            buffer[idx] = blend_px(buffer[idx], cr, cg, cb, glow_alpha);
        }
    }

    // 3. 2px crisp border (opacity = a * 0.90)
    let border_alpha = a * 0.90;
    for x in x1..=x2 {
        let idx1 = y1 * width + x;
        let idx2 = y2 * width + x;
        buffer[idx1] = blend_px(buffer[idx1], cr, cg, cb, border_alpha);
        buffer[idx2] = blend_px(buffer[idx2], cr, cg, cb, border_alpha);
        if y1 + 1 < height {
            let idx3 = (y1 + 1) * width + x;
            buffer[idx3] = blend_px(buffer[idx3], cr, cg, cb, border_alpha);
        }
        if y2 > 0 {
            let idx4 = (y2 - 1) * width + x;
            buffer[idx4] = blend_px(buffer[idx4], cr, cg, cb, border_alpha);
        }
    }
    for y in y1..=y2 {
        let idx1 = y * width + x1;
        let idx2 = y * width + x2;
        buffer[idx1] = blend_px(buffer[idx1], cr, cg, cb, border_alpha);
        buffer[idx2] = blend_px(buffer[idx2], cr, cg, cb, border_alpha);
        if x1 + 1 < width {
            let idx3 = y * width + (x1 + 1);
            buffer[idx3] = blend_px(buffer[idx3], cr, cg, cb, border_alpha);
        }
        if x2 > 0 {
            let idx4 = y * width + (x2 - 1);
            buffer[idx4] = blend_px(buffer[idx4], cr, cg, cb, border_alpha);
        }
    }
}

/// Draw the simulated glowing laser pointer with active color
pub fn render_laser_pointer(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    cx: usize,
    cy: usize,
    color: u32,
) {
    let radius: isize = 13;
    let r_sq = radius * radius;
    let cx_i = cx as isize;
    let cy_i = cy as isize;

    let cr = (color >> 16) & 0xFF;
    let cg = (color >> 8) & 0xFF;
    let cb = color & 0xFF;

    for dy in -radius..=radius {
        let py = cy_i + dy;
        if py < 0 || py >= height as isize {
            continue;
        }
        let py_u = py as usize;
        let row_offset = py_u * width;

        for dx in -radius..=radius {
            let dist_sq = dx * dx + dy * dy;
            if dist_sq > r_sq {
                continue;
            }
            let px = cx_i + dx;
            if px < 0 || px >= width as isize {
                continue;
            }
            let px_u = px as usize;
            let dist = (dist_sq as f32).sqrt();

            let (lr, lg, lb, alpha) = if dist <= 2.5 {
                (255, 255, 255, 1.0f32) // Glowing brilliant core
            } else if dist <= 5.5 {
                (cr, cg, cb, 0.90f32) // Vibrant colored core
            } else {
                let falloff = (1.0 - (dist - 5.5) / (radius as f32 - 5.5)).max(0.0);
                (cr, cg, cb, falloff * falloff * 0.70) // Smooth outer aura
            };

            let bg = buffer[row_offset + px_u];
            let bg_r = (bg >> 16) & 0xFF;
            let bg_g = (bg >> 8) & 0xFF;
            let bg_b = bg & 0xFF;

            let out_r = ((1.0 - alpha) * bg_r as f32 + alpha * lr as f32).min(255.0) as u32;
            let out_g = ((1.0 - alpha) * bg_g as f32 + alpha * lg as f32).min(255.0) as u32;
            let out_b = ((1.0 - alpha) * bg_b as f32 + alpha * lb as f32).min(255.0) as u32;

            buffer[row_offset + px_u] = (0xFF << 24) | (out_r << 16) | (out_g << 8) | out_b;
        }
    }
}

/// Draw fading laser motion blur / phosphorescent trailing tail
pub fn render_laser_trail(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    trail: &[(usize, usize, f32)], // (x, y, freshness factor 0.0..1.0)
    color: u32,
) {
    if trail.len() < 2 {
        return;
    }

    for window in trail.windows(2) {
        let (x0, y0, f0) = window[0];
        let (x1, y1, f1) = window[1];
        let avg_f = (f0 + f1) * 0.5;
        let radius = (1.5 + avg_f * 3.5).round() as isize;
        let alpha = avg_f * avg_f * 0.75;

        draw_thick_line_alpha(
            buffer,
            width,
            height,
            x0 as isize,
            y0 as isize,
            x1 as isize,
            y1 as isize,
            radius,
            color,
            alpha,
        );
    }
}

/// Draw a continuous thick line segment with alpha blending
fn draw_thick_line_alpha(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    mut x0: isize,
    mut y0: isize,
    x1: isize,
    y1: isize,
    radius: isize,
    color: u32,
    alpha: f32,
) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        draw_disk_alpha(buffer, width, height, x0, y0, radius, color, alpha);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

/// Draw a disk stamp with alpha blending
fn draw_disk_alpha(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    cx: isize,
    cy: isize,
    r: isize,
    color: u32,
    alpha: f32,
) {
    let r_sq = r * r;
    let cr = (color >> 16) & 0xFF;
    let cg = (color >> 8) & 0xFF;
    let cb = color & 0xFF;

    for dy in -r..=r {
        let py = cy + dy;
        if py < 0 || py >= height as isize {
            continue;
        }
        let row_offset = (py as usize) * width;
        for dx in -r..=r {
            if dx * dx + dy * dy <= r_sq {
                let px = cx + dx;
                if px >= 0 && px < width as isize {
                    let idx = row_offset + px as usize;
                    let bg = buffer[idx];
                    let bg_r = (bg >> 16) & 0xFF;
                    let bg_g = (bg >> 8) & 0xFF;
                    let bg_b = bg & 0xFF;

                    let out_r = ((1.0 - alpha) * bg_r as f32 + alpha * cr as f32) as u32;
                    let out_g = ((1.0 - alpha) * bg_g as f32 + alpha * cg as f32) as u32;
                    let out_b = ((1.0 - alpha) * bg_b as f32 + alpha * cb as f32) as u32;

                    buffer[idx] = (0xFF << 24) | (out_r << 16) | (out_g << 8) | out_b;
                }
            }
        }
    }
}

/// Draw a disk / circle stamp for ink strokes
fn draw_disk(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    cx: isize,
    cy: isize,
    r: isize,
    color: u32,
) {
    let r_sq = r * r;
    for dy in -r..=r {
        let py = cy + dy;
        if py < 0 || py >= height as isize {
            continue;
        }
        let row_offset = (py as usize) * width;
        for dx in -r..=r {
            if dx * dx + dy * dy <= r_sq {
                let px = cx + dx;
                if px >= 0 && px < width as isize {
                    buffer[row_offset + px as usize] = color;
                }
            }
        }
    }
}

/// Draw a continuous thick line segment between two points
fn draw_thick_line(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    mut x0: isize,
    mut y0: isize,
    x1: isize,
    y1: isize,
    radius: isize,
    color: u32,
) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        draw_disk(buffer, width, height, x0, y0, radius, color);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

/// Render whiteboard pen strokes onto the screen
pub fn render_ink_strokes(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    strokes: &[InkStroke],
) {
    for stroke in strokes {
        let r = (stroke.width / 2).max(1) as isize;
        if stroke.points.is_empty() {
            continue;
        }
        if stroke.points.len() == 1 {
            let (x, y) = stroke.points[0];
            draw_disk(
                buffer,
                width,
                height,
                x as isize,
                y as isize,
                r,
                stroke.color,
            );
            continue;
        }
        for window in stroke.points.windows(2) {
            let (x0, y0) = window[0];
            let (x1, y1) = window[1];
            draw_thick_line(
                buffer,
                width,
                height,
                x0 as isize,
                y0 as isize,
                x1 as isize,
                y1 as isize,
                r,
                stroke.color,
            );
        }
    }
}

/// Calculate dock bounds and individual button bounds
pub fn get_dock_rects(
    screen_w: usize,
    screen_h: usize,
    is_fullscreen: bool,
) -> (Rect, Vec<(DockAction, Rect, &'static str)>) {
    let dock_w = 460.0f32;
    let dock_h = 36.0f32;
    let dock_x = ((screen_w as f32) - dock_w) / 2.0;
    let dock_y = (screen_h as f32) - 48.0;

    let dock_rect = Rect::new(dock_x, dock_y, dock_w, dock_h);

    let full_label = if is_fullscreen {
        "WIN"
    } else {
        "FULL"
    };

    let items_def = [
        (DockAction::Prev, "<"),
        (DockAction::Next, ">"),
        (DockAction::ModeNormal, "PTR"),
        (DockAction::ModeLaser, "LSR"),
        (DockAction::ModePen, "PEN"),
        (DockAction::TogglePalette, "COL"),
        (DockAction::ClearInk, "CLR"),
        (DockAction::ToggleMute, "VOL"),
        (DockAction::ToggleFullscreen, full_label),
        (DockAction::ToggleHelp, "?"),
    ];

    let pad_x = 8.0f32;
    let pad_y = 5.0f32;
    let gap = 4.0f32;
    let total_gaps = (items_def.len() - 1) as f32 * gap;
    let btn_w = (dock_w - pad_x * 2.0 - total_gaps) / (items_def.len() as f32);
    let btn_h = dock_h - pad_y * 2.0;

    let mut item_rects = Vec::new();
    let mut cur_x = dock_x + pad_x;

    for (action, label) in items_def {
        let btn_rect = Rect::new(cur_x, dock_y + pad_y, btn_w, btn_h);
        item_rects.push((action, btn_rect, label));
        cur_x += btn_w + gap;
    }

    (dock_rect, item_rects)
}

/// Check if a mouse coordinate hits any dock action
pub fn hit_test_dock(
    screen_w: usize,
    screen_h: usize,
    is_fullscreen: bool,
    mx: f32,
    my: f32,
) -> Option<DockAction> {
    let (dock_rect, items) = get_dock_rects(screen_w, screen_h, is_fullscreen);
    if !dock_rect.contains(mx, my) {
        return None;
    }
    for (action, rect, _) in items {
        if rect.contains(mx, my) {
            return Some(action);
        }
    }
    None
}

/// Draw the floating interactive dock
pub fn render_dock(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    mode: PresenterMode,
    is_muted: bool,
    volume: u32,
    hovered: Option<DockAction>,
    mouse_near: bool,
    is_fullscreen: bool,
    active_color: u32,
    palette_open: bool,
) {
    let (dock_rect, items) = get_dock_rects(width, height, is_fullscreen);

    let alpha = if mouse_near {
        0.94f32
    } else {
        0.45f32
    };

    let start_x = dock_rect.x as usize;
    let start_y = dock_rect.y as usize;
    let end_x = ((dock_rect.x + dock_rect.width) as usize).min(width.saturating_sub(1));
    let end_y = ((dock_rect.y + dock_rect.height) as usize).min(height.saturating_sub(1));

    // Draw dock background
    for y in start_y..=end_y {
        let row_offset = y * width;
        for x in start_x..=end_x {
            let bg = buffer[row_offset + x];
            let r = (((bg >> 16) & 0xFF) as f32 * (1.0 - alpha) + 0x16 as f32 * alpha) as u32;
            let g = (((bg >> 8) & 0xFF) as f32 * (1.0 - alpha) + 0x1b as f32 * alpha) as u32;
            let b = ((bg & 0xFF) as f32 * (1.0 - alpha) + 0x22 as f32 * alpha) as u32;
            buffer[row_offset + x] = (0xFF << 24) | (r << 16) | (g << 8) | b;
        }
    }

    // Dock outline border
    let border_color = if mouse_near {
        0xFF388bfd
    } else {
        0xFF30363d
    };
    for x in start_x..=end_x {
        buffer[start_y * width + x] = border_color;
        buffer[end_y * width + x] = border_color;
    }
    for y in start_y..=end_y {
        buffer[y * width + start_x] = border_color;
        buffer[y * width + end_x] = border_color;
    }

    // Render each dock button
    for (action, rect, default_label) in items {
        let is_active = match action {
            | DockAction::ModeNormal => mode == PresenterMode::Normal,
            | DockAction::ModeLaser => mode == PresenterMode::Laser,
            | DockAction::ModePen => mode == PresenterMode::Pen,
            | DockAction::TogglePalette => palette_open,
            | _ => false,
        };
        let is_hovered = hovered == Some(action);

        let bx1 = rect.x as usize;
        let by1 = rect.y as usize;
        let bx2 = ((rect.x + rect.width) as usize).min(width.saturating_sub(1));
        let by2 = ((rect.y + rect.height) as usize).min(height.saturating_sub(1));

        // Background color for button
        if is_active {
            for y in by1..=by2 {
                let row = y * width;
                for x in bx1..=bx2 {
                    buffer[row + x] = 0xFF1f6feb; // Vibrant blue for active
                }
            }
        } else if is_hovered {
            for y in by1..=by2 {
                let row = y * width;
                for x in bx1..=bx2 {
                    buffer[row + x] = 0xFF30363d; // Gray hover
                }
            }
        }

        // Custom label / badge for volume
        let label_owned;
        let label = if action == DockAction::ToggleMute {
            if is_muted {
                "MUT"
            } else {
                label_owned = format!("{}%", volume.min(100));
                &label_owned
            }
        } else {
            default_label
        };

        let text_color = if is_active {
            0xFFffffff
        } else if action == DockAction::TogglePalette {
            active_color // Show selected active palette color!
        } else if is_hovered {
            0xFF58a6ff
        } else if action == DockAction::ToggleMute && is_muted {
            0xFFf85149
        } else {
            0xFFc9d1d9
        };

        draw_text_centered(buffer, width, height, rect, label, text_color);
    }
}

/// Calculate bounds for the floating color palette popup
pub fn get_palette_rects(
    screen_w: usize,
    screen_h: usize,
    is_fullscreen: bool,
) -> (Rect, Vec<(usize, Rect)>) {
    let (_, dock_items) = get_dock_rects(screen_w, screen_h, is_fullscreen);
    let col_btn = dock_items
        .iter()
        .find(|(a, _, _)| *a == DockAction::TogglePalette)
        .map(|(_, r, _)| *r)
        .unwrap_or(Rect::new(
            screen_w as f32 / 2.0,
            screen_h as f32 - 48.0,
            36.0,
            26.0,
        ));

    let vol_btn_x = dock_items
        .iter()
        .find(|(a, _, _)| *a == DockAction::ToggleMute)
        .map(|(_, r, _)| r.x)
        .unwrap_or(col_btn.x + 80.0);

    let popup_w = 175.0f32;
    let popup_h = 36.0f32;
    // Constrain palette popup so its right edge is strictly clear of the volume control
    let max_x = vol_btn_x - 8.0 - popup_w;
    let desired_x = col_btn.x + col_btn.width / 2.0 - popup_w / 2.0;
    let popup_x = desired_x.clamp(10.0, max_x.max(10.0));
    let popup_y = col_btn.y - popup_h - 8.0;

    let popup_rect = Rect::new(popup_x, popup_y, popup_w, popup_h);

    let count = PALETTE_COLORS.len();
    let pad = 6.0f32;
    let gap = 3.0f32;
    let chip_w = (popup_w - pad * 2.0 - (count - 1) as f32 * gap) / count as f32;
    let chip_h = popup_h - pad * 2.0;

    let mut chip_rects = Vec::new();
    let mut cur_x = popup_x + pad;
    for i in 0..count {
        chip_rects.push((i, Rect::new(cur_x, popup_y + pad, chip_w, chip_h)));
        cur_x += chip_w + gap;
    }

    (popup_rect, chip_rects)
}

/// Check if mouse clicked on any color swatch chip
pub fn hit_test_palette(
    screen_w: usize,
    screen_h: usize,
    is_fullscreen: bool,
    mx: f32,
    my: f32,
) -> Option<usize> {
    let (popup_rect, chips) = get_palette_rects(screen_w, screen_h, is_fullscreen);
    if !popup_rect.contains(mx, my) {
        return None;
    }
    for (idx, rect) in chips {
        if rect.contains(mx, my) {
            return Some(idx);
        }
    }
    None
}

/// Render the elegant floating color palette popup
pub fn render_palette_popup(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    is_fullscreen: bool,
    active_idx: usize,
    hovered_idx: Option<usize>,
) {
    let (popup_rect, chips) = get_palette_rects(width, height, is_fullscreen);

    let x1 = popup_rect.x as usize;
    let y1 = popup_rect.y as usize;
    let x2 = ((popup_rect.x + popup_rect.width) as usize).min(width.saturating_sub(1));
    let y2 = ((popup_rect.y + popup_rect.height) as usize).min(height.saturating_sub(1));

    // Dark glassmorphic background
    for y in y1..=y2 {
        let row = y * width;
        for x in x1..=x2 {
            buffer[row + x] = 0xF0161b22;
        }
    }

    // Border
    for x in x1..=x2 {
        buffer[y1 * width + x] = 0xFF58a6ff;
        buffer[y2 * width + x] = 0xFF58a6ff;
    }
    for y in y1..=y2 {
        buffer[y * width + x1] = 0xFF58a6ff;
        buffer[y * width + x2] = 0xFF58a6ff;
    }

    // Draw color chips
    for (i, rect) in chips {
        let cx = (rect.x + rect.width / 2.0) as isize;
        let cy = (rect.y + rect.height / 2.0) as isize;
        let color = PALETTE_COLORS[i].1;

        let r: isize = if hovered_idx == Some(i) {
            8
        } else {
            7
        };
        draw_disk(buffer, width, height, cx, cy, r, color);

        // Highlight ring around active selection
        if i == active_idx {
            draw_circle_ring(buffer, width, height, cx, cy, 10, 0xFFFFFFFF);
        }
    }
}

/// Draw an unfilled circle ring
fn draw_circle_ring(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    cx: isize,
    cy: isize,
    radius: isize,
    color: u32,
) {
    let r_min_sq = (radius - 1) * (radius - 1);
    let r_max_sq = radius * radius;

    for dy in -radius..=radius {
        let py = cy + dy;
        if py < 0 || py >= height as isize {
            continue;
        }
        let row_offset = (py as usize) * width;
        for dx in -radius..=radius {
            let dist_sq = dx * dx + dy * dy;
            if dist_sq >= r_min_sq && dist_sq <= r_max_sq {
                let px = cx + dx;
                if px >= 0 && px < width as isize {
                    buffer[row_offset + px as usize] = color;
                }
            }
        }
    }
}

/// Calculate bounds for floating volume slider popup
pub fn get_volume_slider_rects(
    screen_w: usize,
    screen_h: usize,
    is_fullscreen: bool,
) -> (Rect, Rect) {
    let (_, dock_items) = get_dock_rects(screen_w, screen_h, is_fullscreen);
    let vol_btn = dock_items
        .iter()
        .find(|(a, _, _)| *a == DockAction::ToggleMute)
        .map(|(_, r, _)| *r)
        .unwrap_or(Rect::new(
            screen_w as f32 / 2.0,
            screen_h as f32 - 48.0,
            36.0,
            26.0,
        ));

    let popup_w = 34.0f32;
    let popup_h = 132.0f32;
    let popup_x = vol_btn.x + (vol_btn.width - popup_w) / 2.0;
    let popup_y = vol_btn.y - popup_h - 4.0;

    let popup_rect = Rect::new(popup_x, popup_y, popup_w, popup_h);

    let track_w = 8.0f32;
    let track_h = 88.0f32;
    let track_x = popup_x + (popup_w - track_w) / 2.0;
    let track_y = popup_y + 24.0;
    let track_rect = Rect::new(track_x, track_y, track_w, track_h);

    (popup_rect, track_rect)
}

/// Bounding hover/interaction zone seamlessly bridging the volume dock button and popup slider
pub fn get_volume_slider_hover_rect(
    screen_w: usize,
    screen_h: usize,
    is_fullscreen: bool,
) -> Rect {
    let (popup_rect, _) = get_volume_slider_rects(screen_w, screen_h, is_fullscreen);
    let (_, dock_items) = get_dock_rects(screen_w, screen_h, is_fullscreen);
    let vol_btn = dock_items
        .iter()
        .find(|(a, _, _)| *a == DockAction::ToggleMute)
        .map(|(_, r, _)| *r)
        .unwrap_or(Rect::new(
            popup_rect.x,
            popup_rect.y + popup_rect.height,
            popup_rect.width,
            30.0,
        ));

    let left = popup_rect.x.min(vol_btn.x) - 10.0;
    let right = (popup_rect.x + popup_rect.width).max(vol_btn.x + vol_btn.width) + 10.0;
    let top = popup_rect.y - 10.0;
    let bottom = vol_btn.y + vol_btn.height + 6.0;

    Rect::new(
        left,
        top,
        (right - left).max(20.0),
        (bottom - top).max(20.0),
    )
}

/// Check if mouse interacted with vertical volume slider track
pub fn hit_test_volume_slider(
    screen_w: usize,
    screen_h: usize,
    is_fullscreen: bool,
    mx: f32,
    my: f32,
) -> Option<f32> {
    let (popup_rect, track_rect) = get_volume_slider_rects(screen_w, screen_h, is_fullscreen);
    let test_x_min = popup_rect.x - 10.0;
    let test_x_max = popup_rect.x + popup_rect.width + 10.0;
    let test_y_min = popup_rect.y - 6.0;
    let test_y_max = popup_rect.y + popup_rect.height + 6.0;

    if mx < test_x_min || mx > test_x_max || my < test_y_min || my > test_y_max {
        return None;
    }
    // Fraction: bottom is 0.0, top is 1.0
    let norm = 1.0 - ((my - track_rect.y) / track_rect.height).clamp(0.0, 1.0);
    Some(norm)
}

/// Render the interactive volume slider popup
pub fn render_volume_slider_popup(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    is_fullscreen: bool,
    volume: f32,
    is_muted: bool,
) {
    let (popup_rect, track_rect) = get_volume_slider_rects(width, height, is_fullscreen);

    let x1 = popup_rect.x as usize;
    let y1 = popup_rect.y as usize;
    let x2 = ((popup_rect.x + popup_rect.width) as usize).min(width.saturating_sub(1));
    let y2 = ((popup_rect.y + popup_rect.height) as usize).min(height.saturating_sub(1));

    // Background
    for y in y1..=y2 {
        let row = y.saturating_mul(width);
        for x in x1..=x2 {
            buffer[row.saturating_add(x)] = 0xF0161b22;
        }
    }
    // Border
    for x in x1..=x2 {
        buffer[y1.saturating_mul(width).saturating_add(x)] = 0xFF58a6ff;
        buffer[y2.saturating_mul(width).saturating_add(x)] = 0xFF58a6ff;
    }
    for y in y1..=y2 {
        buffer[y.saturating_mul(width).saturating_add(x1)] = 0xFF58a6ff;
        buffer[y.saturating_mul(width).saturating_add(x2)] = 0xFF58a6ff;
    }

    // Top indicator text: "MUT" or "85%"
    let pct_str = if is_muted {
        "MUT".to_string()
    } else {
        format!("{}%", (volume.clamp(0.0, 1.0) * 100.0).round() as u32)
    };
    let text_col = if is_muted {
        0xFFf85149
    } else {
        0xFF58a6ff
    };
    let text_x = (popup_rect.x + (popup_rect.width - pct_str.len() as f32 * 7.0) * 0.5) as usize;
    draw_text(
        buffer,
        width,
        height,
        text_x,
        y1.saturating_add(7),
        &pct_str,
        text_col,
    );

    // Draw track
    let tx1 = track_rect.x as usize;
    let ty1 = track_rect.y as usize;
    let tx2 = ((track_rect.x + track_rect.width) as usize).min(width.saturating_sub(1));
    let ty2 = ((track_rect.y + track_rect.height) as usize).min(height.saturating_sub(1));

    let fill_h = if is_muted {
        0
    } else {
        (track_rect.height * volume.clamp(0.0, 1.0)).round() as usize
    };
    let fill_start_y = ty2.saturating_sub(fill_h);

    for y in ty1..=ty2 {
        let row = y.saturating_mul(width);
        for x in tx1..=tx2 {
            if y >= fill_start_y && !is_muted {
                buffer[row.saturating_add(x)] = 0xFF39d353; // Green fill
            } else {
                buffer[row.saturating_add(x)] = 0xFF30363d; // Inactive track
            }
        }
    }

    // Draw thumb knob
    let thumb_y = if is_muted {
        ty2
    } else {
        fill_start_y
    };
    let thumb_x = (track_rect.x + track_rect.width / 2.0) as isize;
    draw_disk(
        buffer,
        width,
        height,
        thumb_x,
        thumb_y as isize,
        6,
        0xFFFFFFFF,
    );
}

/// Draw volume toast HUD overlay when volume changes
pub fn draw_volume_toast(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    volume_pct: u32,
    is_muted: bool,
    alpha: f32,
) {
    if alpha <= 0.01 {
        return;
    }

    let card_w = 180usize;
    let card_h = 42usize;
    let card_x = width.saturating_sub(card_w + 20);
    let card_y = 20usize;

    let x2 = (card_x + card_w).min(width.saturating_sub(1));
    let y2 = (card_y + card_h).min(height.saturating_sub(1));

    // Draw dark card background with alpha
    for y in card_y..=y2 {
        let row = y * width;
        for x in card_x..=x2 {
            let bg = buffer[row + x];
            let r = (((bg >> 16) & 0xFF) as f32 * (1.0 - alpha) + 0x16 as f32 * alpha) as u32;
            let g = (((bg >> 8) & 0xFF) as f32 * (1.0 - alpha) + 0x1b as f32 * alpha) as u32;
            let b = ((bg & 0xFF) as f32 * (1.0 - alpha) + 0x22 as f32 * alpha) as u32;
            buffer[row + x] = (0xFF << 24) | (r << 16) | (g << 8) | b;
        }
    }

    // Border
    let border_col = if is_muted {
        0xFFf85149
    } else {
        0xFF58a6ff
    };
    for x in card_x..=x2 {
        buffer[card_y * width + x] = border_col;
        buffer[y2 * width + x] = border_col;
    }
    for y in card_y..=y2 {
        buffer[y * width + card_x] = border_col;
        buffer[y * width + x2] = border_col;
    }

    // Text & meter
    let text = if is_muted {
        "VOLUME: MUTED".to_string()
    } else {
        format!("VOLUME: {}%", volume_pct)
    };
    draw_text(
        buffer,
        width,
        height,
        card_x + 12,
        card_y + 8,
        &text,
        0xFFFFFFFF,
    );

    // Progress bar inside toast
    let bar_x1 = card_x + 12;
    let bar_y1 = card_y + 24;
    let bar_w = card_w - 24;
    let bar_h = 8;
    let fill_w = if is_muted {
        0
    } else {
        (bar_w as f32 * (volume_pct as f32 / 100.0)).round() as usize
    };

    for dy in 0..bar_h {
        let y = bar_y1 + dy;
        if y >= height {
            break;
        }
        let row = y * width;
        for dx in 0..bar_w {
            let x = bar_x1 + dx;
            if x >= width {
                break;
            }
            if dx <= fill_w && !is_muted {
                buffer[row + x] = 0xFF39d353;
            } else {
                buffer[row + x] = 0xFF30363d;
            }
        }
    }
}

/// Simple pixel-based badge for slide index and substep counter
pub fn draw_page_badge(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    current_slide: usize,
    total_slides: usize,
    current_step: usize,
    total_steps: usize,
) {
    let badge_str = if total_steps > 0 {
        format!(
            "{} / {}  [STEP {}/{}]",
            current_slide, total_slides, current_step, total_steps
        )
    } else {
        format!("{} / {}", current_slide, total_slides)
    };

    let char_w = 6;
    let char_h = 8;
    let padding = 8;

    let text_w = badge_str.chars().count() * (char_w + 1);
    let box_w = text_w + padding * 2;
    let box_h = char_h + padding * 2;

    let start_x = width.saturating_sub(box_w + 16);
    let start_y = height.saturating_sub(box_h + 16);

    // Draw semi-transparent background box
    for y in start_y..(start_y + box_h).min(height) {
        let row_start = y * width;
        for x in start_x..(start_x + box_w).min(width) {
            let bg = buffer[row_start + x];
            let r = (((bg >> 16) & 0xFF) * 20 + 0x16 * 80) / 100;
            let g = (((bg >> 8) & 0xFF) * 20 + 0x1b * 80) / 100;
            let b = ((bg & 0xFF) * 20 + 0x22 * 80) / 100;
            buffer[row_start + x] = (0xFF << 24) | (r << 16) | (g << 8) | b;
        }
    }

    let mut cur_x = start_x + padding;
    let cur_y = start_y + padding;

    for ch in badge_str.chars() {
        draw_char(buffer, width, height, cur_x, cur_y, ch, 0xFF58a6ff);
        cur_x += char_w + 1;
    }
}

/// Draw a presenter shortcut help overlay in the center of the screen
pub fn draw_help_overlay(
    buffer: &mut [u32],
    width: usize,
    height: usize,
) {
    let lines = [
        "CARGO SLIDE PRESENTER SHORTCUTS",
        "-----------------------------------------",
        "Space / Right / Left-Click: Next step / slide",
        "Backspace / Left / Right-Click: Prev step / slide",
        "F11 / F              : Toggle Fullscreen / Windowed",
        "1 .. 9               : Jump to slide",
        "Home / End           : First / Last slide",
        "L                    : Toggle Laser pointer (with trail)",
        "P                    : Toggle Whiteboard Pen",
        "K / Dock [COL]       : Open Color Palette (1..7 keys)",
        "C / X                : Clear ink strokes",
        "Mouse Wheel          : Adjust Volume (+/- 5%)",
        "+ / = / Up           : Volume +5%",
        "- / _ / Down         : Volume -5%",
        "M                    : Mute / Unmute audio",
        "Bottom Dock          : Touch / Click presentation controls",
        "R                    : Live reload slides",
        "H / ?                : Toggle this help",
        "Esc / Q              : Exit presentation",
    ];

    let char_w = 6;
    let line_height = 14;
    let padding = 18;

    let max_line_len = lines.iter().map(|l| l.len()).max().unwrap_or(20);
    let box_w = max_line_len * (char_w + 1) + padding * 2;
    let box_h = lines.len() * line_height + padding * 2;

    let start_x = (width.saturating_sub(box_w)) / 2;
    let start_y = (height.saturating_sub(box_h)) / 2;

    // Dark modal background
    for y in start_y..(start_y + box_h).min(height) {
        let row_start = y * width;
        for x in start_x..(start_x + box_w).min(width) {
            let bg = buffer[row_start + x];
            let r = (((bg >> 16) & 0xFF) * 8 + 0x10 * 92) / 100;
            let g = (((bg >> 8) & 0xFF) * 8 + 0x14 * 92) / 100;
            let b = ((bg & 0xFF) * 8 + 0x1d * 92) / 100;
            buffer[row_start + x] = (0xFF << 24) | (r << 16) | (g << 8) | b;
        }
    }

    // Modal border
    let end_x = (start_x + box_w).min(width.saturating_sub(1));
    let end_y = (start_y + box_h).min(height.saturating_sub(1));
    let border_color = 0xFF58a6ff;
    for x in start_x..=end_x {
        buffer[start_y * width + x] = border_color;
        buffer[end_y * width + x] = border_color;
    }
    for y in start_y..=end_y {
        buffer[y * width + start_x] = border_color;
        buffer[y * width + end_x] = border_color;
    }

    // Draw text lines
    for (i, line) in lines.iter().enumerate() {
        let y = start_y + padding + i * line_height;
        let color = if i == 0 {
            0xFF58a6ff // Accent title
        } else if i == 1 {
            0xFF8b949e // Divider
        } else {
            0xFFe6edf3 // Body text
        };

        draw_text(buffer, width, height, start_x + padding, y, line, color);
    }
}

/// Truncate text to fit within `max_px` pixels, appending "..." if truncated.
#[must_use]
pub fn truncate_text_to_width(
    text: &str,
    max_px: usize,
) -> String {
    let char_step = 7usize;
    let max_chars = max_px.saturating_div(char_step);
    if max_chars == 0 {
        return String::new();
    }
    let total_chars = text.chars().count();
    if total_chars <= max_chars {
        return text.to_string();
    }
    if max_chars <= 3 {
        return text.chars().take(max_chars).collect();
    }
    let take_count = max_chars.saturating_sub(3);
    let mut truncated: String = text.chars().take(take_count).collect();
    truncated.push_str("...");
    truncated
}

/// Draw a line of text at (x, y) with right boundary clipping (`max_x`)
pub fn draw_text_clipped(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    mut x: usize,
    y: usize,
    text: &str,
    color: u32,
    max_x: usize,
) {
    let char_w = 6usize;
    for ch in text.chars() {
        if x.saturating_add(char_w) >= max_x || x.saturating_add(char_w) >= width {
            break;
        }
        draw_char(buffer, width, height, x, y, ch, color);
        x = x.saturating_add(char_w).saturating_add(1);
    }
}

/// Draw a line of text at (x, y)
pub fn draw_text(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    mut x: usize,
    y: usize,
    text: &str,
    color: u32,
) {
    let char_w = 6;
    for ch in text.chars() {
        draw_char(buffer, width, height, x, y, ch, color);
        x += char_w + 1;
    }
}

/// Draw text centered within a given bounding box
pub fn draw_text_centered(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    rect: Rect,
    text: &str,
    color: u32,
) {
    let char_w = 6;
    let char_h = 7;
    let text_w = text.chars().count().saturating_mul(char_w + 1);
    let start_x = rect.x as usize + (rect.width as usize).saturating_sub(text_w) / 2;
    let start_y = rect.y as usize + (rect.height as usize).saturating_sub(char_h) / 2;
    draw_text(buffer, width, height, start_x, start_y, text, color);
}

/// Draw text centered around a coordinate point (cx, cy)
pub fn draw_text_at_center(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    cx: usize,
    cy: usize,
    text: &str,
    color: u32,
) {
    let char_w = 6;
    let char_h = 7;
    let text_w = text.chars().count().saturating_mul(char_w + 1);
    let start_x = cx.saturating_sub(text_w / 2);
    let start_y = cy.saturating_sub(char_h / 2);
    draw_text(buffer, width, height, start_x, start_y, text, color);
}

/// Minimal 5x7 bitmap font for HUD numerals & basic symbols
fn draw_char(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    ch: char,
    color: u32,
) {
    let upper = ch.to_ascii_uppercase();
    let bitmap: [u8; 7] = match upper {
        | '0' => {
            [
                0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
            ]
        },
        | '1' => {
            [
                0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
            ]
        },
        | '2' => {
            [
                0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
            ]
        },
        | '3' => {
            [
                0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
            ]
        },
        | '4' => {
            [
                0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
            ]
        },
        | '5' => {
            [
                0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
            ]
        },
        | '6' => {
            [
                0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
            ]
        },
        | '7' => {
            [
                0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
            ]
        },
        | '8' => {
            [
                0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
            ]
        },
        | '9' => {
            [
                0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
            ]
        },
        | 'A' => {
            [
                0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
            ]
        },
        | 'B' => {
            [
                0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
            ]
        },
        | 'C' => {
            [
                0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111,
            ]
        },
        | 'D' => {
            [
                0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
            ]
        },
        | 'E' => {
            [
                0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
            ]
        },
        | 'F' => {
            [
                0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
            ]
        },
        | 'G' => {
            [
                0b01111, 0b10000, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111,
            ]
        },
        | 'H' => {
            [
                0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
            ]
        },
        | 'I' => {
            [
                0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
            ]
        },
        | 'J' => {
            [
                0b00111, 0b00010, 0b00010, 0b00010, 0b10010, 0b10010, 0b01100,
            ]
        },
        | 'K' => {
            [
                0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
            ]
        },
        | 'L' => {
            [
                0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
            ]
        },
        | 'M' => {
            [
                0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
            ]
        },
        | 'N' => {
            [
                0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
            ]
        },
        | 'O' => {
            [
                0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
            ]
        },
        | 'P' => {
            [
                0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
            ]
        },
        | 'Q' => {
            [
                0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10001, 0b01101,
            ]
        },
        | 'R' => {
            [
                0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
            ]
        },
        | 'S' => {
            [
                0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
            ]
        },
        | 'T' => {
            [
                0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
            ]
        },
        | 'U' => {
            [
                0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
            ]
        },
        | 'V' => {
            [
                0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
            ]
        },
        | 'W' => {
            [
                0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001,
            ]
        },
        | 'X' => {
            [
                0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
            ]
        },
        | 'Y' => {
            [
                0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
            ]
        },
        | 'Z' => {
            [
                0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
            ]
        },
        | '<' => {
            [
                0b00010, 0b00100, 0b01000, 0b10000, 0b01000, 0b00100, 0b00010,
            ]
        },
        | '>' => {
            [
                0b01000, 0b00100, 0b00010, 0b00001, 0b00010, 0b00100, 0b01000,
            ]
        },
        | '[' => {
            [
                0b11110, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11110,
            ]
        },
        | ']' => {
            [
                0b01111, 0b00001, 0b00001, 0b00001, 0b00001, 0b00001, 0b01111,
            ]
        },
        | '%' => {
            [
                0b11001, 0b11010, 0b00100, 0b01000, 0b01011, 0b10011, 0b00000,
            ]
        },
        | '/' => {
            [
                0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b00000, 0b00000,
            ]
        },
        | '\\' => {
            [
                0b10000, 0b01000, 0b00100, 0b00010, 0b00001, 0b00000, 0b00000,
            ]
        },
        | '-' => {
            [
                0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000,
            ]
        },
        | '+' => {
            [
                0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000,
            ]
        },
        | '=' => {
            [
                0b00000, 0b11111, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000,
            ]
        },
        | '.' => {
            [
                0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100,
            ]
        },
        | ',' => {
            [
                0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b00100,
            ]
        },
        | ':' => {
            [
                0b00000, 0b01100, 0b01100, 0b00000, 0b01100, 0b01100, 0b00000,
            ]
        },
        | '?' => {
            [
                0b01110, 0b10001, 0b00010, 0b00100, 0b00100, 0b00000, 0b00100,
            ]
        },
        | '(' => {
            [
                0b00100, 0b01000, 0b10000, 0b10000, 0b10000, 0b01000, 0b00100,
            ]
        },
        | ')' => {
            [
                0b00100, 0b00010, 0b00001, 0b00001, 0b00001, 0b00010, 0b00100,
            ]
        },
        | '$' => {
            [
                0b00100, 0b01111, 0b10100, 0b01110, 0b00101, 0b11110, 0b00100,
            ]
        },
        | '#' => {
            [
                0b01010, 0b01010, 0b11111, 0b01010, 0b11111, 0b01010, 0b01010,
            ]
        },
        | '|' => {
            [
                0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
            ]
        },
        | '*' => {
            [
                0b00000, 0b10101, 0b01110, 0b11111, 0b01110, 0b10101, 0b00000,
            ]
        },
        | '^' => {
            [
                0b00100, 0b01010, 0b10001, 0b00000, 0b00000, 0b00000, 0b00000,
            ]
        },
        | '!' => {
            [
                0b00100, 0b00100, 0b00100, 0b00100, 0b00000, 0b00100, 0b00000,
            ]
        },
        | ' ' => [0; 7],
        | '▼' | '▽' => {
            [
                0b00000, 0b11111, 0b11111, 0b01110, 0b01110, 0b00100, 0b00000,
            ]
        },
        | '▲' | '△' => {
            [
                0b00000, 0b00100, 0b01110, 0b01110, 0b11111, 0b11111, 0b00000,
            ]
        },
        | '▶' | '▷' => {
            [
                0b00000, 0b11000, 0b11110, 0b11111, 0b11110, 0b11000, 0b00000,
            ]
        },
        | '◀' | '◁' => {
            [
                0b00000, 0b00011, 0b01111, 0b11111, 0b01111, 0b00011, 0b00000,
            ]
        },
        | '↑' => {
            [
                0b00100, 0b01110, 0b10101, 0b00100, 0b00100, 0b00100, 0b00000,
            ]
        },
        | '↓' => {
            [
                0b00000, 0b00100, 0b00100, 0b00100, 0b10101, 0b01110, 0b00100,
            ]
        },
        | '←' => {
            [
                0b00000, 0b00100, 0b01000, 0b11111, 0b01000, 0b00100, 0b00000,
            ]
        },
        | '→' => {
            [
                0b00000, 0b00100, 0b00010, 0b11111, 0b00010, 0b00100, 0b00000,
            ]
        },
        | '•' | '·' => {
            [
                0b00000, 0b00000, 0b01110, 0b01110, 0b01110, 0b00000, 0b00000,
            ]
        },
        | '✓' | '✔' => {
            [
                0b00000, 0b00001, 0b00011, 0b10010, 0b01100, 0b00100, 0b00000,
            ]
        },
        | '✗' | '✕' | '✖' => {
            [
                0b10001, 0b11011, 0b01110, 0b00100, 0b01110, 0b11011, 0b10001,
            ]
        },
        | '_' => {
            [
                0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b11111,
            ]
        },
        | ';' => {
            [
                0b00000, 0b01100, 0b01100, 0b00000, 0b01100, 0b00100, 0b01000,
            ]
        },
        | '~' => {
            [
                0b00000, 0b01001, 0b10110, 0b00000, 0b00000, 0b00000, 0b00000,
            ]
        },
        | '&' => {
            [
                0b01100, 0b10010, 0b01100, 0b10010, 0b10101, 0b10010, 0b01101,
            ]
        },
        | '@' => {
            [
                0b01110, 0b10001, 0b10111, 0b10101, 0b10110, 0b10000, 0b01111,
            ]
        },
        | '"' | '“' | '”' => {
            [
                0b10100, 0b10100, 0b10100, 0b00000, 0b00000, 0b00000, 0b00000,
            ]
        },
        | '\'' | '`' | '‘' | '’' => {
            [
                0b00100, 0b00100, 0b01000, 0b00000, 0b00000, 0b00000, 0b00000,
            ]
        },
        | '{' => {
            [
                0b00010, 0b00100, 0b00100, 0b01000, 0b00100, 0b00100, 0b00010,
            ]
        },
        | '}' => {
            [
                0b01000, 0b00100, 0b00100, 0b00010, 0b00100, 0b00100, 0b01000,
            ]
        },
        | '€' => {
            [
                0b01110, 0b10001, 0b11100, 0b10000, 0b11100, 0b10001, 0b01110,
            ]
        },
        | '£' => {
            [
                0b00110, 0b01001, 0b01000, 0b11100, 0b01000, 0b01001, 0b11111,
            ]
        },
        | '¥' => {
            [
                0b10001, 0b01010, 0b00100, 0b11111, 0b00100, 0b11111, 0b00100,
            ]
        },
        | '°' => {
            [
                0b01100, 0b10010, 0b01100, 0b00000, 0b00000, 0b00000, 0b00000,
            ]
        },
        | '📊' => {
            [
                0b00001, 0b00001, 0b00101, 0b00101, 0b10101, 0b10101, 0b11111,
            ]
        },
        | '📈' => {
            [
                0b00011, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
            ]
        },
        | '🔍' | '🔎' => {
            [
                0b01110, 0b10001, 0b10001, 0b01110, 0b00010, 0b00100, 0b01000,
            ]
        },
        | '🚀' => {
            [
                0b00100, 0b01110, 0b01110, 0b11111, 0b10101, 0b01010, 0b00100,
            ]
        },
        | '⚡' => {
            [
                0b00010, 0b00110, 0b01110, 0b11111, 0b00111, 0b00110, 0b00100,
            ]
        },
        | '✨' => {
            [
                0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000, 0b00000,
            ]
        },
        | '🗄' | '💾' => {
            [
                0b11111, 0b10001, 0b11111, 0b10001, 0b11111, 0b10001, 0b11111,
            ]
        },
        | '📦' => {
            [
                0b01110, 0b10101, 0b11111, 0b10101, 0b10101, 0b10001, 0b11111,
            ]
        },
        | '🎨' => {
            [
                0b01110, 0b10101, 0b11011, 0b10101, 0b10001, 0b01110, 0b00000,
            ]
        },
        | '📄' => {
            [
                0b11100, 0b10010, 0b11110, 0b10010, 0b11110, 0b10000, 0b11111,
            ]
        },
        | '💡' => {
            [
                0b01110, 0b10001, 0b10001, 0b01110, 0b01110, 0b00100, 0b01110,
            ]
        },
        | '⏮' => {
            [
                0b10110, 0b10111, 0b10111, 0b10101, 0b10111, 0b10111, 0b10110,
            ]
        },
        | '⏭' => {
            [
                0b01101, 0b11101, 0b11101, 0b10101, 0b11101, 0b11101, 0b01101,
            ]
        },
        | '🎬' => {
            [
                0b10101, 0b01010, 0b11111, 0b10001, 0b10001, 0b10001, 0b11111,
            ]
        },
        | '🎉' => {
            [
                0b00001, 0b00011, 0b00111, 0b01111, 0b11110, 0b11100, 0b11000,
            ]
        },
        | _ => {
            if ch.is_whitespace() || ch.is_control() {
                [0; 7]
            } else {
                [
                    0b00000, 0b00000, 0b00000, 0b00100, 0b00000, 0b00000, 0b00000,
                ]
            }
        },
    };

    for (row, bits) in bitmap.iter().enumerate() {
        let py = y.saturating_add(row);
        if py >= height {
            break;
        }
        let row_start = py.saturating_mul(width);
        for col in 0..5 {
            let px = x.saturating_add(col);
            if px >= width {
                break;
            }
            let shift = 4usize.saturating_sub(col);
            if (bits & (1 << shift)) != 0
                && let Some(pixel) = buffer.get_mut(row_start.saturating_add(px))
            {
                *pixel = color;
            }
        }
    }
}

/// Quick on-slide action for charts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartQuickAction {
    CycleType,
    OpenInspector,
}

/// Action produced by clicking within the Chart Data Inspector modal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorAction {
    Close,
    SetType(ChartType),
    SetTransform(ChartTransform),
    ToggleSeries(usize),
    ExportCsv,
    SelectCategory(usize),
    ScrollTable(isize),
    FocusSearch,
    ClearFilter,
    CycleFormat,
    SortColumn(usize), // 0: category, 1+: series
}

pub const TRANSFORM_PRESETS: [(&str, ChartTransform); 7] = [
    ("ORIG", ChartTransform::None),
    ("TOP 5", ChartTransform::TopK(5)),
    ("SORT ▼", ChartTransform::SortDesc),
    ("SORT ▲", ChartTransform::SortAsc),
    ("CUM", ChartTransform::Cumulative),
    ("% SHARE", ChartTransform::Percent100),
    ("MA3", ChartTransform::MovingAvg(3)),
];

pub fn get_transform_preset_rects(series_strip: Rect) -> Vec<(Rect, &'static str, ChartTransform)> {
    let mut result = Vec::new();
    let mut right_x = series_strip.x + series_strip.width;
    let gap = 5.0;
    let chip_h = 22.0;

    for &(label, t) in TRANSFORM_PRESETS.iter().rev() {
        let chip_w = (label.chars().count() * 6 + 16) as f32;
        right_x -= chip_w;
        let r = Rect::new(right_x, series_strip.y + 2.0, chip_w, chip_h);
        result.push((r, label, t));
        right_x -= gap;
    }
    result.reverse();
    result
}

fn parse_filter_condition(q: &str) -> Option<(&'static str, &str)> {
    let trimmed = q.trim();
    if let Some(rest) = trimmed.strip_prefix(">=") {
        Some((">=", rest.trim()))
    } else if let Some(rest) = trimmed.strip_prefix("<=") {
        Some(("<=", rest.trim()))
    } else if let Some(rest) = trimmed.strip_prefix("!=") {
        Some(("!=", rest.trim()))
    } else if let Some(rest) = trimmed.strip_prefix("==") {
        Some(("==", rest.trim()))
    } else if let Some(rest) = trimmed.strip_prefix('>') {
        Some((">", rest.trim()))
    } else if let Some(rest) = trimmed.strip_prefix('<') {
        Some(("<", rest.trim()))
    } else if let Some(rest) = trimmed.strip_prefix('=') {
        Some(("=", rest.trim()))
    } else {
        None
    }
}

/// State of the active Chart Data Inspector modal
#[derive(Debug, Clone)]
pub struct ChartInspectorState {
    pub original_chart_data: ChartData,
    pub chart_data: ChartData,
    pub active_type: ChartType,
    pub active_transform: ChartTransform,
    pub hidden_series: std::collections::HashSet<usize>,
    pub hovered_category: Option<usize>,
    pub hovered_series: Option<usize>,
    pub table_scroll: usize,
    pub toast_message: Option<(String, std::time::Instant)>,

    pub search_query: String,
    pub search_active: bool,
    pub sort_column: Option<usize>,
    pub sort_ascending: bool,
    pub marquee_range: Option<(usize, usize)>,
    pub marquee_drag_start: Option<usize>,

    pub cached_filtered_indices: Vec<usize>,
    pub cached_stats: slide_core::chart::ChartStats,
}

impl ChartInspectorState {
    pub fn new(
        chart_data: ChartData,
        initial_type: ChartType,
    ) -> Self {
        let empty_set = std::collections::HashSet::new();
        let cached_filtered_indices: Vec<usize> = (0..chart_data.categories.len()).collect();
        let cached_stats = chart_data.summary_stats_filtered(&empty_set, &cached_filtered_indices);
        Self {
            original_chart_data: chart_data.clone(),
            chart_data,
            active_type: initial_type,
            active_transform: ChartTransform::None,
            hidden_series: empty_set,
            hovered_category: None,
            hovered_series: None,
            table_scroll: 0,
            toast_message: None,
            search_query: String::new(),
            search_active: false,
            sort_column: None,
            sort_ascending: true,
            marquee_range: None,
            marquee_drag_start: None,
            cached_filtered_indices,
            cached_stats,
        }
    }

    pub fn clear_filter(&mut self) {
        self.search_query.clear();
        self.search_active = false;
        self.marquee_range = None;
        self.marquee_drag_start = None;
        self.table_scroll = 0;
        self.recompute_cache();
        self.show_toast("Filter Cleared");
    }

    pub fn cycle_format(&mut self) {
        use slide_core::chart::NumberFormat;
        self.chart_data.format = match self.chart_data.format {
            | NumberFormat::Auto => NumberFormat::Currency,
            | NumberFormat::Currency => NumberFormat::Percentage,
            | NumberFormat::Percentage => NumberFormat::Compact,
            | NumberFormat::Compact => NumberFormat::Scientific,
            | NumberFormat::Scientific => NumberFormat::Integer,
            | NumberFormat::Integer => NumberFormat::Standard,
            | NumberFormat::Standard => NumberFormat::Auto,
        };
        self.original_chart_data.format = self.chart_data.format;
        let name = match self.chart_data.format {
            | NumberFormat::Auto => "Format: Auto",
            | NumberFormat::Currency => "Format: Currency ($)",
            | NumberFormat::Percentage => "Format: Percent (%)",
            | NumberFormat::Compact => "Format: Compact (K/M/B)",
            | NumberFormat::Scientific => "Format: Scientific",
            | NumberFormat::Integer => "Format: Integer",
            | NumberFormat::Standard => "Format: Standard",
        };
        self.show_toast(name);
        self.recompute_cache();
    }

    pub fn compute_filtered_category_indices(&self) -> Vec<usize> {
        let total_cats = self.chart_data.categories.len();
        let mut indices: Vec<usize> = (0..total_cats).collect();

        if let Some((start, end)) = self.marquee_range {
            let min_i = start.min(end);
            let max_i = start.max(end);
            indices.retain(|&i| i >= min_i && i <= max_i);
        }

        let query = self.search_query.trim();
        if !query.is_empty() {
            let query_lower = query.to_lowercase();
            if let Some((op, num_str)) = parse_filter_condition(query) {
                if let Ok(target_num) = num_str.parse::<f64>() {
                    indices.retain(|&cat_i| {
                        self.chart_data.series.iter().enumerate().any(|(s_i, s)| {
                            if self.hidden_series.contains(&s_i) {
                                return false;
                            }
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
                indices.retain(|&cat_i| {
                    if let Some(cat_name) = self.chart_data.categories.get(cat_i)
                        && cat_name.to_lowercase().contains(&query_lower)
                    {
                        return true;
                    }
                    self.chart_data.series.iter().enumerate().any(|(s_i, s)| {
                        if self.hidden_series.contains(&s_i) {
                            return false;
                        }
                        if let Some(&val) = s.values.get(cat_i) {
                            let formatted = self.chart_data.format_number(val).to_lowercase();
                            if formatted.contains(&query_lower) {
                                return true;
                            }
                        }
                        false
                    })
                });
            }
        }

        if let Some(col) = self.sort_column {
            indices.sort_by(|&a, &b| {
                let ord = if col == 0 {
                    let cat_a = self
                        .chart_data
                        .categories
                        .get(a)
                        .map(String::as_str)
                        .unwrap_or("");
                    let cat_b = self
                        .chart_data
                        .categories
                        .get(b)
                        .map(String::as_str)
                        .unwrap_or("");
                    cat_a.cmp(cat_b)
                } else {
                    let s_idx = col.saturating_sub(1);
                    let val_a = self
                        .chart_data
                        .series
                        .get(s_idx)
                        .and_then(|s| s.values.get(a))
                        .copied()
                        .unwrap_or(0.0);
                    let val_b = self
                        .chart_data
                        .series
                        .get(s_idx)
                        .and_then(|s| s.values.get(b))
                        .copied()
                        .unwrap_or(0.0);
                    val_a
                        .partial_cmp(&val_b)
                        .unwrap_or(std::cmp::Ordering::Equal)
                };
                if self.sort_ascending {
                    ord
                } else {
                    ord.reverse()
                }
            });
        }

        indices
    }

    pub fn recompute_cache(&mut self) {
        self.cached_filtered_indices = self.compute_filtered_category_indices();
        self.cached_stats = self
            .chart_data
            .summary_stats_filtered(&self.hidden_series, &self.cached_filtered_indices);
    }

    #[must_use]
    pub fn get_filtered_category_indices(&self) -> &[usize] {
        &self.cached_filtered_indices
    }

    pub fn set_transform(
        &mut self,
        transform: ChartTransform,
    ) {
        if self.active_transform == transform {
            self.active_transform = ChartTransform::None;
            self.chart_data = self.original_chart_data.clone();
            self.show_toast("Reset to Original Data");
        } else {
            self.active_transform = transform;
            self.chart_data = self.original_chart_data.apply_transform(transform);
            let msg = match transform {
                | ChartTransform::None => "Original Data".to_string(),
                | ChartTransform::TopK(k) => format!("Top {} by Total", k),
                | ChartTransform::SortDesc => "Sorted Descending (Max First)".to_string(),
                | ChartTransform::SortAsc => "Sorted Ascending (Min First)".to_string(),
                | ChartTransform::Cumulative => "Cumulative Running Sum".to_string(),
                | ChartTransform::Percent100 => "100% Share Normalized".to_string(),
                | ChartTransform::MovingAvg(w) => format!("Moving Average (MA{})", w),
            };
            self.show_toast(&msg);
        }
        self.recompute_cache();
    }

    pub fn cycle_type(&mut self) {
        self.active_type = match self.active_type {
            | ChartType::Bar => ChartType::Line,
            | ChartType::Line => ChartType::Area,
            | ChartType::Area => ChartType::Pie,
            | ChartType::Pie => ChartType::Donut,
            | ChartType::Donut => ChartType::Bar,
            | ChartType::Scatter => ChartType::Bar,
        };
    }

    pub fn toggle_series(
        &mut self,
        s_idx: usize,
    ) {
        if s_idx < self.chart_data.series.len() {
            if self.hidden_series.contains(&s_idx) {
                self.hidden_series.remove(&s_idx);
            } else {
                // Ensure at least one series remains visible
                if self.hidden_series.len() + 1 < self.chart_data.series.len() {
                    self.hidden_series.insert(s_idx);
                }
            }
            self.recompute_cache();
        }
    }

    pub fn show_toast(
        &mut self,
        msg: &str,
    ) {
        self.toast_message = Some((msg.to_string(), std::time::Instant::now()));
    }
}

/// Get rectangles for on-slide quick action pills `[TYPE]` and `[DATA]`
pub fn get_chart_quick_action_rects(
    metrics: &RenderMetrics,
    chart_rect: Rect,
) -> (Rect, Rect) {
    let (sx, sy) = metrics.svg_to_screen(chart_rect.x, chart_rect.y);
    let sw = chart_rect.width * metrics.scale;
    let btn_w = 48.0;
    let btn_h = 18.0;
    let pad = 6.0;

    let inspect_btn = Rect::new(sx + sw - pad - btn_w, sy + pad, btn_w, btn_h);
    let type_btn = Rect::new(sx + sw - pad - btn_w - 4.0 - btn_w, sy + pad, btn_w, btn_h);
    (type_btn, inspect_btn)
}

/// Hit-test on-slide quick action buttons
pub fn hit_test_chart_quick_actions(
    metrics: &RenderMetrics,
    chart_rect: Rect,
    mouse_x: f32,
    mouse_y: f32,
) -> Option<ChartQuickAction> {
    let (type_btn, inspect_btn) = get_chart_quick_action_rects(metrics, chart_rect);
    if type_btn.contains(mouse_x, mouse_y) {
        Some(ChartQuickAction::CycleType)
    } else if inspect_btn.contains(mouse_x, mouse_y) {
        Some(ChartQuickAction::OpenInspector)
    } else {
        None
    }
}

/// Draw a sleek pill-shaped quick action button
pub fn draw_chart_quick_button(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    rect: Rect,
    label: &str,
    is_hovered: bool,
    accent_color: u32,
) {
    let x1 = (rect.x as usize).min(width.saturating_sub(1));
    let y1 = (rect.y as usize).min(height.saturating_sub(1));
    let x2 = ((rect.x + rect.width) as usize).min(width.saturating_sub(1));
    let y2 = ((rect.y + rect.height) as usize).min(height.saturating_sub(1));

    if x2 <= x1 || y2 <= y1 {
        return;
    }

    let bg_color = if is_hovered {
        0xEE21262d
    } else {
        0xD0161b22
    };
    let border_color = if is_hovered {
        accent_color
    } else {
        0xFF30363d
    };

    for y in y1..=y2 {
        let row = y * width;
        for x in x1..=x2 {
            buffer[row + x] = bg_color;
        }
    }

    // Border
    for x in x1..=x2 {
        buffer[y1 * width + x] = border_color;
        buffer[y2 * width + x] = border_color;
    }
    for y in y1..=y2 {
        buffer[y * width + x1] = border_color;
        buffer[y * width + x2] = border_color;
    }

    // Centered text
    let text_w = label.chars().count() * 6;
    let tx = (rect.x + (rect.width - text_w as f32) * 0.5).max(rect.x + 2.0) as usize;
    let ty = (rect.y + (rect.height - 7.0) * 0.5) as usize;
    draw_text(
        buffer,
        width,
        height,
        tx,
        ty,
        label,
        if is_hovered {
            0xFFFFFFFF
        } else {
            0xFFc9d1d9
        },
    );
}

/// Fast rectangular fill with solid color
pub fn fill_rect(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    color: u32,
) {
    let x2 = (x.saturating_add(w)).min(width);
    let y2 = (y.saturating_add(h)).min(height);
    let start_x = x.min(width);
    if start_x < x2 {
        for py in y.min(height)..y2 {
            let row = py.saturating_mul(width);
            let start = row.saturating_add(start_x);
            let end = row.saturating_add(x2);
            if let Some(slice) = buffer.get_mut(start..end) {
                slice.fill(color);
            }
        }
    }
}

/// Fast rectangular fill with alpha blending
pub fn fill_rect_alpha(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    color: u32,
    alpha_256: u32,
) {
    let x2 = (x.saturating_add(w)).min(width);
    let y2 = (y.saturating_add(h)).min(height);
    let start_x = x.min(width);
    if start_x < x2 {
        for py in y.min(height)..y2 {
            let row = py.saturating_mul(width);
            let start = row.saturating_add(start_x);
            let end = row.saturating_add(x2);
            if let Some(slice) = buffer.get_mut(start..end) {
                for pixel in slice {
                    *pixel = blend_pixel_fast(*pixel, color, alpha_256);
                }
            }
        }
    }
}

/// Draw 1px rectangle outline
pub fn draw_rect_outline(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    color: u32,
) {
    if w == 0 || h == 0 || x >= width || y >= height {
        return;
    }
    let x2 = (x + w - 1).min(width.saturating_sub(1));
    let y2 = (y + h - 1).min(height.saturating_sub(1));
    for px in x..=x2 {
        buffer[y * width + px] = color;
        buffer[y2 * width + px] = color;
    }
    for py in y..=y2 {
        buffer[py * width + x] = color;
        buffer[py * width + x2] = color;
    }
}

/// Draw horizontal dashed line
pub fn draw_dashed_hline(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    x1: usize,
    x2: usize,
    y: usize,
    color: u32,
    dash_len: usize,
    gap_len: usize,
) {
    if y >= height {
        return;
    }
    let row = y * width;
    let mut px = x1;
    while px < x2 && px < width {
        let seg_end = (px + dash_len).min(x2).min(width);
        for x in px..seg_end {
            buffer[row + x] = color;
        }
        px += dash_len + gap_len;
    }
}

/// Render interactive hovering indicators and glassmorphic tooltip card over a chart
pub fn render_chart_hover(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    metrics: &RenderMetrics,
    chart_rect: Rect,
    chart_data: &slide_core::chart::ChartData,
    active_type: ChartType,
    mouse_x: f32,
    mouse_y: f32,
) {
    let (type_btn, inspect_btn) = get_chart_quick_action_rects(metrics, chart_rect);
    let hovered_action = hit_test_chart_quick_actions(metrics, chart_rect, mouse_x, mouse_y);

    // Draw top-right quick action pills
    let type_label = match active_type {
        | ChartType::Bar => "BAR",
        | ChartType::Line => "LINE",
        | ChartType::Area => "AREA",
        | ChartType::Pie => "PIE",
        | ChartType::Donut => "DONUT",
        | ChartType::Scatter => "SCAT",
    };
    draw_chart_quick_button(
        buffer,
        width,
        height,
        type_btn,
        type_label,
        hovered_action == Some(ChartQuickAction::CycleType),
        0xFF38bdf8,
    );
    draw_chart_quick_button(
        buffer,
        width,
        height,
        inspect_btn,
        "DATA",
        hovered_action == Some(ChartQuickAction::OpenInspector),
        0xFF34d399,
    );

    // If mouse is hovering a button pill, don't show category tooltip overlapping the buttons
    if hovered_action.is_some() {
        return;
    }

    if let Some((svg_x, svg_y)) = metrics.screen_to_svg(mouse_x, mouse_y) {
        if !chart_rect.contains(svg_x, svg_y) {
            return;
        }

        match active_type {
            | slide_core::chart::ChartType::Bar
            | slide_core::chart::ChartType::Line
            | slide_core::chart::ChartType::Area
            | slide_core::chart::ChartType::Scatter => {
                if let Some(cat_idx) = chart_data.hit_test_category(chart_rect, svg_x, svg_y) {
                    let cat_name = chart_data
                        .categories
                        .get(cat_idx)
                        .cloned()
                        .unwrap_or_else(|| format!("Item {}", cat_idx + 1));
                    let plot = chart_data.plot_area(chart_rect);
                    let cat_count = chart_data.categories.len().max(1);
                    let col_w = plot.width / cat_count as f32;

                    if active_type == slide_core::chart::ChartType::Bar {
                        // Highlight vertical column with fast bit-shift alpha blending
                        let col_svg_x = plot.x + cat_idx as f32 * col_w;
                        let (screen_col_x, screen_plot_y) =
                            metrics.svg_to_screen(col_svg_x, plot.y);
                        let screen_col_w = col_w * metrics.scale;
                        let screen_plot_h = plot.height * metrics.scale;

                        let x1 = (screen_col_x.max(0.0) as usize).min(width);
                        let y1 = (screen_plot_y.max(0.0) as usize).min(height);
                        let x2 = ((screen_col_x + screen_col_w).max(0.0) as usize).min(width);
                        let y2 = ((screen_plot_y + screen_plot_h).max(0.0) as usize).min(height);

                        for py in y1..y2 {
                            let row = py * width;
                            for px in x1..x2 {
                                buffer[row + px] =
                                    blend_pixel_fast(buffer[row + px], 0xFF388bfd, 56);
                            }
                        }
                    } else {
                        // Vertical guideline crosshair
                        let center_svg_x = plot.x + (cat_idx as f32 + 0.5) * col_w;
                        let (guide_x, guide_top_y) = metrics.svg_to_screen(center_svg_x, plot.y);
                        let (_, guide_bottom_y) =
                            metrics.svg_to_screen(center_svg_x, plot.y + plot.height);
                        let gx = guide_x.round() as usize;
                        if gx < width {
                            let gy1 = (guide_top_y.max(0.0) as usize).min(height);
                            let gy2 = (guide_bottom_y.max(0.0) as usize).min(height);
                            for gy in gy1..gy2 {
                                buffer[gy * width + gx] = 0xAA58a6ff;
                                if gx + 1 < width {
                                    buffer[gy * width + gx + 1] = 0x4458a6ff;
                                }
                            }
                        }

                        // Glowing dots at points on the series
                        let (y_min, y_max, _) = chart_data.nice_scale(5);
                        for s in &chart_data.series {
                            if let Some(&val) = s.values.get(cat_idx) {
                                let pt_svg_y = plot.y + plot.height
                                    - ((val - y_min) / (y_max - y_min).max(1e-6)) as f32
                                        * plot.height;
                                let (screen_pt_x, screen_pt_y) =
                                    metrics.svg_to_screen(center_svg_x, pt_svg_y);
                                draw_glow_marker(
                                    buffer,
                                    width,
                                    height,
                                    screen_pt_x as usize,
                                    screen_pt_y as usize,
                                    0xFF58a6ff,
                                );
                            }
                        }
                    }

                    // Render floating tooltip HUD card
                    render_tooltip_card(
                        buffer,
                        width,
                        height,
                        mouse_x as usize,
                        mouse_y as usize,
                        &cat_name,
                        &chart_data.series,
                        cat_idx,
                    );
                }
            },
            | slide_core::chart::ChartType::Pie | slide_core::chart::ChartType::Donut => {
                if let Some(slice_idx) = chart_data.hit_test_pie_slice(chart_rect, svg_x, svg_y) {
                    let (slice_name, slice_val, total): (String, f64, f64) =
                        if chart_data.series.len() == 1 {
                            let name = chart_data
                                .categories
                                .get(slice_idx)
                                .cloned()
                                .unwrap_or_else(|| format!("Slice {}", slice_idx + 1));
                            let val = chart_data.series[0]
                                .values
                                .get(slice_idx)
                                .copied()
                                .unwrap_or(0.0);
                            let tot = chart_data.series[0].values.iter().sum();
                            (name, val, tot)
                        } else if slice_idx < chart_data.series.len() {
                            let s = &chart_data.series[slice_idx];
                            let val = s.values.first().copied().unwrap_or(0.0);
                            let tot = chart_data
                                .series
                                .iter()
                                .map(|s| s.values.first().copied().unwrap_or(0.0))
                                .sum();
                            (s.name.clone(), val, tot)
                        } else {
                            return;
                        };

                    let pct = if total > 0.0 {
                        (slice_val / total) * 100.0
                    } else {
                        0.0
                    };
                    let lines = vec![
                        format!(
                            "VALUE: {}",
                            slide_core::chart::ChartData::format_value(slice_val)
                        ),
                        format!("SHARE: {:.1}%", pct),
                    ];
                    render_simple_tooltip(
                        buffer,
                        width,
                        height,
                        mouse_x as usize,
                        mouse_y as usize,
                        &slice_name,
                        &lines,
                    );
                }
            },
        }
    }
}

fn draw_glow_marker(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    cx: usize,
    cy: usize,
    color: u32,
) {
    for dy in -4..=4 {
        let py = cy as isize + dy;
        if py < 0 || py as usize >= height {
            continue;
        }
        let uy = py as usize;
        for dx in -4..=4 {
            let px = cx as isize + dx;
            if px < 0 || px as usize >= width {
                continue;
            }
            let ux = px as usize;
            let dist_sq = dx * dx + dy * dy;
            if dist_sq <= 4 {
                buffer[uy * width + ux] = 0xFFffffff; // White center
            } else if dist_sq <= 16 {
                buffer[uy * width + ux] = color; // Colored halo
            }
        }
    }
}

fn render_tooltip_card(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    mx: usize,
    my: usize,
    title: &str,
    series: &[slide_core::chart::SeriesData],
    cat_idx: usize,
) {
    let mut max_chars = title.chars().count().saturating_add(2);
    for s in series {
        let val_str = s
            .values
            .get(cat_idx)
            .map(|&v| slide_core::chart::ChartData::format_value(v))
            .unwrap_or_else(|| "0".into());
        let line_chars = s
            .name
            .chars()
            .count()
            .saturating_add(val_str.chars().count())
            .saturating_add(3);
        if line_chars > max_chars {
            max_chars = line_chars;
        }
    }
    let needed_w = max_chars.saturating_mul(7).saturating_add(34);
    let card_w = needed_w.clamp(170, (width.saturating_sub(40)).min(640));
    let card_h = 32usize.saturating_add(series.len().saturating_mul(18));

    let card_x = if mx.saturating_add(16).saturating_add(card_w) < width {
        mx.saturating_add(16)
    } else {
        mx.saturating_sub(card_w.saturating_add(10))
    };
    let card_y = if my.saturating_add(16).saturating_add(card_h) < height {
        my.saturating_add(16)
    } else {
        my.saturating_sub(card_h.saturating_add(10))
    };

    for y in card_y..(card_y.saturating_add(card_h)).min(height) {
        let row = y.saturating_mul(width);
        let is_edge_y = y == card_y || y == card_y.saturating_add(card_h).saturating_sub(1);
        for x in card_x..(card_x.saturating_add(card_w)).min(width) {
            let is_edge_x = x == card_x || x == card_x.saturating_add(card_w).saturating_sub(1);
            if is_edge_y || is_edge_x {
                buffer[row.saturating_add(x)] = 0xFF30363d;
            } else {
                buffer[row.saturating_add(x)] =
                    blend_pixel_fast(buffer[row.saturating_add(x)], 0xFF161b22, 235);
            }
        }
    }

    let title_display = truncate_text_to_width(title, card_w.saturating_sub(20));
    draw_text_clipped(
        buffer,
        width,
        height,
        card_x.saturating_add(10),
        card_y.saturating_add(8),
        &title_display,
        0xFF58a6ff,
        card_x.saturating_add(card_w).saturating_sub(6),
    );

    let div_y = card_y.saturating_add(22);
    if div_y < height {
        let row = div_y.saturating_mul(width);
        for x in
            (card_x.saturating_add(8))..(card_x.saturating_add(card_w).saturating_sub(8)).min(width)
        {
            buffer[row.saturating_add(x)] = 0xFF30363d;
        }
    }

    for (i, s) in series.iter().enumerate() {
        let sy = card_y
            .saturating_add(28)
            .saturating_add(i.saturating_mul(18));
        if sy.saturating_add(10) >= height {
            break;
        }

        let color = if let Some(ref c) = s.color {
            parse_hex_color(c)
        } else {
            0xFF58a6ff
        };

        for by in 0..6 {
            let brow = (sy.saturating_add(by)).saturating_mul(width);
            for bx in 0..6 {
                if card_x.saturating_add(10).saturating_add(bx) < width {
                    buffer[brow
                        .saturating_add(card_x)
                        .saturating_add(10)
                        .saturating_add(bx)] = color;
                }
            }
        }

        let val_str = s
            .values
            .get(cat_idx)
            .map(|&v| slide_core::chart::ChartData::format_value(v))
            .unwrap_or_else(|| "0".into());
        let label = format!("{}: {}", s.name, val_str);
        let label_display = truncate_text_to_width(&label, card_w.saturating_sub(32));
        draw_text_clipped(
            buffer,
            width,
            height,
            card_x.saturating_add(22),
            sy,
            &label_display,
            0xFFe6edf3,
            card_x.saturating_add(card_w).saturating_sub(6),
        );
    }
}

fn render_simple_tooltip(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    mx: usize,
    my: usize,
    title: &str,
    lines: &[String],
) {
    let max_line_chars = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    let title_chars = title.chars().count();
    let max_chars = title_chars.max(max_line_chars);
    let needed_w = max_chars.saturating_mul(7).saturating_add(28);
    let card_w = needed_w.clamp(150, (width.saturating_sub(40)).min(640));
    let card_h = 28usize.saturating_add(lines.len().saturating_mul(16));

    let card_x = if mx.saturating_add(16).saturating_add(card_w) < width {
        mx.saturating_add(16)
    } else {
        mx.saturating_sub(card_w.saturating_add(10))
    };
    let card_y = if my.saturating_add(16).saturating_add(card_h) < height {
        my.saturating_add(16)
    } else {
        my.saturating_sub(card_h.saturating_add(10))
    };

    for y in card_y..(card_y.saturating_add(card_h)).min(height) {
        let row = y.saturating_mul(width);
        let is_edge_y = y == card_y || y == card_y.saturating_add(card_h).saturating_sub(1);
        for x in card_x..(card_x.saturating_add(card_w)).min(width) {
            let is_edge_x = x == card_x || x == card_x.saturating_add(card_w).saturating_sub(1);
            if is_edge_y || is_edge_x {
                buffer[row.saturating_add(x)] = 0xFF30363d;
            } else {
                buffer[row.saturating_add(x)] =
                    blend_pixel_fast(buffer[row.saturating_add(x)], 0xFF161b22, 235);
            }
        }
    }

    let title_display = truncate_text_to_width(title, card_w.saturating_sub(20));
    draw_text_clipped(
        buffer,
        width,
        height,
        card_x.saturating_add(10),
        card_y.saturating_add(8),
        &title_display,
        0xFF58a6ff,
        card_x.saturating_add(card_w).saturating_sub(6),
    );

    for (i, line) in lines.iter().enumerate() {
        let ly = card_y
            .saturating_add(24)
            .saturating_add(i.saturating_mul(16));
        let line_display = truncate_text_to_width(line, card_w.saturating_sub(20));
        draw_text_clipped(
            buffer,
            width,
            height,
            card_x.saturating_add(10),
            ly,
            &line_display,
            0xFFe6edf3,
            card_x.saturating_add(card_w).saturating_sub(6),
        );
    }
}

fn render_table_hover_card(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    mx: usize,
    my: usize,
    cat_name: &str,
    lines: &[String],
    visible_series: &[(usize, &SeriesData)],
) {
    let mut max_chars = cat_name.chars().count().saturating_add(12);
    for l in lines {
        let count = l.chars().count();
        if count > max_chars {
            max_chars = count;
        }
    }
    let needed_w = max_chars.saturating_mul(7).saturating_add(34);
    let card_w = needed_w.clamp(180, (width.saturating_sub(40)).min(640));
    let card_h = 28usize.saturating_add(lines.len().saturating_mul(18));

    let card_x = if mx.saturating_add(16).saturating_add(card_w) < width {
        mx.saturating_add(16)
    } else {
        mx.saturating_sub(card_w.saturating_add(12))
    };
    let card_y = if my.saturating_add(16).saturating_add(card_h) < height {
        my.saturating_add(16)
    } else {
        my.saturating_sub(card_h.saturating_add(12))
    };

    fill_rect_alpha(
        buffer, width, height, card_x, card_y, card_w, card_h, 0xFF0d1117, 242,
    );
    draw_rect_outline(
        buffer, width, height, card_x, card_y, card_w, card_h, 0xFF388bfd,
    );

    let title = format!("Category: {}", cat_name);
    let title_disp = truncate_text_to_width(&title, card_w.saturating_sub(16));
    draw_text_clipped(
        buffer,
        width,
        height,
        card_x.saturating_add(8),
        card_y.saturating_add(7),
        &title_disp,
        0xFF58a6ff,
        card_x.saturating_add(card_w).saturating_sub(4),
    );

    let div_y = card_y.saturating_add(22);
    if div_y < height {
        let row = div_y.saturating_mul(width);
        for x in
            (card_x.saturating_add(6))..(card_x.saturating_add(card_w).saturating_sub(6)).min(width)
        {
            buffer[row.saturating_add(x)] = 0xFF30363d;
        }
    }

    for (i, line) in lines.iter().enumerate() {
        let sy = card_y
            .saturating_add(26)
            .saturating_add(i.saturating_mul(18));
        if sy.saturating_add(12) >= height {
            break;
        }
        let col = if let Some((s_idx, s)) = visible_series.get(i) {
            parse_hex_color(
                s.color
                    .as_deref()
                    .unwrap_or(DEFAULT_CHART_COLORS[s_idx % DEFAULT_CHART_COLORS.len()]),
            )
        } else {
            0xFFe6edf3
        };
        fill_rect(
            buffer,
            width,
            height,
            card_x.saturating_add(8),
            sy.saturating_add(4),
            6,
            6,
            col,
        );
        let line_disp = truncate_text_to_width(line, card_w.saturating_sub(26));
        draw_text_clipped(
            buffer,
            width,
            height,
            card_x.saturating_add(18),
            sy.saturating_add(2),
            &line_disp,
            0xFFe6edf3,
            card_x.saturating_add(card_w).saturating_sub(4),
        );
    }
}

fn render_kpi_hover_card(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    mx: usize,
    my: usize,
    title: &str,
    val: &str,
    sub: &str,
    desc: &str,
    accent_col: u32,
) {
    let line1 = format!("Metric: {}", title);
    let line2 = format!("Value:  {}", val);
    let line3 = format!("Scope:  {}", sub);
    let line4 = desc.to_string();
    let max_chars = line1
        .len()
        .max(line2.len())
        .max(line3.len())
        .max(line4.len());
    let needed_w = max_chars.saturating_mul(7).saturating_add(32);
    let card_w = needed_w.clamp(200, (width.saturating_sub(40)).min(640));
    let card_h = 76usize;

    let card_x = if mx.saturating_add(16).saturating_add(card_w) < width {
        mx.saturating_add(16)
    } else {
        mx.saturating_sub(card_w.saturating_add(12))
    };
    let card_y = if my.saturating_add(16).saturating_add(card_h) < height {
        my.saturating_add(16)
    } else {
        my.saturating_sub(card_h.saturating_add(12))
    };

    fill_rect_alpha(
        buffer, width, height, card_x, card_y, card_w, card_h, 0xFF0d1117, 242,
    );
    draw_rect_outline(
        buffer, width, height, card_x, card_y, card_w, card_h, accent_col,
    );
    fill_rect(buffer, width, height, card_x, card_y, card_w, 2, accent_col);

    draw_text_clipped(
        buffer,
        width,
        height,
        card_x.saturating_add(8),
        card_y.saturating_add(6),
        &line1,
        accent_col,
        card_x.saturating_add(card_w).saturating_sub(4),
    );
    draw_text_clipped(
        buffer,
        width,
        height,
        card_x.saturating_add(8),
        card_y.saturating_add(22),
        &line2,
        0xFFFFFFFF,
        card_x.saturating_add(card_w).saturating_sub(4),
    );
    draw_text_clipped(
        buffer,
        width,
        height,
        card_x.saturating_add(8),
        card_y.saturating_add(38),
        &line3,
        0xFF8b949e,
        card_x.saturating_add(card_w).saturating_sub(4),
    );
    draw_text_clipped(
        buffer,
        width,
        height,
        card_x.saturating_add(8),
        card_y.saturating_add(56),
        &line4,
        0xFFe6edf3,
        card_x.saturating_add(card_w).saturating_sub(4),
    );
}

pub fn parse_hex_color(hex: &str) -> u32 {
    let clean = hex.trim_start_matches('#');
    if let Ok(val) = u32::from_str_radix(clean, 16) {
        if clean.len() <= 6 {
            0xFF000000 | val
        } else {
            val
        }
    } else {
        0xFF58a6ff
    }
}

/// Calculate modal layout coordinates for the Chart Data Inspector
pub fn get_inspector_layout(
    width: usize,
    height: usize,
) -> (Rect, Rect, Rect, Rect, Rect, Rect, Rect, Rect) {
    let max_w = (width as f32 - 20.0).max(300.0);
    let max_h = (height as f32 - 20.0).max(250.0);
    let modal_w = ((width as f32) * 0.90).clamp(300.0, 1240.0).min(max_w);
    let modal_h = ((height as f32) * 0.88).clamp(250.0, 840.0).min(max_h);
    let modal_x = ((width as f32 - modal_w) * 0.5).max(10.0);
    let modal_y = ((height as f32 - modal_h) * 0.5).max(10.0);
    let modal_rect = Rect::new(modal_x, modal_y, modal_w, modal_h);

    let header_rect = Rect::new(modal_x, modal_y, modal_w, 44.0);
    let close_btn = Rect::new(modal_x + modal_w - 74.0, modal_y + 9.0, 64.0, 26.0);
    let export_btn = Rect::new(modal_x + modal_w - 162.0, modal_y + 9.0, 80.0, 26.0);

    let series_strip = Rect::new(modal_x + 12.0, modal_y + 48.0, modal_w - 24.0, 28.0);
    let kpi_strip = Rect::new(modal_x + 12.0, modal_y + 80.0, modal_w - 24.0, 52.0);

    let body_y = modal_y + 138.0;
    let body_h = (modal_h - 138.0 - 28.0).max(60.0);
    let left_w = ((modal_w - 32.0) * 0.55).round();
    let left_pane = Rect::new(modal_x + 12.0, body_y, left_w, body_h);
    let right_pane = Rect::new(
        modal_x + 12.0 + left_w + 8.0,
        body_y,
        (modal_w - 24.0 - left_w - 8.0).max(100.0),
        body_h,
    );

    (
        modal_rect,
        header_rect,
        close_btn,
        export_btn,
        series_strip,
        kpi_strip,
        left_pane,
        right_pane,
    )
}

/// Calculate bounds for series filter chips in series_strip.
/// Dynamically calculates available space to the transform preset buttons on the right,
/// ensuring chips expand generously into available space so series names are never truncated with "...".
#[must_use]
pub fn get_series_chip_rects(
    series_strip: Rect,
    series: &[slide_core::chart::SeriesData],
) -> Vec<(usize, Rect)> {
    if series.is_empty() {
        return Vec::new();
    }
    let cur_x = series_strip.x + 58.0;
    let presets = get_transform_preset_rects(series_strip);
    let right_limit = presets
        .first()
        .map(|(r, _, _)| r.x - 12.0)
        .unwrap_or(series_strip.x + series_strip.width - 8.0);
    let available_w = (right_limit - cur_x).max(60.0);

    let num_chips = series.len();
    let gap = 6.0f32;
    let total_gaps = (num_chips.saturating_sub(1) as f32) * gap;
    let net_available_w = (available_w - total_gaps).max(50.0);

    // Calculate minimum width required to comfortably show full series name without any truncation
    let min_fit_widths: Vec<f32> = series
        .iter()
        .map(|s| (s.name.chars().count() as f32 * 7.5 + 28.0).max(64.0))
        .collect();
    let total_min_fit: f32 = min_fit_widths.iter().sum();

    let mut chips = Vec::with_capacity(num_chips);
    let mut x = cur_x;
    for (s_idx, &min_fit_w) in min_fit_widths.iter().enumerate() {
        let chip_w = if total_min_fit <= net_available_w {
            // Generously expand into the available space to the right instead of leaving a huge empty void!
            let extra_space = net_available_w - total_min_fit;
            let extra_per_chip = (extra_space / num_chips as f32).min(80.0);
            min_fit_w + extra_per_chip
        } else {
            // Proportionally allocate available width when space is tight
            ((min_fit_w / total_min_fit) * net_available_w).max(48.0)
        };
        let chip_rect = Rect::new(x, series_strip.y + 2.0, chip_w, 22.0);
        chips.push((s_idx, chip_rect));
        x += chip_w + gap;
    }
    chips
}

/// Compute adaptive column widths for data table in inspector right pane.
/// Dynamically balances category column and series columns based on actual text lengths and available space,
/// avoiding arbitrary truncation of either category or series names whenever space permits.
#[must_use]
pub fn get_table_column_widths(
    right_pane_w: f32,
    chart_data: &ChartData,
    num_visible_series: usize,
) -> (f32, f32) {
    let max_cat_chars = chart_data
        .categories
        .iter()
        .map(|c| c.chars().count())
        .max()
        .unwrap_or(8)
        .max(8);

    let max_series_chars = chart_data
        .series
        .iter()
        .map(|s| s.name.chars().count())
        .max()
        .unwrap_or(6)
        .max(6);

    let needed_cat_w = (max_cat_chars as f32 * 7.5 + 24.0).max(90.0);
    let needed_series_w = (max_series_chars as f32 * 7.5 + 24.0).max(75.0);
    let num_s = num_visible_series.max(1);

    if needed_cat_w + needed_series_w * num_s as f32 <= right_pane_w {
        let cat_col_w = needed_cat_w;
        let s_col_w = (right_pane_w - cat_col_w) / num_s as f32;
        (cat_col_w, s_col_w)
    } else {
        let min_series_col_w = 68.0f32;
        let series_needed = min_series_col_w * num_s as f32;
        let max_avail_cat_w = (right_pane_w - series_needed).max(85.0);
        let cat_col_w = needed_cat_w.min(max_avail_cat_w).max(85.0);
        let remaining_w = (right_pane_w - cat_col_w).max(20.0);
        let s_col_w = remaining_w / num_s as f32;
        (cat_col_w, s_col_w)
    }
}

/// Calculate bounds for table controls (search box, clear button, format toggle)
#[must_use]
pub fn get_table_control_rects(right_pane: Rect) -> (Rect, Rect, Rect) {
    let bar_y = right_pane.y + 4.0;
    let bar_h = 22.0;
    let format_w = 66.0;
    let clear_w = 56.0;
    let format_btn = Rect::new(
        right_pane.x + right_pane.width - format_w - 6.0,
        bar_y,
        format_w,
        bar_h,
    );
    let clear_btn = Rect::new(format_btn.x - clear_w - 4.0, bar_y, clear_w, bar_h);
    let search_w = (clear_btn.x - right_pane.x - 12.0).max(60.0);
    let search_box = Rect::new(right_pane.x + 6.0, bar_y, search_w, bar_h);
    (search_box, clear_btn, format_btn)
}

/// Hit-test within the Chart Data Inspector modal
pub fn hit_test_chart_inspector(
    state: &ChartInspectorState,
    width: usize,
    height: usize,
    mouse_x: f32,
    mouse_y: f32,
) -> Option<InspectorAction> {
    let (modal_rect, _, close_btn, export_btn, series_strip, _, left_pane, right_pane) =
        get_inspector_layout(width, height);

    if !modal_rect.contains(mouse_x, mouse_y) {
        return Some(InspectorAction::Close);
    }

    if close_btn.contains(mouse_x, mouse_y) {
        return Some(InspectorAction::Close);
    }

    if export_btn.contains(mouse_x, mouse_y) {
        return Some(InspectorAction::ExportCsv);
    }

    // Hit test type switcher tabs
    let types = [
        ("BAR", ChartType::Bar),
        ("LINE", ChartType::Line),
        ("AREA", ChartType::Area),
        ("PIE", ChartType::Pie),
        ("DONUT", ChartType::Donut),
    ];
    let tab_w = 48.0;
    let tab_h = 26.0;
    let tab_gap = 6.0;
    let start_tabs_x = export_btn.x - 10.0 - (types.len() as f32 * (tab_w + tab_gap));
    for (i, &(_, t)) in types.iter().enumerate() {
        let tx = start_tabs_x + i as f32 * (tab_w + tab_gap);
        let tab_rect = Rect::new(tx, close_btn.y, tab_w, tab_h);
        if tab_rect.contains(mouse_x, mouse_y) {
            return Some(InspectorAction::SetType(t));
        }
    }

    // Hit test series filter chips and transform presets
    if series_strip.contains(mouse_x, mouse_y) {
        for (rect, _, t) in get_transform_preset_rects(series_strip) {
            if rect.contains(mouse_x, mouse_y) {
                return Some(InspectorAction::SetTransform(t));
            }
        }

        for (s_idx, chip_rect) in get_series_chip_rects(series_strip, &state.chart_data.series) {
            if chip_rect.contains(mouse_x, mouse_y) {
                return Some(InspectorAction::ToggleSeries(s_idx));
            }
        }
    }

    // Hit test controls and table in right pane
    if right_pane.contains(mouse_x, mouse_y) {
        let (search_box, clear_btn, format_btn) = get_table_control_rects(right_pane);
        if search_box.contains(mouse_x, mouse_y) {
            return Some(InspectorAction::FocusSearch);
        }
        if clear_btn.contains(mouse_x, mouse_y) {
            return Some(InspectorAction::ClearFilter);
        }
        if format_btn.contains(mouse_x, mouse_y) {
            return Some(InspectorAction::CycleFormat);
        }

        let header_y = right_pane.y + 30.0;
        let header_h = 24.0;
        if mouse_y >= header_y && mouse_y < header_y + header_h {
            let visible_series: Vec<usize> = state
                .chart_data
                .series
                .iter()
                .enumerate()
                .filter(|(idx, _)| !state.hidden_series.contains(idx))
                .map(|(idx, _)| idx)
                .collect();
            let (cat_col_w, s_col_w) =
                get_table_column_widths(right_pane.width, &state.chart_data, visible_series.len());
            if mouse_x < right_pane.x + cat_col_w {
                return Some(InspectorAction::SortColumn(0));
            }
            let rel_x = mouse_x - (right_pane.x + cat_col_w);
            let s_slot = (rel_x / s_col_w) as usize;
            if let Some(&real_s_idx) = visible_series.get(s_slot) {
                return Some(InspectorAction::SortColumn(real_s_idx.saturating_add(1)));
            }
        }

        let table_y = right_pane.y + 56.0;
        let row_h = 24.0;
        let visible_rows = ((right_pane.height - 58.0).max(0.0) / row_h) as usize;
        let filtered_indices = state.get_filtered_category_indices();
        if mouse_y >= table_y && mouse_y < (right_pane.y + right_pane.height) {
            let row_slot = ((mouse_y - table_y) / row_h) as usize;
            if row_slot < visible_rows {
                let list_idx = row_slot.saturating_add(state.table_scroll);
                if let Some(&cat_idx) = filtered_indices.get(list_idx) {
                    return Some(InspectorAction::SelectCategory(cat_idx));
                }
            }
        }
    }

    // Hit test category in left pane visualizer
    if left_pane.contains(mouse_x, mouse_y) {
        let px = left_pane.x + 44.0;
        let pw = left_pane.width - 60.0;
        let py = left_pane.y + 36.0;
        let ph = left_pane.height - 70.0;
        let cat_count = state.chart_data.categories.len().max(1);
        let col_w = pw / cat_count as f32;
        if mouse_x >= px && mouse_x <= (px + pw) && mouse_y >= py && mouse_y <= (py + ph + 20.0) {
            let c_idx = (((mouse_x - px) / col_w) as usize).min(cat_count.saturating_sub(1));
            return Some(InspectorAction::SelectCategory(c_idx));
        }
    }

    None
}

/// Standalone fast raster visualizer for charts (used in Inspector and on-slide override)
pub fn draw_chart_visualizer(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    rect: Rect,
    chart_data: &ChartData,
    chart_type: ChartType,
    hidden_series: &std::collections::HashSet<usize>,
    hovered_cat: Option<usize>,
    _mouse_pos: Option<(f32, f32)>,
    marquee_range: Option<(usize, usize)>,
) {
    let rx = rect.x as usize;
    let ry = rect.y as usize;
    let rw = rect.width as usize;
    let rh = rect.height as usize;

    if rw < 60 || rh < 60 || rx >= width || ry >= height {
        return;
    }

    // Dark card background
    fill_rect(buffer, width, height, rx, ry, rw, rh, 0xFF161b22);
    draw_rect_outline(buffer, width, height, rx, ry, rw, rh, 0xFF30363d);

    // Title at top-left
    if let Some(ref title) = chart_data.title {
        draw_text(buffer, width, height, rx + 14, ry + 10, title, 0xFF58a6ff);
    }

    // Filter visible series
    let visible_series: Vec<(usize, &SeriesData)> = chart_data
        .series
        .iter()
        .enumerate()
        .filter(|(idx, _)| !hidden_series.contains(idx))
        .collect();

    // Legend at top right
    let mut cur_legend_x = (rx + rw).saturating_sub(16);
    for (idx, s) in chart_data.series.iter().enumerate().rev() {
        if hidden_series.contains(&idx) {
            continue;
        }
        let color = parse_hex_color(
            s.color
                .as_deref()
                .unwrap_or(DEFAULT_CHART_COLORS[idx % DEFAULT_CHART_COLORS.len()]),
        );
        let text_len = s.name.chars().count() * 6 + 18;
        if cur_legend_x < rx + text_len + 40 {
            break;
        }
        cur_legend_x -= text_len;
        fill_rect(buffer, width, height, cur_legend_x, ry + 11, 8, 8, color);
        draw_text(
            buffer,
            width,
            height,
            cur_legend_x + 12,
            ry + 11,
            &s.name,
            0xFF8b949e,
        );
    }

    if visible_series.is_empty() {
        draw_text_at_center(
            buffer,
            width,
            height,
            rx + rw / 2,
            ry + rh / 2,
            "No visible series",
            0xFF8b949e,
        );
        return;
    }

    match chart_type {
        | ChartType::Pie | ChartType::Donut => {
            let cx = (rx + rw / 2) as isize;
            let cy = (ry + rh / 2 + 8) as isize;
            let radius = ((rw.min(rh) as f32) * 0.36).max(25.0) as isize;
            let inner_r = if chart_type == ChartType::Donut {
                (radius as f32 * 0.52).round() as isize
            } else {
                0
            };

            let (names, values): (Vec<String>, Vec<f64>) = if chart_data.series.len() == 1 {
                let s = &chart_data.series[0];
                let vals: Vec<f64> = (0..chart_data.categories.len())
                    .map(|i| s.values.get(i).copied().unwrap_or(0.0))
                    .collect();
                (chart_data.categories.clone(), vals)
            } else {
                let n: Vec<String> = visible_series.iter().map(|(_, s)| s.name.clone()).collect();
                let v: Vec<f64> = visible_series
                    .iter()
                    .map(|(_, s)| s.values.first().copied().unwrap_or(0.0))
                    .collect();
                (n, v)
            };

            let total: f64 = values.iter().sum();
            if total > 0.0 {
                let mut slice_angles = Vec::new();
                let mut current_ang = 0.0f32;
                for (i, &v) in values.iter().enumerate() {
                    let frac = (v / total) as f32;
                    let slice_ang = frac * std::f32::consts::TAU;
                    let col = parse_hex_color(DEFAULT_CHART_COLORS[i % DEFAULT_CHART_COLORS.len()]);
                    slice_angles.push((current_ang, current_ang + slice_ang, col));
                    current_ang += slice_ang;
                }

                let r_sq = radius * radius;
                let in_sq = inner_r * inner_r;
                for dy in -radius..=radius {
                    let py = cy + dy;
                    if py < 0 || py as usize >= height {
                        continue;
                    }
                    let row = (py as usize) * width;
                    for dx in -radius..=radius {
                        let px = cx + dx;
                        if px < 0 || px as usize >= width {
                            continue;
                        }
                        let d_sq = dx * dx + dy * dy;
                        if d_sq >= in_sq && d_sq <= r_sq {
                            let mut ang = (dy as f32).atan2(dx as f32);
                            if ang < 0.0 {
                                ang += std::f32::consts::TAU;
                            }
                            for (s_i, &(a1, a2, col)) in slice_angles.iter().enumerate() {
                                if ang >= a1 && ang <= a2 {
                                    let final_col = if hovered_cat == Some(s_i) {
                                        blend_pixel_fast(col, 0xFFFFFFFF, 60)
                                    } else {
                                        col
                                    };
                                    buffer[row + px as usize] = final_col;
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            if chart_type == ChartType::Donut {
                draw_text_at_center(
                    buffer,
                    width,
                    height,
                    cx as usize,
                    (cy - 7) as usize,
                    "TOTAL",
                    0xFF8b949e,
                );
                draw_text_at_center(
                    buffer,
                    width,
                    height,
                    cx as usize,
                    (cy + 4) as usize,
                    &chart_data.format_number(total),
                    0xFFFFFFFF,
                );
            }

            // Hover tooltip for slice
            if let Some(cat_idx) = hovered_cat
                && cat_idx < names.len()
            {
                let val = values.get(cat_idx).copied().unwrap_or(0.0);
                let pct = if total > 0.0 {
                    (val / total) * 100.0
                } else {
                    0.0
                };
                let tip_text = format!(
                    "{}: {} ({:.1}%)",
                    names[cat_idx],
                    chart_data.format_number(val),
                    pct
                );
                let tip_max_px = rw.saturating_sub(24);
                let tip_display = truncate_text_to_width(&tip_text, tip_max_px);
                let tip_y = (cy as usize)
                    .saturating_add(radius as usize)
                    .saturating_add(12);
                if tip_y < height {
                    draw_text_at_center(
                        buffer,
                        width,
                        height,
                        cx as usize,
                        tip_y,
                        &tip_display,
                        0xFF58a6ff,
                    );
                }
            }
        },
        | ChartType::Bar | ChartType::Line | ChartType::Area | ChartType::Scatter => {
            let px = rx + 44;
            let py = ry + 36;
            let pw = rw.saturating_sub(60);
            let ph = rh.saturating_sub(70);

            if pw < 20 || ph < 20 {
                return;
            }

            // Find min/max across visible series
            let mut y_min = 0.0f64;
            let mut y_max = 0.0f64;
            for (_, s) in &visible_series {
                for &v in &s.values {
                    if v < y_min {
                        y_min = v;
                    }
                    if v > y_max {
                        y_max = v;
                    }
                }
            }
            if y_max <= y_min {
                y_max = y_min + 1.0;
            }

            // Draw Y-axis gridlines and labels
            for step in 0..=4 {
                let tick_val = y_min + (step as f64 / 4.0) * (y_max - y_min);
                let tick_y = (py + ph).saturating_sub(((step as f32 / 4.0) * ph as f32) as usize);
                draw_dashed_hline(buffer, width, height, px, px + pw, tick_y, 0xFF21262d, 3, 3);
                let tick_str = chart_data.format_number(tick_val);
                let label_x = px.saturating_sub(tick_str.chars().count() * 6 + 6);
                draw_text(
                    buffer,
                    width,
                    height,
                    label_x,
                    tick_y.saturating_sub(3),
                    &tick_str,
                    0xFF8b949e,
                );
            }

            // Baseline
            fill_rect(buffer, width, height, px, py + ph, pw, 1, 0xFF30363d);

            let cat_count = chart_data.categories.len().max(1);
            let col_w = pw as f32 / cat_count as f32;
            let is_staggered = col_w < 72.0 && cat_count > 3;

            // X-axis category labels
            for (c_idx, cat) in chart_data.categories.iter().enumerate() {
                let cx = px as f32 + (c_idx as f32 + 0.5) * col_w;
                let (label_y, max_cat_w) = if is_staggered {
                    let y_offset = if c_idx.is_multiple_of(2) {
                        6
                    } else {
                        19
                    };
                    let stagger_w = (col_w * 2.0 - 6.0).max(col_w - 4.0) as usize;
                    (py.saturating_add(ph).saturating_add(y_offset), stagger_w)
                } else {
                    (
                        py.saturating_add(ph).saturating_add(8),
                        (col_w - 4.0).max(12.0) as usize,
                    )
                };
                let cat_display = truncate_text_to_width(cat, max_cat_w);
                draw_text_at_center(
                    buffer,
                    width,
                    height,
                    cx as usize,
                    label_y,
                    &cat_display,
                    0xFF8b949e,
                );
            }

            // Highlight hovered column
            if let Some(c_idx) = hovered_cat {
                let hx = (px as f32 + c_idx as f32 * col_w) as usize;
                fill_rect_alpha(
                    buffer,
                    width,
                    height,
                    hx,
                    py,
                    col_w as usize,
                    ph,
                    0xFF388bfd,
                    35,
                );
            }

            // Highlight marquee range if active
            if let Some((start, end)) = marquee_range {
                let min_c = start.min(end).min(cat_count.saturating_sub(1));
                let max_c = start.max(end).min(cat_count.saturating_sub(1));
                let mx1 = (px as f32 + min_c as f32 * col_w) as usize;
                let mx2 = (px as f32 + (max_c as f32 + 1.0) * col_w) as usize;
                let mw = mx2.saturating_sub(mx1).max(1);
                fill_rect_alpha(buffer, width, height, mx1, py, mw, ph, 0xFF388bfd, 65);
                draw_rect_outline(buffer, width, height, mx1, py, mw, ph, 0xFF58a6ff);
            }

            if chart_type == ChartType::Bar {
                let group_pad = col_w * 0.15;
                let bar_w = ((col_w - group_pad * 2.0) / visible_series.len() as f32).max(2.0);

                for (s_order, (s_idx, s)) in visible_series.iter().enumerate() {
                    let col = parse_hex_color(
                        s.color
                            .as_deref()
                            .unwrap_or(DEFAULT_CHART_COLORS[s_idx % DEFAULT_CHART_COLORS.len()]),
                    );
                    for (c_idx, &val) in s.values.iter().enumerate() {
                        let val_h = (((val - y_min) / (y_max - y_min).max(1e-6)) as f32
                            * ph as f32)
                            .clamp(0.0, ph as f32);
                        let bx =
                            (px as f32 + c_idx as f32 * col_w + group_pad + s_order as f32 * bar_w)
                                as usize;
                        let by = (py + ph).saturating_sub(val_h as usize);
                        fill_rect(
                            buffer,
                            width,
                            height,
                            bx,
                            by,
                            (bar_w as usize).saturating_sub(1).max(1),
                            val_h as usize,
                            col,
                        );
                        if hovered_cat == Some(c_idx) {
                            // Bright cap on active bar
                            fill_rect(
                                buffer,
                                width,
                                height,
                                bx,
                                by,
                                (bar_w as usize).saturating_sub(1).max(1),
                                2,
                                0xFFFFFFFF,
                            );
                        }
                    }
                }
            } else {
                for (s_idx, s) in &visible_series {
                    let col = parse_hex_color(
                        s.color
                            .as_deref()
                            .unwrap_or(DEFAULT_CHART_COLORS[s_idx % DEFAULT_CHART_COLORS.len()]),
                    );
                    let mut pts: Vec<(usize, usize)> = Vec::with_capacity(s.values.len());

                    for (c_idx, &val) in s.values.iter().enumerate() {
                        let val_h = (((val - y_min) / (y_max - y_min).max(1e-6)) as f32
                            * ph as f32)
                            .clamp(0.0, ph as f32);
                        let pt_x = (px as f32 + (c_idx as f32 + 0.5) * col_w) as usize;
                        let pt_y = (py + ph).saturating_sub(val_h as usize);
                        pts.push((pt_x, pt_y));
                    }

                    if chart_type == ChartType::Area {
                        for window in pts.windows(2) {
                            let (x0, y0) = window[0];
                            let (x1, y1) = window[1];
                            let base_y = py + ph;
                            for x in x0..=x1.min(width.saturating_sub(1)) {
                                let frac = if x1 > x0 {
                                    (x - x0) as f32 / (x1 - x0) as f32
                                } else {
                                    0.0
                                };
                                let top_y = y0 as f32 + frac * (y1 as f32 - y0 as f32);
                                for y in (top_y as usize)..=base_y.min(height.saturating_sub(1)) {
                                    buffer[y * width + x] =
                                        blend_pixel_fast(buffer[y * width + x], col, 70);
                                }
                            }
                        }
                    }

                    // Line segments
                    if chart_type != ChartType::Scatter {
                        for window in pts.windows(2) {
                            let (x0, y0) = window[0];
                            let (x1, y1) = window[1];
                            draw_thick_line(
                                buffer,
                                width,
                                height,
                                x0 as isize,
                                y0 as isize,
                                x1 as isize,
                                y1 as isize,
                                1,
                                col,
                            );
                        }
                    }

                    // Data markers
                    for &(pt_x, pt_y) in &pts {
                        draw_disk(buffer, width, height, pt_x as isize, pt_y as isize, 3, col);
                        draw_disk(
                            buffer,
                            width,
                            height,
                            pt_x as isize,
                            pt_y as isize,
                            1,
                            0xFFFFFFFF,
                        );
                    }
                }
            }

            // If a category is hovered, draw a glassmorphic category card
            if let Some(cat_idx) = hovered_cat
                && cat_idx < chart_data.categories.len()
            {
                let cat_name = &chart_data.categories[cat_idx];
                let mut max_chars = cat_name.chars().count();
                for (_, s) in &visible_series {
                    let val = s.values.get(cat_idx).copied().unwrap_or(0.0);
                    let val_str = chart_data.format_number(val);
                    let line_len = s
                        .name
                        .chars()
                        .count()
                        .saturating_add(val_str.chars().count())
                        .saturating_add(2);
                    if line_len > max_chars {
                        max_chars = line_len;
                    }
                }
                let needed_w = max_chars.saturating_mul(7).saturating_add(32);
                let max_card_w = rw.saturating_sub(24).min(640);
                let card_w = needed_w.clamp(160, max_card_w);
                let card_h = 24usize.saturating_add(visible_series.len().saturating_mul(16));
                let card_x = px.saturating_add(12);
                let card_y = py.saturating_add(12);

                fill_rect_alpha(
                    buffer, width, height, card_x, card_y, card_w, card_h, 0xFF0d1117, 230,
                );
                draw_rect_outline(
                    buffer, width, height, card_x, card_y, card_w, card_h, 0xFF30363d,
                );
                let cat_display = truncate_text_to_width(cat_name, card_w.saturating_sub(16));
                draw_text_clipped(
                    buffer,
                    width,
                    height,
                    card_x.saturating_add(8),
                    card_y.saturating_add(6),
                    &cat_display,
                    0xFF58a6ff,
                    card_x.saturating_add(card_w).saturating_sub(6),
                );

                for (i, (s_idx, s)) in visible_series.iter().enumerate() {
                    let sy = card_y
                        .saturating_add(22)
                        .saturating_add(i.saturating_mul(16));
                    let col = parse_hex_color(
                        s.color
                            .as_deref()
                            .unwrap_or(DEFAULT_CHART_COLORS[s_idx % DEFAULT_CHART_COLORS.len()]),
                    );
                    fill_rect(
                        buffer,
                        width,
                        height,
                        card_x.saturating_add(8),
                        sy.saturating_add(3),
                        6,
                        6,
                        col,
                    );
                    let val = s.values.get(cat_idx).copied().unwrap_or(0.0);
                    let line = format!("{}: {}", s.name, chart_data.format_number(val));
                    let line_display = truncate_text_to_width(&line, card_w.saturating_sub(24));
                    draw_text_clipped(
                        buffer,
                        width,
                        height,
                        card_x.saturating_add(18),
                        sy,
                        &line_display,
                        0xFFe6edf3,
                        card_x.saturating_add(card_w).saturating_sub(6),
                    );
                }
            }
        },
    }
}

/// Render the complete, responsive HUD-grade Chart Data Inspector modal
pub fn draw_chart_inspector(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    state: &ChartInspectorState,
    mouse_x: f32,
    mouse_y: f32,
) {
    let (
        modal_rect,
        header_rect,
        close_btn,
        export_btn,
        series_strip,
        kpi_strip,
        left_pane,
        right_pane,
    ) = get_inspector_layout(width, height);

    // 1. Dark glass backdrop on surrounding margins outside modal dialog (avoids blending 2M interior pixels)
    let mx = modal_rect.x as usize;
    let my = modal_rect.y as usize;
    let mw = modal_rect.width as usize;
    let mh = modal_rect.height as usize;
    let mx2 = mx.saturating_add(mw).min(width);
    let my2 = my.saturating_add(mh).min(height);

    // Top margin
    if my > 0 {
        fill_rect_alpha(buffer, width, height, 0, 0, width, my, 0x0000_0000, 180);
    }
    // Bottom margin
    if my2 < height {
        fill_rect_alpha(
            buffer,
            width,
            height,
            0,
            my2,
            width,
            height.saturating_sub(my2),
            0x0000_0000,
            180,
        );
    }
    // Left margin
    if mx > 0 && my2 > my {
        fill_rect_alpha(
            buffer,
            width,
            height,
            0,
            my,
            mx,
            my2.saturating_sub(my),
            0x0000_0000,
            180,
        );
    }
    // Right margin
    if mx2 < width && my2 > my {
        fill_rect_alpha(
            buffer,
            width,
            height,
            mx2,
            my,
            width.saturating_sub(mx2),
            my2.saturating_sub(my),
            0x0000_0000,
            180,
        );
    }

    // 2. Main modal dialog body
    fill_rect(buffer, width, height, mx, my, mw, mh, 0xFF0d1117);
    draw_rect_outline(
        buffer,
        width,
        height,
        modal_rect.x as usize,
        modal_rect.y as usize,
        modal_rect.width as usize,
        modal_rect.height as usize,
        0xFF30363d,
    );

    // 3. Header Bar
    fill_rect(
        buffer,
        width,
        height,
        header_rect.x as usize,
        header_rect.y as usize,
        header_rect.width as usize,
        header_rect.height as usize,
        0xFF161b22,
    );
    fill_rect(
        buffer,
        width,
        height,
        header_rect.x as usize,
        (header_rect.y + header_rect.height - 1.0) as usize,
        header_rect.width as usize,
        1,
        0xFF30363d,
    );

    // Header Title
    draw_text(
        buffer,
        width,
        height,
        (header_rect.x + 14.0) as usize,
        (header_rect.y + 14.0) as usize,
        "DATA INSPECTOR",
        0xFF38bdf8,
    );
    if let Some(ref title) = state.chart_data.title {
        let title_str = format!("| {}", title);
        draw_text(
            buffer,
            width,
            height,
            (header_rect.x + 115.0) as usize,
            (header_rect.y + 14.0) as usize,
            &title_str,
            0xFFc9d1d9,
        );
    }

    // Header Type Switcher Tabs
    let types = [
        ("BAR", ChartType::Bar),
        ("LINE", ChartType::Line),
        ("AREA", ChartType::Area),
        ("PIE", ChartType::Pie),
        ("DONUT", ChartType::Donut),
    ];
    let tab_w = 48.0;
    let tab_h = 26.0;
    let tab_gap = 6.0;
    let start_tabs_x = export_btn.x - 10.0 - (types.len() as f32 * (tab_w + tab_gap));

    for (i, &(label, t)) in types.iter().enumerate() {
        let tx = start_tabs_x + i as f32 * (tab_w + tab_gap);
        let tab_rect = Rect::new(tx, close_btn.y, tab_w, tab_h);
        let is_active = state.active_type == t;
        let is_hovered = tab_rect.contains(mouse_x, mouse_y);

        let bg = if is_active {
            0xFF1f6feb
        } else if is_hovered {
            0xFF21262d
        } else {
            0xFF161b22
        };
        let border = if is_active {
            0xFF58a6ff
        } else if is_hovered {
            0xFF8b949e
        } else {
            0xFF30363d
        };
        let text_col = if is_active {
            0xFFFFFFFF
        } else {
            0xFF8b949e
        };

        fill_rect(
            buffer,
            width,
            height,
            tx as usize,
            tab_rect.y as usize,
            tab_w as usize,
            tab_h as usize,
            bg,
        );
        draw_rect_outline(
            buffer,
            width,
            height,
            tx as usize,
            tab_rect.y as usize,
            tab_w as usize,
            tab_h as usize,
            border,
        );
        let text_x = (tx + (tab_w - label.len() as f32 * 6.0) * 0.5) as usize;
        let text_y = (tab_rect.y + 9.0) as usize;
        draw_text(buffer, width, height, text_x, text_y, label, text_col);
    }

    // Export CSV Button
    let is_export_hover = export_btn.contains(mouse_x, mouse_y);
    let export_bg = if is_export_hover {
        0xFF2ea043
    } else {
        0xFF238636
    };
    fill_rect(
        buffer,
        width,
        height,
        export_btn.x as usize,
        export_btn.y as usize,
        export_btn.width as usize,
        export_btn.height as usize,
        export_bg,
    );
    draw_rect_outline(
        buffer,
        width,
        height,
        export_btn.x as usize,
        export_btn.y as usize,
        export_btn.width as usize,
        export_btn.height as usize,
        0xFF3fb950,
    );
    draw_text_centered(buffer, width, height, export_btn, "EXPORT CSV", 0xFFFFFFFF);

    // Close Button
    let is_close_hover = close_btn.contains(mouse_x, mouse_y);
    let close_bg = if is_close_hover {
        0xFFf85149
    } else {
        0xFFda3633
    };
    fill_rect(
        buffer,
        width,
        height,
        close_btn.x as usize,
        close_btn.y as usize,
        close_btn.width as usize,
        close_btn.height as usize,
        close_bg,
    );
    draw_rect_outline(
        buffer,
        width,
        height,
        close_btn.x as usize,
        close_btn.y as usize,
        close_btn.width as usize,
        close_btn.height as usize,
        0xFFf85149,
    );
    draw_text_centered(buffer, width, height, close_btn, "CLOSE [X]", 0xFFFFFFFF);

    // 4. Series Filter Strip
    draw_text(
        buffer,
        width,
        height,
        series_strip.x as usize,
        (series_strip.y + 7.0) as usize,
        "SERIES:",
        0xFF8b949e,
    );
    let mut hovered_chip_info = None;
    for (s_idx, chip_rect) in get_series_chip_rects(series_strip, &state.chart_data.series) {
        let s = &state.chart_data.series[s_idx];
        let chip_w = chip_rect.width;
        let is_hidden = state.hidden_series.contains(&s_idx);
        let is_hovered = chip_rect.contains(mouse_x, mouse_y);
        if is_hovered {
            hovered_chip_info = Some((s.name.clone(), is_hidden));
        }

        let chip_bg = if is_hidden {
            0xFF161b22
        } else if is_hovered {
            0xFF21262d
        } else {
            0xFF1f242c
        };
        let chip_border = if is_hidden {
            0xFF21262d
        } else if is_hovered {
            0xFF58a6ff
        } else {
            0xFF30363d
        };
        let col = parse_hex_color(
            s.color
                .as_deref()
                .unwrap_or(DEFAULT_CHART_COLORS[s_idx % DEFAULT_CHART_COLORS.len()]),
        );

        fill_rect(
            buffer,
            width,
            height,
            chip_rect.x as usize,
            chip_rect.y as usize,
            chip_w as usize,
            chip_rect.height as usize,
            chip_bg,
        );
        draw_rect_outline(
            buffer,
            width,
            height,
            chip_rect.x as usize,
            chip_rect.y as usize,
            chip_w as usize,
            chip_rect.height as usize,
            chip_border,
        );

        // Status circle dot
        let dot_col = if is_hidden { 0xFF484f58 } else { col };
        draw_disk(
            buffer,
            width,
            height,
            (chip_rect.x + 8.0) as isize,
            (chip_rect.y + 11.0) as isize,
            3,
            dot_col,
        );

        // Series label
        let text_col = if is_hidden {
            0xFF6e7681
        } else {
            0xFFe6edf3
        };
        let chip_max_x = (chip_rect.x + chip_rect.width - 4.0) as usize;
        let chip_text =
            truncate_text_to_width(&s.name, (chip_rect.width - 20.0).max(10.0) as usize);
        draw_text_clipped(
            buffer,
            width,
            height,
            (chip_rect.x + 16.0) as usize,
            (chip_rect.y + 7.0) as usize,
            &chip_text,
            text_col,
            chip_max_x,
        );
    }

    // Transform Presets (right-aligned in series_strip)
    for (rect, label, t) in get_transform_preset_rects(series_strip) {
        let is_active = state.active_transform == t;
        let is_hovered = rect.contains(mouse_x, mouse_y);
        let bg = if is_active {
            0xFF1f6feb
        } else if is_hovered {
            0xFF21262d
        } else {
            0xFF161b22
        };
        let border = if is_active {
            0xFF58a6ff
        } else if is_hovered {
            0xFF8b949e
        } else {
            0xFF30363d
        };
        let text_col = if is_active {
            0xFFFFFFFF
        } else {
            0xFF8b949e
        };

        fill_rect(
            buffer,
            width,
            height,
            rect.x as usize,
            rect.y as usize,
            rect.width as usize,
            rect.height as usize,
            bg,
        );
        draw_rect_outline(
            buffer,
            width,
            height,
            rect.x as usize,
            rect.y as usize,
            rect.width as usize,
            rect.height as usize,
            border,
        );
        draw_text_centered(buffer, width, height, rect, label, text_col);
    }

    // 5. KPI Stat Cards Strip (using precomputed cached stats)
    let filtered_indices = state.get_filtered_category_indices();
    let stats = &state.cached_stats;
    let card_count = 6.0;
    let gap = 6.0;
    let kpi_w = (kpi_strip.width - (card_count - 1.0) * gap) / card_count;
    let kpis = [
        (
            "TOTAL SUM",
            state.chart_data.format_number(stats.total_sum),
            "All visible".to_string(),
            0xFF38bdf8,
            "Total cumulative sum across active series and filtered rows",
        ),
        (
            "MEAN / AVG",
            state.chart_data.format_number(stats.avg),
            "Per record".to_string(),
            0xFF34d399,
            "Average mean across visible series values",
        ),
        (
            "MEDIAN",
            state.chart_data.format_number(stats.median),
            "Middle 50%".to_string(),
            0xFF60a5fa,
            "Median midpoint of all visible values",
        ),
        (
            "STD DEV",
            state.chart_data.format_number(stats.std_dev),
            "Dispersion".to_string(),
            0xFFf472b6,
            "Standard deviation measuring data variability",
        ),
        (
            "PEAK / MAX",
            state.chart_data.format_number(stats.max_val),
            if stats.max_cat.is_empty() {
                "Peak".to_string()
            } else {
                stats.max_cat.clone()
            },
            0xFFf59e0b,
            "Highest observed metric value and associated category",
        ),
        (
            "MIN / LOW",
            state.chart_data.format_number(stats.min_val),
            if stats.min_cat.is_empty() {
                "Lowest".to_string()
            } else {
                stats.min_cat.clone()
            },
            0xFFa855f7,
            "Lowest observed metric value and associated category",
        ),
    ];

    let mut hovered_kpi = None;
    for (i, &(kpi_title, ref kpi_val, ref kpi_sub, accent_col, detail_desc)) in
        kpis.iter().enumerate()
    {
        let kx = (kpi_strip.x + i as f32 * (kpi_w + gap)) as usize;
        let ky = kpi_strip.y as usize;
        let kw = kpi_w as usize;
        let kh = kpi_strip.height as usize;

        let is_hovered = mouse_x >= kx as f32
            && mouse_x < (kx.saturating_add(kw)) as f32
            && mouse_y >= ky as f32
            && mouse_y < (ky.saturating_add(kh)) as f32;

        if is_hovered {
            hovered_kpi = Some((
                kpi_title,
                kpi_val.clone(),
                kpi_sub.clone(),
                detail_desc,
                accent_col,
            ));
        }

        let bg_col = if is_hovered {
            0xFF1c2333
        } else {
            0xFF161b22
        };
        let border_col = if is_hovered {
            accent_col
        } else {
            0xFF30363d
        };

        fill_rect(buffer, width, height, kx, ky, kw, kh, bg_col);
        draw_rect_outline(buffer, width, height, kx, ky, kw, kh, border_col);
        // Accent bar on top
        fill_rect(buffer, width, height, kx, ky, kw, 2, accent_col);

        let kpi_title_display = truncate_text_to_width(kpi_title, kw.saturating_sub(12));
        draw_text_clipped(
            buffer,
            width,
            height,
            kx.saturating_add(8),
            ky.saturating_add(6),
            &kpi_title_display,
            0xFF8b949e,
            kx.saturating_add(kw).saturating_sub(4),
        );
        let kpi_val_display = truncate_text_to_width(kpi_val, kw.saturating_sub(12));
        draw_text_clipped(
            buffer,
            width,
            height,
            kx.saturating_add(8),
            ky.saturating_add(19),
            &kpi_val_display,
            0xFFFFFFFF,
            kx.saturating_add(kw).saturating_sub(4),
        );
        let kpi_sub_display = truncate_text_to_width(kpi_sub, kw.saturating_sub(12));
        draw_text_clipped(
            buffer,
            width,
            height,
            kx.saturating_add(8),
            ky.saturating_add(33),
            &kpi_sub_display,
            accent_col,
            kx.saturating_add(kw).saturating_sub(4),
        );
    }

    // 6. Left Pane: Standalone Fast Visualizer
    draw_chart_visualizer(
        buffer,
        width,
        height,
        left_pane,
        &state.chart_data,
        state.active_type,
        &state.hidden_series,
        state.hovered_category,
        Some((mouse_x, mouse_y)),
        state.marquee_range,
    );

    // 7. Right Pane: Interactive Synchronized Data Table
    let rx = right_pane.x as usize;
    let ry = right_pane.y as usize;
    let rw = right_pane.width as usize;
    let rh = right_pane.height as usize;

    fill_rect(buffer, width, height, rx, ry, rw, rh, 0xFF161b22);
    draw_rect_outline(buffer, width, height, rx, ry, rw, rh, 0xFF30363d);

    // Controls bar: Search Box, Clear Button, Format Button
    let (search_box, clear_btn, format_btn) = get_table_control_rects(right_pane);

    let search_border = if state.search_active {
        0xFF58a6ff
    } else if search_box.contains(mouse_x, mouse_y) {
        0xFF8b949e
    } else {
        0xFF30363d
    };
    fill_rect(
        buffer,
        width,
        height,
        search_box.x as usize,
        search_box.y as usize,
        search_box.width as usize,
        search_box.height as usize,
        0xFF0d1117,
    );
    draw_rect_outline(
        buffer,
        width,
        height,
        search_box.x as usize,
        search_box.y as usize,
        search_box.width as usize,
        search_box.height as usize,
        search_border,
    );

    let search_display = if state.search_query.is_empty() {
        if state.search_active {
            "Type to filter (e.g. >50 or Q3)..."
        } else {
            "Search / Filter [Press /]..."
        }
    } else {
        &state.search_query
    };
    let search_col = if state.search_query.is_empty() {
        0xFF6e7681
    } else {
        0xFFe6edf3
    };
    draw_text(
        buffer,
        width,
        height,
        (search_box.x + 6.0) as usize,
        (search_box.y + 6.0) as usize,
        search_display,
        search_col,
    );
    if state.search_active {
        let cursor_x = (search_box.x + 6.0 + (state.search_query.len() as f32 * 7.0)) as usize;
        let cursor_y = (search_box.y + 4.0) as usize;
        if cursor_x + 2 < width {
            fill_rect(buffer, width, height, cursor_x, cursor_y, 2, 14, 0xFF58a6ff);
        }
    }

    let is_clear_hover = clear_btn.contains(mouse_x, mouse_y);
    let has_filter = !state.search_query.is_empty() || state.marquee_range.is_some();
    let clear_bg = if is_clear_hover {
        0xFF21262d
    } else {
        0xFF161b22
    };
    let clear_text_col = if has_filter {
        0xFFf85149
    } else {
        0xFF6e7681
    };
    fill_rect(
        buffer,
        width,
        height,
        clear_btn.x as usize,
        clear_btn.y as usize,
        clear_btn.width as usize,
        clear_btn.height as usize,
        clear_bg,
    );
    draw_rect_outline(
        buffer,
        width,
        height,
        clear_btn.x as usize,
        clear_btn.y as usize,
        clear_btn.width as usize,
        clear_btn.height as usize,
        0xFF30363d,
    );
    draw_text_centered(buffer, width, height, clear_btn, "CLEAR", clear_text_col);

    let is_format_hover = format_btn.contains(mouse_x, mouse_y);
    let format_bg = if is_format_hover {
        0xFF21262d
    } else {
        0xFF161b22
    };
    fill_rect(
        buffer,
        width,
        height,
        format_btn.x as usize,
        format_btn.y as usize,
        format_btn.width as usize,
        format_btn.height as usize,
        format_bg,
    );
    draw_rect_outline(
        buffer,
        width,
        height,
        format_btn.x as usize,
        format_btn.y as usize,
        format_btn.width as usize,
        format_btn.height as usize,
        0xFF30363d,
    );
    let fmt_label = match state.chart_data.format {
        | slide_core::chart::NumberFormat::Auto => "FMT:AUTO",
        | slide_core::chart::NumberFormat::Currency => "FMT:$",
        | slide_core::chart::NumberFormat::Percentage => "FMT:%",
        | slide_core::chart::NumberFormat::Compact => "FMT:1K",
        | slide_core::chart::NumberFormat::Scientific => "FMT:SCI",
        | slide_core::chart::NumberFormat::Integer => "FMT:INT",
        | slide_core::chart::NumberFormat::Standard => "FMT:STD",
    };
    draw_text_centered(buffer, width, height, format_btn, fmt_label, 0xFF58a6ff);

    // Table Header
    let visible_series: Vec<(usize, &SeriesData)> = state
        .chart_data
        .series
        .iter()
        .enumerate()
        .filter(|(idx, _)| !state.hidden_series.contains(idx))
        .collect();
    let (cat_col_w_f, s_col_w_f) =
        get_table_column_widths(right_pane.width, &state.chart_data, visible_series.len());
    let cat_col_w = cat_col_w_f as usize;
    let s_col_w = s_col_w_f as usize;

    let header_y = ry.saturating_add(30);
    fill_rect(buffer, width, height, rx, header_y, rw, 24, 0xFF21262d);
    fill_rect(
        buffer,
        width,
        height,
        rx,
        header_y.saturating_add(23),
        rw,
        1,
        0xFF30363d,
    );

    let cat_sort_icon = if state.sort_column == Some(0) {
        if state.sort_ascending {
            " ^"
        } else {
            " v"
        }
    } else {
        ""
    };
    let cat_header_label = format!("CATEGORY{}", cat_sort_icon);
    let cat_header_display =
        truncate_text_to_width(&cat_header_label, cat_col_w.saturating_sub(14));
    draw_text_clipped(
        buffer,
        width,
        height,
        rx.saturating_add(10),
        header_y.saturating_add(8),
        &cat_header_display,
        0xFF8b949e,
        rx.saturating_add(cat_col_w).saturating_sub(4),
    );

    for (i, (s_idx, s)) in visible_series.iter().enumerate() {
        let col_x = rx
            .saturating_add(cat_col_w)
            .saturating_add(i.saturating_mul(s_col_w));
        let next_col_x = (rx
            .saturating_add(cat_col_w)
            .saturating_add((i.saturating_add(1)).saturating_mul(s_col_w)))
        .min(rx.saturating_add(rw));
        let col_max_w = next_col_x.saturating_sub(col_x).saturating_sub(8);
        let col = parse_hex_color(
            s.color
                .as_deref()
                .unwrap_or(DEFAULT_CHART_COLORS[s_idx % DEFAULT_CHART_COLORS.len()]),
        );
        let s_sort_icon = if state.sort_column == Some(s_idx.saturating_add(1)) {
            if state.sort_ascending {
                " ^"
            } else {
                " v"
            }
        } else {
            ""
        };
        let s_header_label = format!("{}{}", s.name, s_sort_icon);
        let s_header_display = truncate_text_to_width(&s_header_label, col_max_w);
        draw_text_clipped(
            buffer,
            width,
            height,
            col_x.saturating_add(6),
            header_y.saturating_add(8),
            &s_header_display,
            col,
            next_col_x.saturating_sub(2),
        );
    }

    // Table Rows
    let table_y = ry.saturating_add(56);
    let row_h = 24usize;
    let visible_rows = (rh.saturating_sub(58)) / row_h;
    if filtered_indices.is_empty() {
        draw_text_at_center(
            buffer,
            width,
            height,
            rx.saturating_add(rw / 2),
            table_y.saturating_add(30),
            "No matching records",
            0xFF8b949e,
        );
    } else {
        for row_i in 0..visible_rows {
            let list_idx = state.table_scroll.saturating_add(row_i);
            let Some(&cat_idx) = filtered_indices.get(list_idx) else {
                break;
            };
            let row_y = table_y.saturating_add(row_i.saturating_mul(row_h));
            let is_hovered = state.hovered_category == Some(cat_idx);

            let row_bg = if is_hovered {
                0xFF1f385c // Electric blue highlight
            } else if row_i.is_multiple_of(2) {
                0xFF161b22
            } else {
                0xFF0d1117
            };

            fill_rect(
                buffer,
                width,
                height,
                rx.saturating_add(1),
                row_y,
                rw.saturating_sub(2),
                row_h.saturating_sub(1),
                row_bg,
            );

            // Category name
            let cat_name = state
                .chart_data
                .categories
                .get(cat_idx)
                .map(String::as_str)
                .unwrap_or("");
            let text_col = if is_hovered {
                0xFFFFFFFF
            } else {
                0xFFe6edf3
            };
            let cat_display = truncate_text_to_width(cat_name, cat_col_w.saturating_sub(14));
            draw_text_clipped(
                buffer,
                width,
                height,
                rx.saturating_add(10),
                row_y.saturating_add(8),
                &cat_display,
                text_col,
                rx.saturating_add(cat_col_w).saturating_sub(4),
            );

            // Series values
            for (i, (_, s)) in visible_series.iter().enumerate() {
                let col_x = rx
                    .saturating_add(cat_col_w)
                    .saturating_add(i.saturating_mul(s_col_w));
                let next_col_x = (rx
                    .saturating_add(cat_col_w)
                    .saturating_add((i.saturating_add(1)).saturating_mul(s_col_w)))
                .min(rx.saturating_add(rw));
                let col_max_w = next_col_x.saturating_sub(col_x).saturating_sub(8);
                let val = s.values.get(cat_idx).copied().unwrap_or(0.0);
                let val_str = state.chart_data.format_number(val);
                let val_display = truncate_text_to_width(&val_str, col_max_w);
                draw_text_clipped(
                    buffer,
                    width,
                    height,
                    col_x.saturating_add(6),
                    row_y.saturating_add(8),
                    &val_display,
                    text_col,
                    next_col_x.saturating_sub(2),
                );
            }
        }
    }

    // 7.1 Floating hover detail tooltip for table rows
    let is_table_hover = mouse_x >= rx as f32
        && mouse_x < (rx.saturating_add(rw)) as f32
        && mouse_y >= table_y as f32
        && mouse_y < (ry.saturating_add(rh)) as f32;
    if is_table_hover
        && let Some(cat_idx) = state.hovered_category
        && let Some(cat_name) = state.chart_data.categories.get(cat_idx)
    {
        let mut lines = Vec::with_capacity(visible_series.len());
        for (_, s) in &visible_series {
            let v = s.values.get(cat_idx).copied().unwrap_or(0.0);
            lines.push(format!("{}: {}", s.name, state.chart_data.format_number(v)));
        }
        render_table_hover_card(
            buffer,
            width,
            height,
            mouse_x as usize,
            mouse_y as usize,
            cat_name,
            &lines,
            &visible_series,
        );
    }

    // 7.2 Floating hover detail tooltip for KPI cards
    if let Some((kpi_title, kpi_val, kpi_sub, detail_desc, accent_col)) = hovered_kpi {
        render_kpi_hover_card(
            buffer,
            width,
            height,
            mouse_x as usize,
            mouse_y as usize,
            kpi_title,
            &kpi_val,
            &kpi_sub,
            detail_desc,
            accent_col,
        );
    }

    // 7.3 Floating hover detail tooltip for series filter chips
    if let Some((ref name, is_hidden)) = hovered_chip_info {
        let status_str = if is_hidden {
            "Status: Hidden (Click chip or press 1-9 to show)".to_string()
        } else {
            "Status: Active (Click chip or press 1-9 to hide)".to_string()
        };
        let tip_lines = vec![status_str];
        let title_str = format!("Series: {}", name);
        render_simple_tooltip(
            buffer,
            width,
            height,
            mouse_x as usize,
            mouse_y as usize,
            &title_str,
            &tip_lines,
        );
    }

    // 8. Footer Keyboard Shortcuts
    let footer_text = "[Esc] Close   [/] Search   [F] Format   [Tab] Type   [1-9] Series   [C] CSV   [Wheel] Scroll";
    draw_text(
        buffer,
        width,
        height,
        (modal_rect.x + 14.0) as usize,
        (modal_rect.y + modal_rect.height - 18.0) as usize,
        footer_text,
        0xFF8b949e,
    );

    // 9. Toast Notification HUD Pill
    if let Some((ref msg, ref t)) = state.toast_message
        && t.elapsed() < std::time::Duration::from_millis(2500)
    {
        let toast_w = (msg.chars().count() * 6 + 32).max(180) as f32;
        let toast_x = modal_rect.x + (modal_rect.width - toast_w) * 0.5;
        let toast_y = modal_rect.y + 10.0;
        let toast_rect = Rect::new(toast_x, toast_y, toast_w, 24.0);

        fill_rect(
            buffer,
            width,
            height,
            toast_x as usize,
            toast_y as usize,
            toast_w as usize,
            24,
            0xFF238636,
        );
        draw_rect_outline(
            buffer,
            width,
            height,
            toast_x as usize,
            toast_y as usize,
            toast_w as usize,
            24,
            0xFF3fb950,
        );
        draw_text_centered(buffer, width, height, toast_rect, msg, 0xFFFFFFFF);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slide_core::chart::ChartData;
    use slide_core::chart::ChartType;
    use slide_core::chart::SeriesData;

    fn make_sample_chart() -> ChartData {
        let mut chart = ChartData::new(ChartType::Bar);
        chart.title = Some("Quarterly Performance".into());
        chart.categories = vec!["Q1".into(), "Q2".into(), "Q3".into(), "Q4".into()];
        chart.series.push(SeriesData {
            name: "Revenue".into(),
            values: vec![120.0, 200.0, 150.0, 310.0],
            color: Some("#38bdf8".into()),
        });
        chart.series.push(SeriesData {
            name: "Profit".into(),
            values: vec![40.0, 80.0, 60.0, 140.0],
            color: Some("#34d399".into()),
        });
        chart
    }

    #[test]
    fn test_chart_inspector_state() {
        let chart = make_sample_chart();
        let mut inspector = ChartInspectorState::new(chart, ChartType::Bar);

        assert_eq!(inspector.active_type, ChartType::Bar);
        inspector.cycle_type();
        assert_eq!(inspector.active_type, ChartType::Line);
        inspector.cycle_type();
        assert_eq!(inspector.active_type, ChartType::Area);

        // Toggle series
        inspector.toggle_series(1);
        assert!(inspector.hidden_series.contains(&1));
        inspector.toggle_series(1);
        assert!(!inspector.hidden_series.contains(&1));

        // Format cycle
        inspector.cycle_format();
        assert_eq!(
            inspector.chart_data.format,
            slide_core::chart::NumberFormat::Currency
        );

        // Filter by numeric query "> 150"
        inspector.search_query = "> 150".into();
        inspector.recompute_cache();
        let filtered = inspector.get_filtered_category_indices();
        // Series Revenue has 120, 200, 150, 310. Q2 (200) and Q4 (310) match.
        assert_eq!(filtered, &[1, 3]);

        // Filter by text query "Q1"
        inspector.search_query = "q1".into();
        inspector.recompute_cache();
        assert_eq!(inspector.get_filtered_category_indices(), &[0]);

        // Sorting by category descending
        inspector.search_query.clear();
        inspector.sort_column = Some(0);
        inspector.sort_ascending = false;
        inspector.recompute_cache();
        assert_eq!(inspector.get_filtered_category_indices(), &[3, 2, 1, 0]);

        // Clear filter
        inspector.clear_filter();
        assert!(inspector.search_query.is_empty());
        assert_eq!(inspector.get_filtered_category_indices().len(), 4);

        // Toast
        inspector.show_toast("Testing Toast");
        assert!(inspector.toast_message.is_some());
    }

    #[test]
    fn test_chart_inspector_hit_test() {
        let chart = make_sample_chart();
        let inspector = ChartInspectorState::new(chart, ChartType::Bar);
        let width = 1280;
        let height = 720;

        let (_modal_rect, _, close_btn, export_btn, _, _, _, right_pane) =
            get_inspector_layout(width, height);

        // Click outside modal
        let outside_action = hit_test_chart_inspector(&inspector, width, height, 5.0, 5.0);
        assert_eq!(outside_action, Some(InspectorAction::Close));

        // Click close button
        let close_action = hit_test_chart_inspector(
            &inspector,
            width,
            height,
            close_btn.x + close_btn.width * 0.5,
            close_btn.y + close_btn.height * 0.5,
        );
        assert_eq!(close_action, Some(InspectorAction::Close));

        // Click export button
        let export_action = hit_test_chart_inspector(
            &inspector,
            width,
            height,
            export_btn.x + export_btn.width * 0.5,
            export_btn.y + export_btn.height * 0.5,
        );
        assert_eq!(export_action, Some(InspectorAction::ExportCsv));

        // Table controls hit testing
        let (search_box, clear_btn, format_btn) = get_table_control_rects(right_pane);
        assert_eq!(
            hit_test_chart_inspector(
                &inspector,
                width,
                height,
                search_box.x + 2.0,
                search_box.y + 2.0
            ),
            Some(InspectorAction::FocusSearch)
        );
        assert_eq!(
            hit_test_chart_inspector(
                &inspector,
                width,
                height,
                clear_btn.x + 2.0,
                clear_btn.y + 2.0
            ),
            Some(InspectorAction::ClearFilter)
        );
        assert_eq!(
            hit_test_chart_inspector(
                &inspector,
                width,
                height,
                format_btn.x + 2.0,
                format_btn.y + 2.0
            ),
            Some(InspectorAction::CycleFormat)
        );
    }

    #[test]
    fn test_chart_visualizer_rendering_all_types() {
        let chart = make_sample_chart();
        let width = 800;
        let height = 600;
        let mut buffer = vec![0u32; width * height];
        let rect = Rect::new(50.0, 50.0, 700.0, 500.0);
        let empty_set = std::collections::HashSet::new();

        for &t in &[
            ChartType::Bar,
            ChartType::Line,
            ChartType::Area,
            ChartType::Pie,
            ChartType::Donut,
        ] {
            draw_chart_visualizer(
                &mut buffer,
                width,
                height,
                rect,
                &chart,
                t,
                &empty_set,
                Some(1),
                Some((200.0, 200.0)),
                Some((0, 1)),
            );
        }
    }

    #[test]
    fn test_draw_chart_inspector_full() {
        let chart = make_sample_chart();
        let mut inspector = ChartInspectorState::new(chart, ChartType::Bar);
        inspector.show_toast("Export Complete");

        let width = 1024;
        let height = 768;
        let mut buffer = vec![0u32; width * height];

        draw_chart_inspector(&mut buffer, width, height, &inspector, 500.0, 400.0);
    }

    #[test]
    fn test_truncate_text_to_width() {
        assert_eq!(truncate_text_to_width("Short", 100), "Short");
        assert_eq!(
            truncate_text_to_width("A very long category name exceeding max width", 70),
            "A very ..."
        );
        assert_eq!(truncate_text_to_width("Tiny", 14), "Ti");
        assert_eq!(truncate_text_to_width("Zero", 0), "");
        assert_eq!(truncate_text_to_width("ExactFit", 8 * 7), "ExactFit");
    }

    #[test]
    fn test_volume_slider_geometry_and_hover_zone() {
        let (popup_rect, track_rect) = get_volume_slider_rects(1280, 720, false);
        assert!(popup_rect.width >= 30.0);
        assert!(popup_rect.height >= 120.0);
        assert!(popup_rect.contains(track_rect.x, track_rect.y));

        let hover_rect = get_volume_slider_hover_rect(1280, 720, false);
        // Hover rect must contain the popup
        assert!(hover_rect.contains(popup_rect.x, popup_rect.y));
        assert!(hover_rect.contains(
            popup_rect.x + popup_rect.width,
            popup_rect.y + popup_rect.height
        ));

        // Point right below popup (in former gap) must be inside hover rect
        let gap_y = popup_rect.y + popup_rect.height + 2.0;
        assert!(hover_rect.contains(popup_rect.x + popup_rect.width * 0.5, gap_y));

        // Hit testing volume slider
        let center_x = track_rect.x + track_rect.width * 0.5;
        let top_vol = hit_test_volume_slider(1280, 720, false, center_x, track_rect.y);
        assert_eq!(top_vol, Some(1.0));
        let bottom_vol =
            hit_test_volume_slider(1280, 720, false, center_x, track_rect.y + track_rect.height);
        assert_eq!(bottom_vol, Some(0.0));
    }

    #[test]
    fn test_dynamic_table_column_widths() {
        let mut chart = make_sample_chart();
        let (cat_w_short, s_w_short) = get_table_column_widths(400.0, &chart, 2);
        assert!(cat_w_short >= 85.0);
        assert!(s_w_short > 0.0);

        chart.categories = vec![
            "United States Eastern Seaboard Region".into(),
            "Europe & Middle East Central Operations".into(),
        ];
        let (cat_w_long, s_w_long) = get_table_column_widths(400.0, &chart, 2);
        assert!(cat_w_long > cat_w_short);
        assert!(cat_w_long <= 400.0 - 68.0 * 2.0);
        assert!(s_w_long >= 68.0);
        assert!((cat_w_long + s_w_long * 2.0 - 400.0).abs() < 0.1);
    }

    #[test]
    fn test_series_chip_widths_expansion() {
        let chart = make_sample_chart();
        let series_strip = Rect::new(50.0, 48.0, 900.0, 28.0);
        let chips = get_series_chip_rects(series_strip, &chart.series);
        assert_eq!(chips.len(), chart.series.len());

        for (s_idx, chip_rect) in &chips {
            let s = &chart.series[*s_idx];
            // Verify that chip width expands comfortably so text NEVER gets truncated with "..."
            let max_px = (chip_rect.width - 20.0).max(10.0) as usize;
            let chip_text = truncate_text_to_width(&s.name, max_px);
            assert_eq!(
                chip_text, s.name,
                "Series chip label should not be truncated when space is available!"
            );
            // Verify that chip width has expanded into available space to the right
            assert!(
                chip_rect.width >= 100.0,
                "Chip width should expand into available space"
            );
        }
    }

    #[test]
    fn test_unicode_and_emoji_rendering_in_hud() {
        let mut buffer = vec![0u32; 100 * 50];
        let width = 100;
        let height = 50;

        // Test rendering SORT ▼ and SORT ▲
        draw_text(&mut buffer, width, height, 0, 0, "SORT ▼", 0xFFFFFFFF);
        let non_zero_count = buffer.iter().filter(|&&p| p != 0).count();
        assert!(non_zero_count > 0, "SORT ▼ must render visible pixels");

        // Verify draw_text_centered with UTF-8 non-ASCII characters
        let rect = Rect::new(0.0, 0.0, 80.0, 20.0);
        let mut centered_buf = vec![0u32; 80 * 20];
        draw_text_centered(&mut centered_buf, 80, 20, rect, "SORT ▼", 0xFFFFFFFF);
        let centered_pixels = centered_buf.iter().filter(|&&p| p != 0).count();
        assert!(centered_pixels > 0);

        // Test presentation emojis and symbols render properly
        let mut emoji_buf = vec![0u32; 100 * 20];
        draw_text(
            &mut emoji_buf,
            100,
            20,
            0,
            0,
            "📊 📈 🚀 ⚡ ✨ 🗄 💡",
            0xFFFFFFFF,
        );
        assert!(emoji_buf.iter().filter(|&&p| p != 0).count() > 0);

        // Test unmapped character fallback produces a single dot, not a hollow box
        let mut single_char_buf = vec![0u32; 10 * 10];
        draw_char(&mut single_char_buf, 10, 10, 0, 0, '龍', 0xFFFFFFFF);
        let dot_count = single_char_buf.iter().filter(|&&p| p != 0).count();
        assert_eq!(
            dot_count, 1,
            "Unmapped character fallback should be a single dot"
        );
    }
}
