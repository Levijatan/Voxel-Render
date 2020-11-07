use super::chunk;
use super::voxel;

use anyhow::{anyhow, Result};
use std::{collections::HashMap, cmp::Ordering};
use building_blocks::{
    storage::{
        Array3,
        chunk_map::ChunkMap3,
        FastLz4
    }
};

pub type TypeId = u32;
pub type Id = legion::Entity;
pub type Map<M> =  ChunkMap3<voxel::Id, M>;

pub struct Active {}

pub fn create<M>(world_type: TypeId, chunk_meta: M) -> (Map<M>, TypeId)
    where M: Clone + Copy
{
    (Map::new(chunk::CHUNK_SHAPE, 1, chunk_meta, FastLz4 { level: 10 }), world_type)
}

pub fn create_active<M>(world_type: TypeId, chunk_meta: M) -> (Map<M>, TypeId, Active) 
    where M: Clone + Copy
{
    let (map, meta) = create(world_type, chunk_meta);
    (map, meta, Active{})
}

pub struct TypeRegistry<T> {
    world_type_reg: HashMap<u32, Box<dyn TypeTrait<T>>>,
    next_type_key: u32,
}

impl<T> TypeRegistry<T> {
    pub fn new() -> Self {
        Self {
            world_type_reg: HashMap::new(),
            next_type_key: 1,
        }
    }

    pub fn register_world_type(&mut self, world_type: Box<dyn TypeTrait<T>>) -> u32 {
        let id = self.get_next_type_key();
        self.world_type_reg.insert(id, world_type);
        id
    }

    fn get_next_type_key(&mut self) -> u32 {
        let out = self.next_type_key;
        self.next_type_key += 1;
        out
    }

    pub fn world_type(&self, world_id: u32) -> Result<&dyn TypeTrait<T>> {
        if let Some(world_type) = self.world_type_reg.get(&world_id) {
            Ok(&**world_type)
        } else {
            Err(anyhow!("{:?} is not a valid world type id", world_id))
        }
    }
}

pub trait TypeTrait<T>: Send + Sync 
    where T: Copy + Clone
{
    fn gen_chunk(
        &self,
        pos: &chunk::Position,
        extent: &chunk::Extent,
        vox_reg: &voxel::Registry,
    ) -> chunk::CType<T>;
    fn world_type(&self) -> &'static str;
}

pub struct FlatWorldType {}

impl<T> TypeTrait<T> for FlatWorldType 
    where T: Copy + Clone
{
    fn gen_chunk(
        &self,
        pos: &chunk::Position,
        extent: &chunk::Extent,
        vox_reg: &voxel::Registry,
    ) -> chunk::CType<T> {
        let transparent_voxel = vox_reg.key_from_string_id(voxel::TRANSPARENT_VOXEL).unwrap();
        let opaque_voxel = vox_reg.key_from_string_id(voxel::OPAQUE_VOXEL).unwrap();
        let mut meta = chunk::Meta::<T>::new();
        let map = match pos.y().partial_cmp(&0).unwrap() {
            Ordering::Greater => {
                meta.set_visibilty(true);
                meta.set_transparency(63);
                Array3::fill(*extent, transparent_voxel)
            },
            Ordering::Less => Array3::fill(*extent, opaque_voxel),
            Ordering::Equal => {
                meta.set_visibilty(true);
                meta.set_transparency(31);
                meta.voxel_set_range(0..(chunk::CHUNK_SIZE_USIZE*chunk::CHUNK_SIZE_USIZE), true);
                Array3::fill_with(*extent, |pos| {
                    if pos.y() == 0 {
                        opaque_voxel
                    } else {
                        transparent_voxel
                    }
                })
            },
        };
        chunk::CType{metadata: meta, map}
    }
    fn world_type(&self) -> &'static str {
        "FlatWorldType"
    }
}
