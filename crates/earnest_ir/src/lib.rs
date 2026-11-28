//! The typed expression language, its evaluator and the checked model.
//!
//! A [`Model`] is what `earnest_semantic` makes of a draft: fields with finite domains, and
//! actions, rules and ends whose conditions are [`Expr`]s. An expression reads state fields by
//! position and local values by slot, and [`eval`] gives its value in one state. The model
//! keeps each expression the search evaluates as a [`Code`], which gives the same answers faster.

mod code;
mod eval;
mod parts;
mod tlc_order;
mod told;

pub use code::{Code, Coding, PackedTest, PackedUpdate, Scalar, Slot};
pub use eval::{Fault, Frame, MOST_RANGE_VALUES, eval, truth};

use earnest_value::{Domain, IntType, Sym, Symbols, Value};

/// A comparison of two numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Order {
    /// The first is below the second.
    Less,
    /// The first is below or equal to the second.
    LessOrEqual,
    /// The first is above the second.
    Greater,
    /// The first is above or equal to the second.
    GreaterOrEqual,
}

/// Arithmetic on two numbers. A result that does not fit in 64 bits is [`Fault::Overflow`], and
/// so is one past the safe integers when it is wrapped in [`Op::Safe`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Math {
    /// The sum.
    Add,
    /// The first minus the second.
    Subtract,
    /// The product.
    Multiply,
    /// Quint's `imod`: what is left of the first after dividing by the second. A negative first
    /// number or a second number below 1 is [`Fault::Remainder`]. In a frame of Quint's numbers
    /// it is `first - second * (first / second)` for every second number but zero, which is
    /// [`Fault::ByZero`]. A TypeScript or Rust model has [`Math::TruncatedRemainder`] instead.
    Remainder,
    /// Quint's `idiv`: the whole number of times the second goes into the first. A negative
    /// first number or a second number below 1 is [`Fault::Division`]. In a frame of Quint's
    /// numbers it is the floor of the exact quotient for every second number but zero, which is
    /// [`Fault::ByZero`]. A TypeScript or Rust model has [`Math::TruncatedDivide`] instead.
    Divide,
    /// Rust's `/` on integers and TypeScript's `Math.trunc(a / b)`: the exact quotient with its
    /// fraction dropped, so `-7 / 2` is -3. A second number of zero is [`Fault::ByZero`], and a
    /// quotient that does not fit in 64 bits, the smallest `i64` over -1, is
    /// [`Fault::Overflow`]. A smaller type's bound is the [`Op::Fit`] around it.
    TruncatedDivide,
    /// Rust's `%` on integers and JavaScript's `%`: the remainder that goes with
    /// [`Math::TruncatedDivide`], with the sign of the first number, so `-7 % 2` is -1 and
    /// `7 % -2` is 1. A second number of zero is [`Fault::ByZero`]. Where Rust panics because
    /// the quotient overflows, the smallest number of the type given over -1, it is the fault the
    /// quotient has: [`Fault::Outside`] that type, or [`Fault::Overflow`] for the smallest `i64`.
    TruncatedRemainder(Option<IntType>),
}

/// A primitive on a set. The set is the first argument.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SetOp {
    /// Whether the second argument is an item.
    Has,
    /// The set with the second argument as an item.
    Add,
    /// The set without the second argument.
    Delete,
    /// The items of either set.
    Union,
    /// The items of both sets.
    Intersect,
    /// The items of the first set that the second lacks.
    Subtract,
    /// Whether every item of the first set is in the second.
    IsSubset,
    /// How many items it holds.
    Size,
}

/// A primitive on a map. The map is the first argument and the key the second.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum MapOp {
    /// Whether the key has a value.
    Has,
    /// The key's value, or `null` without one.
    Get,
    /// The map with the key set to the third argument.
    Set,
    /// The map without the key.
    Delete,
    /// How many keys it holds.
    Size,
    /// The set of its keys.
    Keys,
    /// The key's value. A key without one is [`Fault::Missing`].
    MustGet,
    /// The map with the key's value replaced by the third argument. A key without a value is
    /// [`Fault::Missing`].
    Replace,
    /// The map with the key set to the third argument. A key that already has another value is
    /// [`Fault::KeyTwice`].
    Once,
}

