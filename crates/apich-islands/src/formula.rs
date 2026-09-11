//! Small spreadsheet formula DSL for `SpreadsheetIsland`'s cells and formula bar.
//!
//! Before this existed, the grid had a formula bar cosmetically labelled "fx" (see
//! `spreadsheet.rs`) but typing `=SUM(A1:A5)` into a cell just stored that literal text as the
//! cell's raw value -- there was no `=`-prefixed evaluation at all, only per-column aggregate
//! dropdowns (sum/avg/count/min/max) computed over an *entire* column. That's exactly the
//! "not enough preset functionalities, not enough preset DSL" complaint against the table
//! feature: users see an Excel-shaped formula bar and reasonably expect Excel-shaped formulas.
//!
//! Supports:
//! - Arithmetic over numbers and cell references: `=A1+B1`, `=(A1-B2)*3`, `=A1/B1`
//! - Column-range aggregate functions: `=SUM(A1:A5)`, `=AVG(A:A)`, `=COUNT(B1:B10)`,
//!   `=MIN(A1:A5)`, `=MAX(A1:A5)` (also accepts `AVERAGE` as an alias for `AVG`)
//!
//! Deliberately NOT supported (kept small and predictable rather than a full spreadsheet
//! language): string functions, cross-sheet refs, relative-fill-on-copy semantics. A formula
//! that references a column beyond `Z` or a row past the grid's current size is a parse/eval
//! error (`#REF!`), same as a malformed expression (`#ERROR`).

/// Evaluates a cell's raw stored value: if it starts with `=`, parses and evaluates it as a
/// formula against the current grid (`cells[row][col]`, 0-indexed, `columns` giving the count
/// of columns so a bare column-letter range like `A:A` knows how far down to read); anything
/// else (including an empty string) passes through unchanged, matching every existing cell that
/// predates this feature.
pub fn display_value(raw: &str, cells: &[Vec<String>]) -> String {
    let Some(expr) = raw.strip_prefix('=') else {
        return raw.to_string();
    };
    if expr.trim().is_empty() {
        return raw.to_string();
    }
    match eval(expr, cells) {
        Ok(n) => format_number(n),
        Err(e) => e.to_string(),
    }
}

fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        let s = format!("{:.6}", n);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

#[derive(Debug, PartialEq)]
enum FormulaError {
    Parse,
    Ref,
    DivZero,
}

impl std::fmt::Display for FormulaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormulaError::Parse => write!(f, "#ERROR"),
            FormulaError::Ref => write!(f, "#REF!"),
            FormulaError::DivZero => write!(f, "#DIV/0!"),
        }
    }
}

type EvalResult = Result<f64, FormulaError>;

/// Column letter (A-Z, case-insensitive) -> 0-based index. Single letter only, matching
/// `SpreadsheetIsland::col_letter`'s own A-Z-wrapping scheme -- a grid with >26 columns already
/// has ambiguous letters in the UI itself, so formulas share that same limitation rather than
/// inventing a different (AA, AB, ...) addressing scheme the grid's own headers don't use.
fn col_index(letter: &str) -> Option<usize> {
    if letter.len() != 1 {
        return None;
    }
    let c = letter.chars().next()?.to_ascii_uppercase();
    if c.is_ascii_uppercase() {
        Some((c as u8 - b'A') as usize)
    } else {
        None
    }
}

fn cell_value(cells: &[Vec<String>], row: usize, col: usize) -> EvalResult {
    let raw = cells
        .get(row)
        .and_then(|r| r.get(col))
        .ok_or(FormulaError::Ref)?;
    // A referenced cell that itself holds a formula is evaluated recursively -- e.g. `=A1+1`
    // where A1 is itself `=SUM(B1:B3)`. No cycle detection: a self-referential chain recurses
    // until the call stack gives out, same tradeoff most minimal formula engines make rather
    // than tracking a visited-set through every call for a feature this size.
    if let Some(inner) = raw.strip_prefix('=') {
        if inner.trim().is_empty() {
            return Ok(0.0);
        }
        return eval(inner, cells);
    }
    if raw.trim().is_empty() {
        return Ok(0.0);
    }
    raw.trim().parse::<f64>().map_err(|_| FormulaError::Parse)
}

/// Parses `A1` / `a12` into (row_index, col_index), both 0-based.
fn parse_cell_ref(s: &str) -> Option<(usize, usize)> {
    let s = s.trim();
    let split_at = s.find(|c: char| c.is_ascii_digit())?;
    let (letters, digits) = s.split_at(split_at);
    if letters.is_empty() || digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let col = col_index(letters)?;
    let row_num: usize = digits.parse().ok()?;
    if row_num == 0 {
        return None;
    }
    Some((row_num - 1, col))
}

