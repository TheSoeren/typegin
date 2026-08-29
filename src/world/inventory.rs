use super::item::Item;

#[derive(Debug)]
pub(crate) struct Inventory {
    pub(crate) items: Vec<Item>,
}
