use super::chunk_gen::ChunkNode;
use super::chunk_gen::GenNode;
use super::consts::OPAQUE_VOXEL;
use super::geom::calc_idx;
use super::geom::Chunk;
use super::geom::ChunkKey;
use super::geom::PointCloud;
use super::render::Camera;
use super::render::ChunkRender;
use super::VoxelReg;

use glm::{distance, Vec3};
use std::collections::HashMap;
use std::marker::Send;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;

pub trait WorldType: Send + Sync {
    fn gen_chunk(&self, pos: Vec3, reg: &VoxelReg) -> Chunk;
    fn world_type(&self) -> &'static str;
}

pub struct FlatWorldType {
    pub chunk_size: usize,
}

impl WorldType for FlatWorldType {
    fn gen_chunk(&self, pos: Vec3, reg: &VoxelReg) -> Chunk {
        let mut c = Chunk::new(self.chunk_size, pos.x, pos.y, pos.z, reg);
        let voxel_type = reg.key_from_string_id(OPAQUE_VOXEL);
        if pos.y == 0.0 {
            for x in 0..self.chunk_size {
                for z in 0..self.chunk_size {
                    let idx = calc_idx(x, 0, z, self.chunk_size);
                    c.set_voxel_idx(idx, voxel_type);
                }
            }
        } else if pos.y < 0.0 {
            for i in 0..c.tot_size {
                c.set_voxel_idx(i, voxel_type);
            }
        }
        c
    }

    fn world_type(&self) -> &'static str {
        "FlatWorldType"
    }
}

pub struct World {
    pub pc: PointCloud,
    pub world_type: u64,
    pub active: bool,
    chunk_radius: f32,
    old_cam_pos: Vec3,
}

impl World {
    pub fn new(active: bool, chunk_size: usize, world_type: u64, chunk_radius: f32) -> World {
        World {
            pc: PointCloud::new(chunk_size),
            world_type,
            active,
            chunk_radius,
            old_cam_pos: Vec3::new(99.0, 99.0, 99.0),
        }
    }

    pub fn update(&mut self, reg: &VoxelReg) {
        self.pc.update(reg);
    }

    pub fn render(&mut self, cam: &Camera, renderer: &mut ChunkRender) {
        let cam_chunk_pos = super::geom::voxel_to_chunk_pos(cam.pos, self.pc.chunk_size);
        let max_x = (cam_chunk_pos.x + self.chunk_radius) as i32;
        let max_y = (cam_chunk_pos.y + self.chunk_radius) as i32;
        let max_z = (cam_chunk_pos.z + self.chunk_radius) as i32;
        let min_x = (cam_chunk_pos.x - self.chunk_radius) as i32;
        let min_y = (cam_chunk_pos.y - self.chunk_radius) as i32;
        let min_z = (cam_chunk_pos.z - self.chunk_radius) as i32;
        for y in min_y..max_y {
            for x in min_x..max_x {
                for z in min_z..max_z {
                    let key = ChunkKey { x, y, z };
                    if self.pc.c.contains_key(&key) && !self.pc.c[&key].in_queue {
                        renderer.add_to_queue(key, &mut self.pc);
                    }
                }
            }
        }
    }

    pub fn check_for_new_chunks(&mut self, cam: &Camera, tx: &Sender<GenNode>, world_id: u64) {
        let cam_chunk_pos = super::geom::voxel_to_chunk_pos(cam.pos, self.pc.chunk_size);
        if self.old_cam_pos != cam_chunk_pos {
            self.old_cam_pos = cam_chunk_pos;
            let max_x = (cam_chunk_pos.x + self.chunk_radius) as i32;
            let max_y = (cam_chunk_pos.y + self.chunk_radius) as i32;
            let max_z = (cam_chunk_pos.z + self.chunk_radius) as i32;
            let min_x = (cam_chunk_pos.x - self.chunk_radius) as i32;
            let min_y = (cam_chunk_pos.y - self.chunk_radius) as i32;
            let min_z = (cam_chunk_pos.z - self.chunk_radius) as i32;
            for y in min_y..max_y {
                for x in min_x..max_x {
                    for z in min_z..max_z {
                        let key = ChunkKey { x, y, z };
                        if !self.pc.keys.contains(&key) {
                            self.pc.keys.push(key);
                            let pos = Vec3::new(x as f32, y as f32, z as f32);
                            let gen = GenNode {
                                priority: distance(&cam_chunk_pos, &pos) as i32,
                                world_id,
                                world_type: self.world_type,
                                pos,
                            };

                            tx.send(gen).unwrap();
                        }
                    }
                }
            }
        }
    }
}

pub struct WorldTypeRegistry {
    pub world_type_reg: HashMap<u64, Box<dyn WorldType>>,
    next_type_key: u64,
}

impl WorldTypeRegistry {
    pub fn new() -> WorldTypeRegistry {
        WorldTypeRegistry {
            world_type_reg: HashMap::new(),
            next_type_key: 1,
        }
    }

    pub fn register_world_type(&mut self, world_type: Box<dyn WorldType>) -> u64 {
        let id = self.get_next_type_key();
        self.world_type_reg.insert(id, world_type);
        id
    }

    fn get_next_type_key(&mut self) -> u64 {
        let out = self.next_type_key;
        self.next_type_key += 1;
        out
    }
}

pub struct WorldRegistry {
    world_reg: HashMap<u64, World>,
    next_world_key: u64,
}

impl WorldRegistry {
    pub fn new() -> WorldRegistry {
        WorldRegistry {
            world_reg: HashMap::new(),
            next_world_key: 1,
        }
    }

    pub fn new_world(&mut self, world: World) {
        let id = self.get_next_world_key();
        self.world_reg.insert(id, world);
    }

    pub fn type_of_world(&self, world_id: u64) -> u64 {
        self.world_reg.get(&world_id).unwrap().world_type
    }

    fn get_next_world_key(&mut self) -> u64 {
        let out = self.next_world_key;
        self.next_world_key += 1;
        out
    }

    pub fn world_mut(&mut self, id: &u64) -> &mut World {
        self.world_reg.get_mut(id).unwrap()
    }

    pub fn fetch_chunks_from_gen(&mut self, rx: &Receiver<ChunkNode>, reg: &VoxelReg) {
        match rx.try_recv() {
            Ok(node) => {
                let key = ChunkKey::new(node.chunk.pos);
                self.world_reg
                    .get_mut(&node.world_id)
                    .unwrap()
                    .pc
                    .insert_chunk(key, node.chunk, reg);
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                panic!("What, where is the generator??");
            }
        }
    }
}
