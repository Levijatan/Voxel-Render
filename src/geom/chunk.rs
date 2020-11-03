use anyhow::{anyhow, Result};
use bitvec::prelude::*;
use building_blocks::{
    prelude::{Extent3, Point3, PointN},
    storage::{Chunk, ChunkMapReader3, LocalChunkCache},
};

use super::util;
use super::voxel;
use super::world;
use crate::consts::CHUNK_SIZE_I32;

pub const CHUNK_SHAPE: Point3<i32> = PointN([CHUNK_SIZE_I32; 3]);

pub type Position = Point3<i32>;
pub type Extent = Extent3<i32>;
pub type CType = Chunk<[i32; 3], voxel::Id, Meta>;
pub type Cache = LocalChunkCache<[i32; 3], voxel::Id, Meta>;
pub type Id = legion::Entity;

impl PositionTrait for Position {
    fn neighbor(&self, dir: util::Direction) -> Self {
        let mut pos = *self;
        pos += util::normals_i32(dir);
        assert!(
            pos != *self,
            "{:?} should not be the same as {:?}",
            pos,
            self
        );
        pos
    }

    fn key(&self) -> Point3<i32> {
        *self * CHUNK_SHAPE
    }

    fn f32(&self) -> glm::Vec3 {
        glm::vec3(self.x() as f32, self.y() as f32, self.z() as f32)
    }

    fn edge_extent(&self, dir: util::Direction) -> Extent {
        use util::Direction::{Down, East, North, South, Up, West};
        let mut min_pos = self.key();
        let max_pos;
        match dir {
            North => {
                min_pos += util::normals_i32(dir) * (CHUNK_SIZE_I32 - 1);
                max_pos = min_pos + (util::normals_i32(Up) * (CHUNK_SIZE_I32 - 1));
            }
            South => {
                max_pos = min_pos + (util::normals_i32(Up) * (CHUNK_SIZE_I32 - 1));
            }
            East => {
                max_pos = min_pos
                    + util::normals_i32(North) * (CHUNK_SIZE_I32 - 1)
                    + (util::normals_i32(Up) * (CHUNK_SIZE_I32 - 1));
            }
            West => {
                min_pos += util::normals_i32(dir) * (CHUNK_SIZE_I32 - 1);
                max_pos = min_pos
                    + util::normals_i32(North) * (CHUNK_SIZE_I32 - 1)
                    + (util::normals_i32(Up) * (CHUNK_SIZE_I32 - 1));
            }
            Up => {
                min_pos += util::normals_i32(dir) * (CHUNK_SIZE_I32 - 1);
                max_pos = min_pos
                    + util::normals_i32(North) * (CHUNK_SIZE_I32 - 1)
                    + (util::normals_i32(West) * (CHUNK_SIZE_I32 - 1));
            }
            Down => {
                max_pos = min_pos
                    + util::normals_i32(North) * (CHUNK_SIZE_I32 - 1)
                    + (util::normals_i32(West) * (CHUNK_SIZE_I32 - 1));
            }
        };
        Extent3::from_min_and_max(min_pos, max_pos)
    }
}

pub trait PositionTrait {
    fn neighbor(&self, dir: util::Direction) -> Self;
    fn key(&self) -> Point3<i32>;
    fn f32(&self) -> glm::Vec3;
    fn edge_extent(&self, dir: util::Direction) -> Extent;
}

#[derive(Debug, Copy, Clone)]
pub struct Meta {
    //0-5 transparency, 6 visibility
    visibility: BitArray<LocalBits, [u8; 1]>,
    id: Option<Id>,

    pub ticket: Option<generational_arena::Index>,
}

impl Meta {
    pub fn new() -> Self {
        Self {
            visibility: BitArray::new([0; 1]),
            ticket: None,
            id: None,
        }
    }

    pub fn set_transparency(&mut self, value: u8) {
        self.visibility[..6].store(value);
    }

    pub fn set_visibilty(&mut self, value: bool) {
        self.visibility.set(6, value)
    }

    pub fn is_transparent(&self, dir: util::Direction) -> bool {
        let idx: usize = dir.into();
        *self.visibility.get(idx).unwrap()
    }

    pub fn is_visible(&self) -> bool {
        *self.visibility.get(6).unwrap()
    }

    pub const fn id(&self) -> Option<Id> {
        self.id
    }

    pub fn has_id(&self) -> bool {
        self.id.is_some()
    }

