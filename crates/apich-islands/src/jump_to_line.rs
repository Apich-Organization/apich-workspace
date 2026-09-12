//! Shared click-to-jump-to-source-line helper for reverse-search surfaces.
//!
//! Used by every reverse-search surface in this crate: `document_editor.rs`'s Typst/slide SVG
//! clicks and plain-markdown-preview clicks, `note_editor.rs`'s outline + preview clicks, and
//! the LaTeX PDF viewer's click handler.
//!
//! This used to be two separate, near-duplicate implementations (`document_editor.rs`'s
//! `jump_to_line_in_dom` and `note_editor.rs`'s own `jump_to_line`) doing the same thing: move a
//! textarea's selection to a 1-based line number. Selects the whole target line (not just a
//! collapsed cursor) for visible feedback, and explicitly scrolls it into view using the
//! textarea's own computed line-height, since `setSelectionRange` alone doesn't reliably scroll a
//! textarea into view across browsers.

#[cfg(feature = "hydrate")]
pub(crate) fn jump_to_line_in_dom(
    textarea_id: &str,
    line: u32,
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
        pos += l.encode_utf16().count() as u32 + 1;
    }
    let line_len = lines
        .get(idx)
        .map(|l| l.encode_utf16().count() as u32)
        .unwrap_or(0);
    let end_pos = pos + line_len;

    let _ = ta.focus();
    let _ = ta.set_selection_range(pos, end_pos);

    let line_height_px = window
        .get_computed_style(&ta)
        .ok()
        .flatten()
        .and_then(|cs| cs.get_property_value("line-height").ok())
        .and_then(|s| s.trim_end_matches("px").parse::<f64>().ok())
        .unwrap_or(20.0);
    ta.set_scroll_top(((line as f64 - 4.0).max(0.0) * line_height_px) as i32);
}
#[cfg(not(feature = "hydrate"))]
pub(crate) const fn jump_to_line_in_dom(
    _textarea_id: &str,
    _line: u32,
) {
}

/// Bridge for the LaTeX PDF viewer's click handler (plain JS, driving PDF.js) to reach the same
/// `jump_to_line_in_dom` every other reverse-search surface calls directly. Not needed by, and not
/// wired up for, anything else -- Typst/markdown/note clicks are real `on:click` handlers with a
/// direct function call, no event indirection involved.
#[cfg(feature = "hydrate")]
pub(crate) fn wire_jump_to_line_listener(textarea_id: &'static str) {
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;
    let closure = Closure::<dyn Fn(web_sys::CustomEvent)>::new(move |ev: web_sys::CustomEvent| {
        if let Some(line) = ev.detail().as_f64() {
            jump_to_line_in_dom(textarea_id, line as u32);
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
