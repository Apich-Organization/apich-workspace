#import "theme.typ": *

#show: slide-theme.with(
  aspect-ratio: "16-9",
  theme: "dark"
)

// Slide 1: Title Slide with Autoplay Ambient Track
#title-slide(
  title: "Building Modern Slides with Rust & Typst",
  subtitle: "Zero-JS Native Engine • 60 FPS Vector Graphics • Hardware Media • Interactive Charts",
  author: "Cargo Slide Team",
  date: "2026",
)
#audio("assets/ambient.wav", autoplay: true, loop: true, volume: 0.5)

// Slide 2: Architecture & Vector Graphics Pipeline (Transition: Slide-Left)
#slide(title: "Architecture: Typst Functional Markup to Native Engine", transition: "slide-left")[
  #align(center)[#image("assets/architecture.svg", height: 4.4cm)]
  
  #v(0.2cm)
  #grid(
    columns: (1fr, 1fr, 1fr),
    gutter: 0.8cm,
    callout(title: "Typst Functional Markup", stroke-color: slide-colors.accent)[
      Instant sub-millisecond compilation, vectorized SVG DOM, and crisp rendering at any DPI without browser overhead.
    ],
    callout(title: "Rust Core Blitter", stroke-color: slide-colors.accent-orange)[
      tiny-skia 32-bit ARGB software blitter, zero GPU driver dependencies, and locked 60 FPS swapchain on all platforms.
    ],
    callout(title: "Multi-Threaded Audio DSP", stroke-color: slide-colors.accent-cyan)[
      Rodio audio mixing thread with volume curve fades, seamless loop playback, and floating HUD volume feedback.
    ]
  )
]

// Slide 3: In-Slide Component Animations & Fragment Builds (Transition: Iris)
#slide(title: "In-Slide Component Animations & Fragment Builds", transition: "iris")[
  Press *Space* / *Left-Click* to reveal each component sequentially (or *Backspace* / *Right-Click* to step back):

  #v(0.25cm)
  #step(1, effect: "fade-in")[
    #callout(title: "Step 1: Compiler Pipeline (#step(1, effect: 'fade-in'))", stroke-color: slide-colors.accent)[
      Typst source compiles into vectorized SVG DOM. Hotspots, internal jumps, media queries, and component fragments are automatically indexed.
    ]
  ]

  #v(0.25cm)
  #step(2, effect: "slide-up")[
    #callout(title: "Step 2: Directional Slide-Up (#step(2, effect: 'slide-up'))", stroke-color: slide-colors.accent-cyan)[
      Smooth cubic interpolation smoothly translates component into place with progressive alpha reveal.
    ]
  ]

  #v(0.25cm)
  #step(3, effect: "glitch")[
    #callout(title: "Step 3: Cyberpunk RGB Glitch (#step(3, effect: 'glitch'))", stroke-color: slide-colors.accent-purple)[
      Zero-overhead scanline jitter and RGB channel chromatic aberration for impactful emphasis and dramatic reveals.
    ]
  ]
]

// Slide 4: Rich Typography & LaTeX Mathematics (Transition: Zoom)
#slide(title: "Mathematical Typography & Code Syntax Highlighting", transition: "zoom")[
  #cols(
    [
      === Quantum Field & Wave Equations
      Typst renders complex LaTeX-grade mathematics natively:

      $ cal(H) |psi(t) chevron.r = i ħ dif / (dif t) |psi(t) chevron.r $

      $ dif / (dif t) frac(partial cal(L), partial dot(q)_i) - frac(partial cal(L), partial q_i) = 0 $

      #v(0.1cm)
      #badge("中文 / 日語 / 한국어", fill: slide-colors.accent-purple) #h(4pt)
      #badge("LaTeX Math", fill: slide-colors.accent) #h(4pt)
      #badge("A11y Ready ♿", fill: slide-colors.accent-cyan)
    ],
    [
      === Native Code Window Mockup
      #code-window(title: "pipeline.rs")[
        #set text(size: 9pt)
        ```rust
        pub fn render(deck: &SlideDeck) -> Result<()> {
            let player = SlidePlayer::new(deck)
                .with_audio("ambient.wav")
                .with_laser_pointer(true);
            player.run()
        }
        ```
      ]
    ]
  )

  #v(0.15cm)
  #callout(title: "Vectorized Multilingual Glyphs & Keyboard Accessibility", stroke-color: slide-colors.accent-cyan)[
    CJK scripts (中文, 日本語, 한국어) and Emojis (🚀 📊 🎨) vectorize into crisp resolution-independent curves with zero font installation dependencies. Full keyboard navigation (*Tab*, *Enter*, *Arrows*) and high-contrast rendering.
  ]
]

