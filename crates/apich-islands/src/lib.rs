//! Interactive Leptos islands: real Rust compiled to WASM for the specific pieces of the UI
//! that need client-side interactivity (canvas drawing, live editing, `WebAuthn`, chat/agent
//! streaming). Everything else in apich-web stays plain server-rendered HTML.
//!
//! This crate has zero dependency on apich-db/apich-sandbox/apich-vcs/sqlx/tokio so it can
//! compile for wasm32-unknown-unknown. Island props must stay primitive/serializable
//! (String, i64, bool, Vec<String>, `serde_json::Value`) -- never a server-only DB model type.

#![allow(missing_docs)]
#![allow(
    clippy::must_use_candidate,
    clippy::too_many_lines,
    clippy::needless_pass_by_value
)]

use leptos::prelude::*;

pub mod ai_drawer;
pub mod auth_tabs;
pub mod code_highlight;
pub mod confirm_submit;
pub mod copy_link;
pub mod delete_row_button;
pub mod document_editor;
pub mod file_share;
pub mod formula;
pub mod jump_to_line;
pub mod modal;
pub mod quick_start;
pub mod name_slug_fields;
pub mod note_editor;
pub mod note_formatting;
pub mod notebook;
pub mod org_signup_fields;
pub mod slide_build;
pub mod spreadsheet;
pub mod sql_console;
pub mod terminal;
pub mod user_select;
pub mod webauthn;
pub mod whiteboard;
pub use ai_drawer::AiDrawerIsland;
pub use auth_tabs::AuthTabsIsland;
pub use confirm_submit::ConfirmSubmitButton;
pub use copy_link::CopyLinkIsland;
pub use delete_row_button::DeleteRowButtonIsland;
pub use document_editor::DocEditorFlags;
pub use document_editor::DocumentEditorIsland;
pub use document_editor::DocumentPreviewKind;
pub use document_editor::HeadingItem;
pub use file_share::FileShareModalIsland;
pub use file_share::ShareableUser;
pub use modal::ModalIsland;
pub use quick_start::QuickStartMenuIsland;
pub use quick_start::QuickStartVariant;
pub use name_slug_fields::NameSlugFieldsIsland;
pub use note_editor::NoteEditorIsland;
pub use note_editor::NoteHeadingItem;
pub use notebook::NotebookCellData;
pub use notebook::NotebookCellImageData;
pub use notebook::NotebookIsland;
pub use org_signup_fields::OrgSignupFieldsIsland;
pub use slide_build::SlideBuildIsland;
pub use spreadsheet::CellStyle as SpreadsheetCellStyle;
pub use spreadsheet::CellStyleEntry;
pub use spreadsheet::SpreadsheetIsland;
pub use sql_console::SqlConsoleIsland;
pub use terminal::TerminalIsland;
pub use webauthn::PasskeyEnrollIsland;
pub use webauthn::PasskeyLoginIsland;
pub use whiteboard::WhiteboardIsland;
pub use user_select::UserItem;
pub use user_select::UserSelectIsland;

/// Minimal bilingual-string helper for islands.
///
/// This crate can't depend on apich-web's `I18n` type (it must stay wasm32-compilable with zero
/// server-only dependencies -- see this module's own doc comment), so pages pass a plain
/// `is_zh: bool` prop (from `I18n::is_zh()`) instead, and island UI text picks between an English
/// and Chinese literal with this. Before this existed, every island's UI text (formula bar, note
/// toolbar, terminal quick-commands, SQL console presets, ...) was hardcoded English regardless
/// of the user's selected language -- `i18n.rs`'s ~200 translated strings never reached any of the
/// interactive islands at all, only the plain server-rendered page chrome around them.
#[must_use]
pub const fn t(
    is_zh: bool,
    en: &'static str,
    zh: &'static str,
) -> &'static str {
    if is_zh {
        zh
    } else {
        en
    }
}

/// Proof-of-concept island validating the SSR-render + WASM-hydrate pipeline end to end.
///
/// A plain server `#[component]` would render this once and never update; because this is
/// `#[island]`, the button click actually runs in the browser as real Rust (compiled to WASM),
/// not a hand-written JS string.
#[island]
#[must_use]
#[allow(clippy::must_use_candidate)]
pub fn PingCounterIsland(#[prop(into)] start: i32) -> impl IntoView {
    let (count, set_count) = signal(start);
    view! {
        <button
            type="button"
            class="btn btn-secondary btn-sm"
            on:click=move |_| set_count.update(|n| *n = n.saturating_add(1))
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
        assert!(
            html.contains("leptos-island"),
            "missing <leptos-island> wrapper: {html}"
        );
        assert!(
            html.contains("data-component"),
            "missing data-component attr: {html}"
        );
        assert!(
            html.contains("PingCounterIsland"),
            "component id should be derived from the fn name: {html}"
        );
    }
}
