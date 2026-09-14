use crate::app::components::ActiveNav;
use crate::app::components::AppShell;
use crate::services::sqlite_table::ColumnViewConfig;
use crate::services::sqlite_table::DatabaseFileInfo;
use crate::services::sqlite_table::DatabaseSchema;
use crate::services::sqlite_table::NotebookCell;
use crate::services::sqlite_table::SqlExecutionResult;
use crate::services::sqlite_table::TableDataPage;
use crate::ui::i18n::I18n;
use apich_db::Project;
use apich_db::User;
use apich_islands::SpreadsheetIsland;
use leptos::prelude::*;

pub struct TablePageState {
    pub databases: Vec<DatabaseFileInfo>,
    pub selected_file: Option<String>,
    pub schema: Option<DatabaseSchema>,
    pub selected_table: Option<String>,
    pub table_data: Option<TableDataPage>,
    pub column_view: ColumnViewConfig,
    pub query_history: Vec<String>,
    pub sql_query: String,
    pub sql_result: Option<SqlExecutionResult>,
    pub mode: String,
    pub search: Option<String>,
    pub notebook_cells: Vec<NotebookCell>,
}

#[component]
pub fn TablePage(
    user: User,
    is_org_or_team_admin: bool,
    project: Project,
    state: TablePageState,
    notice: Option<String>,
    error: Option<String>,
    i18n: I18n,
    current_path: String,
) -> impl IntoView {
    let TablePageState {
        databases,
        selected_file,
        schema,
        selected_table,
        table_data,
        column_view,
        query_history,
        sql_query,
        sql_result,
        mode,
        search,
        notebook_cells,
    } = state;

    let project_id = project.id;

    let alert = notice.map_or_else(
        || {
            error.map(|e| {
                view! { <div class="alert alert-danger" style="margin-bottom:1rem;">{e}</div> }
                    .into_any()
            })
        },
        |n| {
            Some(
                view! { <div class="alert alert-success" style="margin-bottom:1rem;">{n}</div> }
                    .into_any(),
            )
        },
    );

    let cur_file = selected_file
        .clone()
        .unwrap_or_else(|| "data.db".to_string());

    let db_selector = if databases.is_empty() {
        view! {
            <div class="empty-state" style="margin-bottom:1.5rem;">
                <h3 class="empty-title">{i18n.table_no_tables_title()}</h3>
                <p class="empty-desc">{i18n.table_no_tables_desc()}</p>
                <form method="post" action=format!("/projects/{}/table/create-db", project_id)>
                    <button type="submit" class="btn btn-primary">{i18n.table_init_first()}</button>
                </form>
            </div>
        }
        .into_any()
    } else {
        let pills: Vec<_> = databases
            .iter()
            .map(|db| {
                let is_sel = selected_file.as_deref() == Some(db.relative_path.as_str());
                let size_kb = db.size_bytes.div_ceil(1024);
                view! {
                    <a href=format!("/projects/{}/table?file={}&mode={}", project_id, urlencoding::encode(&db.relative_path), mode) class="db-file-pill" class:active=is_sel>
                        "📊 " {db.relative_path.clone()} " (" {size_kb} " KB)"
                    </a>
                }
            })
            .collect();
        view! {
            <div style="margin-bottom:1rem;">
                <div style="font-size:0.775rem; font-weight:600; color:var(--text-sub); margin-bottom:0.35rem; text-transform:uppercase; letter-spacing:0.5px;">{i18n.table_active_file_label()}</div>
                <div class="db-selector-bar">
                    {pills}
                    <form method="post" action=format!("/projects/{}/table/create-db", project_id) class="inline-form" style="margin-left:auto;">
                        <button type="submit" class="btn btn-ghost btn-sm">"+ "{i18n.table_new_file()}</button>
                    </form>
                </div>
            </div>
        }.into_any()
    };

    let main_view = match &schema {
        | Some(s) => {
            render_schema_view(
                &project,
                s,
                selected_table.as_deref(),
                table_data.as_ref(),
                &column_view,
                &query_history,
                &cur_file,
                &sql_query,
                sql_result.as_ref(),
                &mode,
                search.as_deref(),
                notebook_cells,
                i18n.is_zh(),
            )
            .into_any()
        },
        | None => view! { <div></div> }.into_any(),
    };

    view! {
        <AppShell
            user=user
            is_org_or_team_admin=is_org_or_team_admin
            active_nav=ActiveNav::Projects
            current_path=current_path
            page_title=format!("{} - Table", project.name)
            i18n=i18n
        >
            <div class="page-header" style="display:flex; justify-content:space-between; align-items:center; flex-wrap:wrap; gap:0.75rem;">
                <div>
                    <h1 class="page-title">"📊 " {project.name.clone()} " • " {cur_file.clone()}</h1>
                </div>
                <div style="display:flex; align-items:center; gap:0.5rem;">
                    <a href=format!("/projects/{}?tab=vcs", project_id) class="btn btn-secondary btn-sm">"🌿 VCS History"</a>
                    {if project.settings.get("is_single_file").and_then(|v| v.as_bool()).unwrap_or(false) {
                        let cf_file = cur_file.clone();
                        view! {
                            <form
                                method="post"
                                action=format!("/projects/{}/delete", project_id)
                                class="inline-form"
                                onsubmit=format!("return confirm('Are you sure you want to permanently delete \"{}\"? This cannot be undone.');", cf_file)
                            >
                                <button
                                    type="submit"
                                    class="btn btn-danger btn-sm"
                                    title="Permanently delete this file"
                                >
                                    "🗑️ Delete"
                                </button>
                            </form>
                            <a href="/" class="btn btn-outline btn-sm">"← Back to Dashboard"</a>
                        }.into_any()
                    } else {
                        view! { <a href=format!("/projects/{}?tab=files", project_id) class="btn btn-outline btn-sm">"📁 Back to Files"</a> }.into_any()
                    }}
                </div>
            </div>
            {alert}
            {db_selector}
            <div class="db-container">{main_view}</div>
        </AppShell>
    }
}

