# trunc - TODO

## Phase 1: Specification (Current)

- [x] Create repository
- [x] Write VISION.md
- [x] Write comprehensive E2E tests (51 failing tests)
- [x] Create stub implementation that compiles but fails tests
- [x] Set up CI (GitHub Actions)
- [x] Set up release pipeline

## Phase 2: Implementation

- [ ] Add CLI argument parsing (clap)
- [ ] Implement line truncation (-w/--width)
- [ ] Implement basic truncation (first + last)
- [ ] Handle edge cases (short input, exact boundary, trailing newlines)
- [ ] Implement pattern matching mode
- [ ] Implement context around matches
- [ ] Implement match limit
- [ ] Handle overlapping regions (no duplicate lines)
- [ ] Add ellipsis separators between non-contiguous sections

## Phase 3: Polish

- [ ] Performance testing with large inputs
- [ ] Error handling and edge cases
- [ ] Release to crates.io
- [ ] Publish binaries via GitHub releases

## Future Ideas

- Line number display option (`-n`)
- Multiple patterns (OR matching)
- Invert match (`-v` like grep)
