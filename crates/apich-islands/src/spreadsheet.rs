//! Real Rust replacement for the spreadsheet grid's hand-written JS (`table_page.rs`'s
//! `build_grid_script`): cell selection, inline cell editing, the formula bar, row/column
//! highlighting, and live per-column summaries (sum/avg/count/min/max) are all implemented
//! here as compiled Rust. The surrounding ribbon toolbar (Add Row / Import / Export / Delete
//! Row forms, the CSV modal, pagination) stays plain server-rendered HTML in `table_page.rs` --
//! only the interactive grid itself needs a browser-side brain.

use leptos::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Highlight {
    None,
    Row(usize),
    Col(usize),
    All,
}

/// Mirrors `apich_web::services::sqlite_table::CellStyle` -- can't share the type directly
/// across the crate boundary (this crate can't depend on apich-web, and apich-web can't compile
/// for wasm32), so the shape is duplicated here the same way every other island prop already is.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CellStyle {
    pub bold: bool,
    pub italic: bool,
    pub color: Option<String>,
    pub bg_color: Option<String>,
}

/// One entry of the sparse style map passed in from the server (only non-default cells are
/// included) -- a flat `(row_id, column, style)` tuple rather than a nested map, since island
/// props serialize to JSON and a `HashMap` keyed by a non-string tuple doesn't round-trip well.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellStyleEntry {
    pub row_id: i64,
    pub col: String,
    pub style: CellStyle,
}

