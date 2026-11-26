use crate::json::Expr as Quint;
use crate::reader::{Read, Reader, constant};
use earnest_ir::{Expr, MapOp, Math, Op, SeqOp, SetOp, Shelf, Sweep};
use earnest_value::{MOST_VALUES_LISTED, Sym, Value};
use std::collections::BTreeSet;

pub(crate) const TAG: &str = "tag";
pub(crate) const VALUE: &str = "value";
pub(crate) const ANY_OTHER_TAG: &str = "_";
const MOST_POWERSET_ITEMS: u32 = MOST_VALUES_LISTED.ilog2();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Data {
    Apply(Op, usize),
    Record,
    Field,
    With,
    Tuple,
    Item,
    Tuples,
    Variant,
    Match,
    Set,
    List,
    Map,
    In,
    Sweep(Sweep, Shelf),
    SetMap,
    Fold(Shelf),
    Flatten,
    Powerset,
    IsFinite,
    SetBy,
    MapBy,
    SetToMap,
    Indices,
}

pub(crate) fn data_builtin(opcode: &str) -> Option<Data> {
    let set = |op| Data::Apply(Op::Set(op), 2);
    let found = match opcode {
        "Rec" => Data::Record,
        "field" => Data::Field,
        "with" => Data::With,
        "Tup" => Data::Tuple,
        "item" => Data::Item,
        "tuples" => Data::Tuples,
        "variant" => Data::Variant,
        "matchVariant" => Data::Match,
        "Set" => Data::Set,
        "List" => Data::List,
        "Map" => Data::Map,
        "contains" => set(SetOp::Has),
        "in" => Data::In,
        "union" => set(SetOp::Union),
        "intersect" => set(SetOp::Intersect),
        "exclude" => set(SetOp::Subtract),
        "subseteq" => set(SetOp::IsSubset),
        "size" => Data::Apply(Op::Set(SetOp::Size), 1),
        "filter" => Data::Sweep(Sweep::Filter, Shelf::Set),
        "exists" => Data::Sweep(Sweep::Some, Shelf::Set),
        "forall" => Data::Sweep(Sweep::Every, Shelf::Set),
        "select" => Data::Sweep(Sweep::Filter, Shelf::Seq),
        "map" => Data::SetMap,
        "fold" => Data::Fold(Shelf::Set),
        "foldl" => Data::Fold(Shelf::Seq),
        "flatten" => Data::Flatten,
        "powerset" => Data::Powerset,
        "isFinite" => Data::IsFinite,
        "to" => Data::Apply(Op::Interval, 2),
        "get" => Data::Apply(Op::Map(MapOp::MustGet), 2),
        "keys" => Data::Apply(Op::Map(MapOp::Keys), 1),
        "put" => Data::Apply(Op::Map(MapOp::Set), 3),
        "set" => Data::Apply(Op::Map(MapOp::Replace), 3),
        "setBy" => Data::SetBy,
        "mapBy" => Data::MapBy,
        "setToMap" => Data::SetToMap,
        "append" => Data::Apply(Op::Seq(SeqOp::Push), 2),
        "concat" => Data::Apply(Op::Seq(SeqOp::Concat), 2),
        "head" => Data::Apply(Op::Seq(SeqOp::Head), 1),
        "tail" => Data::Apply(Op::Seq(SeqOp::Tail), 1),
        "length" => Data::Apply(Op::Seq(SeqOp::Size), 1),
        "nth" => Data::Apply(Op::Seq(SeqOp::Nth), 2),
        "indices" => Data::Indices,
        "replaceAt" => Data::Apply(Op::Seq(SeqOp::ReplaceAt), 3),
        "slice" => Data::Apply(Op::Seq(SeqOp::Slice), 3),
        "range" => Data::Apply(Op::Ascending, 2),
        "ipow" => Data::Apply(Op::Power, 2),
        _ => return None,
    };
    Some(found)
}

fn boxed(expr: Expr) -> Box<Expr> {
    Box::new(expr)
}

fn apply(op: Op, arguments: Vec<Expr>) -> Expr {
    Expr::Apply(op, arguments)
}

fn empty_set() -> Expr {
    Expr::Const(Value::set(BTreeSet::new()))
}

fn empty_map() -> Expr {
    Expr::Const(Value::map(std::collections::BTreeMap::new()))
}

fn powerset(items: &BTreeSet<Value>) -> BTreeSet<Value> {
    let mut all = vec![BTreeSet::new()];
    for item in items {
        let with_item: Vec<BTreeSet<Value>> = all
            .iter()
            .map(|subset| {
                let mut larger = subset.clone();
                larger.insert(item.clone());
                larger
            })
            .collect();
        all.extend(with_item);
    }
    all.into_iter().map(Value::set).collect()
}

