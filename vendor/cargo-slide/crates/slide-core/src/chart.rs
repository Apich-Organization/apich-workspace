use crate::error::Result;
use crate::error::SlideError;
use crate::model::Rect;
use serde::Deserialize;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ChartType {
    #[default]
    Bar,
    Line,
    Area,
    Pie,
    Donut,
    Scatter,
}

impl std::str::FromStr for ChartType {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s.to_lowercase().trim() {
            | "line" => Self::Line,
            | "area" => Self::Area,
            | "pie" => Self::Pie,
            | "donut" => Self::Donut,
            | "scatter" => Self::Scatter,
            | _ => Self::Bar,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeriesData {
    pub name: String,
    pub values: Vec<f64>,
    #[serde(default)]
    pub color: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NumberFormat {
    #[default]
    Auto,
    Standard,
    Currency,
    Percentage,
    Compact,
    Scientific,
    Integer,
}

impl std::str::FromStr for NumberFormat {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s.to_lowercase().trim() {
            | "currency" | "money" | "cash" => Self::Currency,
            | "percentage" | "percent" | "pct" => Self::Percentage,
            | "compact" | "short" | "kmb" => Self::Compact,
            | "scientific" | "sci" | "exp" => Self::Scientific,
            | "integer" | "int" => Self::Integer,
            | "standard" | "std" | "decimal" => Self::Standard,
            | _ => Self::Auto,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChartData {
    pub chart_type: ChartType,
    pub title: Option<String>,
    pub categories: Vec<String>,
    pub series: Vec<SeriesData>,
    #[serde(default)]
    pub x_label: Option<String>,
    #[serde(default)]
    pub y_label: Option<String>,
    #[serde(default)]
    pub format: NumberFormat,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub prefix: Option<String>,
    #[serde(default)]
    pub precision: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ChartTransform {
    #[default]
    None,
    TopK(usize),
    SortDesc,
    SortAsc,
    Cumulative,
    Percent100,
    MovingAvg(usize),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChartStats {
    pub total_sum: f64,
    pub avg: f64,
    pub median: f64,
    pub std_dev: f64,
    pub max_val: f64,
    pub max_cat: String,
    pub max_series: String,
    pub min_val: f64,
    pub min_cat: String,
    pub count: usize,
}

impl Default for ChartData {
    fn default() -> Self {
        Self {
            chart_type: ChartType::Bar,
            title: None,
            categories: Vec::new(),
            series: Vec::new(),
            x_label: None,
            y_label: None,
            format: NumberFormat::Auto,
            unit: None,
            prefix: None,
            precision: None,
        }
    }
}

pub const DEFAULT_CHART_COLORS: [&str; 6] = [
    "#38bdf8", // Sky blue
    "#34d399", // Emerald green
    "#f59e0b", // Amber / orange
    "#f43f5e", // Rose / red
    "#a855f7", // Purple
    "#6366f1", // Indigo
];

impl ChartData {
    #[must_use]
    pub const fn new(chart_type: ChartType) -> Self {
        Self {
            chart_type,
            title: None,
            categories: Vec::new(),
            series: Vec::new(),
            x_label: None,
            y_label: None,
            format: NumberFormat::Auto,
            unit: None,
            prefix: None,
            precision: None,
        }
    }

    pub fn with_title(
        mut self,
        title: impl Into<String>,
    ) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_format(
        mut self,
        format: NumberFormat,
    ) -> Self {
        self.format = format;
        self
    }

    pub fn with_unit(
        mut self,
        unit: impl Into<String>,
    ) -> Self {
        self.unit = Some(unit.into());
        self
    }

    pub fn with_prefix(
        mut self,
        prefix: impl Into<String>,
    ) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    pub const fn with_precision(
        mut self,
        precision: usize,
    ) -> Self {
        self.precision = Some(precision);
        self
    }

    /// Parse numeric string tolerating currency symbols, commas, and percentage signs
    #[must_use]
    pub fn parse_numeric_cell(cell: &str) -> f64 {
        let cleaned = cell
            .trim()
            .trim_matches(|c| c == '$' || c == '€' || c == '£' || c == '¥' || c == '%' || c == ',')
            .replace(',', "");
        cleaned.parse::<f64>().unwrap_or(0.0)
    }

    /// Parse CSV formatted string into `ChartData`
    pub fn from_csv_str(
        csv_text: &str,
        chart_type: ChartType,
        title: Option<String>,
    ) -> Result<Self> {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .flexible(true)
            .from_reader(csv_text.as_bytes());

        let headers = rdr
            .headers()
            .map_err(|e| SlideError::Chart(format!("Failed to parse CSV headers: {e}")))?
            .clone();

        if headers.is_empty() {
            return Err(SlideError::Chart("CSV headers are empty".to_string()));
        }

        let mut series_names = Vec::new();
        for (i, h) in headers.iter().enumerate() {
            if i > 0 {
                series_names.push(h.trim().to_string());
            }
        }

        let mut categories = Vec::new();
        let mut series_values: Vec<Vec<f64>> = vec![Vec::new(); series_names.len()];

        for result in rdr.records() {
            let record =
                result.map_err(|e| SlideError::Chart(format!("Failed to read CSV row: {e}")))?;
            if record.is_empty() {
                continue;
            }

            let cat = record.get(0).unwrap_or("").trim().to_string();
            categories.push(cat);

            for (i, val_vec) in series_values.iter_mut().enumerate() {
                let cell = record.get(i + 1).unwrap_or("0");
                val_vec.push(Self::parse_numeric_cell(cell));
            }
        }

        let series = series_names
            .into_iter()
            .zip(series_values)
            .enumerate()
            .map(|(idx, (name, values))| {
                SeriesData {
                    name,
                    values,
                    color: Some(DEFAULT_CHART_COLORS[idx % DEFAULT_CHART_COLORS.len()].to_string()),
                }
            })
            .collect();

        Ok(Self {
            chart_type,
            title,
            categories,
            series,
            x_label: None,
            y_label: None,
            format: NumberFormat::Auto,
            unit: None,
            prefix: None,
            precision: None,
        })
    }

    /// Read and parse CSV from a file path
    pub fn from_csv_file(
        path: &Path,
        chart_type: ChartType,
        title: Option<String>,
    ) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            SlideError::Chart(format!("Failed to open CSV file {}: {}", path.display(), e))
        })?;
        Self::from_csv_str(&content, chart_type, title)
    }

    /// Query a SQLite database and convert rows into `ChartData`
    pub fn from_sqlite(
        db_path: &Path,
        query: &str,
        chart_type: ChartType,
        title: Option<String>,
    ) -> Result<Self> {
        let conn = rusqlite::Connection::open(db_path).map_err(|e| {
            SlideError::Database(format!(
                "Failed to open SQLite database {}: {}",
                db_path.display(),
                e
            ))
        })?;

        let mut stmt = conn.prepare(query).map_err(|e| {
            SlideError::Database(format!("SQLite statement error in query '{query}': {e}"))
        })?;

        let column_count = stmt.column_count();
        if column_count < 2 {
            return Err(SlideError::Database(
                "SQLite query must return at least 2 columns (Category, Value1, ...)".to_string(),
            ));
        }

        let mut series_names = Vec::new();
        for i in 1..column_count {
            series_names.push(stmt.column_name(i).unwrap_or("Series").to_string());
        }

        let mut categories = Vec::new();
        let mut series_values: Vec<Vec<f64>> = vec![Vec::new(); series_names.len()];

        let mut rows = stmt
            .query([])
            .map_err(|e| SlideError::Database(format!("Failed to execute SQLite query: {e}")))?;

        while let Some(row) = rows
            .next()
            .map_err(|e| SlideError::Database(format!("Error fetching SQLite row: {e}")))?
        {
            // Category: column 0
            let cat_str: String = if let Ok(s) = row.get(0) {
                s
            } else if let Ok(i) = row.get::<_, i64>(0) {
                i.to_string()
            } else if let Ok(f) = row.get::<_, f64>(0) {
                format!("{f:.1}")
            } else {
                categories.len().to_string()
            };
            categories.push(cat_str);

            for (i, val_vec) in series_values.iter_mut().enumerate() {
                let val: f64 = if let Ok(f) = row.get(i + 1) {
                    f
                } else if let Ok(i_val) = row.get::<_, i64>(i + 1) {
                    i_val as f64
                } else if let Ok(s_val) = row.get::<_, String>(i + 1) {
                    Self::parse_numeric_cell(&s_val)
                } else {
                    0.0
                };
                val_vec.push(val);
            }
        }

        let series = series_names
            .into_iter()
            .zip(series_values)
            .enumerate()
            .map(|(idx, (name, values))| {
                SeriesData {
                    name,
                    values,
                    color: Some(DEFAULT_CHART_COLORS[idx % DEFAULT_CHART_COLORS.len()].to_string()),
                }
            })
            .collect();

        Ok(Self {
            chart_type,
            title,
            categories,
            series,
            x_label: None,
            y_label: None,
            format: NumberFormat::Auto,
            unit: None,
            prefix: None,
            precision: None,
        })
    }

    /// Parse JSON or JSONL formatted string into `ChartData`
    pub fn from_json_str(
        json_text: &str,
        chart_type: ChartType,
        title: Option<String>,
    ) -> Result<Self> {
        let trimmed = json_text.trim();
        if trimmed.is_empty() {
            return Err(SlideError::Chart("JSON content is empty".to_string()));
        }

        // Attempt 1: Direct serde_json deserialization of ChartData
        if let Ok(mut data) = serde_json::from_str::<Self>(trimmed) {
            data.chart_type = chart_type;
            if title.is_some() {
                data.title = title;
            }
            return Ok(data);
        }

        // Attempt 2: Array of objects or records: [{"category": "A", "s1": 10, "s2": 20}, ...]
        // or JSONL: lines of {"category": "A", ...}
        let parsed_val: serde_json::Value = if trimmed.starts_with('[') {
            serde_json::from_str(trimmed)
                .map_err(|e| SlideError::Chart(format!("Failed to parse JSON: {e}")))?
        } else if trimmed.starts_with('{') && !trimmed.contains('\n') {
            serde_json::from_str(trimmed)
                .map_err(|e| SlideError::Chart(format!("Failed to parse JSON: {e}")))?
        } else {
            // Attempt JSON Lines (JSONL)
            let mut array = Vec::new();
            for line in trimmed.lines() {
                let line_trim = line.trim();
                if line_trim.is_empty() {
                    continue;
                }
                let obj: serde_json::Value = serde_json::from_str(line_trim)
                    .map_err(|e| SlideError::Chart(format!("Failed to parse JSONL line: {e}")))?;
                array.push(obj);
            }
            serde_json::Value::Array(array)
        };

        match parsed_val {
            | serde_json::Value::Array(arr) => {
                if arr.is_empty() {
                    return Err(SlideError::Chart("JSON array is empty".to_string()));
                }

                let first_obj = arr[0].as_object().ok_or_else(|| {
                    SlideError::Chart("JSON array elements must be objects".to_string())
                })?;

                // Determine which key is category
                let mut cat_key = "category".to_string();
                for k in [
                    "category", "cat", "name", "label", "x", "date", "month", "year", "time",
                    "item",
                ] {
                    if first_obj.contains_key(k) {
                        cat_key = k.to_string();
                        break;
                    }
                }
                if !first_obj.contains_key(&cat_key)
                    && let Some((first_k, _)) = first_obj.iter().next()
                {
                    cat_key = first_k.clone();
                }

                // Ordered series names
                let mut series_names = Vec::new();
                for k in first_obj.keys() {
                    if k != &cat_key {
                        series_names.push(k.clone());
                    }
                }

                let mut categories = Vec::new();
                let mut series_values: Vec<Vec<f64>> = vec![Vec::new(); series_names.len()];

                for item in arr {
                    let obj = match item.as_object() {
                        | Some(o) => o,
                        | None => continue,
                    };

                    let cat = obj.get(&cat_key).map_or_else(
                        || categories.len().to_string(),
                        |v| {
                            match v {
                                | serde_json::Value::String(s) => s.clone(),
                                | serde_json::Value::Number(n) => n.to_string(),
                                | _ => v.to_string(),
                            }
                        },
                    );
                    categories.push(cat);

                    for (s_idx, s_name) in series_names.iter().enumerate() {
                        let num = obj.get(s_name).map_or(0.0, |v| {
                            match v {
                                | serde_json::Value::Number(n) => n.as_f64().unwrap_or(0.0),
                                | serde_json::Value::String(s) => Self::parse_numeric_cell(s),
                                | _ => 0.0,
                            }
                        });
                        series_values[s_idx].push(num);
                    }
                }

                let series = series_names
                    .into_iter()
                    .zip(series_values)
                    .enumerate()
                    .map(|(idx, (name, values))| {
                        SeriesData {
                            name,
                            values,
                            color: Some(
                                DEFAULT_CHART_COLORS[idx % DEFAULT_CHART_COLORS.len()].to_string(),
                            ),
                        }
                    })
                    .collect();

                Ok(Self {
                    chart_type,
                    title,
                    categories,
                    series,
                    x_label: None,
                    y_label: None,
                    format: NumberFormat::Auto,
                    unit: None,
                    prefix: None,
                    precision: None,
                })
            },
            | serde_json::Value::Object(map) => {
                let mut categories = Vec::new();
                let mut values = Vec::new();

                for (k, v) in map {
                    categories.push(k);
                    let val = match v {
                        | serde_json::Value::Number(n) => n.as_f64().unwrap_or(0.0),
                        | serde_json::Value::String(s) => Self::parse_numeric_cell(&s),
                        | _ => 0.0,
                    };
                    values.push(val);
                }

                let series = vec![SeriesData {
                    name: "Value".to_string(),
                    values,
                    color: Some(DEFAULT_CHART_COLORS[0].to_string()),
                }];

                Ok(Self {
                    chart_type,
                    title,
                    categories,
                    series,
                    x_label: None,
                    y_label: None,
                    format: NumberFormat::Auto,
                    unit: None,
                    prefix: None,
                    precision: None,
                })
            },
            | _ => {
                Err(SlideError::Chart(
                    "Unsupported JSON chart structure".to_string(),
                ))
            },
        }
    }

    /// Read and parse JSON from a file path
    pub fn from_json_file(
        path: &Path,
        chart_type: ChartType,
        title: Option<String>,
    ) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            SlideError::Chart(format!(
                "Failed to open JSON file {}: {}",
                path.display(),
                e
            ))
        })?;
        Self::from_json_str(&content, chart_type, title)
    }

