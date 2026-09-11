use crate::error::WebResult;
use apich_vcs::api::ProjectVcs;
use std::path::Path;

pub struct DemoProjectService;

impl DemoProjectService {
    /// Populate a project directory with all demonstration files and initialize VCS snapshot
    pub async fn seed_demo_files<P: AsRef<Path>>(dir: P) -> WebResult<()> {
        let root = dir.as_ref();
        tokio::fs::create_dir_all(root.join("assets")).await.ok();

        // 1. cargo-slide presentation: slides.typ
        // Follows the cargo-slide DSL defined in ~/dev/cargo-slide
        let slides_typ = r###"#import "theme.typ": *
#import "slide.typ": *

#show: slide-theme.with(
  aspect-ratio: "16-9",
  theme: "dark"
)

// Slide 1: Title Slide with Autoplay Ambient Track
#title-slide(
  title: "Superconducting Quantum Circuit Coherence",
  subtitle: "Pulse Calibration, Coherence Scaling & Real-Time Matrix Analysis",
  author: "APICH Research Group",
  institution: "Quantum Nanoelectronics Laboratory",
  date: "September 2026"
)
#audio("assets/ambient.wav", autoplay: false, loop: true, volume: 0.5)

// Slide 2: Architecture & Vector Graphics (Transition: slide-left)
#slide(title: "1. Hardware Architecture & Dilution Cryostat", transition: "slide-left")[
  #v(0.4cm)
  #cols(
    [
      #callout(title: "Planar Transmon Array", stroke-color: slide-colors.accent)[
        6-qubit ladder geometry with concentric coplanar waveguide resonators. Deep trench capacitive pads reduce two-level system (TLS) dielectric loss at silicon-substrate interfaces.
      ]
      #v(0.3cm)
      #callout(title: "Operating Regime", stroke-color: slide-colors.accent-cyan)[
        Base plate stabilized at $T approx 11.8 "mK"$. Residual magnetic flux density below $2 "nT"$ via dual cryoperm and superconducting lead shields.
      ]
    ],
    [
      #callout(title: "Microwave Telemetry Chain", stroke-color: slide-colors.accent-purple)[
        - Cryogenic HEMT low-noise amplifier (+38 dB gain)
        - Travelling Wave Parametric Amplifier (TWPA)
        - 14-bit 5 GS/s Arbitrary Waveform Generator (AWG)
        - In-situ FastCDC versioning & SQLite database logging
      ]
    ]
  )
]

// Slide 3: Sequential Component Animations & Fragments (Transition: iris)
#slide(title: "2. Sequential Calibration Protocol", transition: "iris")[
  Press *Next* or *Space* to reveal each calibration phase in order:

  #v(0.3cm)
  #step(1, effect: "fade-in")[
    #callout(title: "Step 1: Resonator Spectroscopy (#step(1))", stroke-color: slide-colors.accent)[
      Vector Network Analyzer (VNA) frequency sweep across 7.0 - 7.5 GHz to identify dispersive bare resonator dips and avoid spurious slotline modes.
    ]
  ]

  #v(0.25cm)
  #step(2, effect: "slide-up")[
    #callout(title: "Step 2: Qubit Frequency & Rabi Oscillations (#step(2))", stroke-color: slide-colors.accent-cyan)[
      Two-tone spectroscopy determines transition frequency $f_{01}$. Pulse length sweep calibrates $pi$ and $pi/2$ Gaussian envelope drive amplitudes.
    ]
  ]

  #v(0.25cm)
  #step(3, effect: "fade-in")[
    #callout(title: "Step 3: Coherence Characterization ($T_1$ & $T_2^*$) (#step(3))", stroke-color: slide-colors.accent-orange)[
      Inversion recovery measures longitudinal relaxation $T_1$. Ramsey fringe interferometry with detuned drive extracts dephasing time $T_2^*$.
    ]
  ]
]

