use crate::error::WebError;
use crate::error::WebResult;
use regex::Regex;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Write as _;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarkdownTask {
    pub id: String, // hash or file:line
    pub file_path: String,
    pub line_number: usize,
    pub raw_line: String,
    pub title: String,
    pub completed: bool,
    pub status: String, // "todo", "in_progress", "done"
    pub tags: Vec<String>,
    pub due_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KanbanColumn {
    pub id: String,
    pub title: String,
    pub tasks: Vec<MarkdownTask>,
    /// Mirrors `KanbanColumnDef::is_done` -- carried through onto the built board so a renderer
    /// (or the click-to-cycle-status handler) doesn't need to separately re-parse
    /// `project.settings` just to know which column(s) count as "done". Always `false` for the
    /// synthetic Unsorted catch-all column.
    pub is_done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KanbanBoard {
    pub columns: Vec<KanbanColumn>,
    pub total_tasks: usize,
    pub completed_tasks: usize,
}

/// A team's own configurable Kanban column layout -- persisted as
/// `project.settings["kanban_columns"]` (a plain JSON array), read/written via
/// `parse_kanban_columns`/`serialize_kanban_columns`. Different teams reasonably want different
/// workflows (a simple Todo/Doing/Done vs. an explicit Backlog/Todo/In Review/Blocked/Done), so
/// this is per-project data, not a hardcoded shape, with the original 3-column layout kept only as
/// the *default* a project starts from until customized.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KanbanColumnDef {
    pub id: String,
    pub title: String,
    /// Whether landing a task in this column means "done" for the checkbox/progress-bar
    /// semantics that predate custom columns (`MarkdownTask::completed`, the note view's plain
    /// checkbox, the board's completion percentage). Exactly one built-in column ("done") had
    /// this meaning implicitly before; a custom board names its own equivalent explicitly.
    pub is_done: bool,
}

/// The id reserved for tasks whose resolved status doesn't match any of the project's currently
/// configured columns -- e.g. a column was deleted after tasks were already placed in it via a
/// `#status:<id>` tag. Kept visible (rather than silently hidden) so nothing is lost; the user can
/// re-tag or re-add a matching column.
pub const KANBAN_UNSORTED_COLUMN_ID: &str = "_unsorted";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiLink {
    pub source_path: String,
    pub target_title: String,
    pub display_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub file_path: Option<String>,
    pub exists: bool,
    pub backlink_count: usize,
    pub task_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub id: String,
    pub date: String, // YYYY-MM-DD
    pub title: String,
    pub source_file: String,
    pub line_number: Option<usize>,
    pub is_task: bool,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteSummary {
    pub relative_path: String,
    pub title: String,
    pub modified_rfc3339: Option<String>,
    pub outgoing_links: Vec<String>,
    pub incoming_backlinks: Vec<String>,
    pub task_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UnifiedNoteMeta {
    pub title: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub tags: Vec<String>,
    pub author: Option<String>,
    pub whiteboard: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteHeading {
    pub level: usize,
    pub text: String,
    pub line: usize,
}

pub struct KnowledgeSyncService;

impl KnowledgeSyncService {
    /// The original hardcoded 3-column layout, now just the *seed* a project starts from --
    /// `todo_title`/`in_progress_title`/`done_title` let a caller localize these (the board has
    /// no stored columns yet, so there's nothing else to translate) without baking a fixed
    /// language into the data itself the way a stored custom title inevitably does once a team
    /// renames or adds columns.
    pub fn default_kanban_columns(
        todo_title: &str,
        in_progress_title: &str,
        done_title: &str,
    ) -> Vec<KanbanColumnDef> {
        vec![
            KanbanColumnDef {
                id: "todo".to_string(),
                title: todo_title.to_string(),
                is_done: false,
            },
            KanbanColumnDef {
                id: "in_progress".to_string(),
                title: in_progress_title.to_string(),
                is_done: false,
            },
            KanbanColumnDef {
                id: "done".to_string(),
                title: done_title.to_string(),
                is_done: true,
            },
        ]
    }

    /// Reads `project.settings["kanban_columns"]`, falling back to `defaults` the first time a
    /// project's board is ever viewed (before anyone has customized it, there's nothing to read).
    /// A malformed/foreign-shaped value (hand-edited settings JSON, a future schema change) falls
    /// back the same way rather than erroring the whole board out.
    pub fn parse_kanban_columns(
        settings: &serde_json::Value,
        defaults: Vec<KanbanColumnDef>,
    ) -> Vec<KanbanColumnDef> {
        settings
            .get("kanban_columns")
            .and_then(|v| serde_json::from_value::<Vec<KanbanColumnDef>>(v.clone()).ok())
            .filter(|cols| !cols.is_empty())
            .unwrap_or(defaults)
    }

    pub fn serialize_kanban_columns(columns: &[KanbanColumnDef]) -> serde_json::Value {
        serde_json::to_value(columns).unwrap_or_else(|_| serde_json::json!([]))
    }

    /// Turns a user-typed column title into a stable id (`"In Review"` -> `"in-review"`),
    /// disambiguated against `existing` ids by appending `-2`, `-3`, ... on collision -- the same
    /// approach file/slug generation already uses elsewhere in this app.
    pub fn slugify_kanban_column_id(
        title: &str,
        existing: &[KanbanColumnDef],
    ) -> String {
        let mut slug: String = title
            .trim()
            .to_lowercase()
            .chars()
            .map(|c| {
                if c.is_alphanumeric() {
                    c
                } else {
                    '-'
                }
            })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-");
        if slug.is_empty() {
            slug = "column".to_string();
        }
        if slug == KANBAN_UNSORTED_COLUMN_ID {
            slug.push_str("-col");
        }
        let base = slug.clone();
        let mut n: usize = 2;
        while existing.iter().any(|c| c.id == slug) {
            slug = format!("{base}-{n}");
            n = n.saturating_add(1);
        }
        slug
    }

    /// Turn a free-typed page title into a safe `.anote` filename base -- same approach as
    /// `slugify_kanban_column_id`, just without a caller-supplied uniqueness list (page filenames
    /// are de-duplicated by `create_note_page_action` trying the plain slug first, then
    /// `-2`, `-3`, ... on conflict).
    pub fn slugify_page_title(title: &str) -> String {
        let slug: String = title
            .trim()
            .to_lowercase()
            .chars()
            .map(|c| {
                if c.is_alphanumeric() {
                    c
                } else {
                    '-'
                }
            })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-");
        if slug.is_empty() {
            "untitled".to_string()
        } else {
            slug
        }
    }
}

fn walk_markdown_dir(
    dir: &Path,
    acc: &mut Vec<PathBuf>,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();

        if path.is_dir() {
            if file_name.starts_with('.') || file_name == "target" || file_name == "node_modules" {
                continue;
            }
            walk_markdown_dir(&path, acc);
        } else if path.is_file() {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext.eq_ignore_ascii_case("md")
                || ext.eq_ignore_ascii_case("anote")
                || ext.eq_ignore_ascii_case("note")
            {
                acc.push(path);
            }
        }
    }
}

impl KnowledgeSyncService {
    /// Discover all Markdown and Unified Note (.anote, .note, .md) files in a workspace directory
    pub fn discover_markdown_files<P: AsRef<Path>>(project_dir: P) -> Vec<PathBuf> {
        let mut md_files = Vec::new();
        let project_dir = project_dir.as_ref();

        if !project_dir.exists() {
            return md_files;
        }

        walk_markdown_dir(project_dir, &mut md_files);
        md_files.sort();
        md_files
    }

    /// Extract all actionable tasks from Markdown files in the project
    pub fn extract_all_tasks<P: AsRef<Path>>(project_dir: P) -> WebResult<Vec<MarkdownTask>> {
        let root = project_dir.as_ref();
        let files = Self::discover_markdown_files(root);
        let mut tasks = Vec::new();

        // Pattern for markdown tasks: - [ ] or - [x] or - [/]
        // Group 1: check character
        // Group 2: remaining line content
        let task_regex = Regex::new(r"^\s*[-*]\s+\[([ xX/])\]\s+(.*)$")
            .map_err(|e| WebError::Internal(e.to_string()))?;
        let tag_regex =
            Regex::new(r"#([a-zA-Z0-9_\-]+)").map_err(|e| WebError::Internal(e.to_string()))?;
        let date_regex =
            Regex::new(r"@(\d{4}-\d{2}-\d{2})").map_err(|e| WebError::Internal(e.to_string()))?;
        // Overrides the checkbox-derived 3-state status for custom Kanban columns (a
        // `[ ]`/`[/]`/`[x]` checkbox alone can't distinguish more than 3 board positions) --
        // written/removed by `update_task_status`, see its own comment. Matched and stripped
        // before the generic `tag_regex` pass so it never also shows up as a literal
        // "status:<id>" tag pill.
        let status_tag_regex = Regex::new(r"#status:([a-zA-Z0-9_\-]+)")
            .map_err(|e| WebError::Internal(e.to_string()))?;

        for file in files {
            let rel_path = file
                .strip_prefix(root)
                .unwrap_or(&file)
                .to_string_lossy()
                .replace('\\', "/");

            let Ok(content) = std::fs::read_to_string(&file) else {
                continue;
            };

            for (idx, line) in content.lines().enumerate() {
                let line_number = idx.saturating_add(1);
                if let Some(caps) = task_regex.captures(line) {
                    let mark = caps.get(1).map_or(" ", |m| m.as_str());
                    let raw_body = caps.get(2).map_or("", |m| m.as_str()).trim();

                    let completed = mark == "x" || mark == "X";
                    let checkbox_status = if completed {
                        "done"
                    } else if mark == "/" {
                        "in_progress"
                    } else {
                        "todo"
                    };
                    // A `#status:<id>` tag (written when a task is placed in a custom Kanban
                    // column that isn't one of the 3 built-in ones) overrides the checkbox-derived
                    // status for board placement; `completed`/the plain checkbox state above are
                    // untouched by it, so progress tracking and the note view's inline checkbox
                    // keep meaning exactly what they always did regardless of custom columns.
                    let status_override = status_tag_regex
                        .captures(raw_body)
                        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()));
                    let status = status_override
                        .clone()
                        .unwrap_or_else(|| checkbox_status.to_string());
                    let without_status_tag = status_tag_regex.replace_all(raw_body, "").to_string();

                    // Extract tags
                    let tags: Vec<String> = tag_regex
                        .find_iter(&without_status_tag)
                        .map(|m| m.as_str().trim_start_matches('#').to_string())
                        .collect();

                    // Extract due date
                    let due_date = date_regex
                        .captures(&without_status_tag)
                        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()));

                    // Clean title: strip #tags, #status:, and @date for display
                    let mut clean_title =
                        tag_regex.replace_all(&without_status_tag, "").to_string();
                    clean_title = date_regex.replace_all(&clean_title, "").to_string();
                    let title = clean_title.trim().to_string();

                    let id = format!("{rel_path}:{line_number}");

                    tasks.push(MarkdownTask {
                        id,
                        file_path: rel_path.clone(),
                        line_number,
                        raw_line: line.to_string(),
                        title: if title.is_empty() {
                            raw_body.to_string()
                        } else {
                            title
                        },
                        completed,
                        status,
                        tags,
                        due_date,
                    });
                }
            }
        }

        Ok(tasks)
    }

    /// Build Kanban Board representation directly from Markdown tasks, grouped by the project's
    /// own configured column layout (`column_defs` -- see `parse_kanban_columns`) rather than a
    /// fixed `todo/in_progress/done` shape. `unsorted_title` names the catch-all column shown for
    /// any task whose resolved status doesn't match a configured column (e.g. it was tagged for a
    /// column that has since been deleted or renamed).
    pub fn build_kanban_board<P: AsRef<Path>>(
        project_dir: P,
        column_defs: &[KanbanColumnDef],
        unsorted_title: &str,
    ) -> WebResult<KanbanBoard> {
        let tasks = Self::extract_all_tasks(project_dir)?;
        let total_tasks = tasks.len();
        let completed_tasks = tasks.iter().filter(|t| t.completed).count();

        let mut buckets: HashMap<String, Vec<MarkdownTask>> = HashMap::new();
        for task in tasks {
            buckets.entry(task.status.clone()).or_default().push(task);
        }

        let mut columns: Vec<KanbanColumn> = column_defs
            .iter()
            .map(|def| {
                KanbanColumn {
                    id: def.id.clone(),
                    title: def.title.clone(),
                    tasks: buckets.remove(&def.id).unwrap_or_default(),
                    is_done: def.is_done,
                }
            })
            .collect();

        // Whatever's left in `buckets` belongs to no configured column -- surface it instead of
        // silently dropping those tasks off the board.
        let mut unsorted: Vec<MarkdownTask> = buckets.into_values().flatten().collect();
        unsorted.sort_by(|a, b| {
            (a.file_path.as_str(), a.line_number).cmp(&(b.file_path.as_str(), b.line_number))
        });
        columns.push(KanbanColumn {
            id: KANBAN_UNSORTED_COLUMN_ID.to_string(),
            title: unsorted_title.to_string(),
            tasks: unsorted,
            is_done: false,
        });

        Ok(KanbanBoard {
            columns,
            total_tasks,
            completed_tasks,
        })
    }

    /// Toggle or update a task status in the physical Markdown document. `is_done_column` tells
    /// this whether `new_status`'s column counts as "done" for the checkbox mark (see
    /// `KanbanColumnDef::is_done`) -- for the 3 built-in status ids callers may simply pass
    /// `new_status == "done"`, matching this function's own pre-custom-columns behavior exactly.
    pub fn update_task_status<P: AsRef<Path>>(
        project_dir: P,
        rel_path: &str,
        line_number: usize,
        new_status: &str,
        is_done_column: bool,
    ) -> WebResult<()> {
        let clean = rel_path.trim().trim_start_matches('/');
        if clean.contains("..") || clean.is_empty() {
            return Err(WebError::BadRequest("Invalid file path".to_string()));
        }

        let full_path = project_dir.as_ref().join(clean);
        if !full_path.exists() {
            return Err(WebError::NotFound(format!("File not found: {rel_path}")));
        }

        let content = std::fs::read_to_string(&full_path)
            .map_err(|e| WebError::Internal(format!("Failed to read file: {e}")))?;

        let mut lines: Vec<String> = content
            .lines()
            .map(std::string::ToString::to_string)
            .collect();
        if lines.is_empty() {
            return Err(WebError::BadRequest("File is empty".to_string()));
        }

        let task_regex = Regex::new(r"^(\s*[-*]\s+)\[([ xX/])\](.*)$")
            .map_err(|e| WebError::Internal(e.to_string()))?;

        // 1. Direct file line match
        let mut target_idx = line_number
            .checked_sub(1)
            .and_then(|i| lines.get(i).map(|l| (i, l)))
            .filter(|(_, l)| task_regex.is_match(l))
            .map(|(i, _)| i);

        // 2. If not matched, try body-relative line offset (e.g. for .anote with frontmatter)
        if target_idx.is_none() {
            let (_, body) = Self::parse_unified_note(&content);
            if !body.is_empty() {
                if let Some(body_start) = content.find(&body) {
                    let prefix_lines = content.get(..body_start).map_or(0, |s| s.lines().count());
                    let candidate = line_number.saturating_add(prefix_lines);
                    if let Some(c_idx) = candidate.checked_sub(1) {
                        if lines.get(c_idx).is_some_and(|l| task_regex.is_match(l)) {
                            target_idx = Some(c_idx);
                        }
                    }
                }
            }
        }

        // 3. If still not matched, check nearby lines (+/- 1, 2)
        if target_idx.is_none() {
            for delta in [-1isize, 1, -2, 2] {
                let test_idx = (isize::try_from(line_number).unwrap_or(0).saturating_sub(1))
                    .saturating_add(delta);
                if let Ok(u_idx) = usize::try_from(test_idx) {
                    if lines.get(u_idx).is_some_and(|l| task_regex.is_match(l)) {
                        target_idx = Some(u_idx);
                        break;
                    }
                }
            }
        }

        let Some(idx) = target_idx else {
            if line_number == 0 || line_number > lines.len() {
                return Err(WebError::BadRequest(format!(
                    "Line number {line_number} out of bounds (file has {} lines)",
                    lines.len()
                )));
            }
            let line_preview = line_number
                .checked_sub(1)
                .and_then(|i| lines.get(i))
                .map_or("", String::as_str);
            return Err(WebError::BadRequest(format!(
                "Line {line_number} is not a recognized markdown task: {line_preview}"
            )));
        };

        if let Some(line) = lines.get(idx).cloned() {
            if let Some(caps) = task_regex.captures(&line) {
                let prefix = caps.get(1).map_or("- ", |m| m.as_str()).to_string();
                let suffix = caps.get(3).map_or("", |m| m.as_str());

                let mark = if is_done_column {
                    "x"
                } else if new_status == "todo" {
                    " "
                } else {
                    "/"
                };

                // Built-in status ids stay bracket-only (no `#status:` tag) for a clean, unmodified
                // representation on boards that were never customized; any other id needs the tag so
                // `extract_all_tasks` can tell which custom column this task belongs to, since the
                // 3-state checkbox alone can't encode more than 3 positions.
                let status_tag_regex = Regex::new(r"\s*#status:[a-zA-Z0-9_\-]+")
                    .map_err(|e| WebError::Internal(e.to_string()))?;
                let stripped_suffix = status_tag_regex.replace_all(suffix, "").to_string();
                let new_suffix = if matches!(new_status, "todo" | "in_progress" | "done") {
                    stripped_suffix
                } else {
                    format!("{stripped_suffix} #status:{new_status}")
                };

                if let Some(slot) = lines.get_mut(idx) {
                    *slot = format!("{prefix}[{mark}]{new_suffix}");
                }
            }
        }

        let new_content = lines.join("\n")
            + if content.ends_with('\n') {
                "\n"
            } else {
                ""
            };
        std::fs::write(&full_path, new_content.as_bytes())
            .map_err(|e| WebError::Internal(format!("Failed to write updated file: {e}")))?;

        Ok(())
    }

    /// Extract bidirectional Wiki links [[Target]] or [[Target|Display]] and construct Knowledge Graph
    pub fn build_knowledge_graph<P: AsRef<Path>>(project_dir: P) -> WebResult<KnowledgeGraph> {
        let root = project_dir.as_ref();
        let files = Self::discover_markdown_files(root);

        let wiki_regex = Regex::new(r"\[\[([^\]|]+)(?:\|([^\]]+))?\]\]")
            .map_err(|e| WebError::Internal(e.to_string()))?;

        let mut node_set: HashMap<String, GraphNode> = HashMap::new();
        let mut edges: Vec<GraphEdge> = Vec::new();
        let mut backlinks: HashMap<String, HashSet<String>> = HashMap::new();

        // 1. Register all existing markdown notes as nodes
        for file in &files {
            let rel_path = file
                .strip_prefix(root)
                .unwrap_or(file)
                .to_string_lossy()
                .replace('\\', "/");

            let base_name = file
                .file_stem()
                .map_or_else(|| rel_path.clone(), |s| s.to_string_lossy().to_string());

            let task_count = match std::fs::read_to_string(file) {
                | Ok(c) => {
                    c.lines()
                        .filter(|l| l.contains("- [ ]") || l.contains("- [x]"))
                        .count()
                },
                | Err(_) => 0,
            };

            node_set.insert(
                base_name.to_lowercase(),
                GraphNode {
                    id: base_name.clone(),
                    label: base_name.clone(),
                    file_path: Some(rel_path),
                    exists: true,
                    backlink_count: 0,
                    task_count,
                },
            );
        }

        // 2. Scan for [[Links]] and construct edges
        for file in &files {
            let source_base = file
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();

            let Ok(content) = std::fs::read_to_string(file) else {
                continue;
            };

            for caps in wiki_regex.captures_iter(&content) {
                let target_raw = caps.get(1).map_or("", |m| m.as_str().trim());
                let display = caps.get(2).map(|m| m.as_str().trim().to_string());

                if target_raw.is_empty() {
                    continue;
                }

                let target_key = target_raw.to_lowercase();
                if !node_set.contains_key(&target_key) {
                    // Create uncreated/placeholder target node
                    node_set.insert(
                        target_key.clone(),
                        GraphNode {
                            id: target_raw.to_string(),
                            label: target_raw.to_string(),
                            file_path: None,
                            exists: false,
                            backlink_count: 0,
                            task_count: 0,
                        },
                    );
                }

                let target_id = node_set
                    .get(&target_key)
                    .map_or_else(|| target_raw.to_string(), |n| n.id.clone());

                edges.push(GraphEdge {
                    source: source_base.clone(),
                    target: target_id.clone(),
                    label: display,
                });

                backlinks
                    .entry(target_key)
                    .or_default()
                    .insert(source_base.clone());
            }
        }

        // 3. Populate backlink count
        for (key, sources) in backlinks {
            if let Some(node) = node_set.get_mut(&key) {
                node.backlink_count = sources.len();
            }
        }

        let mut nodes: Vec<GraphNode> = node_set.into_values().collect();
        nodes.sort_by(|a, b| a.label.cmp(&b.label));

        Ok(KnowledgeGraph { nodes, edges })
    }

    /// Extract calendar events from tasks (@YYYY-MM-DD) and file dates
    pub fn extract_calendar_events<P: AsRef<Path>>(
        project_dir: P
    ) -> WebResult<Vec<CalendarEvent>> {
        let root = project_dir.as_ref();
        let files = Self::discover_markdown_files(root);
        let mut events = Vec::new();

        let date_prefix_regex = Regex::new(r"^(\d{4}-\d{2}-\d{2})[-_](.+)$")
            .map_err(|e| WebError::Internal(e.to_string()))?;

        // 1. Events from tasks with @YYYY-MM-DD
        let tasks = Self::extract_all_tasks(root)?;
        for task in tasks {
            if let Some(due) = task.due_date {
                events.push(CalendarEvent {
                    id: format!("task:{}", task.id),
                    date: due,
                    title: task.title,
                    source_file: task.file_path,
                    line_number: Some(task.line_number),
                    is_task: true,
                    completed: task.completed,
                });
            }
        }

        // 2. Events from files named like 2026-09-15-meeting-notes.md
        for file in files {
            let file_stem = file
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            if let Some(caps) = date_prefix_regex.captures(&file_stem) {
                let date = caps
                    .get(1)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();
                let topic = caps
                    .get(2)
                    .map(|m| m.as_str().replace('-', " "))
                    .unwrap_or_default();

                let rel_path = file
                    .strip_prefix(root)
                    .unwrap_or(&file)
                    .to_string_lossy()
                    .replace('\\', "/");

                events.push(CalendarEvent {
                    id: format!("file:{rel_path}"),
                    date,
                    title: topic,
                    source_file: rel_path,
                    line_number: None,
                    is_task: false,
                    completed: false,
                });
            }
        }

        events.sort_by(|a, b| a.date.cmp(&b.date));
        Ok(events)
    }

    /// Parse unified note format (.anote / .note.md) separating YAML frontmatter and markdown body
    pub fn parse_unified_note(raw: &str) -> (UnifiedNoteMeta, String) {
        let trimmed = raw.trim_start();
        if let Some(rest) = trimmed.strip_prefix("---") {
            if let Some(end_idx) = rest.find("\n---") {
                let frontmatter = &rest[..end_idx];
                let body_start = end_idx.saturating_add(4);
                let body = rest
                    .get(body_start..)
                    .map_or("", |s| s.trim_start_matches('\n'))
                    .to_string();

                let mut meta = UnifiedNoteMeta::default();
                for line in frontmatter.lines() {
                    let line = line.trim();
                    if let Some((k, v)) = line.split_once(':') {
                        let k = k.trim();
                        let v = v.trim().trim_matches('"').trim_matches('\'');
                        match k {
                            | "title" => meta.title = v.to_string(),
                            | "created_at" => meta.created_at = Some(v.to_string()),
                            | "updated_at" => meta.updated_at = Some(v.to_string()),
                            | "author" => meta.author = Some(v.to_string()),
                            | "tags" => {
                                let clean = v.trim_matches('[').trim_matches(']');
                                meta.tags = clean
                                    .split(',')
                                    .map(|s| {
                                        s.trim().trim_matches('"').trim_matches('\'').to_string()
                                    })
                                    .filter(|s| !s.is_empty())
                                    .collect();
                            },
                            | "whiteboard" => {
                                // Base64-encoded JSON, so it survives this naive line-based
                                // parser regardless of quotes/colons/brackets inside the JSON.
                                if let Ok(bytes) = base64::Engine::decode(
                                    &base64::engine::general_purpose::STANDARD,
                                    v,
                                ) {
                                    if let Ok(json_str) = String::from_utf8(bytes) {
                                        if let Ok(val) =
                                            serde_json::from_str::<serde_json::Value>(&json_str)
                                        {
                                            meta.whiteboard = Some(val);
                                        }
                                    }
                                }
                            },
                            | _ => {},
                        }
                    }
                }

                return (meta, body);
            }
        }

        // Fallback if no frontmatter
        let first_line = raw.lines().next().unwrap_or("Untitled Note");
        let title = first_line.trim_start_matches('#').trim().to_string();
        (
            UnifiedNoteMeta {
                title: if title.is_empty() {
                    "Untitled Note".to_string()
                } else {
                    title
                },
                created_at: None,
                updated_at: None,
                tags: Vec::new(),
                author: None,
                whiteboard: None,
            },
            raw.to_string(),
        )
    }

    /// Serialize unified note format back to string with YAML frontmatter
    pub fn serialize_unified_note(
        meta: &UnifiedNoteMeta,
        body: &str,
    ) -> String {
        let tags_str = meta
            .tags
            .iter()
            .map(|t| format!("\"{t}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let mut out = String::from("---\n");
        let _ = writeln!(out, "title: \"{}\"", meta.title.replace('"', "\\\""));
        if let Some(ref c) = meta.created_at {
            let _ = writeln!(out, "created_at: \"{c}\"");
        }
        if let Some(ref u) = meta.updated_at {
            let _ = writeln!(out, "updated_at: \"{u}\"");
        }
        if let Some(ref a) = meta.author {
            let _ = writeln!(out, "author: \"{}\"", a.replace('"', "\\\""));
        }
        let _ = writeln!(out, "tags: [{tags_str}]");
        if let Some(ref wb) = meta.whiteboard {
            let json = serde_json::to_string(wb).unwrap_or_default();
            let b64 =
                base64::Engine::encode(&base64::engine::general_purpose::STANDARD, json.as_bytes());
            let _ = writeln!(out, "whiteboard: \"{b64}\"");
        }
        out.push_str("---\n\n");
        out.push_str(body);
        out
    }

    /// Extract headings for Document Outline
    pub fn extract_headings(content: &str) -> Vec<NoteHeading> {
        let mut headings = Vec::new();
        for (i, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with('#') {
                let level = trimmed.chars().take_while(|&c| c == '#').count();
                if level <= 4 {
                    let text = trimmed[level..].trim().to_string();
                    if !text.is_empty() {
                        headings.push(NoteHeading {
                            level,
                            text,
                            line: i.saturating_add(1),
                        });
                    }
                }
            }
        }
        headings
    }

    /// Typst uses `=`/`==`/`===` for real headings (Markdown's own `#` is a *code invocation* in
    /// Typst -- `#import`, `#show`, `#slide(...)`, etc. -- so running `extract_headings` against
    /// a `.typ` file treated every one of those as a fake "heading"). For a cargo-slide deck
    /// specifically, the more useful outline is one entry per slide, so this also picks up each
    /// `#slide(title: "...")` / `#title-slide(title: "...")` call's title.
    pub fn extract_headings_typst(content: &str) -> Vec<NoteHeading> {
        let Ok(heading_re) = Regex::new(r"^(=+)\s+(.+)$") else {
            return Vec::new();
        };
        let Ok(slide_title_re) =
            Regex::new(r#"^#(?:title-slide|slide)\s*\([^)]*?title:\s*"([^"]+)""#)
        else {
            return Vec::new();
        };

        let mut headings = Vec::new();
        for (i, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if let Some(caps) = heading_re.captures(trimmed) {
                let level = caps.get(1).map_or(1, |m| m.as_str().len()).min(4);
                let text = caps
                    .get(2)
                    .map(|m| m.as_str().trim().to_string())
                    .unwrap_or_default();
                if !text.is_empty() {
                    headings.push(NoteHeading {
                        level,
                        text,
                        line: i.saturating_add(1),
                    });
                }
            } else if let Some(caps) = slide_title_re.captures(trimmed) {
                let text = caps
                    .get(1)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();
                if !text.is_empty() {
                    headings.push(NoteHeading {
                        level: 1,
                        text,
                        line: i.saturating_add(1),
                    });
                }
            }
        }
        headings
    }

    /// LaTeX's real sectioning commands, mapped to outline levels the same way a document's own
    /// table of contents would be (`\part`/`\chapter` are broader than `\section`, which is
    /// broader than `\subsection`, etc.) -- `extract_headings`'s `#`-based logic finds nothing at
    /// all in a `.tex` file, since LaTeX commands start with `\`, not `#`.
    pub fn extract_headings_latex(content: &str) -> Vec<NoteHeading> {
        let Ok(re) =
            Regex::new(r"^\\(part|chapter|section|subsection|subsubsection)\*?\{([^}]*)\}")
        else {
            return Vec::new();
        };
        let mut headings = Vec::new();
        for (i, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if let Some(caps) = re.captures(trimmed) {
                let kind = caps.get(1).map_or("section", |m| m.as_str());
                let level = match kind {
                    | "part" | "chapter" => 1,
                    | "section" => 2,
                    | "subsection" => 3,
                    | _ => 4,
                };
                let text = caps
                    .get(2)
                    .map(|m| m.as_str().trim().to_string())
                    .unwrap_or_default();
                if !text.is_empty() {
                    headings.push(NoteHeading {
                        level,
                        text,
                        line: i.saturating_add(1),
                    });
                }
            }
        }
        headings
    }

    /// Scripts have no heading concept, but top-level function/class definitions serve the same
    /// "jump to a named section" purpose an outline is for -- without this, `extract_headings`'s
    /// `#`-based logic picked up every `#`-prefixed *comment* line in a Python/R/Bash script as a
    /// fake heading, which is a real mess on any script with a normal amount of commenting.
    pub fn extract_headings_script(
        content: &str,
        ext: &str,
    ) -> Vec<NoteHeading> {
        let Ok(re) = (match ext {
            | "py" => Regex::new(r"^(def|class)\s+(\w+)"),
            | "r" => Regex::new(r"^(\w+)\s*(?:<-|=)\s*function\s*\("),
            | "rs" => Regex::new(r"^(?:pub\s+)?(fn|struct|enum|trait|impl)\s+(\w+)"),
            | "js" | "ts" => Regex::new(r"^(?:export\s+)?(?:async\s+)?(function|class)\s+(\w+)"),
            | "sh" | "bash" => Regex::new(r"^(?:function\s+)?(\w+)\s*\(\)\s*\{?"),
            | _ => return Vec::new(),
        }) else {
            return Vec::new();
        };

        let mut headings = Vec::new();
        for (i, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if let Some(caps) = re.captures(trimmed) {
                // R and shell each only have one capture group of interest (the name); the
                // others have the keyword in group 1 and the name in group 2.
                let text = caps
                    .get(2)
                    .or_else(|| caps.get(1))
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();
                if !text.is_empty() {
                    headings.push(NoteHeading {
                        level: 1,
                        text,
                        line: i.saturating_add(1),
                    });
                }
            }
        }
        headings
    }

    /// Dispatches to the right outline extractor for a file's actual language, based on its
    /// extension -- the single call site every editor page should use instead of assuming every
    /// file is Markdown.
    pub fn extract_headings_for_file(
        content: &str,
        file_path: &str,
    ) -> Vec<NoteHeading> {
        let ext = std::path::Path::new(file_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        match ext.as_str() {
            | "typ" => Self::extract_headings_typst(content),
            | "tex" | "latex" => Self::extract_headings_latex(content),
            | "py" | "r" | "rs" | "js" | "ts" | "sh" | "bash" => {
                Self::extract_headings_script(content, &ext)
            },
            | _ => Self::extract_headings(content),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_knowledge_sync_tasks_and_bidirectional_update() {
        let dir = tempdir().unwrap();
        let notes_dir = dir.path().join("notes");
        std::fs::create_dir_all(&notes_dir).unwrap();

        let doc1_path = notes_dir.join("quantum_algorithms.md");
        let content1 = r#"# Quantum Algorithms

Introductory notes on Shor's and Grover's algorithm.

## Tasks
- [ ] Implement quantum Fourier transform simulator #quantum #code @2026-10-01
- [/] Benchmark circuit depth for N=64 #benchmark
- [x] Review Nielsen & Chuang chapter 5 #study @2026-09-15

Related to [[Error Correction]] and [[Hardware Specs|Cryo Fridge Specs]].
"#;
        std::fs::write(&doc1_path, content1).unwrap();

        let doc2_path = notes_dir.join("Error Correction.md");
        let content2 = r#"# Error Correction

Discussion on surface codes.
- [ ] Calculate threshold error rate #theory
Backlink to [[Quantum Algorithms]].
"#;
        std::fs::write(&doc2_path, content2).unwrap();

        // 1. Test Task Extraction
        let tasks = KnowledgeSyncService::extract_all_tasks(dir.path()).unwrap();
        assert_eq!(tasks.len(), 4);

        let qft_task = tasks
            .iter()
            .find(|t| t.title.contains("Implement quantum Fourier transform"))
            .unwrap();
        assert_eq!(qft_task.status, "todo");
        assert_eq!(qft_task.tags, vec!["quantum", "code"]);
        assert_eq!(qft_task.due_date, Some("2026-10-01".to_string()));

        let nielsen_task = tasks
            .iter()
            .find(|t| t.title.contains("Review Nielsen & Chuang"))
            .unwrap();
        assert!(nielsen_task.completed);
        assert_eq!(nielsen_task.status, "done");

        // 2. Test Kanban Board (default 3-column layout)
        let default_columns =
            KnowledgeSyncService::default_kanban_columns("To Do", "In Progress", "Completed");
        let kanban =
            KnowledgeSyncService::build_kanban_board(dir.path(), &default_columns, "Unsorted")
                .unwrap();
        assert_eq!(kanban.total_tasks, 4);
        assert_eq!(kanban.completed_tasks, 1);
        assert_eq!(kanban.columns[0].tasks.len(), 2); // todo
        assert_eq!(kanban.columns[1].tasks.len(), 1); // in_progress
        assert_eq!(kanban.columns[2].tasks.len(), 1); // done
        assert_eq!(kanban.columns[3].id, KANBAN_UNSORTED_COLUMN_ID);
        assert!(kanban.columns[3].tasks.is_empty()); // nothing tagged with a status this layout doesn't have

        // 3. Test Bidirectional Update: Complete QFT task
        KnowledgeSyncService::update_task_status(
            dir.path(),
            "notes/quantum_algorithms.md",
            qft_task.line_number,
            "done",
            true,
        )
        .unwrap();

        // Re-read file and verify
        let updated_content = std::fs::read_to_string(&doc1_path).unwrap();
        assert!(updated_content.contains("- [x] Implement quantum Fourier transform simulator"));

        // 4. Test Knowledge Graph and Bidirectional Wiki Links
        let graph = KnowledgeSyncService::build_knowledge_graph(dir.path()).unwrap();
        assert_eq!(graph.edges.len(), 3); // doc1 -> Error Correction, doc1 -> Hardware Specs, doc2 -> Quantum Algorithms

        let err_corr_node = graph
            .nodes
            .iter()
            .find(|n| n.label.eq_ignore_ascii_case("Error Correction"))
            .unwrap();
        assert!(err_corr_node.exists);
        assert_eq!(err_corr_node.backlink_count, 1);

        let hw_node = graph
            .nodes
            .iter()
            .find(|n| n.label.eq_ignore_ascii_case("Hardware Specs"))
            .unwrap();
        assert!(!hw_node.exists); // placeholder node created from [[Hardware Specs|...]]

        // 5. Test Calendar Events
        let cal_events = KnowledgeSyncService::extract_calendar_events(dir.path()).unwrap();
        assert_eq!(cal_events.len(), 2); // 2026-09-15 and 2026-10-01
        assert_eq!(cal_events[0].date, "2026-09-15");
        assert_eq!(cal_events[1].date, "2026-10-01");
    }

    #[test]
    fn test_custom_kanban_columns_status_tag_round_trip_and_unsorted_catchall() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("notes")).unwrap();
        let doc_path = dir.path().join("notes/backlog.md");
        std::fs::write(
            &doc_path,
            "# Backlog\n\n- [ ] Triage new bug reports\n- [ ] Write onboarding doc\n",
        )
        .unwrap();

        // A team-defined 4-column workflow beyond the default 3 -- "review" has no matching
        // checkbox state, so it can only exist via the `#status:` tag.
        let columns = vec![
            KanbanColumnDef {
                id: "backlog".to_string(),
                title: "Backlog".to_string(),
                is_done: false,
            },
            KanbanColumnDef {
                id: "review".to_string(),
                title: "In Review".to_string(),
                is_done: false,
            },
            KanbanColumnDef {
                id: "shipped".to_string(),
                title: "Shipped".to_string(),
                is_done: true,
            },
        ];

        let tasks = KnowledgeSyncService::extract_all_tasks(dir.path()).unwrap();
        let triage_task = tasks.iter().find(|t| t.title.contains("Triage")).unwrap();
        // Neither task has been tagged for any of this custom layout's columns yet -- both must
        // land in the Unsorted catch-all rather than vanish or force themselves into "backlog".
        let board =
            KnowledgeSyncService::build_kanban_board(dir.path(), &columns, "Unsorted").unwrap();
        assert_eq!(
            board
                .columns
                .iter()
                .find(|c| c.id == "backlog")
                .unwrap()
                .tasks
                .len(),
            0
        );
        assert_eq!(
            board
                .columns
                .iter()
                .find(|c| c.id == KANBAN_UNSORTED_COLUMN_ID)
                .unwrap()
                .tasks
                .len(),
            2
        );

        // Move the triage task into "review" (not done) -- must write a #status: tag since the
        // checkbox alone can't represent it, and must NOT mark the box done.
        KnowledgeSyncService::update_task_status(
            dir.path(),
            "notes/backlog.md",
            triage_task.line_number,
            "review",
            false,
        )
        .unwrap();
        let after_review = std::fs::read_to_string(&doc_path).unwrap();
        assert!(
            after_review.contains("- [/] Triage new bug reports #status:review"),
            "expected in-progress bracket + status tag: {after_review}"
        );

        let tasks2 = KnowledgeSyncService::extract_all_tasks(dir.path()).unwrap();
        let triage2 = tasks2.iter().find(|t| t.title.contains("Triage")).unwrap();
        assert_eq!(triage2.status, "review");
        assert!(!triage2.completed);
        assert!(
            triage2.tags.is_empty(),
            "the status tag must not also appear as a plain tag: {:?}",
            triage2.tags
        );

        let board2 =
            KnowledgeSyncService::build_kanban_board(dir.path(), &columns, "Unsorted").unwrap();
        assert_eq!(
            board2
                .columns
                .iter()
                .find(|c| c.id == "review")
                .unwrap()
                .tasks
                .len(),
            1
        );
        assert_eq!(
            board2
                .columns
                .iter()
                .find(|c| c.id == KANBAN_UNSORTED_COLUMN_ID)
                .unwrap()
                .tasks
                .len(),
            1
        );

        // Move it on to "shipped" (a done column) -- checkbox must flip to [x], and the old
        // #status:review tag must be replaced, not left stacked alongside the new one.
        KnowledgeSyncService::update_task_status(
            dir.path(),
            "notes/backlog.md",
            triage_task.line_number,
            "shipped",
            true,
        )
        .unwrap();
        let after_shipped = std::fs::read_to_string(&doc_path).unwrap();
        assert!(after_shipped.contains("- [x] Triage new bug reports #status:shipped"));
        assert!(!after_shipped.contains("status:review"));

        // Finally, back to the built-in "todo" id -- the tag must be fully removed, restoring the
        // plain, untagged representation every pre-custom-columns board already relied on.
        KnowledgeSyncService::update_task_status(
            dir.path(),
            "notes/backlog.md",
            triage_task.line_number,
            "todo",
            false,
        )
        .unwrap();
        let after_todo = std::fs::read_to_string(&doc_path).unwrap();
        assert!(
            after_todo.contains("- [ ] Triage new bug reports\n"),
            "expected a clean, untagged line: {after_todo}"
        );
    }

    #[test]
    fn test_unified_note_whiteboard_round_trip() {
        // The whiteboard field previously round-tripped to nothing: serialize_unified_note
        // never wrote it out, and parse_unified_note never read it back in, despite the
        // struct field existing -- this locks in the real fix.
        let mut meta = UnifiedNoteMeta {
            title: "Sketch Notes".to_string(),
            created_at: Some("2026-09-01T00:00:00Z".to_string()),
            updated_at: None,
            tags: vec!["diagram".to_string()],
            author: Some("Dr. Test".to_string()),
            whiteboard: Some(serde_json::json!({
                "strokes": [{"color": "#ff0000", "width": 3, "points": [[10, 10], [20, 20]]}]
            })),
        };

        let serialized = KnowledgeSyncService::serialize_unified_note(&meta, "# Body content");
        let (parsed_meta, parsed_body) = KnowledgeSyncService::parse_unified_note(&serialized);

        assert_eq!(parsed_body.trim(), "# Body content");
        assert_eq!(parsed_meta.title, "Sketch Notes");
        assert_eq!(parsed_meta.tags, vec!["diagram".to_string()]);
        let strokes = parsed_meta
            .whiteboard
            .as_ref()
            .unwrap()
            .get("strokes")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(strokes.len(), 1);
        assert_eq!(strokes[0]["color"], "#ff0000");
        assert_eq!(strokes[0]["points"][1][0], 20);

        // Also verify saving without a whiteboard change preserves it (read-modify-write shape).
        meta.title = "Renamed".to_string();
        let re_serialized = KnowledgeSyncService::serialize_unified_note(&meta, "# Body content");
        let (re_parsed, _) = KnowledgeSyncService::parse_unified_note(&re_serialized);
        assert_eq!(re_parsed.title, "Renamed");
        assert!(re_parsed.whiteboard.is_some());
    }

    /// Regression test: `extract_headings` (Markdown's `#`-based logic) used to be called for
    /// every file type, including Typst -- where `#` is a *code invocation*, not a heading, so
    /// every `#import`/`#slide(...)` line was picked up as a fake heading. Real Typst headings
    /// use `=`/`==`/`===`; cargo-slide's `#slide(title: "...")` calls should still show up in the
    /// outline (one entry per slide), just correctly, not as a byproduct of misreading `#`.
    #[test]
    fn test_extract_headings_typst_ignores_code_invocations() {
        let content = r#"#import "theme.typ": *
#show: slide-theme.with(aspect-ratio: "16-9")

#title-slide(title: "Opening Slide", subtitle: "A Talk")

= Section One
Some body text.

#slide(title: "Architecture Overview", transition: "slide-left")[
  content here
]

== Subsection
"#;
        let headings = KnowledgeSyncService::extract_headings_typst(content);
        let texts: Vec<&str> = headings.iter().map(|h| h.text.as_str()).collect();
        assert_eq!(
            texts,
            vec![
                "Opening Slide",
                "Section One",
                "Architecture Overview",
                "Subsection"
            ]
        );
        assert_eq!(headings[1].level, 1); // "= Section One"
        assert_eq!(headings[3].level, 2); // "== Subsection"
                                          // None of the plain `#`-invocation lines (#import, #show) leaked in as fake headings.
        assert!(!texts
            .iter()
            .any(|t| t.contains("import") || t.contains("slide-theme")));
    }

    /// LaTeX's real sectioning commands (`\part`/`\chapter`/`\section`/...) start with `\`, not
    /// `#` -- the old Markdown-only extractor found nothing at all in a `.tex` file.
    #[test]
    fn test_extract_headings_latex() {
        let content = r#"\documentclass{article}
\title{A Paper}

\section{Introduction}
Some text.

\subsection{Background}
More text.

\section{Conclusion}
"#;
        let headings = KnowledgeSyncService::extract_headings_latex(content);
        let texts: Vec<&str> = headings.iter().map(|h| h.text.as_str()).collect();
        assert_eq!(texts, vec!["Introduction", "Background", "Conclusion"]);
        assert_eq!(headings[0].level, 2);
        assert_eq!(headings[1].level, 3);
    }

    /// Scripts have no heading syntax, but the old extractor picked up every `#`-prefixed
    /// *comment* line as a fake heading (Python/R/Bash all use `#` for comments) -- a real mess
    /// on any normally-commented script. Top-level function/class definitions are what an
    /// outline should show instead.
    #[test]
    fn test_extract_headings_script_python_ignores_comments() {
        let content = r#"# This is just a comment, not a heading
import csv

def calculate_average_t1(csv_path):
    # inline comment
    pass

class Analyzer:
    pass
"#;
        let headings = KnowledgeSyncService::extract_headings_script(content, "py");
        let texts: Vec<&str> = headings.iter().map(|h| h.text.as_str()).collect();
        assert_eq!(texts, vec!["calculate_average_t1", "Analyzer"]);
    }

    #[test]
    fn test_extract_headings_for_file_dispatches_by_extension() {
        assert_eq!(
            KnowledgeSyncService::extract_headings_for_file("= Heading\n", "paper.typ")[0].text,
            "Heading"
        );
        assert_eq!(
            KnowledgeSyncService::extract_headings_for_file("\\section{Intro}\n", "report.tex")[0]
                .text,
            "Intro"
        );
        assert_eq!(
            KnowledgeSyncService::extract_headings_for_file(
                "def foo():\n    pass\n",
                "analysis.py"
            )[0]
            .text,
            "foo"
        );
        assert_eq!(
            KnowledgeSyncService::extract_headings_for_file("# A Note Heading\n", "notes.md")[0]
                .text,
            "A Note Heading"
        );
    }
}
