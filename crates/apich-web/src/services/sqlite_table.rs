use crate::error::WebError;
use crate::error::WebResult;
use rusqlite::types::ValueRef;
use rusqlite::Connection;
use rusqlite::OpenFlags;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;

/// Per-cell display formatting (bold/italic/text color/background color) -- tables previously
/// could only show plain, unstyled data-typed text, unlike a real spreadsheet. Stored separately
/// from the table's own data, in a small `_apich_cell_styles` metadata table alongside it in the
/// same SQLite file (so it travels with the file, survives export/import of the real data, and
/// never risks corrupting the user's actual columns/types) -- keyed by `rowid` + column name,
/// the same row-identity scheme `update_cell`/`delete_row` already use.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CellStyle {
    pub bold: bool,
    pub italic: bool,
    pub color: Option<String>,
    pub bg_color: Option<String>,
}

impl CellStyle {
    fn is_default(&self) -> bool {
        *self == CellStyle::default()
    }
}

/// Per-table column display preferences -- which real schema columns to hide and what order to
/// show the rest in. Stored the same way as `CellStyle` (a small `_apich_column_view` metadata
/// table inside the same SQLite file), and applied purely at render time in `table_page.rs`: it
/// never touches the actual table schema, so hiding/reordering a column is always reversible and
/// never risks the user's real data or SQL queries against it.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ColumnViewConfig {
    /// Explicit display order for the columns named here; any real column not listed is appended
    /// afterward in its original schema order.
    pub order: Vec<String>,
    /// Column names hidden from the grid (still fully queryable via the SQL console).
    pub hidden: Vec<String>,
}

impl ColumnViewConfig {
    /// Apply this config to a table's real column list, producing the final visible, ordered
    /// column name list the grid should render.
    pub fn apply(
        &self,
        all_columns: &[String],
    ) -> Vec<String> {
        let mut ordered: Vec<String> = self
            .order
            .iter()
            .filter(|c| all_columns.contains(c))
            .cloned()
            .collect();
        for c in all_columns {
            if !ordered.contains(c) {
                ordered.push(c.clone());
            }
        }
        ordered.retain(|c| !self.hidden.contains(c));
        ordered
    }
}

/// One Python/R code cell in a table's notebook -- "integrate python and r into the tables ...
/// just like a python notebook and r notebook" (direct user request). Design, kept deliberately
/// close to the codebase's own existing patterns rather than inventing new infrastructure:
///
/// - **Storage**: same approach as `CellStyle` above -- a `_apich_notebook_cells` metadata table
///   living inside the table's own `.db`/`.table` SQLite file, scoped per real data table via
///   `table_name`. Cells travel with the file (export/import, git sync) and can never corrupt the
///   user's actual columns.
/// - **Execution**: NOT a persistent kernel (a real Jupyter/IRkernel process staying alive across
///   cell runs, with in-memory state carried cell-to-cell) -- that would be a much larger, riskier
///   piece of infrastructure (a long-lived stateful process per open notebook, lifecycle/cleanup
///   to get right, no existing precedent in this codebase to build on). Each "Run" writes the
///   cell's current code to a real `.py`/`.r` file and executes it via the *already-existing*
///   `ProjectManagerService::run_script_in_sandbox_with_env` -- the same infra the standalone
///   Script Runner console already uses, so stdout/stderr capture and plot-image diffing (a real,
///   working feature already) come for free instead of being reimplemented. The tradeoff, stated
///   plainly rather than hidden: no cross-cell variable persistence -- each cell's code must
///   (re-)load the data it needs (see the starter snippets `notebook_cell_starter` returns),
///   the same fresh-process-per-run model the Script Runner already has.
/// - **Data access**: the cell's own process gets the table's SQLite file path via the
///   `APICH_TABLE_DB` environment variable -- not a hardcoded path baked into the starter
///   snippet, so a cell keeps working if the table file is ever renamed/moved. Genuinely full
///   read *and* write access to that file (this app has no per-connection read-only enforcement
///   layer, and building one -- SQLite ACLs, a proxy, whatever -- is real, separate infrastructure
///   work of its own); a cell that writes back is expected to create/write a *new* derived table
///   rather than overwrite the source data, the same convention a real data-science notebook
///   already follows, not something this app can safely force at the SQL layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookCell {
    pub id: i64,
    pub position: i64,
    pub language: String, // "python" | "r"
    pub code: String,
    pub output: String,
    pub output_images: Vec<NotebookCellImage>,
}