    /// Execute an arbitrary SQL query against this chart's data using an in-memory SQLite table `data`
    pub fn query_sql(
        &self,
        sql: &str,
    ) -> Result<Self> {
        let mut conn = rusqlite::Connection::open_in_memory().map_err(|e| {
            SlideError::Database(format!("Failed to create in-memory SQLite database: {e}"))
        })?;

        // Helper to sanitize column names for SQL table schema
        fn sanitize_col_name(
            name: &str,
            idx: usize,
        ) -> String {
            let mut sanitized: String = name
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() || c == '_' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect();
            if sanitized.is_empty() || sanitized.chars().next().unwrap().is_ascii_digit() {
                sanitized = format!("col_{idx}_{sanitized}");
            }
            sanitized
        }

        let mut col_names = Vec::new();
        for (i, s) in self.series.iter().enumerate() {
            col_names.push(sanitize_col_name(&s.name, i));
        }

        let mut create_sql =
            String::from("CREATE TABLE data (id INTEGER PRIMARY KEY, category TEXT");
        for col in &col_names {
            create_sql.push_str(&format!(", \"{col}\" REAL"));
        }
        create_sql.push(')');

        conn.execute(&create_sql, [])
            .map_err(|e| SlideError::Database(format!("Failed to create in-memory table: {e}")))?;

        let mut insert_sql = String::from("INSERT INTO data (id, category");
        for col in &col_names {
            insert_sql.push_str(&format!(", \"{col}\""));
        }
        insert_sql.push_str(") VALUES (?1, ?2");
        for (i, _) in col_names.iter().enumerate() {
            insert_sql.push_str(&format!(", ?{}", i.saturating_add(3)));
        }
        insert_sql.push(')');

        let row_count = self.categories.len();
        {
            let tx = conn
                .transaction()
                .map_err(|e| SlideError::Database(format!("Failed to start transaction: {e}")))?;
            {
                let mut stmt = tx.prepare(&insert_sql).map_err(|e| {
                    SlideError::Database(format!("Failed to prepare insert statement: {e}"))
                })?;
                for row_idx in 0..row_count {
                    let cat = self.categories.get(row_idx).cloned().unwrap_or_default();
                    let mut params: Vec<Box<dyn rusqlite::ToSql>> =
                        Vec::with_capacity(self.series.len().saturating_add(2));
                    let id = (row_idx.saturating_add(1)) as i64;
                    params.push(Box::new(id));
                    params.push(Box::new(cat));
                    for s in &self.series {
                        let v = s.values.get(row_idx).copied().unwrap_or(0.0);
                        params.push(Box::new(v));
                    }

                    let slice: Vec<&dyn rusqlite::ToSql> =
                        params.iter().map(std::convert::AsRef::as_ref).collect();
                    stmt.execute(rusqlite::params_from_iter(slice))
                        .map_err(|e| {
                            SlideError::Database(format!(
                                "Failed to insert row into in-memory table: {e}"
                            ))
                        })?;
                }
            }
            tx.commit().map_err(|e| {
                SlideError::Database(format!("Failed to commit insert transaction: {e}"))
            })?;
        }

        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| SlideError::Database(format!("SQL syntax error in '{sql}': {e}")))?;

