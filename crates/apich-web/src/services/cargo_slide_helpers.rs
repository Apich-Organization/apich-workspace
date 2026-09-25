//! Typst helper files for cargo-slide decks.
//!
//! Provides the `theme.typ` and `slide.typ` files imported by cargo-slide decks.
//! Extracted here as the single source of truth so demo projects, templates, and
//! project creation all provision identical helper files without drift.
//!
//! This content is cargo-slide's own official reference theme (vendored at
//! `vendor/cargo-slide/crates/slide-theme/typst/{theme,slide}.typ`), re-skinned with APICH's own
//! `slide-colors` palette (an `institution:` field was also added back to `title-slide`, which
//! upstream's version doesn't have, since existing content here already relied on it) rather than
//! a from-scratch reimplementation -- that earlier reimplementation was missing two things this
//! version gets for free by tracking upstream: a real `chart()` that renders actual bar/line/area
//! data (not a placeholder box), and, more importantly, the exact `pagebreak(weak: true)`-before-
//! each-slide and `link("slide-meta:" + title)[...]` marker pattern cargo-slide's own overflow
//! checker expects -- without the latter, that checker can't tell which physical page a real
//! overflow happened on and falls back to blindly blaming whichever slide is declared last,
//! confirmed live while chasing down a real, previously-undiscovered overflow bug this
//! reimplementation had introduced (a trailing blank page after the deck's last slide, on top of
//! never establishing a 16:9 page size at all for documents that don't explicitly invoke
//! `#show: slide-theme.with(...)`, which the app's own starter template never did either).

pub const THEME_TYP: &str = r##"// Base theme and styling for cargo-slide presentations
#import "slide.typ": *

#let slide-theme(
  aspect-ratio: "16-9",
  theme: "dark",
  font: none,
  code-font: none,
  body
) = {
  let page-width = 28cm
  let page-height = 15.75cm

  if aspect-ratio == "4-3" {
    page-width = 21cm
    page-height = 15.75cm
  } else if aspect-ratio == "16-10" {
    page-width = 25.2cm
    page-height = 15.75cm
  } else if aspect-ratio == "3-2" {
    page-width = 23.625cm
    page-height = 15.75cm
  }

  let bg-color = if theme == "light" { rgb("#ffffff") } else { slide-colors.bg }
  let fg-color = if theme == "light" { rgb("#0f172a") } else { slide-colors.fg }

  set page(
    width: page-width,
    height: page-height,
    margin: (x: 1.6cm, top: 0.9cm, bottom: 0.8cm),
    fill: bg-color,
    footer: context [
      #set text(size: 9pt, fill: slide-colors.secondary)
      #grid(
        columns: (1fr, 1fr),
        align: (left, right),
        [
          #text(weight: "bold")[APICH]
        ],
        [
          #counter(page).display("1 / 1", both: true)
        ]
      )
    ]
  )

  let default-fonts = (
    "Inter",
    "Roboto",
    "Noto Sans",
    "Segoe UI",
    "SF Pro Display",
    "SF Pro Text",
    "Helvetica Neue",
    "Cantarell",
    "Arial",
    "PingFang SC",
    "Microsoft YaHei",
    "Noto Sans CJK SC",
    "Source Han Sans SC",
    "WenQuanYi Micro Hei",
    "WenQuanYi Zen Hei",
    "PingFang TC",
    "Microsoft JhengHei",
    "Noto Sans CJK TC",
    "Noto Sans CJK HK",
    "Source Han Sans TC",
    "Hiragino Sans",
    "Meiryo",
    "Noto Sans CJK JP",
    "Source Han Sans JP",
    "Malgun Gothic",
    "NanumGothic",
    "Noto Sans CJK KR",
    "Source Han Sans KR",
    "Droid Sans Fallback",
    "Liberation Sans",
    "DejaVu Sans",
    "Noto Color Emoji",
    "Apple Color Emoji",
    "Segoe UI Emoji",
  )

  let active-fonts = if font != none {
    if type(font) == array { font }
    else { (font,) }
  } else {
    default-fonts
  }

  set text(
    font: active-fonts,
    size: 11.5pt,
    fill: fg-color,
  )

  let default-code-fonts = (
    "DejaVu Sans Mono",
    "Consolas",
    "SF Mono",
    "Cascadia Code",
    "Liberation Mono",
    "Menlo",
    "Courier New",
  )

  let active-code-fonts = if code-font != none {
    if type(code-font) == array { code-font }
    else { (code-font,) }
  } else {
    default-code-fonts
  }

  // Math formula styling
  show math.equation: set text(weight: "regular")

  // Link styling
  show link: it => text(fill: slide-colors.accent)[#it]

  // Raw code block styling
  show raw: set text(font: active-code-fonts)
  show raw.where(block: true): it => block(
    width: 100%,
    fill: slide-colors.card-bg,
    inset: 9pt,
    radius: 6pt,
    stroke: 1pt + slide-colors.card-border,
    [
      #set text(size: 10pt)
      #it
    ]
  )

  body
}
"##;

