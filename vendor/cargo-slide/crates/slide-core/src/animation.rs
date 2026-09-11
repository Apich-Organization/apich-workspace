use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// Represents a rendered slide surface (ARGB / 0xAARRGGBB format for fast blitting)
#[derive(Clone)]
pub struct SlideSurface {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>,
    pub bg_color: u32,
}

impl SlideSurface {
    #[must_use]
    pub const fn new(
        width: usize,
        height: usize,
        pixels: Vec<u32>,
        bg_color: u32,
    ) -> Self {
        Self {
            width,
            height,
            pixels,
            bg_color,
        }
    }

    #[must_use]
    pub fn blank(
        width: usize,
        height: usize,
        color: u32,
    ) -> Self {
        Self {
            width,
            height,
            pixels: vec![color; width * height],
            bg_color: color,
        }
    }

    #[must_use]
    pub fn get_pixel(
        &self,
        x: usize,
        y: usize,
    ) -> u32 {
        if x < self.width && y < self.height {
            self.pixels[y * self.width + x]
        } else {
            0
        }
    }

    #[inline(always)]
    pub fn set_pixel(
        &mut self,
        x: usize,
        y: usize,
        color: u32,
    ) {
        if x < self.width && y < self.height {
            self.pixels[y * self.width + x] = color;
        }
    }

