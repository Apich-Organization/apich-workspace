//! Seeds a starter set of ready-to-use, `public`-visibility Template Library entries so the
//! library isn't an empty shelf someone has to fill themselves before it's useful -- publishing
//! your own template still works exactly the same way alongside these. Idempotent: each template
//! below has a fixed, reserved slug under `owner_user_id`; if a template with that slug already
//! exists, seeding it is skipped, so calling this on every server start (see `main.rs`) is safe
//! and cheap after the first run.

use crate::error::WebResult;
use crate::services::knowledge_sync::KanbanColumnDef;
use crate::services::template_library::files_content_from_files;
use crate::services::template_library::kanban_content_from_columns;
use crate::services::template_library::note_content_from_body;
use crate::services::template_library::TemplateFile;
use apich_db::CreateTemplateDto;
use apich_db::PublishTemplateVersionDto;
use apich_db::Repository;
use uuid::Uuid;

struct SeedTemplate {
    kind: &'static str,
    slug: &'static str,
    name: &'static str,
    description: &'static str,
    content: serde_json::Value,
}

fn kanban_cols(cols: &[(&str, &str, bool)]) -> serde_json::Value {
    let defs: Vec<KanbanColumnDef> = cols
        .iter()
        .map(|(id, title, is_done)| {
            KanbanColumnDef {
                id: id.to_string(),
                title: title.to_string(),
                is_done: *is_done,
            }
        })
        .collect();
    kanban_content_from_columns(&defs)
}

fn note(body: &str) -> serde_json::Value {
    note_content_from_body(body)
}

fn single_file(
    path: &str,
    content: &str,
) -> serde_json::Value {
    files_content_from_files(vec![TemplateFile {
        path: path.to_string(),
        content: content.to_string(),
    }])
}

