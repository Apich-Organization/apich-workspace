//! Real Rust replacement for the spreadsheet ribbon's "Delete Row" confirm+submit one-liner.
//! Reads the row id `SpreadsheetIsland` already writes into `#del-row-id-val` (a real, already-
//! established cross-boundary DOM handoff between two independent islands), confirms with the
//! user, then submits the real `#form-del-row` form.

use leptos::prelude::*;

#[island]
pub fn DeleteRowButtonIsland(#[prop(into)] label: String) -> impl IntoView {
    view! {
        <button type="button" class="btn btn-secondary btn-sm" on:click=move |_| confirm_and_delete()>
            {label}
        </button>
    }
}

#[cfg(feature = "hydrate")]
fn confirm_and_delete() {
    use wasm_bindgen::JsCast;
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    let Some(val_input) = doc
        .get_element_by_id("del-row-id-val")
        .and_then(|e| e.dyn_into::<web_sys::HtmlInputElement>().ok())
    else {
        return;
    };
    let val = val_input.value();
    if val.is_empty() {
        if let Some(win) = web_sys::window() {
            let _ = win.alert_with_message("Please click any cell in the row you wish to delete.");
        }
        return;
    }
    let confirmed = web_sys::window()
        .and_then(|w| w.confirm_with_message("Delete selected row?").ok())
        .unwrap_or(false);
    if !confirmed {
        return;
    }
    if let Some(form) = doc
        .get_element_by_id("form-del-row")
        .and_then(|e| e.dyn_into::<web_sys::HtmlFormElement>().ok())
    {
        let _ = form.request_submit();
    }
}

#[cfg(not(feature = "hydrate"))]
fn confirm_and_delete() {}
