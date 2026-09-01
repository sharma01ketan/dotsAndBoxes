#!/usr/bin/env bash
# Stamp wasm-az/pkg with a hash of the Rust sources that feed @dab/dab-wasm-az
# (tensor contract in core + this crate). Separate from the base dab-wasm stamp
# so AZ-only edits do not force a base wasm/pkg rebuild.
set -euo pipefail
root="$(git rev-parse --show-toplevel)"
cd "$root"
stamp="$(
  git ls-files core/src wasm-az/src core/Cargo.toml wasm-az/Cargo.toml Cargo.lock \
    | sort \
    | git hash-object --stdin-paths \
    | git hash-object --stdin
)"
mkdir -p wasm-az/pkg
printf '%s\n' "$stamp" > wasm-az/pkg/SOURCE_STAMP
