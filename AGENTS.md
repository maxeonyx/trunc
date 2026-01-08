# trunc - Agent Instructions

This file contains instructions for AI agents working on this project.

## Project Overview

`trunc` is a Rust CLI tool for truncating pipe output. It shows the first N and last M lines, with an optional pattern-matching mode that extracts matches from the middle.

## Development Commands

```bash
# Run tests
cargo test

# Run a specific test
cargo test test_name

# Build release binary
cargo build --release

# Run with arguments
cargo run -- -f 5 -l 5 < some_file.txt

# Check formatting and lints
cargo fmt --check
cargo clippy
```

## Architecture

The implementation should be simple and streaming:

1. Read stdin line by line
2. Buffer the first N lines (head)
3. Maintain a ring buffer of the last M lines (tail)
4. If pattern mode: also track matches with context
5. On EOF: output head, separator, matches (if any), tail

## Test Strategy

Tests are black-box E2E tests that spawn the `trunc` binary and check stdout.

Test files are in `tests/` directory. Each test:
1. Creates input data
2. Pipes it to the `trunc` binary
3. Asserts on stdout content

## Key Files

- `src/main.rs` - Entry point and CLI parsing
- `src/lib.rs` - Core logic (if we split it out)
- `tests/e2e.rs` - End-to-end tests
- `VISION.md` - Project vision and requirements
- `TODO.md` - Task tracking
- `.github/workflows/ci.yml` - CI pipeline (check, fast tests, E2E tests, cross-platform)
- `.github/workflows/release.yml` - Release pipeline (build binaries, GitHub release, crates.io)

## CI Pipeline

The CI runs on every push and PR to `main`:

1. **Check & Lint** - `cargo fmt --check`, `cargo clippy`, `cargo check`
2. **Fast Tests** - Unit tests only (`cargo test --lib`)
3. **E2E Tests** - Integration tests (`cargo test --test '*'`), depends on Check passing
4. **Cross-Platform** - Full test suite on Linux, macOS, Windows

## Release Pipeline

Triggered by pushing a tag like `v0.1.0`:

1. Builds release binaries for:
   - Linux (x86_64, x86_64-musl, aarch64)
   - macOS (x86_64, aarch64)
   - Windows (x86_64)
2. Creates GitHub Release with all binaries
3. Publishes to crates.io (requires `CARGO_REGISTRY_TOKEN` secret)

## CLI Specification

```
trunc [OPTIONS] [PATTERN]

Arguments:
  [PATTERN]  Regex pattern to search for in the middle section

Options:
  -f, --first <N>     Number of lines to show from start (default: 10)
  -l, --last <N>      Number of lines to show from end (default: 10)
  -H, --head <N>      Alias for --first
  -T, --tail <N>      Alias for --last
  -m, --matches <N>   Max matches to show in pattern mode (default: 5)
  -C, --context <N>   Lines of context around each match (default: 3)
  -w, --width <N>     Chars to show at start/end of long lines (default: 100, 0 = no limit)
  -h, --help          Print help
  -V, --version       Print version
```

### Line Truncation

Lines longer than 2×width (default: 200 chars) are truncated:
```
<first 100 chars>[...]<last 100 chars>
```

Use `-w 0` to disable line truncation.

### Output Format

**Default mode (no pattern):**
```
<first F lines>
[... truncated ...]
<last L lines>
```

**Pattern mode:**
```
<first F lines>
[... matches follow ...]
<match 1 with context>
[...]
<match 2 with context>
[... matches end ...]
<last L lines>
```

Notes:
- `[...]` appears between non-contiguous match groups (when contexts don't overlap)
- `[... matches end ...]` appears between the last match and the tail section
- Adjacent matches (overlapping contexts) are merged without `[...]`

If input is short enough (≤ F + L lines in default mode), output is unchanged with no separator.
