---
title: Install
description: Install the nightly build of the earnest checker, or build it from source.
---

Early access ships as a nightly build of `main`. The install script downloads the archive for your machine and checks its SHA-256 against the `.sha256` file beside it. Then it installs `earnest` in `~/.local/bin`, or in `%LOCALAPPDATA%\Programs\earnest` on Windows.

```sh title="macOS and Linux"
curl -fsSL https://raw.githubusercontent.com/cpeaustriajc/earnest/main/install.sh | sh
```

```powershell title="Windows"
irm https://raw.githubusercontent.com/cpeaustriajc/earnest/main/install.ps1 | iex
```

Set `EARNEST_INSTALL_DIR` to install in another folder. Run the script again to update to the newest night.

## Supported machines

| System | CPU |
|---|---|
| Linux with glibc 2.39 or newer | x64, ARM64 |
| macOS | Apple silicon, Intel |
| Windows | x64 |

Linux with musl has no build. On such a machine, build from source.

## Build from source

```sh
cargo install --git https://github.com/cpeaustriajc/earnest earnest-cli
```

This puts `earnest` in `~/.cargo/bin`.

## The model libraries

- **TypeScript.** A model imports `m` from `@earnest/model`, which is not on npm yet. The checker reads the file's text and never resolves the import, so `earnest verify` works with no `node_modules`. Until the package ships, your editor shows the import as unresolved.
- **Rust.** A model uses the `earnest` crate. Add it from git:

  ```toml
  [dependencies]
  earnest = { git = "https://github.com/cpeaustriajc/earnest" }
  ```