pub const SLIDE_TYP: &str = r##"// Slide macros and components for cargo-slide (re-skinned with APICH's own color palette;
// upstream reference: crates/slide-theme/typst/slide.typ)
//
// Two real, previously-undiscovered bugs this reference theme fixes over an earlier from-scratch
// reimplementation, both confirmed live against cargo-slide's own overflow checker (which compares
// the number of `#title-slide`/`#slide` calls it can detect in the source against the actual
// number of Typst-compiled pages, erroring if there are more pages than calls):
//
// 1. A trailing `pagebreak()` *after* each slide's own content leaves one blank page after the
//    deck's very last slide, since there's nothing left to fill the page it just broke to. Fixed
//    here by breaking *before* each `#slide` call instead (never before `#title-slide`, which is
//    always first and needs no break to a previous page) -- N slide calls then always produce
//    exactly N pages.
// 2. Without a `slide-meta:` marker embedded in each slide's own rendered output, cargo-slide
//    cannot tell which physical page an overflow actually happened on and falls back to blindly
//    blaming the *last* declared slide regardless of where the real problem is. Embedding this
//    marker makes that error message point at the real problem slide.
//
// Note `slide-theme` (theme.typ) is what calls `set page(...)`/`set text(...)`, invoked once via
// `#show: slide-theme.with(...)` at the top of a document (see `ensure_cargo_slide_helpers`'s doc
// comment and every starter/template this app provisions) -- a `set page` inside a function body
// only applies to *that call's own* content in Typst, not to what comes after.

#let slide-colors = (
  bg: rgb("#0f172a"),
  fg: rgb("#f8fafc"),
  accent: rgb("#2563eb"),
  accent-dark: rgb("#1d4ed8"),
  accent-cyan: rgb("#0284c7"),
  accent-purple: rgb("#7c3aed"),
  accent-orange: rgb("#d97706"),
  accent-red: rgb("#dc2626"),
  accent-yellow: rgb("#ca8a04"),
  secondary: rgb("#94a3b8"),
  card-bg: rgb("#1e293b"),
  card-border: rgb("#334155"),
  glass-bg: rgb(30, 41, 59, 85%),
)

/// Title slide macro
#let title-slide(title: "", subtitle: "", author: "", institution: "", date: "", version: "") = {
  link("slide-meta:" + if title != "" { title } else { "title-slide" })[#box(width: 0pt, height: 0pt)[]]
  set align(center + horizon)
  block(
    width: 100%,
    stroke: none,
    [
      #v(-0.6cm)
      #text(size: 32pt, weight: "bold", fill: slide-colors.accent)[#title]

      #if subtitle != "" [
        #v(0.4cm)
        #text(size: 16pt, fill: slide-colors.fg)[#subtitle]
      ]

      #v(0.8cm)
      #line(length: 30%, stroke: 1.5pt + slide-colors.card-border)
      #v(0.6cm)

      #grid(
        columns: (1fr, 1fr),
        align: (right, left),
        gutter: 1.5cm,
        [
          #if author != "" [
            #text(size: 12pt, fill: slide-colors.secondary)[Speaker: ]
            #text(size: 12pt, weight: "bold", fill: slide-colors.fg)[#author]
          ]
          #if institution != "" [
            #linebreak()
            #text(size: 10pt, fill: slide-colors.secondary)[#institution]
          ]
        ],
        [
          #if date != "" [
            #text(size: 12pt, fill: slide-colors.secondary)[Date: ]
            #text(size: 12pt, fill: slide-colors.fg)[#date]
          ]
        ]
      )
    ]
  )
}

