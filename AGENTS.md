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

## Implementation Workflow

When implementing a task from `TODO.md` or a `TASK-*.ignore.md` file:

1. **Read the task file** and `AGENTS.md` first — understand requirements before writing code
2. **Run existing tests** to establish baseline — note which pass/fail
3. **One piece at a time** — implement one marker format change, verify tests pass, commit
4. **Failing tests first** — new test skeletons in `tests/informative_markers.rs` should fail before you write the code that makes them pass. Commit the failing test separately.
5. **Update existing tests** — tests in `tests/e2e.rs` that assert on old marker formats must be updated to match new formats. Do this alongside each implementation step.
6. **Commit and push frequently** — after each piece is verified working
7. **Update `TODO.md`** — check off items as you complete them
8. **Update docs when behavior changes** — VISION.md, AGENTS.md CLI spec, and README if it exists
9. **Run `cargo fmt` and `cargo clippy`** before every commit

## Key Files

- `src/main.rs` - Entry point and CLI parsing
- `src/lib.rs` - Core logic (if we split it out)
- `tests/e2e.rs` - End-to-end tests (existing behavior)
- `tests/informative_markers.rs` - Tests for informative marker formats (new)
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

Lines are truncated only when doing so makes the output strictly shorter.
The marker includes the count of characters removed:
```
<first 100 chars>[... 500 chars ...]<last 100 chars>
```

Use `-w 0` to disable line truncation.

### Output Format

All markers include the count of lines truncated. In pattern mode, markers
also communicate match position and totals.

**Default mode (no pattern):**
```
<first F lines>
[... 80 lines truncated ...]
<last L lines>
```

**Pattern mode (5 shown out of 213 total):**
```
<first F lines>
[... 36 lines truncated, match 1 shown ...]
<context + match 1>
[... 23 lines truncated, match 2 shown ...]
<context + match 2>
[... 31 lines truncated, match 5/5 shown ...]
<context + match 5>
[... 48 lines and 208 matches truncated (213 total) ...]
<last L lines>
```

**Pattern mode (all matches shown, e.g. 1 match):**
```
<first F lines>
[... 24 lines truncated, match 1 shown ...]
<context + match>
[... 48 lines truncated ...]
<last L lines>
```

**Pattern mode (0 matches found):**
```
<first F lines>
[... 980 lines truncated, 0 matches found ...]
<last L lines>
```

Notes:
- The "(N total)" annotation only appears on the end marker, when total > shown
- The "N/N" notation only appears when the match limit (-m) is hit — otherwise just "match N"
- Adjacent matches (overlapping contexts) are merged without a marker between them
- If input is short enough (≤ F + L lines), output is unchanged with no separator