/// A primitive on a sequence or a queue. The sequence is the first argument.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SeqOp {
    /// How many items it holds.
    Size,
    /// The item at the position in the second argument, counted from 0, or `null` when there
    /// is none.
    Get,
    /// The first item, or `null` when empty.
    First,
    /// The last item, or `null` when empty.
    Last,
    /// The sequence with the second argument added at the end.
    Push,
    /// The sequence without its first item. An empty sequence stays empty.
    Shift,
    /// The sequence without its last item. An empty sequence stays empty.
    Pop,
    /// Whether the second argument is an item.
    Includes,
    /// The item at the position in the second argument, counted from 0. A position with no
    /// item is [`Fault::Missing`].
    Nth,
    /// The first item. An empty sequence is [`Fault::Missing`].
    Head,
    /// The sequence without its first item. An empty sequence is [`Fault::Missing`].
    Tail,
    /// The items of the first sequence, then those of the second.
    Concat,
    /// The items from the position in the second argument up to, not including, the position
    /// in the third, counted from 0. Positions that are not `0 <= second <= third <= size` are
    /// [`Fault::Missing`].
    Slice,
    /// The sequence with the item at the position in the second argument, counted from 0,
    /// replaced by the third argument. A position with no item is [`Fault::Missing`].
    ReplaceAt,
}

/// An operation on evaluated arguments, used in [`Expr::Apply`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Op {
    /// The opposite of a boolean.
    Not,
    /// A number with its sign flipped.
    Negate,
    /// A present optional value. It gives its argument back unchanged, because an optional
    /// value has no wrapper.
    SomeOf,
    /// The value of an optional that must be present. `null` is [`Fault::NullRead`].
    Unwrap,
    /// A number that must be one a JavaScript number holds exactly. It gives its argument back
    /// unchanged, and a number beyond `-MOST_SAFE_INTEGER` or `MOST_SAFE_INTEGER` is
    /// [`Fault::Overflow`].
    Safe,
    /// A number that must be a value of this integer type. It gives its argument back
    /// unchanged, and a number outside the type is [`Fault::Outside`].
    Fit(IntType),
    /// Whether two values are the same.
    Equal,
    /// Whether two values are not the same.
    Differ,
    /// A comparison of two numbers.
    Order(Order),
    /// Arithmetic on two numbers.
    Math(Math),
    /// A primitive on a set.
    Set(SetOp),
    /// A primitive on a map.
    Map(MapOp),
    /// A primitive on a sequence or a queue.
    Seq(SeqOp),
    /// The first number to the power of the second. A negative power and zero to the power
    /// zero are [`Fault::Undefined`].
    Power,
    /// The set of whole numbers from the first to the second, both included. Empty when the
    /// first is above the second. More than [`MOST_RANGE_VALUES`] numbers is
    /// [`Fault::TooWide`].
    Interval,
    /// The sequence of whole numbers from the first up to, not including, the second. A first
    /// number above the second is [`Fault::Undefined`]. More than [`MOST_RANGE_VALUES`]
    /// numbers is [`Fault::TooWide`].
    Ascending,
    /// The items of a set as a sequence, in the order TLC's `CHOOSE` takes them. A set, a
    /// sequence, a map or a record with fewer items comes first; two of one size compare item
    /// by item, a set in this same order, a map its keys and then their values, a record its
    /// field names and then its values. Numbers, booleans and texts keep their own order, so
    /// texts are alphabetical, where TLC takes them in the order it first met them. An optional
    /// second argument is a sequence of the field names that are a tuple's items, in position
    /// order: a record whose fields are all among them compares its items by position, as TLC
    /// compares a tuple.
    InTlcOrder,
    /// True when its argument is true. False is [`Fault::Asserted`]: an `assert` in a Quint
    /// action, which Quint's backends read differently when it is false.
    Assert,
}

/// What an [`Expr::Sweep`] does with the test's answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Sweep {
    /// True when every item passes. True for no items.
    Every,
    /// True when at least one item passes. False for no items.
    Some,
    /// The items that pass, in their order.
    Filter,
}

/// The kind of collection an [`Sweep::Filter`] gives back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Shelf {
    /// A set.
    Set,
    /// A sequence or a queue.
    Seq,
}