        let col_count = stmt.column_count();
        if col_count == 0 {
            return Err(SlideError::Database(
                "SQL query returned no columns".to_string(),
            ));
        }

        let mut out_series_names = Vec::new();
        for i in 1..col_count {
            out_series_names.push(stmt.column_name(i).unwrap_or("Series").to_string());
        }
        if out_series_names.is_empty() {
            out_series_names.push(stmt.column_name(0).unwrap_or("Value").to_string());
        }

        let mut out_categories = Vec::new();
        let mut out_series_values: Vec<Vec<f64>> = vec![Vec::new(); out_series_names.len()];

        let mut rows = stmt.query([]).map_err(|e| {
            SlideError::Database(format!("Failed to execute SQL query '{sql}': {e}"))
        })?;

        let mut row_counter = 1usize;
        while let Some(row) = rows
            .next()
            .map_err(|e| SlideError::Database(format!("Error fetching SQL row: {e}")))?
        {
            if col_count == 1 {
                out_categories.push(row_counter.to_string());
                let val: f64 = row
                    .get(0)
                    .unwrap_or_else(|_| row.get::<_, i64>(0).map_or(0.0, |i| i as f64));
                out_series_values[0].push(val);
            } else {
                let cat: String = if let Ok(s) = row.get(0) {
                    s
                } else if let Ok(i) = row.get::<_, i64>(0) {
                    i.to_string()
                } else if let Ok(f) = row.get::<_, f64>(0) {
                    format!("{f:.1}")
                } else {
                    row_counter.to_string()
                };
                out_categories.push(cat);

                for (s_i, s_vals) in out_series_values.iter_mut().enumerate() {
                    let val: f64 = if let Ok(f) = row.get(s_i + 1) {
                        f
                    } else if let Ok(i) = row.get::<_, i64>(s_i + 1) {
                        i as f64
                    } else if let Ok(s) = row.get::<_, String>(s_i + 1) {
                        Self::parse_numeric_cell(&s)
                    } else {
                        0.0
                    };
                    s_vals.push(val);
                }
            }
            row_counter += 1;
        }

        let out_series = out_series_names
            .into_iter()
            .zip(out_series_values)
            .enumerate()
            .map(|(idx, (name, values))| {
                SeriesData {
                    name,
                    values,
                    color: Some(DEFAULT_CHART_COLORS[idx % DEFAULT_CHART_COLORS.len()].to_string()),
                }
            })
            .collect();

        Ok(Self {
            chart_type: self.chart_type,
            title: self.title.clone(),
            categories: out_categories,
            series: out_series,
            x_label: self.x_label.clone(),
            y_label: self.y_label.clone(),
            format: self.format,
            unit: self.unit.clone(),
            prefix: self.prefix.clone(),
            precision: self.precision,
        })
    }

    /// Apply a concise data pipeline DSL (separated by `|`)
    ///
    /// Supported commands:
    /// - `filter <col> <op> <val>`: Filter rows (e.g. `filter Sales > 100`, `filter cat contains Q`)
    /// - `sort [col] [asc|desc]`: Sort categories by column or total (e.g. `sort desc`, `sort Rev asc`)
    /// - `limit <N>` / `top <N>`: Keep first N rows
    /// - `tail <N>` / `last <N>`: Keep last N rows
    /// - `top_k <N>`: Sort descending by total and keep top N
    /// - `select <s1>, <s2>`: Keep only specified series
    /// - `smooth [window]`: Moving average smoothing (default 3)
    /// - `cumulative` / `cumsum`: Running cumulative sum along series
    /// - `percent` / `share`: 100% normalization per category
    pub fn apply_dsl(
        &self,
        dsl: &str,
    ) -> Result<Self> {
        let mut cur = self.clone();

        for stage in dsl.split('|') {
            let stage_trim = stage.trim();
            if stage_trim.is_empty() {
                continue;
            }

            let tokens: Vec<&str> = stage_trim.split_whitespace().collect();
            if tokens.is_empty() {
                continue;
            }

            let cmd = tokens[0].to_lowercase();
            match cmd.as_str() {
                | "filter" | "where" => {
                    if tokens.len() < 4 {
                        return Err(SlideError::Chart(format!(
                            "Invalid filter syntax '{stage_trim}': expected 'filter <col> <op> <val>'"
                        )));
                    }
                    let col = tokens[1];
                    let op = tokens[2];
                    let val_str = tokens[3..].join(" ");
                    cur = cur.filter_rows(col, op, &val_str)?;
                },
                | "sort" | "order" => {
                    let (col_opt, desc) = if tokens.len() == 2 {
                        let arg = tokens[1].to_lowercase();
                        if arg == "desc" {
                            (None, true)
                        } else if arg == "asc" {
                            (None, false)
                        } else {
                            (Some(tokens[1]), false)
                        }
                    } else if tokens.len() >= 3 {
                        let desc = tokens[2].eq_ignore_ascii_case("desc");
                        (Some(tokens[1]), desc)
                    } else {
                        (None, true) // default sort desc
                    };
                    cur = cur.sort_rows(col_opt, desc)?;
                },
                | "limit" | "top" | "head" => {
                    if let Some(n_str) = tokens.get(1) {
                        let n: usize = n_str.parse().unwrap_or(5);
                        cur = cur.limit_rows(n);
                    }
                },
                | "tail" | "last" => {
                    if let Some(n_str) = tokens.get(1) {
                        let n: usize = n_str.parse().unwrap_or(5);
                        cur = cur.tail_rows(n);
                    }
                },
                | "top_k" => {
                    let n: usize = tokens.get(1).and_then(|s| s.parse().ok()).unwrap_or(5);
                    cur = cur.sort_rows(None, true)?;
                    cur = cur.limit_rows(n);
                },
                | "select" => {
                    let names_raw = tokens[1..].join(" ");
                    let wanted: Vec<String> = names_raw
                        .split(&[',', ' '][..])
                        .map(|s| s.trim().to_lowercase())
                        .filter(|s| !s.is_empty())
                        .collect();
                    cur.series
                        .retain(|s| wanted.iter().any(|w| s.name.to_lowercase().contains(w)));
                },
                | "smooth" | "moving_avg" | "ma" => {
                    let window: usize = tokens.get(1).and_then(|s| s.parse().ok()).unwrap_or(3);
                    cur = cur.smooth_moving_avg(window);
                },
                | "cumulative" | "cumsum" | "cum_sum" => {
                    cur = cur.cumulative_sum();
                },
                | "percent" | "share" | "percent100" | "normalize_100" => {
                    cur = cur.normalize_100();
                },
                | _ => {
                    // Unknown DSL command, ignore or log
                },
            }
        }

        Ok(cur)
    }

    /// Apply standard preset `ChartTransform`
    #[must_use]
    pub fn apply_transform(
        &self,
        transform: ChartTransform,
    ) -> Self {
        match transform {
            | ChartTransform::None => self.clone(),
            | ChartTransform::TopK(k) => {
                self.apply_dsl(&format!("top_k {k}"))
                    .unwrap_or_else(|_| self.clone())
            },
            | ChartTransform::SortDesc => {
                self.apply_dsl("sort desc").unwrap_or_else(|_| self.clone())
            },
            | ChartTransform::SortAsc => {
                self.apply_dsl("sort asc").unwrap_or_else(|_| self.clone())
            },
            | ChartTransform::Cumulative => {
                self.apply_dsl("cumulative")
                    .unwrap_or_else(|_| self.clone())
            },
            | ChartTransform::Percent100 => {
                self.apply_dsl("percent").unwrap_or_else(|_| self.clone())
            },
            | ChartTransform::MovingAvg(w) => {
                self.apply_dsl(&format!("smooth {w}"))
                    .unwrap_or_else(|_| self.clone())
            },
        }
    }

