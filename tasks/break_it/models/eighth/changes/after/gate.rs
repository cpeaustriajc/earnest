use earnest::model::{Model, model, require};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Model)]
struct Gate {
    #[initial(0)]
    #[range(0, 2)]
    coins: u8,
    #[initial(false)]
    open: bool,
}

#[model]
impl Gate {
    #[action("the visitor drops a coin")]
    fn pay(&self) -> Option<Self> {
        require!(self.coins < 2 && !self.open);
        Some(Self {
            coins: self.coins + 1,
            ..self.clone()
        })
    }

    #[action("the gate opens for two coins")]
    fn unlock(&self) -> Option<Self> {
        require!(self.coins == 2 && !self.open);
        Some(Self {
            open: true,
            ..self.clone()
        })
    }

    #[end("The gate is open")]
    fn through(&self) -> bool {
        self.open
    }

    fn paid(&self) -> bool {
        self.coins >= 2
    }

    #[eventually("Once the visitor pays, the gate opens", after = paid)]
    fn opens(&self) -> bool {
        self.open
    }
}
