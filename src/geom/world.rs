use super::chunk;
use super::util;
use anyhow::Result;
use legion::{systems, Entity, IntoQuery, Read, SystemBuilder, Write};
use rayon::iter::ParallelIterator;
use std::collections::HashMap;

pub trait WorldType: Send + Sync {
    fn gen_chunk(
        &self,
        pos: &chunk::Position,
        reg: &crate::voxel_registry::VoxelReg,
        out: &mut arrayvec::ArrayVec<[u64; chunk::VOXELS_IN_CHUNK]>,
    ) -> Result<()>;
    fn world_type(&self) -> &'static str;
}

pub struct FlatWorldType {}

impl WorldType for FlatWorldType {
    #[optick_attr::profile]
    fn gen_chunk(
        &self,
        pos: &chunk::Position,
        reg: &crate::voxel_registry::VoxelReg,
        out: &mut arrayvec::ArrayVec<[u64; chunk::VOXELS_IN_CHUNK]>,
    ) -> Result<()> {
        let transparent_voxel = reg
            .key_from_string_id(crate::consts::TRANSPARENT_VOXEL)
            .unwrap();
        let opaque_voxel = reg.key_from_string_id(crate::consts::OPAQUE_VOXEL).unwrap();
        if pos.pos.y > 0 {
            *out = [transparent_voxel; chunk::VOXELS_IN_CHUNK].into();
        } else if pos.pos.y < 0 {
            *out = [opaque_voxel; chunk::VOXELS_IN_CHUNK].into();
        } else {
            *out = [transparent_voxel; chunk::VOXELS_IN_CHUNK].into();
            for idx in 0..crate::consts::CHUNK_SIZE_USIZE * crate::consts::CHUNK_SIZE_USIZE {
                out[idx] = opaque_voxel;
            }
        }
        Ok(())
    }
    fn world_type(&self) -> &'static str {
        "FlatWorldType"
    }
}

#[derive(Debug)]
pub struct World {
    pub world_type: u32,
    pub chunk_map: HashMap<chunk::Position, legion::Entity>,
}

pub struct Active {}

impl PartialEq for World {
    fn eq(&self, other: &Self) -> bool {
        self.world_type == other.world_type
    }
}

pub fn generate_chunks_system(schedule_builder: &mut systems::Builder) {
    schedule_builder.add_system(
        SystemBuilder::new("GenerateChunkSystem")
            .read_resource::<crate::voxel_registry::VoxelReg>()
            .read_resource::<TypeRegistry>()
            .with_query(<(
                Entity,
                Read<chunk::Position>,
                Write<chunk::Data>,
                Read<chunk::MarkedForGen>,
            )>::query())
            .with_query(<(Read<World>, Read<Active>)>::query())
            .build(|cmd, ecs, resources, queries| {
                optick::event!();
                let (chunk_query, world_query) = queries;
                let (voxreg, world_type_reg) = resources;
                let (mut chunk_ecs, world_ecs) = ecs.split_for_query(chunk_query);
                world_query.iter(&world_ecs).for_each(|(w, _)| {
                    let world_type: &Box<dyn WorldType> =
                        world_type_reg.world_type_reg.get(&w.world_type).unwrap();
                    let result = chunk_query
                        .par_iter_mut(&mut chunk_ecs)
                        .map(|(e, pos, data, _)| {
                            world_type
                                .gen_chunk(&pos, &voxreg, &mut data.voxels)
                                .unwrap();
                            (e, pos)
                        })
                        .collect::<Vec<_>>();

                    for (e, pos) in result {
                        cmd.remove_component::<chunk::MarkedForGen>(e.clone());
                        for dir in util::ALL_DIRECTIONS.iter() {
                            let n_pos = pos.neighbor(&dir).unwrap();
                            if w.chunk_map.contains_key(&n_pos) {
                                let entity = w.chunk_map.get(&n_pos).unwrap();
                                cmd.add_component(entity.clone(), chunk::UpdateRender {});
                            }
                        }
                    }
                });
            }),
    );
}

pub struct TypeRegistry {
    pub world_type_reg: HashMap<u32, Box<dyn WorldType>>,
    next_type_key: u32,
}

impl TypeRegistry {
    pub fn new() -> Self {
        Self {
            world_type_reg: HashMap::new(),
            next_type_key: 1,
        }
    }

    pub fn register_world_type(&mut self, world_type: Box<dyn WorldType>) -> u32 {
        let id = self.get_next_type_key();
        self.world_type_reg.insert(id, world_type);
        id
    }

    fn get_next_type_key(&mut self) -> u32 {
        let out = self.next_type_key;
        self.next_type_key += 1;
        out
    }
}
