use crate::choices::{Fastest, Slot, domain_choices, for_each_combination};
use proc_macro2::TokenStream;
use quote::quote;
use syn::ext::IdentExt;
use syn::parse::ParseStream;
use syn::spanned::Spanned;
use syn::{Attribute, FnArg, Ident, ImplItem, ImplItemFn, ItemImpl, LitStr, Pat, Token};

const ROLES: [&str; 6] = ["action", "rule", "end", "eventually", "witness", "setup"];
const OPTIONS: [(&str, &str, bool); 3] = [
    ("action", "fair", false),
    ("eventually", "after", true),
    ("witness", "setup", true),
];

struct Role {
    name: &'static str,
    sentence: LitStr,
    named: Option<Ident>,
}

type Written = (LitStr, Vec<(Ident, Option<Ident>)>);

fn written(input: ParseStream<'_>) -> syn::Result<Written> {
    let sentence: LitStr = input.parse()?;
    let mut options = Vec::new();
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
        options.push((name, value));
    }
    Ok((sentence, options))
}

fn what_it_takes(role: &str) -> String {
    match role {
        "action" => {
            "an action takes its sentence, then `fair`: `#[action(\"...\", fair)]`".to_owned()
        }
        "eventually" => "a rule about runs takes its sentence, then `after = method`, a helper \
            method that takes `&self` and returns `bool`: `#[eventually(\"...\", after = paid)]`"
            .to_owned(),
        "witness" => "a witness takes its sentence, then `setup = method`, a `#[setup]` method: \
            `#[witness(\"...\", setup = behind)]`"
            .to_owned(),
        other => format!("`#[{other}]` takes only its sentence, in quotes: `#[{other}(\"...\")]`"),
    }
}

fn named_option(
    role: &'static str,
    given: Vec<(Ident, Option<Ident>)>,
) -> syn::Result<Option<Ident>> {
    let mut named = None;
    let mut seen: Vec<String> = Vec::new();
    for (option, value) in given {
        let written = option.unraw().to_string();
        let Some((_, _, takes_a_name)) = OPTIONS
            .iter()
            .find(|(owner, known, _)| *owner == role && *known == written)
        else {
            return Err(syn::Error::new(option.span(), what_it_takes(role)));
        };
        if seen.contains(&written) {
            return Err(syn::Error::new(
                option.span(),
                format!("`{written}` is given twice"),
            ));
        }
        seen.push(written);
        match (value, *takes_a_name) {
            (None, false) => {}
            (Some(method), true) => named = Some(method),
            _ => return Err(syn::Error::new(option.span(), what_it_takes(role))),
        }
    }
    Ok(named)
}

