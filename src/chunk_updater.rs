use std::collections::VecDeque;

use super::geom::normals;
use super::geom::ChunkKey;
use super::VoxelReg;
use super::WorldRegistry;

#[derive(Debug, Clone)]
struct ChunkUpdateNode {
    key: ChunkKey,
    world_id: u64,
}

pub struct ChunkUpdater {
    chunk_update_queue: VecDeque<ChunkUpdateNode>,
    voxel_update_queue: VecDeque<ChunkUpdateNode>,
}

impl ChunkUpdater {
    pub fn new() -> ChunkUpdater {
        ChunkUpdater {
            chunk_update_queue: VecDeque::new(),
            voxel_update_queue: VecDeque::new(),
        }
    }

    pub fn add_to_queue(&mut self, key: ChunkKey, world_id: u64) {
        let node = ChunkUpdateNode { key, world_id };
        self.chunk_update_queue.push_back(node);
    }

    pub fn process(&mut self, world_reg: &mut WorldRegistry, voxel_reg: &VoxelReg) {
        self.process_chunk_queue(world_reg);
        self.process_voxel_queue(world_reg, voxel_reg);
    }

    fn process_chunk_queue(&mut self, world_reg: &mut WorldRegistry) {
        while !self.chunk_update_queue.is_empty() {
            let node = self.chunk_update_queue.pop_front().unwrap();
            let mut render = false;
            let world = world_reg.world_mut(&node.world_id);
            let pos = world.pc.chunk_pos(&node.key);
            let mut i = 0;
            while !render && i < 6 {
                let norm = normals(i);
                let n_pos = pos + norm;
                let n_key = ChunkKey::new(n_pos);
                if world.pc.chunk_is_transparent(&n_key, i) {
                    render = true;
                    self.voxel_update_queue.push_back(node.clone());
                }
                i += 1;
            }
            if world.pc.chunk_render(&node.key) != render {
                let mut i = 0;
                while i < 6 {
                    let n_pos = pos + normals(i);
                    let key = ChunkKey::new(n_pos);
                    if world.pc.chunk_exists(&key) {
                        self.add_to_queue(key, node.world_id);
                    }
                    i += 1;
                }
            }
            world.pc.chunk_set_render(&node.key, render);
        }
    }

    fn process_voxel_queue(&mut self, world_reg: &mut WorldRegistry, voxel_reg: &VoxelReg) {
        while !self.voxel_update_queue.is_empty() {
            let node = self.voxel_update_queue.pop_front().unwrap();
            for idx in 0..world_reg.world(&node.world_id).pc.chunk_tot_size() {
                let mut render = false;
                if !world_reg
                    .world(&node.world_id)
                    .pc
                    .voxel_in_chunk_transparency_idx(&node.key, idx, voxel_reg)
                {
                    //let chunk = world_reg.world(&node.world_id).pc.chunk(&node.key);
                    let world = world_reg.world(&node.world_id);
                    let pos = super::geom::idx_to_pos(idx, world.chunk_size());
                    let mut i = 0;
                    while i < 6 && !render {
                        let norm = normals(i);
                        let n_pos = pos + norm;
                        if world.pc.voxel_pos_in_chunk(&node.key, &n_pos) {
                            render = world
                                .pc
                                .voxel_in_chunk_transparency(&node.key, &n_pos, voxel_reg);
                        } else {
                            let n_chunk = world.pc.chunk_pos(&node.key) + norm;
                            let n_key = ChunkKey::new(n_chunk);
                            if world.pc.chunk_exists(&n_key) && !world.pc.chunk_gen(&n_key) {
                                if world.pc.chunk_is_transparent(&n_key, i) {
                                    let n_world_pos = world.pc.voxel_to_world_pos(&n_key, &n_pos);
                                    render = world.pc.voxel_transparency(
                                        &n_world_pos,
                                        &n_key,
                                        voxel_reg,
                                        world.pc.chunk_size(),
                                    );
                                }
                            } else {
                                render = true;
                            }
                        }
                        i += 1;
                    }
                }
                if render {
                    world_reg
                        .world_mut(&node.world_id)
                        .pc
                        .chunk_v_to_render_v(&node.key, idx);
                }
            }
        }
    }
}
