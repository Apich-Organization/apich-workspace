//! The two Typst helper files every cargo-slide deck (`slides.typ`/`*.slide.typ`) imports --
//! `theme.typ` (colors + the page/text setup) and `slide.typ` (the actual `title-slide`/`slide`/
//! `callout`/etc. component macros, which itself imports `theme.typ`). Previously only ever
//! written into a project by `DemoProjectService::seed_demo_files` (the "✨ Seed Showcase Demo"
//! button) -- meaning any *other* project, including one started via the plain "+ New File" ->
//! "Cargo-Slide Deck" starter or a slides-kind Template Library template, had no `slide.typ` to
//! import at all and failed its very first compile with "unknown variable: slide-theme" (a real,
//! confirmed-live bug, not hypothetical) or "file not found: slide.typ". Extracted here as the
//! single source of truth so `demo_project.rs`, `project_manager.rs`'s starter, the Template
//! Library's apply/preview paths, and anything else that provisions a slide deck all agree on the
//! same content instead of each carrying their own copy that can drift.

pub const THEME_TYP: &str = r###"// APICH cargo-slide theme
#let slide-colors = (
  accent: rgb("#2563eb"),
  accent-cyan: rgb("#0284c7"),
  accent-purple: rgb("#7c3aed"),
  accent-orange: rgb("#d97706"),
  bg-dark: rgb("#0f172a"),
  bg-light: rgb("#ffffff"),
  text-dark: rgb("#f8fafc"),
  text-light: rgb("#0f172a"),
)

#let slide-theme(aspect-ratio: "16-9", theme: "dark", body) = {
  set document(title: "APICH Presentation", author: "APICH Research")
  set page(
    paper: if aspect-ratio == "16-9" { "presentation-16-9" } else { "presentation-4-3" },
    margin: (x: 2cm, y: 1.5cm),
    fill: if theme == "dark" { slide-colors.bg-dark } else { slide-colors.bg-light }
  )
  set text(
    font: ("Inter", "Roboto", "Liberation Sans", "DejaVu Sans"),
    size: 20pt,
    fill: if theme == "dark" { slide-colors.text-dark } else { slide-colors.text-light }
  )
  body
}
"###;

pub const SLIDE_TYP: &str = r###"// APICH cargo-slide component macros
#import "theme.typ": slide-colors

#let title-slide(title: "", subtitle: "", author: "", institution: "", date: "") = {
  align(center + horizon)[
    #block(text(size: 32pt, weight: "bold", fill: rgb("#38bdf8"), title))
    #v(0.5em)
    #if subtitle != "" { block(text(size: 19pt, fill: rgb("#cbd5e1"), subtitle)) }
    #v(1.2em)
    #if author != "" { block(text(size: 16pt, weight: "medium", fill: rgb("#f8fafc"), author)) }
    #if institution != "" { block(text(size: 14pt, fill: rgb("#94a3b8"), institution)) }
    #v(0.4em)
    #if date != "" { block(text(size: 13pt, fill: rgb("#64748b"), date)) }
  ]
  pagebreak()
}

#let slide(title: "", transition: "slide-left", body) = {
  block(
    width: 100%,
    stroke: (bottom: 1.5pt + rgb("#334155")),
    inset: (bottom: 0.4em),
    text(size: 24pt, weight: "bold", fill: rgb("#60a5fa"), title)
  )
  v(0.6em)
  body
  pagebreak()
}

#let step(order, effect: "fade-in", body) = {
  body
}

#let callout(title: "", stroke-color: rgb("#2563eb"), body) = {
  block(
    fill: rgb("#1e293b"),
    stroke: (left: 4pt + stroke-color),
    radius: (right: 6pt),
    inset: (x: 1em, y: 0.8em),
    width: 100%,
    [
      #if title != "" { block(text(size: 14pt, weight: "bold", fill: rgb("#f1f5f9"), title)) }
      #v(0.2em)
      #text(size: 12pt, fill: rgb("#cbd5e1"), body)
    ]
  )
}

#let cols(..items) = {
  let count = items.pos().len()
  grid(
    columns: (1fr,) * count,
    gutter: 0.8cm,
    ..items.pos()
  )
}

#let badge(label, fill: rgb("#2563eb")) = {
  box(
    fill: fill,
    radius: 4pt,
    inset: (x: 8pt, y: 4pt),
    baseline: 0%,
    text(size: 10pt, weight: "bold", fill: rgb("#ffffff"), label)
  )
}

#let code-window(title: "code", body) = {
  block(
    fill: rgb("#090d16"),
    stroke: 1pt + rgb("#1e293b"),
    radius: 8pt,
    inset: 0.8em,
    width: 100%,
    [
      #block(text(size: 10pt, fill: rgb("#94a3b8"), font: "monospace", [#("// " + title)]))
      #v(0.3em)
      #body
    ]
  )
}

#let chart(type: "bar", source: "", x: "", y: "", title: "", color: "#38bdf8") = {
  align(center)[
    #block(
      fill: rgb("#1e293b"),
      stroke: 1pt + rgb("#334155"),
      radius: 8pt,
      inset: 1em,
      width: 95%,
      [
        #text(size: 15pt, weight: "bold", fill: rgb("#f1f5f9"), title)
        #v(0.4em)
        #text(size: 12pt, fill: rgb("#94a3b8"), [Data Source: #source (X: #x -> Y: #y)])
      ]
    )
  ]
}

#let audio(source, autoplay: false, loop: false, volume: 0.5) = {
  // Audio playback hint for presentation runner
}
"###;

/// Writes `theme.typ`/`slide.typ` into `project_dir` if either is missing -- called before
/// creating or applying anything that imports them, so a slide deck compiles on the very first
/// try in any project, not only one that happened to run the demo seeder first.
pub async fn ensure_cargo_slide_helpers<P: AsRef<std::path::Path>>(project_dir: P) -> std::io::Result<()> {
    let dir = project_dir.as_ref();
    let theme_path = dir.join("theme.typ");
    if !theme_path.exists() {
        tokio::fs::write(&theme_path, THEME_TYP).await?;
    }
    let slide_path = dir.join("slide.typ");
    if !slide_path.exists() {
        tokio::fs::write(&slide_path, SLIDE_TYP).await?;
    }
    Ok(())
}
