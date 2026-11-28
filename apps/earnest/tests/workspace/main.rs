//! Checks on the repository's own layout: every workspace member takes the workspace lints, and
//! every crate under `crates/` has a test and a `README.md`.
//! Every path in the repository can also be checked out on Windows.

use std::collections::HashMap;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root is two folders above the CLI crate")
}

fn member_manifests() -> Vec<PathBuf> {
    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(cargo)
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(repository())
        .output()
        .expect("cargo metadata runs");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: Value =
        serde_json::from_slice(&output.stdout).expect("cargo metadata prints JSON");
    let members: Vec<&str> = metadata["workspace_members"]
        .as_array()
        .expect("cargo metadata lists the workspace members")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    metadata["packages"]
        .as_array()
        .expect("cargo metadata lists the packages")
        .iter()
        .filter(|package| {
            package["id"]
                .as_str()
                .is_some_and(|id| members.contains(&id))
        })
        .filter_map(|package| package["manifest_path"].as_str())
        .map(PathBuf::from)
        .collect()
}

fn crate_folders() -> Vec<PathBuf> {
    let crates = repository().join("crates");
    let Ok(entries) = fs::read_dir(&crates) else {
        return Vec::new();
    };
    let mut folders: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|folder| folder.join("Cargo.toml").is_file())
        .collect();
    folders.sort();
    folders
}

fn package_folders() -> Vec<PathBuf> {
    let packages = repository().join("packages");
    let Ok(entries) = fs::read_dir(&packages) else {
        return Vec::new();
    };
    let mut folders: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|folder| folder.join("package.json").is_file())
        .collect();
    folders.sort();
    folders
}

fn takes_workspace_lints(manifest: &str) -> bool {
    let compact: Vec<String> = manifest
        .lines()
        .map(|line| line.split_whitespace().collect::<String>())
        .collect();
    let in_table = compact
        .iter()
        .skip_while(|line| line.as_str() != "[lints]")
        .skip(1)
        .take_while(|line| !line.starts_with('['))
        .any(|line| line == "workspace=true");
    let inline = compact
        .iter()
        .any(|line| line == "lints.workspace=true" || line == "lints={workspace=true}");
    in_table || inline
}

fn rust_files(folder: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut files = Vec::new();
    for path in entries.filter_map(Result::ok).map(|entry| entry.path()) {
        if path.is_dir() {
            files.extend(rust_files(&path));
        } else if path.extension() == Some(OsStr::new("rs")) {
            files.push(path);
        }
    }
    files
}

fn has_a_test(folder: &Path) -> bool {
    let has_test_file = !rust_files(&folder.join("tests")).is_empty();
    let has_unit_test = rust_files(&folder.join("src"))
        .iter()
        .any(|file| fs::read_to_string(file).is_ok_and(|source| source.contains("#[test]")));
    has_test_file || has_unit_test
}

const LEFT_OUT_IN_ANY_FOLDER: [&str; 5] = [
    ".git",
    "node_modules",
    ".earnest",
    "__pycache__",
    "_apalache-out",
];
const LEFT_OUT_AT_ROOT: [&str; 6] = [
    "target",
    "mutants.out",
    ".claude/worktrees",
    "apps/docs/dist",
    "apps/docs/.astro",
    "apps/docs/.cloudflare",
];
const LEFT_OUT_IN_EACH_PACKAGE: [&str; 2] = ["dist", ".package.next.json"];
const CHARACTERS_WINDOWS_REFUSES: [char; 8] = ['<', '>', ':', '"', '|', '?', '*', '\\'];
const DEVICES_WINDOWS_RESERVES: [&str; 4] = ["CON", "PRN", "AUX", "NUL"];
const NUMBERED_DEVICES_WINDOWS_RESERVES: [&str; 2] = ["COM", "LPT"];
const DEVICE_NUMBERS: [char; 12] = ['1', '2', '3', '4', '5', '6', '7', '8', '9', '¹', '²', '³'];

