use earnest::model::{Model, model, require};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Model)]
struct Clock {
    #[range(-2, 2)]
    #[initial(0)]
    skew: i8,
    #[initial(false)]
    synced: bool,
}

#[model]
impl Clock {
    #[action("the clock drifts by a second")]
    fn drift(&self, #[one_of(-1, 1)] by: i8) -> Option<Self> {
        require!(!self.synced && self.skew + by >= -2 && self.skew + by <= 2);
        Some(Self {
            skew: self.skew + by,
            ..self.clone()
        })
    }

    #[action("the clock is synced")]
    fn sync(&self) -> Option<Self> {
        require!(!self.synced);
        Some(Self {
            skew: 0,
            synced: true,
        })
    }

    #[end("A synced clock is done")]
    fn done(&self) -> bool {
        self.synced
    }

    #[rule("A synced clock shows no skew")]
    fn shows_zero(&self) -> bool {
        !self.synced || self.skew == 0
    }

    #[eventually("Every run comes to a synced clock")]
    fn syncs(&self) -> bool {
        self.synced
    }

    #[setup("The clock runs two seconds behind")]
    fn behind(&self) -> Self {
        Self {
            skew: -2,
            ..self.clone()
        }
    }

    #[setup("The clock starts synced")]
    fn synced_already(&self) -> Self {
        Self {
            synced: true,
            ..self.clone()
        }
    }

    #[witness("A second ahead after one drift")]
    fn drifts_ahead(&self) -> Option<bool> {
        let after = self.drift(1)?;
        Some(after.skew == 1)
    }

    #[witness("Behind, it drifts forward and is synced", setup = behind)]
    fn catches_up(&self) -> Option<bool> {
        let after = self.drift(1)?.sync()?;
        Some(after.skew == 0 && after.synced)
    }
}
