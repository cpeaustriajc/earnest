use earnest::model::{Domain, Model, model, require};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Domain)]
enum Door {
    Locked,
    Open,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Model)]
struct Gate {
    #[initial(Door::Locked)]
    door: Door,
    #[initial(false)]
    paid: bool,
    #[initial(false)]
    light: bool,
}

#[model]
impl Gate {
    #[action("the light blinks while nobody has paid")]
    fn blink(&self) -> Option<Self> {
        require!(!self.paid);
        Some(Self {
            light: !self.light,
            ..self.clone()
        })
    }

    #[action("the visitor pays", fair)]
    fn pay(&self) -> Option<Self> {
        require!(!self.paid && self.door == Door::Locked);
        Some(Self {
            paid: true,
            ..self.clone()
        })
    }

    #[action("the visitor pushes")]
    fn push(&self) -> Option<Self> {
        require!(self.paid && self.door == Door::Locked);
        Some(Self {
            door: Door::Open,
            ..self.clone()
        })
    }

    #[action("the visitor walks through")]
    fn enter(&self) -> Option<Self> {
        require!(self.door == Door::Open);
        Some(Self {
            door: Door::Locked,
            paid: false,
            ..self.clone()
        })
    }

    fn paid_up(&self) -> bool {
        self.paid
    }

    #[eventually("Every run comes to an open gate")]
    fn opens(&self) -> bool {
        self.door == Door::Open
    }

    #[eventually("Once the visitor pays, the gate opens", after = paid_up)]
    fn opens_once_paid(&self) -> bool {
        self.door == Door::Open
    }

    #[eventually("Every run comes to a paid gate with the light on")]
    fn lit_and_paid(&self) -> bool {
        self.paid && self.light
    }

    #[witness("Pay, then push: the gate opens")]
    fn pay_then_push(&self) -> Option<bool> {
        let after = self.pay()?.push()?;
        Some(after.door == Door::Open)
    }

    #[witness("Push before paying: the gate opens")]
    fn push_first(&self) -> Option<bool> {
        let after = self.push()?;
        Some(after.door == Door::Open)
    }
}
