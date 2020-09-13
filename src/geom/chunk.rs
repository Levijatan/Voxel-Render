use cgmath::{EuclideanSpace, Point3, Vector3};

use crate::consts::INVALID_VOXEL_ID;
use crate::consts::TRANSPARENT_VOXEL;
use crate::VoxelReg;

#[derive(Debug)]
pub struct Chunk {
    pub size: f32,
    pub tot_size: f32,
    pub v: Vec<u64>,
    pub render_v: Vec<u64>,
    pub sunlight: Vec<u8>,
    pub torchlight: Vec<u8>,
    pub pos: Point3<f32>,
    pub world_pos_min: Point3<f32>,
    pub world_pos_max: Point3<f32>,
    pub update: bool,
    pub update_lighting: bool,
}

impl Chunk {
    pub fn new(size: f32, x: f32, y: f32, z: f32, reg: &VoxelReg) -> Chunk {
        let rx = x * size;
        let ry = y * size;
        let rz = z * size;

        let world_pos_min = Point3::new(rx, ry, rz);
        let tot_size = size * size * size;

        Chunk {
            tot_size,
            size,
            v: vec![reg.key_from_string_id(TRANSPARENT_VOXEL); tot_size as usize],
            render_v: vec![0; tot_size as usize],
            sunlight: vec![0; tot_size as usize],
            torchlight: vec![0; tot_size as usize],
            pos: Point3::new(x, y, z),
            world_pos_min,
            world_pos_max: world_pos_min + Vector3::new(size as f32, size as f32, size as f32),
            update: true,
            update_lighting: true,
        }
    }

    pub fn render(&self) -> Vec<f32> {
        let mut out = Vec::new();
        for x in 0..self.size as i32 {
            for y in 0..self.size as i32 {
                for z in 0..self.size as i32 {
                    let i = self.calc_idx(x as f32, y as f32, z as f32);
                    if self.render_v[i as usize] != INVALID_VOXEL_ID {
                        out.push(self.world_pos_min.x + x as f32);
                        out.push(self.world_pos_min.y + y as f32);
                        out.push(self.world_pos_min.z + z as f32);
                    }
                }
            }
        }
        out
    }

    pub fn in_chunk(&self, pos: Point3<f32>) -> bool {
        return self.world_pos_min.x <= pos.x
            && pos.x < self.world_pos_max.x
            && self.world_pos_min.y <= pos.y
            && pos.y < self.world_pos_max.y
            && self.world_pos_min.z <= pos.z
            && pos.z < self.world_pos_max.z;
    }

    pub fn set_voxel(&mut self, voxel_type: u64, world_pos: Point3<f32>) {
        let in_chunk_pos = self.in_chunk_pos(world_pos);
        let idx = self.calc_idx_point(in_chunk_pos);
        self.set_voxel_idx(idx as usize, voxel_type);
    }

    pub fn sunlight_at(&self, idx: usize) -> u8 {
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

    pub fn calc_idx_vect(&self, vect: Vector3<f32>) -> usize {
        self.calc_idx(vect.x, vect.y, vect.z)
    }

    pub fn calc_idx_point(&self, point: Point3<f32>) -> usize {
        self.calc_idx(point.x, point.y, point.z)
    }

    pub fn calc_idx(&self, x: f32, y: f32, z: f32) -> usize {
        ((self.size as f32 * self.size as f32 * z) + (self.size as f32 * y) + x) as usize
    }

    pub fn calc_idx_world(&self, world_pos: Point3<f32>) -> usize {
        let in_chunk = self.in_chunk_pos(world_pos);
        self.calc_idx_point(in_chunk)
    }

    pub fn in_chunk_pos(&self, voxel_pos: Point3<f32>) -> Point3<f32> {
        return Point3::from_vec(voxel_pos - self.world_pos_min);
    }

    pub fn voxel_to_world_pos(&self, pos: Point3<f32>) -> Point3<f32> {
        Point3::new(
            pos.x + self.world_pos_min.x,
            pos.y + self.world_pos_min.y,
            pos.z + self.world_pos_min.z,
        )
    }

    pub fn check_voxel_transparency(&self, pos: Point3<f32>, reg: &VoxelReg) -> bool {
        let in_chunk_pos = pos - self.world_pos_min;
        let p = Point3::new(in_chunk_pos.x, in_chunk_pos.y, in_chunk_pos.z);
        self.check_voxel_in_chunk_transparency(p, reg)
    }

    pub fn check_voxel_in_chunk_transparency(&self, pos: Point3<f32>, reg: &VoxelReg) -> bool {
        let idx = self.calc_idx_point(pos);
        let vox_type = self.v[idx as usize];
        let vox_attr = reg.voxel_attributes(&vox_type);
        vox_attr.transparent
    }

    pub fn is_surface(&self) -> bool {
        self.pos.y >= 0.0
    }
}
