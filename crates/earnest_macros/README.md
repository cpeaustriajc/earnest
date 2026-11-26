# earnest_macros

The macros of the Rust model library. Depend on `earnest` and import them from `earnest::model`;
the code they generate names `::earnest::model`.

| Macro | Goes on | What it generates |
|---|---|---|
| `#[derive(Domain)]` | an enum of plain variants, or a struct with named fields | `earnest::model::Domain`: every value of the type |
| `#[derive(Model)]` | the struct that holds a model's state | `earnest::model::Model`: every start state, from `#[initial(value)]`, `#[range(lo, hi)]` and each field's type |
| `#[model]` | the model's `impl` block | `earnest::model::Behavior`: every step, rule and end of a state, and every rule about runs, witness and setup. The methods stay callable as written, without `#[action]`, `#[rule]`, `#[end]`, `#[eventually]`, `#[witness]`, `#[setup]` and a parameter's `#[range]` or `#[one_of]`. An action that returns `Self` in place of `Option<Self>` always happens. `fair` on an action, `after = method` on a rule about runs and `setup = method` on a witness are accepted, each once and on its own attribute only; `after` and `setup` are called in the generated code, so `rustc` refuses a name that is no method (E0599) or has another signature (E0308, E0061). A witness returns `Option<bool>` when a step is followed by `?` and `bool` when none is; `Witnessed::played` is `Some` of a `bool` result |
| `#[view(...)]` | a second `impl` block of the model's type | `earnest::model::View`: the title, the arrow labels, the colors and the `#[state]` and `#[group]` labels of the statechart. The block holds only those two methods, and the macro refuses an unknown setting, a color part other than `broken`, `start` and `end`, a step for a method the type does not have and a name given twice |

An integer always needs `#[range(lo, hi)]` or `#[one_of(a, b)]`, on a field or on an action's
parameter, and a `&'static str` always needs `#[one_of("a", "b")]`; the macros refuse either
without it, and refuse both on one field. `#[derive(Model)]` also uses the model's type in an
anonymous constant, so a model that only `earnest verify` reads is not dead code to `rustc`.
Generated code is written to pass the workspace's lints in the crate that uses it, and a model
file needs no `pub`, no documentation and no `allow`.