    pub fn set_id(&mut self, value: Id) {
        self.id = Some(value);
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum State {
    Gen(SubState),
    Update(SubState),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SubState {
    Transparent,
    Visibility,
    Waiting,
}

impl State {
    fn step(&mut self) {
        use State::{Gen, Update};
        use SubState::{Transparent, Visibility, Waiting};
        *self = match self {
            Gen(Transparent) => Gen(Visibility),
            Gen(Visibility) | Update(Transparent) | Update(Visibility) => Update(Waiting),
            _ => *self,
        };
    }

    fn set(&mut self, state: Self) -> Result<()> {
        use State::Update;
        use SubState::{Transparent, Visibility};
        match state {
            Update(Transparent) | Update(Visibility) => *self = state,
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

pub const fn new(world: world::Id, pos: Position) -> (world::Id, Position, State) {
    let state = State::Gen(SubState::Transparent);

    (world, pos, state)
}

use crate::render::state::Instance;
pub fn gen_render_instances(
    pos: &Position,
    map: &world::Map,
    vox_reg: &voxel::Registry,
    cache: &Cache,
) -> Vec<Instance> {
    use building_blocks::prelude::ForEachRef as _;
    use building_blocks::prelude::Get as _;
    let mut out = Vec::new();
    let extent = map.chunk_map.extent_for_chunk_at_key(&pos.key());
    let reader = ChunkMapReader3::new(&map.chunk_map, cache);
    reader.for_each_ref(&extent, |p, value| {
        if !vox_reg.is_transparent(*value).unwrap() {
            let mut visible = false;
            for dir in &util::ALL_DIRECTIONS {
                let n = p.neighbor(*dir);
                let n_val = reader.get(&n);
                if vox_reg.is_transparent(n_val).unwrap() {
                    visible = true;
                    break;
                }
            }
            if visible {
                let offset = crate::consts::CHUNK_SIZE_F32;
                let position = (p.f32() * crate::consts::VOXEL_SIZE) - glm::vec3(offset, offset, offset);
                let rotation = glm::quat_angle_axis(0.0, &glm::Vec3::z_axis().into_inner());
                out.push(Instance { position, rotation })
            }
        }
    });
    out
}

use building_blocks::prelude::ForEachRef;
use legion::{component, IntoQuery, Read, Write};
pub fn system(schedule_builder: &mut legion::systems::Builder) {
    transparent_system(schedule_builder);
    visibility_system(schedule_builder);
}

fn transparent_system(schedule_builder: &mut legion::systems::Builder) {
    schedule_builder.add_system(
        legion::SystemBuilder::new("ChunkTransparencySystem")
            .with_query(<(Read<world::Id>, Read<Position>, Write<State>)>::query())
            .with_query(
                <(legion::Entity, Write<world::Map>)>::query().filter(component::<world::Active>()),
            )
            .read_resource::<voxel::Registry>()
            .read_resource::<crate::clock::Clock>()
            .build(| _, ecs, (vox_reg, clock), (chunk_query, world_query)| {
                if clock.cur_tick() > clock.last_tick() {
                    let (mut chunk_ecs, mut world_ecs) = ecs.split_for_query(chunk_query);
                    world_query.for_each_mut(&mut world_ecs, |(world_id, map)| {
                        let cache = Cache::new();
                        chunk_query.for_each_mut(&mut chunk_ecs, |(chunk_world_id, pos, state)| {
                            if world_id == chunk_world_id
                                && (*state == State::Gen(SubState::Transparent)
                                    || *state == State::Update(SubState::Transparent))
                            {
                                let reader = ChunkMapReader3::new(&map.chunk_map, &cache);
                                let mut t: u8 = 0;
                                for dir in &util::ALL_DIRECTIONS {
                                    let extent = pos.edge_extent(*dir);
                                    let mut transparent = false;
                                    reader.for_each_ref(&extent, |_p, voxel| {
                                        if vox_reg.is_transparent(*voxel).unwrap() {
                                            transparent = true;
                                        }
                                    });
                                    if transparent {
                                        let tmp: u8 = (*dir).into();
                                        t += tmp;
                                    }
                                }
                                map.chunk_set_transparency(pos, t);
                                state.step();
                            }
                        });
                        map.chunk_map.flush_chunk_cache(cache);
                    })
                }
            }),
    );
}

fn visibility_system(schedule_builder: &mut legion::systems::Builder) {
    schedule_builder.add_system(legion::SystemBuilder::new("ChunkVisibilitySystem")
        .with_query(<(Read<world::Id>, Read<Position>, Write<State>)>::query())
        .with_query(
        <(legion::Entity, Write<world::Map>)>::query().filter(component::<world::Active>()),
        )
        .read_resource::<crate::clock::Clock>()
        .build(|_, ecs, clock, (chunk_query, world_query)| {
            if clock.cur_tick() > clock.last_tick() {
                let (mut chunk_ecs, mut world_ecs) = ecs.split_for_query(chunk_query);
                world_query.for_each_mut(&mut world_ecs, |(world_id, map)| {
                    let cache = Cache::new();
                    chunk_query.for_each_mut(&mut chunk_ecs, |(chunk_world_id, pos, state)| {
                        if world_id == chunk_world_id
                            && (*state == State::Gen(SubState::Visibility)
                                || *state == State::Update(SubState::Visibility))
                        {
                            let mut visible = false;
                            for dir in &util::ALL_DIRECTIONS {
                                let n = pos.neighbor(*dir);
                                visible = match map.chunk_is_transparent(&n, *dir, &cache) {
                                    Some(val) => val,
                                    None => true,
                                };
                                if visible {
                                    break;
                                }
                            }
                            map.chunk_set_visible(pos, visible);
                            state.step();
                        }
                    });
                });
            }
        })
    );
}
