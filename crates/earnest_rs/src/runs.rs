use crate::reader::{Reader, at, name_of, place_of, unwrapped};
use crate::ty::Ty;
use crate::typing::plain;
use earnest_ast::{
    ConditionDraft, EventuallyDraft, HelperKind, RunsDraft, SetupDraft, StepDraft, TermKind,
    WitnessDraft,
};
use proc_macro2::{Ident, TokenStream, TokenTree};
use std::collections::{BTreeMap, BTreeSet};
use syn::ext::IdentExt;
use syn::parse::ParseStream;
use syn::spanned::Spanned;
use syn::{
    Attribute, Expr, ExprMethodCall, FnArg, ImplItemFn, LitStr, Member, Pat, Signature, Stmt,
    Token, Type,
};

const STATE: &str = "self";
const FAIR: &str = "fair";
const AFTER: &str = "after";
const SETUP: &str = "setup";
const OPTIONS: [(&str, &str, bool); 3] = [
    ("action", FAIR, false),
    ("eventually", AFTER, true),
    ("witness", SETUP, true),
];
const WITNESS_BODY: &str = "a witness's body is `Some(condition)` on `self`, or `let after = \
    self.first()?.second()?;` followed by `Some(condition)` on that name: a chain of the model's \
    actions from `self`, with `?` after each one that returns `Option<Self>`";
const SETUP_BODY: &str = "a setup's body is `Self { field: value, ..self.clone() }`, where each \
    value is a constant: the fields written start there, and every other field where the model \
    declares";

pub(crate) struct Written {
    pub(crate) says: String,
    pub(crate) fair: bool,
    pub(crate) after: Option<Ident>,
    pub(crate) setup: Option<Ident>,
}

#[derive(Clone, Copy)]
enum Called {
    WithQuestionMark,
    Bare,
}

pub(crate) struct ActionShape {
    pub(crate) gives_self: Option<bool>,
    pub(crate) inputs: Vec<String>,
}

#[derive(Default)]
pub(crate) struct Known {
    pub(crate) actions: BTreeMap<String, ActionShape>,
    pub(crate) setups: BTreeSet<String>,
    pub(crate) roles: BTreeSet<String>,
}

type Options = (LitStr, Vec<(Ident, Option<Ident>)>);

fn options(input: ParseStream<'_>) -> syn::Result<Options> {
    let says: LitStr = input.parse()?;
    let mut found = Vec::new();
    while !input.is_empty() {
        input.parse::<Token![,]>()?;
        if input.is_empty() {
            break;
        }
        let name = input.call(Ident::parse_any)?;
        let value = if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            Some(input.call(Ident::parse_any)?)
        } else {
            None
        };
        found.push((name, value));
    }
    Ok((says, found))
}

fn what_it_takes(role: &str) -> String {
    match role {
        "action" => "an action takes its sentence, then `fair` when it is never put off forever \
            while it can happen: `#[action(\"...\", fair)]`"
            .to_owned(),
        "eventually" => "a rule about runs takes its sentence, then `after = method` when it \
            starts in the states where a helper method of the model is true: \
            `#[eventually(\"...\", after = paid)]`"
            .to_owned(),
        "witness" => "a witness takes its sentence, then `setup = method` when it is played from \
            a `#[setup]` method of the model: `#[witness(\"...\", setup = behind)]`"
            .to_owned(),
        other => format!("`#[{other}]` takes only its sentence, in quotes: `#[{other}(\"...\")]`"),
    }
}

fn mentions_the_state(stream: TokenStream) -> bool {
    stream.into_iter().any(|tree| match tree {
        TokenTree::Ident(name) => name == STATE,
        TokenTree::Group(group) => mentions_the_state(group.stream()),
        TokenTree::Punct(_) | TokenTree::Literal(_) => false,
    })
}

fn is_the_state(expr: &Expr) -> bool {
    matches!(unwrapped(expr), Expr::Path(name) if name.path.is_ident(STATE))
}

fn is_a_clone_of_the_state(expr: &Expr) -> bool {
    matches!(unwrapped(expr), Expr::MethodCall(call)
        if call.method == "clone" && call.args.is_empty() && is_the_state(&call.receiver))
}