/// An expression over one state.
///
/// Evaluate it with [`eval`] in a [`Frame`] that holds the state and enough local slots for
/// every `slot` the expression names. To evaluate it in many states, prepare it once as a
/// [`Code`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Expr {
    /// A fixed value.
    Const(Value),
    /// The state field at this position in [`Model::fields`].
    Field(usize),
    /// The local value in this slot: an input, a binding or a test's item.
    Local(usize),
    /// An operation on the values of the arguments, all evaluated first, in order.
    Apply(Op, Vec<Self>),
    /// True when both are true. The second is not evaluated when the first is false.
    And(Box<Self>, Box<Self>),
    /// True when either is true. The second is not evaluated when the first is true.
    Or(Box<Self>, Box<Self>),
    /// The second expression when the first is true, the third otherwise. Only the chosen one
    /// is evaluated.
    Choose(Box<Self>, Box<Self>, Box<Self>),
    /// `body`, evaluated with `value` in `slot`.
    Bind {
        /// The slot `body` reads the value from.
        slot: usize,
        /// The value to bind.
        value: Box<Self>,
        /// The expression that gives the result.
        body: Box<Self>,
    },
    /// A record of these fields.
    Record(Vec<(Sym, Self)>),
    /// The named field of a record.
    Get(Box<Self>, Sym),
    /// A record with some fields replaced or added.
    With(Box<Self>, Vec<(Sym, Self)>),
    /// A set of these items.
    SetOf(Vec<Self>),
    /// A sequence of these items, in order.
    SeqOf(Vec<Self>),
    /// A map of these key and value pairs. A later pair replaces an earlier one with the same
    /// key.
    MapOf(Vec<(Self, Self)>),
    /// A test run on each item of a set or a sequence.
    Sweep {
        /// What to do with the answers.
        sweep: Sweep,
        /// The kind of collection a filter gives back.
        shelf: Shelf,
        /// The slot `body` reads the current item from.
        slot: usize,
        /// The collection whose items are tried.
        source: Box<Self>,
        /// The test, which gives true or false.
        body: Box<Self>,
    },
    /// A running value carried through the items of a set, in its order, or of a sequence:
    /// `start`, then `body` with the value so far in `total` and the next item in `item`.
    Fold {
        /// The kind of collection `source` must give. Another kind is [`Fault::WrongKind`].
        shelf: Shelf,
        /// The collection whose items are taken.
        source: Box<Self>,
        /// The value before the first item.
        start: Box<Self>,
        /// The slot `body` reads the value so far from.
        total: usize,
        /// The slot `body` reads the current item from.
        item: usize,
        /// The next value.
        body: Box<Self>,
    },
}

/// One field of a model's state.
#[derive(Clone, Debug)]
pub struct Field {
    /// The name printed in paths and charts.
    pub name: String,
    /// The same name, interned in the model's [`Symbols`].
    pub sym: Sym,
    /// Every value the field declares. A state that holds another value is out of range.
    pub domain: Domain,
    /// The values the field can start at. The start states are every combination across the
    /// fields.
    pub starts: Vec<Value>,
}

/// One input of an action.
#[derive(Clone, Debug)]
pub struct Input {
    /// The name printed in a path step.
    pub name: String,
    /// The local slot the action's expressions read the input from.
    pub slot: usize,
    /// What the input declares.
    pub domain: Domain,
    /// Every value of `domain`, in the order the search tries them.
    pub values: Vec<Value>,
}

/// One action: for each combination of input values, a guard and the fields it changes.
#[derive(Clone, Debug)]
pub struct Action {
    /// The name of the exported constant or the method.
    pub id: String,
    /// The action's sentence, the default arrow label.
    pub label: String,
    /// The line it is declared on, counted from 1.
    pub line: u32,
    /// The inputs, each in its own slot.
    pub inputs: Vec<Input>,
    /// Whether the action can happen in a state with given inputs. It must give a boolean.
    pub guard: Code,
    /// The new value of each state field, by position in [`Model::fields`]. `None` keeps the
    /// field as it is, and so does a list that ends before the field's position.
    pub updates: Vec<Option<Code>>,
    /// How many local slots `guard` and `updates` need.
    pub slots: usize,
}

