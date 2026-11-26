use earnest::model::{Model, model, require};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Model)]
struct Ladder {
    #[initial(0)]
    #[range(0, 3)]
    rung: u8,
}

#[model]
impl Ladder {
    #[action("the climber goes up one rung")]
    fn up(&self) -> Option<Self> {
        require!(self.rung < 3);
        Some(Self {
            rung: self.rung + 1,
        })
    }

    #[end("The climber is at the top")]
    fn top(&self) -> bool {
        self.rung == 3
    }

    #[witness("Three steps up reach the top")]
    fn three(&self) -> Option<bool> {
        let after = self.up()?.up()?.up()?;
        Some(after.rung == 3)
    }
}