impl Reader<'_> {
    pub(crate) fn written(&mut self, role: &str, attribute: &Attribute) -> Written {
        let mut written = Written {
            says: String::new(),
            fair: false,
            after: None,
            setup: None,
        };
        let Ok((says, given)) = attribute.parse_args_with(options) else {
            let message = format!(
                "write the sentence in quotes: `#[{role}(\"...\")]`; {}",
                what_it_takes(role)
            );
            self.refuse(attribute.span(), &format!("#[{role}]"), &message);
            return written;
        };
        written.says = says.value();
        let mut seen: Vec<String> = Vec::new();
        for (name, value) in given {
            let option = name_of(&name);
            let known = OPTIONS
                .iter()
                .find(|(owner, known, _)| *owner == role && *known == option);
            let Some((_, _, takes_a_name)) = known else {
                self.refuse(name.span(), &option, &what_it_takes(role));
                continue;
            };
            if seen.contains(&option) {
                self.refuse(name.span(), &option, &format!("`{option}` is given twice"));
                continue;
            }
            seen.push(option.clone());
            match (value, *takes_a_name) {
                (None, false) => written.fair = true,
                (Some(method), true) if option == AFTER => written.after = Some(method),
                (Some(method), true) => written.setup = Some(method),
                _ => self.refuse(name.span(), &option, &what_it_takes(role)),
            }
        }
        written
    }

    pub(crate) fn about_runs(
        &mut self,
        role: &str,
        method: &ImplItemFn,
        written: Written,
        known: &Known,
        runs: &mut RunsDraft,
    ) -> Option<Written> {
        match role {
            "eventually" => runs
                .eventually
                .extend(self.eventually(method, written, known)),
            "witness" => runs.witnesses.extend(self.witness(method, written, known)),
            "setup" => runs.setups.extend(self.setup(method, written.says)),
            _ => return Some(written),
        }
        None
    }

    fn only_the_state(&mut self, signature: &Signature, what: &str) {
        self.plain_function(signature);
        self.reads_state(signature);
        if let Some(FnArg::Typed(extra)) = signature.inputs.iter().nth(1) {
            self.refuse(
                extra.span(),
                &name_of(&signature.ident),
                &format!("{what} looks only at the state: it takes `&self` and nothing else"),
            );
        }
    }

    fn eventually(
        &mut self,
        method: &ImplItemFn,
        written: Written,
        known: &Known,
    ) -> Option<EventuallyDraft> {
        let signature = &method.sig;
        self.only_the_state(signature, "a rule about runs");
        self.returns(signature, "bool", "a rule about runs");
        self.walk_body(signature, &method.block, Ty::Bool);
        let reaches = ConditionDraft {
            state_name: Some(STATE.to_owned()),
            term: self.value_of(&method.block),
            text: self.body_text(&method.block),
        };
        let after = match &written.after {
            Some(named) => Some(self.after(named, known)?),
            None => None,
        };
        Some(EventuallyDraft {
            id: name_of(&signature.ident),
            says: written.says,
            place: place_of(signature.fn_token.span),
            after,
            reaches,
        })
    }

    fn after(&mut self, named: &Ident, known: &Known) -> Option<ConditionDraft> {
        let name = name_of(named);
        let takes_only_the_state = self
            .method_params
            .get(&name)
            .is_some_and(std::vec::Vec::is_empty);
        let gives = self
            .method_types
            .get(&name)
            .map(|ty| self.text(ty.span()).replace(' ', ""));
        let why = if known.roles.contains(&name) {
            Some(
                "`after` names a helper method of the model: an action, a rule, an end, a rule \
                 about runs, a witness or a setup can't be called from the model",
            )
        } else if !self.methods.contains(&name) {
            Some(
                "the model's block has no method with this name, and `rustc` refuses the file \
                 (E0599): `after` names a helper method of the model",
            )
        } else if !takes_only_the_state || gives.as_deref() != Some("bool") {
            Some(
                "`after` names a helper method that takes `&self` and nothing else and returns \
                 `bool`, the condition the rule starts from",
            )
        } else {
            None
        };
        if let Some(why) = why {
            self.refuse(named.span(), &name, why);
            return None;
        }
        let span = named.span();
        let state = at(span, TermKind::Name(STATE.to_owned()));
        Some(ConditionDraft {
            state_name: Some(STATE.to_owned()),
            term: at(
                span,
                TermKind::Helper {
                    kind: HelperKind::Method,
                    name: name.clone(),
                    arguments: vec![state],
                },
            ),
            text: name,
        })
    }

    fn setup(&mut self, method: &ImplItemFn, says: String) -> Option<SetupDraft> {
        let signature = &method.sig;
        self.only_the_state(signature, "a setup");
        self.returns(signature, "Self", "a setup");
        let returns = self.returns_of(signature);
        self.walk_body(signature, &method.block, returns);
        let built = match method.block.stmts.as_slice() {
            [Stmt::Expr(last, None)] => match unwrapped(last) {
                Expr::Struct(built)
                    if built.qself.is_none()
                        && (built.path.is_ident("Self")
                            || built.path.get_ident().map(name_of) == self.model_name)
                        && built.rest.as_deref().is_some_and(is_a_clone_of_the_state)
                        && !built.fields.is_empty() =>
                {
                    Some(built)
                }
                _ => None,
            },
            _ => None,
        };
        let Some(built) = built else {
            let span = method.block.span();
            self.refuse(span, &name_of(&signature.ident), SETUP_BODY);
            return None;
        };
        let mut starts = Vec::with_capacity(built.fields.len());
        for field in &built.fields {
            let Member::Named(name) = &field.member else {
                self.refuse(field.member.span(), "a field", SETUP_BODY);
                continue;
            };
            let written = self.text(field.expr.span());
            if written
                .parse::<TokenStream>()
                .map_or(true, mentions_the_state)
            {
                self.refuse(
                    field.expr.span(),
                    &self.short_text(field.expr.span()),
                    SETUP_BODY,
                );
                continue;
            }
            let value = self.term(&field.expr);
            starts.push((name_of(name), place_of(name.span()), value));
        }
        Some(SetupDraft {
            id: name_of(&signature.ident),
            says,
            place: place_of(signature.fn_token.span),
            starts,
        })
    }

    fn witness(
        &mut self,
        method: &ImplItemFn,
        written: Written,
        known: &Known,
    ) -> Option<WitnessDraft> {
        let signature = &method.sig;
        let id = name_of(&signature.ident);
        self.only_the_state(signature, "a witness");
        self.returns(signature, "Option<bool>", "a witness");
        let setup = match &written.setup {
            Some(named) if known.setups.contains(&name_of(named)) => {
                Some((name_of(named), place_of(named.span())))
            }
            Some(named) => {
                self.refuse(
                    named.span(),
                    &name_of(named),
                    "`setup` names a method of the model marked `#[setup(\"...\")]`, whose start \
                     states the witness is played from",
                );
                None
            }
            None => None,
        };
        let block_span = method.block.span();
        let (state, steps, last) = match method.block.stmts.as_slice() {
            [Stmt::Expr(last, None)] => (STATE.to_owned(), Vec::new(), last),
            [Stmt::Local(local), Stmt::Expr(last, None)] => {
                let named = match &local.pat {
                    Pat::Ident(named)
                        if named.by_ref.is_none()
                            && named.mutability.is_none()
                            && named.subpat.is_none() =>
                    {
                        named
                    }
                    other => {
                        self.refuse(other.span(), &self.short_text(other.span()), WITNESS_BODY);
                        return None;
                    }
                };
                self.refuse_constant_name(&named.ident);
                let Some(init) = local.init.as_ref().filter(|init| init.diverge.is_none()) else {
                    self.refuse(local.span(), &self.short_text(local.span()), WITNESS_BODY);
                    return None;
                };
                let steps = self.chain(&init.expr, known)?;
                if steps.is_empty() {
                    self.refuse(
                        init.expr.span(),
                        &self.short_text(init.expr.span()),
                        WITNESS_BODY,
                    );
                    return None;
                }
                (name_of(&named.ident), steps, last)
            }
            _ => {
                self.refuse(block_span, &id, WITNESS_BODY);
                return None;
            }
        };
        let condition = match unwrapped(last) {
            Expr::Call(call)
                if call.args.len() == 1
                    && matches!(unwrapped(&call.func), Expr::Path(path) if path.path.is_ident("Some")) =>
            {
                call.args.first()
            }
            _ => None,
        };
        let Some(condition) = condition else {
            self.refuse(last.span(), &self.short_text(last.span()), WITNESS_BODY);
            return None;
        };
        self.walk_condition(&state, condition);
        let kept = self.bound.len();
        if state != STATE {
            let model = self
                .model_name
                .clone()
                .and_then(|model| syn::parse_str::<Type>(&model).ok());
            self.bound.push(plain(state.clone(), model));
        }
        let expect = self.term(condition);
        self.bound.truncate(kept);
        Some(WitnessDraft {
            id,
            says: written.says,
            place: place_of(signature.fn_token.span),
            setup,
            steps,
            state_name: Some(state),
            expect,
        })
    }

    fn chain(&mut self, expr: &Expr, known: &Known) -> Option<Vec<StepDraft>> {
        match unwrapped(expr) {
            found if is_the_state(found) => Some(Vec::new()),
            Expr::Try(tried) => match unwrapped(&tried.expr) {
                Expr::MethodCall(call) => self.link(call, known, Called::WithQuestionMark),
                other => {
                    self.refuse(other.span(), &self.short_text(other.span()), WITNESS_BODY);
                    None
                }
            },
            Expr::MethodCall(call) => self.link(call, known, Called::Bare),
            other => {
                self.refuse(other.span(), &self.short_text(other.span()), WITNESS_BODY);
                None
            }
        }
    }

    fn link(
        &mut self,
        call: &ExprMethodCall,
        known: &Known,
        tried: Called,
    ) -> Option<Vec<StepDraft>> {
        let name = name_of(&call.method);
        let feature = format!(".{name}()");
        let span = call.method.span();
        let Some(shape) = known.actions.get(&name) else {
            self.refuse(
                span,
                &feature,
                "a witness's steps are calls of the model's own actions, one after another from \
                 `self`",
            );
            return None;
        };
        let refusal = match (shape.gives_self, tried) {
            (Some(true), Called::WithQuestionMark) => Some(
                "this action gives back `Self`, so it is not followed by `?`, and `rustc` \
                 refuses the `?` (E0277)",
            ),
            (Some(false), Called::Bare) => Some(
                "this action gives back `Option<Self>`: write `?` after it, so the witness gives \
                 `None` where the step can't happen",
            ),
            _ => None,
        };
        if let Some(why) = refusal {
            self.refuse(span, &feature, why);
            return None;
        }
        if call.turbofish.is_some() {
            self.refuse(span, &feature, WITNESS_BODY);
            return None;
        }
        if call.args.len() != shape.inputs.len() {
            let message = format!(
                "the action takes {} inputs and the step gives {}, and `rustc` refuses the file \
                 (E0061): give each input a value, in the order of its parameters",
                shape.inputs.len(),
                call.args.len()
            );
            self.refuse(span, &feature, &message);
            return None;
        }
        let mut steps = self.chain(&call.receiver, known)?;
        let inputs = shape
            .inputs
            .iter()
            .cloned()
            .zip(call.args.iter().map(|argument| self.term(argument)))
            .collect();
        steps.push(StepDraft {
            action: name,
            place: place_of(span),
            inputs,
        });
        Some(steps)
    }
}

pub(crate) fn input_names(signature: &Signature) -> Vec<String> {
    signature
        .inputs
        .iter()
        .filter_map(|input| match input {
            FnArg::Typed(typed) => match typed.pat.as_ref() {
                Pat::Ident(named) => Some(name_of(&named.ident)),
                _ => None,
            },
            FnArg::Receiver(_) => None,
        })
        .collect()
}