/// One rule: a condition that must be true in every reachable state.
#[derive(Clone, Debug)]
pub struct Rule {
    /// The name of the exported constant or the method.
    pub id: String,
    /// The rule's sentence.
    pub says: String,
    /// The line it is declared on, counted from 1.
    pub line: u32,
    /// The condition's source text, kept to show a rule change in the report.
    pub text: String,
    /// The condition. It must give a boolean.
    pub holds: Code,
    /// How many local slots `holds` needs.
    pub slots: usize,
}

/// One end: a condition that marks states where stopping is correct.
#[derive(Clone, Debug)]
pub struct End {
    /// The name of the exported constant or the method.
    pub id: String,
    /// The end's sentence.
    pub says: String,
    /// The condition. It must give a boolean.
    pub when: Code,
    /// How many local slots `when` needs.
    pub slots: usize,
}

/// One rule about whole runs: a state every run reaches.
///
/// A run never stops while an action can still change the state, and it does not put off
/// forever an action listed in [`Runs::fair`] that stays possible, unless `fair` gives the
/// rule's own [`Fairness`]. Every such run from a state
/// the rule starts from must come to a state where `reaches` is true, which may be that state.
#[derive(Clone, Debug)]
pub struct Eventually {
    /// The name of the exported constant or the method.
    pub id: String,
    /// The rule's sentence.
    pub says: String,
    /// The line it is declared on, counted from 1.
    pub line: u32,
    /// The states the rule starts from: every reachable state where this is true. `None` is
    /// every start state. It must give a boolean.
    pub after: Option<Code>,
    /// The condition a run must meet. It must give a boolean.
    pub reaches: Code,
    /// How many local slots `after` and `reaches` need.
    pub slots: usize,
    /// The runs this rule is judged over, when the rule says so itself. `None` is the model's:
    /// a run never stops while an action can change the state, and [`Runs::fair`] is fair.
    pub fair: Option<Fairness>,
}

/// Which runs a rule about runs is judged over, when the rule says so itself.
///
/// It is the TLA+ weak fairness of each part: a part that can change the state from some point
/// on forever does so infinitely often. A run that is fair to all of it may stay in one state
/// forever when no fair part can change the state there.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Fairness {
    /// Whether the whole step is fair: a run never stops while an action can change the state.
    pub step: bool,
    /// The fair parts of the step, each the positions in [`Model::actions`] of the actions it
    /// is made of. A part is taken when any one of its actions is.
    pub actions: Vec<Vec<usize>>,
}

/// One step of a [`Witness`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Played {
    /// The position of the action in [`Model::actions`].
    pub action: usize,
    /// The value of each input, in the order of [`Action::inputs`].
    pub input: Vec<Value>,
}

/// One witness: a path the model must be able to take from every start state, and a condition
/// that must be true at its end.
#[derive(Clone, Debug)]
pub struct Witness {
    /// The name of the exported constant or the method.
    pub id: String,
    /// The witness's sentence.
    pub says: String,
    /// The line it is declared on, counted from 1.
    pub line: u32,
    /// The position in [`Runs::setups`] of the setup whose start states the path is played
    /// from. `None` is the model's own start states.
    pub setup: Option<usize>,
    /// The actions taken, first to last.
    pub steps: Vec<Played>,
    /// The condition on the state after the last step. It must give a boolean.
    pub expect: Code,
    /// How many local slots `expect` needs.
    pub slots: usize,
}

/// One step of a [`Plan`]: the actions it may take, and what is checked around it.
#[derive(Clone, Debug)]
pub struct Stage {
    /// The positions in [`Script::actions`] of the actions the step may take. Each action
    /// with each combination of its inputs' values is one alternative: the step can happen
    /// when at least one alternative can, and every alternative that can is played.
    pub choices: Vec<usize>,
    /// A condition on the state before the step, checked before any alternative. False is a
    /// failed assertion, not a step that can't happen. It must give a boolean.
    pub asserts: Option<Code>,
    /// A condition on the state after the step. It must give a boolean.
    pub expect: Option<Code>,
    /// Whether the step must not be able to happen. Only the last step of a plan is refused:
    /// the plan holds where no alternative can happen and breaks where one can.
    pub refused: bool,
}

