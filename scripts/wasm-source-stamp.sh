#!/usr/bin/env bash
# Stamp wasm/pkg with a hash of the Rust sources that feed @dab/dab-wasm.
# The .wasm blob itself is not portable across macOS vs Linux, so CI compares
# this stamp + JS glue instead of the binary.
set -euo pipefail
root="$(git rev-parse --show-toplevel)"
cd "$root"
stamp="$(
  git ls-files core/src wasm/src core/Cargo.toml wasm/Cargo.toml Cargo.lock \
    | sort \
    | git hash-object --stdin-paths \
    | git hash-object --stdin
)"
mkdir -p wasm/pkg
printf '%s\n' "$stamp" > wasm/pkg/SOURCE_STAMP