fn seed_list() -> Vec<SeedTemplate> {
    vec![
        // --- Kanban ---
        SeedTemplate {
            kind: "kanban",
            slug: "simple-todo-board",
            name: "Simple To Do Board",
            description: "The classic 3-column workflow: To Do, In Progress, Done.",
            content: kanban_cols(&[("todo", "To Do", false), ("in_progress", "In Progress", false), ("done", "Done", true)]),
        },
        SeedTemplate {
            kind: "kanban",
            slug: "software-sprint-board",
            name: "Software Sprint Board",
            description: "Backlog through Review for a typical engineering sprint.",
            content: kanban_cols(&[
                ("backlog", "Backlog", false),
                ("todo", "To Do", false),
                ("in_progress", "In Progress", false),
                ("in_review", "In Review", false),
                ("done", "Done", true),
            ]),
        },
        SeedTemplate {
            kind: "kanban",
            slug: "research-project-board",
            name: "Research Project Board",
            description: "For tracking a research project from idea to publication.",
            content: kanban_cols(&[
                ("ideas", "Ideas", false),
                ("planned", "Planned", false),
                ("experimenting", "Experimenting", false),
                ("analyzing", "Analyzing Results", false),
                ("published", "Published / Done", true),
            ]),
        },
        SeedTemplate {
            kind: "kanban",
            slug: "bug-triage-board",
            name: "Bug Triage Board",
            description: "Incoming bug reports through resolution.",
            content: kanban_cols(&[
                ("new", "New", false),
                ("confirmed", "Confirmed", false),
                ("in_progress", "In Progress", false),
                ("fixed", "Fixed", true),
                ("wont_fix", "Won't Fix", true),
            ]),
        },
        // --- Note ---
        SeedTemplate {
            kind: "note",
            slug: "meeting-notes",
            name: "Meeting Notes",
            description: "Attendees, agenda, discussion, and action items.",
            content: note(r#"---
title: "Meeting Notes"
tags: ["meeting"]
---

# Meeting Notes

**Date:** @2026-01-01
**Attendees:**
-

## Agenda
1.

## Discussion

## Action Items
- [ ] #followup
- [ ]
"#),
        },
        SeedTemplate {
            kind: "note",
            slug: "literature-review",
            name: "Literature Review",
            description: "Paper summary, key findings, and relevance to your work.",
            content: note(r#"---
title: "Literature Review: "
tags: ["literature", "review"]
---

# Literature Review

**Paper:**
**Authors:**
**Year:**
**Link/DOI:**

## Summary


## Key Findings
-

## Methodology Notes


## Relevance To My Work


## Open Questions
- [ ]
"#),
        },
        SeedTemplate {
            kind: "note",
            slug: "experiment-log",
            name: "Experiment Log",
            description: "Hypothesis, method, results, and conclusion for a single experiment.",
            content: note(r#"---
title: "Experiment Log"
tags: ["experiment"]
---

# Experiment Log

**Date:** @2026-01-01

## Hypothesis


## Method


## Setup Checklist
- [ ] Calibrate instruments #setup
- [ ] Record baseline conditions

## Results


## Conclusion


## Next Steps
- [ ]
"#),
        },
        // --- LaTeX ---
        SeedTemplate {
            kind: "latex",
            slug: "latex-article-with-abstract",
            name: "Article with Abstract & References",
            description: "A complete \\documentclass{article} skeleton: abstract, sections, and a manual bibliography.",
            content: single_file(
                "paper.tex",
                r#"\documentclass[11pt]{article}
\usepackage[utf8]{inputenc}
\usepackage{amsmath}
\usepackage{graphicx}
\usepackage{hyperref}

\title{Your Paper Title}
\author{Your Name \\ Institution}
\date{\today}

\begin{document}
\maketitle

\begin{abstract}
Write a concise summary of your paper's motivation, method, and findings here.
\end{abstract}

\section{Introduction}
Introduce the problem and why it matters.

\section{Related Work}
Summarize prior work and how this paper differs.

\section{Method}
Describe your approach.

\section{Results}
Present your findings.

\section{Conclusion}
Summarize contributions and future work.

\begin{thebibliography}{9}
\bibitem{example2026}
A. Author, ``An Example Reference,'' \textit{Journal of Examples}, 2026.
\end{thebibliography}

\end{document}
"#,
            ),
        },
        SeedTemplate {
            kind: "latex",
            slug: "latex-report",
            name: "Multi-Section Report",
            description: "A \\documentclass{report} skeleton with a table of contents and chapters.",
            content: single_file(
                "report.tex",
                r#"\documentclass[11pt]{report}
\usepackage[utf8]{inputenc}
\usepackage{amsmath}
\usepackage{graphicx}

\title{Report Title}
\author{Your Name}
\date{\today}

\begin{document}
\maketitle
\tableofcontents

\chapter{Introduction}
Background and motivation.

\chapter{Background}
Context and prior work.

\chapter{Findings}
Your main content.

\chapter{Conclusion}
Summary and recommendations.

\end{document}
"#,
            ),
        },
        // --- Typst ---
        SeedTemplate {
            kind: "typst",
            slug: "typst-academic-paper",
            name: "Academic Paper",
            description: "Title block, abstract, sections, and a references section.",
            // Deliberately no `#set text(font: ...)` (Typst's own bundled default font renders
            // everywhere with no "unknown font family" warning, unlike naming a specific font
            // that may not be installed) and no `#bibliography(...)` call (which requires a
            // second `refs.bib` file this single-file template doesn't ship, and fails the whole
            // compile with "file not found" if referenced without one -- confirmed live). A
            // manual "= References" section mirrors the LaTeX template's own manual bibliography.
            content: single_file(
                "paper.typ",
                r#"#set page(paper: "a4", margin: 2.5cm)
#set text(size: 11pt)
#set heading(numbering: "1.1")

#align(center)[
  #text(size: 18pt, weight: "bold")[Your Paper Title]

  Your Name --- Institution
]

#align(center)[
  #box(width: 80%)[
    *Abstract.* Write a concise summary of your paper's motivation, method, and findings here.
  ]
]

= Introduction
Introduce the problem and why it matters.

= Related Work
Summarize prior work and how this differs.

= Method
Describe your approach.

= Results
Present your findings.

= Conclusion
Summarize contributions and future work.

= References
+ A. Author, "An Example Reference," _Journal of Examples_, 2026.
"#,
            ),
        },
        SeedTemplate {
            kind: "typst",
            slug: "typst-lab-report",
            name: "Lab Report",
            description: "Objective, materials, procedure, results, and discussion.",
            content: single_file(
                "lab_report.typ",
                r#"#set page(paper: "a4", margin: 2.5cm)
#set text(size: 11pt)

= Lab Report

*Date:* #datetime.today().display()

== Objective


== Materials
-

== Procedure
+

== Results


== Discussion


== Conclusion
"#,
            ),
        },
        // --- Slides (cargo-slide, per plan.md) ---
        // Both slide templates use `slide.typ`'s *real* exported API (`title-slide(title:,
        // subtitle:, author:, institution:, date:)`, `slide(title:, body)`) -- confirmed by
        // reading the actual helper file live, since an earlier draft of these (and, separately,
        // this app's own pre-existing blank "Cargo-Slide Deck" file starter in
        // `ProjectManager::create_file`) used a `slide-theme`/`#step[...]` API that doesn't exist
        // in it at all, which fails every compile with "unknown variable: slide-theme". Both also
        // need the `#show: slide-theme.with(...)` header (theme.typ's real API for establishing
        // the 16:9 page size) that an earlier fix pass dropped along with the broken call above --
        // without it every deck silently falls back to Typst's non-16:9 default page and fails
        // cargo-slide's own overflow check on virtually any real content, confirmed live.
        SeedTemplate {
            kind: "slides",
            slug: "slides-conference-talk",
            name: "Conference Talk Deck",
            description: "Title, agenda, two content slides, and a thank-you slide.",
            content: single_file(
                "slides.typ",
                r#"#import "theme.typ": *
#import "slide.typ": *

#show: slide-theme.with(
  aspect-ratio: "16-9",
  theme: "dark"
)

#title-slide(
  title: "Your Talk Title",
  subtitle: "A subtitle or venue name",
  author: "Your Name",
  institution: "Institution",
  date: "2026-01-01",
)

#slide(title: "Agenda")[
  - Motivation
  - Approach
  - Results
  - Conclusion
]

#slide(title: "Motivation")[
  - Why this problem matters
  - What's missing today
]

