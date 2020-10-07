use std::collections::HashMap;

use crate::consts::INVALID_VOXEL_ID;

use flamer::flame;

struct Entry {
    string_id: &'static str,
    attributes: VoxelAttributes,
}

#[derive(Debug, Copy, Clone)]
pub struct VoxelAttributes {
    pub transparent: bool,
}

pub struct VoxelReg {
    reg: HashMap<u64, Entry>,
    next_key: u64,
}

impl VoxelReg {
    #[flame]
    pub fn new() -> VoxelReg {
        VoxelReg {
            reg: HashMap::new(),
            next_key: 1,
        }
    }

    #[flame]
    pub fn get_new_key(&mut self) -> u64 {
        let key = self.next_key;
        self.next_key += 1;
        key
    }

    #[flame]
    pub fn register_voxel_type(&mut self, string_id: &'static str, transparent: bool) -> u64 {
        let key = self.get_new_key();
        self.reg.entry(key).or_insert(Entry {
            string_id,
            attributes: VoxelAttributes { transparent },
        });
        key
    }

    #[flame]
    pub fn voxel_attributes(&self, key: &u64) -> VoxelAttributes {
        if *key != INVALID_VOXEL_ID {
            return self.reg.get(key).unwrap().attributes;
        }
        VoxelAttributes { transparent: true }
    }

    #[flame]
    pub fn is_transparent(&self, key: &u64) -> bool {
        self.voxel_attributes(key).transparent
    }

    #[flame]
    pub fn key_from_string_id(&self, string_id: &str) -> u64 {
        for (key, val) in self.reg.iter() {
            if val.string_id == string_id {
                return *key;
            }
        }
        0
    }
}