#[allow(clippy::too_many_arguments)]
fn render_schema_view(
    project: &Project,
    schema: &DatabaseSchema,
    selected_table: Option<&str>,
    table_data: Option<&TableDataPage>,
    column_view: &ColumnViewConfig,
    query_history: &[String],
    cur_file: &str,
    sql_query: &str,
    sql_result: Option<&SqlExecutionResult>,
    mode: &str,
    search: Option<&str>,
    notebook_cells: Vec<NotebookCell>,
    is_zh: bool,
) -> impl IntoView {
    let project_id = project.id;

    let table_tabs: Vec<_> = schema
        .tables
        .iter()
        .map(|t| {
            let is_sel = selected_table == Some(t.name.as_str());
            let icon = if t.is_view { "👁️" } else { "📑" };
            view! {
                <a
                    href=format!("/projects/{}/table?file={}&table={}&mode={}", project_id, urlencoding::encode(cur_file), urlencoding::encode(&t.name), mode)
                    class="table-tab-btn"
                    class:active=is_sel
                >
                    {icon} " " {t.name.clone()} <span style="font-size:0.7rem; opacity:0.8; margin-left:4px;">"(" {t.row_count} ")"</span>
                </a>
            }
        })
        .collect();

    let sql_table_name = selected_table.or_else(|| schema.tables.first().map(|t| t.name.as_str()));
    let sql_first_column = sql_table_name
        .and_then(|t| schema.tables.iter().find(|table| table.name == t))
        .and_then(|t| t.columns.first())
        .map(|c| c.name.clone());
    let sql_console = render_sql_console(
        project_id,
        cur_file,
        sql_table_name,
        sql_query,
        sql_result,
        mode,
        sql_first_column,
        query_history,
        is_zh,
    );

    let grid = match table_data {
        Some(td) => render_grid(project, td, schema, column_view, cur_file, mode, search, is_zh).into_any(),
        None if !schema.tables.is_empty() => view! { <div class="empty-state"><p>"Select a table tab above to inspect rows."</p></div> }.into_any(),
        None => view! { <div class="empty-state"><p>"Database is empty. Use the SQL console below to create tables."</p></div> }.into_any(),
    };

    let notebook_section = selected_table.map(|tbl| {
        let cells: Vec<apich_islands::NotebookCellData> = notebook_cells
            .into_iter()
            .map(|c| apich_islands::NotebookCellData {
                id: c.id,
                language: c.language,
                code: c.code,
                output: c.output,
                output_images: c.output_images.into_iter().map(|img| apich_islands::NotebookCellImageData { name: img.name, data_uri: img.data_uri }).collect(),
            })
            .collect();
        view! {
            <details style="margin-top:1.25rem; background:var(--bg-surface); border:1px solid var(--border-subtle); border-radius:10px; padding:1.25rem;" open=mode == "notebook">
                <summary style="cursor:pointer; font-weight:700; outline:none;">"🐍 Python / R Notebook (click to expand)"</summary>
                <p class="text-muted" style="font-size:0.8rem; margin:0.5rem 0 1rem;">
                    "Each cell runs its own fresh Python or R process against this table's real data -- no shared state between cells (each cell re-loads what it needs; see the starter code in a new cell). Results, including any plot a cell produces, are saved with the cell."
                </p>
                <apich_islands::NotebookIsland
                    project_id=project_id.to_string()
                    file_path=cur_file.to_string()
                    table_name=tbl.to_string()
                    cells=cells
                />
            </details>
        }
    });

    view! {
        <div class="table-tabs" style="margin-bottom:0.75rem;">{table_tabs}</div>
        {grid}
        {notebook_section}
        {sql_console}
    }
}