#[island]
pub fn SpreadsheetIsland(
    #[prop(into)] project_id: String,
    #[prop(into)] file_path: String,
    #[prop(into)] table_name: String,
    columns: Vec<String>,
    column_types: Vec<String>,
    primary_keys: Vec<bool>,
    rows: Vec<Vec<String>>,
    row_ids: Vec<i64>,
    cell_styles: Vec<CellStyleEntry>,
    is_zh: bool,
) -> impl IntoView {
    let t = move |en: &'static str, zh: &'static str| crate::t(is_zh, en, zh);
    let n_cols = columns.len();
    let cells = RwSignal::new(rows);
    let active = RwSignal::new(None::<(usize, usize)>);
    let highlight = RwSignal::new(Highlight::None);
    let editing = RwSignal::new(None::<(usize, usize)>);
    let edit_draft = RwSignal::new(String::new());
    let fx_value = RwSignal::new(String::new());
    let summary_ops = RwSignal::new(vec!["sum".to_string(); n_cols]);
    let initial_styles: HashMap<(usize, usize), CellStyle> = {
        let row_index: HashMap<i64, usize> =
            row_ids.iter().enumerate().map(|(i, id)| (*id, i)).collect();
        let col_index: HashMap<&str, usize> = columns
            .iter()
            .enumerate()
            .map(|(i, c)| (c.as_str(), i))
            .collect();
        cell_styles
            .into_iter()
            .filter_map(|entry| {
                let r = *row_index.get(&entry.row_id)?;
                let c = *col_index.get(entry.col.as_str())?;
                Some(((r, c), entry.style))
            })
            .collect()
    };
    let styles = RwSignal::new(initial_styles);
    let row_ids = StoredValue::new(row_ids);
    let col_names = StoredValue::new(columns.clone());
    // Stored (not plain String) so every closure below that needs these stays `Copy` --
    // required because `select_cell`/`commit_edit` are each referenced from many independent
    // per-cell closures generated in the loops below, not just one call site.
    let project_id = StoredValue::new(project_id);
    let file_path = StoredValue::new(file_path);
    let table_name = StoredValue::new(table_name);

    let col_letter = |idx: usize| ((b'A' + (idx % 26) as u8) as char).to_string();

    let select_cell = move |r: usize, c: usize| {
        active.set(Some((r, c)));
        let val = cells.with(|rows| {
            rows.get(r)
                .and_then(|row| row.get(c))
                .cloned()
                .unwrap_or_default()
        });
        fx_value.set(val);
        set_delete_row_target(row_ids.with_value(|ids| ids.get(r).copied()));
    };

    let start_edit = move |r: usize, c: usize| {
        let cur = cells.with(|rows| {
            rows.get(r)
                .and_then(|row| row.get(c))
                .cloned()
                .unwrap_or_default()
        });
        edit_draft.set(cur);
        editing.set(Some((r, c)));
    };

    let commit_edit = move |r: usize, c: usize| {
        let Some((er, ec)) = editing.get_untracked() else {
            return;
        };
        if (er, ec) != (r, c) {
            return;
        }
        let new_val = edit_draft.get_untracked();
        cells.update(|rows| {
            if let Some(row) = rows.get_mut(r) {
                if let Some(cell) = row.get_mut(c) {
                    *cell = new_val.clone();
                }
            }
        });
        editing.set(None);
        let col_name = col_names.with_value(|cols| cols.get(c).cloned().unwrap_or_default());
        let rowid = row_ids.with_value(|ids| ids.get(r).copied()).unwrap_or(0);
        save_cell(
            project_id.with_value(|s| s.clone()),
            file_path.with_value(|s| s.clone()),
            table_name.with_value(|s| s.clone()),
            col_name,
            rowid,
            new_val,
        );
    };

    let apply_style = move |mutate: Box<dyn Fn(&mut CellStyle)>| {
        let Some((r, c)) = active.get_untracked() else {
            return;
        };
        let mut new_style = styles.with_untracked(|m| m.get(&(r, c)).cloned().unwrap_or_default());
        mutate(&mut new_style);
        styles.update(|m| {
            if new_style == CellStyle::default() {
                m.remove(&(r, c));
            } else {
                m.insert((r, c), new_style.clone());
            }
        });
        let col_name = col_names.with_value(|cols| cols.get(c).cloned().unwrap_or_default());
        let rowid = row_ids.with_value(|ids| ids.get(r).copied()).unwrap_or(0);
        save_style(
            project_id.with_value(|s| s.clone()),
            file_path.with_value(|s| s.clone()),
            table_name.with_value(|s| s.clone()),
            col_name,
            rowid,
            new_style,
        );
    };
    let active_style = move || {
        active
            .get()
            .and_then(|(r, c)| styles.with(|m| m.get(&(r, c)).cloned()))
            .unwrap_or_default()
    };

    // A spreadsheet reads numbers right-aligned and text left-aligned; SQLite's own type-affinity
    // keywords (see https://www.sqlite.org/datatype3.html#type_affinity) are enough of a signal
    // for this without needing to sniff actual cell values.
    let is_numeric_type = |t: &str| {
        let up = t.to_ascii_uppercase();
        ["INT", "REAL", "FLOA", "DOUB", "NUMERIC", "DECIMAL"]
            .iter()
            .any(|kw| up.contains(kw))
    };
    let col_is_numeric: Vec<bool> = column_types.iter().map(|t| is_numeric_type(t)).collect();

    let header_cells: Vec<_> = columns
        .iter()
        .enumerate()
        .map(|(c, name)| {
            let type_str = column_types
                .get(c)
                .cloned()
                .unwrap_or_else(|| "TEXT".to_string());
            let type_str_title = type_str.clone();
            let numeric = col_is_numeric.get(c).copied().unwrap_or(false);
            let is_pk = primary_keys.get(c).copied().unwrap_or(false);
            let name = name.clone();
            view! {
                <th
                    data-col=c.to_string()
                    class="col-header"
                    class:col-numeric=numeric
                    title=format!("{type_str_title} column -- click to select")
                    on:click=move |_| highlight.set(Highlight::Col(c))
                >
                    <div class="col-letter">{col_letter(c)}</div>
                    <div class="col-name">
                        {name}
                        {is_pk.then(|| view! { <span class="col-pk-badge">"PK"</span> })}
                    </div>
                    <div class="col-type">{type_str}</div>
                </th>
            }
        })
        .collect();

    let n_rows = cells.with_untracked(|r| r.len());
    let body_rows: Vec<_> = (0..n_rows)
        .map(|r| {
            let row_cells: Vec<_> = (0..n_cols)
                .map(|c| {
                    view! {
                        <td
                            class="cell-data"
                            class:col-numeric=col_is_numeric.get(c).copied().unwrap_or(false)
                            class:cell-selected=move || {
                                if editing.get() == Some((r, c)) { return false; }
                                active.get() == Some((r, c))
                                    || matches!(highlight.get(), Highlight::Row(hr) if hr == r)
                                    || matches!(highlight.get(), Highlight::Col(hc) if hc == c)
                                    || matches!(highlight.get(), Highlight::All)
                            }
                            style=move || {
                                let s = styles.with(|m| m.get(&(r, c)).cloned()).unwrap_or_default();
                                format!(
                                    "font-weight:{}; font-style:{}; color:{}; background-color:{};",
                                    if s.bold { "700" } else { "inherit" },
                                    if s.italic { "italic" } else { "normal" },
                                    s.color.as_deref().unwrap_or("inherit"),
                                    s.bg_color.as_deref().unwrap_or("transparent"),
                                )
                            }
                            on:click=move |_| select_cell(r, c)
                            on:dblclick=move |_| start_edit(r, c)
                        >
                            {move || {
                                if editing.get() == Some((r, c)) {
                                    view! {
                                        <textarea
                                            class="cell-edit-textarea"
                                            rows="1"
                                            prop:value=move || edit_draft.get()
                                            on:input=move |ev| edit_draft.set(event_target_value(&ev))
                                            on:blur=move |_| commit_edit(r, c)
                                            on:keydown=move |ev| {
                                                // Plain Enter commits (matching a normal spreadsheet);
                                                // Shift+Enter inserts a real newline for multi-line
                                                // cell content instead.
                                                if ev.key() == "Enter" && !ev.shift_key() {
                                                    ev.prevent_default();
                                                    commit_edit(r, c);
                                                    select_cell((r + 1).min(n_rows.saturating_sub(1)), c);
                                                }
                                            }
                                        ></textarea>
                                    }.into_any()
                                } else {
                                    // A raw value starting with `=` is a formula (see
                                    // `crate::formula`) -- displayed as its computed result while
                                    // still stored/edited as the formula text itself, the same
                                    // display-vs-edit split every spreadsheet does. Line breaks in
                                    // non-formula text are preserved (`.cell-display` is
                                    // `white-space: pre-wrap` in CSS) rather than collapsed.
                                    let text = cells.with(|rows| {
                                        let raw = rows.get(r).and_then(|row| row.get(c)).cloned().unwrap_or_default();
                                        crate::formula::display_value(&raw, rows)
                                    });
                                    view! { <span class="cell-display">{text}</span> }.into_any()
                                }
                            }}
                        </td>
                    }
                })
                .collect();
            view! {
                <tr>
                    <td class="row-index-cell" on:click=move |_| {{ highlight.set(Highlight::Row(r)); select_cell(r, 0); }}>{r + 1}</td>
                    {row_cells}
                </tr>
            }
        })
        .collect();

    let summary_cells: Vec<_> = (0..n_cols)
        .map(|c| {
            let summary_val = move || {
                let op = summary_ops.with(|ops| ops.get(c).cloned().unwrap_or_else(|| "none".to_string()));
                if op == "none" {
                    return "-".to_string();
                }
                // Resolve formula cells (values starting with `=`) to their computed number
                // before aggregating -- otherwise a column summary silently skipped every
                // formula cell (its raw text, e.g. "=SUM(A1:A3)", never parses as f64).
                let nums: Vec<f64> = cells.with(|rows| {
                    rows.iter()
                        .filter_map(|row| row.get(c))
                        .filter_map(|v| crate::formula::display_value(v, rows).parse::<f64>().ok())
                        .collect()
                });
                let count = cells.with(|rows| rows.len());
                match op.as_str() {
                    "count" => count.to_string(),
                    _ if nums.is_empty() => "0".to_string(),
                    "sum" => format!("{:.2}", nums.iter().sum::<f64>()),
                    "avg" => format!("{:.2}", nums.iter().sum::<f64>() / nums.len() as f64),
                    "min" => format!("{:.2}", nums.iter().cloned().fold(f64::INFINITY, f64::min)),
                    "max" => format!("{:.2}", nums.iter().cloned().fold(f64::NEG_INFINITY, f64::max)),
                    _ => "-".to_string(),
                }
            };
            view! {
                <td>
                    <select
                        class="summary-dropdown"
                        on:change=move |ev| {
                            let val = event_target_value(&ev);
                            summary_ops.update(|ops| { if let Some(slot) = ops.get_mut(c) { *slot = val; } });
                        }
                    >
                        <option value="none">{t("--", "——")}</option>
                        <option value="sum" selected=true>{t("Sum", "求和")}</option>
                        <option value="avg">{t("Avg", "平均值")}</option>
                        <option value="count">{t("Count", "计数")}</option>
                        <option value="min">{t("Min", "最小值")}</option>
                        <option value="max">{t("Max", "最大值")}</option>
                    </select>
                    <div style="font-size:0.8rem; margin-top:3px; font-weight:700; color:var(--primary);">{summary_val}</div>
                </td>
            }
        })
        .collect();

    let fx_ref_label = move || {
        active
            .get()
            .map(|(r, c)| format!("{}{}", col_letter(c), r + 1))
            .unwrap_or_else(|| "-".to_string())
    };

    let do_commit_fx = move || {
        let Some((r, c)) = active.get_untracked() else {
            return;
        };
        let val = fx_value.get_untracked();
        cells.update(|rows| {
            if let Some(row) = rows.get_mut(r) {
                if let Some(cell) = row.get_mut(c) {
                    *cell = val.clone();
                }
            }
        });
        let col_name = col_names.with_value(|cols| cols.get(c).cloned().unwrap_or_default());
        let rowid = row_ids.with_value(|ids| ids.get(r).copied()).unwrap_or(0);
        save_cell(
            project_id.with_value(|s| s.clone()),
            file_path.with_value(|s| s.clone()),
            table_name.with_value(|s| s.clone()),
            col_name,
            rowid,
            val,
        );
    };

    view! {
        <div class="formula-bar-container">
            <div class="formula-name-box">{fx_ref_label}</div>
            <span class="formula-fx">"fx"</span>
            <input
                class="formula-input"
                placeholder=t(
                    "Value, or a formula: =SUM(A1:A5), =IF(A1>10,B1,C1), =ROUND(A1/B1,2)",
                    "输入数值，或公式：=SUM(A1:A5)，=IF(A1>10,B1,C1)，=ROUND(A1/B1,2)",
                )
                title=t(
                    "Formulas start with '=': SUM, AVG/AVERAGE, COUNT, MIN, MAX over a range (A1:A5 or a whole column A:A); ABS, SQRT, ROUND(x,digits), POWER(x,y), MOD(a,b), IF(cond,then,else); +-*/^ and comparisons (> < >= <= = <>) with parentheses on cell refs and numbers.",
                    "公式以“=”开头：SUM、AVG/AVERAGE、COUNT、MIN、MAX 可作用于一个区域（如 A1:A5 或整列 A:A）；还支持 ABS、SQRT、ROUND(x,位数)、POWER(x,y)、MOD(a,b)、IF(条件,真值,假值)；以及 +-*/^ 运算符和比较运算符（> < >= <= = <>），可配合括号使用。",
                )
                prop:value=move || fx_value.get()
                on:input=move |ev| fx_value.set(event_target_value(&ev))
                on:keydown=move |ev| { if ev.key() == "Enter" { do_commit_fx(); } }
            />
            <button type="button" class="btn btn-primary btn-sm" on:click=move |_| do_commit_fx()>{t("✓ Commit", "✓ 提交")}</button>
        </div>

        <div class="formula-bar-container" style="gap:0.5rem;">
            <button
                type="button"
                class="btn btn-sm"
                class:btn-primary=move || active_style().bold
                class:btn-secondary=move || !active_style().bold
                title=t("Bold", "加粗")
                disabled=move || active.get().is_none()
                on:click=move |_| apply_style(Box::new(|s| s.bold = !s.bold))
            >
                <strong>"B"</strong>
            </button>
            <button
                type="button"
                class="btn btn-sm"
                class:btn-primary=move || active_style().italic
                class:btn-secondary=move || !active_style().italic
                title=t("Italic", "斜体")
                disabled=move || active.get().is_none()
                on:click=move |_| apply_style(Box::new(|s| s.italic = !s.italic))
            >
                <em>"I"</em>
            </button>
            <label style="display:flex; align-items:center; gap:0.3rem; font-size:0.75rem; color:var(--text-sub);">
                {t("Text", "文字")}
                <input
                    type="color"
                    style="width:26px; height:26px; border:none; padding:0; cursor:pointer;"
                    disabled=move || active.get().is_none()
                    prop:value=move || active_style().color.unwrap_or_else(|| "#0f172a".to_string())
                    on:input=move |ev| { let v = event_target_value(&ev); apply_style(Box::new(move |s| s.color = Some(v.clone()))); }
                />
            </label>
            <label style="display:flex; align-items:center; gap:0.3rem; font-size:0.75rem; color:var(--text-sub);">
                {t("Fill", "填充")}
                <input
                    type="color"
                    style="width:26px; height:26px; border:none; padding:0; cursor:pointer;"
                    disabled=move || active.get().is_none()
                    prop:value=move || active_style().bg_color.unwrap_or_else(|| "#ffffff".to_string())
                    on:input=move |ev| { let v = event_target_value(&ev); apply_style(Box::new(move |s| s.bg_color = Some(v.clone()))); }
                />
            </label>
            <button
                type="button"
                class="btn btn-ghost btn-sm"
                title=t("Clear formatting", "清除格式")
                disabled=move || active.get().is_none()
                on:click=move |_| apply_style(Box::new(|s| *s = CellStyle::default()))
            >
                {t("✕ Clear", "✕ 清除")}
            </button>
        </div>

        <div class="spreadsheet-grid-wrap">
            <table class="spreadsheet-grid">
                <thead>
                    <tr>
                        <th class="row-index-cell" style="cursor:pointer;" on:click=move |_| highlight.set(Highlight::All)>"◰"</th>
                        {header_cells}
                    </tr>
                </thead>
                <tbody>{body_rows}</tbody>
                <tfoot>
                    <tr class="summary-row">
                        <td class="row-index-cell" style="font-weight:700; color:var(--primary);">"Σ"</td>
                        {summary_cells}
                    </tr>
                </tfoot>
            </table>
        </div>
    }
}

