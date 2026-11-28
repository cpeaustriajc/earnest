# Changelog

Changes to Earnest on `main`, newest first. Nightly builds update this file automatically. For curated user-facing highlights, see the [docs changelog](https://earnest.cpeaustriajc.dev/changelog/).

## v0.1.0-nightly - 2026-11-26

### Added
- Check XState and smlang state machines with Earnest's search ([#26](https://github.com/cpeaustriajc/earnest/issues/26)) ([#42](https://github.com/cpeaustriajc/earnest/pull/42))
- Publish to crates.io and npm under @earnestjs, GPL/LGPL licensed ([#23](https://github.com/cpeaustriajc/earnest/pull/23))
- Build the command line each night and install it with a script that checks its checksum ([#17](https://github.com/cpeaustriajc/earnest/pull/17))
- Replace archsnap with earnest version 1, a model checker for agents ([#11](https://github.com/cpeaustriajc/earnest/pull/11))

### Changed
- Write a TypeScript model part as a function of m that takes the model ([#13](https://github.com/cpeaustriajc/earnest/pull/13))
- Rename the tool to Earnest: crate earnest-cli, command earnest ([#2](https://github.com/cpeaustriajc/earnest/pull/2))

### Performance
- Store states as packed rows, merge levels on all threads and run prepared code ([#15](https://github.com/cpeaustriajc/earnest/pull/15))

### Fixed
- Show the value when a test finds a list that should be empty ([#12](https://github.com/cpeaustriajc/earnest/pull/12))

### Documentation
- Lead the README with why, and chart earnest, TLC and Quint from 1k to 10M states ([#31](https://github.com/cpeaustriajc/earnest/pull/31))
- Add adaptive overview SVG, verification comparison, and prioritized README ([#25](https://github.com/cpeaustriajc/earnest/pull/25))
- Trim README and move reference docs to Starlight site ([#22](https://github.com/cpeaustriajc/earnest/pull/22))
- Add a Starlight docs site whose quickstart leads with a prompt for coding agents, and show the TLA+ and Quint comparison as charts ([#19](https://github.com/cpeaustriajc/earnest/pull/19))

### Maintenance & Dependencies
- Run just check at a stop only when the tree changed since the last pass ([#32](https://github.com/cpeaustriajc/earnest/pull/32))
- Publish through trusted publishing, with no registry secrets ([#24](https://github.com/cpeaustriajc/earnest/pull/24))
- Run every tool and dependency on its newest release ([#27](https://github.com/cpeaustriajc/earnest/pull/27))
- Name the nightly release after the workspace version: v0.1.0-nightly ([#20](https://github.com/cpeaustriajc/earnest/pull/20))
- Build the check job with Rust 1.100.0 ([#16](https://github.com/cpeaustriajc/earnest/pull/16))
- Update permissions in settings.json ([63ea078](https://github.com/cpeaustriajc/earnest/commit/63ea0783d2638fd9271c69847d54933917f3cff5))
- Lock the behavior tests and add the check recipes and hooks ([#9](https://github.com/cpeaustriajc/earnest/pull/9))
- Run just check in CI on every pull request ([#8](https://github.com/cpeaustriajc/earnest/pull/8))
- Prove the cron weekday index with Verus ([#7](https://github.com/cpeaustriajc/earnest/pull/7))
- Property tests for the scanner, import resolution and snapshot diff ([#6](https://github.com/cpeaustriajc/earnest/pull/6))
- Remove every comment and add the ast-grep comment rule ([#5](https://github.com/cpeaustriajc/earnest/pull/5))
- Add the Claude Code setup: project skills, test guard and compact hooks ([#4](https://github.com/cpeaustriajc/earnest/pull/4))
