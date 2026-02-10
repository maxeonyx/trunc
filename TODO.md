# trunc - TODO

## Current: Informative Truncation Markers

See `TASK-informative-markers.ignore.md` for full requirements and test plan.

- [ ] Fix pre-existing default mismatch (code says 30, docs/tests say 10)
- [ ] Within-line: `[... N chars ...]` format with char count
- [ ] Within-line: only truncate when result is strictly shorter
- [ ] Across-line: `[... N lines truncated ...]` with line count
- [ ] Pattern mode: match position markers (`match K/M shown`)
- [ ] Pattern mode: total match count on first marker
- [ ] Pattern mode: remaining match count on end marker
- [ ] Pattern mode: `0 matches found` when none found
- [ ] Update existing tests in e2e.rs for new marker formats
- [ ] Update output size guarantee calculations

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
