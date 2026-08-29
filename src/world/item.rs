use crate::world::EntityId;

#[derive(Debug, Clone)]
pub(crate) struct Item {
    pub(crate) id: EntityId,
    pub(crate) primary_name: String,
    pub(crate) aliases: Vec<String>,
}

impl Item {
    pub fn has_name(&self, name: &str) -> bool {
        self.primary_name == name || self.aliases.iter().any(|a| a == name)
    }
}
