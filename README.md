# trunc

Smart truncation for pipe output. Like `head` + `tail` with optional grep-style pattern matching.

## Installation

```bash
cargo install trunc
```

## Usage

### Basic truncation

Show first 10 and last 10 lines:

```bash
some-command | trunc
```

Output:
```
line 1
line 2
...
line 10
... truncated ...
line 91
line 92
...
line 100
```

### Custom line counts

```bash
some-command | trunc -f 5 -l 5    # 5 lines at start and end
some-command | trunc -f 20        # 20 at start, default 10 at end
some-command | trunc -l 3         # default 10 at start, 3 at end
some-command | trunc --first 5 --last 5  # long form
some-command | trunc --head 5 --tail 5   # aliases for head/tail fans
```

### Pattern mode

Show matches from the middle with context:

```bash
some-command | trunc "error"
```

Output:
```
line 1
...
line 10
... matches ...
line 43
line 44
line 45: error occurred here
line 46
line 47
line 48
line 91
...
line 100
```

### Pattern mode options

```bash
trunc -m 10 "error"      # show up to 10 matches (default: 5)
trunc -C 5 "error"       # 5 lines of context per match (default: 3)
trunc -C 0 "error"       # no context, just matching lines
```

### Line truncation

Long lines (>200 chars) are automatically truncated:

```bash
some-command | trunc           # first/last 100 chars per line
some-command | trunc -w 50     # first/last 50 chars per line
some-command | trunc -w 0      # disable line truncation
```

Output for long lines:
```
<first 100 chars>[...]<last 100 chars>
```

## Output Size Guarantees

With defaults, output is bounded to predictable sizes:

| Mode | Max Lines | Max Chars |
|------|-----------|-----------|
| Default | 21 | ~4.3 KB |
| Pattern | 60 | ~12.4 KB |

## Why?

Built for AI agents that need to read command output without wasting context tokens. Predictable output size, zero configuration for the common case.

## License

MIT OR Apache-2.0
