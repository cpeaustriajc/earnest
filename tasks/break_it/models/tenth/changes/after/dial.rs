use earnest::model::{Model, model, require};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Model)]
pub struct Dial {
    #[range(0, 3)]
    #[initial(0)]
    n: u8,
}

#[model]
impl Dial {
    #[action("the dial turns")]
    fn turn(&self, #[range(1, 2)] by: u8, #[range(0, 3)] to: u8) -> Option<Self> {
        require!(self.n == 0 && by == 2);
        Some(Self { n: to })
    }

    #[end("the dial is set")]
    fn set(&self) -> bool {
        self.n != 0
    }

    #[witness("turning to one with two sets the dial to one")]
    fn once(&self) -> Option<bool> {
        let after = self.turn(1, 2)?;
        Some(after.n == 1)
    }
}
