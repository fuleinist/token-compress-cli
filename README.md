# token-compress-cli

Reversible token compression for LLM prompts, logs, and outputs. Pipe-friendly CLI written in Rust.

```bash
cat big_prompt.txt | token-compress compress | your-llm-cli
```

Every transformation is recorded in a reversibility map, so you can always get the original
text back byte-for-byte:

```bash
token-compress compress input.txt -o packed.tc
token-compress expand packed.tc -o restored.txt   # identical to input.txt
```

## Why

LLM users waste tokens on repeated log blocks, long URLs, filler phrases, and stray
whitespace. `token-compress` squeezes those out *reversibly* — compress before sending to
the model, expand afterwards if you need the original. No network, no model, single static
binary.

## Install

```bash
cargo install --git https://github.com/fuleinist/token-compress-cli
```

Or build from source:

```bash
git clone https://github.com/fuleinist/token-compress-cli
cd token-compress-cli
cargo build --release    # binary at target/release/token-compress
```

## Usage

```text
token-compress compress [FILE]     compress FILE or stdin
  -o, --output <FILE>              write result to FILE (default stdout)
  -m, --map <FILE>                   write the map to a sidecar file instead of
                                   embedding an inline footer
      --no-ws / --no-dedup / --no-urls / --no-filler
                                   disable individual stages
      --json                       print stats as JSON to stderr

token-compress expand [FILE]       restore the original text
  -o, --output <FILE>              write result to FILE (default stdout)
  -m, --map <FILE>                 read map from sidecar file

token-compress stats [FILE]        report chars / estimated tokens only
```

Stats go to stderr, so stdout stays clean for pipes.

### Stages

| Stage   | What it does                                                          |
|---------|-----------------------------------------------------------------------|
| `ws`    | collapses space runs, strips trailing whitespace, squeezes blank runs |
| `dedup` | `N` consecutive identical lines (N≥3) → one copy + `⟦REP N×⟧` marker  |
| `urls`  | long http(s) URLs → `⟦U n⟧` markers (identical URLs share one id)     |
| `filler`| common LLM filler phrases → `⟦F n⟧` markers                           |

Literal `⟦` characters in your input are escaped first, so markers can never collide with
your data. Round-trip is byte-identical (tested across prose, code, JSON, CJK, CRLF, and
marker-collision fixtures).

### Output formats

- **Inline (default):** compressed body + a `⟦TCMAP v1⟧` footer containing the JSON map.
  One self-contained stream; `expand` auto-detects the footer.
- **Sidecar (`-m map.json`):** clean body + separate map file. Better compression ratio
  (the map isn't transmitted), but you must keep both files.

Measured on the bundled log fixture: **~1.8× token reduction inline, ~2.3× sidecar**.
Your mileage varies with how repetitive your input is.

## Honest limitations (v0.1)

- Token counts are heuristic estimates (~4 ASCII chars/token, ~1.8 CJK chars/token), not
  tokenizer-exact.
- The filler phrase list is small and fixed; semantic filler is out of scope.
- Compression ratio depends on redundancy in the input; unique prose compresses little.

See [SPEC.md](SPEC.md) for the full acceptance criteria and the v0.2 backlog
(tiktoken-exact counting, tree-sitter code-aware stage, ONNX semantic dedup).

## Development

```bash
cargo test                 # 18 tests incl. round-trip + corrupt-map handling
cargo clippy --all-targets
cargo fmt --check
```

CI runs test + clippy + fmt on ubuntu, windows, and macos.

## License

MIT — see [LICENSE](LICENSE).