/// Standard slide layout
#let slide(title: none, header: none, footer: none, transition: none, body) = {
  pagebreak(weak: true)

  link("slide-meta:" + if title != none { title } else { "slide" })[#box(width: 0pt, height: 0pt)[]]

  if transition != none {
    link("transition:" + transition)[#box(width: 0pt, height: 0pt)[]]
  }

  if title != none [
    #block(
      width: 100%,
      inset: (bottom: 0.35cm),
      stroke: (bottom: 1pt + slide-colors.card-border),
      [
        #text(size: 19pt, weight: "bold", fill: slide-colors.accent)[#title]
      ]
    )
    #v(0.12cm)
  ]

  body
}

/// In-slide step build / fragment reveal macro
#let step(order, effect: "fade-in", body) = {
  link("step:" + str(order) + "?effect=" + effect)[#body]
}
#let pause = step

/// Video placeholder component that hooks into Rust player's FFmpeg playback engine
#let video(
  source,
  caption: none,
  duration: none,
  quality: "4K 60FPS",
  style: "glass", // "glass", "cinema", "minimal"
  width: 90%,
  poster: none,
) = {
  let display-title = if caption != none { caption } else { source }

  align(center)[
    #link("video:" + source)[
      #if style == "cinema" [
        // Cinematic Letterbox Style with REC badge and dark frame
        #rect(
          width: width,
          fill: rgb("#08090d"),
          stroke: 1.5pt + slide-colors.accent-orange,
          radius: 8pt,
          inset: (x: 16pt, y: 14pt),
          [
            #grid(
              columns: (1fr, 1fr),
              align: (left, right),
              [
                #box(
                  fill: rgb(255, 30, 40, 20%),
                  stroke: 1pt + slide-colors.accent-red,
                  radius: 3pt,
                  inset: (x: 6pt, y: 2pt),
                  text(size: 8pt, weight: "bold", fill: slide-colors.accent-red)[● REC]
                )
              ],
              [
                #box(
                  fill: rgb(255, 255, 255, 10%),
                  radius: 3pt,
                  inset: (x: 6pt, y: 2pt),
                  text(size: 8pt, weight: "bold", fill: slide-colors.secondary)[#quality]
                )
                #if duration != none [
                  #h(6pt)
                  #box(
                    fill: rgb(255, 255, 255, 10%),
                    radius: 3pt,
                    inset: (x: 6pt, y: 2pt),
                    text(size: 8pt, fill: slide-colors.secondary)[⏱ #duration]
                  )
                ]
              ]
            )
            #v(10pt)
            #align(center)[
              #circle(
                radius: 20pt,
                fill: slide-colors.accent-orange,
                align(center + horizon)[
                  #text(size: 16pt, fill: rgb("#ffffff"))[▶]
                ]
              )
              #v(8pt)
              #text(size: 13pt, weight: "bold", fill: slide-colors.fg)[#display-title]
              #v(2pt)
              #text(size: 9.5pt, fill: slide-colors.secondary)[Click to launch FFmpeg hardware video player]
            ]
          ]
        )
      ] else if style == "minimal" [
        // Minimal horizontal pill card
        #rect(
          width: width,
          fill: slide-colors.card-bg,
          stroke: 1pt + slide-colors.card-border,
          radius: 6pt,
          inset: (x: 14pt, y: 10pt),
          [
            #grid(
              columns: (32pt, 1fr, auto),
              align: (left + horizon, left + horizon, right + horizon),
              gutter: 10pt,
              circle(
                radius: 14pt,
                fill: slide-colors.accent-dark,
                align(center + horizon)[#text(size: 12pt, fill: rgb("#ffffff"))[▶]]
              ),
              [
                #text(size: 12pt, weight: "bold", fill: slide-colors.fg)[#display-title] \
                #text(size: 9pt, fill: slide-colors.secondary)[Interactive Video Stream]
              ],
              [
                #box(
                  fill: slide-colors.card-border,
                  radius: 3pt,
                  inset: (x: 6pt, y: 3pt),
                  text(size: 8.5pt, weight: "bold", fill: slide-colors.accent)[PLAY]
                )
              ]
            )
          ]
        )
      ] else [
        // Default: Glassmorphic Tech Card
        #rect(
          width: width,
          fill: slide-colors.card-bg,
          stroke: 1.5pt + slide-colors.accent-dark,
          radius: 8pt,
          inset: (x: 16pt, y: 16pt),
          [
            #align(center)[
              #circle(
                radius: 22pt,
                fill: slide-colors.accent-dark,
                align(center + horizon)[
                  #text(size: 18pt, fill: rgb("#ffffff"))[▶]
                ]
              )
              #v(10pt)
              #text(size: 14pt, weight: "bold", fill: slide-colors.fg)[#display-title]
              #v(4pt)
              #text(size: 10pt, fill: slide-colors.secondary)[
                Hardware-accelerated presentation playback with FFmpeg
              ]
              #v(6pt)
              #grid(
                columns: (auto, auto),
                gutter: 8pt,
                align: center,
                box(
                  fill: rgb(31, 111, 235, 20%),
                  stroke: 0.8pt + slide-colors.accent-dark,
                  radius: 3pt,
                  inset: (x: 6pt, y: 2pt),
                  text(size: 8.5pt, weight: "bold", fill: slide-colors.accent)[#quality]
                ),
                if duration != none {
                  box(
                    fill: rgb(255, 255, 255, 10%),
                    radius: 3pt,
                    inset: (x: 6pt, y: 2pt),
                    text(size: 8.5pt, fill: slide-colors.secondary)[⏱ #duration]
                  )
                }
              )
            ]
          ]
        )
      ]
    ]
  ]
}

