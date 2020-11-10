use std::collections::HashMap;
use building_blocks::core::Point3;

use super::chunk;

pub type Id = u16;
pub type Position = Point3<i32>;

pub const OPAQUE_VOXEL: &str = "opaque";
pub const TRANSPARENT_VOXEL: &str = "transparent";
pub const VOXEL_SIZE: f32 = 2.0;

#[derive(Debug, Copy, Clone)]
pub struct Attributes {
    transparent: bool,
}

#[allow(clippy::must_use_candidate)]
pub fn calc_pos(pos: Position) -> glm::Vec3 {
    let offset = chunk::CHUNK_SIZE_F32;
    (glm::vec3(pos.x()  as f32, pos.y() as f32, pos.z() as f32) * VOXEL_SIZE) - glm::vec3(offset, offset, offset)
}

#[allow(clippy::must_use_candidate)]
pub fn rotation() -> glm::Qua<f32> {
    glm::quat_angle_axis(0.0, &glm::Vec3::z_axis().into_inner())
}

pub struct Registry {
    attribute_map: HashMap<Id, Attributes>,
    name_map: HashMap<Id, &'static str>,
    key_map: HashMap<&'static str, Id>,
    next_key: Id,
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

impl Registry {
    #[allow(clippy::must_use_candidate)]
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

    #[allow(clippy::must_use_candidate)]
    pub fn is_transparent(&self, key: Id) -> Option<bool> {
        let attr = self.attributes(key)?;
        Some(attr.transparent)
    }

    #[allow(clippy::must_use_candidate)]
    pub fn key_from_string_id(&self, string_id: &str) -> Option<Id> {
        self.key_map.get(string_id).copied()
    }
}