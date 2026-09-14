use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub const fn new(
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) -> Self {
        Self { x, y, width, height }
    }

    pub fn contains(
        &self,
        px: f32,
        py: f32,
    ) -> bool {
        px >= self.x && px <= (self.x + self.width) && py >= self.y && py <= (self.y + self.height)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ChartType {
    #[default]
    #[serde(rename = "bar")]
    Bar,
    #[serde(rename = "line")]
    Line,
    #[serde(rename = "area")]
    Area,
    #[serde(rename = "pie")]
    Pie,
    #[serde(rename = "donut")]
    Donut,
    #[serde(rename = "scatter")]
    Scatter,
}

impl std::fmt::Display for ChartType {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            | Self::Bar => write!(f, "Bar"),
            | Self::Line => write!(f, "Line"),
            | Self::Area => write!(f, "Area"),
            | Self::Pie => write!(f, "Pie"),
            | Self::Donut => write!(f, "Donut"),
            | Self::Scatter => write!(f, "Scatter"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeriesData {
    pub name: String,
    pub values: Vec<f64>,
    #[serde(default)]
    pub color: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NumberFormat {
    #[default]
    Auto,
    Standard,
    Currency,
    Percentage,
    Compact,
    Scientific,
    Integer,
}

impl NumberFormat {
    pub fn cycle(self) -> Self {
        match self {
            | Self::Auto => Self::Currency,
            | Self::Currency => Self::Percentage,
            | Self::Percentage => Self::Compact,
            | Self::Compact => Self::Scientific,
            | Self::Scientific => Self::Integer,
            | Self::Integer => Self::Standard,
            | Self::Standard => Self::Auto,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            | Self::Auto => "Auto",
            | Self::Currency => "Currency ($)",
            | Self::Percentage => "Percent (%)",
            | Self::Compact => "Compact (K/M/B)",
            | Self::Scientific => "Scientific",
            | Self::Integer => "Integer",
            | Self::Standard => "Standard",
        }
    }
}

pub fn format_with_commas(
    val: f64,
    prec: usize,
) -> String {
    let is_neg = val < 0.0;
    let abs_val = val.abs();
    let s = format!("{abs_val:.prec$}");
    let parts: Vec<&str> = s.split('.').collect();
    let int_part = parts[0];
    let mut with_commas = String::new();
    let chars: Vec<char> = int_part.chars().collect();
    let len = chars.len();
    for (i, &ch) in chars.iter().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            with_commas.push(',');
        }
        with_commas.push(ch);
    }
    let mut out = if is_neg {
        format!("-{with_commas}")
    } else {
        with_commas
    };
    if prec > 0 && parts.len() > 1 {
        out.push('.');
        out.push_str(parts[1]);
    }
    out
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChartData {
    pub chart_type: ChartType,
    pub title: Option<String>,
    pub categories: Vec<String>,
    pub series: Vec<SeriesData>,
    #[serde(default)]
    pub x_label: Option<String>,
    #[serde(default)]
    pub y_label: Option<String>,
    #[serde(default)]
    pub format: NumberFormat,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub prefix: Option<String>,
    #[serde(default)]
    pub precision: Option<usize>,
}

impl ChartData {
    pub fn format_number(
        &self,
        val: f64,
    ) -> String {
        let prec = self.precision.unwrap_or(match self.format {
            | NumberFormat::Currency => 2,
            | NumberFormat::Percentage => 1,
            | NumberFormat::Integer => 0,
            | NumberFormat::Scientific => 2,
            | NumberFormat::Standard => 2,
            | NumberFormat::Compact | NumberFormat::Auto => 1,
        });

        let mut formatted = match self.format {
            | NumberFormat::Auto => {
                let abs = val.abs();
                if abs >= 1_000_000.0 {
                    format!("{:.1}M", val / 1_000_000.0)
                } else if abs >= 1_000.0 {
                    format!("{:.1}K", val / 1_000.0)
                } else if val.fract().abs() < 1e-4 {
                    format!("{:.0}", val)
                } else {
                    format!("{:.1}", val)
                }
            },
            | NumberFormat::Compact => {
                let abs = val.abs();
                if abs >= 1_000_000_000.0 {
                    format!("{:.prec$}B", val / 1_000_000_000.0)
                } else if abs >= 1_000_000.0 {
                    format!("{:.prec$}M", val / 1_000_000.0)
                } else if abs >= 1_000.0 {
                    format!("{:.prec$}K", val / 1_000.0)
                } else {
                    format_with_commas(val, prec)
                }
            },
            | NumberFormat::Currency => {
                let p = self.prefix.as_deref().unwrap_or("$");
                format!("{p}{}", format_with_commas(val, prec))
            },
            | NumberFormat::Percentage => {
                format!("{val:.prec$}%")
            },
            | NumberFormat::Integer => format_with_commas(val.round(), 0),
            | NumberFormat::Scientific => {
                format!("{val:.prec$e}")
            },
            | NumberFormat::Standard => format_with_commas(val, prec),
        };

        if let Some(ref p) = self.prefix
            && self.format != NumberFormat::Currency
            && !formatted.starts_with(p.as_str())
        {
            formatted = format!("{p}{formatted}");
        }

        if let Some(ref u) = self.unit
            && !formatted.ends_with(u.as_str())
        {
            if u.starts_with('%') || u.starts_with('°') {
                formatted.push_str(u);
            } else {
                formatted.push(' ');
                formatted.push_str(u);
            }
        }

        formatted
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Hotspot {
    #[serde(rename = "link")]
    Link { target: String, rect: Rect },
    #[serde(rename = "video")]
    Video {
        source: String,
        rect: Rect,
        caption: Option<String>,
    },
    #[serde(rename = "audio")]
    Audio {
        source: String,
        rect: Rect,
        title: Option<String>,
        autoplay: bool,
        loop_audio: bool,
        volume: f32,
    },
    #[serde(rename = "chart")]
    Chart { rect: Rect, data: ChartData },
}

impl Hotspot {
    pub const fn rect(&self) -> Rect {
        match self {
            | Self::Link { rect, .. }
            | Self::Video { rect, .. }
            | Self::Audio { rect, .. }
            | Self::Chart { rect, .. } => *rect,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StepFragment {
    pub order: usize,
    pub effect: String,
    pub rect: Rect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Slide {
    pub page_number: usize,
    pub svg_data: String,
    pub view_box: Rect,
    #[serde(default)]
    pub hotspots: Vec<Hotspot>,
    #[serde(default)]
    pub animation: Option<String>,
    #[serde(default)]
    pub steps: Vec<StepFragment>,
}

impl Slide {
    pub fn max_step(&self) -> usize {
        self.steps.iter().map(|s| s.order).max().unwrap_or(0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlideDeck {
    pub title: String,
    pub slides: Vec<Slide>,
    #[serde(default = "default_animation")]
    pub default_animation: String,
}

fn default_animation() -> String {
    "fade".to_string()
}
