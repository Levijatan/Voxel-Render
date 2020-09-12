use std::collections::HashMap;
use std::string::String;

struct Entry {
    string_id: &'static str,
    transparent: bool,
}

pub struct VoxelReg {
    reg: HashMap<u64, Entry>,
    next_key: u64,
}

impl VoxelReg {
    pub fn new() -> VoxelReg {
        VoxelReg {
            reg: HashMap::new(),
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
        self.reg.entry(key).or_insert(Entry {
            string_id,
            transparent,
        });
        key
    }

    pub fn voxel_attributes(&self, key: &u64) -> bool {
        self.reg.get(key).unwrap().transparent
    }

    pub fn key_from_string_id(&self, string_id: String) -> u64 {
        for (key, val) in self.reg.iter() {
            if val.string_id == string_id {
                return *key;
            }
        }
        0
    }
}
