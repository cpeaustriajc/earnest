use earnest::model::{Model, model, require};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Model)]
pub struct NarrowLevels {
    #[range(0, 1999)]
    #[initial(0)]
    t: u16,
    #[range(0, 599)]
    #[initial(0)]
    y: u16,
}

#[model]
impl NarrowLevels {
    #[action("time passes")]
    fn step(&self) -> Option<Self> {
        require!(self.t < 1999);
        Some(Self { t: self.t + 1, ..self.clone() })
    }

    #[action("time passes and the walker moves")]
    fn shift(&self) -> Option<Self> {
        require!(self.t < 1999 && self.y < 599);
        Some(Self { t: self.t + 1, y: self.y + 1 })
    }

    #[rule("The walker stays in the corridor")]
    fn inside(&self) -> bool {
        self.y <= 599
    }

    #[end("Time is at its last tick")]
    fn stopped(&self) -> bool {
        self.t == 1999
    }
}
