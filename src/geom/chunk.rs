use anyhow::{anyhow, ensure, Result};
use legion::{component, maybe_changed, systems, Entity, IntoQuery, Read, SystemBuilder, Write};
use log::info;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::hash::Hash;

use super::util;
use super::voxel;
use super::world;

pub const VOXELS_IN_CHUNK: usize = crate::consts::CHUNK_SIZE_USIZE
    * crate::consts::CHUNK_SIZE_USIZE
    * crate::consts::CHUNK_SIZE_USIZE;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Position {
    pub pos: glm::TVec3<i32>,
}

impl Position {
    pub fn neighbor(&self, dir: util::Direction) -> Result<Self> {
        let mut pos = *self;
        pos.pos += util::normals_i32(dir);
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

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum State {
    Gen(SubState),
    Update(SubState),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SubState {
    Voxel,
    Transparent,
    Visibility,
    VoxelVisibilty,
    Waiting,
}

impl State {
    fn next_state(&mut self) {
        use State::{Gen, Update};
        use SubState::{Transparent, Visibility, Voxel, VoxelVisibilty, Waiting};
        *self = match self {
            Gen(Voxel) => Gen(Transparent),
            Gen(Transparent) => Gen(Visibility),
            Gen(Visibility) => Gen(VoxelVisibilty),
            Gen(VoxelVisibilty)
            | Update(Transparent)
            | Update(Visibility)
            | Update(VoxelVisibilty) => Update(Waiting),
            Update(Voxel) => Gen(Voxel),
            _ => *self,
        };
    }

    fn set(&mut self, state: Self) -> Result<()> {
        use State::{Update, Gen};
        use SubState::{Transparent, Visibility, VoxelVisibilty, Voxel};
        match state {
            Update(Transparent) | Update(Visibility) | Update(VoxelVisibilty) | Gen(Voxel) => *self = state,
            _ => return Err(anyhow!("{:?} not allowed!", state)),
        };

        Ok(())
    }

    pub const fn ready_for_render(self) -> bool {
        match self {
            Self::Update(_) => true,
            Self::Gen(_) => false,
        }
    }
}

pub type Voxels = arrayvec::ArrayVec<[voxel::Id; VOXELS_IN_CHUNK]>;
pub type VisibleVoxels = arrayvec::ArrayVec<[bool; VOXELS_IN_CHUNK]>;

pub fn gen_render_instances(
    visible_voxels: &VisibleVoxels,
    pos: &Position,
) -> Vec<crate::render::state::Instance> {
    use crate::render::state::Instance;

    let chunk_pos = pos.get_f32_pos() * crate::consts::CHUNK_SIZE_F32;

    let offset = crate::consts::CHUNK_SIZE_F32;

    (0..VOXELS_IN_CHUNK)
        .into_par_iter()
        .map(|idx| {
            if visible_voxels[idx] {
                let voxel_pos = util::idx_to_pos(idx);
                let position = (chunk_pos + voxel_pos) * crate::consts::VOXEL_SIZE
                    - glm::vec3(offset, offset, offset);
                let rotation = glm::quat_angle_axis(0.0, &glm::Vec3::z_axis().into_inner());
                Ok(Instance { position, rotation })
            } else {
                Err(())
            }
        })
        .filter_map(Result::ok)
        .collect::<Vec<_>>()
}

pub fn new(world: world::Id, pos: Position) -> (world::Id, Position, Voxels, VisibleVoxels, State) {
    let voxels = Voxels::new();
    let visible_voxels = VisibleVoxels::new();
    let state = State::Gen(SubState::Voxel);

    (world, pos, voxels, visible_voxels, state)
}

pub fn voxel_in(pos: &glm::Vec3) -> bool {
    let size = crate::consts::CHUNK_SIZE_F32;
    !(pos.x >= size || pos.x < 0.0 || pos.y >= size || pos.y < 0.0 || pos.z >= size || pos.z < 0.0)
}

#[optick_attr::profile]
fn is_voxel_visible(pos: &glm::Vec3, voxreg: &voxel::Registry, voxels: &Voxels) -> Result<bool> {
    let n_idx = util::calc_idx_pos(pos);
    Ok(voxreg.is_transparent(voxels[n_idx]))
}

#[optick_attr::profile]
fn is_voxel_visible_neighbor_chunk(
    dir: util::Direction,
    pos: &Position,
    world: &world::Map,
) -> bool {
    let n_pos = pos.neighbor(dir).unwrap();
    match world.chunk_is_transparent(&n_pos, dir) {
        Some(val) => val,
        None => true,
    }
}

fn is_vox_transparent(
    x: usize,
    y: usize,
    z: usize,
    voxels: &Voxels,
    voxreg: &voxel::Registry,
) -> bool {
    let idx = util::calc_idx(x, y, z);
    let vox = voxels.get(idx).unwrap();
    voxreg.is_transparent(*vox)
}

pub fn system(schedule_builder: &mut systems::Builder) {
    voxels_system(schedule_builder);
    transparent_system(schedule_builder);
    visibility_system(schedule_builder);
    voxel_visibility_system(schedule_builder);
}

fn voxels_system(schedule_builder: &mut systems::Builder) {
    schedule_builder.add_system(
        SystemBuilder::new("VoxelGenSystem")
            .with_query(<(
                Read<world::Id>,
                Read<Position>,
                Write<Voxels>,
                Write<State>,
            )>::query())
            .with_query(
                <(Entity, Read<world::TypeId>)>::query().filter(component::<world::Active>()),
            )
            .read_resource::<voxel::Registry>()
            .read_resource::<world::TypeRegistry>()
            .read_resource::<crate::clock::Clock>()
            .build(
                |_, ecs, (vox_reg, world_type_reg, clock), (chunk_query, world_query)| {
                    if clock.cur_tick() > clock.last_tick() {
                        let (world_ecs, mut chunk_ecs) = ecs.split_for_query(world_query);
                        world_query.for_each(&world_ecs, |(world_id, world_type)| {
                            chunk_query.par_for_each_mut(
                                &mut chunk_ecs,
                                |(chunk_world_id, pos, voxels, state)| {
                                    if chunk_world_id == world_id
                                        && *state == State::Gen(SubState::Voxel)
                                    {
                                        gen_voxel(
                                            pos,
                                            voxels,
                                            *world_type,
                                            world_type_reg,
                                            vox_reg,
                                        )
                                        .unwrap();
                                        state.next_state();
                                    }
                                },
                            )
                        })
                    }
                },
            ),
    );
}

fn transparent_system(schedule_builder: &mut systems::Builder) {
    schedule_builder.add_system(
        SystemBuilder::new("ChunkTransparencySystem")
            .with_query(<(
                Read<world::Id>,
                Read<Position>,
                Read<Voxels>,
                Write<State>,
            )>::query())
            .with_query(<(Entity, Read<world::Map>)>::query().filter(component::<world::Active>()))
            .read_resource::<voxel::Registry>()
            .read_resource::<crate::clock::Clock>()
            .build(|_, ecs, (vox_reg, clock), (chunk_query, world_query)| {
                if clock.cur_tick() > clock.last_tick() {
                    let (world_ecs, mut chunk_ecs) = ecs.split_for_query(world_query);
                    world_query.for_each(&world_ecs, |(world_id, map)| {
                        chunk_query.par_for_each_mut(
                            &mut chunk_ecs,
                            |(chunk_world_id, pos, voxels, state)| {
                                if world_id == chunk_world_id
                                    && (*state == State::Gen(SubState::Transparent)
                                        || *state == State::Update(SubState::Transparent))
                                {
                                    update_transparent(pos, voxels, map, vox_reg);
                                    state.next_state();
                                }
                            },
                        )
                    })
                }
            }),
    );
}

fn visibility_system(schedule_builder: &mut systems::Builder) {
    schedule_builder.add_system(
        SystemBuilder::new("ChunkVisibilitySystem")
            .with_query(<(Read<world::Id>, Read<Position>, Write<State>)>::query())
            .with_query(<(Entity, Read<world::Map>)>::query().filter(component::<world::Active>()))
            .read_resource::<crate::clock::Clock>()
            .build(|_, ecs, clock, (chunk_query, world_query)| {
                if clock.cur_tick() > clock.last_tick() {
                    let (world_ecs, mut chunk_ecs) = ecs.split_for_query(world_query);
                    world_query.for_each(&world_ecs, |(world_id, map)| {
                        chunk_query.par_for_each_mut(
                            &mut chunk_ecs,
                            |(chunk_world_id, pos, state)| {
                                if world_id == chunk_world_id
                                    && (*state == State::Gen(SubState::Visibility)
                                        || *state == State::Update(SubState::Visibility))
                                {
                                    update_visibility(pos, map);
                                    state.next_state();
                                }
                            },
                        )
                    })
                }
            }),
    );
}

fn voxel_visibility_system(schedule_builder: &mut systems::Builder) {
    schedule_builder.add_system(
        SystemBuilder::new("UpdateChunk")
            .with_query(
                <(
                    Read<world::Id>,
                    Read<Position>,
                    Read<Voxels>,
                    Write<VisibleVoxels>,
                    Write<State>,
                )>::query()
                .filter(maybe_changed::<Voxels>() | maybe_changed::<State>()),
            )
            .with_query(
                <(Entity, Read<world::Map>)>::query()
                    .filter(component::<world::Active>()),
            )
            .read_resource::<voxel::Registry>()
            .read_resource::<crate::clock::Clock>()
            .build(|_, ecs, (vox_reg, clock), (chunk_query, world_query)| {
                if clock.cur_tick() > clock.last_tick() {
                    info!("***Updating Chunks***");
                    let (mut chunk_ecs, world_ecs) = ecs.split_for_query(chunk_query);
                    world_query.for_each(&world_ecs, |(world_id, world)| {
                        chunk_query.par_for_each_mut(
                            &mut chunk_ecs,
                            |(chunk_world_id, pos, voxels, visible_voxels, state)| {
                                if chunk_world_id == world_id
                                    && (*state == State::Gen(SubState::VoxelVisibilty)
                                        || *state == State::Update(SubState::VoxelVisibilty))
                                {
                                    update_voxel_visibility(pos, voxels, visible_voxels, world, vox_reg);
                                    state.next_state();
                                }
                            },
                        );
                    });
                }
            }),
    );
}

fn gen_voxel(
    pos: &Position,
    voxels: &mut Voxels,
    world_type: u32,
    world_type_reg: &world::TypeRegistry,
    vox_reg: &voxel::Registry,
) -> Result<()> {
    info!("Generating voxels at {:#?}", pos);
    let world_type = world_type_reg.world_type(world_type)?;
    world_type.gen_chunk(pos, vox_reg, voxels)?;
    Ok(())
}

fn update_transparent(
    pos: &Position,
    data: &Voxels,
    world: &world::Map,
    vox_reg: &voxel::Registry,
) {
    use crate::consts::CHUNK_SIZE_USIZE;
    use util::Direction::{Down, East, North, South, Up, West};
    info!("Transparency at {:#?}", pos);

    let mut transparency: u8 = 0;
    'outer: for i in 0..CHUNK_SIZE_USIZE {
        for j in 0..CHUNK_SIZE_USIZE {
            let east: u8 = East.into();
            if east & transparency != east
                && is_vox_transparent(i, CHUNK_SIZE_USIZE - 1, j, data, vox_reg)
            {
                transparency += east;
            }
            let west: u8 = West.into();
            if west & transparency != west
                && is_vox_transparent(i, CHUNK_SIZE_USIZE - 1, j, data, vox_reg)
            {
                transparency += west;
            }
            let up: u8 = Up.into();
            if up & transparency != up
                && is_vox_transparent(i, CHUNK_SIZE_USIZE - 1, j, data, vox_reg)
            {
                transparency += up;
            }
            let down: u8 = Down.into();
            if down & transparency != down
                && is_vox_transparent(i, CHUNK_SIZE_USIZE - 1, j, data, vox_reg)
            {
                transparency += down;
            }
            let north: u8 = North.into();
            if north & transparency != north
                && is_vox_transparent(i, CHUNK_SIZE_USIZE - 1, j, data, vox_reg)
            {
                transparency += north;
            }
            let south: u8 = South.into();
            if south & transparency != south
                && is_vox_transparent(i, CHUNK_SIZE_USIZE - 1, j, data, vox_reg)
            {
                transparency += south;
            }

            if transparency == 0b0011_1111 {
                break 'outer;
            }
        }
    }

    world.chunk_set_transparency(pos, transparency).unwrap();
}

fn update_visibility(pos: &Position, world: &world::Map) {
    let mut visible = false;
    info!("visibility at {:#?}", pos);
    for dir in &util::ALL_DIRECTIONS {
        let n_pos = pos.neighbor(*dir).unwrap();
        if let Some(transparent) = world.chunk_is_transparent(&n_pos, *dir) {
            if transparent {
                visible = true;
                break;
            }
        }
    }
    world.chunk_set_visible(pos, visible);
}

fn update_voxel_visibility(
    pos: &Position,
    voxels: &Voxels,
    visible_voxels: &mut VisibleVoxels,
    world: &world::Map,
    vox_reg: &voxel::Registry,
) {
    info!("voxel visibility at {:#?}", pos);
    visible_voxels.clear();
    (0..VOXELS_IN_CHUNK).into_iter().for_each(|idx| {
        let mut visible = false;
        if !vox_reg.is_transparent(voxels[idx]) {
            let voxel_pos = util::idx_to_pos(idx);

            for dir in &util::ALL_DIRECTIONS {
                let n_pos = voxel_pos + util::normals_f32(*dir);
                let tmp: bool;
                if voxel_in(&n_pos) {
                    tmp = is_voxel_visible(&n_pos, vox_reg, voxels).unwrap()
                } else {
                    tmp = is_voxel_visible_neighbor_chunk(*dir, pos, world)
                }
                if tmp {
                    visible = true;
                    break;
                }
            }
        }
        visible_voxels.push(visible);
    });
}