    pub fn fill_rect(
        &mut self,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        color: u32,
    ) {
        let x2 = (x + w).min(self.width);
        let y2 = (y + h).min(self.height);
        for cy in y..y2 {
            let row = cy * self.width;
            for cx in x..x2 {
                self.pixels[row + cx] = color;
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn copy_rect(
        &mut self,
        src: &Self,
        src_x: usize,
        src_y: usize,
        dst_x: usize,
        dst_y: usize,
        w: usize,
        h: usize,
    ) {
        for row_idx in 0..h {
            let sy = src_y + row_idx;
            let dy = dst_y + row_idx;
            if sy >= src.height || dy >= self.height {
                break;
            }
            for col_idx in 0..w {
                let sx = src_x + col_idx;
                let dx = dst_x + col_idx;
                if sx >= src.width || dx >= self.width {
                    break;
                }
                self.pixels[dy * self.width + dx] = src.pixels[sy * src.width + sx];
            }
        }
    }

    /// Mask / blank out all steps whose order > `current_step`
    pub fn mask_steps(
        &mut self,
        steps: &[crate::model::StepFragment],
        current_step: usize,
        metrics: &crate::model::RenderMetrics,
        bg_color: u32,
    ) {
        for step in steps {
            if step.order > current_step {
                let screen_rect = metrics.svg_to_screen_rect(&step.rect);
                let sx1 = (screen_rect.x.max(0.0) as usize).min(self.width);
                let sy1 = (screen_rect.y.max(0.0) as usize).min(self.height);
                let sx2 = ((screen_rect.x + screen_rect.width).max(0.0) as usize).min(self.width);
                let sy2 = ((screen_rect.y + screen_rect.height).max(0.0) as usize).min(self.height);

                if sx1 < sx2 {
                    for y in sy1..sy2 {
                        let row = y.saturating_mul(self.width);
                        let start = row.saturating_add(sx1);
                        let end = row.saturating_add(sx2);
                        if let Some(slice) = self.pixels.get_mut(start..end) {
                            slice.fill(bg_color);
                        }
                    }
                }
            }
        }
    }

    pub fn blend_from_to(
        &mut self,
        from: Option<&Self>,
        to: &Self,
        progress: f32,
    ) {
        let t = progress.clamp(0.0, 1.0);
        let alpha_256 = (t * 256.0).round() as u32;
        let len = self.pixels.len().min(to.pixels.len());
        match from {
            | Some(f) => {
                let f_len = len.min(f.pixels.len());
                if let (Some(dst), Some(src_f), Some(src_to)) = (
                    self.pixels.get_mut(..f_len),
                    f.pixels.get(..f_len),
                    to.pixels.get(..f_len),
                ) {
                    for (d, (&p1, &p2)) in dst.iter_mut().zip(src_f.iter().zip(src_to.iter())) {
                        *d = blend_pixel_fast(p1, p2, alpha_256);
                    }
                }
            },
            | None => {
                if let (Some(dst), Some(src_to)) =
                    (self.pixels.get_mut(..len), to.pixels.get(..len))
                {
                    for (d, &p2) in dst.iter_mut().zip(src_to.iter()) {
                        *d = blend_pixel_fast(0xFF000000, p2, alpha_256);
                    }
                }
            },
        }
    }
}

/// Destination render context for transition frames
pub struct RenderContext<'a> {
    pub width: usize,
    pub height: usize,
    pub buffer: &'a mut [u32],
}

impl<'a> RenderContext<'a> {
    pub const fn new(
        width: usize,
        height: usize,
        buffer: &'a mut [u32],
    ) -> Self {
        Self {
            width,
            height,
            buffer,
        }
    }

    #[inline(always)]
    #[must_use]
    pub const fn get_pixel(
        &self,
        x: usize,
        y: usize,
    ) -> u32 {
        if x < self.width && y < self.height {
            self.buffer[y * self.width + x]
        } else {
            0
        }
    }

    #[inline(always)]
    pub const fn set_pixel(
        &mut self,
        x: usize,
        y: usize,
        color: u32,
    ) {
        if x < self.width && y < self.height {
            self.buffer[y * self.width + x] = color;
        }
    }

    pub fn clear(
        &mut self,
        color: u32,
    ) {
        self.buffer.fill(color);
    }

    pub fn fill_rect(
        &mut self,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        color: u32,
    ) {
        let x2 = (x.saturating_add(w)).min(self.width);
        let y2 = (y.saturating_add(h)).min(self.height);
        let start_x = x.min(self.width);
        if start_x < x2 {
            for cy in y.min(self.height)..y2 {
                let row = cy.saturating_mul(self.width);
                let start = row.saturating_add(start_x);
                let end = row.saturating_add(x2);
                if let Some(slice) = self.buffer.get_mut(start..end) {
                    slice.fill(color);
                }
            }
        }
    }

    pub fn copy_from(
        &mut self,
        src: &SlideSurface,
    ) {
        let len = self.buffer.len().min(src.pixels.len());
        if let (Some(dst), Some(src_slice)) = (self.buffer.get_mut(..len), src.pixels.get(..len)) {
            dst.copy_from_slice(src_slice);
        }
    }

    pub fn blend_from_to(
        &mut self,
        from: Option<&SlideSurface>,
        to: &SlideSurface,
        progress: f32,
    ) {
        let t = progress.clamp(0.0, 1.0);
        let alpha_256 = (t * 256.0).round() as u32;
        let len = self.buffer.len().min(to.pixels.len());
        match from {
            | Some(f) => {
                let f_len = len.min(f.pixels.len());
                if let (Some(dst), Some(src_f), Some(src_to)) = (
                    self.buffer.get_mut(..f_len),
                    f.pixels.get(..f_len),
                    to.pixels.get(..f_len),
                ) {
                    for (d, (&p1, &p2)) in dst.iter_mut().zip(src_f.iter().zip(src_to.iter())) {
                        *d = blend_pixel_fast(p1, p2, alpha_256);
                    }
                }
            },
            | None => {
                if let (Some(dst), Some(src_to)) =
                    (self.buffer.get_mut(..len), to.pixels.get(..len))
                {
                    for (d, &p2) in dst.iter_mut().zip(src_to.iter()) {
                        *d = blend_pixel_fast(0xFF000000, p2, alpha_256);
                    }
                }
            },
        }
    }
}

/// Trait for custom slide transition animations
pub trait SlideAnimation: Send + Sync {
    /// Identifier for this animation
    fn name(&self) -> &str;

    /// Transition duration (default 500ms)
    fn duration(&self) -> Duration {
        Duration::from_millis(500)
    }

    /// Render transition frame
    /// `progress` is in range [0.0, 1.0]
    fn render(
        &self,
        ctx: &mut RenderContext<'_>,
        from: Option<&SlideSurface>,
        to: &SlideSurface,
        progress: f32,
    );
}

/// Instant cut transition
pub struct InstantCut;
impl SlideAnimation for InstantCut {
    fn name(&self) -> &'static str {
        "cut"
    }

    fn duration(&self) -> Duration {
        Duration::from_millis(0)
    }

    fn render(
        &self,
        ctx: &mut RenderContext<'_>,
        _from: Option<&SlideSurface>,
        to: &SlideSurface,
        _progress: f32,
    ) {
        let copy_len = ctx.buffer.len().min(to.pixels.len());
        ctx.buffer[..copy_len].copy_from_slice(&to.pixels[..copy_len]);
    }
}

/// Smooth `CrossFade` transition
pub struct CrossFade;
impl SlideAnimation for CrossFade {
    fn name(&self) -> &'static str {
        "fade"
    }

    fn duration(&self) -> Duration {
        Duration::from_millis(400)
    }