/// A notebook cell's last captured plot -- same `{name, data_uri}` shape
/// `run_script_in_sandbox_with_env`'s `ScriptRunResult.output_images` already returns (a base64
/// data URI, not a bare path), stored as-is so a cell's last output survives a page reload without
/// a second round trip to re-read the image file. The real image file itself still lives in the
/// project's version-controlled workspace, generated fresh by the next Run; this is a persisted
/// *copy* of one past run's result, not a reference to it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookCellImage {
    pub name: String,
    pub data_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseFileInfo {
    pub relative_path: String,
    pub size_bytes: u64,
    pub modified_rfc3339: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub cid: i64,
    pub name: String,
    pub data_type: String,
    pub not_null: bool,
    pub default_value: Option<String>,
    pub is_primary_key: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSchema {
    pub name: String,
    pub is_view: bool,
    pub columns: Vec<ColumnInfo>,
    pub row_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSchema {
    pub file_path: String,
    pub tables: Vec<TableSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableDataPage {
    pub table_name: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    /// SQLite `rowid` for each row, parallel to `rows`. Used to identify a row for cell edits
    /// and deletes -- unlike assuming a column literally named "id" exists (it usually
    /// doesn't; a table's real primary key is just as often "sample_id", "uuid", etc.),
    /// `rowid` reliably identifies any ordinary (non-`WITHOUT ROWID`) SQLite row regardless of
    /// what its declared columns are named.
    pub row_ids: Vec<i64>,
    pub total_rows: u64,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
    /// Sparse: only cells with a non-default style are present. Keyed by `"{rowid}:{column}"`.
    pub styles: HashMap<String, CellStyle>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlExecutionResult {
    pub is_query: bool,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub rows_affected: usize,
    pub execution_time_ms: f64,
    pub message: String,
}

pub struct SqliteTableService;

impl SqliteTableService {
    /// Discover all SQLite database files (.db, .sqlite, .sqlite3) within a project directory
    pub fn discover_databases<P: AsRef<Path>>(project_dir: P) -> WebResult<Vec<DatabaseFileInfo>> {
        let mut dbs = Vec::new();
        let project_dir = project_dir.as_ref();

        if !project_dir.exists() {
            return Ok(dbs);
        }

        fn walk_dir(
            dir: &Path,
            root: &Path,
            dbs: &mut Vec<DatabaseFileInfo>,
        ) {
            let entries = match std::fs::read_dir(dir) {
                | Ok(e) => e,
                | Err(_) => return,
            };

            for entry in entries.flatten() {
                let path = entry.path();
                let file_name = entry.file_name().to_string_lossy().to_string();

                if path.is_dir() {
                    // Skip hidden dirs, VCS metadata dirs
                    if file_name.starts_with('.')
                        || file_name == "target"
                        || file_name == "node_modules"
                    {
                        continue;
                    }
                    walk_dir(&path, root, dbs);
                } else if path.is_file() {
                    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                    // Skip the auto-materialized `<name>.csv.table` sibling `resolve_db_path`
                    // creates behind a `.csv` file -- an implementation detail, not a file the
                    // user should see as a separate entry alongside the CSV it was made from.
                    let is_csv_backing_table = file_name.to_lowercase().ends_with(".csv.table");
                    if !is_csv_backing_table
                        && (ext.eq_ignore_ascii_case("db")
                            || ext.eq_ignore_ascii_case("sqlite")
                            || ext.eq_ignore_ascii_case("sqlite3")
                            || ext.eq_ignore_ascii_case("table")
                            || ext.eq_ignore_ascii_case("csv"))
                    {
                        let meta = entry.metadata().ok();
                        let size_bytes = meta.as_ref().map(|m| m.len()).unwrap_or(0);
                        let modified_rfc3339 = meta
                            .and_then(|m| m.modified().ok())
                            .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339());

                        if let Ok(rel) = path.strip_prefix(root) {
                            dbs.push(DatabaseFileInfo {
                                relative_path: rel.to_string_lossy().replace('\\', "/"),
                                size_bytes,
                                modified_rfc3339,
                            });
                        }
                    }
                }
            }
        }

        walk_dir(project_dir, project_dir, &mut dbs);
        dbs.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
        Ok(dbs)
    }

    /// Resolve safe path to a database file within project directory.
    ///
    /// CSV files are transparently backed by a real SQLite sibling (`data.csv` -> a
    /// `data.csv.table` next to it) so the exact same spreadsheet grid, cell editing, and SQL
    /// console work identically for both -- plan.md's own framing is "table first, SQLite
    /// database underneath it", and a CSV is just another table source, not a second UI. The
    /// sibling is materialized once, the first time the CSV is opened; every later call reuses
    /// it rather than re-importing (`import_csv` only `CREATE TABLE IF NOT EXISTS`s and then
    /// `INSERT`s -- calling it again on an already-imported file would duplicate every row).
    pub fn resolve_db_path<P: AsRef<Path>>(
        project_dir: P,
        rel_path: &str,
    ) -> WebResult<PathBuf> {
        let clean = rel_path.trim().trim_start_matches('/');
        if clean.contains("..") || clean.is_empty() {
            return Err(WebError::BadRequest("Invalid database path".to_string()));
        }

        if clean.to_lowercase().ends_with(".csv") {
            let csv_full_path = project_dir.as_ref().join(clean);
            if !csv_full_path.exists() {
                return Err(WebError::NotFound(format!(
                    "CSV file not found: {}",
                    rel_path
                )));
            }
            let table_rel = format!("{}.table", clean);
            let table_full_path = project_dir.as_ref().join(&table_rel);
            if !table_full_path.exists() {
                // Deliberately not `create_empty_database` here: it seeds a `_apich_metadata`
                // housekeeping table, which -- sorting alphabetically before any real table name
                // starting with a letter -- would win the table page's "default to the first
                // table" auto-selection, landing the user on an empty metadata table instead of
                // their actual CSV data. `import_csv` opens (and so creates, if missing) the
                // SQLite file itself, so plain data is the only table this file ever gets.
                let csv_content = std::fs::read_to_string(&csv_full_path)
                    .map_err(|e| WebError::Internal(format!("Failed to read CSV file: {}", e)))?;
                let table_name = Path::new(clean)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("data")
                    .replace(|c: char| !c.is_alphanumeric() && c != '_', "_");
                Self::import_csv(&table_full_path, &table_name, &csv_content)?;
            }
            return Ok(table_full_path);
        }

        let full_path = project_dir.as_ref().join(clean);
        if !full_path.exists() {
            return Err(WebError::NotFound(format!(
                "Database file not found: {}",
                rel_path
            )));
        }
        Ok(full_path)
    }

    /// Get schema of tables and columns for a SQLite database file
    pub fn get_database_schema<P: AsRef<Path>>(db_path: P) -> WebResult<DatabaseSchema> {
        let path_str = db_path.as_ref().to_string_lossy().to_string();
        let conn = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| WebError::Internal(format!("Failed to open SQLite database: {}", e)))?;

        // `_apich_%` tables (metadata, cell styles) are this app's own bookkeeping, not part of
        // the user's actual data -- excluded from the schema the same way sqlite's own internal
        // `sqlite_%` tables are, so they never show up as a selectable table in the UI.
        let mut stmt = conn
            .prepare("SELECT name, type FROM sqlite_master WHERE type IN ('table', 'view') AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '\\_apich\\_%' ESCAPE '\\' ORDER BY name ASC")
            .map_err(|e| WebError::Internal(format!("Failed to query schema: {}", e)))?;

        let table_iter = stmt
            .query_map([], |row| {
                let name: String = row.get(0)?;
                let tbl_type: String = row.get(1)?;
                Ok((name, tbl_type == "view"))
            })
            .map_err(|e| WebError::Internal(format!("Failed to fetch tables: {}", e)))?;

        let mut tables = Vec::new();

        for item in table_iter {
            let (table_name, is_view) = match item {
                | Ok(val) => val,
                | Err(_) => continue,
            };

            // Query columns
            let pragma_sql = format!("PRAGMA table_info(\"{}\")", table_name.replace('"', "\"\""));
            let mut pragma_stmt = match conn.prepare(&pragma_sql) {
                | Ok(s) => s,
                | Err(_) => continue,
            };

            let cols = pragma_stmt
                .query_map([], |r| {
                    Ok(ColumnInfo {
                        cid: r.get(0)?,
                        name: r.get(1)?,
                        data_type: r.get(2)?,
                        not_null: r.get::<_, i64>(3)? != 0,
                        default_value: r.get(4)?,
                        is_primary_key: r.get::<_, i64>(5)? != 0,
                    })
                })
                .map_err(|e| WebError::Internal(format!("Failed to get table columns: {}", e)))?;

            let mut columns = Vec::new();
            for c in cols.flatten() {
                columns.push(c);
            }

            // Query row count
            let count_sql = format!(
                "SELECT COUNT(*) FROM \"{}\"",
                table_name.replace('"', "\"\"")
            );
            let row_count: u64 = conn.query_row(&count_sql, [], |r| r.get(0)).unwrap_or(0);

            tables.push(TableSchema {
                name: table_name,
                is_view,
                columns,
                row_count,
            });
        }

        Ok(DatabaseSchema {
            file_path: path_str,
            tables,
        })
    }

    /// Retrieve paginated table data
    pub fn get_table_data<P: AsRef<Path>>(
        db_path: P,
        table_name: &str,
        page: usize,
        page_size: usize,
        sort_by: Option<&str>,
        sort_order: Option<&str>,
        search: Option<&str>,
    ) -> WebResult<TableDataPage> {
        let conn = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| WebError::Internal(format!("Failed to open SQLite database: {}", e)))?;

        let clean_table = table_name.replace('"', "\"\"");

        // Validate table exists and retrieve column list
        let pragma_sql = format!("PRAGMA table_info(\"{}\")", clean_table);
        let mut pragma_stmt = conn
            .prepare(&pragma_sql)
            .map_err(|e| WebError::BadRequest(format!("Table {} not found: {}", table_name, e)))?;

        let column_names: Vec<String> = pragma_stmt
            .query_map([], |r| r.get::<_, String>(1))
            .map_err(|e| WebError::Internal(e.to_string()))?
            .filter_map(Result::ok)
            .collect();

        if column_names.is_empty() {
            return Err(WebError::NotFound(format!(
                "Table '{}' not found or has no columns",
                table_name
            )));
        }

        // Count total rows matching search filter
        let mut where_clause = String::new();
        let search_pattern = search.map(|s| format!("%{}%", s.trim()));

        if let Some(ref pat) = search_pattern {
            if !pat.is_empty() && pat != "%%" {
                let clauses: Vec<String> = column_names
                    .iter()
                    .map(|c| format!("CAST(\"{}\" AS TEXT) LIKE ?1", c.replace('"', "\"\"")))
                    .collect();
                where_clause = format!("WHERE {}", clauses.join(" OR "));
            }
        }

        let count_sql = format!("SELECT COUNT(*) FROM \"{}\" {}", clean_table, where_clause);
        let total_rows: u64 = if where_clause.is_empty() {
            conn.query_row(&count_sql, [], |r| r.get(0)).unwrap_or(0)
        } else {
            conn.query_row(&count_sql, [&search_pattern], |r| r.get(0))
                .unwrap_or(0)
        };

        // Validate sort column
        let sort_sql = if let Some(col) = sort_by {
            if column_names.iter().any(|c| c == col) {
                let order = match sort_order.map(|s| s.to_uppercase()).as_deref() {
                    | Some("DESC") => "DESC",
                    | _ => "ASC",
                };
                format!("ORDER BY \"{}\" {}", col.replace('"', "\"\""), order)
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let current_page = if page == 0 { 1 } else { page };
        let page_limit = if page_size == 0 || page_size > 500 {
            50
        } else {
            page_size
        };
        let offset = (current_page - 1) * page_limit;

        let query_sql = format!(
            "SELECT rowid, * FROM \"{}\" {} {} LIMIT {} OFFSET {}",
            clean_table, where_clause, sort_sql, page_limit, offset
        );

        let mut stmt = conn
            .prepare(&query_sql)
            .map_err(|e| WebError::Internal(format!("Query preparation failed: {}", e)))?;

        let mut rows = Vec::new();
        let mut row_ids = Vec::new();
        let mut rows_iter = if where_clause.is_empty() {
            stmt.query([])
        } else {
            stmt.query([&search_pattern])
        }
        .map_err(|e| WebError::Internal(format!("Query execution failed: {}", e)))?;

        while let Some(r) = rows_iter
            .next()
            .map_err(|e| WebError::Internal(e.to_string()))?
        {
            let rowid: i64 = r.get(0).map_err(|e| WebError::Internal(e.to_string()))?;
            row_ids.push(rowid);
            let mut row_vals = Vec::with_capacity(column_names.len());
            for i in 0..column_names.len() {
                // Declared columns start at index 1 (index 0 is the leading `rowid`).
                let val = r
                    .get_ref(i + 1)
                    .map_err(|e| WebError::Internal(e.to_string()))?;
                row_vals.push(sqlite_val_to_json(val));
            }
            rows.push(row_vals);
        }

        let total_pages = if total_rows == 0 {
            1
        } else {
            (total_rows as usize).div_ceil(page_limit)
        };

        // Best-effort: a database with no `_apich_cell_styles` table yet (i.e. nothing has ever
        // been styled) just yields an empty map rather than an error -- styling is optional
        // metadata, its absence isn't a fault of the read-only page load.
        let styles = Self::get_cell_styles(&db_path, table_name).unwrap_or_default();

        Ok(TableDataPage {
            table_name: table_name.to_string(),
            columns: column_names,
            rows,
            row_ids,
            total_rows,
            page: current_page,
            page_size: page_limit,
            total_pages,
            styles,
        })
    }

    fn ensure_style_table(conn: &Connection) -> WebResult<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS _apich_cell_styles (
                table_name TEXT NOT NULL,
                row_id INTEGER NOT NULL,
                col_name TEXT NOT NULL,
                bold INTEGER NOT NULL DEFAULT 0,
                italic INTEGER NOT NULL DEFAULT 0,
                color TEXT,
                bg_color TEXT,
                PRIMARY KEY (table_name, row_id, col_name)
            )",
            [],
        )
        .map_err(|e| WebError::Internal(format!("Failed to ensure cell-style table: {}", e)))?;
        Ok(())
    }

    /// All non-default cell styles recorded for one table, keyed by `"{rowid}:{column}"`.
    pub fn get_cell_styles<P: AsRef<Path>>(
        db_path: P,
        table_name: &str,
    ) -> WebResult<HashMap<String, CellStyle>> {
        let conn = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| WebError::Internal(format!("Failed to open SQLite database: {}", e)))?;

        // A read-only connection can still SELECT from a table that doesn't exist yet -- that's
        // just an empty result via `query_map`'s error path here, handled as "no styles" rather
        // than propagated, since callers treat this as optional metadata (see `get_table_data`).
        let mut stmt = match conn.prepare(
            "SELECT row_id, col_name, bold, italic, color, bg_color FROM _apich_cell_styles WHERE table_name = ?1",
        ) {
            Ok(s) => s,
            Err(_) => return Ok(HashMap::new()),
        };

        let rows = stmt
            .query_map([table_name], |r| {
                let row_id: i64 = r.get(0)?;
                let col_name: String = r.get(1)?;
                let style = CellStyle {
                    bold: r.get::<_, i64>(2)? != 0,
                    italic: r.get::<_, i64>(3)? != 0,
                    color: r.get(4)?,
                    bg_color: r.get(5)?,
                };
                Ok((format!("{}:{}", row_id, col_name), style))
            })
            .map_err(|e| WebError::Internal(e.to_string()))?;

        let mut map = HashMap::new();
        for row in rows {
            let (key, style) = row.map_err(|e| WebError::Internal(e.to_string()))?;
            map.insert(key, style);
        }
        Ok(map)
    }

    /// Set (or, if the style is back to the default, clear) one cell's formatting.
    pub fn set_cell_style<P: AsRef<Path>>(
        db_path: P,
        table_name: &str,
        row_id: i64,
        col_name: &str,
        style: &CellStyle,
    ) -> WebResult<()> {
        let conn = Connection::open(&db_path)
            .map_err(|e| WebError::Internal(format!("Failed to open database: {}", e)))?;
        Self::ensure_style_table(&conn)?;

        if style.is_default() {
            conn.execute(
                "DELETE FROM _apich_cell_styles WHERE table_name = ?1 AND row_id = ?2 AND col_name = ?3",
                rusqlite::params![table_name, row_id, col_name],
            )
            .map_err(|e| WebError::Internal(format!("Failed to clear cell style: {}", e)))?;
            return Ok(());
        }

        conn.execute(
            "INSERT INTO _apich_cell_styles (table_name, row_id, col_name, bold, italic, color, bg_color)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT (table_name, row_id, col_name) DO UPDATE SET
                bold = excluded.bold, italic = excluded.italic, color = excluded.color, bg_color = excluded.bg_color",
            rusqlite::params![table_name, row_id, col_name, style.bold as i64, style.italic as i64, style.color, style.bg_color],
        )
        .map_err(|e| WebError::Internal(format!("Failed to save cell style: {}", e)))?;
        Ok(())
    }

    fn ensure_column_view_table(conn: &Connection) -> WebResult<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS _apich_column_view (
                table_name TEXT PRIMARY KEY,
                config_json TEXT NOT NULL
            )",
            [],
        )
        .map_err(|e| WebError::Internal(format!("Failed to ensure column-view table: {}", e)))?;
        Ok(())
    }

    /// A table's saved column display config (empty default -- original order, nothing hidden --
    /// if none has ever been saved, or if the metadata table doesn't exist yet).
    pub fn get_column_view<P: AsRef<Path>>(
        db_path: P,
        table_name: &str,
    ) -> WebResult<ColumnViewConfig> {
        let conn = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| WebError::Internal(format!("Failed to open SQLite database: {}", e)))?;
        let result: rusqlite::Result<String> = conn.query_row(
            "SELECT config_json FROM _apich_column_view WHERE table_name = ?1",
            [table_name],
            |r| r.get(0),
        );
        match result {
            | Ok(json) => Ok(serde_json::from_str(&json).unwrap_or_default()),
            | Err(_) => Ok(ColumnViewConfig::default()),
        }
    }

    /// Save a table's column display config.
    pub fn set_column_view<P: AsRef<Path>>(
        db_path: P,
        table_name: &str,
        config: &ColumnViewConfig,
    ) -> WebResult<()> {
        let conn = Connection::open(&db_path)
            .map_err(|e| WebError::Internal(format!("Failed to open database: {}", e)))?;
        Self::ensure_column_view_table(&conn)?;
        let json = serde_json::to_string(config)
            .map_err(|e| WebError::Internal(format!("Failed to serialize column view: {}", e)))?;
        conn.execute(
            "INSERT INTO _apich_column_view (table_name, config_json) VALUES (?1, ?2)
             ON CONFLICT (table_name) DO UPDATE SET config_json = excluded.config_json",
            rusqlite::params![table_name, json],
        )
        .map_err(|e| WebError::Internal(format!("Failed to save column view: {}", e)))?;
        Ok(())
    }

    fn ensure_query_history_table(conn: &Connection) -> WebResult<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS _apich_query_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                sql_text TEXT NOT NULL,
                ran_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
            [],
        )
        .map_err(|e| WebError::Internal(format!("Failed to ensure query-history table: {}", e)))?;
        Ok(())
    }

    /// Record a successfully-run SQL console query, keeping only the most recent 10 (deduping a
    /// query that's identical to the immediately-preceding one, so re-running the same query
    /// doesn't spam the history list with repeats).
    pub fn record_query_history<P: AsRef<Path>>(
        db_path: P,
        sql: &str,
    ) -> WebResult<()> {
        let conn = Connection::open(&db_path)
            .map_err(|e| WebError::Internal(format!("Failed to open database: {}", e)))?;
        Self::ensure_query_history_table(&conn)?;

        let last: Option<String> = conn
            .query_row(
                "SELECT sql_text FROM _apich_query_history ORDER BY id DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .ok();
        if last.as_deref() == Some(sql) {
            return Ok(());
        }

        conn.execute(
            "INSERT INTO _apich_query_history (sql_text) VALUES (?1)",
            [sql],
        )
        .map_err(|e| WebError::Internal(format!("Failed to record query history: {}", e)))?;
        conn.execute(
            "DELETE FROM _apich_query_history WHERE id NOT IN (SELECT id FROM _apich_query_history ORDER BY id DESC LIMIT 10)",
            [],
        )
        .map_err(|e| WebError::Internal(format!("Failed to trim query history: {}", e)))?;
        Ok(())
    }

    /// The last 10 distinct SQL console queries run against this file, most recent first.
    pub fn list_query_history<P: AsRef<Path>>(db_path: P) -> WebResult<Vec<String>> {
        let conn = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| WebError::Internal(format!("Failed to open SQLite database: {}", e)))?;
        let mut stmt = match conn
            .prepare("SELECT sql_text FROM _apich_query_history ORDER BY id DESC LIMIT 10")
        {
            | Ok(s) => s,
            | Err(_) => return Ok(Vec::new()),
        };
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| WebError::Internal(e.to_string()))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| WebError::Internal(e.to_string()))?);
        }
        Ok(out)
    }

    fn ensure_notebook_table(conn: &Connection) -> WebResult<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS _apich_notebook_cells (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                table_name TEXT NOT NULL,
                position INTEGER NOT NULL,
                language TEXT NOT NULL,
                code TEXT NOT NULL DEFAULT '',
                output TEXT NOT NULL DEFAULT '',
                output_images TEXT NOT NULL DEFAULT '[]'
            )",
            [],
        )
        .map_err(|e| WebError::Internal(format!("Failed to ensure notebook table: {}", e)))?;
        Ok(())
    }

    /// A table's notebook cells, in display order. Best-effort like `get_cell_styles`: a database
    /// with no `_apich_notebook_cells` table yet (nothing has ever run a notebook cell against
    /// it) is "no cells", not an error.
    pub fn list_notebook_cells<P: AsRef<Path>>(
        db_path: P,
        table_name: &str,
    ) -> WebResult<Vec<NotebookCell>> {
        let conn = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| WebError::Internal(format!("Failed to open SQLite database: {}", e)))?;
        let mut stmt = match conn.prepare(
            "SELECT id, position, language, code, output, output_images FROM _apich_notebook_cells WHERE table_name = ?1 ORDER BY position ASC",
        ) {
            Ok(s) => s,
            Err(_) => return Ok(Vec::new()),
        };
        let rows = stmt
            .query_map([table_name], |r| {
                let images_json: String = r.get(5)?;
                Ok(NotebookCell {
                    id: r.get(0)?,
                    position: r.get(1)?,
                    language: r.get(2)?,
                    code: r.get(3)?,
                    output: r.get(4)?,
                    output_images: serde_json::from_str(&images_json).unwrap_or_default(),
                })
            })
            .map_err(|e| WebError::Internal(e.to_string()))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| WebError::Internal(e.to_string()))?);
        }
        Ok(out)
    }

    /// Creates a new, empty cell at the end of a table's notebook and returns its id.
    pub fn create_notebook_cell<P: AsRef<Path>>(
        db_path: P,
        table_name: &str,
        language: &str,
        starter_code: &str,
    ) -> WebResult<i64> {
        let conn = Connection::open(&db_path)
            .map_err(|e| WebError::Internal(format!("Failed to open database: {}", e)))?;
        Self::ensure_notebook_table(&conn)?;
        let next_position: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(position), -1) + 1 FROM _apich_notebook_cells WHERE table_name = ?1",
                [table_name],
                |r| r.get(0),
            )
            .unwrap_or(0);
        conn.execute(
            "INSERT INTO _apich_notebook_cells (table_name, position, language, code) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![table_name, next_position, language, starter_code],
        )
        .map_err(|e| WebError::Internal(format!("Failed to create notebook cell: {}", e)))?;
        Ok(conn.last_insert_rowid())
    }

    /// Persists a cell's code and, if it was just run, the fresh output -- called both by a plain
    /// "save code" edit and by `run_notebook_cell` right after execution finishes.
    pub fn update_notebook_cell<P: AsRef<Path>>(
        db_path: P,
        cell_id: i64,
        code: &str,
        output: Option<&str>,
        output_images: Option<&[NotebookCellImage]>,
    ) -> WebResult<()> {
        let conn = Connection::open(&db_path)
            .map_err(|e| WebError::Internal(format!("Failed to open database: {}", e)))?;
        Self::ensure_notebook_table(&conn)?;
        match (output, output_images) {
            (Some(out), Some(imgs)) => {
                let images_json = serde_json::to_string(imgs).unwrap_or_else(|_| "[]".to_string());
                conn.execute(
                    "UPDATE _apich_notebook_cells SET code = ?1, output = ?2, output_images = ?3 WHERE id = ?4",
                    rusqlite::params![code, out, images_json, cell_id],
                )
            }
            _ => conn.execute("UPDATE _apich_notebook_cells SET code = ?1 WHERE id = ?2", rusqlite::params![code, cell_id]),
        }
        .map_err(|e| WebError::Internal(format!("Failed to save notebook cell: {}", e)))?;
        Ok(())
    }

    pub fn delete_notebook_cell<P: AsRef<Path>>(
        db_path: P,
        cell_id: i64,
    ) -> WebResult<()> {
        let conn = Connection::open(&db_path)
            .map_err(|e| WebError::Internal(format!("Failed to open database: {}", e)))?;
        conn.execute("DELETE FROM _apich_notebook_cells WHERE id = ?1", [cell_id])
            .map_err(|e| WebError::Internal(format!("Failed to delete notebook cell: {}", e)))?;
        Ok(())
    }

    /// Move a cell one slot earlier ("up") or later ("down") in its table's notebook by swapping
    /// `position` with its immediate neighbor -- a no-op (not an error) if the cell is already at
    /// that end of the list.
    pub fn move_notebook_cell<P: AsRef<Path>>(
        db_path: P,
        cell_id: i64,
        direction: &str,
    ) -> WebResult<()> {
        let conn = Connection::open(&db_path)
            .map_err(|e| WebError::Internal(format!("Failed to open database: {}", e)))?;
        Self::ensure_notebook_table(&conn)?;

        let (table_name, position): (String, i64) = conn
            .query_row(
                "SELECT table_name, position FROM _apich_notebook_cells WHERE id = ?1",
                [cell_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|e| WebError::Internal(format!("Cell not found: {}", e)))?;

        let neighbor: Option<(i64, i64)> = if direction == "up" {
            conn.query_row(
                "SELECT id, position FROM _apich_notebook_cells WHERE table_name = ?1 AND position < ?2 ORDER BY position DESC LIMIT 1",
                rusqlite::params![table_name, position],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .ok()
        } else {
            conn.query_row(
                "SELECT id, position FROM _apich_notebook_cells WHERE table_name = ?1 AND position > ?2 ORDER BY position ASC LIMIT 1",
                rusqlite::params![table_name, position],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .ok()
        };

        if let Some((neighbor_id, neighbor_position)) = neighbor {
            conn.execute(
                "UPDATE _apich_notebook_cells SET position = ?1 WHERE id = ?2",
                rusqlite::params![neighbor_position, cell_id],
            )
            .map_err(|e| WebError::Internal(format!("Failed to reorder cell: {}", e)))?;
            conn.execute(
                "UPDATE _apich_notebook_cells SET position = ?1 WHERE id = ?2",
                rusqlite::params![position, neighbor_id],
            )
            .map_err(|e| WebError::Internal(format!("Failed to reorder cell: {}", e)))?;
        }
        Ok(())
    }

    /// A short starter snippet shown when a new cell is created -- without this, a user staring
    /// at a blank code box has no obvious way to discover `APICH_TABLE_DB` (the one piece of
    /// this feature that isn't self-evident: the environment variable a cell's process needs to
    /// read to find *this* table's real SQLite file, see `NotebookCell`'s own doc comment).
    pub fn notebook_cell_starter(
        language: &str,
        table_name: &str,
    ) -> String {
        match language {
            "r" => format!(
                "library(DBI)\nlibrary(RSQLite)\nlibrary(ggplot2)\n\ncon <- dbConnect(RSQLite::SQLite(), Sys.getenv(\"APICH_TABLE_DB\"))\ndf <- dbReadTable(con, \"{table}\")\nhead(df)\n\n# Save a derived result back as a NEW table (never overwrite the source):\n# dbWriteTable(con, \"{table}_summary\", summary_df, overwrite = TRUE)\n\n# A saved plot appears as real output below, the same way a Jupyter cell's does:\n# ggplot(df, aes(x = some_column)) + geom_histogram()\n# ggsave(\"plot.png\", width = 6, height = 4)\n\ndbDisconnect(con)\n",
                table = table_name
            ),
            _ => format!(
                "import os\nimport sqlite3\nimport pandas as pd\nimport matplotlib.pyplot as plt\n\nconn = sqlite3.connect(os.environ[\"APICH_TABLE_DB\"])\ndf = pd.read_sql_query('SELECT * FROM \"{table}\"', conn)\nprint(df.head())\n\n# Save a derived result back as a NEW table (never overwrite the source):\n# df.describe().to_sql(\"{table}_summary\", conn, if_exists=\"replace\")\n\n# A saved plot appears as real output below, the same way a Jupyter cell's does:\n# df.plot()\n# plt.savefig(\"plot.png\")\n\nconn.close()\n",
                table = table_name
            ),
        }
    }

    /// Update a single cell in a SQLite table
    pub fn update_cell<P: AsRef<Path>>(
        db_path: P,
        table_name: &str,
        row_id_col: &str,
        row_id_val: &str,
        target_col: &str,
        new_val: &str,
    ) -> WebResult<()> {
        let clean_table = table_name.replace('"', "\"\"");
        let clean_id_col = row_id_col.replace('"', "\"\"");
        let clean_target_col = target_col.replace('"', "\"\"");

        let conn = Connection::open(&db_path)
            .map_err(|e| WebError::Internal(format!("Failed to open database: {}", e)))?;

        let sql = format!(
            "UPDATE \"{}\" SET \"{}\" = ?1 WHERE \"{}\" = ?2",
            clean_table, clean_target_col, clean_id_col
        );

        conn.execute(&sql, rusqlite::params![new_val, row_id_val])
            .map_err(|e| WebError::Internal(format!("Failed to update cell: {}", e)))?;

        Ok(())
    }

    /// Insert a new default row into a table
    pub fn insert_row<P: AsRef<Path>>(
        db_path: P,
        table_name: &str,
    ) -> WebResult<i64> {
        let clean_table = table_name.replace('"', "\"\"");
        let conn = Connection::open(&db_path)
            .map_err(|e| WebError::Internal(format!("Failed to open database: {}", e)))?;

        let sql = format!("INSERT INTO \"{}\" DEFAULT VALUES", clean_table);
        conn.execute(&sql, [])
            .map_err(|e| WebError::Internal(format!("Failed to insert row: {}", e)))?;

        Ok(conn.last_insert_rowid())
    }

    /// Delete a row from a table by primary key / ID value
    pub fn delete_row<P: AsRef<Path>>(
        db_path: P,
        table_name: &str,
        row_id_col: &str,
        row_id_val: &str,
    ) -> WebResult<()> {
        let clean_table = table_name.replace('"', "\"\"");
        let clean_id_col = row_id_col.replace('"', "\"\"");

        let conn = Connection::open(&db_path)
            .map_err(|e| WebError::Internal(format!("Failed to open database: {}", e)))?;

        let sql = format!(
            "DELETE FROM \"{}\" WHERE \"{}\" = ?1",
            clean_table, clean_id_col
        );
        conn.execute(&sql, [row_id_val])
            .map_err(|e| WebError::Internal(format!("Failed to delete row: {}", e)))?;

        Ok(())
    }

    /// Export table to CSV formatted string
    /// Fetch every row of a table as `(column_names, rows)`, each cell already converted to a
    /// `serde_json::Value` -- the shared backbone for every export format below (CSV was the
    /// only one that predates this and gets its own hand-rolled quoting rules, since CSV's
    /// escaping is different enough from "just serialize the value" to not be worth forcing
    /// through the same path).
    fn fetch_all_rows<P: AsRef<Path>>(
        db_path: P,
        table_name: &str,
    ) -> WebResult<(Vec<String>, Vec<Vec<serde_json::Value>>)> {
        let clean_table = table_name.replace('"', "\"\"");
        let conn = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| WebError::Internal(format!("Failed to open database: {}", e)))?;

        let sql = format!("SELECT * FROM \"{}\"", clean_table);
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| WebError::Internal(format!("Failed to prepare export query: {}", e)))?;
        let col_names: Vec<String> = stmt
            .column_names()
            .into_iter()
            .map(|s| s.to_string())
            .collect();

        let mut rows_out = Vec::new();
        let mut rows = stmt
            .query([])
            .map_err(|e| WebError::Internal(format!("Failed to query table: {}", e)))?;
        while let Some(r) = rows.next().map_err(|e| WebError::Internal(e.to_string()))? {
            let mut line = Vec::with_capacity(col_names.len());
            for i in 0..col_names.len() {
                let val_ref = r
                    .get_ref(i)
                    .map_err(|e| WebError::Internal(e.to_string()))?;
                let v = match val_ref {
                    | ValueRef::Null => serde_json::Value::Null,
                    | ValueRef::Integer(v) => serde_json::Value::from(v),
                    | ValueRef::Real(v) => serde_json::Value::from(v),
                    | ValueRef::Text(v) => {
                        serde_json::Value::String(String::from_utf8_lossy(v).to_string())
                    },
                    | ValueRef::Blob(_) => serde_json::Value::String("<blob>".to_string()),
                };
                line.push(v);
            }
            rows_out.push(line);
        }
        Ok((col_names, rows_out))
    }

    /// Export a table as a JSON array of `{"column": value, ...}` objects.
    pub fn export_json<P: AsRef<Path>>(
        db_path: P,
        table_name: &str,
    ) -> WebResult<String> {
        let (columns, rows) = Self::fetch_all_rows(db_path, table_name)?;
        let objects: Vec<serde_json::Value> = rows
            .into_iter()
            .map(|row| {
                let map: serde_json::Map<String, serde_json::Value> =
                    columns.iter().cloned().zip(row).collect();
                serde_json::Value::Object(map)
            })
            .collect();
        serde_json::to_string_pretty(&objects)
            .map_err(|e| WebError::Internal(format!("Failed to serialize JSON export: {}", e)))
    }

    /// Export a table as tab-separated values (Excel/Numbers paste-friendly).
    pub fn export_tsv<P: AsRef<Path>>(
        db_path: P,
        table_name: &str,
    ) -> WebResult<String> {
        let (columns, rows) = Self::fetch_all_rows(db_path, table_name)?;
        let cell_to_tsv = |v: &serde_json::Value| -> String {
            match v {
                | serde_json::Value::Null => String::new(),
                | serde_json::Value::String(s) => s.replace('\t', "    ").replace('\n', " "),
                | other => other.to_string(),
            }
        };
        let mut out = String::new();
        out.push_str(&columns.join("\t"));
        out.push('\n');
        for row in &rows {
            let line: Vec<String> = row.iter().map(cell_to_tsv).collect();
            out.push_str(&line.join("\t"));
            out.push('\n');
        }
        Ok(out)
    }

    /// Export a table as a GitHub-flavored Markdown table -- handy for pasting straight into a
    /// note or a README rather than round-tripping through a spreadsheet app first.
    pub fn export_markdown<P: AsRef<Path>>(
        db_path: P,
        table_name: &str,
    ) -> WebResult<String> {
        let (columns, rows) = Self::fetch_all_rows(db_path, table_name)?;
        let cell_to_md = |v: &serde_json::Value| -> String {
            let raw = match v {
                | serde_json::Value::Null => String::new(),
                | serde_json::Value::String(s) => s.clone(),
                | other => other.to_string(),
            };
            raw.replace('|', "\\|").replace('\n', "<br>")
        };
        let mut out = String::new();
        out.push_str("| ");
        out.push_str(&columns.join(" | "));
        out.push_str(" |\n|");
        out.push_str(&" --- |".repeat(columns.len()));
        out.push('\n');
        for row in &rows {
            out.push_str("| ");
            out.push_str(&row.iter().map(cell_to_md).collect::<Vec<_>>().join(" | "));
            out.push_str(" |\n");
        }
        Ok(out)
    }

    pub fn export_csv<P: AsRef<Path>>(
        db_path: P,
        table_name: &str,
    ) -> WebResult<String> {
        let clean_table = table_name.replace('"', "\"\"");
        let conn = Connection::open(&db_path)
            .map_err(|e| WebError::Internal(format!("Failed to open database: {}", e)))?;

        let sql = format!("SELECT * FROM \"{}\"", clean_table);
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| WebError::Internal(format!("Failed to prepare export query: {}", e)))?;

        let col_names: Vec<String> = stmt
            .column_names()
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        let mut csv = String::new();
        // Header
        csv.push_str(&col_names.join(","));
        csv.push('\n');

        let mut rows = stmt
            .query([])
            .map_err(|e| WebError::Internal(format!("Failed to query table: {}", e)))?;

        while let Some(r) = rows.next().map_err(|e| WebError::Internal(e.to_string()))? {
            let mut line = Vec::with_capacity(col_names.len());
            for i in 0..col_names.len() {
                let val_ref = r
                    .get_ref(i)
                    .map_err(|e| WebError::Internal(e.to_string()))?;
                let s = match val_ref {
                    | rusqlite::types::ValueRef::Null => "".to_string(),
                    | rusqlite::types::ValueRef::Integer(v) => v.to_string(),
                    | rusqlite::types::ValueRef::Real(v) => v.to_string(),
                    | rusqlite::types::ValueRef::Text(v) => {
                        let text = String::from_utf8_lossy(v);
                        if text.contains(',') || text.contains('"') || text.contains('\n') {
                            format!("\"{}\"", text.replace('"', "\"\""))
                        } else {
                            text.to_string()
                        }
                    },
                    | rusqlite::types::ValueRef::Blob(_) => "<blob>".to_string(),
                };
                line.push(s);
            }
            csv.push_str(&line.join(","));
            csv.push('\n');
        }

        Ok(csv)
    }

    /// Import CSV formatted data into a table
    pub fn import_csv<P: AsRef<Path>>(
        db_path: P,
        table_name: &str,
        csv_content: &str,
    ) -> WebResult<usize> {
        let clean_table = table_name.replace('"', "\"\"");
        let conn = Connection::open(&db_path)
            .map_err(|e| WebError::Internal(format!("Failed to open database: {}", e)))?;

        let mut lines = csv_content.lines();
        let header = match lines.next() {
            | Some(h) if !h.trim().is_empty() => h,
            | _ => {
                return Err(WebError::BadRequest(
                    "CSV content is empty or missing header".to_string(),
                ))
            },
        };

        let cols: Vec<String> = header
            .split(',')
            .map(|s| s.trim().trim_matches('"').to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if cols.is_empty() {
            return Err(WebError::BadRequest(
                "No valid columns in CSV header".to_string(),
            ));
        }

        // Create table if not exists with TEXT columns
        let col_defs = cols
            .iter()
            .map(|c| format!("\"{}\" TEXT", c.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(", ");
        let create_sql = format!(
            "CREATE TABLE IF NOT EXISTS \"{}\" ({})",
            clean_table, col_defs
        );
        conn.execute(&create_sql, [])
            .map_err(|e| WebError::Internal(format!("Failed to ensure table: {}", e)))?;

        // Prepare insert statement
        let placeholders = cols.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let quoted_cols = cols
            .iter()
            .map(|c| format!("\"{}\"", c.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(", ");
        let insert_sql = format!(
            "INSERT INTO \"{}\" ({}) VALUES ({})",
            clean_table, quoted_cols, placeholders
        );
        let mut insert_stmt = conn.prepare(&insert_sql).map_err(|e| {
            WebError::Internal(format!("Failed to prepare insert statement: {}", e))
        })?;

        let mut count = 0;
        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let fields: Vec<&str> = trimmed
                .split(',')
                .map(|s| s.trim().trim_matches('"'))
                .collect();
            let mut params_vec: Vec<&str> = Vec::new();
            for i in 0..cols.len() {
                params_vec.push(fields.get(i).copied().unwrap_or(""));
            }
            insert_stmt
                .execute(rusqlite::params_from_iter(params_vec))
                .map_err(|e| WebError::Internal(format!("Failed to insert row: {}", e)))?;
            count += 1;
        }

        Ok(count)
    }

    /// Execute arbitrary SQL queries or DDL/DML statements, capping how many result rows a
    /// `SELECT`-shaped query returns at `max_rows` (clamped to a sane [1, 5000] range so the
    /// console's own row-limit field can't be used to pull back an unbounded result set).
    pub fn execute_sql<P: AsRef<Path>>(
        db_path: P,
        sql: &str,
        max_rows: usize,
    ) -> WebResult<SqlExecutionResult> {
        let trimmed = sql.trim();
        if trimmed.is_empty() {
            return Err(WebError::BadRequest(
                "SQL query cannot be empty".to_string(),
            ));
        }

        let start = Instant::now();
        let first_word = trimmed
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_uppercase();

        let is_query = matches!(
            first_word.as_str(),
            "SELECT" | "PRAGMA" | "EXPLAIN" | "WITH" | "VALUES"
        );

        let conn = Connection::open(&db_path)
            .map_err(|e| WebError::Internal(format!("Failed to open database: {}", e)))?;

        if is_query {
            let mut stmt = conn
                .prepare(trimmed)
                .map_err(|e| WebError::BadRequest(format!("SQL compilation error: {}", e)))?;

            let col_count = stmt.column_count();
            let columns: Vec<String> = stmt
                .column_names()
                .into_iter()
                .map(|s| s.to_string())
                .collect();

            let mut rows = Vec::new();
            let mut rows_iter = stmt
                .query([])
                .map_err(|e| WebError::Internal(format!("SQL execution failed: {}", e)))?;

            let max_rows = max_rows.clamp(1, 5000);
            let mut truncated = false;
            while let Some(row) = rows_iter
                .next()
                .map_err(|e| WebError::Internal(e.to_string()))?
            {
                if rows.len() >= max_rows {
                    truncated = true;
                    break;
                }
                let mut row_vals = Vec::with_capacity(col_count);
                for i in 0..col_count {
                    let val = row
                        .get_ref(i)
                        .map_err(|e| WebError::Internal(e.to_string()))?;
                    row_vals.push(sqlite_val_to_json(val));
                }
                rows.push(row_vals);
            }

            let elapsed = start.elapsed().as_secs_f64() * 1000.0;
            let count = rows.len();
            let message = if truncated {
                format!("Query executed successfully ({} rows returned in {:.2} ms, truncated at the {}-row limit -- refine the query or raise the limit to see more)", count, elapsed, max_rows)
            } else {
                format!(
                    "Query executed successfully ({} rows returned in {:.2} ms)",
                    count, elapsed
                )
            };

            Ok(SqlExecutionResult {
                is_query: true,
                columns,
                rows,
                rows_affected: count,
                execution_time_ms: (elapsed * 100.0).round() / 100.0,
                message,
            })
        } else {
            let rows_affected = conn
                .execute_batch(trimmed)
                .map(|_| 1)
                .map_err(|e| WebError::BadRequest(format!("Execution failed: {}", e)))?;

            let elapsed = start.elapsed().as_secs_f64() * 1000.0;

            Ok(SqlExecutionResult {
                is_query: false,
                columns: Vec::new(),
                rows: Vec::new(),
                rows_affected,
                execution_time_ms: (elapsed * 100.0).round() / 100.0,
                message: format!("Batch executed successfully in {:.2} ms", elapsed),
            })
        }
    }

    /// Create a new database file or initialize standard scientific tables
    pub fn create_empty_database<P: AsRef<Path>>(full_path: P) -> WebResult<()> {
        let p = full_path.as_ref();
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                WebError::Internal(format!("Failed to create parent directory: {}", e))
            })?;
        }

        let conn = Connection::open(p)
            .map_err(|e| WebError::Internal(format!("Failed to create SQLite file: {}", e)))?;

        // Initialize with default meta table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS _apich_metadata (
                key TEXT PRIMARY KEY,
                value TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )
        .map_err(|e| WebError::Internal(format!("Failed to initialize table: {}", e)))?;

        conn.execute(
            "INSERT OR REPLACE INTO _apich_metadata (key, value) VALUES ('generator', 'APICH Scientific Database')",
            [],
        )
        .map_err(|e| WebError::Internal(format!("Failed to insert metadata: {}", e)))?;

        Ok(())
    }
}

