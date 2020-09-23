use super::Chunk;
use crate::VoxelReg;

use std::collections::HashMap;
use std::fmt;

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
    c: HashMap<ChunkKey, Chunk>,
    chunk_size: usize,
    tot_chunk_size: usize,
}

impl PointCloud {
    pub fn new(chunk_size: usize) -> PointCloud {
        return PointCloud {
            c: HashMap::new(),
            chunk_size,
            tot_chunk_size: chunk_size * chunk_size * chunk_size,
        };
    }

    pub fn insert_chunk(&mut self, key: ChunkKey, chunk: Chunk) {
        self.c.insert(key, chunk);
    }

    pub fn chunk_exists(&self, key: &ChunkKey) -> bool {
        self.c.contains_key(key) && !self.c[key].gen
    }

    pub fn chunk_is_transparent(&self, key: &ChunkKey, norm_key: i32) -> bool {
        if self.chunk_exists(key) {
            self.c[key].is_transparent(norm_key)
        } else {
            true
        }
    }

    pub fn chunk_pos(&self, key: &ChunkKey) -> Vec3 {
        self.c[key].pos
    }

    pub fn render_chunk(&self, key: &ChunkKey) -> Vec<f32> {
        self.c[key].render(self.chunk_size)
    }

    pub fn chunk_rerender(&self, key: &ChunkKey) -> bool {
        self.c[key].rerender
    }

    pub fn chunk_in_queue(&self, key: &ChunkKey) -> bool {
        self.c[key].in_queue
    }

    pub fn chunk_v_to_render_v(&mut self, key: &ChunkKey, idx: usize) {
        self.c.get_mut(key).unwrap().v_to_render_v(idx)
    }

    pub fn chunk_set_rerender(&mut self, key: &ChunkKey, rerender: bool) {
        self.c.get_mut(key).unwrap().rerender = rerender
    }

    pub fn chunk_set_in_queue(&mut self, key: &ChunkKey, in_queue: bool) {
        self.c.get_mut(key).unwrap().in_queue = in_queue
    }

    pub fn chunk_tot_size(&self) -> usize {
        self.tot_chunk_size
    }

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

    pub fn voxel_in_chunk_transparency_idx(
        &self,
        key: &ChunkKey,
        idx: usize,
        reg: &VoxelReg,
    ) -> bool {
        self.c[key].check_voxel_in_chunk_transparency_idx(idx, reg)
    }

    pub fn voxel_transparency(
        &self,
        voxel_world_pos: &Vec3,
        key: &ChunkKey,
        reg: &VoxelReg,
        chunk_size: usize,
    ) -> bool {
        self.c[key].check_voxel_transparency(voxel_world_pos, reg, chunk_size)
    }

    pub fn chunk_size(&self) -> usize {
        self.chunk_size
    }

    pub fn chunk_world_pos_min(&self, key: &ChunkKey) -> Vec3 {
        self.c[key].world_pos_min
    }

    pub fn voxel_to_world_pos(&self, key: &ChunkKey, voxel_pos: &Vec3) -> Vec3 {
        self.c[key].voxel_to_world_pos(voxel_pos)
    }

    pub fn voxel_pos_in_chunk(&self, key: &ChunkKey, voxel_pos: &Vec3) -> bool {
        self.c[key].voxel_pos_in_chunk(voxel_pos, self.chunk_size)
    }

    pub fn chunk_is_visible(&self, key: &ChunkKey) -> bool {
        if self.chunk_exists(key) {
            self.c[key].visible
        } else {
            false
        }
    }

    pub fn chunk_set_visible(&mut self, key: &ChunkKey, visible: bool) {
        self.c.get_mut(key).unwrap().visible = visible
    }
}
