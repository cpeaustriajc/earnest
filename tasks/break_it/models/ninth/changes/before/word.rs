use earnest::model::{Model, model, require};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Model)]
struct Word {
    #[initial("a b")]
    #[one_of("a b", "a  b")]
    text: &'static str,
    #[initial(false)]
    done: bool,
}

#[model]
impl Word {
    #[action("the word is said")]
    fn say(&self) -> Option<Self> {
        require!(!self.done);
        Some(Self {
            done: true,
            ..self.clone()
        })
    }

    #[end("The word was said")]
    fn said(&self) -> bool {
        self.done
    }

    #[eventually("The word comes to have one space")]
    fn spaced(&self) -> bool {
        self.text == "a b"
    }

    #[setup("The word starts with one space")]
    fn single(&self) -> Self {
        Self {
            text: "a b",
            ..self.clone()
        }
    }
}