#[cfg(feature = "hydrate")]
fn set_delete_row_target(rowid: Option<i64>) {
    let Some(rowid) = rowid else { return };
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    if let Some(el) = doc.get_element_by_id("del-row-id-val") {
        use wasm_bindgen::JsCast;
        if let Ok(input) = el.dyn_into::<web_sys::HtmlInputElement>() {
            input.set_value(&rowid.to_string());
        }
    }
}

#[cfg(not(feature = "hydrate"))]
fn set_delete_row_target(_rowid: Option<i64>) {}

#[cfg(feature = "hydrate")]
fn save_cell(
    project_id: String,
    file_path: String,
    table_name: String,
    col: String,
    rowid: i64,
    val: String,
) {
    wasm_bindgen_futures::spawn_local(async move {
        let body = serde_json::json!({
            "file": file_path,
            "table": table_name,
            "row_id_col": "rowid",
            "row_id_val": rowid.to_string(),
            "col": col,
            "val": val,
        });
        let _ = gloo_net::http::Request::post(&format!("/projects/{}/table/cell-edit", project_id))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(serde_urlencoded_body(&body))
            .expect("valid form body")
            .send()
            .await;
    });
}

#[cfg(not(feature = "hydrate"))]
fn save_cell(
    _project_id: String,
    _file_path: String,
    _table_name: String,
    _col: String,
    _rowid: i64,
    _val: String,
) {
}