pub(crate) fn expand(arguments: &TokenStream, input: TokenStream) -> syn::Result<TokenStream> {
    if !arguments.is_empty() {
        return Err(syn::Error::new(
            arguments.span(),
            "`#[model]` takes no arguments",
        ));
    }
    let mut block: ItemImpl = syn::parse2(input)?;
    if let Some((_, path, _)) = &block.trait_ {
        return Err(syn::Error::new(
            path.span(),
            "`#[model]` goes on the model's own `impl` block, not on a trait's",
        ));
    }
    let mut actions = Vec::new();
    let mut rules = Vec::new();
    let mut ends = Vec::new();
    let mut eventually = Vec::new();
    let mut witnesses = Vec::new();
    let mut setups = Vec::new();
    for item in &mut block.items {
        let ImplItem::Fn(method) = item else {
            continue;
        };
        let Some(role) = take_role(&mut method.attrs)? else {
            continue;
        };
        match role.name {
            "action" => actions.push(steps_of(method, &role.sentence)?),
            "rule" => rules.push(claim_of(method, &role.sentence)),
            "end" => ends.push(claim_of(method, &role.sentence)),
            "eventually" => eventually.push(eventually_of(method, &role)),
            "witness" => witnesses.push(witness_of(method, &role)),
            _ => setups.push(setup_of(method, &role.sentence)),
        }
    }
    let owner = &block.self_ty;
    Ok(quote! {
        #block

        #[automatically_derived]
        impl ::earnest::model::Behavior for #owner {
            fn steps(&self) -> ::std::vec::Vec<::earnest::model::Step<Self>> {
                ::std::iter::empty()
                    #(.chain(#actions))*
                    .collect()
            }

            fn rules(&self) -> ::std::vec::Vec<::earnest::model::Claim> {
                ::std::vec![#(#rules),*]
            }

            fn ends(&self) -> ::std::vec::Vec<::earnest::model::Claim> {
                ::std::vec![#(#ends),*]
            }

            fn eventually(&self) -> ::std::vec::Vec<::earnest::model::Eventually> {
                ::std::vec![#(#eventually),*]
            }

            fn witnesses(&self) -> ::std::vec::Vec<::earnest::model::Witnessed> {
                ::std::vec![#(#witnesses),*]
            }

            fn setups(&self) -> ::std::vec::Vec<::earnest::model::Setup<Self>> {
                ::std::vec![#(#setups),*]
            }
        }
    })
}

fn take_role(attributes: &mut Vec<Attribute>) -> syn::Result<Option<Role>> {
    let mut found = None;
    let mut kept = Vec::with_capacity(attributes.len());
    for attribute in attributes.drain(..) {
        let role = ROLES
            .iter()
            .find(|role| attribute.path().is_ident(role))
            .copied();
        let Some(name) = role else {
            kept.push(attribute);
            continue;
        };
        if found.is_some() {
            return Err(syn::Error::new(
                attribute.span(),
                "a method is an action, a rule, an end, a rule about runs, a witness or a \
                 setup, never two of them",
            ));
        }
        let (sentence, given) = attribute.parse_args_with(written).map_err(|_| {
            syn::Error::new(
                attribute.span(),
                format!(
                    "write the sentence in quotes: `#[{name}(\"...\")]`; {}",
                    what_it_takes(name)
                ),
            )
        })?;
        let named = named_option(name, given)?;
        found = Some(Role {
            name,
            sentence,
            named,
        });
    }
    *attributes = kept;
    Ok(found)
}

fn gives_self(signature: &syn::Signature) -> bool {
    matches!(&signature.output, syn::ReturnType::Type(_, ty)
        if matches!(ty.as_ref(), syn::Type::Path(path) if path.qself.is_none() && path.path.is_ident("Self")))
}

fn takes_self(method: &ImplItemFn) -> bool {
    matches!(method.sig.inputs.first(), Some(FnArg::Receiver(_)))
}

fn claim_of(method: &ImplItemFn, sentence: &LitStr) -> TokenStream {
    let name = &method.sig.ident;
    let id = name.unraw().to_string();
    quote! {
        ::earnest::model::Claim {
            id: #id,
            says: #sentence,
            holds: Self::#name(self),
        }
    }
}

fn eventually_of(method: &ImplItemFn, role: &Role) -> TokenStream {
    let name = &method.sig.ident;
    let id = name.unraw().to_string();
    let sentence = &role.sentence;
    let after = role.named.as_ref().map_or_else(
        || quote! { ::core::option::Option::None },
        |after| quote! { ::core::option::Option::Some(Self::#after(self)) },
    );
    quote! {
        ::earnest::model::Eventually {
            id: #id,
            says: #sentence,
            after: #after,
            reaches: Self::#name(self),
        }
    }
}

fn gives_bool(signature: &syn::Signature) -> bool {
    matches!(&signature.output, syn::ReturnType::Type(_, ty)
        if matches!(ty.as_ref(), syn::Type::Path(path) if path.qself.is_none() && path.path.is_ident("bool")))
}

fn witness_of(method: &ImplItemFn, role: &Role) -> TokenStream {
    let name = &method.sig.ident;
    let id = name.unraw().to_string();
    let sentence = &role.sentence;
    let setup = role.named.as_ref().map_or_else(
        || quote! { ::core::option::Option::None },
        |setup| {
            let named = setup.unraw().to_string();
            quote! {
                {
                    let _: fn(&Self) -> Self = Self::#setup;
                    ::core::option::Option::Some(#named)
                }
            }
        },
    );
    let played = if gives_bool(&method.sig) {
        quote! { ::core::option::Option::Some(Self::#name(self)) }
    } else {
        quote! { Self::#name(self) }
    };
    quote! {
        ::earnest::model::Witnessed {
            id: #id,
            says: #sentence,
            setup: #setup,
            played: #played,
        }
    }
}

fn setup_of(method: &ImplItemFn, sentence: &LitStr) -> TokenStream {
    let name = &method.sig.ident;
    let id = name.unraw().to_string();
    quote! {
        ::earnest::model::Setup {
            id: #id,
            says: #sentence,
            state: Self::#name(self),
        }
    }
}

fn steps_of(method: &mut ImplItemFn, sentence: &LitStr) -> syn::Result<TokenStream> {
    if !takes_self(method) {
        return Err(syn::Error::new(
            method.sig.span(),
            "an action takes `&self` first: it reads the state it starts from",
        ));
    }
    let mut slots = Vec::new();
    for input in &mut method.sig.inputs {
        let FnArg::Typed(typed) = input else {
            continue;
        };
        let Pat::Ident(pattern) = typed.pat.as_ref() else {
            return Err(syn::Error::new(
                typed.pat.span(),
                "an action's input is a plain name with its type",
            ));
        };
        let choices = domain_choices(&typed.attrs, &typed.ty)?;
        slots.push(Slot::new(&pattern.ident, choices));
        typed.attrs.retain(|attribute| {
            !attribute.path().is_ident("range") && !attribute.path().is_ident("one_of")
        });
    }
    let name = &method.sig.ident;
    let id = name.unraw().to_string();
    let copies = slots.iter().map(Slot::copy);
    let shown = slots.iter().map(|slot| {
        let input = slot.name.unraw().to_string();
        let variable = &slot.variable;
        quote! { (#input, ::earnest::model::Report::report(&#variable)) }
    });
    let step = quote! {
        earnest_steps.push(::earnest::model::Step {
            action: #id,
            label: #sentence,
            input: ::std::vec![#(#shown),*],
            state: earnest_next,
        });
    };
    let attempt = if gives_self(&method.sig) {
        quote! {
            let earnest_next = Self::#name(self #(, #copies)*);
            #step
        }
    } else {
        quote! {
            if let ::core::option::Option::Some(earnest_next) = Self::#name(self #(, #copies)*) {
                #step
            }
        }
    };
    let loops = for_each_combination(&slots, Fastest::First, attempt);
    Ok(quote! {
        {
            let mut earnest_steps = ::std::vec::Vec::new();
            #loops
            earnest_steps
        }
    })
}

#[cfg(test)]
mod tests {
    use super::expand;
    use proc_macro2::TokenStream;
    use quote::quote;

    fn refusal(input: TokenStream) -> String {
        expand(&TokenStream::new(), input)
            .expect_err("refused")
            .to_string()
    }

    #[test]
    fn the_methods_stay_and_lose_only_the_attributes_the_macro_reads() {
        let expanded = expand(
            &TokenStream::new(),
            quote! {
                impl Mailbox {
                    #[action("the postman delivers a letter")]
                    #[must_use]
                    fn deliver(&self, letter: Letter, #[range(1, 2)] copies: u8) -> Option<Self> {
                        None
                    }

                    #[rule("A full inbox holds a card")]
                    fn bills(&self) -> bool {
                        true
                    }

                    #[end("The inbox is full")]
                    fn full(&self) -> bool {
                        false
                    }

                    fn helper(&self) -> bool {
                        true
                    }
                }
            },
        )
        .expect("a model block")
        .to_string();
        let kept = quote! {
            #[must_use]
            fn deliver(&self, letter: Letter, copies: u8) -> Option<Self>
        }
        .to_string();
        assert!(expanded.contains(&kept), "{kept} in {expanded}");
        for gone in ["# [action", "# [rule", "# [end", "# [range"] {
            assert!(!expanded.contains(gone), "{gone} in {expanded}");
        }
        let wanted = [
            quote! { impl ::earnest::model::Behavior for Mailbox },
            quote! { for earnest_choice_letter in <Letter as ::earnest::model::Domain>::values() },
            quote! { for earnest_choice_copies in (1)..=(2) },
            quote! { action: "deliver", label: "the postman delivers a letter" },
            quote! { id: "bills", says: "A full inbox holds a card", holds: Self::bills(self) },
            quote! { id: "full", says: "The inbox is full", holds: Self::full(self) },
            quote! { fn helper(&self) -> bool },
        ];
        for piece in wanted {
            let text = piece.to_string();
            assert!(expanded.contains(&text), "{text} in {expanded}");
        }
    }

    #[test]
    fn arguments_a_trait_block_and_two_roles_are_refused() {
        let with_arguments = expand(&quote! { strict }, quote! { impl Mailbox {} })
            .expect_err("refused")
            .to_string();
        assert!(with_arguments.contains("no arguments"), "{with_arguments}");
        let on_trait = refusal(quote! { impl Clone for Mailbox {} });
        assert!(on_trait.contains("not on a trait's"), "{on_trait}");
        let twice = refusal(quote! {
            impl Mailbox {
                #[rule("One")]
                #[end("Two")]
                fn both(&self) -> bool {
                    true
                }
            }
        });
        assert!(twice.contains("never two of them"), "{twice}");
    }

    #[test]
    fn a_missing_sentence_an_action_without_self_and_an_unranged_integer_are_refused() {
        let bare = refusal(quote! {
            impl Mailbox {
                #[rule]
                fn bills(&self) -> bool {
                    true
                }
            }
        });
        assert!(bare.contains("#[rule(\"...\")]"), "{bare}");
        let selfless = refusal(quote! {
            impl Mailbox {
                #[action("the postman delivers a letter")]
                fn deliver(letter: Letter) -> Option<Self> {
                    None
                }
            }
        });
        assert!(selfless.contains("takes `&self` first"), "{selfless}");
        let unranged = refusal(quote! {
            impl Mailbox {
                #[action("the postman delivers some letters")]
                fn deliver(&self, copies: u8) -> Option<Self> {
                    None
                }
            }
        });
        assert!(unranged.contains("#[range(lo, hi)]"), "{unranged}");
        let pattern = refusal(quote! {
            impl Mailbox {
                #[action("the postman delivers a pair")]
                fn deliver(&self, (first, second): (Letter, Letter)) -> Option<Self> {
                    None
                }
            }
        });
        assert!(pattern.contains("a plain name"), "{pattern}");
    }

    #[test]
    fn an_action_that_gives_back_the_state_is_a_step_without_a_match() {
        let expanded = expand(
            &TokenStream::new(),
            quote! {
                impl Lamp {
                    #[action("the lamp is switched")]
                    fn toggle(&self, #[one_of("a", "b")] how: &'static str) -> Self {
                        self.clone()
                    }
                }
            },
        )
        .expect("a model block")
        .to_string();
        let wanted = [
            quote! { for earnest_choice_how in ::core::iter::IntoIterator::into_iter(["a", "b"]) },
            quote! { let earnest_next = Self::toggle(self, ::core::clone::Clone::clone(&earnest_choice_how)); },
        ];
        for piece in wanted {
            let text = piece.to_string();
            assert!(expanded.contains(&text), "{text} in {expanded}");
        }
        assert!(
            !expanded.contains("# [one_of"),
            "the attribute is read: {expanded}"
        );
    }

    #[test]
    fn rules_about_runs_witnesses_and_setups_lose_their_attributes_and_are_called_by_name() {
        let expanded = expand(
            &TokenStream::new(),
            quote! {
                impl Gate {
                    #[action("the visitor pays", fair)]
                    fn pay(&self) -> Option<Self> {
                        None
                    }

                    fn paid(&self) -> bool {
                        true
                    }

                    #[eventually("Once paid, the gate opens", after = paid)]
                    fn opens(&self) -> bool {
                        true
                    }

                    #[eventually("Every run ends")]
                    fn ends(&self) -> bool {
                        true
                    }

                    #[setup("The gate starts open")]
                    fn open(&self) -> Self {
                        self.clone()
                    }

                    #[witness("Pay, then the gate opens", setup = open)]
                    fn pays(&self) -> Option<bool> {
                        let after = self.pay()?;
                        Some(after.opens())
                    }

                    #[witness("The gate starts open")]
                    fn opened(&self) -> bool {
                        self.opens()
                    }
                }
            },
        )
        .expect("a model block")
        .to_string();
        for gone in [
            "# [action",
            "# [eventually",
            "# [setup",
            "# [witness",
            "fair",
            "identity",
        ] {
            assert!(!expanded.contains(gone), "{gone} in {expanded}");
        }
        let wanted = [
            quote! { fn pay(&self) -> Option<Self> },
            quote! { action: "pay", label: "the visitor pays" },
            quote! {
                id: "opens",
                says: "Once paid, the gate opens",
                after: ::core::option::Option::Some(Self::paid(self)),
                reaches: Self::opens(self),
            },
            quote! { after: ::core::option::Option::None, reaches: Self::ends(self), },
            quote! { id: "open", says: "The gate starts open", state: Self::open(self), },
            quote! { let _: fn(&Self) -> Self = Self::open; },
            quote! { played: Self::pays(self), },
            quote! {
                fn pays(&self) -> Option<bool> {
                    let after = self.pay()?;
                    Some(after.opens())
                }
            },
            quote! { fn opened(&self) -> bool { self.opens() } },
            quote! { played: ::core::option::Option::Some(Self::opened(self)), },
        ];
        for piece in wanted {
            let text = piece.to_string();
            assert!(expanded.contains(&text), "{text} in {expanded}");
        }
    }

    #[test]
    fn an_option_another_attribute_takes_or_given_twice_is_refused() {
        for (attribute, words) in [
            (quote! { #[rule("One", fair)] }, "takes only its sentence"),
            (quote! { #[action("One", fair = yes)] }, "`fair`"),
            (quote! { #[action("One", fair, fair)] }, "given twice"),
            (quote! { #[eventually("One", after)] }, "`after = method`"),
            (
                quote! { #[eventually("One", before = paid)] },
                "`after = method`",
            ),
            (quote! { #[witness("One", fair)] }, "`setup = method`"),
            (
                quote! { #[witness("One", setup = 3)] },
                "write the sentence in quotes",
            ),
        ] {
            let found = refusal(quote! {
                impl Gate {
                    #attribute
                    fn one(&self) -> bool {
                        true
                    }
                }
            });
            assert!(found.contains(words), "{words} in {found}");
        }
    }
}
