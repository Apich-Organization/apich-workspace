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
//! - Arithmetic over numbers and cell references: `=A1+B1`, `=(A1-B2)*3`, `=A1/B1`, `=A1^2`
//! - Column-range aggregate functions: `=SUM(A1:A5)`, `=AVG(A:A)`, `=COUNT(B1:B10)`,
//!   `=MIN(A1:A5)`, `=MAX(A1:A5)` (also accepts `AVERAGE` as an alias for `AVG`)
//! - Scalar math functions: `=ROUND(A1, 2)` (digits optional, defaults to 0), `=ABS(A1)`,
//!   `=SQRT(A1)`, `=POWER(A1, 3)` (also `POW`), `=MOD(A1, B1)`
//! - Comparisons (evaluate to `1` for true / `0` for false, usable anywhere a number is):
//!   `=A1>B1`, `=A1<=10`, `=A1=B1`, `=A1<>B1`
//! - Conditional: `=IF(A1>10, B1, C1)` -- condition is any comparison or number (nonzero = true)
//!
//! Deliberately NOT supported (kept small and predictable rather than a full spreadsheet
//! language): string functions, cross-sheet refs, relative-fill-on-copy semantics. A formula
//! that references a column beyond `Z` or a row past the grid's current size is a parse/eval
//! error (`#REF!`), same as a malformed expression (`#ERROR`).

/// Evaluates a cell's raw stored value.
///
/// If it starts with `=`, parses and evaluates it as a
/// formula against the current grid (`cells[row][col]`, 0-indexed, `columns` giving the count
/// of columns so a bare column-letter range like `A:A` knows how far down to read); anything
/// else (including an empty string) passes through unchanged, matching every existing cell that
/// predates this feature.
#[must_use]
pub fn display_value(
    raw: &str,
    cells: &[Vec<String>],
) -> String {
    let Some(expr) = raw.strip_prefix('=') else {
        return raw.to_string();
    };
    if expr.trim().is_empty() {
        return raw.to_string();
    }
    match eval(expr, cells) {
        | Ok(n) => format_number(n),
        | Err(e) => e.to_string(),
    }
}

fn format_number(n: f64) -> String {
    if n == 0.0 {
        return "0".to_string();
    }
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{n:.0}")
    } else {
        let s = format!("{n:.6}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

#[derive(Debug, PartialEq, Eq)]
enum FormulaError {
    Parse,
    Ref,
    DivZero,
}

impl std::fmt::Display for FormulaError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            | Self::Parse => write!(f, "#ERROR"),
            | Self::Ref => write!(f, "#REF!"),
            | Self::DivZero => write!(f, "#DIV/0!"),
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
        Some(usize::from((c as u8).saturating_sub(b'A')))
    } else {
        None
    }
}

fn cell_value(
    cells: &[Vec<String>],
    row: usize,
    col: usize,
) -> EvalResult {
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

/// Parses `A1` / `a12` into (`row_index`, `col_index`), both 0-based.
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
    Some((row_num.saturating_sub(1), col))
}