#slide(title: "Results")[
  - Headline finding
  - Supporting evidence
]

#slide(title: "Thank You")[
  Questions?
]
"#,
            ),
        },
        SeedTemplate {
            kind: "slides",
            slug: "slides-project-update",
            name: "Project Status Update",
            description: "Status, progress, blockers, and next steps -- for a recurring team update.",
            content: single_file(
                "slides.typ",
                r#"#import "theme.typ": *
#import "slide.typ": *

#show: slide-theme.with(
  aspect-ratio: "16-9",
  theme: "dark"
)

#title-slide(
  title: "Project Status Update",
  subtitle: "Week of 2026-01-01",
  author: "Your Name",
  institution: "",
  date: "2026-01-01",
)

#slide(title: "Progress")[
  - What shipped this week
  - Key metrics
]

#slide(title: "Blockers")[
  - Current blockers
  - Who's needed to unblock
]

#slide(title: "Next Steps")[
  - Plan for next week
]
"#,
            ),
        },
    ]
}

/// Publishes each of `seed_list()`'s templates (as version "1.0.0", `public` visibility) under
/// `owner_user_id` if a template with that reserved slug doesn't already exist for that owner.
pub async fn seed_default_templates(
    repo: &Repository<'_>,
    owner_user_id: Uuid,
) -> WebResult<()> {
    let existing = repo.list_templates_owned_by(owner_user_id).await?;
    for seed in seed_list() {
        if existing.iter().any(|t| t.slug == seed.slug) {
            continue;
        }
        let template = repo
            .create_template(CreateTemplateDto {
                kind: seed.kind.to_string(),
                name: seed.name.to_string(),
                slug: seed.slug.to_string(),
                description: Some(seed.description.to_string()),
                owner_user_id,
                visibility: "public".to_string(),
            })
            .await?;
        repo.publish_template_version(PublishTemplateVersionDto {
            template_id: template.id,
            version_label: "1.0.0".to_string(),
            changelog: Some("Initial default template.".to_string()),
            content: seed.content,
            published_by: owner_user_id,
        })
        .await?;
    }
    Ok(())
}
