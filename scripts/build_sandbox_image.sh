#!/usr/bin/env bash
# Builds/rebuilds the `apich-sandbox:latest` image (docker/Containerfile.sandbox) that every
# project's per-user sandbox container runs -- notebooks, scripts, agent CLIs, everything
# `ProjectManager::exec_in_sandbox` shells out to. project_manager.rs's own comment on
# `project_sandbox_config` has referenced this script by name since before it existed; this is
# that script.
#
# Run this whenever Containerfile.sandbox changes (e.g. a new package was added to fix a
# "ModuleNotFoundError" or similar in notebook cells) -- rebuilding retags `apich-sandbox:latest`
# to a new image ID, and `SandboxManager::ensure_running_with_config` (crates/apich-sandbox/src/
# manager.rs) detects that drift on the next request for each already-running project container
# and recreates it automatically, so there's no separate "restart all sandboxes" step needed.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

engine="${APICH_CONTAINER_ENGINE:-podman}"

"$engine" build -t apich-sandbox:latest -f docker/Containerfile.sandbox .

echo "apich-sandbox:latest built. Existing project containers will be recreated from it automatically on next use."
