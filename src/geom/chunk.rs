use building_blocks::{
    prelude::{Extent3, Point3, PointN},
    storage::Chunk,
};
use bitvec::{array::BitArray, order::LocalBits, field::BitField};
use std::ops::Range;

use super::util;
use super::voxel;
use crate::consts::{CHUNK_SIZE_I32, CHUNK_SIZE_USIZE};

pub const CHUNK_SHAPE: Point3<i32> = PointN([CHUNK_SIZE_I32; 3]);
pub const VOXELS_IN_CHUNK: usize = CHUNK_SIZE_USIZE * CHUNK_SIZE_USIZE * CHUNK_SIZE_USIZE;

pub type Position = Point3<i32>;
pub type Extent = Extent3<i32>;
pub type CType = Chunk<[i32; 3], voxel::Id, Meta>;

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
    voxel_visibility: BitArray<LocalBits, [u64; VOXELS_IN_CHUNK/64]>,
    render_offset: Option<crate::render::chunk::BufferOffset>,
    pub render_amount: u16,
}

impl Meta {
    pub fn new() -> Self {
        Self {
            visibility: BitArray::new([0; 1]),
            voxel_visibility: BitArray::new([0; VOXELS_IN_CHUNK/64]),
            render_offset: None,
            render_amount: 0,
        }
    }

    pub fn set_transparency(&mut self, value: u8) {
        assert!(value < 64, "Max val allowed 63");
        self.visibility[..6].store(value);
    }

    pub fn set_visibilty(&mut self, value: bool) {
        self.visibility.set(6, value)
    }

    pub fn is_visible(&self) -> bool {
        *self.visibility.get(6).unwrap()
    }

    pub fn voxel_set_range(&mut self, range: Range<usize>, value: bool) {
        self.voxel_visibility[range].set_all(value);
    }

    pub fn voxel_is_visible(&self, p: voxel::Position) -> bool {
        let idx = util::calc_voxel_idx(p.x() as usize, p.y() as usize, p.z() as usize);
        *self.voxel_visibility.get(idx).unwrap()
    }

    pub fn has_render_offset(&self) -> bool {
        self.render_offset.is_some()
    }

    pub fn set_render_offset(&mut self, value: Option<crate::render::chunk::BufferOffset>) {
        self.render_offset = value;
    }

    pub fn render_offset(&self) -> Option<crate::render::chunk::BufferOffset> {
        self.render_offset
    }
}

pub fn calc_center_point(pos: Position) -> glm::Vec3 {
    let offset = crate::consts::CHUNK_SIZE_F32;
    let mut pos_f32 = glm::vec3(pos.x() as f32, pos.y() as f32, pos.z() as f32);
    pos_f32 *= crate::consts::VOXEL_SIZE;
    pos_f32 -= glm::vec3(offset, offset, offset);
    pos_f32 + glm::vec3(offset/2.0, offset/2.0, offset/2.0)
}

pub fn calc_radius() -> f32 {
    (crate::consts::CHUNK_SIZE_F32*crate::consts::VOXEL_SIZE)/2.0
}
