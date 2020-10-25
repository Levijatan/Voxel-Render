use anyhow::{anyhow, ensure, Result};
use legion::{component, systems, Entity, IntoQuery, Read, SystemBuilder, Write};
use log::info;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::hash::{Hash, Hasher};

use super::util;
use super::voxel;
use super::world;

pub const VOXELS_IN_CHUNK: usize = crate::consts::CHUNK_SIZE_USIZE
    * crate::consts::CHUNK_SIZE_USIZE
    * crate::consts::CHUNK_SIZE_USIZE;

#[derive(Clone, Copy, Debug)]
pub struct Position {
    pub pos: glm::TVec3<i32>,
    pub world_id: Entity,
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

impl PartialEq for Position {
    fn eq(&self, other: &Self) -> bool {
        self.pos == other.pos
    }
}

impl Hash for Position {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.pos.hash(state);
    }
}

impl Eq for Position {}

#[derive(Copy, Clone, Debug)]
enum State {
    Gen(SubState),
    Update(SubState),
}

#[derive(Copy, Clone, Debug)]
enum SubState {
    Voxel,
    Transparent,
    Visibility,
    VoxelVisibilty,
    Waiting,
}

#[derive(Clone, Debug)]
pub struct Data {
    voxels: arrayvec::ArrayVec<[u64; VOXELS_IN_CHUNK]>,
    visible_voxels: arrayvec::ArrayVec<[bool; VOXELS_IN_CHUNK]>,
    state: State,
}

impl Data {
    fn next_state(&mut self) {
        use State::{Gen, Update};
        use SubState::{Transparent, Visibility, Voxel, VoxelVisibilty, Waiting};
        self.state = match self.state {
            Gen(Voxel) => Gen(Transparent),
            Gen(Transparent) => Gen(Visibility),
            Gen(Visibility) => Gen(VoxelVisibilty),
            Gen(VoxelVisibilty)
            | Update(Transparent)
            | Update(Visibility)
            | Update(VoxelVisibilty) => Update(Waiting),
            _ => self.state,
        };
    }

    fn set_state(&mut self, state: State) -> Result<()> {
        use State::Update;
        use SubState::{Transparent, Visibility, VoxelVisibilty};
        match state {
            Update(Transparent) | Update(Visibility) | Update(VoxelVisibilty) => self.state = state,
            _ => return Err(anyhow!("{:?} not allowed!", state)),
        };

        Ok(())
    }