    fn render(
        &self,
        ctx: &mut RenderContext<'_>,
        from: Option<&SlideSurface>,
        to: &SlideSurface,
        progress: f32,
    ) {
        let t = progress.clamp(0.0, 1.0);
        let alpha = (t * 255.0) as u32;
        let inv_alpha = 255 - alpha;

        let len = ctx.buffer.len().min(to.pixels.len());
        match from {
            | Some(from_surf) if from_surf.pixels.len() == to.pixels.len() => {
                for i in 0..len {
                    let c1 = from_surf.pixels[i];
                    let c2 = to.pixels[i];

                    let r = (((c1 >> 16) & 0xFF) * inv_alpha + ((c2 >> 16) & 0xFF) * alpha) / 255;
                    let g = (((c1 >> 8) & 0xFF) * inv_alpha + ((c2 >> 8) & 0xFF) * alpha) / 255;
                    let b = ((c1 & 0xFF) * inv_alpha + (c2 & 0xFF) * alpha) / 255;

                    ctx.buffer[i] = (0xFF << 24) | (r << 16) | (g << 8) | b;
                }
            },
            | _ => {
                // Just fade in target from black/background
                for i in 0..len {
                    let c2 = to.pixels[i];
                    let r = (((c2 >> 16) & 0xFF) * alpha) / 255;
                    let g = (((c2 >> 8) & 0xFF) * alpha) / 255;
                    let b = ((c2 & 0xFF) * alpha) / 255;

                    ctx.buffer[i] = (0xFF << 24) | (r << 16) | (g << 8) | b;
                }
            },
        }
    }
}

/// Slide Push from right to left
pub struct SlideLeft;
impl SlideAnimation for SlideLeft {
    fn name(&self) -> &'static str {
        "slide-left"
    }

    fn duration(&self) -> Duration {
        Duration::from_millis(450)
    }

    fn render(
        &self,
        ctx: &mut RenderContext<'_>,
        from: Option<&SlideSurface>,
        to: &SlideSurface,
        progress: f32,
    ) {
        let t = smooth_step(progress.clamp(0.0, 1.0));
        let width = ctx.width;
        let height = ctx.height;
        let offset_x = (t * width as f32) as usize;

        for y in 0..height {
            let row_start = y * width;
            for x in 0..width {
                let pixel = if x + offset_x < width {
                    if let Some(from_surf) = from {
                        from_surf.get_pixel(x + offset_x, y)
                    } else {
                        0
                    }
                } else {
                    let to_x = (x + offset_x) - width;
                    to.get_pixel(to_x, y)
                };
                ctx.buffer[row_start + x] = pixel;
            }
        }
    }
}

/// Slide Push from left to right
pub struct SlideRight;
impl SlideAnimation for SlideRight {
    fn name(&self) -> &'static str {
        "slide-right"
    }

    fn duration(&self) -> Duration {
        Duration::from_millis(450)
    }

    fn render(
        &self,
        ctx: &mut RenderContext<'_>,
        from: Option<&SlideSurface>,
        to: &SlideSurface,
        progress: f32,
    ) {
        let t = smooth_step(progress.clamp(0.0, 1.0));
        let width = ctx.width;
        let height = ctx.height;
        let offset_x = (t * width as f32) as usize;

        for y in 0..height {
            let row_start = y * width;
            for x in 0..width {
                let pixel = if x < offset_x {
                    let to_x = width - offset_x + x;
                    to.get_pixel(to_x, y)
                } else if let Some(from_surf) = from {
                    from_surf.get_pixel(x - offset_x, y)
                } else {
                    0
                };
                ctx.buffer[row_start + x] = pixel;
            }
        }
    }
}

/// Particle Dissolve transition
pub struct ParticleDissolve;
impl SlideAnimation for ParticleDissolve {
    fn name(&self) -> &'static str {
        "particles"
    }

    fn duration(&self) -> Duration {
        Duration::from_millis(600)
    }

    fn render(
        &self,
        ctx: &mut RenderContext<'_>,
        from: Option<&SlideSurface>,
        to: &SlideSurface,
        progress: f32,
    ) {
        let t = progress.clamp(0.0, 1.0);
        let width = ctx.width;
        let height = ctx.height;

        for y in 0..height {
            let row_start = y * width;
            for x in 0..width {
                // Fast deterministic pseudo-random hash based on coordinates
                let mut h = (x as u32).wrapping_mul(0x45d9f3b) ^ (y as u32).wrapping_mul(0x119de1f);
                h = ((h >> 16) ^ h).wrapping_mul(0x45d9f3b);
                h = (h >> 16) ^ h;
                let threshold = (h % 1000) as f32 / 1000.0;

                let pixel = if t > threshold {
                    to.get_pixel(x, y)
                } else if let Some(from_surf) = from {
                    from_surf.get_pixel(x, y)
                } else {
                    0
                };

                ctx.buffer[row_start + x] = pixel;
            }
        }
    }
}

/// Zoom / Radial Wipe transition
pub struct ZoomWipe;
impl SlideAnimation for ZoomWipe {
    fn name(&self) -> &'static str {
        "zoom"
    }

    fn duration(&self) -> Duration {
        Duration::from_millis(500)
    }

    fn render(
        &self,
        ctx: &mut RenderContext<'_>,
        from: Option<&SlideSurface>,
        to: &SlideSurface,
        progress: f32,
    ) {
        let t = smooth_step(progress.clamp(0.0, 1.0));
        let width = ctx.width;
        let height = ctx.height;
        let cx = width as f32 / 2.0;
        let cy = height as f32 / 2.0;
        let max_r = cx.hypot(cy);
        let cur_r = t * max_r;
        let cur_r2 = cur_r * cur_r;

        for y in 0..height {
            let dy = y as f32 - cy;
            let dy2 = dy * dy;
            let row_start = y * width;
            for x in 0..width {
                let dx = x as f32 - cx;
                let dist2 = dx * dx + dy2;

                let pixel = if dist2 <= cur_r2 {
                    to.get_pixel(x, y)
                } else if let Some(from_surf) = from {
                    from_surf.get_pixel(x, y)
                } else {
                    0
                };
                ctx.buffer[row_start + x] = pixel;
            }
        }
    }
}