// Slide 4: Rich Typography & LaTeX Mathematics (Transition: zoom)
#slide(title: "3. Hamiltonian Dynamics & Wave Equations", transition: "zoom")[
  #cols(
    [
      === Anharmonic Oscillator Hamiltonian
      The circuit Hamiltonian of a transmon qubit in the charge basis:

      $ hat(H) = 4 E_C (hat(n) - n_g)^2 - E_J cos(hat(phi)) $

      Under weakly non-linear approximation:

      $ hat(H) approx h bar omega_0 hat(a)^dagger hat(a) + frac(h bar alpha, 2) hat(a)^dagger hat(a)^dagger hat(a) hat(a) + Omega(t) (hat(a) + hat(a)^dagger) $

      #v(0.2cm)
      #badge("Quantum Optics", fill: slide-colors.accent-purple) #h(4pt)
      #badge("LaTeX Math", fill: slide-colors.accent) #h(4pt)
      #badge("Transmon Qubit", fill: slide-colors.accent-cyan)
    ],
    [
      === Simulation Script Snippet
      #code-window(title: "hamiltonian.rs")[
        #set text(size: 8.5pt)
        ```rust
        pub fn compute_eigenvalues(ec: f64, ej: f64) -> (f64, f64) {
            let omega_0 = (8.0 * ec * ej).sqrt() - ec;
            let alpha = -ec;
            (omega_0, alpha)
        }
        ```
      ]
    ]
  )
]

// Slide 5: Dynamic Charts from CSV / Database (Transition: slide-left)
#slide(title: "4. Telemetry Relaxation Metrics Across Qubits", transition: "slide-left")[
  #chart(
    type: "bar",
    source: "assets/data.csv",
    x: "qubit",
    y: "t1_us",
    title: "Measured Relaxation Time T1 Across 6-Qubit Ladder (Microseconds)",
    color: "#38bdf8"
  )

  #v(0.3cm)
  #cols(
    [
      *Summary Findings*:
      - Average $T_1 = 90.4~mu"s"$ across ladder array
      - Max $T_1 = 102.3~mu"s"$ achieved on Q2
      - Cross-resonance gate fidelity exceeds $99.1%$
    ],
    [
      *Underlying Data Source*:
      - Physical SQLite file: `quantum_measurements.table`
      - Exported CSV replica: `assets/data.csv`
      - Snapshot recorded per cooldown cycle
    ]
  )
]
"###;

        // 2 & 3. cargo-slide theme + component macros: theme.typ / slide.typ -- shared with
        // everything else that provisions a slide deck, see `cargo_slide_helpers`'s own doc
        // comment for why this used to be a copy living only here.
        let theme_typ = crate::services::cargo_slide_helpers::THEME_TYP;
        let slide_typ = crate::services::cargo_slide_helpers::SLIDE_TYP;

        // 4. Sample CSV dataset: assets/data.csv
        let data_csv = "qubit,frequency_ghz,t1_us,t2_us,readout_fidelity\nQ0,4.852,94.2,76.8,0.988\nQ1,5.014,88.5,64.2,0.982\nQ2,5.180,102.3,85.1,0.991\nQ3,4.920,79.8,58.4,0.979\nQ4,5.250,91.4,71.0,0.985\nQ5,5.105,86.2,63.9,0.983\n";

        // 5. Typst academic paper: paper.typ
        let paper_typ = r###"#set page(paper: "a4", margin: (x: 2cm, y: 2.5cm))
#set text(font: "Liberation Serif", size: 11pt)
#set par(justify: true)

#align(center)[
  #text(size: 18pt, weight: "bold")[High-Fidelity Coherence Preservation in Concentric Transmon Qubits]
  #v(0.5em)
  #text(size: 12pt)[Alice Researcher#super[1], Bob Coauthor#super[2]] \
  #text(size: 10pt, style: "italic")[1. Institute for Advanced Quantum Science; 2. Department of Physics]
]

#v(1em)
#block(fill: luma(245), inset: 1em, radius: 4pt)[
  *Abstract* --- We demonstrate coherent control and long-lifetime stabilization in a 6-qubit planar transmon array. By employing deep trench capacitive isolation and Purcell filtering, average relaxation times exceed $T_1 = 90~mu"s"$. In-situ telemetry metrics and statistical parameters were logged in structured SQLite tables and synchronized through APICH version control.
]

= 1. Introduction
Superconducting circuits represent a scalable modality for quantum information processing. The non-linear inductive energy provided by Josephson junctions establishes an anharmonic oscillator ladder:

$ H = 4 E_C (n - n_g)^2 - E_J cos(phi) $

Where $E_C$ denotes the single-electron charging energy and $E_J$ represents the Josephson coupling energy.