struct Carried {
    total: usize,
    item: usize,
}

impl Carried {
    fn total(&self) -> Expr {
        Expr::Local(self.total)
    }

    fn item(&self) -> Expr {
        Expr::Local(self.item)
    }
}

impl<'a> Reader<'a> {
    pub(crate) fn text(&mut self, text: &str) -> Sym {
        self.symbols.intern(text)
    }

    fn fresh(&mut self) -> usize {
        let slot = self.slots;
        self.slots = self.slots.saturating_add(1);
        slot
    }

    fn label(&mut self, arg: &Quint, opcode: &str) -> Read<Sym> {
        match arg {
            Quint::Str { value } => Ok(self.text(value)),
            _ => self.refuse(
                &format!("`{opcode}`"),
                "a field name or a tag that is not written out",
            ),
        }
    }

    fn position(&mut self, number: usize) -> Sym {
        self.text(&format!("_{number}"))
    }

    fn lambda(&self, expr: &'a Quint, wanted: usize, opcode: &str) -> Read<(Vec<u64>, &'a Quint)> {
        let found = match expr {
            Quint::Lambda { params, expr } => Some((params, &**expr)),
            Quint::Name { id, name } => {
                let def = self.operator(self.entry(*id, name)?, name)?;
                match &def.expr {
                    Quint::Lambda { params, expr } => Some((params, &**expr)),
                    _ => None,
                }
            }
            _ => None,
        };
        match found {
            Some((params, body)) if params.len() == wanted => {
                Ok((params.iter().map(|param| param.id).collect(), body))
            }
            _ => self.refuse(
                &format!("`{opcode}`"),
                &format!("its operator does not take {wanted} arguments"),
            ),
        }
    }

