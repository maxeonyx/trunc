---
name: trunc
description: When CLI tools are available in the project
---

# trunc

Pipe long command output through `trunc` to avoid flooding context.

## Install

Curl + chmod the binary to ~/.local/bin from github maxeonyx/trunc latest release.

## Usage

```bash
some-command | trunc              # First 10 + last 10 lines
some-command | trunc -f 5 -l 5   # Custom counts
some-command | trunc "error"     # Pattern mode
```