= 2. Experimental Characterization
Telemetry measurements across cooldown cycles are logged in physical SQLite database format (`quantum_measurements.table`). The cross-resonance drive amplitude is optimized to cancel spurious $Z Z$ crosstalk.

$ S_{21}(f) = 1 - frac(Q_L / |Q_c| e^(i phi_0), 1 + 2 i Q_L (f - f_r)/f_r) $

= 3. Conclusion
The structured workflow integrating Typst documents, cargo-slide presentations, and embedded SQLite tables enables fully reproducible scientific computation.
"###;

        // 6. LaTeX sample: report.tex
        let report_tex = r###"\documentclass[11pt,a4paper]{article}
\usepackage[utf8]{inputenc}
\usepackage{amsmath,amssymb}
\usepackage{geometry}
\geometry{margin=2.5cm}

\title{\textbf{Cryogenic Microwave Interconnect Calibration Report}}
\author{APICH Research Group}
\date{September 2026}

\begin{document}
\maketitle

\begin{abstract}
This report outlines the S-parameter attenuation characterization for high-density coaxial cabling inside the dilution cryostat across 4-8 GHz.
\end{abstract}

\section{System Model}
The total reflection coefficient $\Gamma$ and insertion loss $S_{21}$ are determined using vector network analyzer sweeps:
\begin{equation}
    S_{21}(f) = e^{-\alpha(f) L} e^{-j \beta(f) L}
\end{equation}

\section{Measurement Summary}
Attenuation at 10 mK base temperature remains below 1.2 dB/meter across the target readout band. Dispersive phase shift is matched within 2.5 degrees between adjacent channels.

\end{document}
"###;

        // 7. Unified Note: lab_notebook.anote
        // Unified Note format: YAML Frontmatter (metadata, tags, author, whiteboard nodes/edges) + Markdown Body (tasks with @date #tag, [[wiki links]])
        let lab_notebook_anote = r###"---
title: "Transmon Qubit Coherence Protocol & Lab Notebook"
created_at: 2026-09-08T09:00:00Z
updated_at: 2026-09-08T14:30:00Z
tags: [quantum, cryo, transmon, calibration]
author: "Alice Researcher"
category: "Lab Operations"
whiteboard:
  nodes:
    - id: "n1"
      type: "box"
      label: "AWG Microwave Pulse Source"
      x: 80
      y: 120
      color: "#38bdf8"
    - id: "n2"
      type: "box"
      label: "Dilution Cryostat 10mK Sample Stage"
      x: 360
      y: 120
      color: "#818cf8"
    - id: "n3"
      type: "box"
      label: "HEMT Low-Noise Amp"
      x: 640
      y: 120
      color: "#34d399"
  edges:
    - from: "n1"
      to: "n2"
      label: "Coax Line (attenuated 60dB)"
    - from: "n2"
      to: "n3"
      label: "Dispersive Readout Output"
---

# Transmon Qubit Coherence Protocol & Daily Operations

Related wiki modules: [[cryostat_cooldown|Cryostat Cool-Down Log]], [[microwave_calibration|Microwave Pulse Calibration]], and [[surface_code_readout|Surface Code Readout]].

## Action Tasks & Schedule
- [ ] Measure resonator dispersive shift on Q0 and Q1 #hardware @2026-09-20
- [/] Calibrate pi-pulse Rabi oscillations and amplitude #control @2026-09-22
- [x] Characterize resonator frequency response #rf @2026-09-10
- [x] Room-temperature VNA cable continuity and attenuation #rf @2026-09-10

## Cooldown Observation Notes
Base temperature reached 11.8 mK at 04:30 AM. Residual magnetic shield pressure nominal. All 6 transmon channels exhibit sharp transmission dips around 7.2 GHz.
"###;

        // 8. Python data analysis script: analysis.py
        let analysis_py = r###"# Quantum measurement telemetry analysis script
import csv
import math

def calculate_average_t1(csv_path):
    total = 0.0
    count = 0
    with open(csv_path, mode='r') as f:
        reader = csv.DictReader(f)
        for row in reader:
            total += float(row['t1_us'])
            count += 1
    return total / count if count > 0 else 0.0

if __name__ == '__main__':
    avg = calculate_average_t1('assets/data.csv')
    print(f"Average T1 relaxation time: {avg:.2f} us")
