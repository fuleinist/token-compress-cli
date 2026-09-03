# token-compress-cli — v0.1 Spec

**One line:** Reversible token compression for LLM prompts, logs, and outputs. Pipe-friendly CLI.

## Problem

LLM users waste tokens on verbose prompts, repeated log blocks, boilerplate filler, and long
URLs/paths. Paid tools (headroom) and 14-stage pipelines (claw-compactor) prove demand, but
there's no small, fast, local, open CLI that compresses text *reversibly* so you can expand it
back after the LLM run.

## Goals (v0.1)

- G1: Compress text streams (prompts, logs, markdown, JSON, code) to fewer estimated tokens.
- G2: **Round-trip guarantee** — `expand(compress(x)) == x` byte-for-byte for all enabled stages.
- G3: Pipe-friendly: `cat big.txt | token-compress compress | ... | token-compress expand`.
- G4: Report char + estimated-token savings per run.
- G5: No network, no model, no dependencies at runtime; single static Rust binary.

## Non-goals (v0.1)

- Semantic/embedding-based compression (ONNX) — future.
- AST-aware code compression (tree-sitter) — future.
- Tokenizer-exact counts (tiktoken) — heuristic estimator only.

## CLI surface

```text
token-compress compress [FILE]        # default: stdin if no FILE
  -o, --output <FILE>                 # write compressed text (default stdout)
  -m, --map <FILE>                    # write reversibility map (default: inline footer)
      --no-urls                       # disable URL shortening stage
      --no-dedup                      # disable repeat-dedup stage
      --no-filler                     # disable filler-phrase stage
      --no-ws                         # disable whitespace-squeeze stage
      --inline                        # force inline footer map (default when piping)
      --json                          # print stats JSON to stderr

token-compress expand [FILE]          # restore original from compressed + map
  -o, --output <FILE>
  -m, --map <FILE>                    # external map file if produced with compress -m

token-compress stats [FILE]           # report chars/estimated tokens only, no transform
token-compress --version | --help
```

## Compression stages (all reversible)

| Stage   | Transform                                                                 | Reversibility                                   |
|---------|---------------------------------------------------------------------------|--------------------------------------------------|
| `ws`    | Collapse runs of 2+ spaces to 1; strip trailing ws; squeeze 3+ blank lines to 2 | Map records original run lengths/positions |
| `dedup` | N consecutive identical lines (N≥3) → `⟦REP k×⟧` marker + one copy        | Marker carries count                             |
| `urls`  | URLs longer than 24 chars → `⟦U1⟧`, `⟦U2⟧`, …                             | Map records index→URL                            |
| `filler`| Delete known filler phrases ("Great question!", "I'd be happy to help!", "Certainly! ", etc.) | Map records phrase id + position |

Escaping: literal text that collides with markers (`⟦…⟧`) is escaped with `⟦ESC⟧` prefix;
expand reverses escaping first.

## Output formats

- **Inline mode (default when stdout is a pipe, or `--inline`):** compressed text followed by a
  footer:
  ```text
  ⟦TCMAP v1 stages=ws,dedup,urls,filler⟧
  <JSON map>
  ⟦/TCMAP⟧
  ```
  Expand detects and strips the footer automatically.
- **Sidecar mode (`-m map.json`):** compressed text only; map written to the given file.
  `expand -m map.json` reads it.

Token estimation: `est_tokens = ceil(chars / 4.0)` for ASCII-dominant text, `ceil(chars / 1.8)`
weighting for CJK-heavy segments. Heuristic only; documented as such.

## Acceptance criteria

- A1: Round-trip tests pass byte-identical on fixtures: prose, markdown, JSON, Rust code,
  log dump, CJK text, text already containing `⟦⟧` markers.
- A2: Compression ratio ≥ 1.3× on the bundled log fixture (tokens, inline mode).
- A3: `echo "x" | token-compress compress | token-compress expand` returns `x` (newline-preserving).
- A4: Invalid/corrupt map → clear error, exit code 2, never silent data loss.
- A5: `cargo test` green; `cargo clippy` clean; `cargo fmt` applied.
- A6: CI: GitHub Actions test + clippy + fmt on ubuntu, windows, macos.

## Quality bar

- Errors are actionable messages, not panics.
- No data-loss paths: if a stage can't be recorded reversibly, it is skipped.
- README with install (cargo install), usage, and honest limitations.

## Out-of-scope backlog (v0.2+)

- tiktoken-exact counting, tree-sitter code-aware stage, ONNX semantic dedup,
  streaming expand for huge files, WASM build.
