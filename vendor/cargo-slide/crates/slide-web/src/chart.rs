use crate::model::ChartData;
use crate::model::ChartType;
use crate::model::SeriesData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransformPreset {
    #[default]
    None,
    Top5,
    SortDesc,
    SortAsc,
    Cumulative,
    Percent100,
    MovingAvg3,
}

pub const PALETTE: [&str; 8] = [
    "#38bdf8", // Sky blue
    "#34d399", // Emerald green
    "#f59e0b", // Amber
    "#f43f5e", // Rose
    "#a855f7", // Purple
    "#6366f1", // Indigo
    "#06b6d4", // Cyan
    "#ec4899", // Pink
];

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ChartStats {
    pub total_sum: f64,
    pub avg: f64,
    pub median: f64,
    pub std_dev: f64,
    pub max_val: f64,
    pub max_cat: String,
    pub max_series: String,
    pub min_val: f64,
    pub min_cat: String,
    pub count: usize,
}

/// Calculate comprehensive summary statistics matching desktop player
pub fn calculate_stats(
    data: &ChartData,
    visible_series: &[String],
    category_indices: &[usize],
) -> ChartStats {
    let mut total_sum = 0.0f64;
    let mut max_val = f64::NEG_INFINITY;
    let mut min_val = f64::INFINITY;
    let mut max_c_idx = None;
    let mut max_s_idx = None;
    let mut min_c_idx = None;
    let mut all_vals = Vec::new();

    for (s_idx, s) in data.series.iter().enumerate() {
        if !visible_series.is_empty() && !visible_series.contains(&s.name) {
            continue;
        }
        for &c_idx in category_indices {
            if let Some(&val) = s.values.get(c_idx) {
                all_vals.push(val);
                total_sum += val;
                if val > max_val {
                    max_val = val;
                    max_c_idx = Some(c_idx);
                    max_s_idx = Some(s_idx);
                }
                if val < min_val {
                    min_val = val;
                    min_c_idx = Some(c_idx);
                }
            }
        }
    }

    let count = all_vals.len();
    let avg = if count > 0 {
        total_sum / count as f64
    } else {
        0.0
    };

    all_vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = if count == 0 {
        0.0
    } else if count % 2 == 0 {
        let mid = count / 2;
        let v1 = all_vals.get(mid.saturating_sub(1)).copied().unwrap_or(0.0);
        let v2 = all_vals.get(mid).copied().unwrap_or(0.0);
        (v1 + v2) / 2.0
    } else {
        all_vals.get(count / 2).copied().unwrap_or(0.0)
    };

    let variance = if count > 0 {
        all_vals
            .iter()
            .map(|&v| {
                let diff = v - avg;
                diff * diff
            })
            .sum::<f64>()
            / count as f64
    } else {
        0.0
    };
    let std_dev = variance.sqrt();

    let max_cat = max_c_idx
        .and_then(|idx| data.categories.get(idx))
        .cloned()
        .unwrap_or_default();
    let max_series = max_s_idx
        .and_then(|idx| data.series.get(idx))
        .map(|s| s.name.clone())
        .unwrap_or_default();
    let min_cat = min_c_idx
        .and_then(|idx| data.categories.get(idx))
        .cloned()
        .unwrap_or_default();

    ChartStats {
        total_sum,
        avg,
        median,
        std_dev,
        max_val: if max_val == f64::NEG_INFINITY {
            0.0
        } else {
            max_val
        },
        max_cat,
        max_series,
        min_val: if min_val == f64::INFINITY {
            0.0
        } else {
            min_val
        },
        min_cat,
        count,
    }
}

