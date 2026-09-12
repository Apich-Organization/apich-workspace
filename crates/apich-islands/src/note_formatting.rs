//! Pure text-manipulation logic behind the note editor's formatting toolbar (`note_editor.rs`).
//!
//! Before this, turning a selection into **bold** or a heading meant knowing and hand-typing the
//! literal markdown syntax; these functions do the same thing a rich-text editor's toolbar
//! buttons do, just by inserting/toggling real markdown around the selection instead of setting
//! HTML formatting directly (the editor stays a plain-text markdown source of truth, matching
//! everything else in this app -- git-diffable notes, no hidden document model).
//!
//! Kept separate from `note_editor.rs`'s `#[island]` so this logic is plain, wasm-independent
//! Rust that can be unit tested directly; the island only does the thin DOM glue (reading/setting
//! a `<textarea>`'s value and selection).
//!
//! All offsets in this module are **byte offsets into the UTF-8 `&str`**, not the UTF-16 code
//! unit offsets a browser's `HTMLTextAreaElement.selectionStart` actually uses -- the caller
//! (`note_editor.rs`, wasm-only) is responsible for converting at the boundary, so this module
//! works correctly for notes containing CJK text or emoji without ever touching JS string
//! semantics itself.

/// Wraps the selected text in `prefix`/`suffix` (bold: `**`/`**`, code: `` ` ``/`` ` ``, a link: `[`/`](url)`, etc).
///
/// If nothing is selected, inserts `placeholder` between the markers instead
/// and returns a selection covering just the placeholder, so the user can type straight over it --
/// the same "insert with the fill-in part pre-selected" behavior every markdown-toolbar editor
/// uses. Returns (`new_full_text`, `new_selection_start`, `new_selection_end`).
#[must_use]
pub fn wrap_selection(
    text: &str,
    start: usize,
    end: usize,
    prefix: &str,
    suffix: &str,
    placeholder: &str,
) -> (String, usize, usize) {
    let (start, end) = (start.min(text.len()), end.min(text.len()));
    let (start, end) = (start.min(end), start.max(end));
    let selected = &text[start..end];
    let inner = if selected.is_empty() {
        placeholder
    } else {
        selected
    };

    let mut out = String::with_capacity(
        text.len()
            .saturating_add(prefix.len())
            .saturating_add(suffix.len())
            .saturating_add(inner.len()),
    );
    out.push_str(&text[..start]);
    out.push_str(prefix);
    out.push_str(inner);
    out.push_str(suffix);
    out.push_str(&text[end..]);

    let new_start = start.saturating_add(prefix.len());
    let new_end = new_start.saturating_add(inner.len());
    (out, new_start, new_end)
}

/// Extends a (start, end) byte range to cover every full line it touches -- back to the start of
/// the line `start` is in, forward to the end of the line `end` is in (not including the
/// terminating newline itself).
fn expand_to_full_lines(
    text: &str,
    start: usize,
    end: usize,
) -> (usize, usize) {
    let (start, end) = (start.min(text.len()), end.min(text.len()));
    let (start, end) = (start.min(end), start.max(end));
    let line_start = text[..start].rfind('\n').map_or(0, |i| i.saturating_add(1));
    let line_end = text[end..]
        .find('\n')
        .map_or(text.len(), |i| end.saturating_add(i));
    (line_start, line_end)
}

/// Prepends `prefix` to every line touched by the selection (or just the current line, if the selection is a single caret).
///
/// Used for headings (`# `), quotes (`> `), bullet lists (`- `),
/// and task items (`- [ ] `). Toggles off instead if every touched line already starts with
/// exactly this prefix, so clicking the same button twice undoes it, the same as a rich-text
/// toolbar's "active" toggle state. Returns (`new_full_text`, `new_selection_start`, `new_selection_end`)
/// covering the whole affected block.
#[must_use]
pub fn toggle_line_prefix(
    text: &str,
    start: usize,
    end: usize,
    prefix: &str,
) -> (String, usize, usize) {
    let (line_start, line_end) = expand_to_full_lines(text, start, end);
    let block = &text[line_start..line_end];
    let lines: Vec<&str> = if block.is_empty() {
        vec![""]
    } else {
        block.split('\n').collect()
    };

    let all_have_prefix = lines.iter().all(|l| l.starts_with(prefix));
    let new_lines: Vec<String> = lines
        .iter()
        .map(|l| {
            if all_have_prefix {
                l[prefix.len()..].to_string()
            } else {
                format!("{prefix}{l}")
            }
        })
        .collect();
    let new_block = new_lines.join("\n");

    let mut out = String::with_capacity(text.len().saturating_add(new_block.len()));
    out.push_str(&text[..line_start]);
    out.push_str(&new_block);
    out.push_str(&text[line_end..]);

    (out, line_start, line_start.saturating_add(new_block.len()))
}

