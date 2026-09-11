//! Interactive Leptos islands: real Rust compiled to WASM for the specific pieces of the UI
//! that need client-side interactivity (canvas drawing, live editing, WebAuthn, chat/agent
//! streaming). Everything else in apich-web stays plain server-rendered HTML.
//!
//! This crate has zero dependency on apich-db/apich-sandbox/apich-vcs/sqlx/tokio so it can
//! compile for wasm32-unknown-unknown. Island props must stay primitive/serializable
//! (String, i64, bool, Vec<String>, serde_json::Value) -- never a server-only DB model type.

use leptos::prelude::*;

pub mod ai_drawer;
pub mod auth_tabs;
pub mod code_highlight;
pub mod confirm_submit;
pub mod copy_link;
pub mod delete_row_button;
pub mod document_editor;
pub mod file_share;
pub mod jump_to_line;
pub mod modal;
pub mod name_slug_fields;
pub mod note_editor;
pub mod notebook;
pub mod org_signup_fields;
pub mod spreadsheet;
pub mod terminal;
pub mod webauthn;
pub mod whiteboard;
pub use ai_drawer::AiDrawerIsland;
pub use auth_tabs::AuthTabsIsland;
pub use confirm_submit::ConfirmSubmitButton;
pub use copy_link::CopyLinkIsland;
pub use delete_row_button::DeleteRowButtonIsland;
pub use document_editor::{DocumentEditorIsland, HeadingItem};
pub use file_share::{FileShareModalIsland, ShareableUser};
pub use modal::ModalIsland;
pub use name_slug_fields::NameSlugFieldsIsland;
pub use note_editor::{NoteEditorIsland, NoteHeadingItem};
pub use notebook::{NotebookCellData, NotebookCellImageData, NotebookIsland};
pub use org_signup_fields::OrgSignupFieldsIsland;
pub use spreadsheet::{CellStyle as SpreadsheetCellStyle, CellStyleEntry, SpreadsheetIsland};
pub use terminal::TerminalIsland;
pub use webauthn::{PasskeyEnrollIsland, PasskeyLoginIsland};
pub use whiteboard::WhiteboardIsland;

/// Proof-of-concept island validating the SSR-render + WASM-hydrate pipeline end to end
/// before building the real feature islands. A plain server `#[component]` would render this
/// once and never update; because this is `#[island]`, the button click actually runs in the
/// browser as real Rust (compiled to WASM), not a hand-written JS string.
#[island]
pub fn PingCounterIsland(#[prop(into)] start: i32) -> impl IntoView {
    let (count, set_count) = signal(start);
    view! {
        <button
            type="button"
            class="btn btn-secondary btn-sm"
            on:click=move |_| set_count.update(|n| *n += 1)
        >
            "Island alive, clicked " {move || count.get()} " times"
        </button>
    }
}

// Called explicitly by the island bootstrap script once the wasm module has finished
// instantiating (`mod.default(wasmUrl).then(() => { mod.hydrate(); ... })`) -- must NOT run
// automatically at module-init time (no `#[wasm_bindgen(start)]`), or it would race the
// bootstrap's own `hydrateIslands()` walk of the DOM.
#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_islands();
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;
    use leptos::prelude::RenderHtml;

    /// The `data-component` id the server writes on `<leptos-island>` must match the wasm
    /// module's exported hydration function name exactly, or the browser bootstrap script
    /// silently no-ops (`mod[id]` is undefined) and the island never becomes interactive.
    #[test]
    fn island_marker_matches_expected_shape() {
        let html = view! { <PingCounterIsland start=0 /> }.to_html();
        assert!(html.contains("leptos-island"), "missing <leptos-island> wrapper: {html}");
        assert!(html.contains("data-component"), "missing data-component attr: {html}");
        assert!(html.contains("PingCounterIsland"), "component id should be derived from the fn name: {html}");
    }
}