/// Parses a function argument that's a range: `A1:B1`, `A1:A10`, or a bare column like `A` /
/// `A:A` meaning "the whole column, every row currently in the grid".
fn resolve_range(
    arg: &str,
    cells: &[Vec<String>],
) -> Result<Vec<f64>, FormulaError> {
    let arg = arg.trim();
    let n_rows = cells.len();

    let (start, end) = match arg.split_once(':') {
        | Some((a, b)) => (a.trim(), b.trim()),
        | None => (arg, arg),
    };

    // Bare column letter(s) on both ends (e.g. "A" or "A:A") -> whole column.
    if let (Some(c1), Some(c2)) = (col_index(start), col_index(end)) {
        if !start.chars().next().unwrap_or(' ').is_ascii_digit()
            && start.chars().all(char::is_alphabetic)
        {
            let col = c1;
            if c1 != c2 {
                return Err(FormulaError::Ref);
            }
            let mut out = Vec::with_capacity(n_rows);
            for r in 0..n_rows {
                let raw = cells
                    .get(r)
                    .and_then(|row| row.get(col))
                    .map_or("", String::as_str);
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
    let (lo, hi) = if r1 <= r2 {
        (r1, r2)
    } else {
        (r2, r1)
    };
    let mut out = Vec::with_capacity(hi.saturating_sub(lo).saturating_add(1));
    for r in lo..=hi {
        out.push(cell_value(cells, r, c1)?);
    }
    Ok(out)
}

/// Splits a function's raw argument string on top-level commas (commas nested inside another
/// function call's parens, e.g. `IF(A1>0, SUM(B1:B2), 0)`, are not split points).
fn split_top_level_args(arg: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (i, c) in arg.char_indices() {
        match c {
            | '(' => depth = depth.saturating_add(1),
            | ')' => depth = depth.saturating_sub(1),
            | ',' if depth == 0 => {
                let segment = arg.get(start..i).unwrap_or("").trim();
                parts.push(segment);
                start = i.saturating_add(1);
            },
            | _ => {},
        }
    }
    let last_segment = arg.get(start..).unwrap_or("").trim();
    parts.push(last_segment);
    parts
}

fn eval_function(
    name: &str,
    arg: &str,
    cells: &[Vec<String>],
) -> EvalResult {
    let upper = name.to_ascii_uppercase();
    match upper.as_str() {
        | "SUM" | "AVG" | "AVERAGE" | "COUNT" | "MIN" | "MAX" => {
            let values = resolve_range(arg, cells)?;
            match upper.as_str() {
                | "SUM" => Ok(values.iter().sum()),
                | "AVG" | "AVERAGE" => {
                    if values.is_empty() {
                        Err(FormulaError::DivZero)
                    } else {
                        let count = u32::try_from(values.len()).map_or(1.0, f64::from);
                        Ok(values.iter().sum::<f64>() / count)
                    }
                },
                | "COUNT" => Ok(u32::try_from(values.len()).map_or(0.0, f64::from)),
                | "MIN" => {
                    values
                        .iter()
                        .copied()
                        .fold(None, |acc, v| Some(acc.map_or(v, |a: f64| a.min(v))))
                        .ok_or(FormulaError::DivZero)
                },
                | "MAX" => {
                    values
                        .iter()
                        .copied()
                        .fold(None, |acc, v| Some(acc.map_or(v, |a: f64| a.max(v))))
                        .ok_or(FormulaError::DivZero)
                },
                | _ => unreachable!(),
            }
        },
        | "ABS" => Ok(eval(arg, cells)?.abs()),
        | "SQRT" => {
            let v = eval(arg, cells)?;
            if v < 0.0 {
                Err(FormulaError::Parse)
            } else {
                Ok(v.sqrt())
            }
        },
        | "ROUND" => {
            let parts = split_top_level_args(arg);
            let v = eval(parts.first().copied().unwrap_or(""), cells)?;
            let digits = match parts.get(1) {
                | Some(d) => {
                    format!("{:.0}", eval(d, cells)?)
                        .parse::<i32>()
                        .unwrap_or(0)
                },
                | None => 0,
            };
            let factor = 10f64.powi(digits);
            Ok((v * factor).round() / factor)
        },
        | "POWER" | "POW" => {
            let parts = split_top_level_args(arg);
            let [base_s, exp_s] = match parts.as_slice() {
                | [a, b] => [*a, *b],
                | _ => return Err(FormulaError::Parse),
            };
            let base = eval(base_s, cells)?;
            let exp = eval(exp_s, cells)?;
            Ok(base.powf(exp))
        },
        | "MOD" => {
            let parts = split_top_level_args(arg);
            let [a_s, b_s] = match parts.as_slice() {
                | [a, b] => [*a, *b],
                | _ => return Err(FormulaError::Parse),
            };
            let a = eval(a_s, cells)?;
            let b = eval(b_s, cells)?;
            if b == 0.0 {
                Err(FormulaError::DivZero)
            } else {
                Ok(a % b)
            }
        },
        | "IF" => {
            let parts = split_top_level_args(arg);
            let [cond_s, then_s, else_s] = match parts.as_slice() {
                | [a, b, c] => [*a, *b, *c],
                | _ => return Err(FormulaError::Parse),
            };
            let cond = eval(cond_s, cells)?;
            if cond == 0.0 {
                eval(else_s, cells)
            } else {
                eval(then_s, cells)
            }
        },
        | _ => Err(FormulaError::Parse),
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
    fn new(
        s: &'a str,
        cells: &'a [Vec<String>],
    ) -> Self {
        Self {
            chars: s.chars().collect(),
            pos: 0,
            cells,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_whitespace()) {
            self.pos = self.pos.saturating_add(1);
        }
    }

    /// Top-level entry point: a single comparison (or, if there's no comparison operator, just
    /// falls through to plain arithmetic).
    fn comparison(&mut self) -> EvalResult {
        let lhs = self.expr()?;
        self.skip_ws();
        let first = self.peek();
        let second = self.chars.get(self.pos.saturating_add(1)).copied();
        let op = if matches!(first, Some('<' | '>')) && second == Some('=') {
            let end_pos = self.pos.saturating_add(2);
            let op: String = self
                .chars
                .get(self.pos..end_pos)
                .unwrap_or(&[])
                .iter()
                .collect();
            self.pos = end_pos;
            Some(op)
        } else if first == Some('<') && second == Some('>') {
            self.pos = self.pos.saturating_add(2);
            Some("<>".to_string())
        } else if let Some(c) = first.filter(|&c| c == '<' || c == '>' || c == '=') {
            let op = c.to_string();
            self.pos = self.pos.saturating_add(1);
            Some(op)
        } else {
            None
        };
        let Some(op) = op else { return Ok(lhs) };
        let rhs = self.expr()?;
        let truth = match op.as_str() {
            | "=" => (lhs - rhs).abs() < f64::EPSILON,
            | "<>" => (lhs - rhs).abs() >= f64::EPSILON,
            | "<=" => lhs <= rhs,
            | ">=" => lhs >= rhs,
            | "<" => lhs < rhs,
            | ">" => lhs > rhs,
            | _ => unreachable!(),
        };
        Ok(if truth { 1.0 } else { 0.0 })
    }

    fn expr(&mut self) -> EvalResult {
        let mut val = self.term()?;
        loop {
            self.skip_ws();
            match self.peek() {
                | Some('+') => {
                    self.pos = self.pos.saturating_add(1);
                    val += self.term()?;
                },
                | Some('-') => {
                    self.pos = self.pos.saturating_add(1);
                    val -= self.term()?;
                },
                | _ => break,
            }
        }
        Ok(val)
    }

    fn term(&mut self) -> EvalResult {
        let mut val = self.power()?;
        loop {
            self.skip_ws();
            match self.peek() {
                | Some('*') => {
                    self.pos = self.pos.saturating_add(1);
                    val *= self.power()?;
                },
                | Some('/') => {
                    self.pos = self.pos.saturating_add(1);
                    let rhs = self.power()?;
                    if rhs == 0.0 {
                        return Err(FormulaError::DivZero);
                    }
                    val /= rhs;
                },
                | _ => break,
            }
        }
        Ok(val)
    }

    /// Right-associative exponentiation: `2^3^2` == `2^(3^2)`, matching standard math/spreadsheet
    /// convention.
    fn power(&mut self) -> EvalResult {
        let base = self.factor()?;
        self.skip_ws();
        if self.peek() == Some('^') {
            self.pos = self.pos.saturating_add(1);
            let exp = self.power()?;
            return Ok(base.powf(exp));
        }
        Ok(base)
    }

    fn factor(&mut self) -> EvalResult {
        self.skip_ws();
        match self.peek() {
            | Some('-') => {
                self.pos = self.pos.saturating_add(1);
                Ok(-self.factor()?)
            },
            | Some('(') => {
                self.pos = self.pos.saturating_add(1);
                let val = self.comparison()?;
                self.skip_ws();
                if self.peek() == Some(')') {
                    self.pos = self.pos.saturating_add(1);
                } else {
                    return Err(FormulaError::Parse);
                }
                Ok(val)
            },
            | Some(c) if c.is_ascii_alphabetic() => self.ident_or_cellref(),
            | Some(c) if c.is_ascii_digit() || c == '.' => self.number(),
            | _ => Err(FormulaError::Parse),
        }
    }

    fn number(&mut self) -> EvalResult {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_ascii_digit() || c == '.') {
            self.pos = self.pos.saturating_add(1);
        }
        self.chars
            .get(start..self.pos)
            .unwrap_or(&[])
            .iter()
            .collect::<String>()
            .parse::<f64>()
            .map_err(|_| FormulaError::Parse)
    }

    /// Either a bare cell reference (`A1`) or a function call (`SUM(...)`).
    fn ident_or_cellref(&mut self) -> EvalResult {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_ascii_alphabetic()) {
            self.pos = self.pos.saturating_add(1);
        }
        let letters: String = self
            .chars
            .get(start..self.pos)
            .unwrap_or(&[])
            .iter()
            .collect();
        self.skip_ws();

        if self.peek() == Some('(') {
            self.pos = self.pos.saturating_add(1);
            let arg_start = self.pos;
            let mut depth = 1usize;
            while depth > 0 {
                match self.peek() {
                    | Some('(') => depth = depth.saturating_add(1),
                    | Some(')') => depth = depth.saturating_sub(1),
                    | None => return Err(FormulaError::Parse),
                    | _ => {},
                }
                if depth > 0 {
                    self.pos = self.pos.saturating_add(1);
                }
            }
            let arg: String = self
                .chars
                .get(arg_start..self.pos)
                .unwrap_or(&[])
                .iter()
                .collect();
            self.pos = self.pos.saturating_add(1); // consume ')'
            return eval_function(&letters, &arg, self.cells);
        }

        // Not a function call -> must be a cell reference: letters immediately followed by digits.
        let digit_start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.pos = self.pos.saturating_add(1);
        }
        if self.pos == digit_start {
            return Err(FormulaError::Parse);
        }
        let digits: String = self
            .chars
            .get(digit_start..self.pos)
            .unwrap_or(&[])
            .iter()
            .collect();
        let (row, col) =
            parse_cell_ref(&format!("{letters}{digits}")).ok_or(FormulaError::Parse)?;
        cell_value(self.cells, row, col)
    }
}

fn eval(
    expr: &str,
    cells: &[Vec<String>],
) -> EvalResult {
    let mut parser = Parser::new(expr, cells);
    let val = parser.comparison()?;
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
        if let Some(row) = g.get_mut(2) {
            if let Some(cell) = row.get_mut(1) {
                *cell = "=SUM(A1:A2)".to_string(); // B3 = SUM(A1:A2) = 30
            }
        }
        assert_eq!(display_value("=B3+1", &g), "31");
    }

    #[test]
    fn non_integer_result_keeps_decimals_trimmed() {
        let g = vec![vec!["10".to_string()], vec!["3".to_string()]];
        assert_eq!(display_value("=A1/A2", &g), "3.333333");
    }

    #[test]
    fn exponentiation() {
        assert_eq!(display_value("=A1^2", &grid()), "100");
        assert_eq!(display_value("=2^3^2", &grid()), "512"); // right-assoc: 2^(3^2) = 2^9
    }

    #[test]
    fn comparisons_yield_one_or_zero() {
        assert_eq!(display_value("=A1>A2", &grid()), "0");
        assert_eq!(display_value("=A2>A1", &grid()), "1");
        assert_eq!(display_value("=A1=10", &grid()), "1");
        assert_eq!(display_value("=A1<>A2", &grid()), "1");
        assert_eq!(display_value("=A1<=10", &grid()), "1");
        assert_eq!(display_value("=A1>=20", &grid()), "0");
    }

    #[test]
    fn scalar_math_functions() {
        assert_eq!(display_value("=ABS(A1-A2)", &grid()), "10");
        assert_eq!(display_value("=SQRT(A1)", &grid()), "3.162278");
        assert_eq!(display_value("=ROUND(A1/A2, 2)", &grid()), "0.5");
        assert_eq!(display_value("=ROUND(3.7)", &grid()), "4");
        assert_eq!(display_value("=POWER(2,10)", &grid()), "1024");
        assert_eq!(display_value("=MOD(A2,A1)", &grid()), "0");
        assert_eq!(display_value("=MOD(7,3)", &grid()), "1");
    }

    #[test]
    fn if_function_branches_on_condition() {
        assert_eq!(display_value("=IF(A1>A2, A1, A2)", &grid()), "20");
        assert_eq!(display_value("=IF(A1<A2, A1, A2)", &grid()), "10");
        assert_eq!(display_value("=IF(1, 100, 200)", &grid()), "100");
        assert_eq!(display_value("=IF(0, 100, 200)", &grid()), "200");
    }
}
