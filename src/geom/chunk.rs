use anyhow::{anyhow, ensure, Result};
use bitvec::prelude::*;
use legion::{
    component, maybe_changed, systems, Entity, EntityStore, IntoQuery, Read, SystemBuilder, Write,
};
use rayon::iter::ParallelIterator;

use super::util;
use super::world;

pub const VOXELS_IN_CHUNK: usize = crate::consts::CHUNK_SIZE_USIZE
    * crate::consts::CHUNK_SIZE_USIZE
    * crate::consts::CHUNK_SIZE_USIZE;

#[derive(Clone, Copy, Debug, Hash)]
pub struct Position {
    pub pos: glm::TVec3<i32>,
    pub world_id: Entity,
}

impl Position {
    pub fn neighbor(&self, dir: &util::Direction) -> Result<Position> {
        let mut pos = self.clone();
        pos.pos += util::normals_i32(&dir);
        ensure!(
            pos != *self,
            "{:?} should not be the same as {:?}",
            pos,
            self
        );
        Ok(pos)
    }

    #[optick_attr::profile]
    pub fn get_f32_pos(&self) -> glm::Vec3 {
        let x = self.pos.x as f32;
        let y = self.pos.y as f32;
        let z = self.pos.z as f32;
        glm::vec3(x, y, z)
    }
}

impl PartialEq for Position {
    fn eq(&self, other: &Self) -> bool {
        self.pos.x == other.pos.x && self.pos.y == other.pos.y && self.pos.z == other.pos.z
    }
}

impl Eq for Position {}

#[derive(Clone, Debug, PartialEq)]
pub struct Data {
    pub voxels: arrayvec::ArrayVec<[u64; VOXELS_IN_CHUNK]>,
    pub render: Vec<crate::render::state::Instance>,
    pub transparent: BitArray<LocalBits, [u8; 1]>,
}

