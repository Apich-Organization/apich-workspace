//! Shared click-to-jump-to-source-line and match-search highlighting helper.
//!
//! Used by every reverse-search surface in this crate: `document_editor.rs`'s Typst/slide SVG
//! clicks and plain-markdown-preview clicks, `note_editor.rs`'s outline + preview clicks,
//! the LaTeX PDF viewer's click handler, and `search_replace.rs`'s search match jumps.
//!
//! Moves a textarea's selection to a 1-based line number (and exact matched query text if provided),
//! selects the matched text or the whole target line for visible feedback, and explicitly scrolls it
//! into view using the textarea's own computed line-height.

#[cfg(feature = "hydrate")]
pub(crate) fn jump_to_match_in_dom(
    textarea_id: &str,
    line: u32,
    query: Option<&str>,
    match_start: Option<usize>,
    match_end: Option<usize>,
) {
    use wasm_bindgen::JsCast;
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(doc) = window.document() else { return };
    let Some(el) = doc.get_element_by_id(textarea_id) else {
        return;
    };
    let Ok(ta) = el.dyn_into::<web_sys::HtmlTextAreaElement>() else {
        return;
    };

    let value = ta.value();
    let lines: Vec<&str> = value.split('\n').collect();
    let idx = (line.saturating_sub(1)) as usize;
    let mut pos: u32 = 0;
    for l in lines.iter().take(idx) {
        pos = pos.saturating_add(l.encode_utf16().count() as u32).saturating_add(1);
    }
    let target_line = lines.get(idx).copied().unwrap_or("");
    let line_len = target_line.encode_utf16().count() as u32;

    let (sel_start, sel_end) = if let (Some(s), Some(e)) = (match_start, match_end) {
        if s <= e && e <= target_line.len() {
            let before_s = target_line.get(..s).unwrap_or("");
            let matched = target_line.get(s..e).unwrap_or("");
            let u16_start = pos.saturating_add(before_s.encode_utf16().count() as u32);
            let u16_end = u16_start.saturating_add(matched.encode_utf16().count() as u32);
            (u16_start, u16_end)
        } else {
            (pos, pos.saturating_add(line_len))
        }
    } else if let Some(q) = query.filter(|q| !q.is_empty()) {
        let found_pos = target_line.find(q).or_else(|| {
            let q_lower = q.to_lowercase();
            let line_lower = target_line.to_lowercase();
            line_lower.find(&q_lower)
        });
        if let Some(byte_idx) = found_pos {
            let before = target_line.get(..byte_idx).unwrap_or("");
            let matched_slice = target_line.get(byte_idx..byte_idx.saturating_add(q.len())).unwrap_or(q);
            let u16_start = pos.saturating_add(before.encode_utf16().count() as u32);
            let u16_end = u16_start.saturating_add(matched_slice.encode_utf16().count() as u32);
            (u16_start, u16_end)
        } else {
            (pos, pos.saturating_add(line_len))
        }
    } else {
        (pos, pos.saturating_add(line_len))
    };

    let _ = ta.focus();
    let _ = ta.set_selection_range(sel_start, sel_end);

    let line_height_px = window
        .get_computed_style(&ta)
        .ok()
        .flatten()
        .and_then(|cs| cs.get_property_value("line-height").ok())
        .and_then(|s| s.trim_end_matches("px").parse::<f64>().ok())
        .unwrap_or(20.0);
    let target_scroll = ((f64::from(line) - 4.0).max(0.0) * line_height_px) as i32;
    ta.set_scroll_top(target_scroll);

    if let Some(parent) = ta.parent_element() {
        if let Ok(Some(pre)) = parent.query_selector("pre.code-highlight-overlay") {
            pre.set_scroll_top(target_scroll);
        }
    }
}

#[cfg(not(feature = "hydrate"))]
pub(crate) const fn jump_to_match_in_dom(
    _textarea_id: &str,
    _line: u32,
    _query: Option<&str>,
    _match_start: Option<usize>,
    _match_end: Option<usize>,
) {
}

#[cfg(feature = "hydrate")]
pub(crate) fn jump_to_line_in_dom(
    textarea_id: &str,
    line: u32,
) {
    jump_to_match_in_dom(textarea_id, line, None, None, None);
}

#[cfg(not(feature = "hydrate"))]
pub(crate) const fn jump_to_line_in_dom(
    _textarea_id: &str,
    _line: u32,
) {
}

/// Bridge for reverse-search surfaces dispatching `CustomEvent('apich-jump-to-line')`.
/// Supports `ev.detail` as a plain line number or JSON `{ line, query, match_start, match_end }`.
#[cfg(feature = "hydrate")]
pub(crate) fn wire_jump_to_line_listener(textarea_id: &'static str) {
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;
    let closure = Closure::<dyn Fn(web_sys::CustomEvent)>::new(move |ev: web_sys::CustomEvent| {
        let detail = ev.detail();
        if let Some(line) = detail.as_f64() {
            jump_to_match_in_dom(textarea_id, line as u32, None, None, None);
        } else if let Some(s) = detail.as_string() {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
                let line = v.get("line").and_then(|n| n.as_u64()).map(|n| n as u32).unwrap_or(1);
                let q = v.get("query").and_then(|s| s.as_str());
                let ms = v.get("match_start").and_then(|n| n.as_u64()).map(|n| n as usize);
                let me = v.get("match_end").and_then(|n| n.as_u64()).map(|n| n as usize);
                jump_to_match_in_dom(textarea_id, line, q, ms, me);
            }
        }
    });
    if let Some(win) = web_sys::window() {
        let _ = win.add_event_listener_with_callback(
            "apich-jump-to-line",
            closure.as_ref().unchecked_ref(),
        );
    }
    closure.forget();
}

#[cfg(not(feature = "hydrate"))]
pub(crate) const fn wire_jump_to_line_listener(_textarea_id: &'static str) {}

/// Reads initial `line` and `query` search params or `#L{line}` hash upon editor mount
/// and automatically scrolls to and highlights the target searched text.
#[cfg(feature = "hydrate")]
pub(crate) fn wire_initial_jump_from_url(textarea_id: &'static str) {
    use leptos::prelude::*;
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;

    Effect::new(move |_| {
        let Some(win) = web_sys::window() else { return };
        let loc = win.location();
        let search_str = loc.search().unwrap_or_default();
        let hash_str = loc.hash().unwrap_or_default();

        let mut target_line: Option<u32> = None;
        let mut target_query: Option<String> = None;

        if search_str.starts_with('?') {
            for part in search_str.trim_start_matches('?').split('&') {
                if let Some((k, v)) = part.split_once('=') {
                    if k == "line" {
                        if let Ok(l) = v.parse::<u32>() {
                            target_line = Some(l);
                        }
                    } else if k == "query" {
                        if let Ok(dec) = urlencoding::decode(v) {
                            target_query = Some(dec.into_owned());
                        }
                    }
                }
            }
        }

        if target_line.is_none() && hash_str.starts_with("#L") {
            if let Ok(l) = hash_str.trim_start_matches("#L").parse::<u32>() {
                target_line = Some(l);
            }
        }

        if let Some(line) = target_line {
            let cb = Closure::<dyn Fn()>::new(move || {
                jump_to_match_in_dom(
                    textarea_id,
                    line,
                    target_query.as_deref(),
                    None,
                    None,
                );
            });

            let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                50,
            );
            cb.forget();
        }
    });
}

#[cfg(not(feature = "hydrate"))]
pub(crate) const fn wire_initial_jump_from_url(_textarea_id: &'static str) {}
