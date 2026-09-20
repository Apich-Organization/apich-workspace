#!/usr/bin/env bash
# Builds the apich-islands wasm bundle and drops the wasm-bindgen output (apich_islands.js +
# apich_islands_bg.wasm) into crates/apich-web/pkg, where apich-web serves it from `/pkg`
# (see create_app() in crates/apich-web/src/lib.rs) and the bootstrap script emitted by
# IslandScript (crates/apich-web/src/app/components/mod.rs) loads it in the browser.
#
# Requires: rustup target add wasm32-unknown-unknown
#           cargo install wasm-bindgen-cli --version <matching the `wasm-bindgen` crate version
#           pinned in Cargo.lock -- the CLI and crate versions must match exactly>
#
# Run from the workspace root before `cargo run -p apich-web`, or whenever apich-islands changes.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"
export PATH="${HOME}/.cargo/bin:${PATH}"

cargo build -p apich-islands --target wasm32-unknown-unknown --no-default-features --features hydrate --release

wasm-bindgen \
    target/wasm32-unknown-unknown/release/apich_islands.wasm \
    --target web \
    --out-dir crates/apich-web/pkg \
    --out-name apich_islands \
    --no-typescript

echo "apich-islands wasm bundle built -> crates/apich-web/pkg/"