impl Data {
    #[optick_attr::profile]
    pub fn is_transparent(&self, norm_id: &util::Direction) -> Result<bool> {
        let dir = util::reverse_direction(&norm_id);
        if *self.transparent.get(dir as usize).unwrap() {
            Ok(true)
        } else {
            Err(anyhow!("Solid"))
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UpdateRender {}

#[derive(Clone, Debug, PartialEq)]
pub struct Visible {}

#[derive(Clone, Debug, PartialEq)]
pub struct MarkedForGen {}

#[derive(Clone, Debug, PartialEq)]
pub struct UpdateTransparent {}

pub fn new(
    pos: Position,
) -> (
    Position,
    Data,
    UpdateRender,
    UpdateTransparent,
    MarkedForGen,
) {
    let d = Data {
        voxels: arrayvec::ArrayVec::new(),
        render: Vec::new(),
        transparent: BitArray::new([0; 1]),
    };
    let ur = UpdateRender {};
    let ut = UpdateTransparent {};
    let mg = MarkedForGen {};
    (pos, d, ur, ut, mg)
}

pub fn voxel_in(pos: &glm::Vec3) -> bool {
    let size = crate::consts::CHUNK_SIZE_F32;
    !(pos.x >= size || pos.x < 0.0)
        && !(pos.y >= size || pos.y < 0.0)
        && !(pos.z >= size || pos.z < 0.0)
}

#[optick_attr::profile]
fn is_voxel_visible(
    pos: &glm::Vec3,
    voxreg: &crate::voxel_registry::VoxelReg,
    data: &Data,
) -> Result<bool> {
    let n_idx = util::calc_idx_pos(&pos)?;
    voxreg.is_transparent(&data.voxels[n_idx])
}

#[optick_attr::profile]
fn is_voxel_visible_neighbor_chunk(
    dir: &util::Direction,
    pos: &Position,
    ecs: &legion::world::SubWorld,
) -> Result<bool> {
    let entry = ecs.entry_ref(pos.world_id)?;
    if let Ok(world) = entry.get_component::<world::World>() {
        let n_pos = pos.neighbor(&dir)?;
        match world.chunk_map.get(&n_pos) {
            Some(n_chunk_id) => {
                let n_chunk = ecs.entry_ref(n_chunk_id.clone())?;
                let t = n_chunk.get_component::<Data>()?;
                t.is_transparent(&dir)
            }
            None => Err(anyhow!("No Chunk exists here")),
        }
    } else {
        Err(anyhow!("That went wrong"))
    }
}

fn is_vox_transparent(
    x: usize,
    y: usize,
    z: usize,
    data: &Data,
    voxreg: &crate::voxel_registry::VoxelReg,
) -> bool {
    let idx = util::calc_idx(x, y, z).unwrap();
    let vox = data.voxels.get(idx).unwrap();
    voxreg.is_transparent(vox).is_ok()
}

pub fn update_transparent(schedule_builder: &mut systems::Builder) {
    use crate::consts::CHUNK_SIZE_USIZE;
    use util::Direction;
    schedule_builder.add_system(
        SystemBuilder::new("UpdatingTransparentcy")
            .with_query(
                <(Entity, Write<Data>, Read<UpdateTransparent>)>::query()
                    .filter(maybe_changed::<Data>() & !component::<MarkedForGen>()),
            )
            .read_resource::<crate::voxel_registry::VoxelReg>()
            .build(move |cmd, ecs, res, query| {
                optick::event!();
                let voxreg: &crate::voxel_registry::VoxelReg = res;
                let entities = query
                    .par_iter_mut(ecs)
                    .map(move |(e, data, _)| {
                        let (mut west, mut east, mut up, mut down, mut north, mut south) =
                            (false, false, false, false, false, false);
                        'outer: for i in 0..CHUNK_SIZE_USIZE {
                            for j in 0..CHUNK_SIZE_USIZE {
                                if !west {
                                    west = is_vox_transparent(i, 0, j, &data, &voxreg);
                                }
                                if !east {
                                    east = is_vox_transparent(
                                        i,
                                        CHUNK_SIZE_USIZE - 1,
                                        j,
                                        &data,
                                        voxreg,
                                    )
                                }
                                if !south {
                                    south = is_vox_transparent(i, j, 0, &data, &voxreg)
                                }
                                if !north {
                                    north = is_vox_transparent(
                                        i,
                                        j,
                                        CHUNK_SIZE_USIZE - 1,
                                        &data,
                                        &voxreg,
                                    )
                                }
                                if !down {
                                    down = is_vox_transparent(0, i, j, &data, &voxreg);
                                }
                                if !up {
                                    up = is_vox_transparent(
                                        CHUNK_SIZE_USIZE - 1,
                                        i,
                                        j,
                                        &data,
                                        &voxreg,
                                    );
                                }
                                if west && east && south && north && up && down {
                                    break 'outer;
                                }
                            }
                        }

                        data.transparent.set(Direction::West as usize, west);
                        data.transparent.set(Direction::East as usize, east);
                        data.transparent.set(Direction::South as usize, south);
                        data.transparent.set(Direction::North as usize, north);
                        data.transparent.set(Direction::Down as usize, down);
                        data.transparent.set(Direction::Up as usize, up);

                        e.clone()
                    })
                    .collect::<Vec<_>>();
                for e in entities {
                    cmd.remove_component::<UpdateTransparent>(e);
                }
            }),
    );
}

pub fn culling(schedule_builder: &mut systems::Builder) {
    schedule_builder.add_system(
        SystemBuilder::new("CullingChunks")
            .with_query(<(Entity, Read<Position>)>::query().filter(
                maybe_changed::<Data>()
                    & !component::<MarkedForGen>()
                    & !component::<UpdateRender>(),
            ))
            .with_query(<Read<world::World>>::query())
            .with_query(<Read<Data>>::query())
            .build(move |cmd, ecs, _, (query, _, _)| {
                optick::event!();
                let (mut left, right) = ecs.split_for_query(query);
                let visible = query
                    .par_iter_mut(&mut left)
                    .map(move |(entity, pos)| {
                        let entry = right.entry_ref(pos.world_id).unwrap();
                        let world = entry.get_component::<world::World>().unwrap();
                        let visible = util::ALL_DIRECTIONS
                            .iter()
                            .map(|dir| {
                                let n_pos = pos.neighbor(&dir)?;
                                if let Some(n_chunk_id) = world.chunk_map.get(&n_pos) {
                                    if let Ok(n_chunk) = right.entry_ref(n_chunk_id.clone()) {
                                        let t = n_chunk.get_component::<Data>().unwrap();
                                        t.is_transparent(&dir)
                                    } else {
                                        Ok(true)
                                    }
                                } else {
                                    Ok(true)
                                }
                            })
                            .filter_map(Result::ok)
                            .collect::<Vec<_>>();
                        match visible.is_empty() {
                            true => Err(entity),
                            false => Ok(entity),
                        }
                    })
                    .collect::<Vec<_>>();
                for entity in visible {
                    match entity {
                        Ok(e) => cmd.add_component(e.clone(), Visible {}),
                        Err(e) => cmd.remove_component::<Visible>(e.clone()),
                    }
                }
            }),
    );
}

pub fn update_voxel_render_system(schedule_builder: &mut systems::Builder) {
    schedule_builder.add_system(
        SystemBuilder::new("UpdateVoxelRender")
            .with_query(
                <(Entity, Read<Position>, Write<Data>, Read<UpdateRender>)>::query()
                    .filter(maybe_changed::<Data>() & !component::<MarkedForGen>()),
            )
            .with_query(<Read<world::World>>::query())
            .with_query(<Read<Data>>::query())
            .read_resource::<crate::voxel_registry::VoxelReg>()
            .build(move |cmd, ecs, res, queries| {
                optick::event!();
                let voxreg = res;
                let (query, _, _) = queries;
                let (mut left, right) = ecs.split_for_query(query);
                let remove = query
                    .par_iter_mut(&mut left)
                    .map(move |(e, pos, data, _)| {
                        data.render.clear();
                        (0..VOXELS_IN_CHUNK).into_iter().for_each(|idx| {
                            if voxreg.is_transparent(&data.voxels[idx]).is_err() {
                                let voxel_pos = util::idx_to_pos(idx);

                                let visible = util::ALL_DIRECTIONS
                                    .iter()
                                    .map(|dir| {
                                        let n_pos = voxel_pos + util::normals_f32(&dir);
                                        if voxel_in(&n_pos) {
                                            is_voxel_visible(&n_pos, voxreg, &data)
                                        } else {
                                            is_voxel_visible_neighbor_chunk(&dir, &pos, &right)
                                        }
                                    })
                                    .filter_map(Result::ok)
                                    .collect::<Vec<_>>();

                                if !visible.is_empty() {
                                    let chunk_pos =
                                        pos.get_f32_pos() * crate::consts::CHUNK_SIZE_F32;

                                    let offset = crate::consts::CHUNK_SIZE_F32;

                                    let position: glm::Vec3 = (chunk_pos + voxel_pos)
                                        * crate::consts::VOXEL_SIZE
                                        - glm::vec3(offset, offset, offset);
                                    let rotation = glm::quat_angle_axis(
                                        0.0,
                                        &glm::Vec3::z_axis().into_inner(),
                                    );
                                    data.render.push(crate::render::state::Instance {
                                        position,
                                        rotation,
                                    });
                                }
                            }
                        });
                        e
                    })
                    .collect::<Vec<_>>();
                for e in remove {
                    cmd.remove_component::<UpdateRender>(e.clone());
                }
            }),
    );
}