#[inline]
#[must_use]
pub fn smooth_step(t: f32) -> f32 {
    let clamped = t.clamp(0.0, 1.0);
    clamped * clamped * 2.0f32.mul_add(-clamped, 3.0)
}

#[inline]
#[must_use]
pub fn ease_in_out(t: f32) -> f32 {
    smooth_step(t)
}

#[inline]
#[must_use]
pub fn ease_out_cubic(t: f32) -> f32 {
    let p = 1.0 - t.clamp(0.0, 1.0);
    (p * p).mul_add(-p, 1.0)
}

#[inline]
#[must_use]
pub fn ease_out_back(t: f32) -> f32 {
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    let p = t.clamp(0.0, 1.0) - 1.0;
    (c1 * p).mul_add(p, (c3 * p * p).mul_add(p, 1.0))
}

/// Slide Push from bottom to top
pub struct SlideUp;
impl SlideAnimation for SlideUp {
    fn name(&self) -> &'static str {
        "slide-up"
    }

    fn duration(&self) -> Duration {
        Duration::from_millis(450)
    }

    fn render(
        &self,
        ctx: &mut RenderContext<'_>,
        from: Option<&SlideSurface>,
        to: &SlideSurface,
        progress: f32,
    ) {
        let t = smooth_step(progress.clamp(0.0, 1.0));
        let width = ctx.width;
        let height = ctx.height;
        let offset_y = (t * height as f32) as usize;

        for y in 0..height {
            let row_start = y * width;
            let src_y = y + offset_y;
            for x in 0..width {
                let pixel = if src_y < height {
                    if let Some(from_surf) = from {
                        from_surf.get_pixel(x, src_y)
                    } else {
                        0
                    }
                } else {
                    let to_y = src_y - height;
                    to.get_pixel(x, to_y)
                };
                ctx.buffer[row_start + x] = pixel;
            }
        }
    }
}

/// Slide Push from top to bottom
pub struct SlideDown;
impl SlideAnimation for SlideDown {
    fn name(&self) -> &'static str {
        "slide-down"
    }

    fn duration(&self) -> Duration {
        Duration::from_millis(450)
    }

    fn render(
        &self,
        ctx: &mut RenderContext<'_>,
        from: Option<&SlideSurface>,
        to: &SlideSurface,
        progress: f32,
    ) {
        let t = smooth_step(progress.clamp(0.0, 1.0));
        let width = ctx.width;
        let height = ctx.height;
        let offset_y = (t * height as f32) as usize;

        for y in 0..height {
            let row_start = y * width;
            for x in 0..width {
                let pixel = if y < offset_y {
                    let to_y = height - offset_y + y;
                    to.get_pixel(x, to_y)
                } else if let Some(from_surf) = from {
                    from_surf.get_pixel(x, y - offset_y)
                } else {
                    0
                };
                ctx.buffer[row_start + x] = pixel;
            }
        }
    }
}

/// Linear soft wipe from right to left
pub struct WipeLeft;
impl SlideAnimation for WipeLeft {
    fn name(&self) -> &'static str {
        "wipe-left"
    }

    fn duration(&self) -> Duration {
        Duration::from_millis(500)
    }

    fn render(
        &self,
        ctx: &mut RenderContext<'_>,
        from: Option<&SlideSurface>,
        to: &SlideSurface,
        progress: f32,
    ) {
        let t = progress.clamp(0.0, 1.0);
        let width = ctx.width;
        let height = ctx.height;
        let feather = 30.0f32;
        let split_x = (1.0 - t).mul_add(feather.mul_add(2.0, width as f32), -feather);

        for y in 0..height {
            let row_start = y * width;
            for x in 0..width {
                let px = x as f32;
                let pixel = if px >= split_x + feather {
                    to.get_pixel(x, y)
                } else if px <= split_x - feather {
                    if let Some(from_surf) = from {
                        from_surf.get_pixel(x, y)
                    } else {
                        0
                    }
                } else {
                    let alpha = ((px - (split_x - feather)) / (feather * 2.0)).clamp(0.0, 1.0);
                    let c1 = from.map_or(0xFF000000, |f| f.get_pixel(x, y));
                    let c2 = to.get_pixel(x, y);
                    blend_pixel(c1, c2, alpha)
                };
                ctx.buffer[row_start + x] = pixel;
            }
        }
    }
}

