//! Real Rust replacement for the app's several near-identical hand-written "show/hide a modal
//! backdrop" JS one-liners (`onclick="document.getElementById('modal-x').style.display=...'"`).
//! One generic, reusable island: the trigger button and the modal (backdrop + card + close
//! button) share real Rust state instead of each modal getting its own copy-pasted toggle glue.
//! The modal body itself renders as real server HTML passed in as `children` -- forms inside it
//! (file names, descriptions, etc.) stay plain, non-interactive `<form>` elements that submit
//! natively; only open/closed state is reactive.

use leptos::prelude::*;

#[island]
pub fn ModalIsland(
    #[prop(into)] trigger_label: String,
    #[prop(into)] trigger_class: String,
    #[prop(into)] title: String,
    children: Children,
) -> impl IntoView {
    let open = RwSignal::new(false);

    view! {
        <button type="button" class=trigger_class on:click=move |_| open.set(true)>{trigger_label}</button>
        <div class="modal-backdrop" style:display=move || if open.get() { "flex" } else { "none" }>
            <div class="modal-card">
                <div class="modal-header">
                    <h3 class="modal-title">{title}</h3>
                    <button type="button" class="modal-close" on:click=move |_| open.set(false)>"×"</button>
                </div>
                {children()}
            </div>
        </div>
    }
}
