//! Acceptance tests for the `earnest` binary, one or more for each requirement of the version 1
//! spec. Every test runs the real binary in its own temporary folder on the models in
//! `tasks/conformance/models`, and takes its expected values from `tasks/conformance/README.md`.

mod cache;
mod changes;
mod chart;
mod expected;
mod harness;
mod limits;
mod offline;
mod quint;
mod quint_actions;
mod quint_arguments;
mod quint_checks;
mod quint_data;
mod quint_expected;
mod quint_fairness;
mod quint_first;
mod quint_imports;
mod quint_picks;
mod quint_problems;
mod quint_reads;
mod quint_runs;
mod quint_setups;
mod quint_temporal;
mod replay;
mod results;
mod runs;
mod runs_eventually;
mod runs_rust;
mod runs_setups;
mod select;
mod support;
mod tla;
mod twins;
mod typescript;
mod unreadable;
mod verify;
