# 为 cargo-slide 作出贡献

[English](CONTRIBUTING.md) | [简体中文](CONTRIBUTING_zh.md)

首先，感谢您考虑为 **cargo-slide** 做出贡献！正是像您这样的人让 Rust 社区变得更加美好。

---

## 行为准则

我们致力于提供一个温馨且富有启发性的社区。我们绝不容忍骚扰、人肉搜索或任何形式的仇恨言论。请尊重所有贡献者和库的原作者。详情请参阅 [CODE_OF_CONDUCT_zh.md](CODE_OF_CONDUCT_zh.md)。

---

## 性能至上

**cargo-slide** 专为**极致性能**和原生 60 FPS 演示渲染而构建。任何影响热路径（光栅化位块传送循环、SVG 实体清洗/解析器、SQLite/DSL 图表查询管线、音频 DSP 混音器）的 PR **必须**考量性能并避免出现退化。

- **基准测试**：确保渲染吞吐量和延迟得到改善或不受影响（无退化）。
- **内存安全**：我们在追求性能的同时将内存安全置于首位。所有 `unsafe` 代码块必须具备完备的安全论证理由，并在适当时使用 Miri 进行验证：
  ```bash
  MIRIFLAGS="-Zmiri-disable-isolation" cargo +nightly miri test --all-features --no-fail-fast
  ```
- **热路径内存分配**：保持每帧渲染与动画插值循环中的堆内存分配为零或严格受限。

---

## 技术标准

- **Rust 版本**：我们维护 MSRV（最低支持的 Rust 版本）。在使用非常新的语言特性前，请检查 `Cargo.toml` 或 `README_zh.md`。
- **文档规范**：所有新的公有 API 必须配备详尽的文档注释（`///`）和示例。文档内链应通过 `cargo doc --workspace --no-deps` 进行验证。
- **中英双语文案**：在引入或更新面向用户的 CLI 参数、Typst 模版 API 或功能特性时，请确保中英双语文档（`README.md` / `README_zh.md`，`CONTRIBUTING.md` / `CONTRIBUTING_zh.md`）保持同步。
- **严格的代码检查（Clippy）规范**：工作区配置了严苛的编译器与 Clippy lint 检查（`missing_docs`, `clippy::pedantic`, `clippy::all`, `clippy::unwrap_used`, `clippy::arithmetic_side_effects` 等）。请尽量通过严谨的设计解决关键 lint，而非无合理解释地直接压制。

---

## 工作流

1. **Fork 本仓库**并从 `main` 分支创建您的特性分支。
2. **实现您的改动**并补充相关测试。
3. **运行测试套件**：
   ```bash
   cargo test --all-features --all-targets
   cargo clippy --all-features
   cargo doc --workspace --no-deps
   ```
4. **进行基准测试（如适用）**：
   ```bash
   cargo bench --all-features
   ```
5. **提交 Pull Request**，并清晰描述您的更改内容及性能测试结果。

---

## AI 辅助开发

我们欢迎借助 AI 系统讨论和提交的贡献。然而，作为贡献者，您对任何由 AI 建议的代码的验证准确性、安全性和性能负全责。

---

## 许可证

参与贡献即表示您同意您的贡献将遵循 GNU Affero 通用公共许可证第 3 版或更高版本（**AGPL-3.0-or-later**）。
