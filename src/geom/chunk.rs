use super::util;
use crate::consts::INVALID_VOXEL_ID;
use crate::consts::TRANSPARENT_VOXEL;
use crate::VoxelReg;

use glm::Vec3;

#[derive(Debug)]
pub struct Chunk {
    v: Vec<u64>,
    render_v: Vec<u64>,
    pub pos: Vec3,
    pub world_pos_min: Vec3,
    world_pos_max: Vec3,
    pub in_queue: bool,
    pub render: bool,
    pub rerender: bool,
    pub gen: bool,

    transparent_north: bool,
    transparent_east: bool,
    transparent_south: bool,
    transparent_west: bool,
    transparent_up: bool,
    transparent_down: bool,
}

impl Chunk {
    pub fn new_gen(size: usize, x: f32, y: f32, z: f32) -> Chunk {
        let rx = x * size as f32;
        let ry = y * size as f32;
        let rz = z * size as f32;

        let world_pos_min = Vec3::new(rx, ry, rz);
        let tot_size = size * size * size;

        Chunk {
            v: vec![0; tot_size],
            render_v: vec![0; tot_size],
            pos: Vec3::new(x, y, z),
            world_pos_min,
            world_pos_max: world_pos_min + Vec3::new(size as f32, size as f32, size as f32),
            in_queue: false,
            render: false,
            rerender: false,
            gen: true,

            transparent_north: true,
            transparent_east: true,
            transparent_south: true,
            transparent_west: true,
            transparent_up: true,
            transparent_down: true,
        }
    }

    pub fn new(size: usize, x: f32, y: f32, z: f32, reg: &VoxelReg) -> Chunk {
        let rx = x * size as f32;
        let ry = y * size as f32;
        let rz = z * size as f32;

        let world_pos_min = Vec3::new(rx, ry, rz);
        let tot_size = size * size * size;

        Chunk {
            v: vec![reg.key_from_string_id(TRANSPARENT_VOXEL); tot_size],
            render_v: vec![0; tot_size],
            pos: Vec3::new(x, y, z),
            world_pos_min,
            world_pos_max: world_pos_min + Vec3::new(size as f32, size as f32, size as f32),
            in_queue: false,
            render: false,
            rerender: false,
            gen: false,

            transparent_north: true,
            transparent_east: true,
            transparent_south: true,
            transparent_west: true,
            transparent_up: true,
            transparent_down: true,
        }
    }

    pub fn render(&self, chunk_size: usize) -> Vec<f32> {
        let mut out = Vec::new();
        let tot_size = chunk_size * chunk_size * chunk_size;
        for idx in 0..tot_size {
            let pos = util::idx_to_pos(idx, chunk_size);
            if self.render_v[idx] != INVALID_VOXEL_ID {
                out.push(self.world_pos_min.x + pos.x as f32);
                out.push(self.world_pos_min.y + pos.y as f32);
                out.push(self.world_pos_min.z + pos.z as f32);
            }
        }
        out
    }

    pub fn set_voxel(
        &mut self,
        voxel_type: u64,
        in_chunk_pos: &Vec3,
        vox_reg: &VoxelReg,
        chunk_size: usize,
    ) {
        let idx = self.calc_idx_point(in_chunk_pos, chunk_size);
        self.v[idx] = voxel_type;
        self.update_transparency(&voxel_type, &in_chunk_pos, chunk_size, vox_reg)
    }

    fn calc_idx_point(&self, point: &Vec3, chunk_size: usize) -> usize {
        super::calc_idx(
            point.x as usize,
            point.y as usize,
            point.z as usize,
            chunk_size,
        )
    }

    pub fn voxel_to_world_pos(&self, pos: &Vec3) -> Vec3 {
        pos + self.world_pos_min
    }

    pub fn check_voxel_transparency(&self, pos: &Vec3, reg: &VoxelReg, chunk_size: usize) -> bool {
        let in_chunk_pos = pos - self.world_pos_min;
        self.check_voxel_in_chunk_transparency(&in_chunk_pos, reg, chunk_size)
    }

    pub fn check_voxel_in_chunk_transparency(
        &self,
        pos: &Vec3,
        reg: &VoxelReg,
        chunk_size: usize,
    ) -> bool {
        let idx = self.calc_idx_point(pos, chunk_size);
        self.check_voxel_in_chunk_transparency_idx(idx, reg)
    }

    pub fn check_voxel_in_chunk_transparency_idx(&self, idx: usize, reg: &VoxelReg) -> bool {
        let vox_type = self.v[idx as usize];
        reg.is_transparent(&vox_type)
    }