/// Linear soft wipe from left to right
pub struct WipeRight;
impl SlideAnimation for WipeRight {
    fn name(&self) -> &'static str {
        "wipe-right"
    }

    fn duration(&self) -> Duration {
        Duration::from_millis(500)
    }

    fn render(
        &self,
        ctx: &mut RenderContext<'_>,
        from: Option<&SlideSurface>,
        to: &SlideSurface,
        progress: f32,
    ) {
        let t = progress.clamp(0.0, 1.0);
        let width = ctx.width;
        let height = ctx.height;
        let feather = 30.0f32;
        let split_x = t * feather.mul_add(2.0, width as f32) - feather;

        for y in 0..height {
            let row_start = y * width;
            for x in 0..width {
                let px = x as f32;
                let pixel = if px <= split_x - feather {
                    to.get_pixel(x, y)
                } else if px >= split_x + feather {
                    if let Some(from_surf) = from {
                        from_surf.get_pixel(x, y)
                    } else {
                        0
                    }
                } else {
                    let alpha = ((split_x + feather - px) / (feather * 2.0)).clamp(0.0, 1.0);
                    let c1 = from.map_or(0xFF000000, |f| f.get_pixel(x, y));
                    let c2 = to.get_pixel(x, y);
                    blend_pixel(c1, c2, alpha)
                };
                ctx.buffer[row_start + x] = pixel;
            }
        }
    }
}

/// Circular iris aperture reveal transition
pub struct IrisWipe;
impl SlideAnimation for IrisWipe {
    fn name(&self) -> &'static str {
        "iris"
    }

    fn duration(&self) -> Duration {
        Duration::from_millis(550)
    }

    fn render(
        &self,
        ctx: &mut RenderContext<'_>,
        from: Option<&SlideSurface>,
        to: &SlideSurface,
        progress: f32,
    ) {
        let t = smooth_step(progress.clamp(0.0, 1.0));
        let width = ctx.width;
        let height = ctx.height;
        let cx = width as f32 / 2.0;
        let cy = height as f32 / 2.0;
        let max_r = cx.hypot(cy);
        let cur_r = t * (max_r + 20.0);
        let feather = 18.0f32;

        for y in 0..height {
            let dy = y as f32 - cy;
            let dy2 = dy * dy;
            let row_start = y * width;
            for x in 0..width {
                let dx = x as f32 - cx;
                let dist = (dx * dx + dy2).sqrt();

                let pixel = if dist <= cur_r - feather {
                    to.get_pixel(x, y)
                } else if dist >= cur_r + feather {
                    if let Some(from_surf) = from {
                        from_surf.get_pixel(x, y)
                    } else {
                        0
                    }
                } else {
                    let alpha = ((cur_r + feather - dist) / (feather * 2.0)).clamp(0.0, 1.0);
                    let c1 = from.map_or(0xFF000000, |f| f.get_pixel(x, y));
                    let c2 = to.get_pixel(x, y);
                    blend_pixel(c1, c2, alpha)
                };
                ctx.buffer[row_start + x] = pixel;
            }
        }
    }
}

/// Cyberpunk digital glitch scanline transition with RGB chromatic displacement
pub struct CyberGlitch;
impl SlideAnimation for CyberGlitch {
    fn name(&self) -> &'static str {
        "glitch"
    }

    fn duration(&self) -> Duration {
        Duration::from_millis(450)
    }

    fn render(
        &self,
        ctx: &mut RenderContext<'_>,
        from: Option<&SlideSurface>,
        to: &SlideSurface,
        progress: f32,
    ) {
        let t = progress.clamp(0.0, 1.0);
        let width = ctx.width;
        let height = ctx.height;

        // Glitch intensity peaks at mid-transition
        let intensity = (1.0 - (t * 2.0 - 1.0).abs()).max(0.0);

        let slice_height = 8usize;
        let num_slices = height.div_ceil(slice_height);

        for s in 0..num_slices {
            let y_start = s * slice_height;
            let y_end = (y_start + slice_height).min(height);

            // Pseudo-random shift based on slice index and progress frame
            let frame = (t * 20.0) as u32;
            let mut h = (s as u32).wrapping_mul(0x9E3779B9) ^ frame.wrapping_mul(0x85EBCA6B);
            h = ((h >> 16) ^ h).wrapping_mul(0xC2B2AE35);
            h = (h >> 16) ^ h;

            let is_glitched = (h % 100) as f32 / 100.0 < (intensity * 0.85);
            let shift_x = if is_glitched {
                (((h % 61) as i32) - 30) as f32 * intensity
            } else {
                0.0
            };

            let choose_to = if t > 0.65 {
                true
            } else if t < 0.35 {
                false
            } else {
                h.is_multiple_of(2)
            };

            for y in y_start..y_end {
                let row_start = y * width;
                let is_scanline = (y % 4) == 0 && intensity > 0.2;

                for x in 0..width {
                    let rx = ((x as f32 + shift_x * 1.5).round() as isize)
                        .clamp(0, width as isize - 1) as usize;
                    let gx = ((x as f32 + shift_x).round() as isize).clamp(0, width as isize - 1)
                        as usize;
                    let bx = ((x as f32 - shift_x * 1.2).round() as isize)
                        .clamp(0, width as isize - 1) as usize;

                    let (base_surf, fallback_surf) = if choose_to {
                        (to, from)
                    } else {
                        match from {
                            | Some(f) => (f, Some(to)),
                            | None => (to, None),
                        }
                    };

                    let c_r = base_surf.get_pixel(rx, y);
                    let c_g = base_surf.get_pixel(gx, y);
                    let c_b = fallback_surf.unwrap_or(base_surf).get_pixel(bx, y);

                    let r = (c_r >> 16) & 0xFF;
                    let g = (c_g >> 8) & 0xFF;
                    let b = c_b & 0xFF;

                    let pixel = if is_scanline {
                        (0xFF << 24) | ((r * 7 / 10) << 16) | ((g * 7 / 10) << 8) | (b * 7 / 10)
                    } else {
                        (0xFF << 24) | (r << 16) | (g << 8) | b
                    };

                    ctx.buffer[row_start + x] = pixel;
                }
            }
        }
    }
}