/// A "Columns" dropdown next to Export/Import letting a user hide a column, restore a hidden one,
/// nudge a visible column left/right, or reset back to the table's real schema order -- all
/// plain server-rendered forms (no island needed; each just POSTs and redirects back to the same
/// view), matching the same all-server pattern the rest of the ribbon toolbar already uses.
fn render_column_panel(
    project_id: uuid::Uuid,
    cur_file: &str,
    table_name: &str,
    mode: &str,
    all_columns: &[String],
    column_view: &ColumnViewConfig,
) -> impl IntoView {
    let action = format!("/projects/{project_id}/table/column-view");
    let visible = column_view.apply(all_columns);
    let n_visible = visible.len();

    let row_form = |label: String, col_name: String, extra: Vec<(&'static str, String)>| {
        let action = action.clone();
        let cur_file = cur_file.to_string();
        let table_name = table_name.to_string();
        let mode = mode.to_string();
        let extra_inputs: Vec<_> = extra
            .into_iter()
            .map(|(k, v)| view! { <input type="hidden" name=k value=v /> })
            .collect();
        view! {
            <form method="post" action=action class="inline-form" style="display:inline;">
                <input type="hidden" name="file" value=cur_file />
                <input type="hidden" name="table" value=table_name />
                <input type="hidden" name="mode" value=mode />
                <input type="hidden" name="column" value=col_name />
                {extra_inputs}
                <button type="submit" class="btn btn-ghost btn-sm" style="padding:0.15rem 0.4rem;">{label}</button>
            </form>
        }
    };

    let visible_rows: Vec<_> = visible
        .iter()
        .enumerate()
        .map(|(i, name)| {
            view! {
                <div style="display:flex; align-items:center; justify-content:space-between; padding:0.3rem 0; border-bottom:1px solid var(--border-subtle);">
                    <span style="font-size:0.8rem; font-family:var(--font-mono);">{name.clone()}</span>
                    <div style="display:flex; gap:0.15rem;">
                        {(i > 0).then(|| row_form("◀".to_string(), name.clone(), vec![("action", "move-left".to_string())]))}
                        {(i.saturating_add(1) < n_visible).then(|| row_form("▶".to_string(), name.clone(), vec![("action", "move-right".to_string())]))}
                        {row_form("🙈 Hide".to_string(), name.clone(), vec![("action", "hide".to_string())])}
                    </div>
                </div>
            }
        })
        .collect();

    let hidden_rows: Vec<_> = column_view
        .hidden
        .iter()
        .filter(|c| all_columns.contains(c))
        .map(|name| {
            view! {
                <div style="display:flex; align-items:center; justify-content:space-between; padding:0.3rem 0; border-bottom:1px solid var(--border-subtle); opacity:0.6;">
                    <span style="font-size:0.8rem; font-family:var(--font-mono);">{name.clone()}</span>
                    {row_form("👁 Show".to_string(), name.clone(), vec![("action", "show".to_string())])}
                </div>
            }
        })
        .collect();
    let hidden_section = (!hidden_rows.is_empty()).then(|| view! {
        <div style="margin-top:0.5rem; padding-top:0.5rem; border-top:2px solid var(--border-subtle);">
            <div style="font-size:0.7rem; text-transform:uppercase; letter-spacing:0.5px; color:var(--text-sub); margin-bottom:0.25rem;">"Hidden"</div>
            {hidden_rows}
        </div>
    });

    let has_customization = !column_view.order.is_empty() || !column_view.hidden.is_empty();
    let reset_btn = has_customization.then(|| {
        row_form(
            "↺ Reset to default order".to_string(),
            String::new(),
            vec![("action", "reset".to_string())],
        )
    });

    view! {
        <div class="dropdown-menu-wrap" style="position:relative; display:inline-block;">
            <details style="display:inline-block;">
                <summary class="btn btn-secondary btn-sm" style="list-style:none; cursor:pointer;">"🧱 Columns"</summary>
                <div style="position:absolute; z-index:20; background:var(--bg-surface); border:1px solid var(--border-subtle); border-radius:8px; box-shadow:var(--shadow-md); padding:0.6rem; margin-top:0.25rem; min-width:220px; max-height:320px; overflow-y:auto;">
                    {visible_rows}
                    {hidden_section}
                    {reset_btn.map(|b| view! { <div style="margin-top:0.5rem;">{b}</div> })}
                </div>
            </details>
        </div>
    }
}

