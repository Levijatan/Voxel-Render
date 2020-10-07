use legion::*;
use rayon::prelude::*;

use crate::consts::*;
use crate::render::util::in_render_radius;

pub const VOXELS_IN_CHUNK: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Position {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Data {
    voxels: arrayvec::ArrayVec<[u64; VOXELS_IN_CHUNK]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RenderData {
    pub render: Vec<crate::render::Instance>,
    update: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Render {}

#[derive(Clone, Debug, PartialEq)]
pub struct Transparent {
    update: bool,
    north: bool,
    east: bool,
    south: bool,
    west: bool,
    up: bool,
    down: bool,
}

pub fn new_empty(cmd: &mut systems::CommandBuffer, x: i64, y: i64, z: i64) -> legion::Entity {
    new(cmd, x, y, z, [0; VOXELS_IN_CHUNK].into())
}

pub fn new(
    cmd: &mut systems::CommandBuffer,
    x: i64,
    y: i64,
    z: i64,
    voxels: arrayvec::ArrayVec<[u64; VOXELS_IN_CHUNK]>,
) -> legion::Entity {
    let pos = Position { x, y, z };

    new_at_pos(cmd, pos, voxels)
}

pub fn new_at_pos(
    cmd: &mut systems::CommandBuffer,
    pos: Position,
    voxels: arrayvec::ArrayVec<[u64; VOXELS_IN_CHUNK]>,
) -> legion::Entity {
    let d = Data { voxels };
    let rd = RenderData {
        render: Vec::new(),
        update: true,
    };
    let t = Transparent {
        update: true,
        north: false,
        south: false,
        east: false,
        west: false,
        up: false,
        down: false,
    };
    cmd.push((pos, d, rd, t))
}

pub fn voxel_in_chunk(pos: &glm::Vec3) -> bool {
    let size = CHUNK_SIZE as f32;
    !(pos.x >= size || pos.x < 0.0)
        && !(pos.y >= size || pos.y < 0.0)
        && !(pos.z >= size || pos.z < 0.0)
}

#[system(par_for_each)]
#[filter(maybe_changed::<Data>())]
pub fn update_transparency(
    voxel_data: &Data,
    transparent: &mut Transparent,
    #[resource] voxreg: &crate::voxel_registry::VoxelReg,
) {
    if transparent.update {
        transparent.update = false;
        let west = (0..CHUNK_SIZE)
            .into_par_iter()
            .flat_map(|x| {
                (0..CHUNK_SIZE).into_par_iter().map(move |z| {
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

        let east = (0..CHUNK_SIZE)
            .into_par_iter()
            .flat_map(|x| {
                (0..CHUNK_SIZE).into_par_iter().map(move |z| {
                    let idx = super::util::calc_idx(x, CHUNK_SIZE - 1, z);
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

        let south = (0..CHUNK_SIZE)
            .into_par_iter()
            .flat_map(|x| {
                (0..CHUNK_SIZE).into_par_iter().map(move |y| {
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

        let north = (0..CHUNK_SIZE)
            .into_par_iter()
            .flat_map(|x| {
                (0..CHUNK_SIZE).into_par_iter().map(move |y| {
                    let idx = super::util::calc_idx(x, y, CHUNK_SIZE - 1);
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

        let down = (0..CHUNK_SIZE)
            .into_par_iter()
            .flat_map(|y| {
                (0..CHUNK_SIZE).into_par_iter().map(move |z| {
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

        let up = (0..CHUNK_SIZE)
            .into_par_iter()
            .flat_map(|y| {
                (0..CHUNK_SIZE).into_par_iter().map(move |z| {
                    let idx = super::util::calc_idx(CHUNK_SIZE - 1, y, z);
                    if voxreg.is_transparent(voxel_data.voxels.get(idx).unwrap()) {
                        Ok(true)
                    } else {
                        Err(false)
                    }
                })
            })
            .filter_map(Result::ok)
            .collect::<Vec<_>>();

        transparent.up = !up.is_empty()
    }
}

#[system(par_for_each)]
#[filter(maybe_changed::<Data>())]
pub fn update_chunk_render(
    pos: &Position,
    voxel_data: &Data,
    render_data: &mut RenderData,
    #[resource] voxreg: &crate::voxel_registry::VoxelReg,
) {
    if render_data.update {
        render_data.update = false;
        render_data.render = (0..VOXELS_IN_CHUNK)
            .into_par_iter()
            .map(|idx| {
                if voxreg.is_transparent(&voxel_data.voxels[idx]) {
                    Err("none")
                } else {
                    let voxel_pos = super::util::idx_to_pos(idx);

                    let visible = (0..6)
                        .into_par_iter()
                        .map(|n| {
                            let n_pos = voxel_pos + super::util::normals(n);
                            if voxel_in_chunk(&n_pos) {
                                let n_idx = super::util::calc_idx_pos(&n_pos);
                                if voxreg.is_transparent(&voxel_data.voxels[n_idx]) {
                                    Ok(true)
                                } else {
                                    Err(false)
                                }
                            } else {
                                Ok(true)
                            }
                        })
                        .filter_map(Result::ok)
                        .collect::<Vec<_>>();

                    if visible.is_empty() {
                        Err("none")
                    } else {
                        let x = VOXEL_SIZE
                            * (((pos.x * CHUNK_SIZE as i64) as f32 + voxel_pos.x)
                                - CHUNK_SIZE as f32 / 2.0);
                        let y = VOXEL_SIZE
                            * (((pos.y * CHUNK_SIZE as i64) as f32 + voxel_pos.y)
                                - CHUNK_SIZE as f32 / 2.0);
                        let z = VOXEL_SIZE
                            * (((pos.z * CHUNK_SIZE as i64) as f32 + voxel_pos.z)
                                - CHUNK_SIZE as f32 / 2.0);
                        let position = glm::vec3(x, y, z);
                        let rotation = glm::quat_angle_axis(0.0, &glm::Vec3::z_axis().into_inner());
                        Ok(crate::render::Instance { position, rotation })
                    }
                }
            })
            .filter_map(Result::ok)
            .collect::<Vec<_>>();
    }
}

pub fn should_render_system(schedule_builder: &mut systems::Builder) {
    schedule_builder.add_system(
        SystemBuilder::new("ShouldChunkBeRendered")
            .with_query(<(Entity, Read<Position>)>::query().filter(!component::<Render>()))
            .build(|cmd, ecs, _, query| {
                let mut render_components: Vec<(Entity, Render)> = query
                    .par_iter(ecs)
                    .map(|(e, pos)| {
                        if in_render_radius(pos) {
                            Ok((e.clone(), Render {}))
                        } else {
                            Err("Should not render")
                        }
                    })
                    .filter_map(Result::ok)
                    .collect();
                for (e, c) in render_components {
                    cmd.add_component(e, c);
                }
            }),
    );
}

pub fn should_not_render(schedule_builder: &mut systems::Builder) {
    schedule_builder.add_system(
        SystemBuilder::new("ShouldChunkNotBeRendered")
            .with_query(<(Entity, Read<Position>, Read<Render>)>::query())
            .build(|cmd, ecs, _, query| {
                let mut render_components: Vec<Entity> = query
                    .par_iter(ecs)
                    .map(|(e, pos, _)| {
                        if in_render_radius(pos) {
                            Err("Should render")
                        } else {
                            Ok(e.clone())
                        }
                    })
                    .filter_map(Result::ok)
                    .collect();

                for e in render_components.drain(..) {
                    cmd.remove_component::<Render>(e);
                }
            }),
    );
}
