//! Lightweight, zero-dependency syntax highlighter for the web file preview modal.

pub fn highlight_content(
    content: &str,
    file_path: &str,
) -> Vec<(usize, String)> {
    let ext = file_path.rsplit('.').next().unwrap_or("").to_lowercase();

    content
        .lines()
        .enumerate()
        .map(|(line_idx, line)| {
            let line_num = line_idx + 1;
            let highlighted = match ext.as_str() {
                | "md" | "markdown" => highlight_markdown_line(line),
                | "rs" => highlight_code_line(line, &RUST_KEYWORDS, &RUST_TYPES, "//"),
                | "typ" => highlight_code_line(line, &TYPST_KEYWORDS, &TYPST_TYPES, "//"),
                | "toml" => highlight_toml_line(line),
                | "json" => highlight_json_line(line),
                | "sql" => highlight_code_line(line, &SQL_KEYWORDS, &SQL_TYPES, "--"),
                | "py" => highlight_code_line(line, &PYTHON_KEYWORDS, &PYTHON_TYPES, "#"),
                | "csv" => highlight_csv_line(line, line_num == 1),
                | _ => escape_html(line),
            };
            (line_num, highlighted)
        })
        .collect()
}

fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            | '&' => out.push_str("&amp;"),
            | '<' => out.push_str("&lt;"),
            | '>' => out.push_str("&gt;"),
            | '"' => out.push_str("&quot;"),
            | '\'' => out.push_str("&#39;"),
            | _ => out.push(c),
        }
    }
    out
}

const RUST_KEYWORDS: [&str; 32] = [
    "fn", "let", "mut", "pub", "struct", "enum", "impl", "match", "if", "else", "return", "use",
    "mod", "const", "type", "async", "await", "self", "Self", "where", "for", "in", "loop",
    "while", "break", "continue", "trait", "unsafe", "move", "ref", "static", "as",
];

const RUST_TYPES: [&str; 24] = [
    "String", "Vec", "Option", "Result", "bool", "u8", "u16", "u32", "u64", "usize", "i8", "i16",
    "i32", "i64", "isize", "f32", "f64", "char", "str", "Some", "None", "Ok", "Err", "PathBuf",
];

const TYPST_KEYWORDS: [&str; 20] = [
    "#slide", "#step", "#v", "#h", "#grid", "#cols", "#callout", "#text", "#table", "#link",
    "#image", "#rect", "let", "set", "show", "import", "include", "false", "true", "none",
];

const TYPST_TYPES: [&str; 6] = ["title", "transition", "effect", "order", "fill", "stroke"];

const SQL_KEYWORDS: [&str; 24] = [
    "SELECT", "FROM", "WHERE", "INSERT", "INTO", "UPDATE", "DELETE", "JOIN", "LEFT", "RIGHT",
    "INNER", "ORDER", "BY", "GROUP", "HAVING", "LIMIT", "AND", "OR", "NOT", "IN", "CREATE",
    "TABLE", "PRIMARY", "KEY",
];

const SQL_TYPES: [&str; 6] = ["INTEGER", "REAL", "TEXT", "BLOB", "NUMERIC", "BOOLEAN"];

const PYTHON_KEYWORDS: [&str; 22] = [
    "def", "class", "import", "from", "return", "if", "elif", "else", "for", "while", "in", "is",
    "not", "and", "or", "try", "except", "finally", "with", "as", "yield", "pass",
];

const PYTHON_TYPES: [&str; 6] = ["True", "False", "None", "self", "int", "str"];