/// Parse filter query for numeric operators like >50, <=100, !=0
pub fn parse_filter_condition(query: &str) -> Option<(&str, &str)> {
    let q = query.trim();
    if let Some(rest) = q.strip_prefix(">=") {
        Some((">=", rest.trim()))
    } else if let Some(rest) = q.strip_prefix('>') {
        Some((">", rest.trim()))
    } else if let Some(rest) = q.strip_prefix("<=") {
        Some(("<=", rest.trim()))
    } else if let Some(rest) = q.strip_prefix('<') {
        Some(("<", rest.trim()))
    } else if let Some(rest) = q.strip_prefix("==") {
        Some(("==", rest.trim()))
    } else if let Some(rest) = q.strip_prefix('=') {
        Some(("=", rest.trim()))
    } else if let Some(rest) = q.strip_prefix("!=") {
        Some(("!=", rest.trim()))
    } else {
        None
    }
}

pub fn apply_transform(
    data: &ChartData,
    transform: TransformPreset,
) -> ChartData {
    match transform {
        | TransformPreset::None => data.clone(),
        | TransformPreset::Top5 => {
            let mut indexed_sums: Vec<(usize, f64)> = (0..data.categories.len())
                .map(|idx| {
                    let sum: f64 = data.series.iter().filter_map(|s| s.values.get(idx)).sum();
                    (idx, sum)
                })
                .collect();
            indexed_sums.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            indexed_sums.truncate(5);

            let new_categories = indexed_sums
                .iter()
                .map(|&(i, _)| data.categories.get(i).cloned().unwrap_or_default())
                .collect();
            let new_series = data
                .series
                .iter()
                .map(|s| {
                    let new_vals = indexed_sums
                        .iter()
                        .map(|&(i, _)| s.values.get(i).copied().unwrap_or(0.0))
                        .collect();
                    SeriesData {
                        name: s.name.clone(),
                        values: new_vals,
                        color: s.color.clone(),
                    }
                })
                .collect();

            ChartData {
                chart_type: data.chart_type,
                title: data.title.clone(),
                categories: new_categories,
                series: new_series,
                x_label: data.x_label.clone(),
                y_label: data.y_label.clone(),
                format: data.format,
                unit: data.unit.clone(),
                prefix: data.prefix.clone(),
                precision: data.precision,
            }
        },
        | TransformPreset::SortDesc | TransformPreset::SortAsc => {
            let is_desc = transform == TransformPreset::SortDesc;
            let mut indexed_sums: Vec<(usize, f64)> = (0..data.categories.len())
                .map(|idx| {
                    let sum: f64 = data.series.iter().filter_map(|s| s.values.get(idx)).sum();
                    (idx, sum)
                })
                .collect();
            if is_desc {
                indexed_sums
                    .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            } else {
                indexed_sums
                    .sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            }

            let new_categories = indexed_sums
                .iter()
                .map(|&(i, _)| data.categories.get(i).cloned().unwrap_or_default())
                .collect();
            let new_series = data
                .series
                .iter()
                .map(|s| {
                    let new_vals = indexed_sums
                        .iter()
                        .map(|&(i, _)| s.values.get(i).copied().unwrap_or(0.0))
                        .collect();
                    SeriesData {
                        name: s.name.clone(),
                        values: new_vals,
                        color: s.color.clone(),
                    }
                })
                .collect();

            ChartData {
                chart_type: data.chart_type,
                title: data.title.clone(),
                categories: new_categories,
                series: new_series,
                x_label: data.x_label.clone(),
                y_label: data.y_label.clone(),
                format: data.format,
                unit: data.unit.clone(),
                prefix: data.prefix.clone(),
                precision: data.precision,
            }
        },
        | TransformPreset::Cumulative => {
            let new_series = data
                .series
                .iter()
                .map(|s| {
                    let mut acc = 0.0;
                    let new_vals = s
                        .values
                        .iter()
                        .map(|&v| {
                            acc += v;
                            acc
                        })
                        .collect();
                    SeriesData {
                        name: s.name.clone(),
                        values: new_vals,
                        color: s.color.clone(),
                    }
                })
                .collect();

            ChartData {
                chart_type: data.chart_type,
                title: data.title.clone(),
                categories: data.categories.clone(),
                series: new_series,
                x_label: data.x_label.clone(),
                y_label: data.y_label.clone(),
                format: data.format,
                unit: data.unit.clone(),
                prefix: data.prefix.clone(),
                precision: data.precision,
            }
        },
        | TransformPreset::Percent100 => {
            let totals: Vec<f64> = (0..data.categories.len())
                .map(|idx| {
                    data.series
                        .iter()
                        .filter_map(|s| s.values.get(idx))
                        .sum::<f64>()
                        .max(0.0001)
                })
                .collect();

            let new_series = data
                .series
                .iter()
                .map(|s| {
                    let new_vals = s
                        .values
                        .iter()
                        .enumerate()
                        .map(|(idx, &v)| {
                            let tot = totals.get(idx).copied().unwrap_or(1.0);
                            (v / tot * 100.0).round()
                        })
                        .collect();
                    SeriesData {
                        name: s.name.clone(),
                        values: new_vals,
                        color: s.color.clone(),
                    }
                })
                .collect();

            ChartData {
                chart_type: data.chart_type,
                title: data.title.clone(),
                categories: data.categories.clone(),
                series: new_series,
                x_label: data.x_label.clone(),
                y_label: Some("%".to_string()),
                format: crate::model::NumberFormat::Percentage,
                unit: Some("%".to_string()),
                prefix: None,
                precision: Some(1),
            }
        },
        | TransformPreset::MovingAvg3 => {
            let new_series = data
                .series
                .iter()
                .map(|s| {
                    let new_vals = (0..s.values.len())
                        .map(|idx| {
                            let start = idx.saturating_sub(2);
                            let slice = &s.values[start..=idx];
                            let sum: f64 = slice.iter().sum();
                            sum / slice.len() as f64
                        })
                        .collect();
                    SeriesData {
                        name: s.name.clone(),
                        values: new_vals,
                        color: s.color.clone(),
                    }
                })
                .collect();

            ChartData {
                chart_type: data.chart_type,
                title: data.title.clone(),
                categories: data.categories.clone(),
                series: new_series,
                x_label: data.x_label.clone(),
                y_label: data.y_label.clone(),
                format: data.format,
                unit: data.unit.clone(),
                prefix: data.prefix.clone(),
                precision: data.precision,
            }
        },
    }
}

