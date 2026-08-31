use super::direction::Direction;

#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    Look,
    Go(Direction),
    Examine(String),
    Take(String),
    Drop(String),
    Use {
        item: String,
        target: Option<String>,
    },
    Unknown(String),
}
