use super::Chunk;
use crate::VoxelReg;

use dashmap::DashMap;
use std::fmt;

use flamer::flame;
use glm::Vec3;

#[derive(Hash, Eq, PartialEq, Debug, Copy, Clone, PartialOrd, Ord)]
pub struct ChunkKey {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl fmt::Display for ChunkKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

impl ChunkKey {
    #[flame("ChunkKey")]
    pub fn new(point: Vec3) -> ChunkKey {
        return ChunkKey {
            x: point.x as i32,
            y: point.y as i32,
            z: point.z as i32,
        };
    }
}

#[derive(Debug)]
pub struct PointCloud {
    c: DashMap<ChunkKey, Chunk>,
    chunk_size: usize,
    tot_chunk_size: usize,
}

impl PointCloud {
    #[flame("PointCloud")]
    pub fn new(chunk_size: usize) -> PointCloud {
        return PointCloud {
            c: DashMap::new(),
            chunk_size,
            tot_chunk_size: chunk_size * chunk_size * chunk_size,
        };
    }

    #[flame("PointCloud")]
    pub fn insert_chunk(&self, key: ChunkKey, c: Chunk) {
        self.c.insert(key, c);
    }

    #[flame("PointCloud")]
    pub fn chunk_exists(&self, key: &ChunkKey) -> bool {
        self.c.contains_key(key)
    }

    #[flame("PointCloud")]
    pub fn chunk_set_render_data(&self, key: &ChunkKey, render_data: Vec<f32>) {
        self.c.get_mut(key).unwrap().set_render_data(render_data);
    }

    #[flame("PointCloud")]
    pub fn chunk_is_transparent(&self, key: &ChunkKey, norm_key: i32) -> bool {
        if self.chunk_exists(key) {
            self.c.get(key).unwrap().is_transparent(norm_key)
        } else {
            true
        }
    }

    #[flame("PointCloud")]
    pub fn chunk_tot_size(&self) -> usize {
        self.tot_chunk_size
    }

    #[flame("PointCloud")]
    pub fn voxel_in_chunk_transparency(
        &self,
        key: &ChunkKey,
        in_chunk_pos: &Vec3,
        reg: &VoxelReg,
    ) -> bool {
        let idx = super::calc_idx(
            in_chunk_pos.x as usize,
            in_chunk_pos.y as usize,
            in_chunk_pos.z as usize,
            self.chunk_size,
        );
        self.voxel_in_chunk_transparency_idx(key, idx, reg)
    }

    #[flame("PointCloud")]
    pub fn voxel_in_chunk_transparency_idx(
        &self,
        key: &ChunkKey,
        idx: usize,
        reg: &VoxelReg,
    ) -> bool {
        self.c
            .get(key)
            .unwrap()
            .check_voxel_in_chunk_transparency_idx(idx, reg)
    }

    #[flame("PointCloud")]
    pub fn voxel_transparency(
        &self,
        voxel_world_pos: &Vec3,
        key: &ChunkKey,
        reg: &VoxelReg,
        chunk_size: usize,
    ) -> bool {
        self.c
            .get(key)
            .unwrap()
            .check_voxel_transparency(voxel_world_pos, reg, chunk_size)
    }

    #[flame("PointCloud")]
    pub fn voxel_to_world_pos(&self, key: &ChunkKey, voxel_pos: &Vec3) -> Vec3 {
        self.c.get(key).unwrap().voxel_to_world_pos(voxel_pos)
    }

    #[flame("PointCloud")]
    pub fn voxel_pos_in_chunk(&self, key: &ChunkKey, voxel_pos: &Vec3) -> bool {
        self.c
            .get(key)
            .unwrap()
            .voxel_pos_in_chunk(voxel_pos, self.chunk_size)
    }

    #[flame("PointCloud")]
    pub fn chunk_render(&self, key: &ChunkKey) -> Vec<f32> {
        self.c.get(key).unwrap().get_render_date().clone()
    }
}