/// Hidden or compact audio trigger macro
#let audio(source, autoplay: true, loop: true, volume: 0.8) = {
  let vol-str = str(calc.clamp(volume, 0.0, 1.0))
  let auto-str = if autoplay { "true" } else { "false" }
  let loop-str = if loop { "true" } else { "false" }
  let uri = "audio:" + source + "?autoplay=" + auto-str + ";loop=" + loop-str + ";vol=" + vol-str
  link(uri)[#box(width: 1pt, height: 1pt, fill: none)[]]
}

/// Interactive Audio Player card component
#let audio-player(
  source,
  title: "Background Music",
  artist: "cargo-slide soundtrack",
  autoplay: false,
  loop: true,
  volume: 0.8,
  width: 100%,
) = {
  let vol-str = str(calc.clamp(volume, 0.0, 1.0))
  let auto-str = if autoplay { "true" } else { "false" }
  let loop-str = if loop { "true" } else { "false" }
  let uri = "audio:" + source + "?autoplay=" + auto-str + ";loop=" + loop-str + ";vol=" + vol-str
  let vol-pct = str(calc.round(volume * 100)) + "%"

  link(uri)[
    #rect(
      width: width,
      fill: slide-colors.card-bg,
      stroke: 1.2pt + slide-colors.accent-cyan,
      radius: 8pt,
      inset: (x: 14pt, y: 10pt),
      [
        #grid(
          columns: (30pt, 1fr, auto),
          gutter: 12pt,
          align: (center + horizon, left + horizon, right + horizon),
          circle(
            radius: 15pt,
            fill: rgb(2, 132, 199, 25%),
            stroke: 1pt + slide-colors.accent-cyan,
            align(center + horizon)[#text(size: 13pt, fill: slide-colors.accent-cyan)[♫]]
          ),
          [
            #text(size: 12pt, weight: "bold", fill: slide-colors.fg)[#title] \
            #text(size: 9.5pt, fill: slide-colors.secondary)[#artist]
          ],
          [
            #grid(
              columns: (auto, auto, auto),
              gutter: 6pt,
              align: horizon,
              // Equalizer wave simulation bars
              stack(
                dir: ltr,
                spacing: 2pt,
                rect(width: 2.5pt, height: 7pt, fill: slide-colors.accent-cyan, radius: 1pt),
                rect(width: 2.5pt, height: 13pt, fill: slide-colors.accent-cyan, radius: 1pt),
                rect(width: 2.5pt, height: 9pt, fill: slide-colors.accent-cyan, radius: 1pt),
                rect(width: 2.5pt, height: 15pt, fill: slide-colors.accent-cyan, radius: 1pt),
                rect(width: 2.5pt, height: 6pt, fill: slide-colors.accent-cyan, radius: 1pt),
              ),
              box(
                fill: rgb(2, 132, 199, 15%),
                radius: 3pt,
                inset: (x: 5pt, y: 2pt),
                text(size: 8.5pt, weight: "bold", fill: slide-colors.accent-cyan)[🔊 #vol-pct]
              ),
              if loop [
                #box(
                  fill: rgb(255, 255, 255, 10%),
                  radius: 3pt,
                  inset: (x: 5pt, y: 2pt),
                  text(size: 8.5pt, fill: slide-colors.secondary)[🔁 Loop]
                )
              ]
            )
          ]
        )
      ]
    )
  ]
}

