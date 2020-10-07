use super::consts::{OPAQUE_VOXEL, TRANSPARENT_VOXEL};
use super::geom::ChunkKey;
use super::geom::PointCloud;
use super::VoxelReg;

use std::collections::HashMap;
use std::marker::Send;

use flamer::flame;

pub trait WorldType: Send + Sync {
    fn gen_chunk(&self, key: &ChunkKey, reg: &VoxelReg) -> Vec<u64>;
    fn world_type(&self) -> &'static str;
}

pub struct FlatWorldType {
    pub chunk_size: usize,
}

impl WorldType for FlatWorldType {
    #[flame("FlatWorldType")]
    fn gen_chunk(&self, key: &ChunkKey, reg: &VoxelReg) -> Vec<u64> {
        let transparent_voxel = reg.key_from_string_id(TRANSPARENT_VOXEL);
        let mut c = vec![transparent_voxel; self.chunk_size * self.chunk_size * self.chunk_size];
        let voxel_type = reg.key_from_string_id(OPAQUE_VOXEL);
        if key.y == 0 {
            for x in 0..self.chunk_size {
                for z in 0..self.chunk_size {
                    let idx = crate::geom::calc_idx(x, 0, z, self.chunk_size);
                    c[idx] = voxel_type;
                }
            }
        } else if key.y < 0 {
            for y in 0..self.chunk_size {
                for x in 0..self.chunk_size {
                    for z in 0..self.chunk_size {
                        let idx = crate::geom::calc_idx(x, y, z, self.chunk_size);
                        c[idx] = voxel_type;
                    }
                }
            }
        }
        c
    }

    #[flame("FlatWorldType")]
    fn world_type(&self) -> &'static str {
        "FlatWorldType"
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct World {
    pub pc: PointCloud,
    pub world_type: u64,
    pub active: bool,
    chunk_size: usize,
}

impl World {
    #[flame("World")]
    pub fn new(active: bool, chunk_size: usize, world_type: u64) -> World {
        World {
            pc: PointCloud::new(chunk_size),
            world_type,
            active,
            chunk_size,
        }
    }

    #[flame("World")]
    pub fn chunk_size(&self) -> usize {
        self.chunk_size
    }
}

pub struct WorldTypeRegistry {
    pub world_type_reg: HashMap<u64, Box<dyn WorldType>>,
    next_type_key: u64,
}

impl WorldTypeRegistry {
    #[flame("WorldTypeRegistry")]
    pub fn new() -> WorldTypeRegistry {
        WorldTypeRegistry {
            world_type_reg: HashMap::new(),
            next_type_key: 1,
        }
    }

    #[flame("WorldTypeRegistry")]
    pub fn register_world_type(&mut self, world_type: Box<dyn WorldType>) -> u64 {
        let id = self.get_next_type_key();
        self.world_type_reg.insert(id, world_type);
        id
    }

    #[flame("WorldTypeRegistry")]
    fn get_next_type_key(&mut self) -> u64 {
        let out = self.next_type_key;
        self.next_type_key += 1;
        out
    }
}

pub struct WorldRegistry {
    world_reg: HashMap<u64, World>,
    next_world_key: u64,
}

impl WorldRegistry {
    #[flame("WorldRegistry")]
    pub fn new() -> WorldRegistry {
        WorldRegistry {
            world_reg: HashMap::new(),
            next_world_key: 1,
        }
    }

    #[flame("WorldRegistry")]
    pub fn new_world(&mut self, world: World) -> u64 {
        let id = self.get_next_world_key();
        self.world_reg.insert(id, world);
        id
    }

    #[flame("WorldRegistry")]
    fn get_next_world_key(&mut self) -> u64 {
        let out = self.next_world_key;
        self.next_world_key += 1;
        out
    }

    #[flame("WorldRegistry")]
    pub fn world(&self, id: &u64) -> &World {
        self.world_reg.get(id).unwrap()
    }
}