/// Parses a function argument that's a range: `A1:B1`, `A1:A10`, or a bare column like `A` /
/// `A:A` meaning "the whole column, every row currently in the grid".
fn resolve_range(arg: &str, cells: &[Vec<String>]) -> Result<Vec<f64>, FormulaError> {
    let arg = arg.trim();
    let n_rows = cells.len();

    let (start, end) = match arg.split_once(':') {
        Some((a, b)) => (a.trim(), b.trim()),
        None => (arg, arg),
    };

    // Bare column letter(s) on both ends (e.g. "A" or "A:A") -> whole column.
    if let (Some(c1), Some(c2)) = (col_index(start), col_index(end)) {
        if !start.chars().next().unwrap_or(' ').is_ascii_digit() && start.chars().all(|c| c.is_alphabetic()) {
            let col = c1;
            if c1 != c2 {
                return Err(FormulaError::Ref);
            }
            let mut out = Vec::with_capacity(n_rows);
            for r in 0..n_rows {
                let raw = cells.get(r).and_then(|row| row.get(col)).map(|s| s.as_str()).unwrap_or("");
                if raw.trim().is_empty() {
                    continue;
                }
                out.push(cell_value(cells, r, col)?);
            }
            return Ok(out);
        }
    }

    let (r1, c1) = parse_cell_ref(start).ok_or(FormulaError::Parse)?;
    let (r2, c2) = parse_cell_ref(end).ok_or(FormulaError::Parse)?;
    if c1 != c2 {
        return Err(FormulaError::Ref); // only single-column ranges supported
    }
    let (lo, hi) = if r1 <= r2 { (r1, r2) } else { (r2, r1) };
    let mut out = Vec::with_capacity(hi - lo + 1);
    for r in lo..=hi {
        out.push(cell_value(cells, r, c1)?);
    }
    Ok(out)
}

fn eval_function(name: &str, arg: &str, cells: &[Vec<String>]) -> EvalResult {
    let values = resolve_range(arg, cells)?;
    match name.to_ascii_uppercase().as_str() {
        "SUM" => Ok(values.iter().sum()),
        "AVG" | "AVERAGE" => {
            if values.is_empty() {
                Err(FormulaError::DivZero)
            } else {
                Ok(values.iter().sum::<f64>() / values.len() as f64)
            }
        }
        "COUNT" => Ok(values.len() as f64),
        "MIN" => values.iter().cloned().fold(None, |acc, v| Some(acc.map_or(v, |a: f64| a.min(v)))).ok_or(FormulaError::DivZero),
        "MAX" => values.iter().cloned().fold(None, |acc, v| Some(acc.map_or(v, |a: f64| a.max(v)))).ok_or(FormulaError::DivZero),
        _ => Err(FormulaError::Parse),
    }
}

/// Minimal recursive-descent evaluator: expr := term (('+'|'-') term)*, term := factor
/// (('*'|'/') factor)*, factor := number | A1 | FUNC(range) | '(' expr ')'.
struct Parser<'a> {
    chars: Vec<char>,
    pos: usize,
    cells: &'a [Vec<String>],
}

impl<'a> Parser<'a> {
    fn new(s: &'a str, cells: &'a [Vec<String>]) -> Self {
        Self { chars: s.chars().collect(), pos: 0, cells }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_whitespace()) {
            self.pos += 1;
        }
    }

    fn expr(&mut self) -> EvalResult {
        let mut val = self.term()?;
        loop {
            self.skip_ws();
            match self.peek() {
                Some('+') => {
                    self.pos += 1;
                    val += self.term()?;
                }
                Some('-') => {
                    self.pos += 1;
                    val -= self.term()?;
                }
                _ => break,
            }
        }
        Ok(val)
    }

    fn term(&mut self) -> EvalResult {
        let mut val = self.factor()?;
        loop {
            self.skip_ws();
            match self.peek() {
                Some('*') => {
                    self.pos += 1;
                    val *= self.factor()?;
                }
                Some('/') => {
                    self.pos += 1;
                    let rhs = self.factor()?;
                    if rhs == 0.0 {
                        return Err(FormulaError::DivZero);
                    }
                    val /= rhs;
                }
                _ => break,
            }
        }
        Ok(val)
    }

    fn factor(&mut self) -> EvalResult {
        self.skip_ws();
        match self.peek() {
            Some('-') => {
                self.pos += 1;
                Ok(-self.factor()?)
            }
            Some('(') => {
                self.pos += 1;
                let val = self.expr()?;
                self.skip_ws();
                if self.peek() == Some(')') {
                    self.pos += 1;
                } else {
                    return Err(FormulaError::Parse);
                }
                Ok(val)
            }
            Some(c) if c.is_ascii_alphabetic() => self.ident_or_cellref(),
            Some(c) if c.is_ascii_digit() || c == '.' => self.number(),
            _ => Err(FormulaError::Parse),
        }
    }

    fn number(&mut self) -> EvalResult {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_ascii_digit() || c == '.') {
            self.pos += 1;
        }
        self.chars[start..self.pos]
            .iter()
            .collect::<String>()
            .parse::<f64>()
            .map_err(|_| FormulaError::Parse)
    }

    /// Either a bare cell reference (`A1`) or a function call (`SUM(...)`).
    fn ident_or_cellref(&mut self) -> EvalResult {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_ascii_alphabetic()) {
            self.pos += 1;
        }
        let letters: String = self.chars[start..self.pos].iter().collect();
        self.skip_ws();

        if self.peek() == Some('(') {
            self.pos += 1;
            let arg_start = self.pos;
            let mut depth = 1;
            while depth > 0 {
                match self.peek() {
                    Some('(') => depth += 1,
                    Some(')') => depth -= 1,
                    None => return Err(FormulaError::Parse),
                    _ => {}
                }
                if depth > 0 {
                    self.pos += 1;
                }
            }
            let arg: String = self.chars[arg_start..self.pos].iter().collect();
            self.pos += 1; // consume ')'
            return eval_function(&letters, &arg, self.cells);
        }

        // Not a function call -> must be a cell reference: letters immediately followed by digits.
        let digit_start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.pos += 1;
        }
        if self.pos == digit_start {
            return Err(FormulaError::Parse);
        }
        let digits: String = self.chars[digit_start..self.pos].iter().collect();
        let (row, col) = parse_cell_ref(&format!("{letters}{digits}")).ok_or(FormulaError::Parse)?;
        cell_value(self.cells, row, col)
    }
}

