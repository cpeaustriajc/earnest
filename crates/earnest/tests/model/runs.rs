use earnest::model::{Behavior, Eventually, Model, Setup, Witnessed, model, require};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Model)]
struct Shop {
    #[range(0, 3)]
    #[initial(0)]
    stock: u8,
    #[initial(false)]
    open: bool,
}

#[model]
impl Shop {
    #[action("the shop opens", fair)]
    fn open_up(&self) -> Option<Self> {
        require!(!self.open);
        Some(Self {
            open: true,
            ..self.clone()
        })
    }

    #[action("a delivery comes")]
    fn deliver(&self, #[range(1, 2)] boxes: u8) -> Option<Self> {
        require!(self.stock + boxes <= 3);
        Some(Self {
            stock: self.stock + boxes,
            ..self.clone()
        })
    }

    #[action("the shop closes")]
    fn close(&self) -> Self {
        Self {
            open: false,
            ..self.clone()
        }
    }

    fn stocked(&self) -> bool {
        self.stock > 0
    }

    #[eventually("Every run opens the shop")]
    fn opens(&self) -> bool {
        self.open
    }

    #[eventually("A stocked shop fills up", after = stocked)]
    fn fills(&self) -> bool {
        self.stock == 3
    }

    #[setup("The shop starts full")]
    fn full(&self) -> Self {
        Self {
            stock: 3,
            ..self.clone()
        }
    }

    #[witness("Open, close, then deliver two boxes")]
    fn two_boxes(&self) -> Option<bool> {
        let after = self.open_up()?.close().deliver(2)?;
        Some(after.stock == 2 && !after.open)
    }

    #[witness("A full shop takes no delivery", setup = full)]
    fn no_room(&self) -> Option<bool> {
        let after = self.deliver(1)?;
        Some(after.stock == 3)
    }
}

#[test]
fn a_witness_method_plays_its_steps_and_gives_none_where_one_can_not_happen() {
    let start = Shop::starts().remove(0);
    assert_eq!(start.two_boxes(), Some(true), "the steps can be taken");
    assert_eq!(
        start.witnesses(),
        [
            Witnessed {
                id: "two_boxes",
                says: "Open, close, then deliver two boxes",
                setup: None,
                played: Some(true),
            },
            Witnessed {
                id: "no_room",
                says: "A full shop takes no delivery",
                setup: Some("full"),
                played: Some(false),
            },
        ],
        "from the model's start, a delivery of one is no full shop"
    );
    let full = start.full();
    assert_eq!(
        full.no_room(),
        None,
        "a full shop takes no delivery, so the witness breaks"
    );
    assert_eq!(
        start.setups(),
        [Setup {
            id: "full",
            says: "The shop starts full",
            state: Shop {
                stock: 3,
                open: false
            },
        }],
        "the setup applied to the start"
    );
}

#[test]
fn the_rules_about_runs_give_what_their_methods_give_in_a_state() {
    let start = Shop::starts().remove(0);
    assert_eq!(
        start.eventually(),
        [
            Eventually {
                id: "opens",
                says: "Every run opens the shop",
                after: None,
                reaches: false,
            },
            Eventually {
                id: "fills",
                says: "A stocked shop fills up",
                after: Some(false),
                reaches: false,
            },
        ],
        "at the start"
    );
    let full = start.full();
    let judged: Vec<(Option<bool>, bool)> = full
        .eventually()
        .iter()
        .map(|rule| (rule.after, rule.reaches))
        .collect();
    assert_eq!(
        judged,
        [(None, false), (Some(true), true)],
        "a full shop is stocked and full"
    );
    let ids: Vec<&str> = start.steps().iter().map(|step| step.action).collect();
    assert_eq!(
        ids,
        ["open_up", "deliver", "deliver", "close"],
        "a fair action is an action like any other"
    );
}