#[allow(clippy::too_many_arguments)]
fn render_grid(
    project: &Project,
    td: &TableDataPage,
    schema: &DatabaseSchema,
    column_view: &ColumnViewConfig,
    cur_file: &str,
    mode: &str,
    search: Option<&str>,
    is_zh: bool,
) -> impl IntoView {
    let project_id = project.id;
    let cur_tbl_schema = schema.tables.iter().find(|t| t.name == td.table_name);

    // `td.columns` is every real column, in schema order (see `get_table_data`'s `SELECT *`).
    // `visible_order` applies the saved hide/reorder preferences on top of that -- purely a
    // display-time transform, never touching the actual query or schema.
    let orig_index: std::collections::HashMap<&str, usize> = td
        .columns
        .iter()
        .enumerate()
        .map(|(i, c)| (c.as_str(), i))
        .collect();
    let visible_order = column_view.apply(&td.columns);

    let column_types: Vec<String> = visible_order
        .iter()
        .map(|col| {
            cur_tbl_schema
                .and_then(|ts| ts.columns.iter().find(|c| &c.name == col))
                .map_or_else(|| "TEXT".to_string(), |c| c.data_type.clone())
        })
        .collect();
    let primary_keys: Vec<bool> = visible_order
        .iter()
        .map(|col| {
            cur_tbl_schema
                .and_then(|ts| ts.columns.iter().find(|c| &c.name == col))
                .is_some_and(|c| c.is_primary_key)
        })
        .collect();
    let rows: Vec<Vec<String>> = td
        .rows
        .iter()
        .map(|row| {
            visible_order
                .iter()
                .map(|col| {
                    let idx = orig_index.get(col.as_str()).copied().unwrap_or(0);
                    match row.get(idx) {
                        | Some(serde_json::Value::Null) | None => String::new(),
                        | Some(serde_json::Value::String(s)) => s.clone(),
                        | Some(other) => other.to_string(),
                    }
                })
                .collect()
        })
        .collect();

    let cell_styles: Vec<apich_islands::CellStyleEntry> = td
        .styles
        .iter()
        .filter_map(|(key, style)| {
            let (row_id_str, col) = key.split_once(':')?;
            let row_id: i64 = row_id_str.parse().ok()?;
            Some(apich_islands::CellStyleEntry {
                row_id,
                col: col.to_string(),
                style: apich_islands::SpreadsheetCellStyle {
                    bold: style.bold,
                    italic: style.italic,
                    color: style.color.clone(),
                    bg_color: style.bg_color.clone(),
                },
            })
        })
        .collect();

    let cur_search = search.unwrap_or("").to_string();
    let prev_page = if td.page > 1 {
        td.page.saturating_sub(1)
    } else {
        1
    };
    let next_page = if td.page < td.total_pages {
        td.page.saturating_add(1)
    } else {
        td.total_pages
    };
    let prev_url = format!(
        "/projects/{}/table?file={}&table={}&page={}&search={}&mode={}",
        project_id,
        urlencoding::encode(cur_file),
        urlencoding::encode(&td.table_name),
        prev_page,
        urlencoding::encode(&cur_search),
        mode
    );
    let next_url = format!(
        "/projects/{}/table?file={}&table={}&page={}&search={}&mode={}",
        project_id,
        urlencoding::encode(cur_file),
        urlencoding::encode(&td.table_name),
        next_page,
        urlencoding::encode(&cur_search),
        mode
    );

    view! {
        <div class="spreadsheet-box">
            <div class="ribbon-toolbar">
                <div style="display:flex; gap:0.4rem; align-items:center;">
                    <form method="post" action=format!("/projects/{}/table/row-add", project_id) class="inline-form">
                        <input type="hidden" name="file" value=cur_file.to_string() />
                        <input type="hidden" name="table" value=td.table_name.clone() />
                        <button type="submit" class="btn btn-primary btn-sm">"+ Add Row"</button>
                    </form>
                    <form id="form-del-row" method="post" action=format!("/projects/{}/table/row-delete", project_id) class="inline-form">
                        <input type="hidden" name="file" value=cur_file.to_string() />
                        <input type="hidden" name="table" value=td.table_name.clone() />
                        <input type="hidden" id="del-row-id-col" name="row_id_col" value="rowid" />
                        <input type="hidden" id="del-row-id-val" name="row_id_val" value="" />
                        <apich_islands::DeleteRowButtonIsland label="🗑️ Delete Row".to_string() />
                    </form>
                    {
                        let import_action = format!("/projects/{project_id}/table/import");
                        let cur_file_owned = cur_file.to_string();
                        let table_name_owned = td.table_name.clone();
                        view! {
                            <apich_islands::ModalIsland trigger_label="📥 Import CSV".to_string() trigger_class="btn btn-secondary btn-sm".to_string() title="Import CSV Data into Table".to_string()>
                                <form method="post" action=import_action>
                                    <input type="hidden" name="file" value=cur_file_owned />
                                    <input type="hidden" name="table" value=table_name_owned />
                                    <div class="form-group">
                                        <label>"Paste RFC-4180 CSV Data (with Header Row)"</label>
                                        <textarea name="csv_data" rows="8" class="form-control" style="font-family:var(--font-mono); font-size:0.825rem;" required=true placeholder="sample_id,frequency_ghz,fidelity"></textarea>
                                    </div>
                                    <div style="display:flex; justify-content:flex-end; gap:0.5rem; margin-top:1.25rem;">
                                        <button type="submit" class="btn btn-primary">"Import Rows"</button>
                                    </div>
                                </form>
                            </apich_islands::ModalIsland>
                        }
                    }
                    {render_column_panel(project_id, cur_file, &td.table_name, mode, &td.columns, column_view)}
                    {
                        let export_base = format!("/projects/{}/table/export?file={}&table={}", project_id, urlencoding::encode(cur_file), urlencoding::encode(&td.table_name));
                        view! {
                            <div class="dropdown-menu-wrap" style="position:relative; display:inline-block;">
                                <details style="display:inline-block;">
                                    <summary class="btn btn-secondary btn-sm" style="list-style:none; cursor:pointer;">"📤 Export"</summary>
                                    <div style="position:absolute; z-index:20; background:var(--bg-surface); border:1px solid var(--border-subtle); border-radius:8px; box-shadow:var(--shadow-md); padding:0.35rem; margin-top:0.25rem; min-width:140px;">
                                        <a href=format!("{}&format=csv", export_base) class="dropdown-item" style="display:block; padding:0.4rem 0.6rem; font-size:0.8rem; border-radius:6px; color:var(--text-main); text-decoration:none;">"CSV"</a>
                                        <a href=format!("{}&format=tsv", export_base) class="dropdown-item" style="display:block; padding:0.4rem 0.6rem; font-size:0.8rem; border-radius:6px; color:var(--text-main); text-decoration:none;">"TSV"</a>
                                        <a href=format!("{}&format=json", export_base) class="dropdown-item" style="display:block; padding:0.4rem 0.6rem; font-size:0.8rem; border-radius:6px; color:var(--text-main); text-decoration:none;">"JSON"</a>
                                        <a href=format!("{}&format=md", export_base) class="dropdown-item" style="display:block; padding:0.4rem 0.6rem; font-size:0.8rem; border-radius:6px; color:var(--text-main); text-decoration:none;">"Markdown"</a>
                                    </div>
                                </details>
                            </div>
                        }
                    }
                </div>
                <div style="display:flex; gap:0.5rem; align-items:center;">
                    <form method="get" action=format!("/projects/{}/table", project_id) style="display:flex; gap:0.35rem; align-items:center;">
                        <input type="hidden" name="file" value=cur_file.to_string() />
                        <input type="hidden" name="table" value=td.table_name.clone() />
                        <input type="hidden" name="mode" value=mode.to_string() />
                        <input type="text" name="search" value=cur_search placeholder="Search cells..." class="form-control" style="width:180px; padding:0.25rem 0.6rem; font-size:0.8rem;" />
                        <button type="submit" class="btn btn-secondary btn-sm">"Search"</button>
                    </form>
                    <span style="font-size:0.775rem; color:var(--text-sub);">"Total: "<strong>{td.total_rows}</strong></span>
                </div>
            </div>

            <SpreadsheetIsland
                project_id=project_id.to_string()
                file_path=cur_file.to_string()
                table_name=td.table_name.clone()
                columns=visible_order
                column_types=column_types
                primary_keys=primary_keys
                rows=rows
                row_ids=td.row_ids.clone()
                cell_styles=cell_styles
                is_zh=is_zh
            />

            <div style="display:flex; justify-content:space-between; align-items:center; padding:0.65rem 1rem; background:var(--bg-muted); border-top:1px solid var(--border-subtle); font-size:0.8rem;">
                <div>"Page "<strong>{td.page}</strong>" of "<strong>{td.total_pages}</strong></div>
                <div style="display:flex; gap:0.5rem;">
                    <a href=prev_url class="btn btn-secondary btn-sm">"< Previous"</a>
                    <a href=next_url class="btn btn-secondary btn-sm">"Next >"</a>
                </div>
            </div>
        </div>
    }
}