/// Simulated 3D cube rotation transition
pub struct Cube3D;
impl SlideAnimation for Cube3D {
    fn name(&self) -> &'static str {
        "cube"
    }

    fn duration(&self) -> Duration {
        Duration::from_millis(500)
    }

    fn render(
        &self,
        ctx: &mut RenderContext<'_>,
        from: Option<&SlideSurface>,
        to: &SlideSurface,
        progress: f32,
    ) {
        let t = smooth_step(progress.clamp(0.0, 1.0));
        let width = ctx.width;
        let height = ctx.height;

        let w_f = width as f32;
        let split = (1.0 - t) * w_f;

        for y in 0..height {
            let row_start = y * width;
            for x in 0..width {
                let px = x as f32;
                let pixel = if px < split {
                    // Leaving face: compresses horizontally towards left
                    if let Some(from_surf) = from {
                        let scale = (1.0f32 - t).max(0.001f32);
                        let orig_x = ((px / scale).round() as usize).min(width.saturating_sub(1));
                        let col = from_surf.get_pixel(orig_x, y);
                        // Dim slightly for 3D lighting falloff
                        dim_pixel(col, 0.6f32.mul_add(1.0 - t, 0.4))
                    } else {
                        0
                    }
                } else {
                    // Incoming face: expands from right towards left
                    let scale = t.max(0.001f32);
                    let local_x = px - split;
                    let orig_x = ((local_x / scale).round() as usize).min(width.saturating_sub(1));
                    let col = to.get_pixel(orig_x, y);
                    dim_pixel(col, 0.6f32.mul_add(t, 0.4))
                };
                ctx.buffer[row_start + x] = pixel;
            }
        }
    }
}

#[inline(always)]
#[must_use]
pub fn blend_pixel(
    c1: u32,
    c2: u32,
    alpha: f32,
) -> u32 {
    let a = (alpha.clamp(0.0, 1.0) * 255.0) as u32;
    let inv_a = 255 - a;

    let r = (((c1 >> 16) & 0xFF) * inv_a + ((c2 >> 16) & 0xFF) * a) / 255;
    let g = (((c1 >> 8) & 0xFF) * inv_a + ((c2 >> 8) & 0xFF) * a) / 255;
    let b = ((c1 & 0xFF) * inv_a + (c2 & 0xFF) * a) / 255;

    (0xFF << 24) | (r << 16) | (g << 8) | b
}

/// Blazing fast integer alpha blending using bit-shifts (0..256 alpha, zero divisions)
#[inline(always)]
#[must_use]
pub fn blend_pixel_fast(
    c1: u32,
    c2: u32,
    alpha_256: u32,
) -> u32 {
    let a = alpha_256.min(256);
    let inv_a = 256 - a;

    let r = (((c1 >> 16) & 0xFF) * inv_a + ((c2 >> 16) & 0xFF) * a) >> 8;
    let g = (((c1 >> 8) & 0xFF) * inv_a + ((c2 >> 8) & 0xFF) * a) >> 8;
    let b = ((c1 & 0xFF) * inv_a + (c2 & 0xFF) * a) >> 8;

    (0xFF << 24) | (r << 16) | (g << 8) | b
}

#[inline(always)]
#[must_use]
pub fn dim_pixel(
    color: u32,
    factor: f32,
) -> u32 {
    let f = factor.clamp(0.0, 1.0);
    let r = (((color >> 16) & 0xFF) as f32 * f) as u32;
    let g = (((color >> 8) & 0xFF) as f32 * f) as u32;
    let b = ((color & 0xFF) as f32 * f) as u32;
    (0xFF << 24) | (r << 16) | (g << 8) | b
}