"###;

        // 9. README.md
        let readme_md = r###"# APICH Demonstration Workspace

This project contains sample files demonstrating the application modalities supported in APICH:

### 1. Document & Slide Tools (`/projects/:id/editor`)
- **`slides.typ`**: Presentation deck authored with the `cargo-slide` domain-specific language. Open to explore the outline, code editor, and live presentation preview.
- **`theme.typ`** & **`slide.typ`**: Presentation theme and component macros.
- **`paper.typ`**: Academic document authored in Typst with mathematical formulas and section hierarchy.
- **`report.tex`**: Standard LaTeX academic article format.

### 2. Spreadsheet Application (`/projects/:id/table`)
- **`quantum_measurements.table`**: SQLite database file presented as a spreadsheet table with formula bar, row/column editing, and SQL console.

### 3. Unified Note Studio (`/projects/:id/note`)
Five tabs share one file format and one sub-navigation: Editor, Whiteboard, Wiki, Calendar, Kanban.
- **`lab_notebook.anote`**: Main lab notebook -- YAML frontmatter (tags, author, embedded whiteboard diagram) plus a Markdown body with dated/tagged tasks and wiki-style links to the three pages below.
- **`cryostat_cooldown.anote`**, **`microwave_calibration.anote`**, **`surface_code_readout.anote`**: Linked wiki pages forming a real bidirectional graph with `lab_notebook.anote`, each contributing their own dated tasks to the shared Kanban board and Calendar.

### 4. Computational Scripts (multi-language sandbox)
- **`analysis.py`**: Python script for telemetry calculations.
- **`analysis.R`**: R script computing summary statistics over the same dataset.
- **`simulate.rs`**: Rust script simulating transmon eigenvalues; also a plain-code-file example for the lightweight syntax-highlighted editor.

### 5. VCS & Sharing
Open the "VCS" tab to see a real snapshot history including a feature branch (`readout-experiment`) merged back with APICH's weave-free reconciler. The "Share" panel on this project demonstrates a public read-and-review link plus a couple of per-file sharing rules.
"###;

        // 10. R data analysis script: analysis.R
        let analysis_r = r###"# Quantum measurement telemetry analysis (R)
telemetry <- read.csv("assets/data.csv")

cat(sprintf("Mean T1: %.2f us (sd %.2f)\n", mean(telemetry$t1_us), sd(telemetry$t1_us)))
cat(sprintf("Mean T2: %.2f us (sd %.2f)\n", mean(telemetry$t2_us), sd(telemetry$t2_us)))
cat(sprintf("Mean readout fidelity: %.4f\n", mean(telemetry$readout_fidelity)))

fit <- lm(t1_us ~ frequency_ghz, data = telemetry)
cat("Linear fit T1 ~ frequency_ghz:\n")
print(summary(fit)$coefficients)
"###;

        // 11. Rust script: simulate.rs (plain-code-file editor demo + Rust sandbox support)
        let simulate_rs = r###"// Transmon anharmonic-oscillator eigenvalue estimate.
// Run inside the project sandbox: `rustc simulate.rs -o /tmp/simulate && /tmp/simulate`

fn compute_eigenvalues(ec_ghz: f64, ej_ghz: f64) -> (f64, f64) {
    let omega_0 = (8.0 * ec_ghz * ej_ghz).sqrt() - ec_ghz;
    let anharmonicity = -ec_ghz;
    (omega_0, anharmonicity)
}

fn main() {
    let qubits = [
        ("Q0", 0.24, 14.8),
        ("Q1", 0.25, 15.1),
        ("Q2", 0.23, 15.4),
    ];

    for (label, ec, ej) in qubits {
        let (omega_0, alpha) = compute_eigenvalues(ec, ej);
        println!("{label}: omega_0 = {omega_0:.4} GHz, anharmonicity = {alpha:.4} GHz");
    }
}
"###;

        // 12-14. Linked wiki pages: form a real bidirectional graph with lab_notebook.anote,
        // and contribute their own dated/tagged tasks to the shared Kanban board and Calendar.
        let cryostat_cooldown_anote = r###"---
title: "Cryostat Cool-Down Log"
created_at: 2026-09-05T07:00:00Z
updated_at: 2026-09-08T09:00:00Z
tags: [cryo, operations]
author: "Bob Coauthor"
category: "Lab Operations"
---

