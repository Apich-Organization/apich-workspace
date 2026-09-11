use crate::error::Result;
use crate::error::SlideError;
use crate::model::Hotspot;
use crate::model::Rect;
use crate::model::StepFragment;
use roxmltree::Document;
use roxmltree::Node;
use std::path::Path;
use std::str::FromStr;
use svgtypes::Transform;

/// Parsed SVG slide metadata
#[derive(Debug, Clone)]
pub struct SvgSlideInfo {
    pub view_box: Rect,
    pub hotspots: Vec<Hotspot>,
    pub steps: Vec<StepFragment>,
    pub transition: Option<String>,
}

/// Sanitize an SVG string by escaping bare `&` characters so XML parsers (usvg, roxmltree) do not fail with malformed entity reference errors
#[must_use]
pub fn sanitize_svg(svg: &str) -> String {
    let mut result = String::with_capacity(svg.len());
    let mut chars = svg.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '&' {
            let mut entity = String::new();
            let mut is_valid = false;
            let lookahead = chars.clone();
            for c in lookahead {
                if c == ';' {
                    is_valid = true;
                    break;
                }
                if c.is_alphanumeric() || c == '#' {
                    entity.push(c);
                    if entity.len() > 10 {
                        break;
                    }
                } else {
                    break;
                }
            }

            if is_valid {
                if entity == "amp"
                    || entity == "lt"
                    || entity == "gt"
                    || entity == "quot"
                    || entity == "apos"
                    || (entity.starts_with('#') && entity.len() > 1)
                {
                    result.push('&');
                } else if entity == "nbsp" {
                    result.push_str("&#160;");
                    // Advance chars past "nbsp;"
                    for _ in 0..5 {
                        chars.next();
                    }
                } else {
                    result.push_str("&amp;");
                }
            } else {
                result.push_str("&amp;");
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Accumulate affine transforms from root down to this node
#[inline]
fn multiply_transform(
    ts1: &Transform,
    ts2: &Transform,
) -> Transform {
    Transform {
        a: ts1.c.mul_add(ts2.b, ts1.a * ts2.a),
        b: ts1.d.mul_add(ts2.b, ts1.b * ts2.a),
        c: ts1.c.mul_add(ts2.d, ts1.a * ts2.c),
        d: ts1.d.mul_add(ts2.d, ts1.b * ts2.c),
        e: ts1.c.mul_add(ts2.f, ts1.a * ts2.e) + ts1.e,
        f: ts1.d.mul_add(ts2.f, ts1.b * ts2.e) + ts1.f,
    }
}

fn get_accumulated_transform(node: &Node<'_, '_>) -> Transform {
    let mut chain = Vec::new();
    let mut cur = Some(*node);
    while let Some(n) = cur {
        if let Some(t_str) = n.attribute("transform")
            && let Ok(t) = Transform::from_str(t_str)
        {
            chain.push(t);
        }
        cur = n.parent();
    }
    chain.reverse();
    let mut acc = Transform::default();
    for t in chain {
        acc = multiply_transform(&acc, &t);
    }
    acc
}

/// Transform rectangle corners and compute axis-aligned bounding box
fn transform_rect(
    t: &Transform,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> Rect {
    let pts = [(x, y), (x + w, y), (x + w, y + h), (x, y + h)];
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for (px, py) in pts {
        let tx = t.c.mul_add(py, t.a * px) + t.e;
        let ty = t.d.mul_add(py, t.b * px) + t.f;
        min_x = min_x.min(tx);
        max_x = max_x.max(tx);
        min_y = min_y.min(ty);
        max_y = max_y.max(ty);
    }

    Rect::new(
        min_x as f32,
        min_y as f32,
        (max_x - min_x) as f32,
        (max_y - min_y) as f32,
    )
}

enum ParsedElement {
    Hotspot(Hotspot),
    Step(StepFragment),
    Transition(String),
}

/// Parse SVG content and extract viewBox, interactive hotspots, component step fragments, and transition metadata
pub fn parse_svg_slide(svg_content: &str) -> Result<SvgSlideInfo> {
    parse_svg_slide_with_root(svg_content, None)
}

/// Parse SVG content with an optional presentation root directory for resolving media/chart sources
pub fn parse_svg_slide_with_root(
    svg_content: &str,
    root_dir: Option<&Path>,
) -> Result<SvgSlideInfo> {
    let sanitized = sanitize_svg(svg_content);
    let doc = Document::parse(&sanitized)
        .map_err(|e| SlideError::SvgParse(format!("XML parse error: {e}")))?;

    let root = doc.root_element();
    let view_box = parse_view_box(&root)?;

    let mut hotspots = Vec::new();
    let mut steps = Vec::new();
    let mut transition = None;

    for node in root.descendants() {
        if node.is_element()
            && node.tag_name().name() == "a"
            && let Some(elem) = parse_a_element(&node, root_dir)
        {
            match elem {
                | ParsedElement::Hotspot(h) => hotspots.push(h),
                | ParsedElement::Step(s) => steps.push(s),
                | ParsedElement::Transition(t) => {
                    if transition.is_none() {
                        transition = Some(t);
                    }
                },
            }
        }
    }

    // Sort step fragments by order
    steps.sort_by_key(|s| s.order);

    Ok(SvgSlideInfo {
        view_box,
        hotspots,
        steps,
        transition,
    })
}

fn parse_view_box(root: &Node<'_, '_>) -> Result<Rect> {
    if let Some(vb) = root.attribute("viewBox") {
        let parts: Vec<f32> = vb
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        if parts.len() == 4 {
            return Ok(Rect::new(parts[0], parts[1], parts[2], parts[3]));
        }
    }

    // Fallback to width & height
    let w = root
        .attribute("width")
        .and_then(parse_dimension)
        .unwrap_or(800.0);
    let h = root
        .attribute("height")
        .and_then(parse_dimension)
        .unwrap_or(450.0);

    Ok(Rect::new(0.0, 0.0, w, h))
}

fn parse_dimension(val: &str) -> Option<f32> {
    let trimmed = val
        .trim_end_matches("pt")
        .trim_end_matches("px")
        .trim_end_matches("cm")
        .trim_end_matches("mm")
        .trim();
    trimmed.parse::<f32>().ok()
}

fn collect_descendant_bounds(
    node: &Node<'_, '_>,
    current_transform: &Transform,
    min_x: &mut f64,
    min_y: &mut f64,
    max_x: &mut f64,
    max_y: &mut f64,
    found: &mut bool,
) {
    for child in node.children().filter(roxmltree::Node::is_element) {
        let child_transform = child
            .attribute("transform")
            .and_then(|t| Transform::from_str(t).ok())
            .unwrap_or_default();
        let full_transform = multiply_transform(current_transform, &child_transform);

        let tag = child.tag_name().name();
        if tag == "rect" {
            let x: f64 = child
                .attribute("x")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            let y: f64 = child
                .attribute("y")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            let w: f64 = child
                .attribute("width")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            let h: f64 = child
                .attribute("height")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);

            if w > 0.0 && h > 0.0 {
                let r = transform_rect(&full_transform, x, y, w, h);
                *min_x = min_x.min(f64::from(r.x));
                *min_y = min_y.min(f64::from(r.y));
                *max_x = max_x.max(f64::from(r.x + r.width));
                *max_y = max_y.max(f64::from(r.y + r.height));
                *found = true;
            }
        } else if tag == "g" || tag == "a" {
            collect_descendant_bounds(&child, &full_transform, min_x, min_y, max_x, max_y, found);
        }
    }
}

fn parse_a_element(
    node: &Node<'_, '_>,
    root_dir: Option<&Path>,
) -> Option<ParsedElement> {
    let href = node
        .attribute("href")
        .or_else(|| node.attribute(("http://www.w3.org/1999/xlink", "href")))
        .or_else(|| {
            node.attributes()
                .find(|a| a.name() == "href")
                .map(|a| a.value())
        })?;

    let clean_href = href.strip_prefix('#').unwrap_or(href);

    // Check transition metadata link: transition:name or #transition:name
    if let Some(name) = clean_href.strip_prefix("transition:") {
        return Some(ParsedElement::Transition(name.trim().to_string()));
    }

    // Accumulate all ancestor transforms down to this <a> node
    let acc_transform = get_accumulated_transform(node);

    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut found = false;

    collect_descendant_bounds(
        node,
        &acc_transform,
        &mut min_x,
        &mut min_y,
        &mut max_x,
        &mut max_y,
        &mut found,
    );

    if !found || max_x <= min_x || max_y <= min_y {
        return None;
    }

    let rect = Rect::new(
        min_x as f32,
        min_y as f32,
        (max_x - min_x) as f32,
        (max_y - min_y) as f32,
    );

    if let Some(step_query) = clean_href.strip_prefix("step:") {
        let (order_str, params) = match step_query.split_once('?') {
            | Some((o, p)) => (o, p),
            | None => (step_query, ""),
        };
        let order = order_str.parse::<usize>().unwrap_or(1);
        let mut effect = "fade-in".to_string();
        for pair in params.split(['&', ';']) {
            if let Some((k, v)) = pair.split_once('=')
                && k == "effect"
            {
                effect = v.to_string();
            }
        }
        Some(ParsedElement::Step(StepFragment { order, effect, rect }))
    } else if let Some(video_path) = clean_href.strip_prefix("video:") {
        Some(ParsedElement::Hotspot(Hotspot::Video {
            source: video_path.to_string(),
            rect,
            caption: None,
        }))
    } else if let Some(audio_query) = clean_href.strip_prefix("audio:") {
        let (source, params) = match audio_query.split_once('?') {
            | Some((s, p)) => (s, p),
            | None => (audio_query, ""),
        };

        let mut autoplay = false;
        let mut loop_audio = false;
        let mut volume = 0.8f32;

        for pair in params.split(['&', ';']) {
            if let Some((k, v)) = pair.split_once('=') {
                match k {
                    | "autoplay" => autoplay = v == "true" || v == "1",
                    | "loop" => loop_audio = v == "true" || v == "1",
                    | "vol" | "volume" => {
                        if let Ok(val) = v.parse::<f32>() {
                            volume = val.clamp(0.0, 1.0);
                        }
                    },
                    | _ => {},
                }
            }
        }

        Some(ParsedElement::Hotspot(Hotspot::Audio {
            source: source.to_string(),
            rect,
            title: None,
            autoplay,
            loop_audio,
            volume,
        }))
    } else if let Some(chart_payload) = clean_href.strip_prefix("chart:") {
        parse_chart_href_with_root(chart_payload, rect, root_dir).map(ParsedElement::Hotspot)
    } else {
        Some(ParsedElement::Hotspot(Hotspot::Link {
            target: href.to_string(),
            rect,
        }))
    }
}

#[must_use]
pub fn parse_chart_href(
    chart_str: &str,
    rect: Rect,
) -> Option<Hotspot> {
    parse_chart_href_with_root(chart_str, rect, None)
}

fn decode_chart_param(val: &str) -> String {
    val.replace("%26", "&").replace('+', " ")
}

#[must_use]
pub fn parse_chart_href_with_root(
    chart_str: &str,
    rect: Rect,
    root_dir: Option<&Path>,
) -> Option<Hotspot> {
    use crate::chart::ChartData;
    use crate::chart::ChartType;
    use crate::chart::DEFAULT_CHART_COLORS;
    use crate::chart::NumberFormat;
    use crate::chart::SeriesData;

    let chart_str_trimmed = chart_str.trim();
    if chart_str_trimmed.starts_with('{')
        && let Ok(data) = serde_json::from_str::<ChartData>(chart_str_trimmed)
    {
        return Some(Hotspot::Chart { rect, data });
    }

    let mut chart_type = ChartType::Bar;
    let mut title = None;
    let mut source = None;
    let mut sql_query = None;
    let mut dsl_pipeline = None;
    let mut chart_format = NumberFormat::Auto;
    let mut chart_unit = None;
    let mut chart_prefix = None;
    let mut chart_precision = None;
    let mut categories = Vec::new();
    let mut series = Vec::new();

    let normalized_chart_str = chart_str.replace("&amp;", "&");
    for pair in normalized_chart_str.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            let key = k.trim();
            let val = v.trim();
            match key {
                | "type" => {
                    chart_type = val.parse().unwrap_or(ChartType::Bar);
                },
                | "title" => {
                    title = Some(decode_chart_param(val));
                },
                | "format" => {
                    chart_format = val.parse().unwrap_or(NumberFormat::Auto);
                },
                | "unit" => {
                    chart_unit = Some(decode_chart_param(val));
                },
                | "prefix" => {
                    chart_prefix = Some(decode_chart_param(val));
                },
                | "precision" | "prec" => {
                    chart_precision = val.parse::<usize>().ok();
                },
                | "source" => {
                    source = Some(val.replace("%26", "&"));
                },
                | "query" | "sql" => {
                    sql_query = Some(decode_chart_param(val));
                },
                | "dsl" | "pipeline" | "transform" => {
                    dsl_pipeline = Some(decode_chart_param(val));
                },
                | "categories" | "cats" => {
                    categories = val
                        .split(',')
                        .map(|s| decode_chart_param(s.trim()))
                        .collect();
                },
                | "series" => {
                    for s_item in val.split(';') {
                        if let Some((s_name, s_vals)) = s_item.split_once(':') {
                            let values = s_vals
                                .split(',')
                                .filter_map(|x| x.trim().parse::<f64>().ok())
                                .collect();
                            let color_idx = series.len() % DEFAULT_CHART_COLORS.len();
                            series.push(SeriesData {
                                name: decode_chart_param(s_name.trim()),
                                values,
                                color: Some(DEFAULT_CHART_COLORS[color_idx].to_string()),
                            });
                        }
                    }
                },
                | _ => {},
            }
        }
    }

    let mut loaded_data: Option<ChartData> = None;

    if let Some(src) = source {
        let (path_str, embedded_query) = match src.split_once('?') {
            | Some((p, q)) => {
                let q_param = q.strip_prefix("query=").unwrap_or(q);
                (p, Some(decode_chart_param(q_param)))
            },
            | None => (src.as_str(), sql_query.clone()),
        };

        let p_raw = Path::new(path_str);
        let resolved_path = if p_raw.is_absolute() {
            p_raw.to_path_buf()
        } else if let Some(rd) = root_dir {
            let candidate = rd.join(p_raw);
            if candidate.exists() {
                candidate
            } else {
                p_raw.to_path_buf()
            }
        } else {
            p_raw.to_path_buf()
        };

        let path = resolved_path.as_path();
        if path_str.ends_with(".db") || path_str.ends_with(".sqlite") {
            let sql = embedded_query.as_deref().unwrap_or("SELECT * FROM data");
            if let Ok(data) = ChartData::from_sqlite(path, sql, chart_type, title.clone()) {
                loaded_data = Some(data);
            } else {
                let cache_csv = format!("{}.cache.csv", path.display());
                if let Ok(data) =
                    ChartData::from_csv_file(Path::new(&cache_csv), chart_type, title.clone())
                {
                    loaded_data = Some(data);
                }
            }
        } else if path_str.ends_with(".json") || path_str.ends_with(".jsonl") {
            if let Ok(data) = ChartData::from_json_file(path, chart_type, title.clone()) {
                loaded_data = Some(data);
            } else {
                let cache_csv = format!("{}.cache.csv", path.display());
                if let Ok(data) =
                    ChartData::from_csv_file(Path::new(&cache_csv), chart_type, title.clone())
                {
                    loaded_data = Some(data);
                }
            }
        } else if let Ok(data) = ChartData::from_csv_file(path, chart_type, title.clone()) {
            loaded_data = Some(data);
        } else {
            let cache_csv = format!("{}.cache.csv", path.display());
            if let Ok(data) =
                ChartData::from_csv_file(Path::new(&cache_csv), chart_type, title.clone())
            {
                loaded_data = Some(data);
            }
        }
    }

    if loaded_data.is_none() && (!categories.is_empty() || !series.is_empty()) {
        loaded_data = Some(ChartData {
            chart_type,
            title: title.clone(),
            categories,
            series,
            x_label: None,
            y_label: None,
            format: chart_format,
            unit: chart_unit.clone(),
            prefix: chart_prefix.clone(),
            precision: chart_precision,
        });
    }

    let mut data = loaded_data.unwrap_or_else(|| {
        ChartData {
            chart_type,
            title,
            categories: vec!["A".into(), "B".into(), "C".into()],
            series: vec![SeriesData {
                name: "Series 1".into(),
                values: vec![10.0, 20.0, 30.0],
                color: Some(DEFAULT_CHART_COLORS[0].to_string()),
            }],
            x_label: None,
            y_label: None,
            format: chart_format,
            unit: chart_unit.clone(),
            prefix: chart_prefix.clone(),
            precision: chart_precision,
        }
    });

    data.format = chart_format;
    if chart_unit.is_some() {
        data.unit = chart_unit;
    }
    if chart_prefix.is_some() {
        data.prefix = chart_prefix;
    }
    if chart_precision.is_some() {
        data.precision = chart_precision;
    }

    if let Some(sql) = &sql_query
        && let Ok(res) = data.query_sql(sql)
    {
        data = res;
    }

    if let Some(dsl) = &dsl_pipeline
        && let Ok(res) = data.apply_dsl(dsl)
    {
        data = res;
    }

    Some(Hotspot::Chart { rect, data })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_svg_unescaped_entities() {
        let input = r#"<svg><a href="chart:type=line&source=data.csv&query=SELECT * FROM data">Link</a></svg>"#;
        let sanitized = sanitize_svg(input);
        assert!(sanitized.contains("&amp;source="));
        assert!(sanitized.contains("&amp;query="));
        // Verify roxmltree can parse it cleanly
        let doc = roxmltree::Document::parse(&sanitized);
        assert!(doc.is_ok());
    }

    #[test]
    fn test_sanitize_svg_already_escaped_entities() {
        let input = r#"<svg><text>Rust &amp; Typst &lt;rocks&gt; &#160; &#x20;</text></svg>"#;
        let sanitized = sanitize_svg(input);
        assert_eq!(sanitized, input);
    }

    #[test]
    fn test_sanitize_svg_nbsp_handling() {
        let input = r#"<svg><text>Item 1&nbsp;Item 2</text></svg>"#;
        let sanitized = sanitize_svg(input);
        assert_eq!(sanitized, r#"<svg><text>Item 1&#160;Item 2</text></svg>"#);
        let doc = roxmltree::Document::parse(&sanitized);
        assert!(doc.is_ok());
    }

    #[test]
    fn test_sanitize_svg_idempotency() {
        let input = r#"<svg><a href="https://example.com?a=1&b=2&c=3">Test</a></svg>"#;
        let pass1 = sanitize_svg(input);
        let pass2 = sanitize_svg(&pass1);
        assert_eq!(pass1, pass2);
        assert!(pass1.contains("&amp;b=2"));
        assert!(pass1.contains("&amp;c=3"));
    }
}