fn build_output(relative: &str) -> bool {
    let parts: Vec<&str> = relative.split('/').collect();
    let last = parts.last().copied().unwrap_or_default();
    LEFT_OUT_IN_ANY_FOLDER.contains(&last)
        || LEFT_OUT_AT_ROOT.contains(&relative)
        || matches!(parts.as_slice(), ["packages", _, part] if LEFT_OUT_IN_EACH_PACKAGE.contains(part))
}

fn reserved_device(stem: &str) -> bool {
    let upper = stem.to_uppercase();
    let numbered = NUMBERED_DEVICES_WINDOWS_RESERVES.iter().any(|device| {
        upper.strip_prefix(device).is_some_and(|number| {
            let mut characters = number.chars();
            characters
                .next()
                .is_some_and(|first| DEVICE_NUMBERS.contains(&first))
                && characters.next().is_none()
        })
    });
    DEVICES_WINDOWS_RESERVES.contains(&upper.as_str()) || numbered
}

fn why_windows_refuses(name: &str) -> Option<String> {
    if let Some(refused) = name
        .chars()
        .find(|character| CHARACTERS_WINDOWS_REFUSES.contains(character) || character.is_control())
    {
        return Some(format!("holds {refused:?}"));
    }
    if name.ends_with('.') || name.ends_with(' ') {
        return Some("ends in a dot or a space".to_owned());
    }
    let stem = name.split('.').next().unwrap_or(name).trim_end_matches(' ');
    reserved_device(stem).then(|| format!("is the device `{stem}` Windows reserves"))
}

#[derive(Default)]
struct Layout {
    paths_seen: Vec<String>,
    refused: Vec<String>,
    same_but_for_case: Vec<String>,
}

impl Layout {
    fn of_repository() -> Self {
        let root = repository();
        let mut layout = Self::default();
        layout.walk(&root, &root);
        layout
    }

