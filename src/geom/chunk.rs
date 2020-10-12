use legion::*;
use rayon::prelude::*;

use crate::consts::*;

pub const VOXELS_IN_CHUNK: usize = CHUNK_SIZE_USIZE * CHUNK_SIZE_USIZE * CHUNK_SIZE_USIZE;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Position {
    pub x: i64,
    pub y: i64,
    pub z: i64,
    pub world_id: Entity,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Data {
    voxels: arrayvec::ArrayVec<[u64; VOXELS_IN_CHUNK]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RenderData {
    pub render: Vec<crate::render::state::Instance>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Render {
    pub culled: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UpdateRender {}

#[derive(Clone, Debug, PartialEq)]
pub struct Visible {}

#[derive(Clone, Debug, PartialEq)]
pub struct MarkedForGen {}

#[derive(Clone, Debug, PartialEq)]
pub struct Transparent {
    west: bool,
    east: bool,
    up: bool,
    down: bool,
    north: bool,
    south: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UpdateTransparent {}

impl Transparent {
    pub fn is_transparent(&self, norm_id: &super::util::Direction) -> bool {
        use super::util::Direction::*;
        let dir = super::util::reverse_direction(norm_id);
        match dir {
            East => self.east,
            West => self.west,
            Up => self.up,
            Down => self.down,
            North => self.north,
            South => self.south,
        }
    }
}

pub fn new_at_pos(
    cmd: &mut systems::CommandBuffer,
    entity: Entity,
    voxels: arrayvec::ArrayVec<[u64; VOXELS_IN_CHUNK]>,
) {
    let d = Data { voxels };
    let rd = RenderData { render: Vec::new() };
    let t = Transparent {
        north: false,
        south: false,
        east: false,
        west: false,
        up: false,
        down: false,
    };
    let r = Render { culled: false };
    let ut = UpdateTransparent {};
    let ur = UpdateRender {};
    cmd.add_component(entity, d);
    cmd.add_component(entity, rd);
    cmd.add_component(entity, t);
    cmd.add_component(entity, ur);
    cmd.add_component(entity, ut);
    cmd.add_component(entity, r);
}

pub fn voxel_in(pos: &glm::Vec3) -> bool {
    let size = CHUNK_SIZE_F32;
    !(pos.x >= size || pos.x < 0.0)
        && !(pos.y >= size || pos.y < 0.0)
        && !(pos.z >= size || pos.z < 0.0)
}

fn is_voxel_visible(
    pos: &glm::Vec3,
    voxreg: &crate::voxel_registry::VoxelReg,
    data: &Data,
) -> Result<bool, String> {
    let n_idx = super::util::calc_idx_pos(&pos);
    if voxreg.is_transparent(&data.voxels[n_idx]) {
        Ok(true)
    } else {
        Err("Not Visible".into())
    }
}

fn is_voxel_visible_neighbor_chunk(
    n: &super::util::Direction,
    pos: &Position,
    ecs: &world::SubWorld,
) -> Result<bool, String> {
    let entry = ecs.entry_ref(pos.world_id).unwrap();
    if let Ok(world) = entry.get_component::<super::world::World>() {
        let norm = super::util::normals_i64(n);
        let mut n_pos = pos.clone();
        n_pos.x += norm.x;
        n_pos.y += norm.y;
        n_pos.z += norm.z;
        if let Some(chunk_map_entry) = world.chunk_map.get(&n_pos) {
            let n_chunk_id = chunk_map_entry.value();
            let n_chunk = ecs.entry_ref(n_chunk_id.clone()).unwrap();
            let t = n_chunk.get_component::<Transparent>().unwrap();
            if t.is_transparent(n) {
                Ok(true)
            } else {
                Err("That was solid huh".into())
            }
        } else {
            Err("No Chunk exists here".into())
        }
    } else {
        Err("No world! HOW?".into())
    }
}

pub fn update_transparent(schedule_builder: &mut systems::Builder) {
    schedule_builder.add_system(
        SystemBuilder::new("UpdatingTransparentcy")
            .with_query(
                <(
                    Entity,
                    Read<Data>,
                    Write<Transparent>,
                    Read<UpdateTransparent>,
                )>::query()
                .filter(!component::<MarkedForGen>()),
            )
            .read_resource::<crate::voxel_registry::VoxelReg>()
            .build(move |cmd, ecs, res, query| {
                let voxreg: &crate::voxel_registry::VoxelReg = res;
                let entities = query
                    .par_iter_mut(ecs)
                    .map(|(e, voxel_data, transparent, _)| {
                        let west = (0..CHUNK_SIZE_USIZE)
                            .into_par_iter()
                            .flat_map(|x| {
                                (0..CHUNK_SIZE_USIZE).into_par_iter().map(move |z| {
                                    let idx = super::util::calc_idx(x, 0, z);
                                    if voxreg.is_transparent(voxel_data.voxels.get(idx).unwrap()) {
                                        Ok(true)
                                    } else {
                                        Err(false)
                                    }
                                })
                            })
                            .filter_map(Result::ok)
                            .collect::<Vec<_>>();

                        transparent.west = !west.is_empty();

                        let east = (0..CHUNK_SIZE_USIZE)
                            .into_par_iter()
                            .flat_map(|x| {
                                (0..CHUNK_SIZE_USIZE).into_par_iter().map(move |z| {
                                    let idx = super::util::calc_idx(x, CHUNK_SIZE_USIZE - 1, z);
                                    if voxreg.is_transparent(voxel_data.voxels.get(idx).unwrap()) {
                                        Ok(true)
                                    } else {
                                        Err(false)
                                    }
                                })
                            })
                            .filter_map(Result::ok)
                            .collect::<Vec<_>>();

                        transparent.east = !east.is_empty();

                        let south = (0..CHUNK_SIZE_USIZE)
                            .into_par_iter()
                            .flat_map(|x| {
                                (0..CHUNK_SIZE_USIZE).into_par_iter().map(move |y| {
                                    let idx = super::util::calc_idx(x, y, 0);
                                    if voxreg.is_transparent(voxel_data.voxels.get(idx).unwrap()) {
                                        Ok(true)
                                    } else {
                                        Err(false)
                                    }
                                })
                            })
                            .filter_map(Result::ok)
                            .collect::<Vec<_>>();

                        transparent.south = !south.is_empty();

                        let north = (0..CHUNK_SIZE_USIZE)
                            .into_par_iter()
                            .flat_map(|x| {
                                (0..CHUNK_SIZE_USIZE).into_par_iter().map(move |y| {
                                    let idx = super::util::calc_idx(x, y, CHUNK_SIZE_USIZE - 1);
                                    if voxreg.is_transparent(voxel_data.voxels.get(idx).unwrap()) {
                                        Ok(true)
                                    } else {
                                        Err(false)
                                    }
                                })
                            })
                            .filter_map(Result::ok)
                            .collect::<Vec<_>>();

                        transparent.north = !north.is_empty();

                        let down = (0..CHUNK_SIZE_USIZE)
                            .into_par_iter()
                            .flat_map(|y| {
                                (0..CHUNK_SIZE_USIZE).into_par_iter().map(move |z| {
                                    let idx = super::util::calc_idx(0, y, z);
                                    if voxreg.is_transparent(voxel_data.voxels.get(idx).unwrap()) {
                                        Ok(true)
                                    } else {
                                        Err(false)
                                    }
                                })
                            })
                            .filter_map(Result::ok)
                            .collect::<Vec<_>>();

                        transparent.down = !down.is_empty();

                        let up = (0..CHUNK_SIZE_USIZE)
                            .into_par_iter()
                            .flat_map(|y| {
                                (0..CHUNK_SIZE_USIZE).into_par_iter().map(move |z| {
                                    let idx = super::util::calc_idx(CHUNK_SIZE_USIZE - 1, y, z);
                                    if voxreg.is_transparent(voxel_data.voxels.get(idx).unwrap()) {
                                        Ok(true)
                                    } else {
                                        Err(false)
                                    }
                                })
                            })
                            .filter_map(Result::ok)
                            .collect::<Vec<_>>();

                        transparent.up = !up.is_empty();
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
            .with_query(
                <(Entity, Read<Position>, Write<RenderData>)>::query()
                    .filter(!component::<MarkedForGen>()),
            )
            .with_query(<Read<super::world::World>>::query())
            .with_query(<Read<Transparent>>::query())
            .build(move |cmd, ecs, _, (query, _, _)| {
                let (mut left, right) = ecs.split_for_query(query);
                let visible = query
                    .par_iter_mut(&mut left)
                    .map(move |(entity, pos, _)| {
                        let entry = right.entry_ref(pos.world_id).unwrap();
                        let world = entry.get_component::<super::world::World>().unwrap();
                        let visible = super::util::ALL_DIRECTIONS
                            .into_par_iter()
                            .map(|n| {
                                let norm = super::util::normals_i64(n);
                                let mut n_pos = pos.clone();
                                n_pos.x += norm.x;
                                n_pos.y += norm.y;
                                n_pos.z += norm.z;
                                if let Some(chunk_map_entry) = world.chunk_map.get(&n_pos) {
                                    let n_chunk_id = chunk_map_entry.value();
                                    if let Ok(n_chunk) = right.entry_ref(n_chunk_id.clone()) {
                                        let t = n_chunk.get_component::<Transparent>().unwrap();
                                        if t.is_transparent(n) {
                                            Ok(true)
                                        } else {
                                            Err("Not transparent huh")
                                        }
                                    } else {
                                        Ok(true)
                                    }
                                } else {
                                    Ok(true)
                                }
                            })
                            .filter_map(Result::ok)
                            .collect::<Vec<_>>();
                        if visible.is_empty() {
                            Err(entity)
                        } else {
                            Ok(entity)
                        }
                    })
                    .collect::<Vec<_>>();
                for entity in visible {
                    if let Ok(e) = entity {
                        cmd.add_component(e.clone(), Visible {});
                    }
                    if let Err(e) = entity {
                        cmd.remove_component::<Visible>(e.clone());
                    }
                }
            }),
    );
}

pub fn update_voxel_render_system(schedule_builder: &mut systems::Builder) {
    schedule_builder.add_system(
        SystemBuilder::new("UpdateVoxelRender")
            .with_query(
                <(
                    Entity,
                    Read<Position>,
                    Read<Data>,
                    Write<RenderData>,
                    Read<UpdateRender>,
                )>::query()
                .filter(!component::<MarkedForGen>()),
            )
            .with_query(<Read<super::world::World>>::query())
            .with_query(<(Read<Transparent>, Read<Data>)>::query())
            .read_resource::<crate::voxel_registry::VoxelReg>()
            .build(move |cmd, ecs, res, queries| {
                let voxreg = res;
                let (query, _, _) = queries;
                let (mut left, right) = ecs.split_for_query(query);
                let remove = query
                    .par_iter_mut(&mut left)
                    .map(move |(e, pos, data, render, _)| {
                        render.render = (0..VOXELS_IN_CHUNK)
                            .into_par_iter()
                            .map(|idx| {
                                if voxreg.is_transparent(&data.voxels[idx]) {
                                    Err("none")
                                } else {
                                    let voxel_pos = super::util::idx_to_pos(idx);

                                    let visible = super::util::ALL_DIRECTIONS
                                        .into_par_iter()
                                        .map(|n| {
                                            let n_pos = voxel_pos + super::util::normals_f32(n);
                                            if voxel_in(&n_pos) {
                                                is_voxel_visible(&n_pos, voxreg, &data)
                                            } else {
                                                is_voxel_visible_neighbor_chunk(n, &pos, &right)
                                            }
                                        })
                                        .filter_map(Result::ok)
                                        .collect::<Vec<_>>();

                                    if visible.is_empty() {
                                        Err("none")
                                    } else {
                                        let x = VOXEL_SIZE
                                            * (((pos.x * CHUNK_SIZE_I64) as f32 + voxel_pos.x)
                                                - CHUNK_SIZE_F32 / 2.0);
                                        let y = VOXEL_SIZE
                                            * (((pos.y * CHUNK_SIZE_I64) as f32 + voxel_pos.y)
                                                - CHUNK_SIZE_F32 / 2.0);
                                        let z = VOXEL_SIZE
                                            * (((pos.z * CHUNK_SIZE_I64) as f32 + voxel_pos.z)
                                                - CHUNK_SIZE_F32 / 2.0);
                                        let position = glm::vec3(x, y, z);
                                        let rotation = glm::quat_angle_axis(
                                            0.0,
                                            &glm::Vec3::z_axis().into_inner(),
                                        );
                                        Ok(crate::render::state::Instance { position, rotation })
                                    }
                                }
                            })
                            .filter_map(Result::ok)
                            .collect::<Vec<_>>();
                        e
                    })
                    .collect::<Vec<_>>();
                for e in remove {
                    cmd.remove_component::<UpdateRender>(e.clone());
                }
            }),
    );
}

#[system(par_for_each)]
pub fn frustum_cull(
    pos: &Position,
    ren: &mut Render,
    #[resource] frustum: &crate::render::camera::Frustum,
    #[resource] state: &crate::render::state::State,
) {
    let size = CHUNK_SIZE_I64 * VOXEL_SIZE as i64;
    let x = VOXEL_SIZE
        * (((pos.x * CHUNK_SIZE_I64) as f32 + (CHUNK_SIZE_F32 / 2.0)) - CHUNK_SIZE_F32 / 2.0);
    let y = VOXEL_SIZE
        * (((pos.y * CHUNK_SIZE_I64) as f32 + (CHUNK_SIZE_F32 / 2.0)) - CHUNK_SIZE_F32 / 2.0);
    let z = VOXEL_SIZE
        * (((pos.z * CHUNK_SIZE_I64) as f32 + (CHUNK_SIZE_F32 / 2.0)) - CHUNK_SIZE_F32 / 2.0);
    ren.culled = frustum.cube(
        &glm::vec3(x as f32, y as f32, z as f32),
        (size / 2) as f32,
        &state.camera.pos,
        &state.projection,
    ) == crate::render::camera::FrustumPos::Outside
}
