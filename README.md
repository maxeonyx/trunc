# trunc

Smart truncation for pipe output. Like `head` + `tail` with optional grep-style pattern matching.

**Site:** https://trunc.maxeonyx.com

`trunc` accepts arbitrary piped bytes. It splits stdin on newlines and lossily decodes invalid UTF-8 so unexpected output does not fail the pipeline.

## Install

### Pre-built binaries

Download a pre-built binary from https://trunc.maxeonyx.com or browse the GitHub Releases at https://github.com/maxeonyx/trunc/releases/latest.

Available binaries:

- Linux x86_64: `trunc-x86_64-linux`
- Linux x86_64-musl: `trunc-x86_64-linux-musl`
- Linux aarch64: `trunc-aarch64-linux`
- macOS x86_64: `trunc-x86_64-macos`
- macOS aarch64: `trunc-aarch64-macos`
- Windows x86_64: `trunc-x86_64-windows.exe`

On Unix, make the downloaded binary executable and put it on your `PATH`.

### cargo install

This project is not currently published to crates.io, so `cargo install trunc` is not available for this repo.

### Build from source

```bash
git clone git@github.com:maxeonyx/trunc.git
cd trunc
cargo build --release
```

## Usage

### Basic truncation

Show first 30 and last 30 lines:

```bash
some-command | trunc
```

Output:

```
line 1
...
line 30
[... 40 lines truncated ...]
line 71
...
line 100
```

### Custom line counts

```bash
some-command | trunc -f 5 -l 5    # 5 lines at start and end
some-command | trunc -f 20        # 20 at start, default 30 at end
some-command | trunc -l 3         # default 30 at start, 3 at end
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
line 30
[... 12 lines truncated, match 1 shown ...]
line 43
line 44
line 45: error occurred here
line 46
line 47
line 48
[... 42 lines truncated ...]
line 71
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
<first 100 chars>[... 500 chars ...]<last 100 chars>
```

### Interrupted pipelines

If `trunc` receives `SIGINT` or `SIGTERM`, it flushes the tail buffer before exiting so you still see the most recent lines received:

```text
<already streamed output>
[... 48 lines truncated, interrupted ...]
<last lines received before interruption>
```

If the input is interrupted before it grows beyond `first + last`, `trunc` just outputs what it has received so far with no marker.

## Output Size Guarantees

With defaults, output is bounded to predictable sizes:

| Mode | Max Lines | Notes |
| --- | --- | --- |
| Default | 61 | 30 first + 1 marker + 30 last |
| Pattern | ~101 | 30 first + 5×(1 marker + 7 context) + 1 end marker + 30 last |

These bounds apply to the portion of the stream actually received. If the producer is interrupted, `trunc` finalizes from the data seen so far.

## Why?

Built for AI agents that need to read command output without wasting context tokens. Predictable output size, zero configuration for the common case.

## License

MIT
