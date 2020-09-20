use super::consts::MAX_LIGHT;
use super::consts::MIN_LIGHT;
use super::util;
use crate::consts::INVALID_VOXEL_ID;
use crate::consts::TRANSPARENT_VOXEL;
use crate::render::min_max_norm;
use crate::VoxelReg;

use glm::Vec3;

#[derive(Debug)]
pub struct Chunk {
    pub size: usize,
    pub tot_size: usize,
    pub v: Vec<u64>,
    pub render_v: Vec<u64>,
    pub sunlight: Vec<u8>,
    pub torchlight: Vec<u8>,
    pub pos: Vec3,
    pub world_pos_min: Vec3,
    pub world_pos_max: Vec3,
    pub update: bool,
    pub update_lighting: bool,
    pub in_queue: bool,
    pub transparent: bool,
    pub render: bool,
}

impl Chunk {
    pub fn new(size: usize, x: f32, y: f32, z: f32, reg: &VoxelReg) -> Chunk {
        let rx = x * size as f32;
        let ry = y * size as f32;
        let rz = z * size as f32;

        let world_pos_min = Vec3::new(rx, ry, rz);
        let tot_size = size * size * size;

        Chunk {
            tot_size,
            size,
            v: vec![reg.key_from_string_id(TRANSPARENT_VOXEL); tot_size as usize],
            render_v: vec![0; tot_size as usize],
            sunlight: vec![0; tot_size as usize],
            torchlight: vec![0; tot_size as usize],
            pos: Vec3::new(x, y, z),
            world_pos_min,
            world_pos_max: world_pos_min + Vec3::new(size as f32, size as f32, size as f32),
            update: true,
            update_lighting: true,
            in_queue: false,
            transparent: false,
            render: false,
        }
    }

    pub fn render(&self) -> Vec<f32> {
        let mut out = Vec::new();
        for idx in 0..self.tot_size as usize {
            let pos = util::idx_to_pos(idx, self.size);
            if self.render_v[idx] != INVALID_VOXEL_ID {
                out.push(self.world_pos_min.x + pos.x as f32);
                out.push(self.world_pos_min.y + pos.y as f32);
                out.push(self.world_pos_min.z + pos.z as f32);
                let lightlevel = min_max_norm(
                    self.sunlight_at(idx).max(self.torchlight_at(idx)) as f32,
                    MIN_LIGHT as f32,
                    MAX_LIGHT as f32,
                );
                out.push(lightlevel);
            }
        }
        out
    }

    pub fn in_chunk(&self, pos: Vec3) -> bool {
        return self.world_pos_min.x <= pos.x
            && pos.x < self.world_pos_max.x
            && self.world_pos_min.y <= pos.y
            && pos.y < self.world_pos_max.y
            && self.world_pos_min.z <= pos.z
            && pos.z < self.world_pos_max.z;
    }

    pub fn set_voxel(&mut self, voxel_type: u64, world_pos: Vec3) {
        let in_chunk_pos = self.in_chunk_pos(world_pos);
        let idx = self.calc_idx_point(in_chunk_pos);
        self.set_voxel_idx(idx as usize, voxel_type);
    }

    pub fn sunlight_at(&self, idx: usize) -> u8 {
        if idx >= self.tot_size {
            panic!("To big idx value: {}", idx);
        }
        self.sunlight[idx]
    }

    pub fn torchlight_at(&self, idx: usize) -> u8 {
        self.torchlight[idx]
    }

    pub fn set_sunlight_at(&mut self, idx: usize, val: u8) {
        self.sunlight[idx] = val;
    }

    pub fn set_torchlight_at(&mut self, idx: usize, val: u8) {
        self.torchlight[idx] = val;
    }

    pub fn set_voxel_idx(&mut self, idx: usize, v: u64) {
        self.update_lighting = true;
        self.update = true;
        self.v[idx] = v;
    }

    pub fn calc_idx_point(&self, point: Vec3) -> usize {
        self.calc_idx(point.x, point.y, point.z)
    }

    pub fn calc_idx(&self, x: f32, y: f32, z: f32) -> usize {
        util::calc_idx(x as usize, y as usize, z as usize, self.size)
    }

    pub fn calc_idx_world(&self, world_pos: Vec3) -> usize {
        let in_chunk = self.in_chunk_pos(world_pos);
        self.calc_idx_point(in_chunk)
    }

    pub fn in_chunk_pos(&self, voxel_pos: Vec3) -> Vec3 {
        voxel_pos - self.world_pos_min
    }

    pub fn voxel_to_world_pos(&self, pos: Vec3) -> Vec3 {
        Vec3::new(
            pos.x + self.world_pos_min.x,
            pos.y + self.world_pos_min.y,
            pos.z + self.world_pos_min.z,
        )
    }

    pub fn check_voxel_transparency(&self, pos: Vec3, reg: &VoxelReg) -> bool {
        let in_chunk_pos = pos - self.world_pos_min;
        let p = Vec3::new(in_chunk_pos.x, in_chunk_pos.y, in_chunk_pos.z);
        self.check_voxel_in_chunk_transparency(p, reg)
    }

    pub fn check_voxel_in_chunk_transparency(&self, pos: Vec3, reg: &VoxelReg) -> bool {
        let idx = self.calc_idx_point(pos);
        let vox_type = self.v[idx as usize];
        let vox_attr = reg.voxel_attributes(&vox_type);
        vox_attr.transparent
    }

    pub fn is_surface(&self) -> bool {
        self.pos.y >= 0.0
    }
}