pub fn export_csv_string(
    data: &ChartData,
    visible_series: &[String],
    filtered_indices: &[usize],
) -> String {
    let mut csv = String::new();
    csv.push_str("Category");
    let active_series: Vec<&SeriesData> = data
        .series
        .iter()
        .filter(|s| visible_series.is_empty() || visible_series.contains(&s.name))
        .collect();

    for s in &active_series {
        csv.push(',');
        csv.push_str(&format!("\"{}\"", s.name.replace('"', "\"\"")));
    }
    csv.push('\n');

    for &cat_idx in filtered_indices {
        let cat = data.categories.get(cat_idx).cloned().unwrap_or_default();
        csv.push_str(&format!("\"{}\"", cat.replace('"', "\"\"")));
        for s in &active_series {
            csv.push(',');
            let v = s.values.get(cat_idx).copied().unwrap_or(0.0);
            csv.push_str(&v.to_string());
        }
        csv.push('\n');
    }

    csv
}

pub fn render_svg_chart(
    data: &ChartData,
    chart_type: ChartType,
    visible_series: &[String],
    filtered_indices: &[usize],
    width: f64,
    height: f64,
) -> String {
    let pad_left = 65.0;
    let pad_right = 25.0;
    let pad_top = 35.0;
    let pad_bottom = 60.0;

    let plot_w = (width - pad_left - pad_right).max(100.0);
    let plot_h = (height - pad_top - pad_bottom).max(100.0);

    let active_series: Vec<(usize, &SeriesData)> = data
        .series
        .iter()
        .enumerate()
        .filter(|(_, s)| visible_series.is_empty() || visible_series.contains(&s.name))
        .collect();

    let mut svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="100%" height="100%" class="chart-svg">
  <defs>
    <linearGradient id="chartCanvasBg" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="#141a24" stop-opacity="0.95"/>
      <stop offset="100%" stop-color="#0d1117" stop-opacity="0.98"/>
    </linearGradient>
  </defs>
  <rect x="{pad_left}" y="{pad_top}" width="{plot_w}" height="{plot_h}" fill="url(#chartCanvasBg)" rx="8" stroke="#30363d" stroke-width="1"/>
"##
    );

    // Pie & Donut Charts
    if chart_type == ChartType::Pie || chart_type == ChartType::Donut {
        let is_donut = chart_type == ChartType::Donut;
        let cx = pad_left + plot_w * 0.48;
        let cy = pad_top + plot_h * 0.50;
        let radius = (plot_h.min(plot_w) * 0.42).max(35.0);
        let inner_radius = if is_donut {
            radius * 0.54
        } else {
            0.0
        };

        let slice_values: Vec<(usize, String, f64)> = filtered_indices
            .iter()
            .map(|&c_idx| {
                let cat = data.categories.get(c_idx).cloned().unwrap_or_default();
                let sum: f64 = active_series
                    .iter()
                    .filter_map(|(_, s)| s.values.get(c_idx))
                    .sum();
                (c_idx, cat, sum.max(0.0))
            })
            .collect();

        let grand_total: f64 = slice_values.iter().map(|s| s.2).sum::<f64>().max(0.0001);

        let mut current_angle: f64 = -std::f64::consts::FRAC_PI_2; // Start from top
        for (i, &(_, ref cat, val)) in slice_values.iter().enumerate() {
            if val <= 0.0 {
                continue;
            }
            let slice_angle = (val / grand_total) * 2.0 * std::f64::consts::PI;
            let end_angle = current_angle + slice_angle;
            let color = PALETTE[i % PALETTE.len()];
            let pct = (val / grand_total * 100.0).clamp(0.0, 100.0);

            let x1 = cx + radius * current_angle.cos();
            let y1 = cy + radius * current_angle.sin();
            let x2 = cx + radius * end_angle.cos();
            let y2 = cy + radius * end_angle.sin();

            let large_arc = if slice_angle > std::f64::consts::PI {
                1
            } else {
                0
            };

            let path_d = if is_donut {
                let x1_in = cx + inner_radius * end_angle.cos();
                let y1_in = cy + inner_radius * end_angle.sin();
                let x2_in = cx + inner_radius * current_angle.cos();
                let y2_in = cy + inner_radius * current_angle.sin();
                format!(
                    "M {x1:.1} {y1:.1} A {radius:.1} {radius:.1} 0 {large_arc} 1 {x2:.1} {y2:.1} L {x1_in:.1} {y1_in:.1} A {inner_radius:.1} {inner_radius:.1} 0 {large_arc} 0 {x2_in:.1} {y2_in:.1} Z"
                )
            } else {
                format!(
                    "M {cx:.1} {cy:.1} L {x1:.1} {y1:.1} A {radius:.1} {radius:.1} 0 {large_arc} 1 {x2:.1} {y2:.1} Z"
                )
            };

            let formatted_val = data.format_number(val);
            svg.push_str(&format!(
                r##"  <path d="{path_d}" fill="{color}" opacity="0.88" stroke="#0d1117" stroke-width="2" class="pie-slice">
    <title>{cat}: {formatted_val} ({pct:.1}%)</title>
  </path>
"##
            ));

            // Percentage label on larger slices
            if pct >= 5.0 {
                let mid_angle = current_angle + slice_angle * 0.5;
                let label_r = if is_donut {
                    (radius + inner_radius) * 0.5
                } else {
                    radius * 0.65
                };
                let lx = cx + label_r * mid_angle.cos();
                let ly = cy + label_r * mid_angle.sin() + 4.0;
                svg.push_str(&format!(
                    r##"  <text x="{lx:.1}" y="{ly:.1}" fill="#ffffff" font-size="11" font-weight="bold" text-anchor="middle" pointer-events="none">{pct:.0}%</text>
"##
                ));
            }

            current_angle = end_angle;
        }

        // Central text inside Donut hole
        if is_donut {
            svg.push_str(&format!(
                r##"  <circle cx="{cx:.1}" cy="{cy:.1}" r="{inner_radius:.1}" fill="#0d1117"/>
  <text x="{cx:.1}" y="{:.1}" fill="#8b949e" font-size="10" font-weight="600" text-anchor="middle">TOTAL</text>
  <text x="{cx:.1}" y="{:.1}" fill="#58a6ff" font-size="14" font-weight="bold" text-anchor="middle">{}</text>
"##,
                cy - 3.0,
                cy + 15.0,
                data.format_number(grand_total)
            ));
        }

        svg.push_str("</svg>");
        return svg;
    }

    // Standard Cartesian Charts (Bar, Line, Area, Scatter)
    let mut max_y = 1.0f64;
    for (_, s) in &active_series {
        for &idx in filtered_indices {
            if let Some(&v) = s.values.get(idx)
                && v > max_y
            {
                max_y = v;
            }
        }
    }
    max_y = (max_y * 1.15).max(1.0);

    // Y Axis Grid lines
    let grid_steps = 4;
    for i in 0..=grid_steps {
        let val = max_y * (i as f64) / (grid_steps as f64);
        let y = pad_top + plot_h - (val / max_y * plot_h);
        let formatted = data.format_number(val);
        svg.push_str(&format!(
            r##"  <line x1="{pad_left}" y1="{y:.1}" x2="{:.1}" y2="{y:.1}" stroke="#21262d" stroke-dasharray="3,3" stroke-width="1"/>
  <text x="{:.1}" y="{:.1}" fill="#8b949e" font-size="11" text-anchor="end">{formatted}</text>
"##,
            pad_left + plot_w,
            pad_left - 8.0,
            y + 4.0
        ));
    }

    let cat_count = filtered_indices.len().max(1);
    let cat_step = plot_w / cat_count as f64;

    // X Axis Category Labels
    for (i, &cat_idx) in filtered_indices.iter().enumerate() {
        let cat = data.categories.get(cat_idx).cloned().unwrap_or_default();
        let x = pad_left + (i as f64 + 0.5) * cat_step;
        let y = pad_top + plot_h + 18.0;
        let display_cat = if cat.len() > 10 {
            format!("{}…", &cat[..8])
        } else {
            cat
        };
        svg.push_str(&format!(
            r##"  <text x="{x:.1}" y="{y:.1}" fill="#c9d1d9" font-size="11" text-anchor="middle">{display_cat}</text>
"##
        ));
    }

    match chart_type {
        | ChartType::Bar => {
            let num_s = active_series.len().max(1);
            let bar_group_w = (cat_step * 0.72).min(120.0);
            let single_bar_w = (bar_group_w / num_s as f64).max(3.0);

            for (s_local_idx, &(s_global_idx, s)) in active_series.iter().enumerate() {
                let color = s
                    .color
                    .as_deref()
                    .unwrap_or(PALETTE[s_global_idx % PALETTE.len()]);

                for (c_local_idx, &cat_idx) in filtered_indices.iter().enumerate() {
                    let val = s.values.get(cat_idx).copied().unwrap_or(0.0);
                    let cat = data.categories.get(cat_idx).cloned().unwrap_or_default();
                    let bar_h = (val / max_y * plot_h).max(0.0);
                    let x = pad_left
                        + c_local_idx as f64 * cat_step
                        + (cat_step - bar_group_w) / 2.0
                        + s_local_idx as f64 * single_bar_w;
                    let y = pad_top + plot_h - bar_h;
                    let formatted_val = data.format_number(val);

                    svg.push_str(&format!(
                        r##"  <rect x="{x:.1}" y="{y:.1}" width="{:.1}" height="{bar_h:.1}" fill="{color}" rx="3" opacity="0.9" class="chart-bar">
    <title>{cat} • {}: {formatted_val}</title>
  </rect>
"##,
                        (single_bar_w - 2.0).max(2.0),
                        s.name
                    ));
                }
            }
        },
        | ChartType::Line | ChartType::Area => {
            let is_area = chart_type == ChartType::Area;
            for &(s_global_idx, s) in &active_series {
                let color = s
                    .color
                    .as_deref()
                    .unwrap_or(PALETTE[s_global_idx % PALETTE.len()]);

                let mut points = Vec::new();
                for (c_local_idx, &cat_idx) in filtered_indices.iter().enumerate() {
                    let val = s.values.get(cat_idx).copied().unwrap_or(0.0);
                    let cat = data.categories.get(cat_idx).cloned().unwrap_or_default();
                    let x = pad_left + (c_local_idx as f64 + 0.5) * cat_step;
                    let y = pad_top + plot_h - (val / max_y * plot_h);
                    points.push((x, y, val, cat));
                }

                if points.is_empty() {
                    continue;
                }

                let path_d: String = points
                    .iter()
                    .enumerate()
                    .map(|(idx, (x, y, _, _))| {
                        if idx == 0 {
                            format!("M {x:.1} {y:.1}")
                        } else {
                            format!(" L {x:.1} {y:.1}")
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("");

                if is_area {
                    let first_x = points.first().unwrap().0;
                    let last_x = points.last().unwrap().0;
                    let baseline_y = pad_top + plot_h;
                    let area_d = format!(
                        "{path_d} L {last_x:.1} {baseline_y:.1} L {first_x:.1} {baseline_y:.1} Z"
                    );
                    svg.push_str(&format!(
                        r##"  <path d="{area_d}" fill="{color}" fill-opacity="0.22" class="chart-area"/>
"##
                    ));
                }

                svg.push_str(&format!(
                    r##"  <path d="{path_d}" fill="none" stroke="{color}" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" class="chart-line"/>
"##
                ));

                // Circles on data points
                for &(x, y, val, ref cat) in &points {
                    let formatted_val = data.format_number(val);
                    svg.push_str(&format!(
                        r##"  <circle cx="{x:.1}" cy="{y:.1}" r="5" fill="{color}" stroke="#0d1117" stroke-width="2" class="chart-dot">
    <title>{cat} • {}: {formatted_val}</title>
  </circle>
"##,
                        s.name
                    ));
                }
            }
        },
        | ChartType::Scatter => {
            for &(s_global_idx, s) in &active_series {
                let color = s
                    .color
                    .as_deref()
                    .unwrap_or(PALETTE[s_global_idx % PALETTE.len()]);

                for (c_local_idx, &cat_idx) in filtered_indices.iter().enumerate() {
                    let val = s.values.get(cat_idx).copied().unwrap_or(0.0);
                    let cat = data.categories.get(cat_idx).cloned().unwrap_or_default();
                    let x = pad_left + (c_local_idx as f64 + 0.5) * cat_step;
                    let y = pad_top + plot_h - (val / max_y * plot_h);
                    let formatted_val = data.format_number(val);

                    svg.push_str(&format!(
                        r##"  <circle cx="{x:.1}" cy="{y:.1}" r="6.5" fill="{color}" opacity="0.88" stroke="#ffffff" stroke-width="1.5" class="chart-scatter-dot">
    <title>{cat} • {}: {formatted_val}</title>
  </circle>
"##,
                        s.name
                    ));
                }
            }
        },
        | _ => {},
    }

    svg.push_str("</svg>");
    svg
}
