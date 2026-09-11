// Base theme and styling for cargo-slide presentations
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
  
  let bg-color = if theme == "light" { rgb("f6f8fa") } else { rgb("0f111a") }
  let fg-color = if theme == "light" { rgb("1f2328") } else { rgb("e6edf3") }

  set page(
    width: page-width,
    height: page-height,
    margin: (x: 1.6cm, top: 0.9cm, bottom: 0.8cm),
    fill: bg-color,
    footer: context [
      #set text(size: 9pt, fill: rgb("8b949e"))
      #grid(
        columns: (1fr, 1fr),
        align: (left, right),
        [
          #text(weight: "bold")[cargo-slide]
        ],
        [
          #counter(page).display("1 / 1", both: true)
        ]
      )
    ]
  )

  let default-fonts = (
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
    "Liberation Sans",
    "DejaVu Sans",
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
  show link: it => text(fill: rgb("58a6ff"))[#it]

  // Raw code block styling
  show raw: set text(font: active-code-fonts)
  show raw.where(block: true): it => block(
    width: 100%,
    fill: rgb("161b22"),
    inset: 9pt,
    radius: 6pt,
    stroke: 1pt + rgb("30363d"),
    [
      #set text(size: 10pt)
      #it
    ]
  )

  body
}
