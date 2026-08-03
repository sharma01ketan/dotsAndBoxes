# dab-cli

Terminal hotseat playground for **Dots and Boxes**, backed by `dab-core`.

Use this to validate that the Rust game engine (rules, scoring, extra turns) works
before wiring WASM / the web UI.

## Run

From the repo root:

```bash
# Default 2×2 boxes
cargo run -p dab-cli

# Custom size
cargo run -p dab-cli -- --rows 3 --cols 3
```

## How to play

Two humans share the keyboard. On each turn:

1. Look at the ASCII board (`·` = undrawn edge, `---` / `|` = drawn, `1`/`2` = claimed).
2. Type `legal` to see undrawn edges as `#id  H r c` or `#id  V r c`.
3. Play with either:
   - `H <row> <col>` — horizontal edge  
   - `V <row> <col>` — vertical edge  
   - `<id>` or `#<id>` — dense edge id  

Completing a box scores a point and grants an **extra turn**.

Other commands: `board`, `help`, `quit`.

## Layout

```
cli/
├─ Cargo.toml      # package: dab-cli
├─ README.md
└─ src/
   ├─ main.rs      # REPL / args / game loop
   └─ render.rs    # ASCII board + help text
```

This crate only depends on `dab-core`. It does not touch the web app or the server.
