use earnest::model::{Domain, Model, model, require};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Domain)]
enum Phase {
    Cold,
    Warm,
    Hot,
    Off,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Model)]
struct Kettle {
    #[initial(Phase::Cold)]
    phase: Phase,
}

#[model]
impl Kettle {
    #[action("the kettle heats the water")]
    fn heat(&self) -> Option<Self> {
        require!(self.phase == Phase::Cold);
        Some(Self { phase: Phase::Warm })
    }

    #[action("the water boils")]
    fn boil(&self) -> Option<Self> {
        require!(self.phase == Phase::Warm);
        Some(Self { phase: Phase::Hot })
    }

    #[action("someone unplugs the cold kettle")]
    fn unplug(&self) -> Option<Self> {
        require!(self.phase == Phase::Cold);
        Some(Self { phase: Phase::Off })
    }

    #[end("A kettle that is hot or off is done")]
    fn stopped(&self) -> bool {
        self.phase == Phase::Hot || self.phase == Phase::Off
    }

    #[eventually("Every run comes to hot water")]
    fn boils(&self) -> bool {
        self.phase == Phase::Hot
    }

    #[eventually("Every run comes to a kettle that is hot or off")]
    fn settles(&self) -> bool {
        self.phase == Phase::Hot || self.phase == Phase::Off
    }

    #[witness("The water starts cold")]
    fn starts_cold(&self) -> Option<bool> {
        Some(self.phase == Phase::Cold)
    }

    #[witness("One heat makes the water hot")]
    fn hot_at_once(&self) -> Option<bool> {
        let after = self.heat()?;
        Some(after.phase == Phase::Hot)
    }
}
