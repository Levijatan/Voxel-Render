use anyhow::{anyhow, Result};
use building_blocks::prelude::{Point3, PointN};

use super::util;
use super::world;
use crate::consts::CHUNK_SIZE_i32;

pub const CHUNK_SHAPE: Point3<i32> = PointN([CHUNK_SIZE_i32; 3]);

pub type Position = Point3<i32>;

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

    fn key(&self) -> Point3<i32>{
        *self * CHUNK_SHAPE
    }
}

pub trait PositionTrait {
    fn neighbor(&self, dir: util::Direction) -> Self;
    fn key(&self) -> Point3<i32>;
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

pub fn new(world: world::Id, pos: Position) -> (world::Id, Position, State) {

    let state = State::Gen(SubState::Voxel);

    (world, pos, state)
}