    pub fn gen_render_instances(&self, pos: &Position) -> Vec<crate::render::state::Instance> {
        use crate::render::state::Instance;

        let chunk_pos = pos.get_f32_pos() * crate::consts::CHUNK_SIZE_F32;

        let offset = crate::consts::CHUNK_SIZE_F32;

        (0..VOXELS_IN_CHUNK)
            .into_par_iter()
            .map(|idx| {
                if self.visible_voxels[idx] {
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

    pub const fn ready_for_render(&self) -> bool {
        match self.state {
            State::Update(_) => true,
            State::Gen(_) => false,
        }
    }
}

pub fn new(pos: Position) -> (Position, Data) {
    let d = Data {
        voxels: arrayvec::ArrayVec::new(),
        visible_voxels: arrayvec::ArrayVec::new(),
        state: State::Gen(SubState::Voxel),
    };

    (pos, d)
}

pub fn voxel_in(pos: &glm::Vec3) -> bool {
    let size = crate::consts::CHUNK_SIZE_F32;
    !(pos.x >= size || pos.x < 0.0 || pos.y >= size || pos.y < 0.0 || pos.z >= size || pos.z < 0.0)
}

#[optick_attr::profile]
fn is_voxel_visible(pos: &glm::Vec3, voxreg: &voxel::Registry, data: &Data) -> Result<bool> {
    let n_idx = util::calc_idx_pos(pos)?;
    Ok(voxreg.is_transparent(data.voxels[n_idx]))
}

#[optick_attr::profile]
fn is_voxel_visible_neighbor_chunk(
    dir: util::Direction,
    pos: &Position,
    world: &world::World,
) -> bool {
    let n_pos = pos.neighbor(dir).unwrap();
    match world.chunk_is_transparent(&n_pos, dir) {
        Some(val) => val,
        None => true,
    }
}

fn is_vox_transparent(x: usize, y: usize, z: usize, data: &Data, voxreg: &voxel::Registry) -> bool {
    let idx = util::calc_idx(x, y, z).unwrap();
    let vox = data.voxels.get(idx).unwrap();
    voxreg.is_transparent(*vox)
}

pub fn system(schedule_builder: &mut systems::Builder) {
    schedule_builder.add_system(
        SystemBuilder::new("UpdateChunk")
            .with_query(<(Read<Position>, Write<Data>)>::query())
            .with_query(
                <(Entity, Read<world::World>)>::query().filter(component::<world::Active>()),
            )
            .read_resource::<voxel::Registry>()
            .read_resource::<world::TypeRegistry>()
            .read_resource::<crate::clock::Clock>()
            .build(
                |_, ecs, (vox_reg, world_type_reg, clock), (chunk_query, world_query)| {
                    if clock.cur_tick() > clock.last_tick() {
                        info!("***Updating Chunks***");
                        let (mut chunk_ecs, world_ecs) = ecs.split_for_query(chunk_query);
                        world_query.for_each(&world_ecs, |(world_id, world)| {
                            chunk_query.par_for_each_mut(&mut chunk_ecs, |(pos, data)| {
                                if pos.world_id == *world_id && world.chunk_has_ticket(pos) {
                                    match data.state {
                                        State::Gen(val) => gen_chunk(
                                            val,
                                            pos,
                                            data,
                                            world,
                                            world.world_type,
                                            world_type_reg,
                                            vox_reg,
                                        ),
                                        State::Update(val) => {
                                            update_chunk(val, pos, data, world, vox_reg)
                                        }
                                    };
                                    data.next_state();
                                }
                            });
                        });
                    }
                },
            ),
    );
}

fn gen_chunk(
    state: SubState,
    pos: &Position,
    data: &mut Data,
    world: &world::World,
    world_type: u32,
    world_type_reg: &world::TypeRegistry,
    vox_reg: &voxel::Registry,
) {
    use SubState::{Transparent, Visibility, Voxel, VoxelVisibilty, Waiting};
    info!("Generating Chunk at {:#?}", pos);
    match state {
        Voxel => gen_voxel(pos, data, world_type, world_type_reg, vox_reg).unwrap(),
        Transparent => gen_transparent(pos, world, data, vox_reg),
        Visibility => gen_visibility(pos, world),
        VoxelVisibilty => gen_voxel_visibility(pos, data, world, vox_reg),
        Waiting => {}
    }
}

fn gen_voxel(
    pos: &Position,
    data: &mut Data,
    world_type: u32,
    world_type_reg: &world::TypeRegistry,
    vox_reg: &voxel::Registry,
) -> Result<()> {
    info!("Generating voxels at {:#?}", pos);
    let world_type = world_type_reg.world_type(world_type)?;
    world_type.gen_chunk(pos, vox_reg, &mut data.voxels)?;
    Ok(())
}

fn gen_transparent(pos: &Position, world: &world::World, data: &Data, vox_reg: &voxel::Registry) {
    update_transparent(pos, data, world, vox_reg)
}

fn gen_visibility(pos: &Position, world: &world::World) {
    update_visibility(pos, world)
}

fn gen_voxel_visibility(
    pos: &Position,
    data: &mut Data,
    world: &world::World,
    vox_reg: &voxel::Registry,
) {
    update_voxel_visibility(pos, data, world, vox_reg)
}

fn update_chunk(
    state: SubState,
    pos: &Position,
    data: &mut Data,
    world: &world::World,
    vox_reg: &voxel::Registry,
) {
    info!("Updating chunk at {:#?}", pos);
    match state {
        SubState::Transparent => update_transparent(pos, data, world, vox_reg),
        SubState::Visibility => update_visibility(pos, world),
        SubState::VoxelVisibilty => update_voxel_visibility(pos, data, world, vox_reg),
        _ => {}
    }
}

fn update_transparent(
    pos: &Position,
    data: &Data,
    world: &world::World,
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

fn update_visibility(pos: &Position, world: &world::World) {
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
    data: &mut Data,
    world: &world::World,
    vox_reg: &voxel::Registry,
) {
    info!("voxel visibility at {:#?}", pos);
    data.visible_voxels.clear();
    (0..VOXELS_IN_CHUNK).into_iter().for_each(|idx| {
        let mut visible = false;
        if !vox_reg.is_transparent(data.voxels[idx]) {
            let voxel_pos = util::idx_to_pos(idx);

            for dir in &util::ALL_DIRECTIONS {
                let n_pos = voxel_pos + util::normals_f32(*dir);
                let tmp: bool;
                if voxel_in(&n_pos) {
                    tmp = is_voxel_visible(&n_pos, vox_reg, data).unwrap()
                } else {
                    tmp = is_voxel_visible_neighbor_chunk(*dir, pos, world)
                }
                if tmp {
                    visible = true;
                    break;
                }
            }
        }
        data.visible_voxels.push(visible);
    });
}