    fn under(&mut self, lambda: (Vec<u64>, &'a Quint), values: Vec<Expr>) -> Read<Expr> {
        let (ids, body) = lambda;
        self.with_values(ids.into_iter().zip(values).collect(), |reader| {
            reader.pure(body)
        })
    }

    fn carried(&mut self) -> Carried {
        Carried {
            total: self.fresh(),
            item: self.fresh(),
        }
    }

    fn folded(carried: &Carried, source: Expr, start: Expr, body: Expr) -> Expr {
        Self::folded_as(Shelf::Set, carried, source, start, body)
    }

    fn folded_as(shelf: Shelf, carried: &Carried, source: Expr, start: Expr, body: Expr) -> Expr {
        Expr::Fold {
            shelf,
            source: boxed(source),
            start: boxed(start),
            total: carried.total,
            item: carried.item,
            body: boxed(body),
        }
    }

    fn given(&self, opcode: &str, args: &[Quint], wanted: usize) -> Read<()> {
        if args.len() == wanted {
            return Ok(());
        }
        self.refuse(
            &format!("`{opcode}`"),
            &format!("it is given {} arguments", args.len()),
        )
    }

    pub(crate) fn data(&mut self, data: Data, opcode: &str, args: &'a [Quint]) -> Read<Expr> {
        self.built(data, opcode, args)
    }

    fn built(&mut self, data: Data, opcode: &str, args: &'a [Quint]) -> Read<Expr> {
        match data {
            Data::Apply(op, wanted) => {
                self.given(opcode, args, wanted)?;
                Ok(apply(op, self.each(args)?))
            }
            Data::Record => self.record(args, opcode),
            Data::Field => {
                self.given(opcode, args, 2)?;
                let record = self.pure(&args[0])?;
                let name = self.label(&args[1], opcode)?;
                Ok(Expr::Get(boxed(record), name))
            }
            Data::With => {
                self.given(opcode, args, 3)?;
                let record = self.pure(&args[0])?;
                let name = self.label(&args[1], opcode)?;
                let value = self.pure(&args[2])?;
                Ok(Expr::With(boxed(record), vec![(name, value)]))
            }
            Data::Tuple => {
                let items = self.each(args)?;
                Ok(self.tuple(items))
            }
            Data::Item => {
                self.given(opcode, args, 2)?;
                let tuple = self.pure(&args[0])?;
                let Quint::Int { value } = &args[1] else {
                    return self.refuse("`item`", "a tuple position that is not written out");
                };
                let Some(position) = value.as_u64().and_then(|at| usize::try_from(at).ok()) else {
                    return self.refuse("`item`", "a tuple position below 1");
                };
                let name = self.position(position);
                Ok(Expr::Get(boxed(tuple), name))
            }
            Data::Tuples => {
                let sets = self.each(args)?;
                Ok(self.tuples(sets))
            }
            Data::Variant => {
                self.given(opcode, args, 2)?;
                let tag = self.label(&args[0], opcode)?;
                let value = self.pure(&args[1])?;
                let (tag_name, value_name) = (self.text(TAG), self.text(VALUE));
                Ok(Expr::Record(vec![
                    (tag_name, Expr::Const(Value::Str(tag))),
                    (value_name, value),
                ]))
            }
            Data::Match => self.matched(args),
            Data::Set => Ok(Expr::SetOf(self.each(args)?)),
            Data::List => Ok(Expr::SeqOf(self.each(args)?)),
            Data::Map => self.map_of(args),
            Data::In => {
                self.given(opcode, args, 2)?;
                let mut both = self.each(args)?;
                both.reverse();
                Ok(apply(Op::Set(SetOp::Has), both))
            }
            computed => self.computed(computed, opcode, args),
        }
    }

    fn computed(&mut self, data: Data, opcode: &str, args: &'a [Quint]) -> Read<Expr> {
        match data {
            Data::Sweep(sweep, kept_as) => {
                self.given(opcode, args, 2)?;
                let source = self.pure(&args[0])?;
                let lambda = self.lambda(&args[1], 1, opcode)?;
                let slot = self.fresh();
                let body = self.under(lambda, vec![Expr::Local(slot)])?;
                Ok(Expr::Sweep {
                    sweep,
                    shelf: kept_as,
                    slot,
                    source: boxed(source),
                    body: boxed(body),
                })
            }
            Data::SetMap | Data::MapBy => {
                self.given(opcode, args, 2)?;
                let source = self.pure(&args[0])?;
                let lambda = self.lambda(&args[1], 1, opcode)?;
                let carried = self.carried();
                let image = self.under(lambda, vec![carried.item()])?;
                let (start, body) = if data == Data::SetMap {
                    let added = apply(Op::Set(SetOp::Add), vec![carried.total(), image]);
                    (empty_set(), added)
                } else {
                    let put = vec![carried.total(), carried.item(), image];
                    (empty_map(), apply(Op::Map(MapOp::Set), put))
                };
                Ok(Self::folded(&carried, source, start, body))
            }
            Data::Fold(kept_as) => {
                self.given(opcode, args, 3)?;
                let source = self.pure(&args[0])?;
                let start = self.pure(&args[1])?;
                let lambda = self.lambda(&args[2], 2, opcode)?;
                let carried = self.carried();
                let body = self.under(lambda, vec![carried.total(), carried.item()])?;
                Ok(Self::folded_as(kept_as, &carried, source, start, body))
            }
            Data::Flatten => {
                self.given(opcode, args, 1)?;
                let source = self.pure(&args[0])?;
                let carried = self.carried();
                let body = apply(Op::Set(SetOp::Union), vec![carried.total(), carried.item()]);
                Ok(Self::folded(&carried, source, empty_set(), body))
            }
            Data::SetToMap => {
                self.given(opcode, args, 1)?;
                let source = self.pure(&args[0])?;
                let carried = self.carried();
                let (key, value) = (self.position(1), self.position(2));
                let body = apply(
                    Op::Map(MapOp::Set),
                    vec![
                        carried.total(),
                        Expr::Get(boxed(carried.item()), key),
                        Expr::Get(boxed(carried.item()), value),
                    ],
                );
                Ok(Self::folded(&carried, source, empty_map(), body))
            }
            Data::Powerset => self.powerset(opcode, args),
            Data::IsFinite => {
                self.given(opcode, args, 1)?;
                let source = self.pure(&args[0])?;
                Ok(apply(Op::Equal, vec![source.clone(), source]))
            }
            Data::SetBy => self.set_by(opcode, args),
            Data::Indices => {
                self.given(opcode, args, 1)?;
                let list = self.pure(&args[0])?;
                let size = apply(Op::Seq(SeqOp::Size), vec![list]);
                let highest = apply(
                    Op::Math(Math::Subtract),
                    vec![size, Expr::Const(Value::Int(1))],
                );
                Ok(apply(
                    Op::Interval,
                    vec![Expr::Const(Value::Int(0)), highest],
                ))
            }
            _ => self.built(data, opcode, args),
        }
    }

    fn powerset(&mut self, opcode: &str, args: &'a [Quint]) -> Read<Expr> {
        self.given(opcode, args, 1)?;
        let source = self.pure(&args[0])?;
        match constant(&source, self.slots)? {
            Some(Value::Set(items))
                if u32::try_from(items.len()).is_ok_and(|count| count <= MOST_POWERSET_ITEMS) =>
            {
                Ok(Expr::Const(Value::set(powerset(&items))))
            }
            Some(Value::Set(_)) => self.refuse(
                "`powerset`",
                &format!("the powerset of a set of more than {MOST_POWERSET_ITEMS} items"),
            ),
            _ => self.refuse(
                "`powerset`",
                "`powerset` is read only of a set that does not change, worked out when \
                         the file is read",
            ),
        }
    }

    fn set_by(&mut self, opcode: &str, args: &'a [Quint]) -> Read<Expr> {
        self.given(opcode, args, 3)?;
        let map = self.pure(&args[0])?;
        let key = self.pure(&args[1])?;
        let lambda = self.lambda(&args[2], 1, opcode)?;
        let slot = self.fresh();
        let changed = self.under(lambda, vec![Expr::Local(slot)])?;
        let old = apply(Op::Map(MapOp::MustGet), vec![map.clone(), key.clone()]);
        Ok(Expr::Bind {
            slot,
            value: boxed(old),
            body: boxed(apply(Op::Map(MapOp::Set), vec![map, key, changed])),
        })
    }

    fn each(&mut self, args: &'a [Quint]) -> Read<Vec<Expr>> {
        args.iter().map(|each| self.pure(each)).collect()
    }

    fn record(&mut self, args: &'a [Quint], opcode: &str) -> Read<Expr> {
        let mut fields = Vec::with_capacity(args.len() / 2);
        for pair in args.chunks(2) {
            let [name, value] = pair else {
                return self.refuse("`Rec`", "a field without a value");
            };
            let name = self.label(name, opcode)?;
            fields.push((name, self.pure(value)?));
        }
        Ok(Expr::Record(fields))
    }

    fn tuple(&mut self, items: Vec<Expr>) -> Expr {
        let fields = items
            .into_iter()
            .enumerate()
            .map(|(index, item)| (self.position(index.saturating_add(1)), item))
            .collect();
        Expr::Record(fields)
    }

    fn tuples(&mut self, sets: Vec<Expr>) -> Expr {
        let mut picked = Vec::with_capacity(sets.len());
        let mut folds = Vec::with_capacity(sets.len());
        for set in sets {
            let carried = self.carried();
            picked.push(carried.item());
            folds.push((carried, set));
        }
        let mut inner = None;
        let one = self.tuple(picked);
        for (carried, set) in folds.into_iter().rev() {
            let body = match inner.take() {
                None => apply(Op::Set(SetOp::Add), vec![carried.total(), one.clone()]),
                Some(deeper) => apply(Op::Set(SetOp::Union), vec![carried.total(), deeper]),
            };
            inner = Some(Self::folded(&carried, set, empty_set(), body));
        }
        inner.unwrap_or_else(|| Expr::Const(Value::set(BTreeSet::from([Value::record(vec![])]))))
    }

    fn map_of(&mut self, args: &'a [Quint]) -> Read<Expr> {
        let (key, value) = (self.position(1), self.position(2));
        let mut entries = Vec::with_capacity(args.len());
        for pair in self.each(args)? {
            let entry = match pair {
                Expr::Record(fields) if fields.len() == 2 => {
                    let mut parts = fields.into_iter().map(|(_, part)| part);
                    match (parts.next(), parts.next()) {
                        (Some(first), Some(second)) => (first, second),
                        _ => return self.refuse("`Map`", "a pair without two parts"),
                    }
                }
                other => (
                    Expr::Get(boxed(other.clone()), key),
                    Expr::Get(boxed(other), value),
                ),
            };
            entries.push(entry);
        }
        Ok(Expr::MapOf(entries))
    }

    fn matched(&mut self, args: &'a [Quint]) -> Read<Expr> {
        let Some((subject, cases)) = args.split_first() else {
            return self.refuse("`match`", "a `match` without a value");
        };
        let subject = self.pure(subject)?;
        let slot = self.fresh();
        let (tag_name, value_name) = (self.text(TAG), self.text(VALUE));
        let tag = Expr::Get(boxed(Expr::Local(slot)), tag_name);
        let value = Expr::Get(boxed(Expr::Local(slot)), value_name);
        let no_case = apply(Op::Map(MapOp::MustGet), vec![empty_map(), tag.clone()]);
        let mut read = Vec::with_capacity(cases.len() / 2);
        for case in cases.chunks(2) {
            let [Quint::Str { value: name }, lambda] = case else {
                return self.refuse("`match`", "a case that is not a tag and an operator");
            };
            let lambda = self.lambda(lambda, 1, "match")?;
            let body = self.under(lambda, vec![value.clone()])?;
            let tested = (name != ANY_OTHER_TAG).then(|| self.text(name));
            read.push((tested, body));
        }
        let mut chosen = no_case;
        for (tested, body) in read.into_iter().rev() {
            chosen = match tested {
                None => body,
                Some(name) => Expr::Choose(
                    boxed(apply(
                        Op::Equal,
                        vec![tag.clone(), Expr::Const(Value::Str(name))],
                    )),
                    boxed(body),
                    boxed(chosen),
                ),
            };
        }
        Ok(Expr::Bind {
            slot,
            value: boxed(subject),
            body: boxed(chosen),
        })
    }
}
