use super::consts::MAX_LIGHT;
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
struct LightNode {
    world_pos: Point3<f32>,
    idx: usize,
    chunk: ChunkKey,
}

#[derive(Debug)]
struct LightRemovalNode {
    world_pos: Point3<f32>,
    idx: usize,
    chunk: ChunkKey,
    lightlevel: u8,
}

#[derive(Debug)]
pub struct PointCloud {
    pub c: HashMap<ChunkKey, Chunk>,
    pub chunk_size: f32,
    keys: Vec<ChunkKey>,
    light_queue: Vec<LightNode>,
    remove_light_queue: Vec<LightRemovalNode>,
    sun_light_queue: Vec<LightNode>,
}

impl PointCloud {
    pub fn new(chunk_size: f32) -> PointCloud {
        return PointCloud {
            c: HashMap::new(),
            chunk_size,
            keys: Vec::new(),
            light_queue: Vec::new(),
            remove_light_queue: Vec::new(),
            sun_light_queue: Vec::new(),
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

    pub fn init_sunlight(&mut self, key: ChunkKey, reg: &VoxelReg) {
        let mut c = self.c.get_mut(&key).unwrap();
        let top_pos = c.pos + NORMALS[2];
        let top_key = ChunkKey::new(top_pos);
        if self.keys.contains(&top_key) {
            let top = self.c.get_mut(&top_key).unwrap();
            for i in 0..top.tot_size as usize {
                if top.sunlight[i] != 0 {
                    let pos = util::idx_to_pos(i, top.size);
                    let world_pos = top.voxel_to_world_pos(pos);
                    self.sun_light_queue.push(LightNode {
                        idx: i,
                        chunk: top_key,
                        world_pos,
                    })
                }
            }
        } else if c.is_surface() {
            let y = c.size - 1.0;
            for x in 0..c.size as i32 {
                for z in 0..c.size as i32 {
                    let idx = c.calc_idx(x as f32, y as f32, z as f32);
                    let pos = Point3::new(x as f32, y as f32, z as f32);
                    if c.check_voxel_in_chunk_transparency(pos, reg) {
                        c.set_sunlight_at(idx, MAX_LIGHT);
                        let world_pos = c.voxel_to_world_pos(pos);
                        self.sun_light_queue.push(LightNode {
                            idx,
                            chunk: key,
                            world_pos,
                        });
                    }
                }
            }
        }
    }

    pub fn update_sunlight(&mut self, reg: &VoxelReg) {}

    pub fn add_torch_lighting(&mut self, reg: &VoxelReg) {
        while !self.light_queue.is_empty() {
            let node = self.light_queue.pop().unwrap();
            let lightlevel = self.c[&node.chunk].torchlight_at(node.idx);

            for norm in &NORMALS {
                let n_pos = node.world_pos + norm;
                let key = self.voxel_to_chunk_key(n_pos);
                if self.c[&key].check_voxel_transparency(n_pos, reg) {
                    let idx = self.c[&key].calc_idx_point(n_pos) as usize;
                    if self.c[&key].torchlight_at(idx) + 1 < lightlevel {
                        self.c
                            .get_mut(&key)
                            .unwrap()
                            .set_torchlight_at(idx, lightlevel - 1);
                    }
                }
            }
        }
    }

    pub fn remove_torch_lighting(&mut self) {
        while !self.remove_light_queue.is_empty() {
            let node = self.remove_light_queue.pop().unwrap();
            let lightlevel = node.lightlevel;

            for norm in &NORMALS {
                let n_pos = node.world_pos + norm;
                let key = self.voxel_to_chunk_key(n_pos);
                let idx = self.c[&key].calc_idx_world(n_pos);
                let n_level = self.c[&key].torchlight_at(idx);
                if n_level != 0 && n_level < lightlevel {
                    self.remove_torchlight_at(n_pos);
                } else if n_level >= lightlevel {
                    self.light_queue.push(LightNode {
                        world_pos: n_pos,
                        idx,
                        chunk: key,
                    });
                }
            }
        }
    }

    pub fn update_lighting(&mut self, reg: &VoxelReg) {
        self.remove_torch_lighting();
        self.add_torch_lighting(reg);
    }

    pub fn voxel_to_chunk_key(&self, pos: Point3<f32>) -> ChunkKey {
        ChunkKey::new(util::voxel_to_chunk_pos(pos, self.chunk_size))
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

    pub fn render(&self) -> Vec<ChunkKey> {
        self.keys.clone()
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

    pub fn remove_torchlight_at(&mut self, pos: Point3<f32>) {
        let key = self.voxel_to_chunk_key(pos);
        if self.c.contains_key(&key) {
            let idx = self.c[&key].calc_idx_world(pos);
            let val = self.c[&key].torchlight_at(idx);
            self.remove_light_queue.push(LightRemovalNode {
                world_pos: pos,
                idx,
                chunk: key,
                lightlevel: val,
            });
            self.c.get_mut(&key).unwrap().set_torchlight_at(idx, 0);
        }
    }

    pub fn set_voxel(&mut self, v: u64, pos: Point3<f32>, reg: &VoxelReg) {
        let chunk_pos = util::voxel_to_chunk_pos(pos, self.chunk_size);
        let key = self.new_chunk_point(chunk_pos, reg);
        self.c.get_mut(&key).unwrap().set_voxel(v, pos);
    }
}
