use getset::Getters;

use crate::world::object;

#[derive(Debug, Getters)]
#[getset(get = "pub(crate)")]
pub(crate) struct Player {
    objects: Vec<object::Object>,
}

impl Player {
    pub(crate) fn new() -> Self {
        Player {
            objects: Vec::new(),
        }
    }

    pub(crate) fn get_object(&self, id: object::ObjectId) -> object::ObjectResolution {
        match self.objects.iter().find(|object| object.id == id) {
            Some(object) => object::ObjectResolution::Found(object.id),
            None => object::ObjectResolution::NotFound,
        }
    }

    pub(crate) fn find_by_id(&self, id: object::ObjectId) -> Option<&object::Object> {
        self.objects.iter().find(|object| object.id == id)
    }

    pub(crate) fn find_object(&self, name: &str) -> object::ObjectResolution {
        object::Object::resolve_by_name(self.objects(), name)
    }

    pub(crate) fn holds(&self, id: object::ObjectId) -> bool {
        self.objects.iter().any(|object| object.id == id)
    }

    pub(crate) fn add_object(&mut self, object: object::Object) {
        self.objects.push(object);
    }

    pub(crate) fn remove_object(&mut self, id: object::ObjectId) -> Option<object::Object> {
        let position = self.objects.iter().position(|object| object.id == id);
        match position {
            Some(pos) => Some(self.objects.remove(pos)),
            None => None,
        }
    }
}

impl Default for Player {
    fn default() -> Self {
        Self::new()
    }
}