/// One way to play a [`Script`], from every one of its start states.
#[derive(Clone, Debug)]
pub struct Plan {
    /// The values the run chose before its first step, each with its name, printed with a
    /// broken plan. Empty when the run chooses nothing.
    pub chosen: Vec<(String, Value)>,
    /// A condition on the start state, before the first step. It must give a boolean.
    pub start: Option<Code>,
    /// The states the plan starts from, each one value per field of [`Script::fields`] (or of
    /// [`Model::fields`] when the script has none of its own). `None` starts from the start
    /// states of whichever setup the model starts in.
    pub starts: Option<Vec<Vec<Value>>>,
    /// The steps, first to last.
    pub steps: Vec<Stage>,
    /// Whether a branch that gets through every step breaks the plan: the run is false there.
    pub must_not_finish: bool,
    /// What a branch on which the run is false does to the script.
    pub falls: Falls,
    /// Values the run's first action can pick, inside it, for which it can't happen, though it
    /// can for others: `quint test` tries no other value of a `nondet` over 100 values or
    /// more, so the first action can be false. Each value comes with its name. Empty when the
    /// first action can't be false so.
    pub start_falls: Vec<(String, Value)>,
}

/// The number of values below which `quint test` tries the next value of a `nondet` where
/// the action it picks for is false. Over a set of this many values or more, it keeps the
/// value it picked first.
pub const RETRIED_BELOW: usize = 100;

/// What a branch of a [`Plan`] on which the run is false does, as against one that ends in
/// an error. An error always breaks the script.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Falls {
    /// It breaks the script.
    Breaks,
    /// The plan is one combination of the values the `nondet`s at the top of the run pick,
    /// and Quint tries other values where the run is false: the script holds when no plan
    /// ends in an error and the combinations give the run `true` on every sample.
    Retried {
        /// Whether the run is false, not in error, where its last step can't happen, or,
        /// in a run of one action, where that action can't: the step is the last part of the
        /// run, not the first part of a `then`, a `reps` or an `expect`.
        last_step: bool,
        /// For each value of [`Plan::chosen`], whether Quint tries the other values of its
        /// set when the run is false for it: it does when the set has fewer than 100 values.
        retried: Vec<bool>,
        /// For each value of [`Plan::chosen`], the position in `calls` of the called run whose
        /// `nondet` picked it, which Quint tries again while that run is false, or `None` for a
        /// `nondet` tried again while the whole run is false.
        within: Vec<Option<usize>>,
        /// The runs the run calls with a `nondet` of their own, where something comes after
        /// them, so that a called run that is false is an error.
        calls: Vec<Call>,
        /// For each value of [`Plan::chosen`], whether it numbers a part of an `or`, from 1:
        /// Quint plays a part only where every part before it is false.
        ordered: Vec<bool>,
    },
}

/// A run called inside another with a `nondet` of its own, as Quint plays it: its `nondet` is
/// tried again while the called run is false, and the called run that stays false is an
/// error, since something comes after it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Call {
    /// What the called run is, for a message, such as ``the run `start` ``.
    pub says: String,
    /// The position in [`Plan::steps`] of the called run's last step, where the called run is
    /// false when the step can't happen; [`Plan::steps`]' length when the called run is false
    /// once the steps before it are taken; `None` when the called run can't be false.
    pub last: Option<usize>,
    /// Whether the called run being false makes the whole run false, rather than an error.
    pub ends_the_run: bool,
    /// Why the called run is false where `last` is [`Plan::steps`]' length, for a message.
    pub reason: String,
}

/// A generalised witness: steps that each choose among alternatives, with conditions after
/// any step, played along every resolution of the choices. A [`Witness`] is the case of one
/// alternative per step, with its inputs given, and one condition at the end.
#[derive(Clone, Debug)]
pub struct Script {
    /// The run's name.
    pub id: String,
    /// The run's sentence.
    pub says: String,
    /// The line it is declared on, counted from 1.
    pub line: u32,
    /// The actions the steps take. They are the script's own, not the model's.
    pub actions: Vec<Action>,
    /// The fields the script is played over, when they are not [`Model::fields`]: its
    /// actions, conditions and start states read and change these, by position, and its paths
    /// print these. Empty plays it over the model's fields.
    pub fields: Vec<Field>,
    /// The plans. The script holds when every plan holds, or, for plans whose run is
    /// [`Falls::Retried`], as Quint judges the combinations.
    pub plans: Vec<Plan>,
    /// How many local slots the conditions of the plans need.
    pub slots: usize,
}

