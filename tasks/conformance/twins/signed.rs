use earnest::model::{Model, model, require};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Model)]
struct Signed {
    #[one_of(-7, 7)]
    a: i8,
    #[one_of(-2, 2)]
    b: i8,
    #[range(-4, 4)]
    #[initial(0)]
    q: i8,
    #[range(-2, 2)]
    #[initial(0)]
    r: i8,
    #[initial(false)]
    done: bool,
}

#[model]
impl Signed {
    #[action("the pair is divided")]
    fn divide(&self) -> Option<Self> {
        require!(!self.done);
        Some(Self {
            q: self.a / self.b,
            r: self.a % self.b,
            done: true,
            ..self.clone()
        })
    }

    #[end("A divided pair is done")]
    fn over(&self) -> bool {
        self.done
    }

    #[rule("The quotient and the remainder give the number back")]
    fn rebuilt(&self) -> bool {
        !self.done || self.q * self.b + self.r == self.a
    }

    #[rule("The quotient drops its fraction")]
    fn truncated(&self) -> bool {
        !self.done || self.q == 3 || self.q == -3
    }

    #[rule("The remainder has the sign of the number")]
    fn signs(&self) -> bool {
        !self.done || self.r * self.a > 0
    }

    #[rule("The quotient is the floor")]
    fn floored(&self) -> bool {
        !self.done || self.q != -3
    }
}
