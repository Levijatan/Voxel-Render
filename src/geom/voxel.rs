use std::collections::HashMap;

pub type Id = u16;

#[derive(Debug, Copy, Clone)]
pub struct Attributes {
    transparent: bool,
}

pub struct Registry {
    attribute_map: HashMap<Id, Attributes>,
    name_map: HashMap<Id, &'static str>,
    key_map: HashMap<&'static str, Id>,
    next_key: Id,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            attribute_map: HashMap::new(),
            name_map: HashMap::new(),
            key_map: HashMap::new(),
            next_key: 1,
        }
    }

    pub fn get_new_key(&mut self) -> Id {
        let key = self.next_key;
        self.next_key += 1;
        key
    }

    pub fn register_voxel_type(&mut self, string_id: &'static str, transparent: bool) -> Id {
        let key = self.get_new_key();
        self.attribute_map
            .entry(key)
            .or_insert(Attributes { transparent });
        self.name_map.entry(key).or_insert(string_id);
        self.key_map.entry(string_id).or_insert(key);
        key
    }

    fn attributes(&self, key: Id) -> Option<Attributes> {
        self.attribute_map.get(&key).copied()
    }

    pub fn is_transparent(&self, key: Id) -> Option<bool> {
        let attr = self.attributes(key)?;
        Some(attr.transparent)
    }

    pub fn key_from_string_id(&self, string_id: &str) -> Option<Id> {
        self.key_map.get(string_id).copied()
    }
}