fn highlight_markdown_line(line: &str) -> String {
    let trimmed = line.trim_start();
    let indent = &line[..line.len() - trimmed.len()];
    let esc_indent = escape_html(indent);

    // Headers
    if trimmed.starts_with("#### ") {
        return format!(
            "{esc_indent}<span class=\"hl-h4\">{}</span>",
            escape_html(trimmed)
        );
    }
    if trimmed.starts_with("### ") {
        return format!(
            "{esc_indent}<span class=\"hl-h3\">{}</span>",
            escape_html(trimmed)
        );
    }
    if trimmed.starts_with("## ") {
        return format!(
            "{esc_indent}<span class=\"hl-h2\">{}</span>",
            escape_html(trimmed)
        );
    }
    if trimmed.starts_with("# ") {
        return format!(
            "{esc_indent}<span class=\"hl-h1\">{}</span>",
            escape_html(trimmed)
        );
    }

    // Blockquote
    if trimmed.starts_with('>') {
        return format!(
            "{esc_indent}<span class=\"hl-quote\">{}</span>",
            escape_html(trimmed)
        );
    }

    // Code block fence
    if trimmed.starts_with("```") {
        return format!(
            "{esc_indent}<span class=\"hl-fence\">{}</span>",
            escape_html(trimmed)
        );
    }

    // List item
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
        let bullet = &trimmed[..2];
        let rest = &trimmed[2..];
        return format!(
            "{esc_indent}<span class=\"hl-list\">{bullet}</span>{}",
            highlight_inline_md(rest)
        );
    }

    format!("{esc_indent}{}", highlight_inline_md(trimmed))
}

fn highlight_inline_md(text: &str) -> String {
    let escaped = escape_html(text);
    // Highlight inline code `...`
    let mut result = String::new();
    let mut in_code = false;
    let mut buf = String::new();

    for c in escaped.chars() {
        if c == '`' {
            if in_code {
                result.push_str("<span class=\"hl-code\">`");
                result.push_str(&buf);
                result.push_str("`</span>");
                buf.clear();
                in_code = false;
            } else {
                result.push_str(&buf);
                buf.clear();
                in_code = true;
            }
        } else {
            buf.push(c);
        }
    }
    result.push_str(&buf);
    result
}

fn highlight_code_line(
    line: &str,
    keywords: &[&str],
    types: &[&str],
    comment_prefix: &str,
) -> String {
    let esc = escape_html(line);
    // Comment check
    if let Some(pos) = esc.find(comment_prefix) {
        let before = &esc[..pos];
        let comm = &esc[pos..];
        return format!("{before}<span class=\"hl-comm\">{comm}</span>");
    }

    let mut out = String::new();
    let words = esc.split_inclusive(|c: char| !c.is_alphanumeric() && c != '_' && c != '#');

    for word in words {
        let token = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != '#');
        if keywords.iter().any(|&k| k.eq_ignore_ascii_case(token)) {
            let replaced = word.replace(token, &format!("<span class=\"hl-kw\">{token}</span>"));
            out.push_str(&replaced);
        } else if types.contains(&token) {
            let replaced = word.replace(token, &format!("<span class=\"hl-type\">{token}</span>"));
            out.push_str(&replaced);
        } else if token.chars().all(|c| c.is_ascii_digit() || c == '.') && !token.is_empty() {
            let replaced = word.replace(token, &format!("<span class=\"hl-num\">{token}</span>"));
            out.push_str(&replaced);
        } else {
            out.push_str(word);
        }
    }

    out
}

fn highlight_toml_line(line: &str) -> String {
    let trimmed = line.trim();
    if trimmed.starts_with('#') {
        return format!("<span class=\"hl-comm\">{}</span>", escape_html(line));
    }
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        return format!("<span class=\"hl-h2\">{}</span>", escape_html(line));
    }
    if let Some(eq_pos) = line.find('=') {
        let key = &line[..eq_pos];
        let val = &line[eq_pos + 1..];
        return format!(
            "<span class=\"hl-kw\">{}</span>=<span class=\"hl-str\">{}</span>",
            escape_html(key),
            escape_html(val)
        );
    }
    escape_html(line)
}

fn highlight_json_line(line: &str) -> String {
    let esc = escape_html(line);
    if let Some(colon) = esc.find(':') {
        let key = &esc[..colon];
        let val = &esc[colon + 1..];
        format!("<span class=\"hl-kw\">{key}</span>:<span class=\"hl-str\">{val}</span>")
    } else {
        esc
    }
}

fn highlight_csv_line(
    line: &str,
    is_header: bool,
) -> String {
    let esc = escape_html(line);
    if is_header {
        format!("<span class=\"hl-kw\">{esc}</span>")
    } else {
        format!("<span class=\"hl-str\">{esc}</span>")
    }
}