# Cryostat Cool-Down Log

Parent notebook: [[lab_notebook|Transmon Qubit Coherence Protocol & Lab Notebook]]. See also [[microwave_calibration|Microwave Pulse Calibration]].

## Cool-Down Checklist
- [x] Evacuate outer vacuum can to below 1e-5 mbar #cryo @2026-09-05
- [x] Start pulse-tube precool, verify 50K/4K stage temperatures #cryo @2026-09-06
- [ ] Log final base-temperature stabilization curve #cryo @2026-09-21

## Notes
Standard cool-down to 11.8 mK base takes approximately 36 hours from room temperature. Magnetic shielding installed before the final descent below 4K.
"###;

        let microwave_calibration_anote = r###"---
title: "Microwave Pulse Calibration"
created_at: 2026-09-06T10:00:00Z
updated_at: 2026-09-08T11:00:00Z
tags: [control, calibration]
author: "Alice Researcher"
category: "Lab Operations"
---

# Microwave Pulse Calibration

Parent notebook: [[lab_notebook|Transmon Qubit Coherence Protocol & Lab Notebook]]. Feeds into [[surface_code_readout|Surface Code Readout]].

## Calibration Tasks
- [x] Sweep AWG pulse amplitude for pi-pulse on Q0-Q5 #control @2026-09-07
- [/] Fine-tune DRAG coefficient to suppress leakage to |2> #control @2026-09-18
- [ ] Re-calibrate after next cooldown cycle #control @2026-09-25

## Notes
DRAG correction reduced leakage error by roughly 40% on Q2, the qubit with the tightest anharmonicity margin.
"###;

        let surface_code_readout_anote = r###"---
title: "Surface Code Readout"
created_at: 2026-09-07T13:00:00Z
updated_at: 2026-09-08T14:00:00Z
tags: [readout, surface-code]
author: "Alice Researcher"
category: "Experiment Design"
---

# Surface Code Readout

Parent notebook: [[lab_notebook|Transmon Qubit Coherence Protocol & Lab Notebook]]. Depends on [[microwave_calibration|Microwave Pulse Calibration]] being complete.

## Readout Tasks
- [ ] Implement dispersive-shift lookup table for 6-qubit ladder #readout @2026-09-24
- [ ] Benchmark single-shot readout fidelity against threshold discriminator #readout @2026-09-27
- [x] Draft syndrome-extraction circuit diagram #surface-code @2026-09-08

