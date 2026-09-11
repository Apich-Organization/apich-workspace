//! Generic confirm-before-submit button for any destructive form action (delete project, delete
//! file, etc.) -- a real `window.confirm()` gate in front of the form's native submit, reusable
//! anywhere instead of a one-off hand-written `onclick="return confirm(...)"` string.

use leptos::prelude::*;

#[island]
pub fn ConfirmSubmitButton(
    #[prop(into)] label: String,
    #[prop(into)] message: String,
    #[prop(into)] button_class: String,
    #[prop(into)] button_style: String,
) -> impl IntoView {
    let on_click = move |ev: leptos::ev::MouseEvent| confirm_or_prevent(ev, message.clone());

    view! {
        <button type="submit" class=button_class style=button_style on:click=on_click>{label}</button>
    }
}

#[cfg(feature = "hydrate")]
fn confirm_or_prevent(
    ev: leptos::ev::MouseEvent,
    message: String,
) {
    let confirmed = web_sys::window()
        .and_then(|w| w.confirm_with_message(&message).ok())
        .unwrap_or(false);
    if !confirmed {
        ev.prevent_default();
    }
}
#[cfg(not(feature = "hydrate"))]
fn confirm_or_prevent(
    _ev: leptos::ev::MouseEvent,
    _message: String,
) {
}
