# Contributing to cargo-slide

[English](CONTRIBUTING.md) | [简体中文](CONTRIBUTING_zh.md)

First off, thank you for considering contributing to **cargo-slide**! It's people like you who make the Rust community great.

---

## Code of Conduct

We are committed to providing a welcoming and inspiring community. We do not tolerate harassment, doxxing, or any form of hate speech. Please be respectful to all contributors and the original authors of the library. Please see [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) for more details.

---

## Performance First

**cargo-slide** is built for **extreme performance** and native 60 FPS presentation rendering. Any PR that affects the hot-path (rasterization blitter loop, SVG entity sanitizer/parser, SQLite/DSL chart querying, audio DSP mixer) **must** consider performance and avoid regressions.

- **Benchmarks**: Ensure that rendering throughput and latency are either improved or unaffected (no regressions).
- **Memory Safety**: We prioritize memory safety alongside performance. All `unsafe` blocks must have sound safety justifications and be validated with Miri where applicable:
  ```bash
  MIRIFLAGS="-Zmiri-disable-isolation" cargo +nightly miri test --all-features --no-fail-fast
  ```
- **Hot-Path Allocations**: Keep per-frame rendering and animation interpolation loops zero-allocation or bounded.

---

## Technical Standards

- **Rust Version**: We maintain an MSRV (Minimum Supported Rust Version). Please check `Cargo.toml` or `README.md` before using very new language features.
- **Documentation**: All new public APIs must be documented with comprehensive doc comments (`///`) and examples. Intra-doc links should be validated with `cargo doc --workspace --no-deps`.
- **Bilingual Documentation**: When introducing or updating user-facing CLI options, Typst template APIs, or features, please maintain parity between English and Chinese documentation files (`README.md` / `README_zh.md`, `CONTRIBUTING.md` / `CONTRIBUTING_zh.md`).
- **Strict Lint Policy**: The workspace enforces strict compiler and Clippy lints (`missing_docs`, `clippy::pedantic`, `clippy::all`, `clippy::unwrap_used`, `clippy::arithmetic_side_effects`, etc.). Resolve all critical lints rather than suppressing them without justification.

---

## Workflow

1. **Fork the repository** and create your branch from branch `main`.
2. **Implement your changes** and add tests.
3. **Run the test suite**:
   ```bash
   cargo test --all-features --all-targets
   cargo clippy --all-features
   cargo doc --workspace --no-deps
   ```
4. **Benchmark if applicable**:
   ```bash
   cargo bench --all-features
   ```
5. **Submit a Pull Request** with a clear description of your changes and performance results.

---

## AI-Assisted Development

We welcome discussions and contributions generated with the help of AI systems. However, as a contributor, you are responsible for verified accuracy, safety, and performance of any AI-suggested code.

---

## License

By contributing, you agree that your contributions will be licensed under the GNU Affero General Public License, Version 3 or later (**AGPL-3.0-or-later**).
