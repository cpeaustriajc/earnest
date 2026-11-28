use crate::read;

const READ_BEFORE: [&str; 74] = [
    "crates/earnest_quint/fixtures/assertbroken.json",
    "crates/earnest_quint/fixtures/asserts.json",
    "crates/earnest_quint/fixtures/drain.json",
    "crates/earnest_quint/fixtures/draining.json",
    "crates/earnest_quint/fixtures/emptyhead.json",
    "crates/earnest_quint/fixtures/everything.json",
    "crates/earnest_quint/fixtures/fairconfirmed.json",
    "crates/earnest_quint/fixtures/fairtuple.json",
    "crates/earnest_quint/fixtures/fixes.json",
    "crates/earnest_quint/fixtures/flags.json",
    "crates/earnest_quint/fixtures/fromstate.json",
    "crates/earnest_quint/fixtures/grows.json",
    "crates/earnest_quint/fixtures/initpick.json",
    "crates/earnest_quint/fixtures/keys.json",
    "crates/earnest_quint/fixtures/lookalike.json",
    "crates/earnest_quint/fixtures/matches.json",
    "crates/earnest_quint/fixtures/missingkey.json",
    "crates/earnest_quint/fixtures/mixedpick.json",
    "crates/earnest_quint/fixtures/onlyfault.json",
    "crates/earnest_quint/fixtures/params.json",
    "crates/earnest_quint/fixtures/pass.json",
    "crates/earnest_quint/fixtures/pastend.json",
    "crates/earnest_quint/fixtures/played.json",
    "crates/earnest_quint/fixtures/power.json",
    "crates/earnest_quint/fixtures/printed.json",
    "crates/earnest_quint/fixtures/reads.json",
    "crates/earnest_quint/fixtures/serve.json",
    "crates/earnest_quint/fixtures/servedruns.json",
    "crates/earnest_quint/fixtures/servedsetup.json",
    "crates/earnest_quint/fixtures/setvar.json",
    "crates/earnest_quint/fixtures/table.json",
    "crates/earnest_quint/fixtures/tested.json",
    "crates/earnest_quint/fixtures/text.json",
    "crates/earnest_quint/fixtures/values.json",
    "crates/earnest_quint/fixtures/widening.json",
    "tasks/quint/models/bank.json",
    "tasks/quint/models/counter.json",
    "tasks/quint/models/dead.json",
    "tasks/quint/models/largebits.json",
    "tasks/quint/models/mixed.json",
    "tasks/quint/models/record.json",
    "tasks/quint/models/runs.json",
    "tasks/quint/models/runs_ints.json",
    "tasks/quint/models/shapes.json",
    "tasks/quint/models/smallbits.json",
    "tasks/quint/models/starts.json",
    "tasks/quint/models/stuck.json",
    "tasks/quint/models/swing.json",
    "tasks/quint/probe/fair2.json",
    "tasks/quint/setups/fixtures/clock.Large.json",
    "tasks/quint/setups/fixtures/clock.Small.json",
    "tasks/quint/setups/fixtures/clockbits.SmallBits.json",
    "tasks/quint/setups/fixtures/compiled.Nested.json",
    "tasks/quint/setups/fixtures/compiled.Renamed.json",
    "tasks/quint/setups/fixtures/compiled.Single.json",
    "tasks/quint/setups/fixtures/ints.High.json",
    "tasks/quint/setups/fixtures/ints.Low.json",
    "tasks/quint/setups/fixtures/kinds.Setup.json",
    "tasks/quint/setups/fixtures/lamp.Blinding.json",
    "tasks/quint/setups/fixtures/lamp.Bright.json",
    "tasks/quint/setups/fixtures/lamp.Dim.json",
    "tasks/quint/setups/fixtures/more.ByName.json",
    "tasks/quint/setups/fixtures/more.ByOwn.json",
    "tasks/quint/setups/fixtures/more.Named.json",
    "tasks/quint/setups/fixtures/more.Pair.json",
    "tasks/quint/setups/fixtures/more.Plain.json",
    "tasks/quint/setups/fixtures/more.Reexported.json",
    "tasks/quint/setups/fixtures/more.Two.json",
    "tasks/quint/setups/fixtures/own.Tight.json",
    "tasks/quint/setups/fixtures/pure.Four.json",
    "tasks/quint/setups/fixtures/readsetup.ReadSetup.json",
    "tasks/quint/setups/fixtures/runs.One.json",
    "tasks/quint/setups/fixtures/runs.Three.json",
    "tasks/quint/setups/fixtures/shadow.S.json",
];

#[test]
fn every_compiled_fixture_the_reader_took_before_runs_had_their_own_variables_is_still_read() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let refused: Vec<String> = READ_BEFORE
        .iter()
        .filter_map(|path| {
            let source = std::fs::read_to_string(format!("{root}/{path}"))
                .unwrap_or_else(|error| panic!("{path} can't be read: {error}"));
            read(path, &source)
                .err()
                .map(|problems| format!("{path}: {problems:?}"))
        })
        .collect();
    assert!(
        refused.is_empty(),
        "read by 7f18149, so read now: {refused:#?}"
    );
}

#[test]
fn every_compiled_sample_of_a_shape_a_project_writes_is_still_read() {
    let folder = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tasks/quint/samples/fixtures/compiled"
    );
    let mut paths: Vec<std::path::PathBuf> = std::fs::read_dir(folder)
        .unwrap_or_else(|error| panic!("{folder} can't be listed: {error}"))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "{folder} holds the compiled samples");
    let refused: Vec<String> = paths
        .iter()
        .filter_map(|path| {
            let source = std::fs::read_to_string(path)
                .unwrap_or_else(|error| panic!("{} can't be read: {error}", path.display()));
            read(&path.display().to_string(), &source)
                .err()
                .map(|problems| format!("{}: {problems:?}", path.display()))
        })
        .collect();
    assert!(refused.is_empty(), "a sample stopped reading: {refused:#?}");
}
