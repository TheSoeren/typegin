use getset::Getters;

use crate::world::object;

/// The player's inventory.
#[derive(Debug, Default, Getters)]
#[getset(get = "pub(crate)")]
pub(crate) struct Player {
    objects: Vec<object::Object>,
}

impl Player {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Find a carried object by id.
    pub(crate) fn get_object(&self, id: &object::ObjectId) -> object::ObjectResolution {
        match self.objects.iter().find(|object| object.id == *id) {
            Some(object) => object::ObjectResolution::Found(object.id.clone()),
            None => object::ObjectResolution::NotFound,
        }
    }

    pub(crate) fn find_by_id(&self, id: &object::ObjectId) -> Option<&object::Object> {
        self.objects.iter().find(|object| object.id == *id)
    }

    /// Resolve `name` against the carried objects.
    pub(crate) fn find_object(&self, name: &str) -> object::ObjectResolution {
        object::Object::resolve_by_name(self.objects(), name)
    }

    /// Whether the player carries the object with `id`.
    pub(crate) fn holds(&self, id: &object::ObjectId) -> bool {
        self.objects.iter().any(|object| object.id == *id)
    }

    pub(crate) fn add_object(&mut self, object: object::Object) {
        self.objects.push(object);
    }

    pub(crate) fn remove_object(&mut self, id: &object::ObjectId) -> Option<object::Object> {
        let position = self.objects.iter().position(|object| object.id == *id);
        match position {
            Some(pos) => Some(self.objects.remove(pos)),
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::object_data::ObjectKind;
    use std::collections::HashMap;

    fn item(id: &str) -> object::Object {
        object::Object {
            id: object::ObjectId::new(id),
            primary_name: id.to_string(),
            aliases: vec![format!("{id}-alias")],
            kind: ObjectKind::Item,
            door: None,
            extra: HashMap::new(),
        }
    }

    #[test]
    fn new_and_default_start_empty() {
        assert!(Player::new().objects().is_empty());
        assert!(Player::default().objects().is_empty());
    }

    #[test]
    fn add_object_makes_it_held() {
        let mut player = Player::new();
        player.add_object(item("sword"));
        assert!(player.holds(&object::ObjectId::new("sword")));
        assert_eq!(player.objects().len(), 1);
    }

    #[test]
    fn get_object_reports_found_or_not_found() {
        let mut player = Player::new();
        player.add_object(item("sword"));
        assert_eq!(
            player.get_object(&object::ObjectId::new("sword")),
            object::ObjectResolution::Found(object::ObjectId::new("sword"))
        );
        assert_eq!(
            player.get_object(&object::ObjectId::new("shield")),
            object::ObjectResolution::NotFound
        );
    }

    #[test]
    fn find_by_id_returns_the_object_reference() {
        let mut player = Player::new();
        player.add_object(item("sword"));
        assert_eq!(
            player
                .find_by_id(&object::ObjectId::new("sword"))
                .map(|o| &o.id),
            Some(&object::ObjectId::new("sword"))
        );
        assert!(
            player
                .find_by_id(&object::ObjectId::new("shield"))
                .is_none()
        );
    }

    #[test]
    fn find_object_resolves_by_name_or_alias() {
        let mut player = Player::new();
        player.add_object(item("sword"));
        assert_eq!(
            player.find_object("sword"),
            object::ObjectResolution::Found(object::ObjectId::new("sword"))
        );
        assert_eq!(
            player.find_object("sword-alias"),
            object::ObjectResolution::Found(object::ObjectId::new("sword"))
        );
        assert_eq!(
            player.find_object("shield"),
            object::ObjectResolution::NotFound
        );
    }

    #[test]
    fn holds_is_false_before_adding_and_true_after() {
        let mut player = Player::new();
        assert!(!player.holds(&object::ObjectId::new("sword")));
        player.add_object(item("sword"));
        assert!(player.holds(&object::ObjectId::new("sword")));
    }

    #[test]
    fn remove_object_takes_it_out_of_the_inventory() {
        let mut player = Player::new();
        player.add_object(item("sword"));
        let removed = player.remove_object(&object::ObjectId::new("sword"));
        assert_eq!(removed.map(|o| o.id), Some(object::ObjectId::new("sword")));
        assert!(!player.holds(&object::ObjectId::new("sword")));
    }

    #[test]
    fn remove_object_not_held_returns_none() {
        let mut player = Player::new();
        assert!(
            player
                .remove_object(&object::ObjectId::new("sword"))
                .is_none()
        );
    }
}