/// Trait for in-slide component animation effects (fragments / incremental builds)
pub trait ComponentAnimation: Send + Sync {
    fn name(&self) -> &str;

    fn duration(&self) -> Duration {
        Duration::from_millis(300)
    }

    /// Render in-slide component animation
    /// `progress`: 0.0 (hidden/start) to 1.0 (fully revealed)
    /// `screen_rect`: bounding box on window screen coordinates
    /// `full_slide`: rendered complete slide surface
    /// `output`: target frame buffer
    #[allow(clippy::too_many_arguments)]
    fn render(
        &self,
        progress: f32,
        screen_rect: crate::model::Rect,
        full_slide: &SlideSurface,
        output: &mut [u32],
        width: usize,
        height: usize,
        bg_color: u32,
    );
}

/// Component smooth fade-in
pub struct ComponentFadeIn;
impl ComponentAnimation for ComponentFadeIn {
    fn name(&self) -> &'static str {
        "fade-in"
    }

    fn render(
        &self,
        progress: f32,
        rect: crate::model::Rect,
        full_slide: &SlideSurface,
        output: &mut [u32],
        width: usize,
        height: usize,
        bg_color: u32,
    ) {
        let t = progress.clamp(0.0, 1.0);
        let x1 = (rect.x.max(0.0) as usize).min(width);
        let y1 = (rect.y.max(0.0) as usize).min(height);
        let x2 = ((rect.x + rect.width).max(0.0) as usize).min(width);
        let y2 = ((rect.y + rect.height).max(0.0) as usize).min(height);

        for y in y1..y2 {
            let row = y * width;
            for x in x1..x2 {
                let target_pixel = full_slide.get_pixel(x, y);
                output[row + x] = blend_pixel(bg_color, target_pixel, t);
            }
        }
    }
}

/// Component slide up and reveal
pub struct ComponentSlideUp;
impl ComponentAnimation for ComponentSlideUp {
    fn name(&self) -> &'static str {
        "slide-up"
    }

    fn render(
        &self,
        progress: f32,
        rect: crate::model::Rect,
        full_slide: &SlideSurface,
        output: &mut [u32],
        width: usize,
        height: usize,
        bg_color: u32,
    ) {
        let t = smooth_step(progress.clamp(0.0, 1.0));
        let x1 = (rect.x.max(0.0) as usize).min(width);
        let y1 = (rect.y.max(0.0) as usize).min(height);
        let x2 = ((rect.x + rect.width).max(0.0) as usize).min(width);
        let y2 = ((rect.y + rect.height).max(0.0) as usize).min(height);

        let max_offset = 24.0f32;
        let cur_offset = ((1.0 - t) * max_offset).round() as usize;

        for y in y1..y2 {
            let row = y * width;
            for x in x1..x2 {
                if y + cur_offset < y2 {
                    let target_pixel = full_slide.get_pixel(x, y + cur_offset);
                    output[row + x] = blend_pixel(bg_color, target_pixel, t);
                } else {
                    output[row + x] = bg_color;
                }
            }
        }
    }
}

/// Component scale zoom-in
pub struct ComponentZoomIn;
impl ComponentAnimation for ComponentZoomIn {
    fn name(&self) -> &'static str {
        "zoom-in"
    }

    fn render(
        &self,
        progress: f32,
        rect: crate::model::Rect,
        full_slide: &SlideSurface,
        output: &mut [u32],
        width: usize,
        height: usize,
        bg_color: u32,
    ) {
        let t = smooth_step(progress.clamp(0.0, 1.0));
        let x1 = (rect.x.max(0.0) as usize).min(width);
        let y1 = (rect.y.max(0.0) as usize).min(height);
        let x2 = ((rect.x + rect.width).max(0.0) as usize).min(width);
        let y2 = ((rect.y + rect.height).max(0.0) as usize).min(height);

        let cx = (x1 + x2) as f32 / 2.0;
        let cy = (y1 + y2) as f32 / 2.0;
        let scale = 0.2f32.mul_add(t, 0.8);

        for y in y1..y2 {
            let row = y * width;
            let dy = y as f32 - cy;
            for x in x1..x2 {
                let dx = x as f32 - cx;
                let src_x = ((cx + dx / scale).round() as usize).clamp(x1, x2.saturating_sub(1));
                let src_y = ((cy + dy / scale).round() as usize).clamp(y1, y2.saturating_sub(1));
                let target_pixel = full_slide.get_pixel(src_x, src_y);
                output[row + x] = blend_pixel(bg_color, target_pixel, t);
            }
        }
    }
}

