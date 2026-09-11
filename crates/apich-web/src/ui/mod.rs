pub mod handlers;
pub mod i18n;
pub mod template_handlers;
pub mod views;

pub use handlers::build_ui_router;
pub use i18n::{resolve_language, I18n, Lang};