    fn walk(&mut self, root: &Path, folder: &Path) {
        let entries = fs::read_dir(folder)
            .unwrap_or_else(|error| panic!("{} can't be listed: {error}", folder.display()));
        let mut entries: Vec<fs::DirEntry> = entries
            .map(|entry| {
                entry.unwrap_or_else(|error| {
                    panic!("an entry of {} can't be read: {error}", folder.display())
                })
            })
            .collect();
        entries.sort_by_key(fs::DirEntry::file_name);
        let mut by_lower_case: HashMap<String, String> = HashMap::new();
        for entry in entries {
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .expect("the walk stays under the repository root")
                .components()
                .map(|part| part.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            if build_output(&relative) {
                continue;
            }
            match entry.file_name().to_str() {
                None => self
                    .refused
                    .push(format!("{relative}: its name is not UTF-8")),
                Some(name) => {
                    if let Some(reason) = why_windows_refuses(name) {
                        self.refused.push(format!("{relative}: {reason}"));
                    }
                    if let Some(other) = by_lower_case.insert(name.to_lowercase(), relative.clone())
                    {
                        self.same_but_for_case
                            .push(format!("{other} and {relative}"));
                    }
                }
            }
            let is_folder = entry
                .file_type()
                .unwrap_or_else(|error| panic!("{relative} can't be read: {error}"))
                .is_dir();
            self.paths_seen.push(relative);
            if is_folder {
                self.walk(root, &path);
            }
        }
    }
}

fn names(folders: &[PathBuf]) -> String {
    folders
        .iter()
        .map(|folder| folder.display().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

#[test]
fn every_workspace_member_takes_the_workspace_lints() {
    let manifests = member_manifests();
    assert!(!manifests.is_empty(), "the workspace has no members");
    let without: Vec<PathBuf> = manifests
        .into_iter()
        .filter(|manifest| {
            let text = fs::read_to_string(manifest).expect("a member's Cargo.toml can be read");
            !takes_workspace_lints(&text)
        })
        .collect();
    assert!(
        without.is_empty(),
        "these manifests lack `[lints] workspace = true`: {}",
        names(&without)
    );
}

#[test]
fn every_crate_has_a_test() {
    let without: Vec<PathBuf> = crate_folders()
        .into_iter()
        .filter(|folder| !has_a_test(folder))
        .collect();
    assert!(
        without.is_empty(),
        "these crates have neither a `tests/` folder with a test nor a `#[test]` in `src/`: {}",
        names(&without)
    );
}

#[test]
fn every_crate_has_a_readme() {
    let without: Vec<PathBuf> = crate_folders()
        .into_iter()
        .filter(|folder| !folder.join("README.md").is_file())
        .collect();
    assert!(
        without.is_empty(),
        "these crates have no `README.md`: {}",
        names(&without)
    );
}

#[test]
fn every_package_has_a_readme() {
    let folders = package_folders();
    assert!(
        !folders.is_empty(),
        "`packages/` holds the packages published to npm"
    );
    let without: Vec<PathBuf> = folders
        .into_iter()
        .filter(|folder| !folder.join("README.md").is_file())
        .collect();
    assert!(
        without.is_empty(),
        "these packages have no `README.md`: {}",
        names(&without)
    );
}

#[test]
fn the_lint_check_reads_each_way_of_writing_the_table() {
    assert!(
        takes_workspace_lints("[package]\nname = \"a\"\n\n[lints]\nworkspace = true\n"),
        "a `[lints]` table with `workspace = true` takes the workspace lints"
    );
    assert!(
        takes_workspace_lints("[package]\nname = \"a\"\nlints.workspace = true\n"),
        "a dotted `lints.workspace = true` takes the workspace lints"
    );
    assert!(
        !takes_workspace_lints("[package]\nname = \"a\"\n\n[dependencies]\nworkspace = true\n"),
        "`workspace = true` under another table does not take the workspace lints"
    );
    assert!(
        !takes_workspace_lints("[lints]\n\n[lints.clippy]\nall = \"warn\"\n"),
        "a `[lints]` table of its own lints does not take the workspace lints"
    );
}

#[test]
fn every_path_in_the_repository_can_be_checked_out_on_windows() {
    let layout = Layout::of_repository();
    assert!(
        layout.paths_seen.iter().any(|path| path == "Cargo.toml")
            && layout
                .paths_seen
                .iter()
                .any(|path| path == "apps/earnest/tests/workspace/main.rs"),
        "the walk starts at the workspace root and reaches this file"
    );
    assert!(
        layout.refused.is_empty(),
        "Windows refuses these names, so `git checkout` stops there: {:#?}",
        layout.refused
    );
    assert!(
        layout.same_but_for_case.is_empty(),
        "these paths differ only by letter case, so Windows keeps one of each pair: {:#?}",
        layout.same_but_for_case
    );
}

#[test]
fn the_windows_check_refuses_each_kind_of_name_windows_refuses() {
    for refused in [
        "Named@S::ok@.json",
        "a<b",
        "a>b",
        "a\"b",
        "a|b",
        "a?b",
        "a*b",
        "a\\b",
        "a\tb",
        "notes.",
        "notes ",
        "CON",
        "con.txt",
        "Aux.tar.gz",
        "nul ",
        "PRN .md",
        "com1",
        "LPT9.json",
        "COM\u{b9}",
    ] {
        assert!(
            why_windows_refuses(refused).is_some(),
            "Windows refuses {refused:?}"
        );
    }
    for kept in [
        "Named@S.ok@.json",
        "console.rs",
        "COM10",
        "LPT0",
        "com",
        "nul_fixture.json",
        ".gitignore",
        "a.b.c",
    ] {
        assert_eq!(why_windows_refuses(kept), None, "Windows keeps {kept:?}");
    }
}