/// Interactive Chart component supporting CSV, JSON, SQLite databases, SQL queries, and concise DSL pipelines
#let chart(
  type: "bar", // "bar", "line", "area", "pie", "donut", "scatter"
  title: none,
  source: none, // e.g. "data.csv", "metrics.json", or "data.db?query=SELECT ..."
  sql: none,    // e.g. "SELECT category, sales FROM data WHERE sales > 100"
  dsl: none,    // e.g. "filter sales > 100 | sort desc | limit 5 | smooth 3"
  format: none, // "currency", "percentage", "compact", "scientific", "integer", "standard"
  unit: none,   // e.g. "USD", "EUR", "k ops/s", "MB", "%"
  prefix: none, // e.g. "$", "¥"
  precision: none, // e.g. 0, 1, 2
  data: none,   // e.g. (categories: ("Q1", "Q2"), series: ((name: "Rev", values: (100, 200)), ...))
  width: 100%,
  height: 220pt,
  colors: (slide-colors.accent-cyan, slide-colors.accent-cyan, slide-colors.accent-orange, slide-colors.accent-red, slide-colors.accent-purple, slide-colors.accent),
) = {
  // Resolve source: if it refers to SQLite (.db or .sqlite) or JSON (.json or .jsonl), load the preprocessed .cache.csv
  let resolved-source = if source != none and (source.contains(".db") or source.contains(".sqlite") or source.contains(".json") or source.contains(".jsonl")) {
    let clean-path = source.split("?").at(0)
    clean-path + ".cache.csv"
  } else {
    source
  }

  // Load categories and series
  let categories = ()
  let series = ()

  if resolved-source != none {
    let raw-csv = csv(resolved-source)
    if raw-csv.len() > 0 {
      let headers = raw-csv.at(0)
      let series-count = headers.len() - 1
      for i in range(1, headers.len()) {
        series.push((name: headers.at(i), values: ()))
      }
      for row-idx in range(1, raw-csv.len()) {
        let row = raw-csv.at(row-idx)
        if row.len() > 0 {
          categories.push(row.at(0))
          for s-idx in range(0, series-count) {
            let raw-val = if row.len() > s-idx + 1 { row.at(s-idx + 1) } else { "0" }
            let num-val = float(raw-val.trim().trim("$").trim("€").trim("%").replace(",", ""))
            series.at(s-idx).values.push(num-val)
          }
        }
      }
    }
  } else if data != none {
    if "categories" in data {
      categories = data.categories
    }
    if "series" in data {
      series = data.series
    }
  }

  // Calculate min, max for scaling
  let max-val = 1.0
  let min-val = 0.0
  for s in series {
    for v in s.values {
      let fv = float(v)
      if fv > max-val { max-val = fv }
      if fv < min-val { min-val = fv }
    }
  }
  if max-val <= 0.0 { max-val = 1.0 }

  // Build chart link target with metadata for Rust slide-player interactive HUD
  // Prefix with '#' so PDF and preview viewers treat it as an in-slide anchor rather than an external link
  let link-target = "#chart:type=" + type
  if title != none {
    let clean-title = title.replace("&", "%26").replace(" ", "+")
    link-target += "&title=" + clean-title
  }
  if source != none {
    let clean-source = source.replace("&", "%26").replace(" ", "+")
    link-target += "&source=" + clean-source
  }
  if sql != none {
    let clean-sql = sql.replace("&", "%26").replace(" ", "+")
    link-target += "&sql=" + clean-sql
  }
  if dsl != none {
    let clean-dsl = dsl.replace("&", "%26").replace(" ", "+")
    link-target += "&dsl=" + clean-dsl
  }
  if categories.len() > 0 {
    let clean-cats = categories.map(c => c.replace("&", "%26").replace(" ", "+"))
    link-target += "&categories=" + clean-cats.join(",")
  }
  if series.len() > 0 {
    let s-strings = ()
    for s in series {
      let v-strs = s.values.map(str)
      let clean-name = s.name.replace("&", "%26").replace(" ", "+")
      s-strings.push(clean-name + ":" + v-strs.join(","))
    }
    link-target += "&series=" + s-strings.join(";")
  }
  if format != none {
    link-target += "&format=" + str(format).replace("&", "%26").replace(" ", "+")
  }
  if unit != none {
    link-target += "&unit=" + str(unit).replace("&", "%26").replace(" ", "+")
  }
  if prefix != none {
    link-target += "&prefix=" + str(prefix).replace("&", "%26").replace(" ", "+")
  }
  if precision != none {
    link-target += "&precision=" + str(precision)
  }

  align(center)[
    #link(link-target)[
      #rect(
        width: width,
        height: height,
        fill: slide-colors.card-bg,
        stroke: 1pt + slide-colors.card-border,
        radius: 8pt,
        inset: (x: 16pt, y: 12pt),
        [
          #grid(
            columns: (1fr, auto),
            align: (left + top, right + top),
            [
              #if title != none [
                #text(size: 11pt, weight: "bold", fill: slide-colors.accent)[#title]
              ]
            ],
            [
              #stack(
                dir: ltr,
                spacing: 10pt,
                ..series.enumerate().map(it => {
                  let (idx, s) = it
                  let col = colors.at(calc.rem(idx, colors.len()))
                  stack(
                    dir: ltr,
                    spacing: 4pt,
                    box(width: 7pt, height: 7pt, fill: col, radius: 1.5pt),
                    text(size: 8pt, fill: slide-colors.secondary)[#s.name]
                  )
                })
              )
            ]
          )

          #v(8pt)

          #layout(size => {
            let plot-w = size.width - 40pt
            let plot-h = height - 60pt
            let cat-count = calc.max(categories.len(), 1)
            let col-w = plot-w / cat-count

            let grid-lines = ()
            for tick-idx in range(0, 5) {
              let t = tick-idx / 4.0
              let tick-y = (1.0 - t) * plot-h
              let tick-val = min-val + t * (max-val - min-val)
              let tick-label = if tick-val >= 1000000.0 {
                str(calc.round(tick-val / 1000000.0, digits: 1)) + "M"
              } else if tick-val >= 1000.0 {
                str(calc.round(tick-val / 1000.0, digits: 0)) + "K"
              } else {
                str(calc.round(tick-val, digits: 0))
              }

              grid-lines.push(
                place(top + left, dx: 0pt, dy: tick-y)[
                  #line(start: (30pt, 0pt), end: (plot-w + 30pt, 0pt), stroke: (paint: slide-colors.card-border, dash: "dashed", thickness: 0.8pt))
                ]
              )
              grid-lines.push(
                place(top + left, dx: 0pt, dy: tick-y - 4pt)[
                  #box(width: 25pt, align(right)[#text(size: 7.5pt, fill: slide-colors.secondary)[#tick-label]])
                ]
              )
            }

            let chart-elements = ()

            if type == "bar" {
              let series-count = calc.max(series.len(), 1)
              let group-pad = col-w * 0.15
              let bar-area-w = col-w - group-pad * 2.0
              let single-w = bar-area-w / series-count

              for (c-idx, cat) in categories.enumerate() {
                let cat-x = 30pt + c-idx * col-w + group-pad

                chart-elements.push(
                  place(top + left, dx: 30pt + c-idx * col-w, dy: plot-h + 4pt)[
                    #box(width: col-w, align(center)[#text(size: 8pt, fill: slide-colors.secondary)[#cat]])
                  ]
                )

                for (s-idx, s) in series.enumerate() {
                  let val = if c-idx < s.values.len() { s.values.at(c-idx) } else { 0.0 }
                  let bar-h = ((val - min-val) / calc.max(max-val - min-val, 0.001)) * plot-h
                  let bx = cat-x + s-idx * single-w
                  let by = plot-h - bar-h
                  let col = colors.at(calc.rem(s-idx, colors.len()))

                  chart-elements.push(
                    place(top + left, dx: bx, dy: by)[
                      #rect(
                        width: calc.max(single-w - 2pt, 2pt),
                        height: calc.max(bar-h, 1pt),
                        fill: col,
                        radius: (top: 2pt)
                      )
                    ]
                  )
                }
              }
            } else if type == "line" or type == "area" {
              for (c-idx, cat) in categories.enumerate() {
                chart-elements.push(
                  place(top + left, dx: 30pt + c-idx * col-w, dy: plot-h + 4pt)[
                    #box(width: col-w, align(center)[#text(size: 8pt, fill: slide-colors.secondary)[#cat]])
                  ]
                )
              }

              for (s-idx, s) in series.enumerate() {
                let col = colors.at(calc.rem(s-idx, colors.len()))
                let prev-pt = none

                for (c-idx, val) in s.values.enumerate() {
                  let px = 30pt + (c-idx + 0.5) * col-w
                  let py = plot-h - ((val - min-val) / calc.max(max-val - min-val, 0.001)) * plot-h

                  if prev-pt != none {
                    chart-elements.push(
                      place(top + left)[
                        #line(start: prev-pt, end: (px, py), stroke: 2pt + col)
                      ]
                    )
                  }
                  prev-pt = (px, py)

                  chart-elements.push(
                    place(top + left, dx: px - 3pt, dy: py - 3pt)[
                      #circle(radius: 3pt, fill: col, stroke: 1.5pt + slide-colors.card-bg)
                    ]
                  )
                }
              }
            } else {
              chart-elements.push(
                place(top + left, dx: 30pt, dy: plot-h / 2)[
                  #text(size: 10pt, fill: slide-colors.secondary)[Interactive Chart (#type)]
                ]
              )
            }

            stack(
              ..grid-lines,
              ..chart-elements,
            )
          })
        ]
      )
    ]
  ]
}