    //Norm is the normal key (see normals() in geom::utils) used to generate the the key to find this chunk
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

    pub fn v_to_render_v(&mut self, idx: usize) {
        self.rerender = true;
        self.render_v[idx] = self.v[idx];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voxel_pos_in_chunk() {
        let mut reg = VoxelReg::new();
        reg.register_voxel_type(
            crate::consts::TRANSPARENT_VOXEL,
            true,
            crate::voxel_registry::Material {
                ambient: Vec3::new(0.0, 0.0, 0.0),
                diffuse: Vec3::new(0.0, 0.0, 0.0),
                specular: Vec3::new(0.0, 0.0, 0.0),
                shininess: 0.0,
            },
        );
        let size = 16;
        let c = Chunk::new(size, 0.0, 0.0, 0.0, &reg);
        let pos1 = Vec3::new(0.0, 0.0, 0.0);
        let pos2 = Vec3::new(15.0, 15.0, 15.0);
        let pos3 = Vec3::new(16.0, 16.0, 16.0);
        assert!(c.voxel_pos_in_chunk(&pos1, size));
        assert!(c.voxel_pos_in_chunk(&pos2, size));
        assert!(!c.voxel_pos_in_chunk(&pos3, size));
    }

    #[test]
    fn test_set_voxel() {
        let mut reg = VoxelReg::new();
        reg.register_voxel_type(
            crate::consts::TRANSPARENT_VOXEL,
            true,
            crate::voxel_registry::Material {
                ambient: Vec3::new(0.0, 0.0, 0.0),
                diffuse: Vec3::new(0.0, 0.0, 0.0),
                specular: Vec3::new(0.0, 0.0, 0.0),
                shininess: 0.0,
            },
        );

        reg.register_voxel_type(
            crate::consts::OPAQUE_VOXEL,
            false,
            crate::voxel_registry::Material {
                ambient: Vec3::new(1.0, 1.0, 1.0),
                diffuse: Vec3::new(0.8, 0.8, 0.8),
                specular: Vec3::new(0.5, 0.8, 0.1),
                shininess: 0.1,
            },
        );

        let size = 16;

        let mut c = Chunk::new(size, 0.0, 0.0, 0.0, &reg);
        let pos = Vec3::new(0.0, 0.0, 0.0);
        let voxel_type = reg.key_from_string_id(crate::consts::OPAQUE_VOXEL);
        c.set_voxel(voxel_type, &pos, &reg, size);
        assert_eq!(c.v[0], voxel_type);
    }

    #[test]
    fn test_update_transparency() {
        let mut reg = VoxelReg::new();
        reg.register_voxel_type(
            crate::consts::TRANSPARENT_VOXEL,
            true,
            crate::voxel_registry::Material {
                ambient: Vec3::new(0.0, 0.0, 0.0),
                diffuse: Vec3::new(0.0, 0.0, 0.0),
                specular: Vec3::new(0.0, 0.0, 0.0),
                shininess: 0.0,
            },
        );

        reg.register_voxel_type(
            crate::consts::OPAQUE_VOXEL,
            false,
            crate::voxel_registry::Material {
                ambient: Vec3::new(1.0, 1.0, 1.0),
                diffuse: Vec3::new(0.8, 0.8, 0.8),
                specular: Vec3::new(0.5, 0.8, 0.1),
                shininess: 0.1,
            },
        );
        let size = 16;
        let mut c = Chunk::new(size, 0.0, 0.0, 0.0, &reg);

        assert!(c.transparent_west);
        assert!(c.transparent_east);
        assert!(c.transparent_up);
        assert!(c.transparent_down);
        assert!(c.transparent_north);
        assert!(c.transparent_south);

        let mut voxel_type = reg.key_from_string_id(crate::consts::OPAQUE_VOXEL);
        let x = 0.0;
        for y in 0..size {
            for z in 0..size {
                let pos = Vec3::new(x, y as f32, z as f32);
                c.set_voxel(voxel_type, &pos, &reg, size);
            }
        }

        assert!(!c.transparent_west);
        assert!(c.transparent_east);
        assert!(c.transparent_up);
        assert!(c.transparent_down);
        assert!(c.transparent_north);
        assert!(c.transparent_south);

        let x = 15.0;
        for y in 0..size {
            for z in 0..size {
                let pos = Vec3::new(x, y as f32, z as f32);
                c.set_voxel(voxel_type, &pos, &reg, size);
            }
        }

        assert!(!c.transparent_west);
        assert!(!c.transparent_east);
        assert!(c.transparent_up);
        assert!(c.transparent_down);
        assert!(c.transparent_north);
        assert!(c.transparent_south);

        let y = 0;
        for x in 0..size {
            for z in 0..size {
                let pos = Vec3::new(x as f32, y as f32, z as f32);
                c.set_voxel(voxel_type, &pos, &reg, size);
            }
        }

        assert!(!c.transparent_west);
        assert!(!c.transparent_east);
        assert!(c.transparent_up);
        assert!(!c.transparent_down);
        assert!(c.transparent_north);
        assert!(c.transparent_south);

        let y = 15;
        for x in 0..size {
            for z in 0..size {
                let pos = Vec3::new(x as f32, y as f32, z as f32);
                c.set_voxel(voxel_type, &pos, &reg, size);
            }
        }

        assert!(!c.transparent_west);
        assert!(!c.transparent_east);
        assert!(!c.transparent_up);
        assert!(!c.transparent_down);
        assert!(c.transparent_north);
        assert!(c.transparent_south);

        let z = 0;
        for x in 0..size {
            for y in 0..size {
                let pos = Vec3::new(x as f32, y as f32, z as f32);
                c.set_voxel(voxel_type, &pos, &reg, size);
            }
        }

        assert!(!c.transparent_west);
        assert!(!c.transparent_east);
        assert!(!c.transparent_up);
        assert!(!c.transparent_down);
        assert!(c.transparent_north);
        assert!(!c.transparent_south);

        let z = 15;
        for x in 0..size {
            for y in 0..size {
                let pos = Vec3::new(x as f32, y as f32, z as f32);
                c.set_voxel(voxel_type, &pos, &reg, size);
            }
        }

        assert!(!c.transparent_west);
        assert!(!c.transparent_east);
        assert!(!c.transparent_up);
        assert!(!c.transparent_down);
        assert!(!c.transparent_north);
        assert!(!c.transparent_south);

        voxel_type = reg.key_from_string_id(crate::consts::TRANSPARENT_VOXEL);

        let mut pos = Vec3::new(8.0, 8.0, 8.0);

        assert!(!c.transparent_west);
        assert!(!c.transparent_east);
        assert!(!c.transparent_up);
        assert!(!c.transparent_down);
        assert!(!c.transparent_north);
        assert!(!c.transparent_south);

        pos.x = 0.0;

        c.set_voxel(voxel_type, &pos, &reg, size);

        assert!(c.transparent_west);
        assert!(!c.transparent_east);
        assert!(!c.transparent_up);
        assert!(!c.transparent_down);
        assert!(!c.transparent_north);
        assert!(!c.transparent_south);

        pos.x = 15.0;

        c.set_voxel(voxel_type, &pos, &reg, size);

        assert!(c.transparent_west);
        assert!(c.transparent_east);
        assert!(!c.transparent_up);
        assert!(!c.transparent_down);
        assert!(!c.transparent_north);
        assert!(!c.transparent_south);

        pos.x = 8.0;
        pos.y = 0.0;

        c.set_voxel(voxel_type, &pos, &reg, size);

        assert!(c.transparent_west);
        assert!(c.transparent_east);
        assert!(!c.transparent_up);
        assert!(c.transparent_down);
        assert!(!c.transparent_north);
        assert!(!c.transparent_south);

        pos.y = 15.0;

        c.set_voxel(voxel_type, &pos, &reg, size);

        assert!(c.transparent_west);
        assert!(c.transparent_east);
        assert!(c.transparent_up);
        assert!(c.transparent_down);
        assert!(!c.transparent_north);
        assert!(!c.transparent_south);

        pos.y = 8.0;
        pos.z = 0.0;

        c.set_voxel(voxel_type, &pos, &reg, size);

        assert!(c.transparent_west);
        assert!(c.transparent_east);
        assert!(c.transparent_up);
        assert!(c.transparent_down);
        assert!(!c.transparent_north);
        assert!(c.transparent_south);

        pos.z = 15.0;

        c.set_voxel(voxel_type, &pos, &reg, size);

        assert!(c.transparent_west);
        assert!(c.transparent_east);
        assert!(c.transparent_up);
        assert!(c.transparent_down);
        assert!(c.transparent_north);
        assert!(c.transparent_south);
    }

    #[test]
    fn test_is_transparent() {
        let mut reg = VoxelReg::new();
        reg.register_voxel_type(
            crate::consts::TRANSPARENT_VOXEL,
            true,
            crate::voxel_registry::Material {
                ambient: Vec3::new(0.0, 0.0, 0.0),
                diffuse: Vec3::new(0.0, 0.0, 0.0),
                specular: Vec3::new(0.0, 0.0, 0.0),
                shininess: 0.0,
            },
        );

        reg.register_voxel_type(
            crate::consts::OPAQUE_VOXEL,
            false,
            crate::voxel_registry::Material {
                ambient: Vec3::new(1.0, 1.0, 1.0),
                diffuse: Vec3::new(0.8, 0.8, 0.8),
                specular: Vec3::new(0.5, 0.8, 0.1),
                shininess: 0.1,
            },
        );
        let size = 16;
        let mut c = Chunk::new(size, 0.0, 0.0, 0.0, &reg);
        let voxel_type = reg.key_from_string_id(crate::consts::OPAQUE_VOXEL);

        let y = 0;
        for x in 0..size {
            for z in 0..size {
                let pos = Vec3::new(x as f32, y as f32, z as f32);
                let pos2 = Vec3::new(y as f32, x as f32, z as f32);
                let pos3 = Vec3::new(x as f32, z as f32, y as f32);
                c.set_voxel(voxel_type, &pos, &reg, size);
                c.set_voxel(voxel_type, &pos2, &reg, size);
                c.set_voxel(voxel_type, &pos3, &reg, size);
            }
        }

        assert!(!c.is_transparent(0));
        assert!(c.is_transparent(1));
        assert!(!c.is_transparent(2));
        assert!(c.is_transparent(3));
        assert!(!c.is_transparent(4));
        assert!(c.is_transparent(5));
    }

    #[test]
    fn test_v_to_render_v() {
        let mut reg = VoxelReg::new();
        reg.register_voxel_type(
            crate::consts::TRANSPARENT_VOXEL,
            true,
            crate::voxel_registry::Material {
                ambient: Vec3::new(0.0, 0.0, 0.0),
                diffuse: Vec3::new(0.0, 0.0, 0.0),
                specular: Vec3::new(0.0, 0.0, 0.0),
                shininess: 0.0,
            },
        );

        reg.register_voxel_type(
            crate::consts::OPAQUE_VOXEL,
            false,
            crate::voxel_registry::Material {
                ambient: Vec3::new(1.0, 1.0, 1.0),
                diffuse: Vec3::new(0.8, 0.8, 0.8),
                specular: Vec3::new(0.5, 0.8, 0.1),
                shininess: 0.1,
            },
        );

        let chunk_size = 16;

        let mut c = Chunk::new(chunk_size, 0.0, 0.0, 0.0, &reg);
        let voxel_type = reg.key_from_string_id(crate::consts::OPAQUE_VOXEL);
        c.set_voxel(voxel_type, &Vec3::new(0.0, 0.0, 0.0), &reg, chunk_size);
        assert!(!c.rerender);
        c.v_to_render_v(0);
        assert!(c.rerender);

        assert_eq!(c.v[0], c.render_v[0]);

        assert_eq!(c.render_v[1], 0);
    }

    #[test]
    fn test_render() {
        let mut reg = VoxelReg::new();
        reg.register_voxel_type(
            crate::consts::TRANSPARENT_VOXEL,
            true,
            crate::voxel_registry::Material {
                ambient: Vec3::new(0.0, 0.0, 0.0),
                diffuse: Vec3::new(0.0, 0.0, 0.0),
                specular: Vec3::new(0.0, 0.0, 0.0),
                shininess: 0.0,
            },
        );

        reg.register_voxel_type(
            crate::consts::OPAQUE_VOXEL,
            false,
            crate::voxel_registry::Material {
                ambient: Vec3::new(1.0, 1.0, 1.0),
                diffuse: Vec3::new(0.8, 0.8, 0.8),
                specular: Vec3::new(0.5, 0.8, 0.1),
                shininess: 0.1,
            },
        );

        let chunk_size = 16;

        let mut c = Chunk::new(chunk_size, 0.0, 0.0, 0.0, &reg);

        let voxel_type = reg.key_from_string_id(crate::consts::OPAQUE_VOXEL);
        c.set_voxel(voxel_type, &Vec3::new(0.0, 0.0, 0.0), &reg, chunk_size);

        c.v_to_render_v(0);

        let mut d = c.render(chunk_size);

        assert_eq!(d.len(), 3);

        assert_eq!(d.pop().unwrap(), 0.0);
        assert_eq!(d.pop().unwrap(), 0.0);
        assert_eq!(d.pop().unwrap(), 0.0);
    }
}
