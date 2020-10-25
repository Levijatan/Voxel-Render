use std::collections::HashMap;

#[derive(Debug, Copy, Clone)]
pub struct Attributes {
    transparent: bool,
}

pub struct Registry {
    attribute_map: HashMap<u64, Attributes>,
    name_map: HashMap<u64, &'static str>,
    key_map: HashMap<&'static str, u64>,
    next_key: u64,
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

    pub fn get_new_key(&mut self) -> u64 {
        let key = self.next_key;
        self.next_key += 1;
        key
    }

    pub fn register_voxel_type(&mut self, string_id: &'static str, transparent: bool) -> u64 {
        let key = self.get_new_key();
        self.attribute_map
            .entry(key)
            .or_insert(Attributes { transparent });
        self.name_map.entry(key).or_insert(string_id);
        self.key_map.entry(string_id).or_insert(key);
        key
    }

    fn attributes(&self, key: u64) -> Attributes {
        if key == crate::consts::INVALID_VOXEL_ID {
            Attributes { transparent: true }
        } else {
            *self.attribute_map.get(&key).unwrap()
        }
    }

    pub fn is_transparent(&self, key: u64) -> bool {
        self.attributes(key).transparent
    }

    pub fn key_from_string_id(&self, string_id: &str) -> Option<u64> {
        self.key_map.get(string_id).copied()
    }
}
