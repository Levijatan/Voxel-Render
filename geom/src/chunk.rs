use std::ops::Range;
use building_blocks::{
    prelude::{Extent3, Point3, PointN},
    storage::Chunk,
};


use super::util;
use super::voxel;

pub const CHUNK_SIZE_F32: f32 = 16.0;
pub const CHUNK_SIZE_U32: u32 = CHUNK_SIZE_F32 as u32;
pub const CHUNK_SIZE_I32: i32 = CHUNK_SIZE_F32 as i32;
pub const CHUNK_SIZE_USIZE: usize = CHUNK_SIZE_F32 as usize;

pub const CHUNK_SHAPE: Point3<i32> = PointN([CHUNK_SIZE_I32; 3]);
pub const VOXELS_IN_CHUNK: usize = CHUNK_SIZE_USIZE * CHUNK_SIZE_USIZE * CHUNK_SIZE_USIZE;

pub type Position = Point3<i32>;
pub type Extent = Extent3<i32>;
pub type CType<M> = Chunk<[i32; 3], voxel::Id, M>;

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

#[allow(clippy::must_use_candidate)]
pub fn calc_center_point(pos: Position) -> glm::Vec3 {
    let offset = CHUNK_SIZE_F32;
    let mut pos_f32 = glm::vec3(pos.x() as f32, pos.y() as f32, pos.z() as f32);
    pos_f32 *= voxel::VOXEL_SIZE;
    pos_f32 -= glm::vec3(offset, offset, offset);
    pos_f32 + glm::vec3(offset/2.0, offset/2.0, offset/2.0)
}

#[allow(clippy::must_use_candidate, clippy::missing_const_for_fn)]
pub fn calc_radius() -> f32 {
    (CHUNK_SIZE_F32*voxel::VOXEL_SIZE)/2.0
}

pub trait Meta
{
    fn set_transparency(&mut self, value: u8);
    fn set_visibilty(&mut self, value: bool);
    fn is_visible(&self) -> bool;
    fn voxel_set_range(&mut self, range: Range<usize>, value: bool);
    fn voxel_is_visible(&self, p: voxel::Position) -> bool;
}
