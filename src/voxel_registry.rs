use anyhow::{anyhow, Result};
use std::collections::HashMap;

#[derive(Debug, Copy, Clone)]
pub struct VoxelAttributes {
    pub transparent: bool,
}

pub struct VoxelReg {
    attribute_map: HashMap<u64, VoxelAttributes>,
    name_map: HashMap<u64, &'static str>,
    key_map: HashMap<&'static str, u64>,
    next_key: u64,
}

impl VoxelReg {
    #[optick_attr::profile]
    pub fn new() -> VoxelReg {
        VoxelReg {
            attribute_map: HashMap::new(),
            name_map: HashMap::new(),
            key_map: HashMap::new(),
            next_key: 1,
        }
    }

    #[optick_attr::profile]
    pub fn get_new_key(&mut self) -> u64 {
        let key = self.next_key;
        self.next_key += 1;
        key
    }

    #[optick_attr::profile]
    pub fn register_voxel_type(&mut self, string_id: &'static str, transparent: bool) -> u64 {
        let key = self.get_new_key();
        self.attribute_map
            .entry(key)
            .or_insert(VoxelAttributes { transparent });
        self.name_map.entry(key).or_insert(string_id);
        self.key_map.entry(string_id).or_insert(key);
        key
    }

    #[optick_attr::profile]
    pub fn voxel_attributes(&self, key: &u64) -> VoxelAttributes {
        if *key == crate::consts::INVALID_VOXEL_ID {
            VoxelAttributes { transparent: true }
        } else {
            *self.attribute_map.get(key).unwrap()
        }
    }

    #[optick_attr::profile]
    pub fn is_transparent(&self, key: &u64) -> Result<bool> {
        if self.voxel_attributes(key).transparent {
            Ok(true)
        } else {
            Err(anyhow!("Solid"))
        }
    }

    #[optick_attr::profile]
    pub fn key_from_string_id(&self, string_id: &str) -> Option<u64> {
        if let Some(key) = self.key_map.get(string_id) {
            Some(*key)
        } else {
            None
        }
    }
}