// Slide 5: 13 Built-in Page Transitions & Transition Gallery (Transition: Glitch)
#slide(title: "13 Built-in Page Transitions & Visual Effects", transition: "glitch")[
  Every slide can define its own transition via `#slide(transition: "...")`:

  #v(0.2cm)
  #grid(
    columns: (1fr, 1fr, 1fr, 1fr),
    gutter: 0.6cm,
    metric("fade", "Cross Fade", change: "Linear Alpha", color: slide-colors.accent),
    metric("wipe", "Directional Wipe", change: "Spatial Reveal", color: slide-colors.accent-cyan),
    metric("glitch", "Cyber Glitch", change: "RGB Channel Shift", color: slide-colors.accent-red),
    metric("cube", "3D Rotation", change: "Perspective Turn", color: slide-colors.accent-orange),
  )

  #v(0.3cm)
  #callout(title: "13 Built-in Transitions Ready Out-of-the-Box", stroke-color: slide-colors.accent-purple)[
    `fade`, `cut`, `slide-left`, `slide-right`, `slide-up`, `slide-down`, `zoom`, `wipe-left`, `wipe-right`, `iris`, `glitch`, `cube`, `particles`. Each transition executes at a locked 60 FPS directly on the 32-bit ARGB frame buffer with zero GPU dependencies.
  ]
]

// Slide 6: Extensible SlideTransition & ComponentAnimation Traits (Transition: Cube)
#slide(title: "Extensible Transition & Component Animation Traits", transition: "cube")[
  Implement custom transitions and fragment reveals with Rust traits:

  #v(0.15cm)
  #cols(
    [
      #code-window(title: "slide_transition.rs")[
        #set text(size: 9.5pt)
        ```rust
        pub trait SlideTransition: Send + Sync {
            fn render(&self, from: &[u32], to: &[u32],
                      out: &mut [u32], w: usize, h: usize, t: f32);
        }
        ```
      ]
    ],
    [
      #code-window(title: "component_animation.rs")[
        #set text(size: 9.5pt)
        ```rust
        pub trait ComponentAnimation: Send + Sync {
            fn render(&self, buffer: &mut [u32],
                      w: usize, h: usize, bbox: Rect, t: f32);
        }
        ```
      ]
    ]
  )

  #v(0.2cm)
  #callout(title: "Zero-Allocation Buffer Transforms", stroke-color: slide-colors.accent-cyan)[
    Register custom transitions via `SlideApp::register_transition("name", MyTransition)`. The player provides raw pixel buffer access and normalized progress `t ∈ [0.0, 1.0]`.
  ]
]

