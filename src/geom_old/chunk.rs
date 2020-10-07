use super::util;
use super::ChunkKey;

use crate::VoxelReg;

use glm::Vec3;

use flamer::flame;

#[derive(Debug)]
pub struct Chunk {
    v: Vec<u64>,
    render_data: Vec<f32>,
    pub world_pos_min: Vec3,

    transparent_north: bool,
    transparent_east: bool,
    transparent_south: bool,
    transparent_west: bool,
    transparent_up: bool,
    transparent_down: bool,
}

impl Chunk {
    #[flame("Chunk")]
    pub fn new(size: usize, key: &ChunkKey, v: Vec<u64>, vox_reg: &VoxelReg) -> Chunk {
        let rx = (key.x * size as i32) as f32;
        let ry = (key.y * size as i32) as f32;
        let rz = (key.z * size as i32) as f32;

        let world_pos_min = Vec3::new(rx, ry, rz);

        let mut c = Chunk {
            v,
            world_pos_min,
            render_data: Vec::new(),

            transparent_north: true,
            transparent_east: true,
            transparent_south: true,
            transparent_west: true,
            transparent_up: true,
            transparent_down: true,
        };

        for i in 0..c.v.len() {
            let pos = util::idx_to_pos(i, size);
            let vox_type = c.v[i];
            c.update_transparency(&vox_type, &pos, size, vox_reg)
        }
        c
    }

    #[flame("Chunk")]
    pub fn set_render_data(&mut self, render_data: Vec<f32>) {
        self.render_data = render_data;
    }

    #[flame("Chunk")]
    pub fn get_render_date(&self) -> &Vec<f32> {
        &self.render_data
    }

    #[flame("Chunk")]
    fn calc_idx_point(&self, point: &Vec3, chunk_size: usize) -> usize {
        super::calc_idx(
            point.x as usize,
            point.y as usize,
            point.z as usize,
            chunk_size,
        )
    }

    #[flame("Chunk")]
    pub fn voxel_to_world_pos(&self, pos: &Vec3) -> Vec3 {
        pos + self.world_pos_min
    }

    #[flame("Chunk")]
    pub fn check_voxel_transparency(&self, pos: &Vec3, reg: &VoxelReg, chunk_size: usize) -> bool {
        let in_chunk_pos = pos - self.world_pos_min;
        self.check_voxel_in_chunk_transparency(&in_chunk_pos, reg, chunk_size)
    }

    #[flame("Chunk")]
    pub fn check_voxel_in_chunk_transparency(
        &self,
        pos: &Vec3,
        reg: &VoxelReg,
        chunk_size: usize,
    ) -> bool {
        let idx = self.calc_idx_point(pos, chunk_size);
        self.check_voxel_in_chunk_transparency_idx(idx, reg)
    }

    #[flame("Chunk")]
    pub fn check_voxel_in_chunk_transparency_idx(&self, idx: usize, reg: &VoxelReg) -> bool {
        let vox_type = self.v[idx as usize];
        reg.is_transparent(&vox_type)
    }

    //Norm is the normal key (see normals() in geom::utils) used to generate the the key to find this chunk
    #[flame("Chunk")]
    pub fn is_transparent(&self, norm: i32) -> bool {
        match norm {
            0 => self.transparent_west,
            1 => self.transparent_east,
            2 => self.transparent_down,
            3 => self.transparent_up,
            4 => self.transparent_south,
            5 => self.transparent_north,
            _ => panic!("Not valid use"),
        }
    }

    #[flame("Chunk")]
    fn update_transparency(
        &mut self,
        voxel_type: &u64,
        in_chunk_pos: &Vec3,
        chunk_size: usize,
        vox_reg: &VoxelReg,
    ) {
        if !vox_reg.is_transparent(voxel_type) {
            let size = (chunk_size - 1) as f32;
            if in_chunk_pos.x == 0.0 {
                let mut t = false;
                'outer_x_1: for y in 0..chunk_size {
                    for z in 0..chunk_size {
                        let idx = super::calc_idx(0, y, z, chunk_size);
                        if self.check_voxel_in_chunk_transparency_idx(idx, vox_reg) {
                            t = true;
                            break 'outer_x_1;
                        }
                    }
                }
                self.transparent_west = t;
            } else if in_chunk_pos.x == size {
                let mut t = false;
                'outer_x_2: for y in 0..chunk_size {
                    for z in 0..chunk_size {
                        let idx = super::calc_idx(size as usize, y, z, chunk_size);
                        if self.check_voxel_in_chunk_transparency_idx(idx, vox_reg) {
                            t = true;
                            break 'outer_x_2;
                        }
                    }
                }
                self.transparent_east = t;
            }

            if in_chunk_pos.y == 0.0 {
                let mut t = false;
                'outer_y_1: for x in 0..chunk_size {
                    for z in 0..chunk_size {
                        let idx = super::calc_idx(x, 0, z, chunk_size);
                        if self.check_voxel_in_chunk_transparency_idx(idx, vox_reg) {
                            t = true;
                            break 'outer_y_1;
                        }
                    }
                }
                self.transparent_down = t;
            } else if in_chunk_pos.y == size {
                let mut t = false;
                'outer_y_2: for x in 0..chunk_size {
                    for z in 0..chunk_size {
                        let idx = super::calc_idx(x, size as usize, z, chunk_size);
                        if self.check_voxel_in_chunk_transparency_idx(idx, vox_reg) {
                            t = true;
                            break 'outer_y_2;
                        }
                    }
                }
                self.transparent_up = t;
            }

            if in_chunk_pos.z == 0.0 {
                let mut t = false;
                'outer_z_1: for y in 0..chunk_size {
                    for x in 0..chunk_size {
                        let idx = super::calc_idx(x, y, 0, chunk_size);
                        if self.check_voxel_in_chunk_transparency_idx(idx, vox_reg) {
                            t = true;
                            break 'outer_z_1;
                        }
                    }
                }
                self.transparent_south = t;
            } else if in_chunk_pos.z == size {
                let mut t = false;
                'outer_z_2: for y in 0..chunk_size {
                    for x in 0..chunk_size {
                        let idx = super::calc_idx(x, y, size as usize, chunk_size);
                        if self.check_voxel_in_chunk_transparency_idx(idx, vox_reg) {
                            t = true;
                            break 'outer_z_2;
                        }
                    }
                }
                self.transparent_north = t;
            }
        } else {
            let size = (chunk_size - 1) as f32;
            if in_chunk_pos.x == 0.0 {
                self.transparent_west = true;
            } else if in_chunk_pos.x == size {
                self.transparent_east = true;
            }

            if in_chunk_pos.y == 0.0 {
                self.transparent_down = true;
            } else if in_chunk_pos.y == size {
                self.transparent_up = true;
            }

            if in_chunk_pos.z == 0.0 {
                self.transparent_south = true;
            } else if in_chunk_pos.z == size {
                self.transparent_north = true;
            }
        }
    }

    #[flame("Chunk")]
    pub fn voxel_pos_in_chunk(&self, pos: &Vec3, chunk_size: usize) -> bool {
        let size = chunk_size as f32;
        if pos.x >= size || pos.x < 0.0 {
            return false;
        } else if pos.y >= size || pos.y < 0.0 {
            return false;
        } else if pos.z >= size || pos.z < 0.0 {
            return false;
        } else {
            true
        }
    }
}
