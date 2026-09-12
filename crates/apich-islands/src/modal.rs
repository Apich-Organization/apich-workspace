//! Reusable reactive modal backdrop and card island.
//!
//! Replaces hand-written "show/hide a modal backdrop" JS one-liners. The trigger button and
//! the modal (backdrop + card + close button) share real Rust state instead of each modal
//! getting its own copy-pasted toggle glue. The modal body itself renders as real server HTML
//! passed in as `children` -- forms inside it stay plain, non-interactive `<form>` elements
//! that submit natively; only open/closed state is reactive.

use leptos::prelude::*;

#[island]
pub fn ModalIsland(
    #[prop(into)] trigger_label: String,
    #[prop(into)] trigger_class: String,
    #[prop(into)] title: String,
    children: Children,
) -> impl IntoView {
    let open = RwSignal::new(false);
    let backdrop_ref = NodeRef::<leptos::html::Div>::new();
    reparent_to_body(backdrop_ref);

    view! {
        <button type="button" class=trigger_class on:click=move |_| open.set(true)>{trigger_label}</button>
        <div node_ref=backdrop_ref class="modal-backdrop" style:display=move || if open.get() { "flex" } else { "none" }>
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

/// Moves the rendered backdrop to be a direct child of `<body>` once, after hydration.
///
/// `.modal-backdrop` is `position: fixed; inset: 0`, which is *supposed* to make it cover the
/// viewport no matter where in the document it lives. It doesn't, wherever this island is used
/// inside a frosted-glass container: a non-`none` `backdrop-filter` (`.section-card`,
/// `.page-header`, and several others all use one) makes that element a containing block for its
/// `position: fixed` descendants, so the backdrop anchors to *that card's* box instead of the
/// viewport. Confirmed live: opening "+ New File"/"Upload File" from the Project Files panel put
/// the dialog down at the panel's own position, off-screen on a long page, so the user had to
/// scroll to find a dialog that's meant to be a viewport-centered overlay.
///
/// Re-parenting to `<body>` (rather than dropping the `backdrop-filter` design, or reaching for a
/// `Portal` -- which renders nothing during SSR and would cost this modal its
/// server-rendered-children property) escapes every such ancestor at once, so `inset: 0` means
/// what it says again. Safe to do after the fact: the children are plain `<form>` markup that
/// submits natively, with no dependency on its position in the document tree.
#[cfg(feature = "hydrate")]
pub(crate) fn reparent_to_body(backdrop_ref: NodeRef<leptos::html::Div>) {
    Effect::new(move |_| {
        let Some(el) = backdrop_ref.get() else {
            return;
        };
        let Some(body) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.body())
        else {
            return;
        };
        let _ = body.append_child(&el);
    });
}

#[cfg(not(feature = "hydrate"))]
pub(crate) const fn reparent_to_body(_backdrop_ref: NodeRef<leptos::html::Div>) {}
