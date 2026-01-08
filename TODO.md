# trunc - TODO

## Completed

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

## Current: Streaming Output

- [ ] Stream first N lines immediately as they arrive
- [ ] Stream matches (with context) as they are found
- [ ] Only buffer the last M lines (ring buffer)
- [ ] Tests: `streaming::first_lines_stream_immediately`, `streaming::matches_stream_as_they_arrive`

## Future Ideas

- Line number display option (`-n`)
- Multiple patterns (OR matching)
- Invert match (`-v` like grep)