    fn filter_rows(
        &self,
        col: &str,
        op: &str,
        val_str: &str,
    ) -> Result<Self> {
        let is_cat = col.eq_ignore_ascii_case("category") || col.eq_ignore_ascii_case("cat");
        let is_total = col.eq_ignore_ascii_case("total") || col.eq_ignore_ascii_case("sum");

        let series_idx = if !is_cat && !is_total {
            self.series
                .iter()
                .position(|s| s.name.eq_ignore_ascii_case(col))
        } else {
            None
        };

        let num_val = val_str.parse::<f64>().ok();

        let mut keep_indices = Vec::new();
        for (i, cat) in self.categories.iter().enumerate() {
            let matched = if is_cat {
                match op {
                    | "==" | "=" => cat.eq_ignore_ascii_case(val_str),
                    | "!=" => !cat.eq_ignore_ascii_case(val_str),
                    | "contains" => cat.to_lowercase().contains(&val_str.to_lowercase()),
                    | _ => true,
                }
            } else {
                let cell_num = if is_total {
                    self.series
                        .iter()
                        .map(|s| s.values.get(i).copied().unwrap_or(0.0))
                        .sum()
                } else if let Some(s_i) = series_idx {
                    self.series[s_i].values.get(i).copied().unwrap_or(0.0)
                } else {
                    self.series
                        .first()
                        .and_then(|s| s.values.get(i).copied())
                        .unwrap_or(0.0)
                };

                if let Some(target) = num_val {
                    match op {
                        | ">" => cell_num > target,
                        | ">=" => cell_num >= target,
                        | "<" => cell_num < target,
                        | "<=" => cell_num <= target,
                        | "==" | "=" => (cell_num - target).abs() < 1e-6,
                        | "!=" => (cell_num - target).abs() >= 1e-6,
                        | _ => true,
                    }
                } else {
                    true
                }
            };

            if matched {
                keep_indices.push(i);
            }
        }

        let new_categories = keep_indices
            .iter()
            .map(|&i| self.categories[i].clone())
            .collect();
        let new_series = self
            .series
            .iter()
            .map(|s| {
                SeriesData {
                    name: s.name.clone(),
                    values: keep_indices
                        .iter()
                        .map(|&i| s.values.get(i).copied().unwrap_or(0.0))
                        .collect(),
                    color: s.color.clone(),
                }
            })
            .collect();

        Ok(Self {
            chart_type: self.chart_type,
            title: self.title.clone(),
            categories: new_categories,
            series: new_series,
            x_label: self.x_label.clone(),
            y_label: self.y_label.clone(),
            format: self.format,
            unit: self.unit.clone(),
            prefix: self.prefix.clone(),
            precision: self.precision,
        })
    }

