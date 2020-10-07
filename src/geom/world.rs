use super::chunk::VOXELS_IN_CHUNK;
use legion::*;
use std::collections::HashMap;

pub trait WorldType: Send + Sync {
    fn gen_chunk(
        &self,
        pos: &super::chunk::Position,
        reg: &crate::voxel_registry::VoxelReg,
    ) -> arrayvec::ArrayVec<[u64; VOXELS_IN_CHUNK]>;
    fn world_type(&self) -> &'static str;
}

pub struct FlatWorldType {}

impl WorldType for FlatWorldType {
    fn gen_chunk(
        &self,
        pos: &super::chunk::Position,
        reg: &crate::voxel_registry::VoxelReg,
    ) -> arrayvec::ArrayVec<[u64; VOXELS_IN_CHUNK]> {
        let transparent_voxel = reg.key_from_string_id(crate::consts::TRANSPARENT_VOXEL);
        let opaque_voxel = reg.key_from_string_id(crate::consts::OPAQUE_VOXEL);
        if pos.y > 0 {
            [transparent_voxel; VOXELS_IN_CHUNK].into()
        } else if pos.y < 0 {
            [opaque_voxel; VOXELS_IN_CHUNK].into()
        } else {
            let mut out = [transparent_voxel; VOXELS_IN_CHUNK];
            for x in 0..crate::consts::CHUNK_SIZE {
                for z in 0..crate::consts::CHUNK_SIZE {
                    let idx = super::util::calc_idx(x, 0, z);
                    out[idx] = opaque_voxel;
                }
            }
            out.into()
        }
    }
    fn world_type(&self) -> &'static str {
        "FlatWorldType"
    }
}

#[derive(Clone, Debug)]
pub struct World {
    pub active: bool,
    pub world_type: u32,
    pub chunk_map: dashmap::DashMap<super::chunk::Position, legion::Entity>,
}

impl PartialEq for World {
    fn eq(&self, other: &Self) -> bool {
        self.world_type == other.world_type
    }
}

impl World {
    pub fn add_chunk(&mut self, pos: super::chunk::Position, ent: legion::Entity) {
        self.chunk_map.insert(pos, ent);
    }

    pub fn gen_chunk(
        &mut self,
        pos: super::chunk::Position,
        cmd: &mut systems::CommandBuffer,
        world_type_reg: &WorldTypeRegistry,
        vox_reg: &crate::voxel_registry::VoxelReg,
    ) -> legion::Entity {
        let world_type = world_type_reg.world_type_reg.get(&self.world_type).unwrap();
        let voxels = world_type.gen_chunk(&pos, &vox_reg);
        let c = super::chunk::new_at_pos(cmd, pos, voxels);
        self.add_chunk(pos, c);
        c
    }
}

pub fn generate_chunks_system(schedule_builder: &mut systems::Builder) {
    use crate::consts::RENDER_RADIUS;
    schedule_builder.add_system(
        SystemBuilder::new("GenerateChunkSystem")
            .read_resource::<crate::voxel_registry::VoxelReg>()
            .read_resource::<WorldTypeRegistry>()
            .with_query(<(Entity, Write<World>)>::query())
            .build(move |cmd, ecs, resources, queries| {
                let (voxreg, world_type_reg) = resources;
                for (e, w) in queries.iter_mut(ecs) {
                    if w.active {
                        let world_type = world_type_reg.world_type_reg.get(&w.world_type).unwrap();

                        for x in -RENDER_RADIUS..=RENDER_RADIUS {
                            for y in -RENDER_RADIUS..=RENDER_RADIUS {
                                for z in -RENDER_RADIUS..=RENDER_RADIUS {
                                    let pos = super::chunk::Position { x, y, z };
                                    if !w.chunk_map.contains_key(&pos) {
                                        let voxels = world_type.gen_chunk(&pos, &voxreg);
                                        let c = super::chunk::new_at_pos(cmd, pos, voxels);
                                        w.chunk_map.insert(pos, c);
                                        return;
                                    }
                                }
                            }
                        }
                    }
                }
            }),
    );
}

pub struct WorldTypeRegistry {
    pub world_type_reg: HashMap<u32, Box<dyn WorldType>>,
    next_type_key: u32,
}

impl WorldTypeRegistry {
    pub fn new() -> WorldTypeRegistry {
        WorldTypeRegistry {
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
