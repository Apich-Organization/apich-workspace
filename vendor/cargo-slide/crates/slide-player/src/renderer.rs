use slide_core::animation::SlideSurface;
use slide_core::error::Result;
use slide_core::error::SlideError;
use usvg::Options;
use usvg::Tree;

pub struct SvgRenderer {
    options: Options<'static>,
}

impl Default for SvgRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl SvgRenderer {
    pub fn new() -> Self {
        let mut opt = Options::default();
        opt.dpi = 72.0; // Typst SVG coordinates are in 72 DPI points (pt)
        opt.fontdb_mut().load_system_fonts();
        Self { options: opt }
    }

    /// Render an SVG string into a `SlideSurface` with specified dimensions, maintaining aspect ratio
    pub fn render_svg(
        &self,
        svg_content: &str,
        target_width: usize,
        target_height: usize,
    ) -> Result<(SlideSurface, RenderMetrics)> {
        let sanitized_svg = slide_core::svg::sanitize_svg(svg_content);
        let tree = Tree::from_str(&sanitized_svg, &self.options)
            .map_err(|e| SlideError::SvgParse(format!("usvg error: {}", e)))?;

        let (vb_x, vb_y, vb_w, vb_h) = match slide_core::svg::parse_svg_slide(svg_content) {
            | Ok(info) => {
                (
                    info.view_box.x,
                    info.view_box.y,
                    info.view_box.width,
                    info.view_box.height,
                )
            },
            | Err(_) => (0.0, 0.0, tree.size().width(), tree.size().height()),
        };

        let svg_w = if vb_w > 0.0 {
            vb_w
        } else {
            tree.size().width()
        };
        let svg_h = if vb_h > 0.0 {
            vb_h
        } else {
            tree.size().height()
        };

        if target_width == 0 || target_height == 0 || svg_w <= 0.0 || svg_h <= 0.0 {
            return Ok((
                SlideSurface::blank(target_width.max(1), target_height.max(1), 0xFF000000),
                RenderMetrics::default(),
            ));
        }

        // Calculate aspect-preserving scale and letterbox offset
        let scale_x = target_width as f32 / svg_w;
        let scale_y = target_height as f32 / svg_h;
        let scale = scale_x.min(scale_y);

        let content_w = (svg_w * scale).round() as usize;
        let content_h = (svg_h * scale).round() as usize;

        let offset_x = (target_width.saturating_sub(content_w)) / 2;
        let offset_y = (target_height.saturating_sub(content_h)) / 2;

        let mut pixmap =
            tiny_skia::Pixmap::new(content_w.max(1) as u32, content_h.max(1) as u32)
                .ok_or_else(|| SlideError::Format("Failed to allocate pixmap".to_string()))?;

        // Render SVG to pixmap with scaling
        let transform = tiny_skia::Transform::from_scale(scale, scale);
        resvg::render(&tree, transform, &mut pixmap.as_mut());

        let pixmap_data = pixmap.data();

        // Detect slide background color by sampling margin pixels (top, left, right margins)
        let detected_bg = {
            let sample_points = [
                (
                    (content_w / 2).min(content_w.saturating_sub(1)),
                    2.min(content_h.saturating_sub(1)),
                ),
                (
                    2.min(content_w.saturating_sub(1)),
                    2.min(content_h.saturating_sub(1)),
                ),
                (
                    content_w.saturating_sub(3),
                    2.min(content_h.saturating_sub(1)),
                ),
                (
                    2.min(content_w.saturating_sub(1)),
                    (content_h / 2).min(content_h.saturating_sub(1)),
                ),
            ];
            let mut bg = 0xFF0f111a;
            for (sx, sy) in sample_points {
                let idx = (sy * content_w + sx) * 4;
                if let (Some(&r), Some(&g), Some(&b), Some(&a)) = (
                    pixmap_data.get(idx),
                    pixmap_data.get(idx + 1),
                    pixmap_data.get(idx + 2),
                    pixmap_data.get(idx + 3),
                ) && a > 0
                {
                    bg = ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
                    break;
                }
            }
            bg
        };

        // Allocate surface buffer for full window and blit pixmap with seamless letterbox padding
        let total_pixels = target_width.saturating_mul(target_height);
        let mut surface_pixels = vec![detected_bg; total_pixels];

        let copy_w = content_w.min(target_width.saturating_sub(offset_x));
        let copy_bytes = copy_w.saturating_mul(4);

        for cy in 0..content_h {
            let win_y = offset_y.saturating_add(cy);
            if win_y >= target_height {
                break;
            }
            let win_row_start = win_y.saturating_mul(target_width).saturating_add(offset_x);
            let pix_row_start = cy.saturating_mul(content_w).saturating_mul(4);

            if let (Some(src_row), Some(dst_row)) = (
                pixmap_data.get(pix_row_start..pix_row_start.saturating_add(copy_bytes)),
                surface_pixels.get_mut(win_row_start..win_row_start.saturating_add(copy_w)),
            ) {
                for (&[r, g, b, a], pixel) in
                    src_row.as_chunks::<4>().0.iter().zip(dst_row.iter_mut())
                {
                    *pixel =
                        ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
                }
            }
        }

        let surface = SlideSurface::new(target_width, target_height, surface_pixels, detected_bg);
        let metrics = RenderMetrics {
            scale,
            offset_x: offset_x as f32,
            offset_y: offset_y as f32,
            content_width: content_w as f32,
            content_height: content_h as f32,
            view_box_x: vb_x,
            view_box_y: vb_y,
        };

        Ok((surface, metrics))
    }
}

pub use slide_core::model::RenderMetrics;