    fn sort_rows(
        &self,
        col: Option<&str>,
        desc: bool,
    ) -> Result<Self> {
        let count = self.categories.len();
        if count <= 1 {
            return Ok(self.clone());
        }

        let mut row_indices: Vec<usize> = (0..count).collect();

        let is_cat = col
            .is_some_and(|c| c.eq_ignore_ascii_case("category") || c.eq_ignore_ascii_case("cat"));
        let series_idx = if is_cat {
            None
        } else {
            col.and_then(|c| {
                self.series
                    .iter()
                    .position(|s| s.name.eq_ignore_ascii_case(c))
            })
        };

        row_indices.sort_by(|&a, &b| {
            let ordering = if is_cat {
                self.categories[a].cmp(&self.categories[b])
            } else if let Some(s_i) = series_idx {
                let va = self.series[s_i].values.get(a).copied().unwrap_or(0.0);
                let vb = self.series[s_i].values.get(b).copied().unwrap_or(0.0);
                va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
            } else {
                // Default: compare row total sum
                let sum_a: f64 = self
                    .series
                    .iter()
                    .map(|s| s.values.get(a).copied().unwrap_or(0.0))
                    .sum();
                let sum_b: f64 = self
                    .series
                    .iter()
                    .map(|s| s.values.get(b).copied().unwrap_or(0.0))
                    .sum();
                sum_a
                    .partial_cmp(&sum_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            };

            if desc {
                ordering.reverse()
            } else {
                ordering
            }
        });

        let new_categories = row_indices
            .iter()
            .map(|&i| self.categories[i].clone())
            .collect();
        let new_series = self
            .series
            .iter()
            .map(|s| {
                SeriesData {
                    name: s.name.clone(),
                    values: row_indices
                        .iter()
                        .map(|&i| s.values.get(i).copied().unwrap_or(0.0))
                        .collect(),
                    color: s.color.clone(),
                }
            })
            .collect();

        Ok(Self {
            chart_type: self.chart_type,
            title: self.title.clone(),
            categories: new_categories,
            series: new_series,
            x_label: self.x_label.clone(),
            y_label: self.y_label.clone(),
            format: self.format,
            unit: self.unit.clone(),
            prefix: self.prefix.clone(),
            precision: self.precision,
        })
    }

    fn limit_rows(
        &self,
        n: usize,
    ) -> Self {
        let new_categories: Vec<String> = self.categories.iter().take(n).cloned().collect();
        let new_series = self
            .series
            .iter()
            .map(|s| {
                SeriesData {
                    name: s.name.clone(),
                    values: s.values.iter().take(n).copied().collect(),
                    color: s.color.clone(),
                }
            })
            .collect();

        Self {
            chart_type: self.chart_type,
            title: self.title.clone(),
            categories: new_categories,
            series: new_series,
            x_label: self.x_label.clone(),
            y_label: self.y_label.clone(),
            format: self.format,
            unit: self.unit.clone(),
            prefix: self.prefix.clone(),
            precision: self.precision,
        }
    }

    fn tail_rows(
        &self,
        n: usize,
    ) -> Self {
        let skip = self.categories.len().saturating_sub(n);
        let new_categories: Vec<String> = self.categories.iter().skip(skip).cloned().collect();
        let new_series = self
            .series
            .iter()
            .map(|s| {
                SeriesData {
                    name: s.name.clone(),
                    values: s.values.iter().skip(skip).copied().collect(),
                    color: s.color.clone(),
                }
            })
            .collect();

        Self {
            chart_type: self.chart_type,
            title: self.title.clone(),
            categories: new_categories,
            series: new_series,
            x_label: self.x_label.clone(),
            y_label: self.y_label.clone(),
            format: self.format,
            unit: self.unit.clone(),
            prefix: self.prefix.clone(),
            precision: self.precision,
        }
    }

    fn smooth_moving_avg(
        &self,
        window: usize,
    ) -> Self {
        let w = window.max(1);
        let new_series = self
            .series
            .iter()
            .map(|s| {
                let mut smoothed = Vec::with_capacity(s.values.len());
                for i in 0..s.values.len() {
                    let start = i.saturating_sub(w - 1);
                    let slice = &s.values[start..=i];
                    let avg = slice.iter().sum::<f64>() / slice.len() as f64;
                    smoothed.push(avg);
                }
                SeriesData {
                    name: format!("{} (MA{})", s.name, w),
                    values: smoothed,
                    color: s.color.clone(),
                }
            })
            .collect();

        Self {
            chart_type: self.chart_type,
            title: self.title.clone(),
            categories: self.categories.clone(),
            series: new_series,
            x_label: self.x_label.clone(),
            y_label: self.y_label.clone(),
            format: self.format,
            unit: self.unit.clone(),
            prefix: self.prefix.clone(),
            precision: self.precision,
        }
    }

    fn cumulative_sum(&self) -> Self {
        let new_series = self
            .series
            .iter()
            .map(|s| {
                let mut cum = Vec::with_capacity(s.values.len());
                let mut running = 0.0f64;
                for &v in &s.values {
                    running += v;
                    cum.push(running);
                }
                SeriesData {
                    name: format!("{} (Cum)", s.name),
                    values: cum,
                    color: s.color.clone(),
                }
            })
            .collect();

        Self {
            chart_type: self.chart_type,
            title: self.title.clone(),
            categories: self.categories.clone(),
            series: new_series,
            x_label: self.x_label.clone(),
            y_label: self.y_label.clone(),
            format: self.format,
            unit: self.unit.clone(),
            prefix: self.prefix.clone(),
            precision: self.precision,
        }
    }

    fn normalize_100(&self) -> Self {
        let cat_len = self.categories.len();
        let mut totals = vec![0.0f64; cat_len];
        for s in &self.series {
            for (i, &v) in s.values.iter().enumerate() {
                if i < cat_len {
                    totals[i] += v;
                }
            }
        }

        let new_series = self
            .series
            .iter()
            .map(|s| {
                let mut pcts = Vec::with_capacity(s.values.len());
                for (i, &v) in s.values.iter().enumerate() {
                    let t = totals.get(i).copied().unwrap_or(0.0);
                    let pct = if t > 0.0 {
                        (v / t) * 100.0
                    } else {
                        0.0
                    };
                    pcts.push(pct);
                }
                SeriesData {
                    name: s.name.clone(),
                    values: pcts,
                    color: s.color.clone(),
                }
            })
            .collect();

        Self {
            chart_type: self.chart_type,
            title: self.title.clone(),
            categories: self.categories.clone(),
            series: new_series,
            x_label: self.x_label.clone(),
            y_label: Some("% Share".to_string()),
            format: self.format,
            unit: self.unit.clone(),
            prefix: self.prefix.clone(),
            precision: self.precision,
        }
    }

    /// Serialize this chart's dataset into a standard CSV string
    #[must_use]
    pub fn to_csv(&self) -> String {
        let mut out = String::new();
        out.push_str("Category");
        for s in &self.series {
            out.push(',');
            out.push_str(&s.name);
        }
        out.push('\n');

        for (i, cat) in self.categories.iter().enumerate() {
            out.push_str(cat);
            for s in &self.series {
                out.push(',');
                let v = s.values.get(i).copied().unwrap_or(0.0);
                out.push_str(&v.to_string());
            }
            out.push('\n');
        }

        out
    }

    /// Minimum value across all series (clamped to 0.0 minimum if all positive)
    #[must_use]
    pub fn min_value(&self) -> f64 {
        let mut min = 0.0f64;
        for s in &self.series {
            for &v in &s.values {
                if v < min {
                    min = v;
                }
            }
        }
        min
    }

    /// Maximum value across all series
    #[must_use]
    pub fn max_value(&self) -> f64 {
        let mut max = 0.0f64;
        for s in &self.series {
            for &v in &s.values {
                if v > max {
                    max = v;
                }
            }
        }
        if max <= 0.0 { 1.0 } else { max }
    }

    /// Calculate statistical summary for visible series across specified category indices
    #[must_use]
    pub fn summary_stats_filtered(
        &self,
        hidden_series: &std::collections::HashSet<usize>,
        category_indices: &[usize],
    ) -> ChartStats {
        let mut total_sum = 0.0f64;
        let mut max_val = f64::NEG_INFINITY;
        let mut min_val = f64::INFINITY;
        let mut max_c_idx = None;
        let mut max_s_idx = None;
        let mut min_c_idx = None;
        let mut all_vals =
            Vec::with_capacity(self.series.len().saturating_mul(category_indices.len()));

        for (s_idx, s) in self.series.iter().enumerate() {
            if hidden_series.contains(&s_idx) {
                continue;
            }
            for &c_idx in category_indices {
                if let Some(&val) = s.values.get(c_idx) {
                    all_vals.push(val);
                    total_sum += val;
                    if val > max_val {
                        max_val = val;
                        max_c_idx = Some(c_idx);
                        max_s_idx = Some(s_idx);
                    }
                    if val < min_val {
                        min_val = val;
                        min_c_idx = Some(c_idx);
                    }
                }
            }
        }

        let max_cat = max_c_idx
            .and_then(|idx| self.categories.get(idx))
            .cloned()
            .unwrap_or_default();
        let max_series = max_s_idx
            .and_then(|idx| self.series.get(idx))
            .map(|s| s.name.clone())
            .unwrap_or_default();
        let min_cat = min_c_idx
            .and_then(|idx| self.categories.get(idx))
            .cloned()
            .unwrap_or_default();

        let count = all_vals.len();
        let avg = if count > 0 {
            total_sum / count as f64
        } else {
            0.0
        };

        all_vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = if count == 0 {
            0.0
        } else if count % 2 == 0 {
            let mid = count / 2;
            let v1 = all_vals.get(mid.saturating_sub(1)).copied().unwrap_or(0.0);
            let v2 = all_vals.get(mid).copied().unwrap_or(0.0);
            (v1 + v2) / 2.0
        } else {
            all_vals.get(count / 2).copied().unwrap_or(0.0)
        };

        let variance = if count > 0 {
            all_vals
                .iter()
                .map(|&v| {
                    let diff = v - avg;
                    diff * diff
                })
                .sum::<f64>()
                / count as f64
        } else {
            0.0
        };
        let std_dev = variance.sqrt();

        if max_val == f64::NEG_INFINITY {
            max_val = 0.0;
        }
        if min_val == f64::INFINITY {
            min_val = 0.0;
        }

        ChartStats {
            total_sum,
            avg,
            median,
            std_dev,
            max_val,
            max_cat,
            max_series,
            min_val,
            min_cat,
            count,
        }
    }

    /// Calculate statistical summary for visible series across all categories
    #[must_use]
    pub fn summary_stats(
        &self,
        hidden_series: &std::collections::HashSet<usize>,
    ) -> ChartStats {
        let all_cats: Vec<usize> = (0..self.categories.len()).collect();
        self.summary_stats_filtered(hidden_series, &all_cats)
    }

    /// Create a clone with hidden series excluded
    #[must_use]
    pub fn with_hidden_filtered(
        &self,
        hidden_series: &std::collections::HashSet<usize>,
    ) -> Self {
        let mut clone = self.clone();
        let filtered: Vec<SeriesData> = self
            .series
            .iter()
            .enumerate()
            .filter(|(idx, _)| !hidden_series.contains(idx))
            .map(|(_, s)| s.clone())
            .collect();
        clone.series = filtered;
        clone
    }

    /// Export chart data to CSV format, filtering out hidden series and including total summary
    #[must_use]
    pub fn export_csv(
        &self,
        hidden_series: &std::collections::HashSet<usize>,
    ) -> String {
        let mut csv = String::new();
        // Header row
        csv.push_str("Category");
        for (idx, s) in self.series.iter().enumerate() {
            if !hidden_series.contains(&idx) {
                csv.push(',');
                if s.name.contains(',') || s.name.contains('"') {
                    csv.push_str(&format!("\"{}\"", s.name.replace('"', "\"\"")));
                } else {
                    csv.push_str(&s.name);
                }
            }
        }
        csv.push('\n');

        // Data rows
        for (c_idx, cat) in self.categories.iter().enumerate() {
            if cat.contains(',') || cat.contains('"') {
                csv.push_str(&format!("\"{}\"", cat.replace('"', "\"\"")));
            } else {
                csv.push_str(cat);
            }
            for (s_idx, s) in self.series.iter().enumerate() {
                if !hidden_series.contains(&s_idx) {
                    csv.push(',');
                    let val = s.values.get(c_idx).copied().unwrap_or(0.0);
                    csv.push_str(&Self::format_value(val));
                }
            }
            csv.push('\n');
        }

        // Summary Total row
        csv.push_str("TOTAL");
        for (s_idx, s) in self.series.iter().enumerate() {
            if !hidden_series.contains(&s_idx) {
                csv.push(',');
                let sum: f64 = s.values.iter().sum();
                csv.push_str(&Self::format_value(sum));
            }
        }
        csv.push('\n');

        csv
    }

    /// Calculate nice rounded ticks for Y-axis (min, max, ticks)
    #[must_use]
    pub fn nice_scale(
        &self,
        tick_count: usize,
    ) -> (f64, f64, Vec<f64>) {
        let min = self.min_value();
        let max = self.max_value();
        let diff = max - min;
        if diff <= 1e-9 || !diff.is_finite() {
            let base = if max.abs() > 1e-9 { max } else { 1.0 };
            return (0.0, base, vec![0.0, base * 0.5, base]);
        }

        let range = nice_number(diff, false).max(1e-6);
        let tick_spacing = nice_number(range / (tick_count.max(2) - 1) as f64, true).max(1e-6);
        let nice_min = (min / tick_spacing).floor() * tick_spacing;
        let nice_max = (max / tick_spacing).ceil() * tick_spacing;

        let mut ticks = Vec::new();
        let mut current = nice_min;
        let limit = 0.5f64.mul_add(tick_spacing, nice_max);
        let mut count = 0;
        while current <= limit && count < 100 {
            ticks.push(current);
            current += tick_spacing;
            count += 1;
        }

        (nice_min, nice_max, ticks)
    }

    /// Format number nicely for HUD labels (e.g. 1.25M, 450K, 32.5, 100)
    #[must_use]
    pub fn format_value(val: f64) -> String {
        let abs = val.abs();
        if abs >= 1_000_000.0 {
            format!("{:.2}M", val / 1_000_000.0)
        } else if abs >= 1_000.0 {
            format!("{:.1}K", val / 1_000.0)
        } else if abs.fract() == 0.0 {
            format!("{val:.0}")
        } else {
            format!("{val:.1}")
        }
    }

    /// Format number according to configured NumberFormat, prefix, unit, and precision
    #[must_use]
    pub fn format_number(
        &self,
        val: f64,
    ) -> String {
        let prec = self.precision.unwrap_or(match self.format {
            | NumberFormat::Currency => 2,
            | NumberFormat::Percentage => 1,
            | NumberFormat::Integer => 0,
            | NumberFormat::Scientific => 2,
            | NumberFormat::Standard => 2,
            | NumberFormat::Compact | NumberFormat::Auto => 1,
        });

        let mut formatted = match self.format {
            | NumberFormat::Auto => Self::format_value(val),
            | NumberFormat::Compact => {
                let abs = val.abs();
                if abs >= 1_000_000_000.0 {
                    format!("{:.prec$}B", val / 1_000_000_000.0)
                } else if abs >= 1_000_000.0 {
                    format!("{:.prec$}M", val / 1_000_000.0)
                } else if abs >= 1_000.0 {
                    format!("{:.prec$}K", val / 1_000.0)
                } else {
                    format_with_commas(val, prec)
                }
            },
            | NumberFormat::Currency => {
                let formatted_num = format_with_commas(val, prec);
                let p = self.prefix.as_deref().unwrap_or("$");
                format!("{p}{formatted_num}")
            },
            | NumberFormat::Percentage => {
                format!("{val:.prec$}%")
            },
            | NumberFormat::Integer => format_with_commas(val.round(), 0),
            | NumberFormat::Scientific => {
                format!("{val:.prec$e}")
            },
            | NumberFormat::Standard => format_with_commas(val, prec),
        };

        if let Some(ref p) = self.prefix
            && self.format != NumberFormat::Currency
            && !formatted.starts_with(p.as_str())
        {
            formatted = format!("{p}{formatted}");
        }

        if let Some(ref u) = self.unit
            && !formatted.ends_with(u.as_str())
        {
            if u.starts_with('%') || u.starts_with('°') {
                formatted.push_str(u);
            } else {
                formatted.push(' ');
                formatted.push_str(u);
            }
        }

        formatted
    }
}

/// Helper to insert commas into integer portion without arithmetic side-effects
fn format_with_commas(
    val: f64,
    prec: usize,
) -> String {
    let raw = format!("{val:.prec$}");
    let (int_part, fract_part) = raw.split_once('.').unwrap_or((&raw, ""));
    let (sign, digits) = if let Some(stripped) = int_part.strip_prefix('-') {
        ("-", stripped)
    } else {
        ("", int_part)
    };

    let mut rev_chars = Vec::with_capacity(digits.len().saturating_add(4));
    for (i, ch) in digits.chars().rev().enumerate() {
        if i > 0 && i.is_multiple_of(3) {
            rev_chars.push(',');
        }
        rev_chars.push(ch);
    }
    let formatted_int: String = rev_chars.into_iter().rev().collect();

    if fract_part.is_empty() {
        format!("{sign}{formatted_int}")
    } else {
        format!("{sign}{formatted_int}.{fract_part}")
    }
}

impl ChartData {
    #[must_use]
    pub fn plot_area(
        &self,
        chart_rect: Rect,
    ) -> Rect {
        let left = 46.0f32;
        let right = 26.0f32;
        let top = if self.title.is_some() {
            35.0f32
        } else {
            20.0f32
        };
        let bottom = 25.0f32;

        Rect::new(
            chart_rect.x + left,
            chart_rect.y + top,
            (chart_rect.width - left - right).max(10.0),
            (chart_rect.height - top - bottom).max(10.0),
        )
    }

