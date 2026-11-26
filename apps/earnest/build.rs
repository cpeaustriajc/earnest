//! Gives the binary its engine version: the package version and a fingerprint of the checker's
//! sources, so a cached result is never served by a build whose code differs. It also names the
//! commit in `earnest --version` when `EARNEST_BUILD_COMMIT` is set for the build.

#[path = "src/sources.rs"]
mod sources;

use std::env;
use std::io::{self, Write};
use std::path::Path;

fn main() -> io::Result<()> {
    let manifest = env::var("CARGO_MANIFEST_DIR").map_err(io::Error::other)?;
    let version = env::var("CARGO_PKG_VERSION").map_err(io::Error::other)?;
    let package = Path::new(&manifest);
    let workspace = package.join("../..");
    let repository = if workspace.join("apps/earnest/Cargo.toml").is_file() {
        workspace.canonicalize()?
    } else {
        package.to_path_buf()
    };
    let engine = sources::engine(&version, &repository)?;
    let mut cargo = io::stdout().lock();
    for root in sources::roots(&repository)? {
        writeln!(cargo, "cargo::rerun-if-changed={}", root.display())?;
    }
    writeln!(cargo, "cargo::rerun-if-changed=build.rs")?;
    writeln!(cargo, "cargo::rerun-if-env-changed=EARNEST_BUILD_COMMIT")?;
    let commit_suffix = match env::var("EARNEST_BUILD_COMMIT") {
        Ok(commit) if !commit.trim().is_empty() => format!(" (commit {})", commit.trim()),
        _ => String::new(),
    };
    writeln!(
        cargo,
        "cargo::rustc-env=EARNEST_COMMIT_SUFFIX={commit_suffix}"
    )?;
    writeln!(cargo, "cargo::rustc-env=EARNEST_ENGINE={engine}")
}