fn sqlite_val_to_json(val: ValueRef<'_>) -> serde_json::Value {
    match val {
        | ValueRef::Null => serde_json::Value::Null,
        | ValueRef::Integer(i) => serde_json::Value::Number(serde_json::Number::from(i)),
        | ValueRef::Real(f) => {
            serde_json::Number::from_f64(f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null)
        },
        | ValueRef::Text(t) => {
            let s = String::from_utf8_lossy(t);
            serde_json::Value::String(s.into_owned())
        },
        | ValueRef::Blob(b) => {
            serde_json::Value::String(format!(
                "<BLOB {} bytes: 0x{}>",
                b.len(),
                hex::encode(&b[..b.len().min(16)])
            ))
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_sqlite_table_lifecycle_and_queries() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("experiment_data.sqlite");

        // 1. Create database
        SqliteTableService::create_empty_database(&db_path).unwrap();
        assert!(db_path.exists());

        // 2. Discover databases
        let dbs = SqliteTableService::discover_databases(dir.path()).unwrap();
        assert_eq!(dbs.len(), 1);
        assert_eq!(dbs[0].relative_path, "experiment_data.sqlite");

        // 3. Create table and insert data
        let create_sql = "
            CREATE TABLE experiments (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                sample_name TEXT NOT NULL,
                temperature REAL,
                status TEXT DEFAULT 'pending'
            );
            INSERT INTO experiments (sample_name, temperature, status) VALUES ('Sample-A', 298.15, 'completed');
            INSERT INTO experiments (sample_name, temperature, status) VALUES ('Sample-B', 77.0, 'completed');
            INSERT INTO experiments (sample_name, temperature, status) VALUES ('Sample-C', 4.2, 'in_progress');
        ";
        let res = SqliteTableService::execute_sql(&db_path, create_sql, 5000).unwrap();
        assert!(!res.is_query);

        // 4. Introspect schema
        let schema = SqliteTableService::get_database_schema(&db_path).unwrap();
        let exp_tbl = schema
            .tables
            .iter()
            .find(|t| t.name == "experiments")
            .unwrap();
        assert_eq!(exp_tbl.columns.len(), 4);
        assert_eq!(exp_tbl.row_count, 3);

        // 5. Paginate table data
        let page = SqliteTableService::get_table_data(
            &db_path,
            "experiments",
            1,
            2,
            Some("temperature"),
            Some("ASC"),
            None,
        )
        .unwrap();

        assert_eq!(page.total_rows, 3);
        assert_eq!(page.total_pages, 2);
        assert_eq!(page.rows.len(), 2);
        // lowest temperature first (4.2 then 77.0)
        assert_eq!(page.rows[0][1], serde_json::json!("Sample-C"));

        // 6. Search filter
        let filtered = SqliteTableService::get_table_data(
            &db_path,
            "experiments",
            1,
            10,
            None,
            None,
            Some("Sample-B"),
        )
        .unwrap();
        assert_eq!(filtered.total_rows, 1);
        assert_eq!(filtered.rows[0][1], serde_json::json!("Sample-B"));

        // 7. SQL Console Query
        let query_res = SqliteTableService::execute_sql(
            &db_path,
            "SELECT sample_name, temperature FROM experiments WHERE status = 'completed' ORDER BY temperature DESC",
            5000,
        )
        .unwrap();
        assert!(query_res.is_query);
        assert_eq!(query_res.rows.len(), 2);
        assert_eq!(query_res.columns, vec!["sample_name", "temperature"]);
        assert_eq!(query_res.rows[0][0], serde_json::json!("Sample-A"));
    }

    #[test]
    fn test_cell_edit_and_row_delete_without_id_column() {
        // Real user tables very often don't have a column literally named "id" (plan.md's own
        // example is "sample_id,frequency_ghz,fidelity"). The spreadsheet UI used to hardcode
        // row_id_col="id" for every cell edit and row delete, which silently matched zero rows
        // on any table like this one. `rowid` identifies any ordinary SQLite row regardless of
        // its declared column names, so that's what get_table_data now reports per row.
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("qubits.db");
        SqliteTableService::create_empty_database(&db_path).unwrap();

        SqliteTableService::execute_sql(
            &db_path,
            "CREATE TABLE qubit_telemetry (sample_id TEXT, frequency_ghz REAL, fidelity REAL);
             INSERT INTO qubit_telemetry VALUES ('Q1', 5.12, 0.994);
             INSERT INTO qubit_telemetry VALUES ('Q2', 5.34, 0.991);",
            5000,
        )
        .unwrap();

        let page = SqliteTableService::get_table_data(
            &db_path,
            "qubit_telemetry",
            1,
            10,
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(page.columns, vec!["sample_id", "frequency_ghz", "fidelity"]);
        assert_eq!(page.row_ids.len(), 2);

        let first_rowid = page.row_ids[0];
        SqliteTableService::update_cell(
            &db_path,
            "qubit_telemetry",
            "rowid",
            &first_rowid.to_string(),
            "fidelity",
            "0.999",
        )
        .unwrap();

        let reloaded = SqliteTableService::get_table_data(
            &db_path,
            "qubit_telemetry",
            1,
            10,
            None,
            None,
            None,
        )
        .unwrap();
        // SQLite's REAL column affinity converts the bound text "0.999" to a numeric storage class.
        assert_eq!(reloaded.rows[0][2], serde_json::json!(0.999));

        let second_rowid = page.row_ids[1];
        SqliteTableService::delete_row(
            &db_path,
            "qubit_telemetry",
            "rowid",
            &second_rowid.to_string(),
        )
        .unwrap();

        let after_delete = SqliteTableService::get_table_data(
            &db_path,
            "qubit_telemetry",
            1,
            10,
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(after_delete.total_rows, 1);
        assert_eq!(after_delete.rows[0][0], serde_json::json!("Q1"));
    }

    /// A `.csv` file should open through the exact same table machinery as a real `.table`
    /// SQLite file -- `resolve_db_path` materializes a `<name>.csv.table` sibling on first open,
    /// import the CSV's rows into it, and reuse that same sibling (not re-import, which would
    /// duplicate every row) on every later call.
    #[test]
    fn test_csv_file_resolves_to_backing_sqlite_table() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("readings.csv"),
            "qubit,t1_us\nQ0,94.2\nQ1,88.5\n",
        )
        .unwrap();

        let resolved_once =
            SqliteTableService::resolve_db_path(dir.path(), "readings.csv").unwrap();
        assert_eq!(resolved_once, dir.path().join("readings.csv.table"));
        assert!(resolved_once.exists());

        let data =
            SqliteTableService::get_table_data(&resolved_once, "readings", 1, 10, None, None, None)
                .unwrap();
        assert_eq!(data.total_rows, 2);

        // Resolving again must not duplicate rows.
        let resolved_twice =
            SqliteTableService::resolve_db_path(dir.path(), "readings.csv").unwrap();
        let data_again = SqliteTableService::get_table_data(
            &resolved_twice,
            "readings",
            1,
            10,
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(data_again.total_rows, 2);

        // The auto-materialized sibling shouldn't show up as its own separate entry.
        let dbs = SqliteTableService::discover_databases(dir.path()).unwrap();
        assert_eq!(
            dbs.iter()
                .map(|d| d.relative_path.as_str())
                .collect::<Vec<_>>(),
            vec!["readings.csv"]
        );
    }

    /// Tables previously could only show plain, unstyled data -- this exercises the full
    /// round trip a real cell-formatting toolbar drives: set a style, read it back attached to
    /// the right row/column, clear it back to default (which should delete the row rather than
    /// leave a default-valued one lying around), and confirm the metadata table it's stored in
    /// never shows up as a selectable table of its own.
    #[test]
    fn test_cell_style_round_trip_and_hidden_metadata_table() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("styled.sqlite");
        SqliteTableService::execute_sql(
            &db_path,
            "CREATE TABLE readings (qubit TEXT, t1_us REAL); \
             INSERT INTO readings VALUES ('Q0', 94.2), ('Q1', 88.5);",
            5000,
        )
        .unwrap();

        let data =
            SqliteTableService::get_table_data(&db_path, "readings", 1, 10, None, None, None)
                .unwrap();
        assert!(data.styles.is_empty(), "no styles set yet");
        let row0 = data.row_ids[0];

        let style = CellStyle {
            bold: true,
            italic: false,
            color: Some("#dc2626".to_string()),
            bg_color: Some("#fef2f2".to_string()),
        };
        SqliteTableService::set_cell_style(&db_path, "readings", row0, "qubit", &style).unwrap();

        let reloaded =
            SqliteTableService::get_table_data(&db_path, "readings", 1, 10, None, None, None)
                .unwrap();
        let key = format!("{}:qubit", row0);
        assert_eq!(reloaded.styles.get(&key), Some(&style));
        // Only the one styled cell should be present -- this is a sparse map, not one entry per
        // cell in the table.
        assert_eq!(reloaded.styles.len(), 1);

        // Clearing back to the default should remove the row entirely, not store a no-op default.
        SqliteTableService::set_cell_style(
            &db_path,
            "readings",
            row0,
            "qubit",
            &CellStyle::default(),
        )
        .unwrap();
        let cleared =
            SqliteTableService::get_table_data(&db_path, "readings", 1, 10, None, None, None)
                .unwrap();
        assert!(cleared.styles.is_empty());

        // The `_apich_cell_styles` bookkeeping table must never appear as a real, selectable
        // table in the schema.
        let schema = SqliteTableService::get_database_schema(&db_path).unwrap();
        assert!(
            schema.tables.iter().all(|t| t.name == "readings"),
            "internal _apich_cell_styles table leaked into the schema: {:?}",
            schema.tables.iter().map(|t| &t.name).collect::<Vec<_>>()
        );
    }
}
