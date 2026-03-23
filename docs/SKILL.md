---
name: trunc
description: When CLI tools are available in the project
---

# trunc

Pipe long command output through `trunc` to avoid flooding context.

## Install

```bash
curl -Lo ~/.local/bin/trunc https://trunc.maxeonyx.com/releases/trunc-x86_64-linux
chmod +x ~/.local/bin/trunc
```

## Usage

```bash
some-command | trunc              # First 30 + last 30 lines
some-command | trunc -f 5 -l 5   # Custom counts
some-command | trunc "error"     # Pattern mode
```

If the pipeline is interrupted with `SIGINT` or `SIGTERM`, `trunc` flushes its buffered tail before exiting so the most recent lines are still visible.