## Notes
Target readout fidelity for the distance-3 surface code prototype is 99.5%; current single-qubit average sits at 98.5% (see `quantum_measurements.table`).
"###;

        // Write all text files
        tokio::fs::write(root.join("slides.typ"), slides_typ).await.ok();
        tokio::fs::write(root.join("theme.typ"), theme_typ).await.ok();
        tokio::fs::write(root.join("slide.typ"), slide_typ).await.ok();
        tokio::fs::write(root.join("assets").join("data.csv"), data_csv).await.ok();
        // Minimal valid (silent, 1-sample) WAV file so slides.typ's #audio() asset reference resolves to a real file.
        let ambient_wav: [u8; 46] = [
            0x52, 0x49, 0x46, 0x46, 0x26, 0x00, 0x00, 0x00, 0x57, 0x41, 0x56, 0x45, 0x66, 0x6d, 0x74, 0x20,
            0x10, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x44, 0xac, 0x00, 0x00, 0x88, 0x58, 0x01, 0x00,
            0x02, 0x00, 0x10, 0x00, 0x64, 0x61, 0x74, 0x61, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        tokio::fs::write(root.join("assets").join("ambient.wav"), ambient_wav).await.ok();
        tokio::fs::write(root.join("paper.typ"), paper_typ).await.ok();
        tokio::fs::write(root.join("report.tex"), report_tex).await.ok();
        tokio::fs::write(root.join("lab_notebook.anote"), lab_notebook_anote).await.ok();
        tokio::fs::write(root.join("cryostat_cooldown.anote"), cryostat_cooldown_anote).await.ok();
        tokio::fs::write(root.join("microwave_calibration.anote"), microwave_calibration_anote).await.ok();
        tokio::fs::write(root.join("surface_code_readout.anote"), surface_code_readout_anote).await.ok();
        tokio::fs::write(root.join("analysis.py"), analysis_py).await.ok();
        tokio::fs::write(root.join("analysis.R"), analysis_r).await.ok();
        tokio::fs::write(root.join("simulate.rs"), simulate_rs).await.ok();
        tokio::fs::write(root.join("README.md"), readme_md).await.ok();

        // 10. Create SQLite physical table file: quantum_measurements.table
        let db_path = root.join("quantum_measurements.table");
        if let Ok(conn) = rusqlite::Connection::open(&db_path) {
            let _ = conn.execute_batch(r#"
                CREATE TABLE IF NOT EXISTS qubit_characterization (
                    id INTEGER PRIMARY KEY,
                    qubit_label TEXT NOT NULL,
                    frequency_ghz REAL NOT NULL,
                    t1_us REAL NOT NULL,
                    t2_echo_us REAL NOT NULL,
                    readout_fidelity REAL NOT NULL,
                    status TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS calibration_telemetry (
                    id INTEGER PRIMARY KEY,
                    timestamp TEXT NOT NULL,
                    temperature_mk REAL NOT NULL,
                    attenuation_db REAL NOT NULL,
                    operator TEXT NOT NULL
                );

                DELETE FROM qubit_characterization;
                INSERT INTO qubit_characterization VALUES
                    (1, 'Q0_Transmon', 4.852, 94.2, 76.8, 0.988, 'Calibrated'),
                    (2, 'Q1_Transmon', 5.014, 88.5, 64.2, 0.982, 'Calibrated'),
                    (3, 'Q2_Transmon', 5.180, 102.3, 85.1, 0.991, 'Calibrated'),
                    (4, 'Q3_Transmon', 4.920, 79.8, 58.4, 0.979, 'Tune-up'),
                    (5, 'Q4_Transmon', 5.250, 91.4, 71.0, 0.985, 'Calibrated'),
                    (6, 'Q5_Transmon', 5.105, 86.2, 63.9, 0.983, 'Calibrated');

                DELETE FROM calibration_telemetry;
                INSERT INTO calibration_telemetry VALUES
                    (1, '2026-09-08 04:30', 11.8, 60.0, 'Alice'),
                    (2, '2026-09-08 08:15', 12.1, 60.0, 'Bob'),
                    (3, '2026-09-08 12:00', 11.9, 60.0, 'Alice');
            "#);
        }

        // Initialize VCS with a real multi-snapshot history: an initial commit, a follow-up edit,
        // then a feature branch with its own commit merged back via the weave-free reconciler --
        // so the VCS tab has real branch/merge content to inspect, not just one bare snapshot.
        if let Ok(vcs) = ProjectVcs::open_or_init(root) {
            let _ = vcs.snapshot_if_changed(
                "Initial commit: Demonstration project with cargo-slide, Typst, LaTeX, SQLite tables, and Unified Notes.",
            );

            if let Some(main_branch) = vcs.current_branch().ok().flatten() {
                // Follow-up edit on the main line.
                tokio::fs::write(
                    root.join("README.md"),
                    format!(
                        "{}\n### Reproducibility\nAll telemetry in `assets/data.csv` and `quantum_measurements.table` is regenerated per cooldown cycle; see `analysis.py`/`analysis.R` for the summary statistics used in `paper.typ`.\n",
                        readme_md
                    ),
                )
                .await
                .ok();
                let _ = vcs.snapshot_if_changed("Add reproducibility notes to README");

                // Feature branch: extend the readout wiki page, then merge back.
                if vcs.branch_create("readout-experiment").is_ok() && vcs.branch_switch("readout-experiment").is_ok() {
                    let extended = format!(
                        "{}\n## Update ({})\n- [ ] Cross-check syndrome extraction timing against calibration branch #readout @2026-09-28\n",
                        surface_code_readout_anote, "readout-experiment branch"
                    );
                    tokio::fs::write(root.join("surface_code_readout.anote"), extended).await.ok();
                    let _ = vcs.snapshot_if_changed("Add cross-check task on readout-experiment branch");

                    if vcs.branch_switch(&main_branch).is_ok() {
                        let _ = vcs.merge("readout-experiment");
                    }
                }
            }
        }

        Ok(())
    }
}
