# `apich-sandbox`

`apich-sandbox` 是 APICH Technical & Academic Workspace 的容器化沙盒调度与生命周期管理核心组件，基于 rootless Podman 构建。

## 主要功能

1. **用户容器生命周期管理 (`SandboxManager` / `UserContainer`)**
   - `ensure_running`: 幂等启动，按需自动创建用户存储目录并以隔离容器挂载启动。
   - `status` / `inspect`: 查询容器状态（`Running`, `Paused`, `Exited`, `Stopped`, `Created`）。
   - `pause` / `unpause`: 快速挂起与唤醒。
   - `stop`: 优雅停止（支持超时时间配置）。
   - `restart` & `destroy`: 重启与安全清理。

2. **存储与文件安全读写 (`fs`)**
   - 自动挂载主机存储目录至容器内部（如 `/workspace`），支持 SELinux 自动重标记 (`:Z`)。
   - 提供安全的直接读写 API，防御 `../` 路径穿越攻击。
   - 支持跨容器复制文件 (`copy_into`, `copy_out`)。

3. **命令执行与实时流 (`exec`)**
   - 同步执行并捕获 `stdout`、`stderr`、`exit_code`、耗时与超时控制。
   - 异步流式输出 (`exec_stream`)，支持实时按 chunk 接收输出，无缝对接前端 WebSocket / SSE 终端与 Notebook。

4. **学术与研发工具链抽象 (`tools`)**
   - **Rust**: `cargo_build`, `cargo_run`, `cargo_test`, `cargo_check`, `cargo_slide`
   - **Python**: `run_code`, `run_file`, `pip_install`, `python_version`
   - **Git**: `init`, `status`, `add`, `commit`, `log`, `diff`, `current_branch`
   - **R**: `run_code`, `run_file`, `r_version`
   - **LaTeX**: `compile` (支持 `xelatex`, `pdflatex`, `latexmk`), `clean_aux_files`
   - **Typst**: `compile` (PDF / SVG / PNG 格式支持), `query`

## 测试

```bash
cargo test
```
