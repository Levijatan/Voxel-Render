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
            for x in 0..crate::consts::CHUNK_SIZE_USIZE {
                for z in 0..crate::consts::CHUNK_SIZE_USIZE {
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
    pub world_type: u32,
    pub chunk_map: dashmap::DashMap<super::chunk::Position, legion::Entity>,
}

pub struct Active {}

impl PartialEq for World {
    fn eq(&self, other: &Self) -> bool {
        self.world_type == other.world_type
    }
}

pub fn generate_chunks_system(schedule_builder: &mut systems::Builder) {
    use super::chunk::*;
    schedule_builder.add_system(
        SystemBuilder::new("GenerateChunkSystem")
            .read_resource::<crate::voxel_registry::VoxelReg>()
            .read_resource::<TypeRegistry>()
            .with_query(<(Read<World>, Read<Active>)>::query())
            .with_query(<(Entity, Read<Position>, Write<MarkedForGen>)>::query())
            .build(move |cmd, ecs, resources, queries| {
                let (world_query, chunk_query) = queries;
                let (voxreg, world_type_reg) = resources;
                let (mut world_ecs, mut chunk_ecs) = ecs.split_for_query(world_query);
                world_query.iter(&mut world_ecs).for_each(|(w, _)| {
                    let world_type = world_type_reg.world_type_reg.get(&w.world_type).unwrap();
                    chunk_query
                        .iter_mut(&mut chunk_ecs)
                        .for_each(|(e, pos, _)| {
                            let voxels = world_type.gen_chunk(&pos, &voxreg);
                            super::chunk::new_at_pos(cmd, e.clone(), voxels);
                            for n in super::util::ALL_DIRECTIONS.iter() {
                                let norm = super::util::normals_i64(n);
                                let mut n_pos = pos.clone();
                                n_pos.x += norm.x;
                                n_pos.y += norm.y;
                                n_pos.z += norm.z;
                                if w.chunk_map.contains_key(&n_pos) {
                                    let entity_ref = w.chunk_map.get(&n_pos).unwrap();
                                    let entity = entity_ref.value();
                                    cmd.add_component(entity.clone(), UpdateRender {});
                                }
                            }
                            cmd.remove_component::<MarkedForGen>(e.clone());
                        });
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