// Slide 7: Interactive Hyperlinks & Multi-target Navigation (Transition: Wipe-Left)
#slide(title: "Interactive Hyperlinks & Multi-Target Document Jumps", transition: "wipe-left")[
  Click any link below to test real-time coordinate mapping and external resolvers:

  #v(0.3cm)
  #grid(
    columns: (1fr, 1fr, 1fr),
    gutter: 0.8cm,
    [
      #callout(title: "Internal Slide Jump", stroke-color: slide-colors.accent)[
        Click to jump directly back to slide 2: \
        #v(4pt)
        #link("#page=2")[*➔ Jump to Slide 2 (Architecture)*]
      ]
    ],
    [
      #callout(title: "External Web URL", stroke-color: slide-colors.accent-cyan)[
        Click to open external URL in browser: \
        #v(4pt)
        #link("https://github.com/typst/typst")[*🌐 Open Typst Repository*]
      ]
    ],
    [
      #callout(title: "Local File Resolver", stroke-color: slide-colors.accent-orange)[
        Click to open local markdown file: \
        #v(4pt)
        #link("README.md")[*📄 Open README.md*]
      ]
    ]
  )

  #v(0.3cm)
  #callout(title: "Pixel-Perfect Coordinate Mapping", stroke-color: slide-colors.accent-purple)[
    All hotspot boundaries are transformed with 72 DPI PostScript precision and ancestor affine transforms, guaranteeing exact click registration across any window resolution and letterbox aspect ratios.
  ]
]

// Slide 8: Animated Interactive Hotspots & Glowing Feedback (Transition: Fade)
#slide(title: "Smooth Animated Hotspots & Interactive Feedback", transition: "fade")[
  Hotspot hovering features smooth 60 FPS interpolation and zero premature pops:

  #v(0.25cm)
  #cols(
    callout(title: "Smooth 60 FPS Alpha Interpolation", stroke-color: slide-colors.accent)[
      - *Exponential Lerp*: Smooth ~80ms fade-in and ~100ms fade-out.
      - *Zero-Popping*: Hover highlights are suppressed during page transitions and cleared immediately on slide switch.
    ],
    callout(title: "Translucent Tint & Outer Glow Halo", stroke-color: slide-colors.accent-cyan)[
      - *Visual Feedback*: Subtle interior card tint with glowing boundary edge.
      - *Pointer Morph*: Automatically changes system cursor to `PointingHand`.
    ]
  )

  #v(0.25cm)
  #align(center)[
    #box(stroke: 1pt + slide-colors.card-border, radius: 6pt, inset: 10pt, fill: slide-colors.card-bg)[
      #text(weight: "bold", fill: slide-colors.fg)[Hover over quick navigation anchors: ] #h(8pt)
      #link("#page=1")[*⏮ Cover*] #h(14pt)
      #link("#page=3")[*⚡ Builds*] #h(14pt)
      #link("#page=5")[*✨ Transitions*] #h(14pt)
      #link("#page=13")[*📊 CSV Data*] #h(14pt)
      #link("#page=15")[*🗄 SQLite DB*] #h(14pt)
      #link("#page=16")[*📈 KPI Analytics*]
    ]
  ]
]

// Slide 9: Interactive Audio Engine & Realtime Volume (Transition: Slide-Right)
#slide(title: "Multi-Channel Audio Engine & Dynamic Volume Controls", transition: "slide-right")[
  Cargo Slide features a synchronous audio engine with multi-channel mixing:

  #v(0.2cm)
  #audio-player(
    "assets/ambient.wav",
    title: "Cyberpunk Ambient Pad (44.1kHz Stereo)",
    artist: "Synth Arpeggio • Seamless Loop",
    loop: true,
    volume: 0.75,
  )

  #v(0.2cm)
  #audio-player(
    "assets/chime.wav",
    title: "Notification Ping (One-Shot Chime)",
    artist: "SFX Sound Generator",
    loop: false,
    volume: 0.9,
  )

  #v(0.25cm)
  #callout(title: "Flexible Volume Adjustments", stroke-color: slide-colors.accent-cyan)[
    - *Mouse Scroll Wheel*: Scroll wheel adjusts volume up/down by ±5% anywhere on screen. Clamped to [0%, 100%].
    - *Interactive Dock Slider*: Hover or click the *VOL* button on the Dock to drag the volume slider.
    - *On-Screen Volume Toast*: Modern HUD card pops up with real-time percentage and level meter.
  ]
]

