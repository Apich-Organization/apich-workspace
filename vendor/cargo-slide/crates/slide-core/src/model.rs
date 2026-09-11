use serde::Deserialize;
use serde::Serialize;

/// 2D Rectangle in slide / SVG coordinates
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    #[must_use]
    pub const fn new(
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) -> Self {
        Self { x, y, width, height }
    }

    #[must_use]
    pub fn contains(
        &self,
        px: f32,
        py: f32,
    ) -> bool {
        px >= self.x && px <= (self.x + self.width) && py >= self.y && py <= (self.y + self.height)
    }
}

/// Information used to map screen coordinates back to SVG coordinates
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct RenderMetrics {
    pub scale: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub content_width: f32,
    pub content_height: f32,
    pub view_box_x: f32,
    pub view_box_y: f32,
}

impl RenderMetrics {
    /// Convert window screen coordinate (px, py) to SVG coordinate space
    #[must_use]
    pub fn screen_to_svg(
        &self,
        screen_x: f32,
        screen_y: f32,
    ) -> Option<(f32, f32)> {
        if self.scale <= 0.0 {
            return None;
        }

        if screen_x < self.offset_x
            || screen_x > (self.offset_x + self.content_width)
            || screen_y < self.offset_y
            || screen_y > (self.offset_y + self.content_height)
        {
            return None;
        }

        let svg_x = self.view_box_x + (screen_x - self.offset_x) / self.scale;
        let svg_y = self.view_box_y + (screen_y - self.offset_y) / self.scale;
        Some((svg_x, svg_y))
    }

    /// Convert SVG coordinate (x, y) to window screen coordinates
    #[must_use]
    pub fn svg_to_screen(
        &self,
        svg_x: f32,
        svg_y: f32,
    ) -> (f32, f32) {
        (
            (svg_x - self.view_box_x).mul_add(self.scale, self.offset_x),
            (svg_y - self.view_box_y).mul_add(self.scale, self.offset_y),
        )
    }

    /// Convert SVG `Rect` to window screen coordinate `Rect`
    #[must_use]
    pub fn svg_to_screen_rect(
        &self,
        rect: &Rect,
    ) -> Rect {
        let (x, y) = self.svg_to_screen(rect.x, rect.y);
        Rect {
            x,
            y,
            width: rect.width * self.scale,
            height: rect.height * self.scale,
        }
    }
}

/// Interactive hotspot on a slide
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Hotspot {
    /// Web hyperlink or internal page jump
    #[serde(rename = "link")]
    Link { target: String, rect: Rect },
    /// Embedded video placeholder to be played with `FFmpeg`
    #[serde(rename = "video")]
    Video {
        source: String,
        rect: Rect,
        caption: Option<String>,
    },
    /// Embedded audio player or background music
    #[serde(rename = "audio")]
    Audio {
        source: String,
        rect: Rect,
        title: Option<String>,
        autoplay: bool,
        loop_audio: bool,
        volume: f32,
    },
    /// Interactive data chart (Bar, Line, Area, Pie, Donut, Scatter)
    #[serde(rename = "chart")]
    Chart {
        rect: Rect,
        data: crate::chart::ChartData,
    },
}

impl Hotspot {
    #[must_use]
    pub const fn rect(&self) -> Rect {
        match self {
            | Self::Link { rect, .. } => *rect,
            | Self::Video { rect, .. } => *rect,
            | Self::Audio { rect, .. } => *rect,
            | Self::Chart { rect, .. } => *rect,
        }
    }

    #[must_use]
    pub fn contains(
        &self,
        px: f32,
        py: f32,
    ) -> bool {
        self.rect().contains(px, py)
    }
}

/// An in-slide incremental build / component step fragment
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StepFragment {
    pub order: usize,
    pub effect: String,
    pub rect: Rect,
}

/// A single presentation slide
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Slide {
    pub page_number: usize,
    pub svg_data: String,
    pub view_box: Rect,
    pub hotspots: Vec<Hotspot>,
    pub animation: Option<String>,
    #[serde(default)]
    pub steps: Vec<StepFragment>,
}

impl Slide {
    /// Total number of distinct in-slide build steps (0 if slide is static)
    #[must_use]
    pub fn max_step(&self) -> usize {
        self.steps.iter().map(|s| s.order).max().unwrap_or(0)
    }
}

/// Complete presentation deck
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlideDeck {
    pub title: String,
    pub slides: Vec<Slide>,
    pub default_animation: String,
}

impl SlideDeck {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            slides: Vec::new(),
            default_animation: "fade".to_string(),
        }
    }

    #[must_use]
    pub const fn total_slides(&self) -> usize {
        self.slides.len()
    }

    #[must_use]
    pub fn get_slide(
        &self,
        index: usize,
    ) -> Option<&Slide> {
        self.slides.get(index)
    }
}
