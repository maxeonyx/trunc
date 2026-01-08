# trunc - Vision

**Smart truncation for pipe output.**

`trunc` is a pipe destination that intelligently shortens command output. It combines head, tail, and grep into a single tool optimized for when you need to see "just enough" of a command's output without being overwhelmed.

## Primary Use Case

AI agents need to read command output, but long outputs waste context tokens and obscure important information. `trunc` gives them exactly what they need:

- The beginning (often contains headers, initial state, or early errors)
- The end (often contains final results, summaries, or recent errors)
- Optionally: specific matches from the middle (when looking for something specific)

## Core Behavior

### Default Mode (No Pattern)

```
$ some-long-command | trunc
```

Shows:
1. First 10 lines
2. `... truncated ...`
3. Last 10 lines

If the input is 20 lines or fewer, output is unchanged (no truncation marker).

### Pattern Mode

```
$ some-long-command | trunc "error"
```

Shows:
1. First 10 lines
2. `... matches ...`
3. Up to 5 matches from the middle, each with 3 lines of context on either side
4. Last 10 lines

The worst-case output in pattern mode is 56 lines:
- 10 (start) + 1 (separator) + 35 (5 matches × 7 lines) + 10 (end) = 56

## Design Principles

1. **Fast and simple.** Single binary, minimal dependencies, streams input.
2. **Predictable output size.** The user can calculate max output before running.
3. **Zero config for common case.** Defaults are sensible; options are rare.
4. **Grep-compatible patterns.** Regex syntax should feel familiar.

## Non-Goals

- Colorization or formatting (pipe to another tool if needed)
- File watching or tailing (use `tail -f`)
- Complex query languages (use `awk` or `jq`)