// Slide 10: Video Placeholders (Transition: Slide-Down)
#slide(title: "Hardware Video Cards (Glassmorphism & Cinematic)", transition: "slide-down")[
  #cols(
    [
      === Glassmorphism Style
      #video(
        "assets/demo.mp4",
        caption: "Microservices Architecture Breakdown",
        duration: "04:12",
        quality: "4K 60FPS",
        style: "glass",
      )
    ],
    [
      === Cinematic Letterbox Style
      #video(
        "assets/demo.mp4",
        caption: "Deep Learning Neural Inference Demo",
        duration: "02:35",
        quality: "HDR 10-Bit",
        style: "cinema",
      )
    ]
  )

  #v(0.2cm)
  #callout(title: "Native Media Player Integration", stroke-color: slide-colors.accent-purple)[
    Clicking video cards launches hardware-accelerated playback with zero latency and automatic aspect-ratio preservation.
  ]
]

// Slide 11: Presenter Tools: Laser Trail, Color Palette & Dock (Transition: Glitch)
#slide(title: "Presenter Interaction Tools: Laser Trail, Palette & Whiteboard", transition: "glitch")[
  #cols(
    [
      === Phosphorescent Laser Pointer
      - Press `L` or click *LSR* on Dock.
      - Realistic motion blur: recent positions leave a glowing luminous tail.
      - Central white-hot plasma core with customizable color halo.

      === Floating 7-Color Palette
      - Press `K` or click *COL* on Dock to open floating palette.
      - Press keys `1` .. `7` to switch colors on the fly:
        #text(fill: rgb("00e5ff"))[Cyan], #text(fill: rgb("ff3366"))[Red], #text(fill: rgb("00e676"))[Green], #text(fill: rgb("ffd600"))[Yellow], #text(fill: rgb("d500f9"))[Purple], #text(fill: rgb("ffffff"))[White], #text(fill: rgb("ff9100"))[Orange].
    ],
    [
      === Whiteboard Ink Pen & Freehand Annotation
      - Press `P` or click *PEN* on Dock to enter whiteboard drawing mode.
      - Draw annotations directly over slides with smooth Bézier line blending.
      - Press `C` or `X` to clear annotations for the current slide.

      === Touchscreen Dock Navigation
      - Floating dock at screen bottom with touch-friendly navigation buttons.
      - Next/Prev buttons, slide jump input, volume slider, and tool toggles.
    ]
  )
]

// Slide 12: Dual Window Modes & Presenter Controls (Transition: Wipe-Right)
#slide(title: "Dual Window Modes & Comprehensive Presenter Controls", transition: "wipe-right")[
  #cols(
    [
      === Dual Presentation Modes
      - *Borderless Fullscreen*: Press `F11` / `F` or click *FULL* on Dock.
      - *Windowed Mode*: Press `F11` / `F` or click *WIN* on Dock.
      - Pixel-perfect 16:9 canvas with automatic letterboxing on any monitor.

      === Remote Clicker Compatibility
      - Wireless clickers work out-of-the-box (PageUp / PageDown / B / F5).
      - Left-Click advances step or slide; Right-Click reverses step or slide.
    ],
    [
      === Complete Keyboard Cheat Sheet
      #table(
        columns: (auto, 1fr),
        stroke: 0.5pt + slide-colors.card-border,
        fill: (col, row) => if calc.even(row) { slide-colors.card-bg } else { rgb("12161c") },
        inset: 5pt,
        [*Space / Left-Click*], [Next step / Next slide],
        [*Backspace / Right-Click*], [Prev step / Prev slide],
        [*F11 / F*], [Toggle Fullscreen / Windowed],
        [*L / P / K*], [Laser / Whiteboard Pen / Palette],
        [*1 .. 7*], [Select Pen & Laser Color],
        [*C / X*], [Clear Whiteboard Strokes],
        [*Wheel / M*], [Adjust Master Volume / Mute],
        [*H / ?*], [Show Interactive Help HUD],
      )
    ]
  )
]

