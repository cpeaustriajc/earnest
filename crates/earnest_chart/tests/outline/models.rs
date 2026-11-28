use earnest_ir::{
    Action, Code, End, Expr, Field, Label, Math, Model, Op, Order, Piece, Rule, View,
};
use earnest_value::{Domain, Symbols, Value};

const LABELS: [&str; 5] = ["step", "turn", "go", " ", ""];

pub(crate) struct Dice(u64);

impl Dice {
    pub(crate) fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1)
    }

    pub(crate) fn below(&mut self, count: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        let count = u64::try_from(count.max(1)).expect("a small count");
        usize::try_from(self.0 % count).expect("a small number")
    }

    fn chance(&mut self, one_in: usize) -> bool {
        self.below(one_in) == 0
    }

    fn label(&mut self) -> String {
        LABELS[self.below(LABELS.len())].to_owned()
    }
}

fn int(number: i64) -> Expr {
    Expr::Const(Value::Int(number))
}

fn apply(op: Op, left: Expr, right: Expr) -> Expr {
    Expr::Apply(op, vec![left, right])
}

fn number(value: usize) -> i64 {
    i64::try_from(value).expect("a small number")
}

fn shown(fields: &[usize], text: &str) -> Label {
    let mut pieces = vec![Piece::Text(text.to_owned())];
    for field in fields {
        pieces.push(Piece::Shown(Expr::Field(*field).into()));
        pieces.push(Piece::Text(",".to_owned()));
    }
    Label { pieces, slots: 0 }
}

fn action(dice: &mut Dice, id: usize, tops: &[usize]) -> Action {
    let field = dice.below(tops.len());
    let here = Expr::Field(field);
    let top = number(tops[field]);
    let (guard, update) = match dice.below(4) {
        0 => (
            apply(Op::Order(Order::Less), here.clone(), int(top)),
            apply(Op::Math(Math::Add), here, int(1)),
        ),
        1 => (
            apply(Op::Differ, here.clone(), int(0)),
            apply(Op::Math(Math::Add), here, int(-1)),
        ),
        2 => (
            Expr::Const(Value::Bool(true)),
            apply(Op::Math(Math::Add), here, int(1)),
        ),
        _ => (
            Expr::Const(Value::Bool(true)),
            int(number(dice.below(tops[field].saturating_add(1)))),
        ),
    };
    let mut updates = vec![None; tops.len()];
    updates[field] = Some(Code::new(update));
    Action {
        id: format!("a{id}"),
        label: dice.label(),
        line: 1,
        inputs: Vec::new(),
        guard: guard.into(),
        updates,
        slots: 0,
    }
}

fn subset(dice: &mut Dice, count: usize) -> Vec<usize> {
    (0..count).filter(|_| dice.chance(2)).collect()
}

fn view(dice: &mut Dice, fields: usize, actions: &[Action]) -> View {
    let mut view = View::default();
    if dice.chance(4) {
        view.title = Some("Random \"model\"".to_owned());
    }
    if !dice.chance(3) {
        view.state = Some(shown(&subset(dice, fields), "v "));
    }
    if dice.chance(2) {
        view.group = Some(shown(&subset(dice, fields), "g "));
    }
    for each in actions {
        if dice.chance(2) {
            view.steps.push((each.id.clone(), dice.label()));
        }
    }
    let colors = [("start", "#0f0"), ("end", "navy"), ("broken", "#123456")];
    for (name, color) in colors {
        if dice.chance(3) {
            view.colors.push((name.to_owned(), color.to_owned()));
        }
    }
    view
}

pub(crate) fn random_model(dice: &mut Dice) -> Model {
    let mut symbols = Symbols::default();
    let tops: Vec<usize> = (0..=dice.below(3))
        .map(|_| dice.below(3).saturating_add(1))
        .collect();
    let fields: Vec<Field> = tops
        .iter()
        .enumerate()
        .map(|(position, top)| {
            let name = format!("f{position}");
            let mut starts = vec![Value::Int(0)];
            if dice.chance(4) {
                starts.push(Value::Int(1));
            }
            Field {
                sym: symbols.intern(&name),
                name,
                domain: Domain::Int {
                    lo: 0,
                    hi: number(*top),
                },
                starts,
            }
        })
        .collect();
    let actions: Vec<Action> = (0..dice.below(5).saturating_add(2))
        .map(|id| action(dice, id, &tops))
        .collect();
    let rules = (0..dice.below(3))
        .map(|id| {
            let field = dice.below(tops.len());
            let value = number(dice.below(tops[field].saturating_add(1)));
            Rule {
                id: format!("r{id}"),
                says: "a rule".to_owned(),
                line: 1,
                text: String::new(),
                holds: apply(Op::Differ, Expr::Field(field), int(value)).into(),
                slots: 0,
            }
        })
        .collect();
    let ends = (0..dice.below(2))
        .map(|id| {
            let field = dice.below(tops.len());
            End {
                id: format!("e{id}"),
                says: "an end".to_owned(),
                when: apply(Op::Equal, Expr::Field(field), int(number(tops[field]))).into(),
                slots: 0,
            }
        })
        .collect();
    let view = view(dice, tops.len(), &actions);
    Model {
        name: "Random".to_owned(),
        file: "random.model.ts".to_owned(),
        line: 1,
        symbols,
        fields,
        actions,
        rules,
        ends,
        view,
        runs: earnest_ir::Runs::default(),
    }
}