/// Same idea as `toggle_line_prefix`, but each line gets an incrementing `1. `, `2. `, ... marker instead of a constant one.
///
/// A numbered list only makes sense with per-line numbers, so it
/// can't reuse the constant-prefix toggle above. Always numbers from 1 (this note's own list
/// context, e.g. "continue numbering from an existing list above", isn't tracked -- scoped out
/// deliberately to keep this predictable rather than guessing at intent).
#[must_use]
pub fn numbered_list(
    text: &str,
    start: usize,
    end: usize,
) -> (String, usize, usize) {
    let (line_start, line_end) = expand_to_full_lines(text, start, end);
    let block = &text[line_start..line_end];
    let lines: Vec<&str> = if block.is_empty() {
        vec![""]
    } else {
        block.split('\n').collect()
    };

    // Toggle off if every line already looks like "<number>. " -- mirrors toggle_line_prefix's
    // own toggle behavior for the constant-prefix cases.
    let already_numbered = lines.iter().all(|l| {
        let Some(dot) = l.find(". ") else {
            return false;
        };
        l[..dot].chars().all(|c| c.is_ascii_digit()) && !l[..dot].is_empty()
    });

    let new_lines: Vec<String> = if already_numbered {
        lines
            .iter()
            .map(|l| {
                l.find(". ").map_or_else(
                    || l.to_string(),
                    |dot| {
                        l.get(dot.saturating_add(2)..)
                            .unwrap_or_default()
                            .to_string()
                    },
                )
            })
            .collect()
    } else {
        lines
            .iter()
            .enumerate()
            .map(|(i, l)| format!("{}. {}", i.saturating_add(1), l))
            .collect()
    };
    let new_block = new_lines.join("\n");

    let mut out = String::with_capacity(text.len().saturating_add(new_block.len()));
    out.push_str(&text[..line_start]);
    out.push_str(&new_block);
    out.push_str(&text[line_end..]);

    (out, line_start, line_start.saturating_add(new_block.len()))
}

/// Replaces the current selection (or inserts at the caret) with a fixed multi-line snippet.
///
/// Used for the table-skeleton button. Cursor lands at the end of the inserted snippet.
#[must_use]
pub fn insert_block(
    text: &str,
    start: usize,
    end: usize,
    snippet: &str,
) -> (String, usize, usize) {
    let (start, end) = (start.min(text.len()), end.min(text.len()));
    let (start, end) = (start.min(end), start.max(end));
    let mut out = String::with_capacity(text.len().saturating_add(snippet.len()));
    out.push_str(&text[..start]);
    out.push_str(snippet);
    out.push_str(&text[end..]);
    let new_pos = start.saturating_add(snippet.len());
    (out, new_pos, new_pos)
}

/// Default Markdown table template snippet.
pub const TABLE_SNIPPET: &str = "| Column A | Column B |\n| --- | --- |\n| value | value |\n";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_with_selection_bolds_it() {
        let (text, s, e) = wrap_selection("hello world", 6, 11, "**", "**", "bold");
        assert_eq!(text, "hello **world**");
        assert_eq!(&text[s..e], "world");
    }

    #[test]
    fn wrap_with_no_selection_inserts_placeholder_preselected() {
        let (text, s, e) = wrap_selection("hello ", 6, 6, "**", "**", "bold");
        assert_eq!(text, "hello **bold**");
        assert_eq!(&text[s..e], "bold");
    }

    #[test]
    fn wrap_handles_multibyte_selection_correctly() {
        // "笔记" (2 CJK chars, 3 bytes each in UTF-8) selected out of a longer note.
        let text = "我的笔记本";
        let start = "我的".len();
        let end = start + "笔记".len();
        let (out, s, e) = wrap_selection(text, start, end, "**", "**", "x");
        assert_eq!(out, "我的**笔记**本");
        assert_eq!(&out[s..e], "笔记");
    }

    #[test]
    fn heading_prefix_applies_to_current_line_only() {
        let text = "first\nsecond\nthird";
        let caret = "first\nsec".len(); // caret inside "second"
        let (out, s, e) = toggle_line_prefix(text, caret, caret, "## ");
        assert_eq!(out, "first\n## second\nthird");
        assert_eq!(&out[s..e], "## second");
    }

    #[test]
    fn heading_prefix_toggles_off_on_second_click() {
        let text = "## already a heading";
        let (out, _, _) = toggle_line_prefix(text, 0, 0, "## ");
        assert_eq!(out, "already a heading");
    }

    #[test]
    fn bullet_prefix_applies_to_every_selected_line() {
        let text = "one\ntwo\nthree";
        let (out, _, _) = toggle_line_prefix(text, 0, text.len(), "- ");
        assert_eq!(out, "- one\n- two\n- three");
    }

    #[test]
    fn numbered_list_numbers_each_selected_line() {
        let text = "alpha\nbeta\ngamma";
        let (out, _, _) = numbered_list(text, 0, text.len());
        assert_eq!(out, "1. alpha\n2. beta\n3. gamma");
    }

    #[test]
    fn numbered_list_toggles_off_when_already_numbered() {
        let text = "1. alpha\n2. beta";
        let (out, _, _) = numbered_list(text, 0, text.len());
        assert_eq!(out, "alpha\nbeta");
    }

    #[test]
    fn task_prefix_uses_checkbox_marker() {
        let text = "buy milk";
        let (out, _, _) = toggle_line_prefix(text, 0, text.len(), "- [ ] ");
        assert_eq!(out, "- [ ] buy milk");
    }

    #[test]
    fn insert_block_places_snippet_at_caret() {
        let (out, s, e) = insert_block("before\nafter", 7, 7, TABLE_SNIPPET);
        assert!(out.starts_with("before\n| Column A"));
        assert!(out.ends_with("\nafter"));
        assert_eq!(s, e);
        assert_eq!(s, 7 + TABLE_SNIPPET.len());
    }

    #[test]
    fn wiki_link_wrap_uses_double_brackets() {
        let (text, s, e) = wrap_selection("see ", 4, 4, "[[", "]]", "Note Name");
        assert_eq!(text, "see [[Note Name]]");
        assert_eq!(&text[s..e], "Note Name");
    }
}