/// Component digital scanline glitch reveal
pub struct ComponentGlitch;
impl ComponentAnimation for ComponentGlitch {
    fn name(&self) -> &'static str {
        "glitch"
    }

    fn duration(&self) -> Duration {
        Duration::from_millis(250)
    }

    fn render(
        &self,
        progress: f32,
        rect: crate::model::Rect,
        full_slide: &SlideSurface,
        output: &mut [u32],
        width: usize,
        height: usize,
        bg_color: u32,
    ) {
        let t = progress.clamp(0.0, 1.0);
        let x1 = (rect.x.max(0.0) as usize).min(width);
        let y1 = (rect.y.max(0.0) as usize).min(height);
        let x2 = ((rect.x + rect.width).max(0.0) as usize).min(width);
        let y2 = ((rect.y + rect.height).max(0.0) as usize).min(height);

        let intensity = 1.0 - t;

        for y in y1..y2 {
            let row = y * width;
            let jitter = if intensity > 0.1 && (y % 3) == 0 {
                ((((y as u32).wrapping_mul(0x9E3779B9) % 21) as i32) - 10) as f32 * intensity
            } else {
                0.0
            };

            for x in x1..x2 {
                let jx = ((x as f32 + jitter).round() as usize).clamp(x1, x2.saturating_sub(1));
                let target_pixel = full_slide.get_pixel(jx, y);
                output[row + x] = blend_pixel(bg_color, target_pixel, t);
            }
        }
    }
}

/// Component left-to-right wipe
pub struct ComponentWipe;
impl ComponentAnimation for ComponentWipe {
    fn name(&self) -> &'static str {
        "wipe"
    }

    fn render(
        &self,
        progress: f32,
        rect: crate::model::Rect,
        full_slide: &SlideSurface,
        output: &mut [u32],
        width: usize,
        height: usize,
        bg_color: u32,
    ) {
        let t = progress.clamp(0.0, 1.0);
        let x1 = (rect.x.max(0.0) as usize).min(width);
        let y1 = (rect.y.max(0.0) as usize).min(height);
        let x2 = ((rect.x + rect.width).max(0.0) as usize).min(width);
        let y2 = ((rect.y + rect.height).max(0.0) as usize).min(height);

        let split_x = rect.x + t * rect.width;
        let feather = 10.0f32;

        for y in y1..y2 {
            let row = y * width;
            for x in x1..x2 {
                let px = x as f32;
                if px <= split_x - feather {
                    output[row + x] = full_slide.get_pixel(x, y);
                } else if px >= split_x + feather {
                    output[row + x] = bg_color;
                } else {
                    let alpha = ((split_x + feather - px) / (feather * 2.0)).clamp(0.0, 1.0);
                    let target_pixel = full_slide.get_pixel(x, y);
                    output[row + x] = blend_pixel(bg_color, target_pixel, alpha);
                }
            }
        }
    }
}

pub use SlideAnimation as SlideTransition;

/// Registry of available slide transitions and in-slide component animations
#[derive(Clone)]
pub struct AnimationRegistry {
    animations: HashMap<String, Arc<dyn SlideAnimation>>,
    component_animations: HashMap<String, Arc<dyn ComponentAnimation>>,
}

impl Default for AnimationRegistry {
    fn default() -> Self {
        let mut registry = Self::new();
        // Page transitions
        registry.register(InstantCut);
        registry.register(CrossFade);
        registry.register(SlideLeft);
        registry.register(SlideRight);
        registry.register(SlideUp);
        registry.register(SlideDown);
        registry.register(ZoomWipe);
        registry.register(WipeLeft);
        registry.register(WipeRight);
        registry.register(IrisWipe);
        registry.register(CyberGlitch);
        registry.register(Cube3D);
        registry.register(ParticleDissolve);

        // Component fragment animations
        registry.register_component(ComponentFadeIn);
        registry.register_component(ComponentSlideUp);
        registry.register_component(ComponentZoomIn);
        registry.register_component(ComponentGlitch);
        registry.register_component(ComponentWipe);

        registry
    }
}

impl AnimationRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            animations: HashMap::new(),
            component_animations: HashMap::new(),
        }
    }

    /// Register a slide transition animation
    pub fn register(
        &mut self,
        anim: impl SlideAnimation + 'static,
    ) {
        self.animations
            .insert(anim.name().to_string(), Arc::new(anim));
    }

    /// Retrieve a slide transition animation by name
    #[must_use]
    pub fn get(
        &self,
        name: &str,
    ) -> Option<Arc<dyn SlideAnimation>> {
        self.animations.get(name).cloned()
    }

    /// Register an in-slide component animation
    pub fn register_component(
        &mut self,
        comp: impl ComponentAnimation + 'static,
    ) {
        self.component_animations
            .insert(comp.name().to_string(), Arc::new(comp));
    }

    /// Retrieve an in-slide component animation by name
    #[must_use]
    pub fn get_component(
        &self,
        name: &str,
    ) -> Option<Arc<dyn ComponentAnimation>> {
        self.component_animations.get(name).cloned()
    }
}
