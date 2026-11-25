Check one part of this project with Earnest, a model checker.

A model is a small TypeScript or Rust file, in this project's language, with fields, actions and rules. `earnest verify` visits every reachable state and prints the shortest path to each broken rule. Earnest is in early access: nightly builds only, nothing on npm. Fetch the language reference for this project before writing the model:

- TypeScript: https://earnest.cpeaustriajc.dev/_llms-txt/typescript-models.txt
- Rust: https://earnest.cpeaustriajc.dev/_llms-txt/rust-models.txt
- Full index: https://earnest.cpeaustriajc.dev/llms.txt

If the project already has tests around a state machine library, an integration runs the search from those tests instead of a model file. Earnest has one for Rust's smlang: https://earnest.cpeaustriajc.dev/integrations/smlang/

1. Install: `curl -fsSL https://raw.githubusercontent.com/cpeaustriajc/earnest/main/install.sh | sh` (Windows: `irm https://raw.githubusercontent.com/cpeaustriajc/earnest/main/install.ps1 | iex`). It installs to `~/.local/bin`.
2. Add `.earnest/` to `.gitignore`.
3. Pick one stateful piece with rules that matter, such as a checkout, a queue, a retry loop or a permission flow. Prefer the code that the current task changes. If nothing stands out, ask me, with two or three candidates.
4. Write the model next to that code, with the code's names. Export every action, rule and end: its id is the constant's name. Give each one a sentence a reviewer can check. Give every number a small range (`m.int(0, 3)`, `#[range(0, 3)]`).
   - TypeScript: `<name>.model.ts`, `import { m } from "@earnestjs/model"`. Do not install the package. The checker never resolves it, and `tsc` reports the import as missing until it ships.
   - Rust: add `earnest = { git = "https://github.com/cpeaustriajc/earnest" }` to `[dependencies]` and put the model in a module.
5. Run `earnest verify <file>`. Fix each `unreadable` line in the model. For each `broken`, `stuck` or `fault`, walk the printed path through the code and decide whether the model or the app is wrong.
6. Change one action so a rule breaks, check that `verify` prints `fail` with a path, and undo it.

Change only the model, `.gitignore` and, for Rust, `Cargo.toml` and the module line. Report an app bug with its path, and ask before you fix it.

Report: the model file, its rules as sentences, the state count, the final `earnest verify` output, any app bug, and what the model leaves out.