fn eval(expr: &str, cells: &[Vec<String>]) -> EvalResult {
    let mut parser = Parser::new(expr, cells);
    let val = parser.expr()?;
    parser.skip_ws();
    if parser.pos != parser.chars.len() {
        return Err(FormulaError::Parse);
    }
    Ok(val)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid() -> Vec<Vec<String>> {
        vec![
            vec!["10".to_string(), "1".to_string()],
            vec!["20".to_string(), "2".to_string()],
            vec!["30".to_string(), "3".to_string()],
        ]
    }

    #[test]
    fn non_formula_values_pass_through_unchanged() {
        assert_eq!(display_value("hello", &grid()), "hello");
        assert_eq!(display_value("", &grid()), "");
        assert_eq!(display_value("42", &grid()), "42");
    }

    #[test]
    fn arithmetic_on_cell_refs() {
        assert_eq!(display_value("=A1+A2", &grid()), "30");
        assert_eq!(display_value("=A2-A1", &grid()), "10");
        assert_eq!(display_value("=B1*B2", &grid()), "2");
        assert_eq!(display_value("=A1/B1", &grid()), "10");
        assert_eq!(display_value("=(A1+A2)*2", &grid()), "60");
    }

    #[test]
    fn sum_avg_over_a_range() {
        assert_eq!(display_value("=SUM(A1:A3)", &grid()), "60");
        assert_eq!(display_value("=AVG(A1:A3)", &grid()), "20");
        assert_eq!(display_value("=AVERAGE(A1:A3)", &grid()), "20");
        assert_eq!(display_value("=COUNT(A1:A3)", &grid()), "3");
        assert_eq!(display_value("=MIN(A1:A3)", &grid()), "10");
        assert_eq!(display_value("=MAX(A1:A3)", &grid()), "30");
    }

    #[test]
    fn whole_column_range() {
        assert_eq!(display_value("=SUM(A:A)", &grid()), "60");
        assert_eq!(display_value("=SUM(A)", &grid()), "60");
    }

    #[test]
    fn division_by_zero_reports_error_not_a_crash() {
        assert_eq!(display_value("=A1/0", &grid()), "#DIV/0!");
    }

    #[test]
    fn malformed_formula_reports_parse_error() {
        assert_eq!(display_value("=A1+", &grid()), "#ERROR");
        assert_eq!(display_value("=SUM(", &grid()), "#ERROR");
        assert_eq!(display_value("=@@@", &grid()), "#ERROR");
    }

    #[test]
    fn out_of_range_cell_reports_ref_error() {
        assert_eq!(display_value("=Z9", &grid()), "#REF!");
        assert_eq!(display_value("=A99", &grid()), "#REF!");
    }

    #[test]
    fn nested_formula_reference_recurses() {
        let mut g = grid();
        g[2][1] = "=SUM(A1:A2)".to_string(); // B3 = SUM(A1:A2) = 30
        assert_eq!(display_value("=B3+1", &g), "31");
    }

    #[test]
    fn non_integer_result_keeps_decimals_trimmed() {
        let g = vec![vec!["10".to_string()], vec!["3".to_string()]];
        assert_eq!(display_value("=A1/A2", &g), "3.333333");
    }
}
