//! Content models and serialization for the Template Library.
//!
//! Handles pure (de)serialization and apply logic for kind-specific template shapes
//! (`apich_db::Template`/`TemplateVersion`). The DB layer stores each version's
//! content as an opaque JSONB blob, which is interpreted here per kind.

use crate::error::WebError;
use crate::error::WebResult;
use crate::services::knowledge_sync::KanbanColumnDef;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KanbanTemplateContent {
    pub columns: Vec<KanbanColumnDef>,
}

pub fn kanban_content_from_columns(columns: &[KanbanColumnDef]) -> serde_json::Value {
    serde_json::to_value(KanbanTemplateContent {
        columns: columns.to_vec(),
    })
    .unwrap_or_else(|_| serde_json::json!({"columns": []}))
}

pub fn kanban_columns_from_content(content: &serde_json::Value) -> WebResult<Vec<KanbanColumnDef>> {
    let parsed: KanbanTemplateContent = serde_json::from_value(content.clone())
        .map_err(|e| WebError::BadRequest(format!("Malformed kanban template content: {e}")))?;
    if parsed.columns.is_empty() {
        return Err(WebError::BadRequest(
            "Kanban template has no columns".to_string(),
        ));
    }
    Ok(parsed.columns)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteTemplateContent {
    pub body: String,
}

pub fn note_content_from_body(body: &str) -> serde_json::Value {
    serde_json::to_value(NoteTemplateContent {
        body: body.to_string(),
    })
    .unwrap_or_else(|_| serde_json::json!({"body": ""}))
}

pub fn note_body_from_content(content: &serde_json::Value) -> WebResult<String> {
    let parsed: NoteTemplateContent = serde_json::from_value(content.clone())
        .map_err(|e| WebError::BadRequest(format!("Malformed note template content: {e}")))?;
    Ok(parsed.body)
}

/// One file within a multi-file template (a LaTeX/Typst/slides project can be more than a single
/// `.tex`/`.typ` -- includes, bibliography, per-slide files). Text content only: binary assets
/// (images, fonts) aren't supported by this version of the template library.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateFile {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesTemplateContent {
    pub files: Vec<TemplateFile>,
}

pub fn files_content_from_files(files: Vec<TemplateFile>) -> serde_json::Value {
    serde_json::to_value(FilesTemplateContent { files })
        .unwrap_or_else(|_| serde_json::json!({"files": []}))
}

pub fn files_from_content(content: &serde_json::Value) -> WebResult<Vec<TemplateFile>> {
    let parsed: FilesTemplateContent = serde_json::from_value(content.clone())
        .map_err(|e| WebError::BadRequest(format!("Malformed template content: {e}")))?;
    if parsed.files.is_empty() {
        return Err(WebError::BadRequest("Template has no files".to_string()));
    }
    Ok(parsed.files)
}

/// Reads a set of project-relative file paths (as chosen by whoever is publishing a
/// latex/typst/slides template from their own project) into a `FilesTemplateContent`. Rejects
/// path traversal and missing files up front rather than silently skipping them, since a
/// publisher should know immediately if what they asked to publish doesn't exist.
pub fn read_files_from_project<P: AsRef<std::path::Path>>(
    project_dir: P,
    rel_paths: &[String],
) -> WebResult<serde_json::Value> {
    let root = project_dir.as_ref();
    let mut files = Vec::with_capacity(rel_paths.len());
    for rel in rel_paths {
        let clean = rel.trim().trim_start_matches('/');
        if clean.is_empty() || clean.contains("..") {
            return Err(WebError::BadRequest(format!("Invalid file path: {rel}")));
        }
        let full = root.join(clean);
        let content = std::fs::read_to_string(&full)
            .map_err(|e| WebError::BadRequest(format!("Failed to read {rel}: {e}")))?;
        files.push(TemplateFile {
            path: clean.to_string(),
            content,
        });
    }
    if files.is_empty() {
        return Err(WebError::BadRequest(
            "No files selected to publish".to_string(),
        ));
    }
    Ok(files_content_from_files(files))
}

