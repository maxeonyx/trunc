# trunc - Agent Instructions

This repository is self-contained for development. A standalone clone must build, test, and release without an `agent-tools` checkout.

## TDD ratchet — read before testing

Run `cargo ratchet`, not plain `cargo test`. A new test must be red when first introduced and committed as `pending`; that expected red test keeps CI green. A new test must not pass when first introduced—doing so makes the ratchet and CI red. Implement only after the red commit, then rerun the ratchet and commit the promotion to `passing`.

## Integration workflow

Run `devenv test` before committing and pushing; it includes `actionlint`, so workflow syntax is checked offline. Source CI does not run on push. Open a pull request, merge current `main` into the feature branch, then explicitly dispatch:

```bash
gh workflow run ci.yml --ref <feature-branch> -f pr_number=<number>
```

The repository-serialized run records the required `Ready` check, builds the release artifacts, auto-merges the pull request, publishes those same artifacts, and records `integrated-ci` on the exact merge commit.

## Project Overview

`trunc` is a Rust CLI tool for truncating pipe output. It shows the first N and last M lines, with an optional pattern-matching mode that extracts matches from the middle.

## Development Commands

```bash
# Run tests
cargo ratchet

# Run a specific test while debugging (the full ratchet remains the gate)
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

1. Read stdin line by line, decoding each line lossily from bytes so arbitrary command output does not fail on invalid UTF-8
2. Buffer the first N lines (head)
3. Maintain a ring buffer of the last M lines (tail)
4. If pattern mode: also track matches with context
5. On EOF or interruption: finalize from accumulated state and output any remaining marker + tail

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
- `.github/workflows/ci.yml` - CI/CD pipeline (check, build, release, pages — all in one file)

## CI Pipeline

Source CI runs only when explicitly dispatched for an open pull request. It is serialized per repository, requires the branch to contain current `main`, and auto-merges only after the Ready and build jobs pass. Everything is in `.github/workflows/ci.yml`.

1. **Ready** - PR/base validation, actionlint, version enforcement, format, clippy, and the test ratchet
2. **Build** - Linux and Windows release binaries from the validated PR head
3. **Merge** - Enables auto-merge and records the exact merge commit
4. **Release and Pages** - Publishes the already-built artifacts from that same run
5. **Integrated** - Records `integrated-ci` on the merge commit

## Release Pipeline

Releases happen automatically inside the explicitly dispatched integration run.

**Version-bump enforcement:** The Ready job rejects any version whose release tag already exists. Bump `Cargo.toml`, `Cargo.lock`, and `docs/version.json` together before dispatching.

**Release creation:** The Release job creates a new tag at the merge commit and attaches the validated build artifacts.

**Workflow:** Make changes → run `devenv test` → bump the version → open the PR → merge current main → dispatch CI.

There is no crates.io publishing step.

## CLI Specification

```
trunc [OPTIONS] [PATTERN]

Arguments:
  [PATTERN]  Regex pattern to search for in the middle section

Options:
  -f, --first <N>     Number of lines to show from start (default: 30)
  -l, --last <N>      Number of lines to show from end (default: 30)
  -H, --head <N>      Alias for --first
  -T, --tail <N>      Alias for --last
  -m, --matches <N>       Match head/tail count — sets both --match-first and --match-last (default: 3)
  --match-first <N>       First N matches to show from middle (overrides -m for this side)
  --match-last <N>        Last N matches to show from middle (overrides -m for this side)
  -C, --context <N>       Lines of context around each match (default: 3)
  -w, --width <N>         Chars to show at start/end of long lines (default: 100, 0 = no limit)
  -h, --help              Print help
  -V, --version           Print version
```

### Line Truncation

Lines are truncated only when doing so makes the output strictly shorter. The marker includes the count of characters removed:

```
<first 100 chars>[... 500 chars ...]<last 100 chars>
```

Use `-w 0` to disable line truncation.

### Output Format

All markers include the count of lines truncated. In pattern mode, markers also communicate match position and totals.

**Default mode (no pattern):**

```
<first F lines>
[... 80 lines truncated ...]
<last L lines>
```

**Pattern mode (6 shown out of 60 total, first 3 + last 3):**

```
<first F lines>
[... 36 lines truncated, match 1 shown ...]
<context + match 1>
[... 23 lines truncated, match 2 shown ...]
<context + match 2>
[... 31 lines truncated, match 3 shown ...]
<context + match 3>
[... 412 lines and 54 matches truncated, match 58 shown ...]
<context + match 58>
[... 18 lines truncated, match 59 shown ...]
<context + match 59>
[... 27 lines truncated, match 60 shown ...]
<context + match 60>
[... 48 lines truncated ...]
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

**Interrupted before EOF:**

```
<already streamed output>
[... 48 lines truncated, interrupted ...]
<last L lines received before interruption>
```

Notes:

- Pattern matches follow head/tail philosophy: show first N and last M matches, elide the middle
- Early matches (match-first) stream immediately; recent matches (match-last) are buffered until EOF/interruption, like tail lines
- The transition marker between head and tail matches reports hidden matches in that gap
- The "(N total)" annotation only appears on the end marker, when total > shown
- Adjacent matches (overlapping contexts) are merged without a marker between them
- If input is short enough (≤ F + L lines), output is unchanged with no separator
- On `SIGINT`/`SIGTERM`, `trunc` flushes the tail buffer before exiting and uses exit codes 130/143
