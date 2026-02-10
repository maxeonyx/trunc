# trunc - TODO

## Current: Informative Truncation Markers

See `TASK-informative-markers.ignore.md` for full requirements and test plan.

- [x] Fix pre-existing default mismatch (code says 30, docs/tests say 10) — 30/30 is correct
- [x] Within-line: `[... N chars ...]` format with char count
- [x] Within-line: only truncate when result is strictly shorter
- [x] Across-line: `[... N lines truncated ...]` with line count
- [x] Pattern mode: match position markers (`match K shown`, `match K/K shown`)
- [x] Pattern mode: total match count on end marker
- [x] Pattern mode: remaining match count on end marker
- [x] Pattern mode: `0 matches found` when none found
- [x] Update existing tests in e2e.rs for new marker formats
- [x] Recompute output size guarantee calculations

## Completed

- [x] Streaming output (first lines + matches stream immediately)
- [x] Create repository
- [x] Write VISION.md
- [x] Write comprehensive E2E tests
- [x] Implement CLI argument parsing (clap)
- [x] Implement line truncation (-w/--width)
- [x] Implement basic truncation (first + last)
- [x] Handle edge cases (short input, exact boundary, trailing newlines)
- [x] Implement pattern matching mode with context
- [x] Handle overlapping regions (no duplicate lines)
- [x] Set up CI (GitHub Actions)
- [x] Publish v0.1.0 via GitHub releases

## Future Ideas

- Line number display option (`-n`)
- Multiple patterns (OR matching)
- Invert match (`-v` like grep)