// Slide 13: Interactive Charts: CSV Benchmarks & Interactive Crosshairs (Transition: Slide-Left)
#slide(title: "Interactive Charts: CSV Benchmarks & Inspector", transition: "slide-left")[
  Native chart rendering directly from standard CSV files with interactive crosshairs:

  #chart(
    type: "bar",
    title: "Presentation Engine Benchmark Comparison (Lower is Better)",
    source: "assets/benchmarks.csv",
    height: 120pt,
  )

  #v(0.12cm)
  #cols(
    callout(title: "Vector Chart Blitting", stroke-color: slide-colors.accent)[
      Charts are compiled as crisp SVG vector elements. Interactive scrubbing tracks cursor across categories with real-time value tooltips.
    ],
    callout(title: "HUD Data Inspector", stroke-color: slide-colors.accent-cyan)[
      Click *[DATA]* or press `D` on any chart to open the full HUD Data Inspector: view raw values, summary KPIs, and export to CSV.
    ]
  )
]

// Slide 14: Interactive Charts: JSON Datasets & Concise Pipeline DSL (Transition: Fade)
#slide(title: "Interactive Charts: JSON Datasets & Concise Pipeline DSL", transition: "fade")[
  Ingest JSON/JSONL with concise pipeline transformations: `#chart(source: "...", dsl: "filter ... | sort desc | limit 5")`:

  #chart(
    type: "bar",
    title: "Microservice Throughput (RPS) — Filtered & Sorted via Pipeline DSL",
    source: "assets/metrics.json",
    dsl: "filter throughput > 8000 | sort desc | limit 5",
    height: 120pt,
  )

  #v(0.12cm)
  #cols(
    callout(title: "Concise Pipeline DSL Syntax", stroke-color: slide-colors.accent)[
      Chain readable operations: `filter <col> > <val> | sort [asc|desc] | limit <N> | smooth <W> | cumulative | percent`.
    ],
    callout(title: "Live Analytical Transforms in HUD", stroke-color: slide-colors.accent-cyan)[
      Click *[DATA]* to open Inspector: toggle presets `[TOP 5]`, `[SORT ▼]`, `[100% SHARE]`, `[CUMULATIVE]`, `[SMOOTH]`, or press *S*, *T*, *P*, *U*, *M*!
    ]
  )
]

// Slide 15: Interactive Charts: Native SQLite & In-Memory SQL Queries (Transition: Cube)
#slide(title: "Interactive Charts: Native SQLite & Programmable In-Memory SQL", transition: "cube")[
  Execute SQL queries directly on local databases or in-memory tables: `sql: "SELECT ... FROM data WHERE ..."`:

  #chart(
    type: "line",
    title: "Internal Telemetry Latencies (ms) via SQL Query",
    source: "assets/telemetry.db?query=SELECT stage, p50, p95, p99 FROM latency_stats",
    sql: "SELECT stage, p50, p95, p99 FROM latency_stats",
    height: 120pt,
  )

  #v(0.12cm)
  #cols(
    callout(title: "Programmable In-Memory SQL", stroke-color: slide-colors.accent-purple)[
      Every CSV or JSON dataset can be queried with standard SQL (`SELECT category, col1 + col2 AS total FROM data WHERE ...`).
    ],
    callout(title: "Full Interactivity & Crosshairs", stroke-color: slide-colors.accent-orange)[
      Scrub mouse for vertical guidelines & multi-series halos. Cycle chart types with *[BAR]*, inspect KPIs, or export to CSV.
    ]
  )
]

