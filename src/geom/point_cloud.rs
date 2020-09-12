use super::consts::NORMALS;
use super::Chunk;
use super::Voxel;

use super::util;

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

    pub fn new_chunk_point(&mut self, point: Point3<f32>) -> ChunkKey {
        return self.new_chunk(point.x, point.y, point.z);
    }

    pub fn new_chunk(&mut self, x: f32, y: f32, z: f32) -> ChunkKey {
        self.keys.sort();
        let key = ChunkKey::new(Point3::new(x, y, z));
        self.c
            .entry(key)
            .or_insert(Chunk::new(self.chunk_size, x, y, z));
        if !self.keys.contains(&key) {
            self.keys.push(key);
        }
        return key;
    }

    pub fn update(&mut self) {
        for key in self.keys.iter() {
            println!("Updating {}", key);
            for i in 0..self.c[key].tot_size as i32 {
                if !self.c[key].v[i as usize].transparent {
                    let mut render = false;
                    for norm in &NORMALS {
                        let voxel_neigh = self.c[key].v[i as usize].pos + norm;
                        let n = self.c[key].chunk_normal_from_voxel_pos(voxel_neigh);
                        let chunk_neigh = self.c[key].pos + n;
                        let k = ChunkKey::new(chunk_neigh);
                        if self.c[key].in_chunk(voxel_neigh) {
                            let in_chunk_pos = voxel_neigh - self.c[key].world_pos_min;
                            let idx = self.c[key].calc_idx_vect(in_chunk_pos);
                            if self.c[key].v[idx as usize].transparent {
                                render = true;
                            }
                        } else if self.c.contains_key(&k) {
                            let n_chunk = self.c.get(&k).unwrap();
                            let in_chunk_pos = voxel_neigh - n_chunk.world_pos_min;
                            let idx = n_chunk.calc_idx_vect(in_chunk_pos);
                            if n_chunk.v[idx as usize].transparent {
                                render = true;
                            }
                        } else if n.y == 1.0 {
                            render = true;
                        }
                    }
                    self.c
                        .get_mut(key)
                        .unwrap()
                        .update_voxel(i as usize, render);
                }
            }
        }
    }

    pub fn create_cube(&mut self, start: Point3<f32>, stop: Point3<f32>) {
        let (t_start, t_stop) = util::check_start_stop_to_i32(start, stop);

        for x in t_start.x..t_stop.x {
            for y in t_start.y..t_stop.y {
                for z in t_start.z..t_stop.z {
                    self.set_voxel(Voxel::new(false, x as f32, y as f32, z as f32));
                }
            }
        }
    }

    pub fn set_voxel(&mut self, v: Voxel) {
        let chunk_pos = util::voxel_to_chunk_pos(v.pos, self.chunk_size);
        let key = self.new_chunk_point(chunk_pos);
        self.c.get_mut(&key).unwrap().set_voxel(v);
    }

    pub fn render(&self) -> Vec<&Chunk> {
        let mut out = Vec::new();
        for val in self.c.values() {
            if val.render {
                out.push(val);
            }
        }
        return out;
    }
}
