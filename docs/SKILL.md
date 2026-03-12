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
some-command | trunc              # First 10 + last 10 lines
some-command | trunc -f 5 -l 5   # Custom counts
some-command | trunc "error"     # Pattern mode
```
