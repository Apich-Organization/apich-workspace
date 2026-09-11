//! Preset query buttons for the table page's raw SQL console (`table_page.rs`'s
//! `render_sql_console`). Before this existed, the console was just a bare textarea + "Run SQL"
//! button -- every query had to be hand-typed even for the handful of things almost every table
//! user reaches for first (row count, distinct rows, column info, a quick group-by). Matches the
//! "not enough preset functionalities" half of the table feedback the same way the terminal
//! page's quick-command buttons already do for shell commands.
//!
//! Clicking a preset fills the textarea (still a real `<textarea name="sql">` inside the
//! surrounding server-rendered `<form method="post">`) without submitting -- the user can edit
//! it before running, same as typing it by hand.

use leptos::prelude::*;

#[island]
pub fn SqlConsoleIsland(
    #[prop(into)] initial_sql: String,
    #[prop(into)] table_name: String,
    first_column: Option<String>,
    query_history: Vec<String>,
    is_zh: bool,
) -> impl IntoView {
    let t = move |en: &'static str, zh: &'static str| crate::t(is_zh, en, zh);
    let sql = RwSignal::new(initial_sql);
    let quoted = format!("\"{}\"", table_name.replace('"', "\"\""));

    let mut presets: Vec<(&'static str, String)> = vec![
        (
            t("Preview 50 rows", "预览 50 行"),
            format!("SELECT * FROM {quoted} LIMIT 50;"),
        ),
        (
            t("Count rows", "统计行数"),
            format!("SELECT COUNT(*) FROM {quoted};"),
        ),
        (
            t("Distinct rows", "去重行"),
            format!("SELECT DISTINCT * FROM {quoted};"),
        ),
        (
            t("Column info", "列信息"),
            format!("PRAGMA table_info({quoted});"),
        ),
    ];
    if let Some(col) = first_column.filter(|c| !c.is_empty()) {
        let qcol = format!("\"{}\"", col.replace('"', "\"\""));
        presets.push((
            t("Group by first column", "按第一列分组"),
            format!("SELECT {qcol}, COUNT(*) AS n FROM {quoted} GROUP BY {qcol} ORDER BY n DESC;"),
        ));
    }

    let history_row = (!query_history.is_empty()).then(|| {
        view! {
            <div style="display:flex; gap:0.4rem; flex-wrap:wrap; margin-bottom:0.6rem;">
                <span style="font-size:0.75rem; color:#94a3b8; margin-right:0.25rem; display:flex; align-items:center;">{t("Recent:", "最近：")}</span>
                {query_history.into_iter().map(|q| {
                    let label = if q.chars().count() > 42 { format!("{}…", q.chars().take(42).collect::<String>()) } else { q.clone() };
                    let full = q.clone();
                    view! {
                        <button
                            type="button"
                            class="btn btn-ghost btn-sm"
                            title=q.clone()
                            style="background:transparent; color:#7dd3fc; border:1px solid #334155; font-size:0.72rem; font-family:var(--font-mono);"
                            on:click=move |_| sql.set(full.clone())
                        >
                            {label}
                        </button>
                    }
                }).collect::<Vec<_>>()}
            </div>
        }
    });

    view! {
        {history_row}
        <div style="display:flex; gap:0.4rem; flex-wrap:wrap; margin-bottom:0.6rem;">
            <span style="font-size:0.75rem; color:#94a3b8; margin-right:0.25rem; display:flex; align-items:center;">{t("Presets:", "预设：")}</span>
            {presets.into_iter().map(|(label, query)| {
                view! {
                    <button
                        type="button"
                        class="btn btn-secondary btn-sm"
                        style="background:#1e293b; color:#cbd5e1; border-color:#334155; font-size:0.75rem;"
                        on:click=move |_| sql.set(query.clone())
                    >
                        {label}
                    </button>
                }
            }).collect::<Vec<_>>()}
        </div>
        <textarea
            name="sql"
            class="sql-textarea"
            style="width:100%; height:80px; background:#1e293b; color:#f8fafc; border:1px solid #334155; border-radius:6px; font-family:var(--font-mono); font-size:0.85rem; padding:0.75rem;"
            required=true
            title=t("Tip: Ctrl+Enter (Cmd+Enter on Mac) runs the query", "提示：Ctrl+Enter（Mac 上为 Cmd+Enter）可运行查询")
            prop:value=move || sql.get()
            on:input=move |ev| sql.set(event_target_value(&ev))
            on:keydown=move |ev| {
                if ev.key() == "Enter" && (ev.ctrl_key() || ev.meta_key()) {
                    ev.prevent_default();
                    submit_enclosing_form(&ev);
                }
            }
        ></textarea>
    }
}

/// Ctrl+Enter/Cmd+Enter in the SQL textarea submits its enclosing `<form>` directly, rather than
/// requiring a click on the "Run SQL" button below -- the form itself stays plain server-rendered
/// HTML (see `table_page.rs`'s `render_sql_console`), this island's textarea is just one of its
/// children, so the browser's own form/textarea association is enough to find it.
#[cfg(feature = "hydrate")]
fn submit_enclosing_form(ev: &leptos::ev::KeyboardEvent) {
    use wasm_bindgen::JsCast;
    let Some(target) = ev.target() else { return };
    let Ok(textarea) = target.dyn_into::<web_sys::HtmlTextAreaElement>() else {
        return;
    };
    if let Some(form) = textarea.form() {
        let _ = form.request_submit();
    }
}
#[cfg(not(feature = "hydrate"))]
fn submit_enclosing_form(_ev: &leptos::ev::KeyboardEvent) {}
