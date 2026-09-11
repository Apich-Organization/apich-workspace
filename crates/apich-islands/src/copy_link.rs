//! Real Rust replacement for the "copy public share link" button's hand-written
//! `onclick="navigator.clipboard.writeText(...)"` one-liner.
//!
//! The visible input used to show only the bare relative path (`/shared/<uuid>`) -- correct as a
//! copy-to-clipboard *target* (the click handler always prefixed it with the real origin before
//! writing to the clipboard), but as *displayed text* a bare path with no host looks like a
//! fragment, not a link, and a user scanning the page for "the share link" could easily not
//! recognize it as one. Now shown as the real absolute URL from the moment the island hydrates.

use leptos::prelude::*;

#[island]
pub fn CopyLinkIsland(
    #[prop(into)] link: String,
    #[prop(into)] button_label: String,
) -> impl IntoView {
    let status = RwSignal::new(button_label.clone());
    let link_for_copy = link.clone();
    let display_link = RwSignal::new(link.clone());
    absolutize_display_link(display_link, link.clone());

    view! {
        <input type="text" readonly=true prop:value=move || display_link.get() class="form-control" style="background:var(--bg-muted); font-family:var(--font-mono); font-size:0.85rem;" onfocus="this.select()" />
        <button
            type="button"
            class="btn btn-secondary"
            on:click=move |_| {
                copy_to_clipboard(&link_for_copy);
                status.set("✓ Copied".to_string());
            }
        >
            {move || status.get()}
        </button>
    }
}

#[cfg(feature = "hydrate")]
fn absolutize_display_link(
    display_link: RwSignal<String>,
    path: String,
) {
    if let Some(win) = web_sys::window() {
        let origin = win.location().origin().unwrap_or_default();
        display_link.set(format!("{origin}{path}"));
    }
}
#[cfg(not(feature = "hydrate"))]
fn absolutize_display_link(
    _display_link: RwSignal<String>,
    _path: String,
) {
}

#[cfg(feature = "hydrate")]
fn copy_to_clipboard(path: &str) {
    if let Some(win) = web_sys::window() {
        let origin = win.location().origin().unwrap_or_default();
        let _ = win
            .navigator()
            .clipboard()
            .write_text(&format!("{origin}{path}"));
    }
}
#[cfg(not(feature = "hydrate"))]
fn copy_to_clipboard(_path: &str) {}
