use crate::cache::fingerprint_of;
use crate::places::Places;
use crate::quint::{Quint, cache_path, kept, scratch, shown_string, store};
use earnest_quint::Typechecked;
use std::fs;
use std::path::Path;
use std::process::Command;

pub(crate) fn typechecked(
    real_file: &Path,
    shown: &str,
    quint: &Quint,
    places: &Places,
) -> Option<Typechecked> {
    let key = format!(
        "{:016x}",
        fingerprint_of(&[quint.version.as_bytes(), &places.read.to_le_bytes()])
    );
    let target = cache_path(&format!(
        "typecheck-{:016x}.json",
        fingerprint_of(&[shown_string(real_file).as_bytes()])
    ));
    if let Some(text) = kept(&target, &key) {
        return Typechecked::read(&text);
    }
    let out = scratch("typecheck").ok()?;
    let ran = Command::new(&quint.program)
        .arg("typecheck")
        .arg(shown)
        .arg("--out")
        .arg(&out)
        .output();
    let text = fs::read_to_string(&out);
    fs::remove_file(&out).unwrap_or_default();
    if !ran.ok()?.status.success() {
        return None;
    }
    let text = text.ok()?;
    let read = Typechecked::read(&text)?;
    store(&target, &format!("{key}\n{text}"));
    Some(read)
}
