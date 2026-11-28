//! Gives the binary its engine version: the package version and a fingerprint of the checker's
//! sources, so a cached result is never served by a build whose code differs. Gives it also the
//! line `--version` prints, which adds the commit the build was made from when there is one.

#[path = "src/sources.rs"]
mod sources;

use std::env;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

const COMMIT_LOGS: [&str; 2] = [".git/logs/HEAD", ".git/HEAD"];

fn commit(repository: &Path) -> Option<String> {
    if !repository.join(".git").exists() {
        return None;
    }
    let output = Command::new("git")
        .arg("-C")
        .arg(repository)
        .args(["rev-parse", "--short=12", "HEAD"])
        .output()
        .ok()?;
    let text = String::from_utf8(output.stdout).ok()?;
    let hash = text.trim();
    let whole = output.status.success()
        && !hash.is_empty()
        && hash.chars().all(|digit| digit.is_ascii_hexdigit());
    whole.then(|| hash.to_owned())
}

fn printed(version: &str, engine: &str, commit: Option<&str>) -> String {
    let sources = engine.rsplit('+').next().unwrap_or(engine);
    match commit {
        Some(hash) => format!("{version} (commit {hash}, sources {sources})"),
        None => format!("{version} (sources {sources})"),
    }
}

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
    if let Some(log) = COMMIT_LOGS
        .iter()
        .map(|path| repository.join(path))
        .find(|path| path.is_file())
    {
        writeln!(cargo, "cargo::rerun-if-changed={}", log.display())?;
    }
    writeln!(cargo, "cargo::rerun-if-changed=build.rs")?;
    let line = printed(&version, &engine, commit(&repository).as_deref());
    writeln!(cargo, "cargo::rustc-env=EARNEST_VERSION={line}")?;
    writeln!(cargo, "cargo::rustc-env=EARNEST_ENGINE={engine}")
}
