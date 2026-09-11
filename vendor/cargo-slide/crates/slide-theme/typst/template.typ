#import "theme.typ": *

#show: slide-theme.with(
  aspect-ratio: "16-9",
  theme: "dark"
)

#title-slide(
  title: "Building Modern Slides with Rust & Typst",
  subtitle: "High Performance, Single Binary, Code-Driven Presentation",
  author: "Cargo Slide Team",
  date: "2026",
)

#slide(title: "Why Cargo Slide?")[
  #cols(
    [
      === The Traditional Pain Points
      - *Marp / Slidev*: Bulky Node.js ecosystem, flow-layout limitations.
      - *LaTeX Beamer*: Slow compile times, dated presentation UI.
      - *Typst to PDF*: Static, no native animations, broken video support.
    ],
    [
      === The Cargo Slide Solution
      #callout(title: "Modern Geek Stack")[
        - *Typst*: Blazing fast compile, gorgeous math & layout.
        - *Rust*: Native GUI runtime, 60 FPS vector rendering.
        - *Zero Dependency*: Standalone single binary output!
      ]
    ]
  )
]

#slide(title: "Mathematical Formulas in Typst")[
  Typst provides elegant, native math typesetting:

  $ F_mu nu = partial_mu A_nu - partial_nu A_mu $

  $ dif / (dif t) frac(partial L, partial dot(q)_i) - frac(partial L, partial q_i) = 0 $

  #v(0.5cm)
  #badge("Quantum Mechanics") #badge("Field Theory", fill: rgb("238636"))
  
  $ cal(H) |psi(t) chevron.r = i ħ dif / (dif t) |psi(t) chevron.r $
]

#slide(title: "Interactive Hyperlinks & Hotspots")[
  Typst SVGs preserve clickable bounding boxes natively parsed by Rust:

  - Jump to official Rust website: #link("https://www.rust-lang.org")[Rust Language Home]
  - Explore Typst documentation: #link("https://typst.app")[Typst Official App]
  - Jump back to first slide: #link("#page=1")[Return to Title Slide]

  #v(0.8cm)
  #callout(title: "Hotspot Engine")[
    Rust parses `<a xlink:href="...">` coordinates from SVG output and converts them to clickable screen-space interaction zones.
  ]
]

#slide(title: "Hardware Accelerated Video with FFmpeg")[
  #video("assets/demo.mp4", caption: "Deep Dive Architecture Video (Click to Play)")
]

#slide(title: "Thank You & Q&A")[
  #align(center + horizon)[
    #text(size: 28pt, weight: "bold", fill: slide-colors.accent)[Ready to create next-gen slides?]

    #v(1cm)
    #text(size: 16pt)[Run `cargo slide build` to export your standalone single binary.]
  ]
]