#[cfg(feature = "hydrate")]
fn save_style(
    project_id: String,
    file_path: String,
    table_name: String,
    col: String,
    rowid: i64,
    style: CellStyle,
) {
    wasm_bindgen_futures::spawn_local(async move {
        let body = serde_json::json!({
            "file": file_path,
            "table": table_name,
            "row_id": rowid,
            "col": col,
            "bold": style.bold,
            "italic": style.italic,
            "color": style.color,
            "bg_color": style.bg_color,
        });
        let _ =
            gloo_net::http::Request::post(&format!("/projects/{}/table/cell-style", project_id))
                .json(&body)
                .expect("valid json body")
                .send()
                .await;
    });
}

#[cfg(not(feature = "hydrate"))]
fn save_style(
    _project_id: String,
    _file_path: String,
    _table_name: String,
    _col: String,
    _rowid: i64,
    _style: CellStyle,
) {
}

/// The `table/cell-edit` endpoint expects a classic `application/x-www-form-urlencoded` body
/// (it's shared with the plain HTML `<form>` fallback), not JSON -- encode the same fields the
/// original hand-written JS sent via `URLSearchParams`.
#[cfg(feature = "hydrate")]
fn serde_urlencoded_body(body: &serde_json::Value) -> String {
    body.as_object()
        .map(|obj| {
            obj.iter()
                .map(|(k, v)| {
                    let v = v.as_str().unwrap_or_default();
                    format!("{}={}", urlencode(k), urlencode(v))
                })
                .collect::<Vec<_>>()
                .join("&")
        })
        .unwrap_or_default()
}

#[cfg(feature = "hydrate")]
fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            | b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            },
            | _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}
