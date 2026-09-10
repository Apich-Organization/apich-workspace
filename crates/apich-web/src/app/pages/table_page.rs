use crate::app::components::{ActiveNav, AppShell};
use crate::services::sqlite_table::{DatabaseFileInfo, DatabaseSchema, SqlExecutionResult, TableDataPage};
use crate::ui::i18n::I18n;
use apich_db::{Project, User};
use apich_islands::SpreadsheetIsland;
use leptos::prelude::*;

#[allow(clippy::too_many_arguments)]
#[component]
pub fn TablePage(
    user: User,
    is_org_or_team_admin: bool,
    project: Project,
    databases: Vec<DatabaseFileInfo>,
    selected_file: Option<String>,
    schema: Option<DatabaseSchema>,
    selected_table: Option<String>,
    table_data: Option<TableDataPage>,
    sql_query: String,
    sql_result: Option<SqlExecutionResult>,
    mode: String,
    search: Option<String>,
    notice: Option<String>,
    error: Option<String>,
    i18n: I18n,
    current_path: String,
) -> impl IntoView {
    let project_id = project.id;

    let alert = if let Some(n) = notice {
        Some(view! { <div class="alert alert-success" style="margin-bottom:1rem;">{n}</div> }.into_any())
    } else {
        error.map(|e| view! { <div class="alert alert-danger" style="margin-bottom:1rem;">{e}</div> }.into_any())
    };

    let cur_file = selected_file.clone().unwrap_or_else(|| "data.db".to_string());

    let db_selector = if databases.is_empty() {
        view! {
            <div class="empty-state" style="margin-bottom:1.5rem;">
                <h3 class="empty-title">"No SQLite Databases Found"</h3>
                <p class="empty-desc">"Every table in APICH is a physical SQLite database file (.db/.sqlite/.table) in your version-controlled repository."</p>
                <form method="post" action=format!("/projects/{}/table/create-db", project_id)>
                    <button type="submit" class="btn btn-primary">"+ Initialize data.db"</button>
                </form>
            </div>
        }.into_any()
    } else {
        let pills: Vec<_> = databases
            .iter()
            .map(|db| {
                let is_sel = selected_file.as_deref() == Some(db.relative_path.as_str());
                let size_kb = db.size_bytes.div_ceil(1024);
                view! {
                    <a href=format!("/projects/{}/table?file={}&mode={}", project_id, urlencoding::encode(&db.relative_path), mode) class="db-file-pill" class:active=is_sel>
                        "📁 " {db.relative_path.clone()} " (" {size_kb} " KB)"
                    </a>
                }
            })
            .collect();
        view! {
            <div style="margin-bottom:1rem;">
                <div style="font-size:0.775rem; font-weight:600; color:var(--text-sub); margin-bottom:0.35rem; text-transform:uppercase; letter-spacing:0.5px;">"Active Database File:"</div>
                <div class="db-selector-bar">
                    {pills}
                    <form method="post" action=format!("/projects/{}/table/create-db", project_id) class="inline-form" style="margin-left:auto;">
                        <button type="submit" class="btn btn-ghost btn-sm">"+ New .db File"</button>
                    </form>
                </div>
            </div>
        }.into_any()
    };

    let main_view = match &schema {
        Some(s) => render_schema_view(&project, s, selected_table.as_deref(), table_data.as_ref(), &cur_file, &sql_query, sql_result.as_ref(), &mode, search.as_deref()).into_any(),
        None => view! { <div></div> }.into_any(),
    };

    view! {
        <AppShell
            user=user.clone()
            is_org_or_team_admin=is_org_or_team_admin
            active_nav=ActiveNav::Projects
            current_path=current_path
            page_title=format!("{} - Table", project.name)
            i18n=i18n
        >
            <div class="page-header">
                <div>
                    <h1 class="page-title">"📊 " {project.name.clone()} " • " {cur_file.clone()}</h1>
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
    cur_file: &str,
    sql_query: &str,
    sql_result: Option<&SqlExecutionResult>,
    mode: &str,
    search: Option<&str>,
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

    let sql_console = render_sql_console(project_id, cur_file, selected_table.or_else(|| schema.tables.first().map(|t| t.name.as_str())), sql_query, sql_result, mode);

    let grid = match table_data {
        Some(td) => render_grid(project, td, schema, cur_file, mode, search).into_any(),
        None if !schema.tables.is_empty() => view! { <div class="empty-state"><p>"Select a table tab above to inspect rows."</p></div> }.into_any(),
        None => view! { <div class="empty-state"><p>"Database is empty. Use the SQL console below to create tables."</p></div> }.into_any(),
    };

    view! {
        <div class="table-tabs" style="margin-bottom:0.75rem;">{table_tabs}</div>
        {grid}
        {sql_console}
    }
}

fn render_grid(project: &Project, td: &TableDataPage, schema: &DatabaseSchema, cur_file: &str, mode: &str, search: Option<&str>) -> impl IntoView {
    let project_id = project.id;
    let cur_tbl_schema = schema.tables.iter().find(|t| t.name == td.table_name);

    let column_types: Vec<String> = td
        .columns
        .iter()
        .map(|col| {
            cur_tbl_schema
                .and_then(|ts| ts.columns.iter().find(|c| &c.name == col))
                .map(|c| c.data_type.clone())
                .unwrap_or_else(|| "TEXT".to_string())
        })
        .collect();
    let primary_keys: Vec<bool> = td
        .columns
        .iter()
        .map(|col| {
            cur_tbl_schema
                .and_then(|ts| ts.columns.iter().find(|c| &c.name == col))
                .map(|c| c.is_primary_key)
                .unwrap_or(false)
        })
        .collect();
    let rows: Vec<Vec<String>> = td
        .rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|cell| match cell {
                    serde_json::Value::Null => String::new(),
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
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
    let prev_page = if td.page > 1 { td.page - 1 } else { 1 };
    let next_page = if td.page < td.total_pages { td.page + 1 } else { td.total_pages };
    let prev_url = format!("/projects/{}/table?file={}&table={}&page={}&search={}&mode={}", project_id, urlencoding::encode(cur_file), urlencoding::encode(&td.table_name), prev_page, urlencoding::encode(&cur_search), mode);
    let next_url = format!("/projects/{}/table?file={}&table={}&page={}&search={}&mode={}", project_id, urlencoding::encode(cur_file), urlencoding::encode(&td.table_name), next_page, urlencoding::encode(&cur_search), mode);

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
                        let import_action = format!("/projects/{}/table/import", project_id);
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
                    <a href=format!("/projects/{}/table/export?file={}&table={}", project_id, cur_file, td.table_name) class="btn btn-secondary btn-sm">"📤 Export CSV"</a>
                </div>
                <div style="display:flex; gap:0.5rem; align-items:center;">
                    <form method="get" action=format!("/projects/{}/table", project_id) style="display:flex; gap:0.35rem; align-items:center;">
                        <input type="hidden" name="file" value=cur_file.to_string() />
                        <input type="hidden" name="table" value=td.table_name.clone() />
                        <input type="hidden" name="mode" value=mode.to_string() />
                        <input type="text" name="search" value=cur_search.clone() placeholder="Search cells..." class="form-control" style="width:180px; padding:0.25rem 0.6rem; font-size:0.8rem;" />
                        <button type="submit" class="btn btn-secondary btn-sm">"Search"</button>
                    </form>
                    <span style="font-size:0.775rem; color:var(--text-sub);">"Total: "<strong>{td.total_rows}</strong></span>
                </div>
            </div>

            <SpreadsheetIsland
                project_id=project_id.to_string()
                file_path=cur_file.to_string()
                table_name=td.table_name.clone()
                columns=td.columns.clone()
                column_types=column_types
                primary_keys=primary_keys
                rows=rows
                row_ids=td.row_ids.clone()
                cell_styles=cell_styles
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

fn render_sql_console(project_id: uuid::Uuid, cur_file: &str, table_name: Option<&str>, sql_query: &str, sql_result: Option<&SqlExecutionResult>, mode: &str) -> impl IntoView {
    let default_sql = if !sql_query.is_empty() {
        sql_query.to_string()
    } else if let Some(t) = table_name {
        format!("SELECT * FROM \"{}\" LIMIT 50;", t)
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

    view! {
        <details style="margin-top:1.5rem; background:#0f172a; border-radius:10px; padding:1.25rem; color:#fff;" open=is_open>
            <summary style="cursor:pointer; font-weight:700; color:#38bdf8; outline:none;">"💻 SQLite Console & Raw SQL (click to expand)"</summary>
            <form method="post" action=format!("/projects/{}/table/sql", project_id) style="margin-top:1rem;">
                <input type="hidden" name="file" value=cur_file.to_string() />
                <textarea name="sql" class="sql-textarea" style="width:100%; height:80px; background:#1e293b; color:#f8fafc; border:1px solid #334155; border-radius:6px; font-family:var(--font-mono); font-size:0.85rem; padding:0.75rem;" required=true>{default_sql}</textarea>
                <div style="display:flex; justify-content:space-between; align-items:center; margin-top:0.5rem;">
                    <span style="font-size:0.75rem; color:#94a3b8;">"Target: "<code>{cur_file.to_string()}</code></span>
                    <button type="submit" class="btn btn-primary btn-sm">"▶ Run SQL"</button>
                </div>
            </form>
            {result}
        </details>
    }
}
