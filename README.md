# APICH Next-Gen Technical & Academic Workspace

本工程是面向技术研发与学术研究的开源 Workspace 解决方案（纯 Rust 架构底座）。

## 模块构成与开发进度

- [x] **`crates/apich-sandbox`**: 基于 Podman 的单用户容器沙盒生命周期管理核心 crate。
  - 用户独立容器隔离与资源配额控制（CPU、内存、PIDs）。
  - 用户工作空间目录挂载与安全读写（防路径穿越、自动处理 SELinux `:Z` 标签）。
  - 容器完整生命周期（创建、启动、暂停、恢复、重启、优雅关闭、销毁、检查）。
  - 同步命令执行与异步实时流式标准输出/错误接收（支持 Web 终端与 Notebook 计算输出）。
  - 学术与技术工具链抽象（Rust / Python / Git / R / LaTeX / Typst）。
- [x] **`docker/Containerfile.sandbox`**: 包含 Rust、Python3、Git、R、LaTeX (XeTeX/latexmk)、Typst 的完整学术与计算运行环境镜像定义。
- [x] **`compose.yaml`**: 标准编排文件，配置开发者沙盒与 Forgejo 独立 Git 托管实例。
- [x] **全方位单元测试与集成测试**: 14 个测试全覆盖并全部通过。

## 快速测试与运行

```bash
# 运行单元测试与 Podman 集成测试
cargo test
```