/// One setup: other values for some fields to start at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Setup {
    /// The name of the exported constant or the method.
    pub id: String,
    /// The setup's sentence.
    pub says: String,
    /// The line it is declared on, counted from 1.
    pub line: u32,
    /// The one value a field starts at, by the field's position in [`Model::fields`]. A field
    /// that is not listed starts where the model declares.
    pub starts: Vec<(usize, Value)>,
}

/// An input of an action whose values are worked out in each state: the items of a set.
///
/// The input's own [`Input::values`] are not used. The search tries every item of `from` in
/// each state, and none when the set is empty, so the action cannot happen there.
#[derive(Clone, Debug)]
pub struct Pick {
    /// The position in [`Runs::scripts`] of the script whose [`Script::actions`] hold the
    /// action. `None` for an action of [`Model::actions`].
    pub script: Option<usize>,
    /// The position of the action in [`Model::actions`], or in the script's actions.
    pub action: usize,
    /// The position of the input in [`Action::inputs`].
    pub input: usize,
    /// The set whose items the input takes. It reads the state and the action's earlier
    /// inputs, in the action's slots, and must give a set.
    pub from: Code,
}

/// What a model says about whole runs instead of single states.
#[derive(Clone, Debug, Default)]
pub struct Runs {
    /// The positions in [`Model::actions`] of the actions that are not put off forever.
    pub fair: Vec<usize>,
    /// The rules about a state every run reaches.
    pub eventually: Vec<Eventually>,
    /// The witnesses.
    pub witnesses: Vec<Witness>,
    /// The generalised witnesses, played from the start states of whichever setup the model
    /// starts in.
    pub scripts: Vec<Script>,
    /// The setups the model is also checked in.
    pub setups: Vec<Setup>,
    /// The inputs whose values are worked out in each state.
    pub picks: Vec<Pick>,
    /// The position in `setups` of the setup this model starts in: [`Model::in_setup`] gave
    /// it. `None` for the model as its file declares it.
    pub chosen: Option<usize>,
    /// The start states, each one value per field in the order of [`Model::fields`], when the
    /// model lists them. `None` starts from every combination of the fields' `starts`. A
    /// setup does not change a listed start state.
    pub starts: Option<Vec<Vec<Value>>>,
    /// The fair steps narrower than an action, which a rule's own [`Fairness`] may be fair to.
    /// A part of [`Fairness::actions`] names fair step `k` by the position
    /// `actions.len() + k`, where `actions` is [`Model::actions`].
    pub fair_steps: Vec<FairStep>,
}

/// A step a part of a [`Fairness`] is fair to that is narrower than an action: TLA+'s
/// `<<A(c)>>_e`, a step of one action, with some of its inputs given, that changes `e`.
///
/// An arrow counts as such a step when the action gives it with an input combination that
/// holds the given values, and some field of `over` differs between its two states.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FairStep {
    /// The position of the action in [`Model::actions`].
    pub action: usize,
    /// The inputs the step takes, each the position in [`Action::inputs`] and its value.
    pub inputs: Vec<(usize, Value)>,
    /// The positions in [`Model::fields`] of which the step must change one, in the order the
    /// fairness named them. Empty is any field.
    pub over: Vec<usize>,
    /// The action read from its own definition, outside the step, as TLA+ reads `WF_e(A)`.
    /// When present, [`FairStep::action`] and [`FairStep::inputs`] are not used: the step is
    /// possible in a state where some way through `A` alone changes a field of `over`, and an
    /// arrow counts when some way through `A` alone gives its target and changes `over`.
    pub alone: Option<Alone>,
}

/// An action read from its own definition, apart from the step that calls it, for fairness.
///
/// Each of [`Alone::actions`] is one way through the definition. An update that is `None`
/// leaves the field free: the rest of the step gives it, so any value is allowed, where in
/// [`Model::actions`] `None` keeps the field's value.
///
/// Two are equal when their [`Alone::named`] is: the same definition given the same arguments.
#[derive(Clone, Debug)]
pub struct Alone {
    /// The definition and the arguments it is given, as written: `pay(1)`.
    pub named: String,
    /// Each way through the definition, with its inputs, guard and the fields it gives.
    pub actions: Vec<Action>,
    /// The inputs of [`Alone::actions`] whose set reads the state, as [`Runs::picks`] gives
    /// them for [`Model::actions`]: [`Pick::script`] is `None` and [`Pick::action`] is the
    /// position in [`Alone::actions`].
    pub picks: Vec<Pick>,
}