#[allow(clippy::too_many_arguments)]
fn render_sql_console(
    project_id: uuid::Uuid,
    cur_file: &str,
    table_name: Option<&str>,
    sql_query: &str,
    sql_result: Option<&SqlExecutionResult>,
    mode: &str,
    first_column: Option<String>,
    query_history: &[String],
    is_zh: bool,
) -> impl IntoView {
    let default_sql = if !sql_query.is_empty() {
        sql_query.to_string()
    } else if let Some(t) = table_name {
        format!("SELECT * FROM \"{t}\" LIMIT 50;")
    } else {
        "SELECT 1;".to_string()
    };

    let result = sql_result.map(|res| {
        let status_class = if res.is_query { "alert-success" } else { "alert-info" };
        let table_html = (res.is_query && !res.columns.is_empty()).then(|| {
            let thead: Vec<_> = res.columns.iter().map(|c| view! { <th>{c.clone()}</th> }).collect();
            let tbody: Vec<_> = res
                .rows
                .iter()
                .map(|row| {
                    let cells: Vec<_> = row
                        .iter()
                        .map(|cell| {
                            let s = match cell {
                                serde_json::Value::Null => view! { <span style="color:var(--text-sub);">"NULL"</span> }.into_any(),
                                serde_json::Value::String(s) => s.clone().into_any(),
                                other => other.to_string().into_any(),
                            };
                            view! { <td>{s}</td> }
                        })
                        .collect();
                    view! { <tr>{cells}</tr> }
                })
                .collect();
            view! {
                <div class="sql-result-wrap" style="margin-top:0.75rem; max-height:260px; overflow:auto;">
                    <table class="sql-table" style="width:100%; border-collapse:collapse; font-size:0.8rem;">
                        <thead><tr>{thead}</tr></thead>
                        <tbody>{tbody}</tbody>
                    </table>
                </div>
            }
        });
        view! {
            <div style="margin-top:0.75rem;">
                <div class=format!("alert {}", status_class) style="padding:0.5rem 0.75rem; font-size:0.8rem; margin-bottom:0.5rem;">{res.message.clone()}</div>
                {table_html}
            </div>
        }
    });

    let is_open = mode == "sql" || sql_result.is_some();

    let sql_input = match table_name {
        Some(t) => view! {
            <apich_islands::SqlConsoleIsland initial_sql=default_sql table_name=t.to_string() first_column=first_column query_history=query_history.to_vec() is_zh=is_zh />
        }.into_any(),
        None => view! {
            <textarea name="sql" class="sql-textarea" style="width:100%; height:80px; background:#1e293b; color:#f8fafc; border:1px solid #334155; border-radius:6px; font-family:var(--font-mono); font-size:0.85rem; padding:0.75rem;" required=true>{default_sql}</textarea>
        }.into_any(),
    };

    view! {
        <details style="margin-top:1.5rem; background:#0f172a; border-radius:10px; padding:1.25rem; color:#fff;" open=is_open>
            <summary style="cursor:pointer; font-weight:700; color:#38bdf8; outline:none;">"💻 SQLite Console & Raw SQL (click to expand)"</summary>
            <form method="post" action=format!("/projects/{}/table/sql", project_id) style="margin-top:1rem;">
                <input type="hidden" name="file" value=cur_file.to_string() />
                {sql_input}
                <div style="display:flex; justify-content:space-between; align-items:center; margin-top:0.5rem;">
                    <span style="font-size:0.75rem; color:#94a3b8; display:flex; align-items:center; gap:0.6rem;">
                        "Target: "<code>{cur_file.to_string()}</code>
                        <label style="display:flex; align-items:center; gap:0.3rem;">
                            "Row limit:"
                            <input type="number" name="max_rows" value="500" min="1" max="5000" style="width:70px; background:#1e293b; color:#f8fafc; border:1px solid #334155; border-radius:4px; padding:0.15rem 0.35rem; font-size:0.75rem;" />
                        </label>
                    </span>
                    <button type="submit" class="btn btn-primary btn-sm" title="Ctrl+Enter / Cmd+Enter also runs">"▶ Run SQL"</button>
                </div>
            </form>
            {result}
        </details>
    }
}