    /// Hit-test category index for Bar, Line, Area, Scatter charts
    #[must_use]
    pub fn hit_test_category(
        &self,
        chart_rect: Rect,
        mouse_x: f32,
        mouse_y: f32,
    ) -> Option<usize> {
        if self.categories.is_empty() {
            return None;
        }
        let plot = self.plot_area(chart_rect);
        if mouse_x >= plot.x
            && mouse_x <= (plot.x + plot.width)
            && mouse_y >= plot.y
            && mouse_y <= (plot.y + plot.height)
        {
            let t = ((mouse_x - plot.x) / plot.width).clamp(0.0, 0.999);
            let idx = (t * self.categories.len() as f32).floor() as usize;
            Some(idx.min(self.categories.len() - 1))
        } else {
            None
        }
    }

    /// Hit-test slice index for Pie and Donut charts
    #[must_use]
    pub fn hit_test_pie_slice(
        &self,
        chart_rect: Rect,
        mouse_x: f32,
        mouse_y: f32,
    ) -> Option<usize> {
        if self.series.is_empty() || self.categories.is_empty() {
            return None;
        }

        let cx = chart_rect.width.mul_add(0.5, chart_rect.x);
        let cy = chart_rect.height.mul_add(0.55, chart_rect.y);
        let radius = (chart_rect.width.min(chart_rect.height) * 0.38).max(10.0);
        let inner_radius = if self.chart_type == ChartType::Donut {
            radius * 0.5
        } else {
            0.0
        };

        let dx = mouse_x - cx;
        let dy = mouse_y - cy;
        let dist = dx.hypot(dy);

        if dist < inner_radius || dist > radius {
            return None;
        }

        let mut angle = dy.atan2(dx);
        if angle < 0.0 {
            angle += std::f32::consts::TAU;
        }

        // Aggregate values (either single series across categories, or multiple series)
        let values: Vec<f64> = if self.series.len() == 1 {
            self.series[0].values.clone()
        } else {
            self.series
                .iter()
                .map(|s| s.values.first().copied().unwrap_or(0.0))
                .collect()
        };

        let total: f64 = values.iter().sum();
        if total <= 0.0 {
            return None;
        }

        let mut current_angle = 0.0f32;
        for (i, &v) in values.iter().enumerate() {
            let slice_angle = ((v / total) as f32) * std::f32::consts::TAU;
            if angle >= current_angle && angle <= (current_angle + slice_angle) {
                return Some(i);
            }
            current_angle += slice_angle;
        }

        None
    }

    /// Standalone SVG generation for full visual rendering
    #[must_use]
    pub fn render_svg(
        &self,
        width: f32,
        height: f32,
    ) -> String {
        let mut svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#
        );

        // Dark card background with subtle border
        svg.push_str(&format!(
            r##"<rect x="0" y="0" width="{width}" height="{height}" rx="10" fill="#161b22" stroke="#30363d" stroke-width="1.5"/>"##
        ));

        // Chart Title
        if let Some(ref title) = self.title {
            svg.push_str(&format!(
                r##"<text x="20" y="24" fill="#58a6ff" font-size="14" font-weight="bold" font-family="sans-serif">{title}</text>"##
            ));
        }

        // Legend at top right
        let mut legend_x = width - 20.0;
        for (idx, s) in self.series.iter().enumerate().rev() {
            let color = s
                .color
                .as_deref()
                .unwrap_or(DEFAULT_CHART_COLORS[idx % DEFAULT_CHART_COLORS.len()]);
            let text_len = s.name.len() as f32 * 7.5;
            legend_x -= text_len + 16.0;
            svg.push_str(&format!(
                r#"<rect x="{legend_x}" y="14" width="8" height="8" rx="2" fill="{color}"/>"#
            ));
            svg.push_str(&format!(
                r##"<text x="{}" y="22" fill="#8b949e" font-size="10" font-family="sans-serif">{}</text>"##,
                legend_x + 12.0, s.name
            ));
        }

        let plot = self.plot_area(Rect::new(0.0, 0.0, width, height));
        let (y_min, y_max, ticks) = self.nice_scale(5);

        // Y-axis grid lines and labels
        for tick in &ticks {
            let t = ((tick - y_min) / (y_max - y_min).max(1e-6)) as f32;
            let y = plot.y + plot.height - t * plot.height;
            svg.push_str(&format!(
                r##"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="#21262d" stroke-dasharray="3,3" stroke-width="1"/>"##,
                plot.x, y, plot.x + plot.width, y
            ));
            let label = Self::format_value(*tick);
            svg.push_str(&format!(
                r##"<text x="{}" y="{}" fill="#8b949e" font-size="9" text-anchor="end" font-family="sans-serif">{}</text>"##,
                plot.x - 8.0, y + 3.0, label
            ));
        }

        // X-axis baseline
        let baseline_y = plot.y + plot.height;
        svg.push_str(&format!(
            r##"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="#30363d" stroke-width="1.5"/>"##,
            plot.x,
            baseline_y,
            plot.x + plot.width,
            baseline_y
        ));