/// Writes a multi-file template's content into a real project directory -- the "apply" half of
/// the latex/typst/slides flow. `dest_subdir` (e.g. an empty string for the project root, or a
/// name like "paper-template") lets the caller avoid silently overwriting existing files with the
/// same names; refuses to overwrite anything that already exists rather than guessing which
/// version should win. Returns the project-relative paths actually written.
pub fn apply_files_content_to_project<P: AsRef<std::path::Path>>(
    project_dir: P,
    content: &serde_json::Value,
    dest_subdir: &str,
) -> WebResult<Vec<String>> {
    let files = files_from_content(content)?;
    let root = project_dir.as_ref();
    let clean_subdir = dest_subdir
        .trim()
        .trim_start_matches('/')
        .trim_end_matches('/');
    if clean_subdir.contains("..") {
        return Err(WebError::BadRequest(
            "Invalid destination folder".to_string(),
        ));
    }

    let mut written = Vec::with_capacity(files.len());
    for file in &files {
        let clean = file.path.trim().trim_start_matches('/');
        if clean.is_empty() || clean.contains("..") {
            return Err(WebError::BadRequest(format!(
                "Invalid file path in template: {}",
                file.path
            )));
        }
        let rel = if clean_subdir.is_empty() {
            clean.to_string()
        } else {
            format!("{clean_subdir}/{clean}")
        };
        let full = root.join(&rel);
        if full.exists() {
            return Err(WebError::BadRequest(format!(
                "{rel} already exists in this project -- choose a different destination folder"
            )));
        }
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| WebError::Internal(format!("Failed to create directory: {e}")))?;
        }
        std::fs::write(&full, &file.content)
            .map_err(|e| WebError::Internal(format!("Failed to write {rel}: {e}")))?;
        written.push(rel);
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kanban_content_round_trips() {
        let columns = vec![
            KanbanColumnDef {
                id: "todo".to_string(),
                title: "To Do".to_string(),
                is_done: false,
            },
            KanbanColumnDef {
                id: "done".to_string(),
                title: "Done".to_string(),
                is_done: true,
            },
        ];
        let content = kanban_content_from_columns(&columns);
        let parsed = kanban_columns_from_content(&content).unwrap();
        assert_eq!(parsed, columns);
    }

    #[test]
    fn kanban_content_rejects_empty_columns() {
        let content = serde_json::json!({"columns": []});
        assert!(kanban_columns_from_content(&content).is_err());
    }

    #[test]
    fn note_content_round_trips() {
        let content = note_content_from_body("# Hello\n\nSome text.");
        assert_eq!(
            note_body_from_content(&content).unwrap(),
            "# Hello\n\nSome text."
        );
    }

    #[test]
    fn files_content_round_trip_and_apply_writes_real_files() {
        let dir = tempfile::tempdir().unwrap();
        let content = files_content_from_files(vec![
            TemplateFile {
                path: "main.typ".to_string(),
                content: "= Title\n".to_string(),
            },
            TemplateFile {
                path: "refs.bib".to_string(),
                content: "@article{x}".to_string(),
            },
        ]);

        let written = apply_files_content_to_project(dir.path(), &content, "").unwrap();
        assert_eq!(written.len(), 2);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("main.typ")).unwrap(),
            "= Title\n"
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("refs.bib")).unwrap(),
            "@article{x}"
        );
    }

    #[test]
    fn apply_refuses_to_overwrite_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("main.typ"), "existing content").unwrap();
        let content = files_content_from_files(vec![TemplateFile {
            path: "main.typ".to_string(),
            content: "new".to_string(),
        }]);
        let result = apply_files_content_to_project(dir.path(), &content, "");
        assert!(result.is_err());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("main.typ")).unwrap(),
            "existing content"
        );
    }

    #[test]
    fn apply_rejects_path_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let content = files_content_from_files(vec![TemplateFile {
            path: "../escape.txt".to_string(),
            content: "x".to_string(),
        }]);
        assert!(apply_files_content_to_project(dir.path(), &content, "").is_err());
    }

    #[test]
    fn read_files_from_project_rejects_path_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let result = read_files_from_project(dir.path(), &["../etc/passwd".to_string()]);
        assert!(result.is_err());
    }
}
