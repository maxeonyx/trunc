# trunc - Vision

**Smart truncation for pipe output.**

`trunc` is a pipe destination that intelligently shortens command output. It combines head, tail, and grep into a single tool optimized for when you need to see "just enough" of a command's output without being overwhelmed.

## Primary Use Case

AI agents need to read command output, but long outputs waste context tokens and obscure important information. `trunc` gives them exactly what they need:

- The beginning (often contains headers, initial state, or early errors)
- The end (often contains final results, summaries, or recent errors)
- Optionally: specific matches from the middle (when looking for something specific)

## Core Behavior

### Line Truncation

Long lines are truncated to show the first 100 and last 100 characters,
with a marker showing how many characters were removed:
```
<first 100 chars>[... 500 chars ...]<last 100 chars>
```

Lines are only truncated when the result would be strictly shorter than the
original (accounting for the marker length). Use `-w 0` to disable.

### Default Mode (No Pattern)

```
$ some-long-command | trunc
```

Shows:
1. First 30 lines
2. `[... 40 lines truncated ...]`
3. Last 30 lines

If the input is 60 lines or fewer, output is unchanged (no truncation marker).

### Pattern Mode

```
$ some-long-command | trunc "error"
```

Shows:
1. First 30 lines
2. `[... 36 lines truncated, match 1 shown ...]`
3. Up to 5 matches from the middle, each with 3 lines of context on either side
4. `[... 23 lines truncated, match 2 shown ...]` between non-contiguous match groups
5. `[... 48 lines and 208 matches truncated (213 total) ...]` before the tail
6. Last 30 lines

When all matches are shown, the end marker omits the match count.
When the match limit (-m) is hit, the last shown match says "match N/N".
When 0 matches found: `[... 980 lines truncated, 0 matches found ...]`

## Output Size Guarantees

With defaults, output size is bounded. The marker format is longer than before
(e.g. `[... 500 chars ...]` instead of `[...]`), but total output remains small:

| Mode | Max Lines | Notes |
|------|-----------|-------|
| Default | 61 | 30 first + 1 marker + 30 last |
| Pattern | ~101 | 30 first + 5×(1 marker + 7 context) + 1 end marker + 30 last |

## Design Principles

1. **Fast and simple.** Single binary, minimal dependencies, streams input.
2. **Streaming output.** First lines appear immediately; matches stream as found. Only the tail must wait for EOF.
3. **Predictable output size.** The user can calculate max output before running.
4. **Zero config for common case.** Defaults are sensible; options are rare.
5. **Grep-compatible patterns.** Regex syntax should feel familiar.

## Non-Goals

- Colorization or formatting (pipe to another tool if needed)
- File watching or tailing (use `tail -f`)
- Complex query languages (use `awk` or `jq`)