        // Render chart contents based on chart type
        match self.chart_type {
            | ChartType::Bar => {
                let cat_count = self.categories.len().max(1);
                let col_width = plot.width / cat_count as f32;
                let group_pad = col_width * 0.2;
                let bar_area_w = col_width - group_pad * 2.0;
                let series_count = self.series.len().max(1);
                let single_bar_w = bar_area_w / series_count as f32;

                for (cat_idx, cat_name) in self.categories.iter().enumerate() {
                    let cat_x = (cat_idx as f32).mul_add(col_width, plot.x);

                    // X-axis label
                    svg.push_str(&format!(
                        r##"<text x="{}" y="{}" fill="#8b949e" font-size="10" text-anchor="middle" font-family="sans-serif">{}</text>"##,
                        cat_x + col_width * 0.5, baseline_y + 16.0, cat_name
                    ));

                    for (s_idx, s) in self.series.iter().enumerate() {
                        let color = s
                            .color
                            .as_deref()
                            .unwrap_or(DEFAULT_CHART_COLORS[s_idx % DEFAULT_CHART_COLORS.len()]);
                        let val = s.values.get(cat_idx).copied().unwrap_or(0.0);
                        let bar_h = (((val - y_min) / (y_max - y_min).max(1e-6)) as f32
                            * plot.height)
                            .max(0.0);
                        let bx = (s_idx as f32).mul_add(single_bar_w, cat_x + group_pad);
                        let by = baseline_y - bar_h;

                        svg.push_str(&format!(
                            r#"<rect x="{}" y="{}" width="{}" height="{}" rx="3" fill="{}" opacity="0.9"/>"#,
                            bx, by, single_bar_w.max(2.0) - 2.0, bar_h, color
                        ));
                    }
                }
            },
            | ChartType::Line | ChartType::Area => {
                let cat_count = self.categories.len().max(1);
                let col_width = plot.width / cat_count as f32;

                // X-axis labels
                for (cat_idx, cat_name) in self.categories.iter().enumerate() {
                    let cx = (cat_idx as f32 + 0.5).mul_add(col_width, plot.x);
                    svg.push_str(&format!(
                        r##"<text x="{}" y="{}" fill="#8b949e" font-size="10" text-anchor="middle" font-family="sans-serif">{}</text>"##,
                        cx, baseline_y + 16.0, cat_name
                    ));
                }

                for (s_idx, s) in self.series.iter().enumerate() {
                    let color = s
                        .color
                        .as_deref()
                        .unwrap_or(DEFAULT_CHART_COLORS[s_idx % DEFAULT_CHART_COLORS.len()]);
                    let mut pts: Vec<(f32, f32)> = Vec::new();

                    for (cat_idx, &val) in s.values.iter().enumerate() {
                        let cx = (cat_idx as f32 + 0.5).mul_add(col_width, plot.x);
                        let cy = (((val - y_min) / (y_max - y_min).max(1e-6)) as f32)
                            .mul_add(-plot.height, baseline_y);
                        pts.push((cx, cy));
                    }

                    if self.chart_type == ChartType::Area && !pts.is_empty() {
                        let mut path_str = format!("M {},{} ", pts[0].0, baseline_y);
                        for p in &pts {
                            path_str.push_str(&format!("L {},{} ", p.0, p.1));
                        }
                        path_str.push_str(&format!("L {},{} Z", pts.last().unwrap().0, baseline_y));
                        svg.push_str(&format!(
                            r#"<path d="{path_str}" fill="{color}" opacity="0.25"/>"#
                        ));
                    }

                    // Stroke line
                    if pts.len() >= 2 {
                        let mut poly = String::new();
                        for (i, p) in pts.iter().enumerate() {
                            if i > 0 {
                                poly.push(' ');
                            }
                            poly.push_str(&format!("{},{}", p.0, p.1));
                        }
                        svg.push_str(&format!(
                            r#"<polyline points="{poly}" fill="none" stroke="{color}" stroke-width="2.5" stroke-linejoin="round"/>"#
                        ));
                    }

                    // Data point circles
                    for p in &pts {
                        svg.push_str(&format!(
                            r##"<circle cx="{}" cy="{}" r="4" fill="{}" stroke="#161b22" stroke-width="2"/>"##,
                            p.0, p.1, color
                        ));
                    }
                }
            },
            | _ => {},
        }

        svg.push_str("</svg>");
        svg
    }
}

