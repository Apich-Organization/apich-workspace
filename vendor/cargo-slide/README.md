# cargo-slide

> **A code-driven presentation engine combining Typst typesetting with a native Rust runtime**  
> *Native 60 FPS Software Rendering · Standalone Binary, `.slide` Archive & Pure Rust WASM Delivery · Interactive SQL Charts · Extensible Animation Traits*

[English](README.md) | [简体中文](README_zh.md)

---

## Overview

`cargo-slide` is an open-source command-line tool and presentation runtime designed for developers and researchers who prefer authoring slide decks in code. Developed as part of the backend infrastructure for the [Apich workspace](https://github.com/Apich-Organization), it brings together two complementary technologies:

- **[Typst](https://typst.app/) for typesetting**: Sub-second compilation, clean markup syntax, first-class mathematical equation typesetting, and modular macros compiled directly to high-precision vector graphics (SVG).
- **Rust for presentation playback**: A standalone runtime built on [`tiny-skia`](https://github.com/RazrFalcon/tiny-skia) that renders slides at a steady 60 FPS without web browser or Electron dependencies, offering presenter tools, audio DSP mixing, interactive charts, and three distinct distribution formats.

---

## Three Delivery Paradigms

`cargo-slide` supports three delivery formats to accommodate different presentation environments:

| Paradigm | Target Use Case | Format & Size | Runtime Dependencies |
| :--- | :--- | :--- | :--- |
| **Standalone Binary** | Production talks on trusted machines | Single executable (~14–15 MB) | None (fully self-contained) |
| **Universal `.slide` Archive** | Sharing via email, chat, or flash drive | Ultra-compressed archive (~1 MB, LZMA2 extreme) | Played via `slide-viewer` |
| **WebAssembly CSR** | Online presentations & web embedding | Static HTML/WASM bundle (Zero inline JavaScript) | Any modern Web browser |

---

## Key Features

- **Typst-Centric Authoring**: Slide decks are written in pure Typst markup (`slides.typ`). Rust is only required when implementing custom transition algorithms or shader-like effects via traits.
- **Minimal Project Scaffolding**: `cargo slide init` initializes a clean 5-file project structure without unnecessary boilerplate.
- **Universal Viewer (`slide-viewer`)**:
  - Resizable modern GUI launcher with true fullscreen support (`F` / `F11`).
  - Automatic detection of local presentations and embedded 22-slide reference showcase.
  - Native file picker (`O` / `Ctrl+O`) supporting `.slide`, `.typ`, and `deck.json`.
  - One-click cross-platform installer (`I` key or `slide-viewer install` / `cargo slide install-viewer`) with file associations on Linux (`.desktop`, MIME), macOS (`.app` bundle), and Windows (Registry).
- **Presenter Tools**:
  - **4-Brush Whiteboard**: Solid Pen (`Pen`), Translucent Highlighter (`Highlighter`, multiply blend), Neon Glow (`Neon`, dual-layer halo), and Directional Arrow (`Arrow`, auto-oriented arrowhead).
  - **14-Color Palette & Color Tuning**: Quick swatches (`1`–`0`), stroke width adjustments (`[` / `]`), undo (`U` / `Ctrl+Z`), and per-slide clear (`C` / `X`).
  - **Laser Pointer**: Responsive red pointer with animated decay trail (`L` key).
  - **Audio Engine**: Background music playback, track looping, smooth crossfades, master volume slider, and self-dismissing on-screen toast notifications.
  - **Video Playback**: Video cards delegate playback to system media players (`mpv`, `ffplay`, or default player) in a detached non-blocking window.
- **Interactive SQL Charts & HUD Data Inspector**:
  - Read tabular data directly from `.csv`, `.json`, or `.db` (SQLite) files.
  - Execute in-memory SQL queries (`SELECT ... WHERE ...`) or pipeline DSL transformations.
  - In-presentation HUD Data Inspector: click any chart or table to switch visualization types (Bar, Line, Area, Scatter), apply transform presets (`TOP 5`, `SORT ▼`, `SORT ▲`, `CUM`, `% SHARE`, `MA3`), filter by condition (`>50`), sort columns, and export filtered data to CSV.
- **Slide Overflow Protection**: Compares compiled vector pages against declared `#slide(...)` counts. If content exceeds vertical canvas bounds, compilation reports the specific slide title and source line number.
- **Vectorized Typography & Multilingual Support**: Typst resolves text (Latin, CJK, math formulas, and Unicode emojis) into vector Bézier `<path>` outlines, ensuring identical rendering across platforms without requiring target fonts.
- **13 Built-in Transitions & In-Slide Fragments**: `fade`, `cut`, `slide-left`, `slide-right`, `slide-up`, `slide-down`, `zoom`, `wipe-left`, `wipe-right`, `iris`, `glitch`, `cube`, and `particles`. Control sequential item reveals with `#step(order, effect: "...")`.

---

## Getting Started

### Prerequisites

1. **Rust Toolchain**: Rust 1.80+ (recommended: latest stable). Install via [rustup.rs](https://rustup.rs/).
2. **Typst CLI**: `typst` executable available in your `PATH`. Install via package manager (`cargo install --locked typst-cli`, `brew install typst`, or Linux packages).
3. *(Optional)* **External Media Player**: `mpv` or `ffplay` for video playback cards.

### Step 1: Install `cargo-slide` and `slide-viewer`

Install from local source:
```bash
cargo install --path crates/cargo-slide
cargo install --path crates/slide-viewer
```

Or install the viewer to system associations:
```bash
slide-viewer install
```

### Step 2: Initialize a New Presentation

```bash
cargo slide init my-talk
cd my-talk
```

The generated project contains 5 clean files:
```text
my-talk/
├── slides.typ          # Main presentation content in Typst markup
├── theme.typ           # Theme styling, color palette, and canvas aspect ratio
├── slide.typ           # Built-in component macros (#slide, #step, #chart, etc.)
├── assets/
│   └── data.csv        # Sample dataset for interactive charts
└── .gitignore          # Excludes build caches and compiled binaries
```

*(Note: If you plan to implement custom Rust animation traits, add `--rust`: `cargo slide init my-talk --rust`.)*

### Step 3: Author and Present

Edit `slides.typ` with your favorite text editor, then run:

```bash
# Launch native presentation player with 60 FPS rendering
cargo slide run

# Or launch development mode with live hot-reloading
cargo slide dev
```

### Step 4: Build, Pack, or Serve

```bash
# 1. Build a self-contained single binary (~14 MB)
cargo slide build

# 2. Pack into an ultra-compressed .slide archive (~1 MB, LZMA2 extreme)
cargo slide pack -o presentation.slide

# 3. Export to Leptos WebAssembly CSR static website
cargo slide export --format wasm -o dist-web/

# 4. Host presentation locally with built-in static web server
cargo slide serve --open

# 5. Export to multi-page PDF or individual SVGs
cargo slide export --format pdf -o presentation.pdf
cargo slide export --format svg -o exported-svgs/
```

---

## Presenter Controls & Keyboard Shortcuts

| Key / Action | Function |
| :--- | :--- |
| **Space** / **Enter** / **Right** / **Down** / **Left Click** | Advance to next step (or next slide) |
| **Backspace** / **Left** / **Up** / **Right Click** | Step back to previous step (or previous slide) |
| **PageDown** / **PageUp** | Advance / reverse whole slide directly |
| **Home** / **End** | Jump to first / last slide |
| **Digit(s) + Enter** | Jump directly to specific slide number |
| **F11** / **F** / Dock `FULL` | Toggle native fullscreen and windowed modes |
| **L** / Dock `LSR` | Toggle laser pointer with decay trail |
| **P** / Dock `PEN` | Toggle whiteboard annotation pen |
| **T** | Cycle whiteboard brush mode (Pen, Highlighter, Neon, Arrow) |
| **[** / **]** | Decrease / increase brush stroke width (2px, 4px, 8px, 14px) |
| **Keys 1 .. 0** | Quick palette colors (14 vibrant swatches) |
| **U** / **Ctrl+Z** | Undo last whiteboard ink stroke |
| **C** / **X** / Dock `CLR` | Clear annotations on current slide |
| **Mouse Wheel** / **`+` / `-`** | Adjust master audio volume (0% to 100%) |
| **M** / Dock `VOL` | Mute / unmute audio |
| **H** / **?** / Dock `HELP` | Toggle shortcut help overlay |
| **O** / **Ctrl+O** | Open presentation file picker (.slide, .typ, .json) |
| **I** | Install viewer to system application launcher |
| **Click on Chart / Table** | Open interactive HUD Data Inspector modal |
| **Click on Video Card** | Launch external video player in detached window |
| **Click on Hyperlink** | Open external URL or preview local source text |
| **Esc** / **Q** | Close active modal / exit presentation |

---

## Interactive Charts & SQL Queries

### 1. Simple Chart from CSV
```typst
#chart(
  data: "assets/data.csv",
  type: "bar",
  title: "Runtime Memory Benchmarks",
  width: 100%,
  height: 6cm
)
```

### 2. In-Memory SQL Query
Query CSV, JSON, or SQLite datasets using standard SQL syntax:
```typst
#chart(
  data: "assets/telemetry.db",
  sql: "SELECT service, p99_latency FROM traces WHERE p99_latency > 50 ORDER BY p99_latency DESC",
  type: "bar",
  title: "High-Latency Microservices"
)
```

### 3. Pipeline DSL
For quick transformations without SQL:
```typst
#chart(
  data: "assets/metrics.json",
  dsl: "source -> filter(fps >= 30) -> select(Framework, FPS)",
  type: "line",
  title: "Frame Rate Comparison"
)
```

---

## Technical Comparison

| Feature | `cargo-slide` | Marp / Slidev | LaTeX Beamer |
| :--- | :--- | :--- | :--- |
| **Primary Engine** | Native Rust (`tiny-skia`) / Pure WASM | Web / Electron / Node.js | TeX / PDF reader |
| **Primary Output** | Single binary / `.slide` / WebAssembly | HTML bundle / PDF / App | Static PDF |
| **Typography** | Typst native vector paths | Browser CSS / Web fonts | LaTeX native fonts |
| **Math Quality** | Typst LaTeX-grade equations | KaTeX / MathJax | Native TeX math |
| **Frame Rate** | Steady 60 FPS | Variable (DOM/browser dependent) | Static (no transitions) |
| **Presenter Tools** | 4-brush ink, laser trail, audio mixer, SQL inspector | Plugin dependent | PDF reader dependent |
| **Dependencies** | None for output binary or `.slide` | Browser or Node runtime | PDF viewer |

---

## Workspace Structure

```text
cargo-slide/
├── Cargo.toml                      # Workspace configuration
├── crates/
│   ├── slide-core/                 # Typst compiler bridge, SVG parser, charts/SQL, package (LZMA2)
│   ├── slide-player/               # Native player: tiny-skia blitter, windowing, audio mixer, presenter tools, HUD
│   ├── slide-viewer/               # Standalone universal presentation player binary with GUI launcher for .slide files
│   ├── slide-web/                  # Pure Rust Leptos 0.7 CSR Web presentation player (strictly zero inline JS)
│   ├── slide-theme/                # Built-in themes, default templates, and Typst macros (#slide, #step, #chart)
│   └── cargo-slide/                # CLI tool implementing init, new, run, dev, build, pack, serve, export
└── examples/
    ├── geek-presentation/          # 22-slide reference showcase demonstrating all engine capabilities
    ├── dist-web/                   # Exported Leptos CSR static web bundle
    └── slides.slide                # Pre-built reference .slide package (LZMA2 extreme)
```

---

## Maintainers & Contact

`cargo-slide` is maintained as part of the backend infrastructure for the [Apich workspace](https://github.com/Apich-Organization).

- **Maintainer**: Xinyu Yang ([Xinyu.Yang@apich.org](mailto:Xinyu.Yang@apich.org))
- **Organization**: Apich Organization ([info@apich.org](mailto:info@apich.org))
- **Repository**: [https://github.com/Apich-Organization/cargo-slide](https://github.com/Apich-Organization/cargo-slide)

---

## License

This project is licensed under the [GNU Affero General Public License v3.0 or later](LICENSE) (`AGPL-3.0-or-later`).
