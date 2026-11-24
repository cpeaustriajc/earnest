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

Each night that `main` moves, the `v0.1.0-nightly` pre-release on the [GitHub Releases](https://github.com/cpeaustriajc/earnest/releases) page is rebuilt from `main` and replaced. To verify a downloaded archive by hand, keep the archive and its `.sha256` file in the same folder and run:

```sh
sha256sum --check earnest-linux-x64-gnu.tar.gz.sha256   # Linux
shasum -a 256 --check earnest-darwin-arm64.tar.gz.sha256 # macOS
```

## Supported machines

| System | CPU |
|---|---|
| Linux with glibc 2.39 or newer | x64, ARM64 |
| macOS | Apple silicon, Intel |
| Windows | x64 |

Linux with musl has no prebuilt binary. On musl machines, build from source.

## Build from source

```sh
cargo install --git https://github.com/cpeaustriajc/earnest earnest-cli
```

From a local clone of the repository:

```sh
cargo install --path apps/earnest
```

Both install `earnest` in `~/.cargo/bin`.

## The model libraries

- **TypeScript:** A model imports `m` from `@earnest/model`, which is not on npm yet. The checker reads the file's text and never resolves the import on disk, so `earnest verify` works with no `node_modules`. The import matters only to your editor, TypeScript type checker, and Node.
- **Rust:** A model uses the `earnest` crate. Add it from git:

  ```toml
  [dependencies]
  earnest = { git = "https://github.com/cpeaustriajc/earnest" }
  ```

When early access ends, the checker and TypeScript library will also ship on npm (`npm install --save-dev @earnest/cli @earnest/model`, then `npx earnest verify`). `@earnest/cli` is a small Node launcher with per-platform optional binary packages, no install script, and no runtime download. See [`packages/cli/README.md`](https://github.com/cpeaustriajc/earnest/blob/main/packages/cli/README.md) and [`packages/model/README.md`](https://github.com/cpeaustriajc/earnest/blob/main/packages/model/README.md).
