use super::item::Item;

#[derive(Debug)]
pub(crate) struct Room {
    pub(crate) id: i32,
    pub(crate) name: String,
    pub(crate) items: Vec<Item>,
    pub(crate) hidden_items: Vec<Item>,
}