/// Two-column layout helper
#let cols(left, right, ratio: (1fr, 1fr)) = {
  grid(
    columns: ratio,
    gutter: 1.2cm,
    left,
    right
  )
}

/// Tech badge
#let badge(label, fill: slide-colors.accent-dark, text-color: rgb("#ffffff")) = {
  box(
    fill: fill,
    radius: 4pt,
    inset: (x: 7pt, y: 3.5pt),
    baseline: 0%,
    text(size: 9.5pt, weight: "bold", fill: text-color)[#label]
  )
}

/// Callout box
#let callout(title: none, body, stroke-color: slide-colors.accent) = {
  rect(
    width: 100%,
    fill: slide-colors.card-bg,
    stroke: (left: 3.5pt + stroke-color, rest: 1pt + slide-colors.card-border),
    radius: (right: 6pt),
    inset: 10pt,
    [
      #if title != none [
        #block(inset: (bottom: 4pt))[
          #text(weight: "bold", size: 11pt, fill: stroke-color)[#title]
        ]
      ]
      #body
    ]
  )
}

/// Modern code editor window mockup with traffic lights
#let code-window(title: "main.rs", body) = {
  rect(
    width: 100%,
    fill: slide-colors.card-bg,
    stroke: 1pt + slide-colors.card-border,
    radius: 6pt,
    inset: 0pt,
    [
      // Window titlebar
      #rect(
        width: 100%,
        fill: rgb("#12161c"),
        stroke: (bottom: 1pt + slide-colors.card-border),
        inset: (x: 10pt, y: 6pt),
        [
          #grid(
            columns: (auto, 1fr),
            align: (left + horizon, center + horizon),
            [
              #stack(
                dir: ltr,
                spacing: 5pt,
                circle(radius: 4pt, fill: rgb("#ff5f56")),
                circle(radius: 4pt, fill: rgb("#ffbd2e")),
                circle(radius: 4pt, fill: rgb("#27c93f")),
              )
            ],
            [
              #text(size: 9pt, fill: slide-colors.secondary, font: "DejaVu Sans Mono")[#title]
            ]
          )
        ]
      )
      #block(inset: 10pt)[#body]
    ]
  )
}

/// High-impact metric card for stats & benchmarks
#let metric(value, label, change: none, color: slide-colors.accent) = {
  rect(
    fill: slide-colors.card-bg,
    stroke: 1pt + slide-colors.card-border,
    radius: 6pt,
    inset: 10pt,
    width: 100%,
    [
      #text(size: 22pt, weight: "bold", fill: color)[#value]
      #v(2pt)
      #text(size: 10.5pt, fill: slide-colors.fg)[#label]
      #if change != none [
        #v(3pt)
        #text(size: 9pt, weight: "bold", fill: slide-colors.accent-cyan)[#change]
      ]
    ]
  )
}
"##;

/// Writes `theme.typ`/`slide.typ` into `project_dir` if either is missing -- called before
/// creating or applying anything that imports them, so a slide deck compiles on the very first
/// try in any project, not only one that happened to run the demo seeder first.
pub async fn ensure_cargo_slide_helpers<P: AsRef<std::path::Path>>(
    project_dir: P
) -> std::io::Result<()> {
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