// Slide 16: Interactive Charts: Custom Formats, Live Search & Statistical KPIs (Transition: Slide-Left)
#slide(title: "Interactive Charts: Custom Formats, Live Search & Statistical KPIs", transition: "slide-left")[
  Live presentation interactivity with dynamic filtering, statistical summaries, and formatting:

  #chart(
    type: "bar",
    title: "Cloud Infrastructure Cost & Efficiency Analytics (2024)",
    format: "currency",
    unit: "USD",
    precision: 0,
    data: (
      categories: ("US-East", "US-West", "EU-Central", "AP-East", "AP-South", "SA-East"),
      series: (
        (name: "Monthly Spend", values: (12400, 9800, 15600, 8200, 6400, 4300)),
        (name: "Optimized Savings", values: (3800, 2900, 5100, 2400, 1900, 1100)),
      ),
    ),
    height: 110pt,
  )

  #v(0.1cm)
  #grid(
    columns: (1fr, 1fr, 1fr),
    gutter: 0.6cm,
    callout(title: "Custom Formats & Units", stroke-color: slide-colors.accent)[
      - *Format Enum*: Currency (`$`), Percent (`%`), Compact (`1K/1M`), Scientific (`1e6`), Integer.
      - *Cycle Format*: Press `F` in HUD or click `[FMT]` to dynamically cycle formats.
    ],
    callout(title: "Live Search & Numeric Filters", stroke-color: slide-colors.accent-cyan)[
      - *Instant Typing*: Press `/` to search categories or series values in real time.
      - *Comparison Filters*: Enter expressions like `> 5000`, `<= 3000`, or `!= 0`.
    ],
    callout(title: "Marquee Drag & 6-KPI Stats", stroke-color: slide-colors.accent-purple)[
      - *Box Selection*: Click and drag across columns in visualizer to isolate ranges.
      - *6-Card Metrics*: Recomputes Total, Mean, Median, StdDev, Max, Min instantly.
    ],
  )
]

// Slide 17: Automatic Slide Overflow Protection & Layout Safety (Transition: Slide-Up)
#slide(title: "Automatic Slide Overflow Protection & Layout Safety", transition: "slide-up")[
  #cols(
    callout(title: "The Slide Overflow Problem", stroke-color: slide-colors.accent-red)[
      - In standard Typst, content exceeding 15.75cm height triggers an implicit pagebreak.
      - Overflow creates orphan spillover pages lacking titles, corrupting slide numbering and presentation flow.
    ],
    callout(title: "Cargo Slide Compiler Guard", stroke-color: slide-colors.accent-cyan)[
      - Automatically counts declared `#slide(...)` headers and checks compiled SVG page count.
      - Reports compile errors with exact slide title and line number before presentation runs.
    ]
  )

  #v(0.25cm)
  #callout(title: "Precise Error Diagnosis & Remediation", stroke-color: slide-colors.accent-orange)[
    ```text
    Error: Slide 3 ("In-Slide Component Animations", line 38) exceeded the vertical 16:9 canvas bounds.
    Spillover content pushed onto compiled page 4.
    Remediation: Reduce component heights or spacing, or set CARGO_SLIDE_ALLOW_OVERFLOW=1.
    ```
  ]
]

// Slide 18: Summary & Standalone Binary Delivery (Transition: Iris)
#slide(title: "Ready for Your Next Presentation", transition: "iris")[
  #align(center + horizon)[
    #text(size: 24pt, weight: "bold", fill: slide-colors.accent)[One Command to Build & Deliver]

    #v(0.4cm)
    #code-window(title: "terminal")[
      ```bash
      # Compile into a standalone native binary with all SVGs & assets bundled
      cargo slide build

      # Run anywhere with zero dependencies (no Typst, no Node.js needed!)
      ./slides-presentation
      ```
    ]

    #v(0.3cm)
    #badge("Cross-Platform", fill: slide-colors.accent-cyan) #h(6pt)
    #badge("Single Binary (~12 MB)", fill: slide-colors.accent-dark) #h(6pt)
    #badge("60 FPS Native", fill: slide-colors.accent-purple) #h(6pt)
    #badge("In-Slide Builds", fill: slide-colors.accent-red) #h(6pt)
    #badge("Interactive SQL/DSL", fill: slide-colors.accent-orange)
  ]
]