fn nice_number(
    range: f64,
    round: bool,
) -> f64 {
    if range <= 0.0 || !range.is_finite() {
        return 1.0;
    }
    let exponent = range.log10().floor();
    let fraction = range / 10.0f64.powf(exponent);

    let nice_fraction = if round {
        if fraction < 1.5 {
            1.0
        } else if fraction < 3.0 {
            2.0
        } else if fraction < 7.0 {
            5.0
        } else {
            10.0
        }
    } else {
        if fraction <= 1.0 {
            1.0
        } else if fraction <= 2.0 {
            2.0
        } else if fraction <= 5.0 {
            5.0
        } else {
            10.0
        }
    };

    nice_fraction * 10.0f64.powf(exponent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csv_parsing() {
        let csv_data = "\
Quarter,Revenue,Profit,Growth
Q1 2025,\"$120,500\",$34000,12.5%
Q2 2025,\"$150,000\",$45500,15.2%
Q3 2025,\"$210,000\",$72000,22.8%
Q4 2025,\"$280,000\",$98000,31.4%
";

        let chart =
            ChartData::from_csv_str(csv_data, ChartType::Bar, Some("Financial Growth".into()))
                .unwrap();
        assert_eq!(chart.title.as_deref(), Some("Financial Growth"));
        assert_eq!(
            chart.categories,
            vec!["Q1 2025", "Q2 2025", "Q3 2025", "Q4 2025"]
        );
        assert_eq!(chart.series.len(), 3);
        assert_eq!(chart.series[0].name, "Revenue");
        assert_eq!(
            chart.series[0].values,
            vec![120500.0, 150000.0, 210000.0, 280000.0]
        );
        assert_eq!(chart.series[1].name, "Profit");
        assert_eq!(
            chart.series[1].values,
            vec![34000.0, 45500.0, 72000.0, 98000.0]
        );
        assert_eq!(chart.series[2].name, "Growth");
        assert_eq!(chart.series[2].values, vec![12.5, 15.2, 22.8, 31.4]);
    }

    #[test]
    fn test_sqlite_querying() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("metrics.db");

        // Set up sqlite database and populate sample records
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute(
            "CREATE TABLE server_load (id INTEGER PRIMARY KEY, region TEXT, cpu_load REAL, mem_gb REAL)",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO server_load (region, cpu_load, mem_gb) VALUES ('US-East', 64.5, 128.0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO server_load (region, cpu_load, mem_gb) VALUES ('EU-West', 42.0, 96.0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO server_load (region, cpu_load, mem_gb) VALUES ('AP-South', 78.2, 160.0)",
            [],
        )
        .unwrap();

        let chart = ChartData::from_sqlite(
            &db_path,
            "SELECT region, cpu_load, mem_gb FROM server_load ORDER BY id ASC",
            ChartType::Line,
            Some("Multi-Region Cloud Load".into()),
        )
        .unwrap();

        assert_eq!(chart.chart_type, ChartType::Line);
        assert_eq!(chart.title.as_deref(), Some("Multi-Region Cloud Load"));
        assert_eq!(chart.categories, vec!["US-East", "EU-West", "AP-South"]);
        assert_eq!(chart.series.len(), 2);
        assert_eq!(chart.series[0].name, "cpu_load");
        assert_eq!(chart.series[0].values, vec![64.5, 42.0, 78.2]);
        assert_eq!(chart.series[1].name, "mem_gb");
        assert_eq!(chart.series[1].values, vec![128.0, 96.0, 160.0]);

        // Verify CSV export
        let csv = chart.to_csv();
        assert!(csv.contains("Category,cpu_load,mem_gb"));
        assert!(csv.contains("US-East,64.5,128"));
    }

    #[test]
    fn test_nice_scale_and_format() {
        let mut chart = ChartData::new(ChartType::Bar);
        chart.series.push(SeriesData {
            name: "Test".into(),
            values: vec![12.0, 85.0, 192.0],
            color: None,
        });

        let (y_min, y_max, ticks) = chart.nice_scale(5);
        assert!(y_min <= 0.0);
        assert!(y_max >= 192.0);
        assert!(ticks.len() >= 4);

        assert_eq!(ChartData::format_value(1_500_000.0), "1.50M");
        assert_eq!(ChartData::format_value(450_000.0), "450.0K");
        assert_eq!(ChartData::format_value(25.0), "25");
    }

    #[test]
    fn test_hit_testing() {
        let mut chart = ChartData::new(ChartType::Bar);
        chart.categories = vec!["A".into(), "B".into(), "C".into(), "D".into()];
        chart.series.push(SeriesData {
            name: "S1".into(),
            values: vec![10.0, 20.0, 30.0, 40.0],
            color: None,
        });

        let rect = Rect::new(0.0, 0.0, 400.0, 300.0);
        let plot = chart.plot_area(rect);

        // Test mouse inside first quarter of plot area
        let hit = chart.hit_test_category(rect, plot.x + 10.0, plot.y + 10.0);
        assert_eq!(hit, Some(0));

        // Test mouse inside last quarter of plot area
        let hit_last = chart.hit_test_category(rect, plot.x + plot.width - 5.0, plot.y + 10.0);
        assert_eq!(hit_last, Some(3));

        // Test mouse outside plot area
        let hit_outside = chart.hit_test_category(rect, 5.0, 5.0);
        assert_eq!(hit_outside, None);
    }

    #[test]
    fn test_nice_scale_edge_cases() {
        // Test all zeroes
        let mut chart_zero = ChartData::new(ChartType::Line);
        chart_zero.series.push(SeriesData {
            name: "Zeroes".into(),
            values: vec![0.0, 0.0, 0.0],
            color: None,
        });
        let (_, _, ticks) = chart_zero.nice_scale(5);
        assert!(!ticks.is_empty());

        // Test constant non-zero value
        let mut chart_const = ChartData::new(ChartType::Line);
        chart_const.series.push(SeriesData {
            name: "Const".into(),
            values: vec![100.0, 100.0, 100.0],
            color: None,
        });
        let (_, _, ticks) = chart_const.nice_scale(5);
        assert!(!ticks.is_empty());
    }

    #[test]
    fn test_export_csv() {
        let mut chart = ChartData::new(ChartType::Bar);
        chart.categories = vec!["Q1".into(), "Q2".into()];
        chart.series.push(SeriesData {
            name: "Sales".into(),
            values: vec![100.0, 250.0],
            color: None,
        });
        chart.series.push(SeriesData {
            name: "Costs".into(),
            values: vec![80.0, 150.0],
            color: None,
        });

        let mut hidden = std::collections::HashSet::new();
        hidden.insert(1); // Hide Costs

        let csv = chart.export_csv(&hidden);
        assert!(csv.contains("Category,Sales\n"));
        assert!(csv.contains("Q1,100\n"));
        assert!(csv.contains("Q2,250\n"));
        assert!(csv.contains("TOTAL,350\n"));
        assert!(!csv.contains("Costs"));
    }

    #[test]
    fn test_from_json_formats() {
        // Test array of objects
        let json_arr = r#"[
            {"category": "Jan", "sales": 120.5, "profit": 35.0},
            {"category": "Feb", "sales": 150.0, "profit": 42.0},
            {"category": "Mar", "sales": 210.0, "profit": 60.0}
        ]"#;
        let chart =
            ChartData::from_json_str(json_arr, ChartType::Line, Some("Sales 2026".into())).unwrap();
        assert_eq!(chart.chart_type, ChartType::Line);
        assert_eq!(chart.categories, vec!["Jan", "Feb", "Mar"]);
        assert_eq!(chart.series.len(), 2);
        let has_sales = chart
            .series
            .iter()
            .any(|s| s.name == "sales" && s.values == vec![120.5, 150.0, 210.0]);
        let has_profit = chart
            .series
            .iter()
            .any(|s| s.name == "profit" && s.values == vec![35.0, 42.0, 60.0]);
        assert!(has_sales);
        assert!(has_profit);

        // Test key-value object
        let json_kv = r#"{"Alpha": 10.0, "Beta": 25.5, "Gamma": 40.0}"#;
        let chart_kv = ChartData::from_json_str(json_kv, ChartType::Donut, None).unwrap();
        assert_eq!(chart_kv.chart_type, ChartType::Donut);
        assert_eq!(chart_kv.categories.len(), 3);
        assert_eq!(chart_kv.series[0].values.len(), 3);

        // Test JSON Lines (JSONL)
        let jsonl = "{\"cat\": \"2024\", \"val\": 100}\n{\"cat\": \"2025\", \"val\": 200}\n";
        let chart_jsonl = ChartData::from_json_str(jsonl, ChartType::Bar, None).unwrap();
        assert_eq!(chart_jsonl.categories, vec!["2024", "2025"]);
        assert_eq!(chart_jsonl.series[0].values, vec![100.0, 200.0]);
    }

    #[test]
    fn test_in_memory_sql_queries() {
        let mut chart = ChartData::new(ChartType::Bar);
        chart.categories = vec!["A".into(), "B".into(), "C".into(), "D".into()];
        chart.series.push(SeriesData {
            name: "sales".into(),
            values: vec![50.0, 150.0, 80.0, 200.0],
            color: None,
        });
        chart.series.push(SeriesData {
            name: "profit".into(),
            values: vec![10.0, 30.0, 15.0, 50.0],
            color: None,
        });

        // Filter and Order By SQL
        let sql = "SELECT category, sales, profit FROM data WHERE sales >= 100 ORDER BY sales DESC";
        let res = chart.query_sql(sql).unwrap();
        assert_eq!(res.categories, vec!["D", "B"]);
        assert_eq!(res.series[0].values, vec![200.0, 150.0]);

        // Computed column SQL
        let sql_calc =
            "SELECT category, (sales - profit) AS cost FROM data ORDER BY cost ASC LIMIT 2";
        let res_calc = chart.query_sql(sql_calc).unwrap();
        assert_eq!(res_calc.categories, vec!["A", "C"]);
        assert_eq!(res_calc.series[0].name, "cost");
        assert_eq!(res_calc.series[0].values, vec![40.0, 65.0]);
    }

    #[test]
    fn test_dsl_pipeline_transformations() {
        let mut chart = ChartData::new(ChartType::Bar);
        chart.categories = vec![
            "P1".into(),
            "P2".into(),
            "P3".into(),
            "P4".into(),
            "P5".into(),
        ];
        chart.series.push(SeriesData {
            name: "score".into(),
            values: vec![10.0, 80.0, 30.0, 95.0, 45.0],
            color: None,
        });

        // Test filter | sort | limit
        let dsl = "filter score >= 40 | sort desc | limit 2";
        let res = chart.apply_dsl(dsl).unwrap();
        assert_eq!(res.categories, vec!["P4", "P2"]);
        assert_eq!(res.series[0].values, vec![95.0, 80.0]);

        // Test cumulative
        let res_cum = chart.apply_dsl("limit 3 | cumulative").unwrap();
        assert_eq!(res_cum.series[0].values, vec![10.0, 90.0, 120.0]);

        // Test percent normalization
        let mut chart_multi = ChartData::new(ChartType::Bar);
        chart_multi.categories = vec!["Cat1".into()];
        chart_multi.series.push(SeriesData {
            name: "S1".into(),
            values: vec![25.0],
            color: None,
        });
        chart_multi.series.push(SeriesData {
            name: "S2".into(),
            values: vec![75.0],
            color: None,
        });
        let res_pct = chart_multi.apply_transform(ChartTransform::Percent100);
        assert_eq!(res_pct.series[0].values[0], 25.0);
        assert_eq!(res_pct.series[1].values[0], 75.0);

        // Test moving average
        let mut chart_ma = ChartData::new(ChartType::Line);
        chart_ma.categories = vec!["T1".into(), "T2".into(), "T3".into()];
        chart_ma.series.push(SeriesData {
            name: "Val".into(),
            values: vec![10.0, 20.0, 30.0],
            color: None,
        });
        let res_ma = chart_ma.apply_transform(ChartTransform::MovingAvg(2));
        assert_eq!(res_ma.series[0].values, vec![10.0, 15.0, 25.0]);
    }

    #[test]
    fn test_custom_formatting_and_statistics() {
        let mut chart = ChartData::new(ChartType::Bar)
            .with_format(NumberFormat::Currency)
            .with_prefix("$")
            .with_unit("USD")
            .with_precision(2);

        chart.categories = vec!["A".into(), "B".into(), "C".into(), "D".into(), "E".into()];
        chart.series.push(SeriesData {
            name: "Revenue".into(),
            values: vec![1000.0, 2500.5, 500.0, 4000.0, 1500.0],
            color: None,
        });

        assert_eq!(chart.format_number(1_234_567.89), "$1,234,567.89 USD");
        assert_eq!(chart.format_number(2500.5), "$2,500.50 USD");

        let stats = chart.summary_stats(&std::collections::HashSet::new());
        assert_eq!(stats.count, 5);
        assert_eq!(stats.total_sum, 9500.5);
        assert_eq!(stats.avg, 1900.1);
        assert_eq!(stats.median, 1500.0);
        assert!(stats.std_dev > 0.0);
        assert_eq!(stats.max_val, 4000.0);
        assert_eq!(stats.min_val, 500.0);

        // Filtered stats (only categories 0 and 2)
        let filtered_stats =
            chart.summary_stats_filtered(&std::collections::HashSet::new(), &[0, 2]);
        assert_eq!(filtered_stats.count, 2);
        assert_eq!(filtered_stats.total_sum, 1500.0);
        assert_eq!(filtered_stats.avg, 750.0);
        assert_eq!(filtered_stats.median, 750.0);
    }
}
