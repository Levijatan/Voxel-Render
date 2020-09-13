use super::consts::NORMALS;
use super::Chunk;

use super::util;
use crate::consts::OPAQUE_VOXEL;
use crate::VoxelReg;

use cgmath::Point3;
use std::collections::HashMap;
use std::fmt;

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
    pub fn new(point: Point3<f32>) -> ChunkKey {
        return ChunkKey {
            x: point.x as i32,
            y: point.y as i32,
            z: point.z as i32,
        };
    }
}

#[derive(Debug)]
pub struct PointCloud {
    pub c: HashMap<ChunkKey, Chunk>,
    pub chunk_size: f32,
    keys: Vec<ChunkKey>,
}

impl PointCloud {
    pub fn new(chunk_size: f32) -> PointCloud {
        return PointCloud {
            c: HashMap::new(),
            chunk_size,
            keys: Vec::new(),
        };
    }

    pub fn new_chunk_point(&mut self, point: Point3<f32>, reg: &VoxelReg) -> ChunkKey {
        return self.new_chunk(point.x, point.y, point.z, reg);
    }

    pub fn new_chunk(&mut self, x: f32, y: f32, z: f32, reg: &VoxelReg) -> ChunkKey {
        self.keys.sort();
        let key = ChunkKey::new(Point3::new(x, y, z));
        self.c
            .entry(key)
            .or_insert(Chunk::new(self.chunk_size, x, y, z, reg));
        if !self.keys.contains(&key) {
            self.keys.push(key);
        }
        return key;
    }

    pub fn update(&mut self, reg: &VoxelReg) {
        for key in self.keys.iter() {
            if self.c[key].update {
                for x in 0..self.chunk_size as i32 {
                    for y in 0..self.chunk_size as i32 {
                        for z in 0..self.chunk_size as i32 {
                            let idx = self.c[key].calc_idx(x as f32, y as f32, z as f32);
                            let mut vox_type = self.c[key].v[idx as usize];
                            let vox_attr = reg.voxel_attributes(&vox_type);
                            let vox_chunk_pos = Point3::new(x as f32, y as f32, z as f32);
                            let vox_world_pos = self.c[key].voxel_to_world_pos(vox_chunk_pos);

                            let mut render = false;
                            if !vox_attr.transparent {
                                for norm in &NORMALS {
                                    let voxel_neigh = vox_world_pos + norm;

                                    if self.c[key].in_chunk(voxel_neigh) {
                                        render =
                                            self.c[key].check_voxel_transparency(voxel_neigh, reg);
                                    } else {
                                        let chunk_neigh = self.c[key].pos + norm;
                                        let k = ChunkKey::new(chunk_neigh);

                                        if self.c.contains_key(&k) {
                                            render = self.c[&k]
                                                .check_voxel_transparency(voxel_neigh, reg);
                                        } else if norm.y == 1.0 {
                                            render = true;
                                        }
                                    }

                                    if render {
                                        break;
                                    }
                                }
                            }
                            if !render {
                                vox_type = 0;
                            }
                            self.c.get_mut(key).unwrap().render_v[idx as usize] = vox_type;
                        }
                    }
                }
                self.c.get_mut(key).unwrap().update = false;
            }
        }
    }

    pub fn render(&self) -> Vec<&Chunk> {
        self.c.values().collect()
    }

    pub fn create_cube(&mut self, start: Point3<f32>, stop: Point3<f32>, reg: &VoxelReg) {
        let (t_start, t_stop) = util::check_start_stop_to_i32(start, stop);

        let voxel_type = reg.key_from_string_id(OPAQUE_VOXEL);
        for x in t_start.x..t_stop.x {
            for y in t_start.y..t_stop.y {
                for z in t_start.z..t_stop.z {
                    self.set_voxel(voxel_type, Point3::new(x as f32, y as f32, z as f32), reg);
                }
            }
        }
    }

    pub fn set_voxel(&mut self, v: u64, pos: Point3<f32>, reg: &VoxelReg) {
        let chunk_pos = util::voxel_to_chunk_pos(pos, self.chunk_size);
        let key = self.new_chunk_point(chunk_pos, reg);
        self.c.get_mut(&key).unwrap().set_voxel(v, pos);
    }
}