impl PartialEq for Alone {
    fn eq(&self, other: &Self) -> bool {
        self.named == other.named
    }
}

impl Eq for Alone {}

/// One part of a chart label.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum Piece {
    /// Text copied as it is.
    Text(String),
    /// An expression whose value is printed without quotes.
    Shown(Code),
}

/// A chart label computed from a state.
#[derive(Clone, Debug)]
pub struct Label {
    /// The parts, joined without a separator.
    pub pieces: Vec<Piece>,
    /// How many local slots the parts need.
    pub slots: usize,
}

/// How a model wants its chart drawn. Every part is optional.
#[derive(Clone, Debug, Default)]
pub struct View {
    /// The chart's title.
    pub title: Option<String>,
    /// The label of a state. States with the same label and group are drawn as one.
    pub state: Option<Label>,
    /// The label of the box a state is drawn in.
    pub group: Option<Label>,
    /// Arrow labels as action id and label pairs. An action without one uses its sentence.
    pub steps: Vec<(String, String)>,
    /// Colors as pairs of `broken`, `start` or `end` and a CSS color.
    pub colors: Vec<(String, String)>,
}

/// A checked model, ready to search.
#[derive(Clone, Debug)]
pub struct Model {
    /// The model's name.
    pub name: String,
    /// The path of the file it was read from.
    pub file: String,
    /// The line it is declared on, counted from 1.
    pub line: u32,
    /// The names behind every [`Sym`] in the model. Print the model's values with it.
    pub symbols: Symbols,
    /// The state fields. A state is one value per field, in this order.
    pub fields: Vec<Field>,
    /// The actions, in the order the search tries them.
    pub actions: Vec<Action>,
    /// The rules.
    pub rules: Vec<Rule>,
    /// The ends.
    pub ends: Vec<End>,
    /// The view, empty when the model declares none.
    pub view: View,
    /// The eventually rules, witnesses and setups, empty when the model declares none.
    pub runs: Runs,
}

impl Model {
    /// The id of everything this model is checked for and reported by its id: each rule, then
    /// each rule about runs, then each witness that is played from this model's start states,
    /// then each script.
    pub fn rule_ids(&self) -> impl Iterator<Item = &str> {
        let always = self.rules.iter().map(|rule| rule.id.as_str());
        let eventually = self.runs.eventually.iter().map(|rule| rule.id.as_str());
        let witnesses = self
            .runs
            .witnesses
            .iter()
            .filter(|witness| witness.setup == self.runs.chosen)
            .map(|witness| witness.id.as_str());
        let scripts = self.runs.scripts.iter().map(|script| script.id.as_str());
        always.chain(eventually).chain(witnesses).chain(scripts)
    }

    /// The setup this model starts in, when [`Self::in_setup`] gave it.
    #[must_use]
    pub fn setup(&self) -> Option<&Setup> {
        self.runs.setups.get(self.runs.chosen?)
    }

    /// The model's name for a line of output: the name alone for the model as declared, and
    /// the name with the setup's id in round brackets after it for a model in a setup.
    #[must_use]
    pub fn run_name(&self) -> String {
        match self.setup() {
            Some(setup) => format!("{} ({})", self.name, setup.id),
            None => self.name.clone(),
        }
    }

    /// The same model started in the setup at `position` of [`Runs::setups`]: each field the
    /// setup lists starts at the setup's value, and [`Runs::chosen`] names the setup. `None`
    /// for a position past the last setup.
    #[must_use]
    pub fn in_setup(&self, position: usize) -> Option<Self> {
        let setup = self.runs.setups.get(position)?;
        let mut started = self.clone();
        for (field, value) in &setup.starts {
            if let Some(found) = started.fields.get_mut(*field) {
                found.starts = vec![value.clone()];
            }
        }
        started.runs.chosen = Some(position);
        Some(started)
    }
}
