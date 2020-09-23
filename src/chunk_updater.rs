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
            let mut visible = false;
            let world = world_reg.world_mut(&node.world_id);
            let pos = world.pc.chunk_pos(&node.key);
            let mut i = 0;
            while !visible && i < 6 {
                let norm = normals(i);
                let n_pos = pos + norm;
                let n_key = ChunkKey::new(n_pos);
                if world.pc.chunk_is_transparent(&n_key, i) {
                    visible = true;
                    self.voxel_update_queue.push_back(node.clone());
                }
                i += 1;
            }

            world.pc.chunk_set_visible(&node.key, visible);
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
                            if world.pc.chunk_exists(&n_key)
                                && world.pc.chunk_is_transparent(&n_key, i)
                            {
                                let n_world_pos = world.pc.voxel_to_world_pos(&node.key, &n_pos);
                                render = world.pc.voxel_transparency(
                                    &n_world_pos,
                                    &n_key,
                                    voxel_reg,
                                    world.pc.chunk_size(),
                                );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::Chunk;
    use crate::World;
    use glm::Vec3;

    #[test]
    fn test_add_to_queue() {
        let mut updater = ChunkUpdater::new();
        assert!(updater.voxel_update_queue.len() == 0);
        assert!(updater.chunk_update_queue.len() == 0);
        let key = ChunkKey { x: 0, y: 0, z: 0 };
        let world_id = 0;
        updater.add_to_queue(key, world_id);
        assert!(updater.chunk_update_queue.len() == 1);
        assert!(updater.voxel_update_queue.len() == 0);

        let node = updater.chunk_update_queue.pop_front().unwrap();
        assert_eq!(key, node.key);
        assert_eq!(world_id, node.world_id);
    }

    #[test]
    fn test_process_chunk_queue_empty() {
        let mut world_reg = WorldRegistry::new();
        let mut updater = ChunkUpdater::new();

        assert!(updater.voxel_update_queue.len() == 0);
        assert!(updater.chunk_update_queue.len() == 0);
        updater.process_chunk_queue(&mut world_reg);
        assert!(updater.voxel_update_queue.len() == 0);
        assert!(updater.chunk_update_queue.len() == 0);
    }

    #[test]
    fn test_process_chunk_queue_one_visible_chunk_empty() {
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

        let mut world_reg = WorldRegistry::new();
        let chunk_size = 16;
        let mut world = World::new(true, chunk_size, 0, 8.0);
        let key = ChunkKey { x: 0, y: 0, z: 0 };
        let chunk = Chunk::new(chunk_size, 0.0, 0.0, 0.0, &reg);
        world.pc.insert_chunk(key, chunk);
        world_reg.new_world(world);
        let mut updater = ChunkUpdater::new();
        updater.add_to_queue(key, 1);
        assert!(world_reg.world(&1).pc.chunk_is_visible(&key));
        assert!(updater.chunk_update_queue.len() == 1);
        assert!(updater.voxel_update_queue.len() == 0);
        updater.process_chunk_queue(&mut world_reg);
        assert!(world_reg.world(&1).pc.chunk_is_visible(&key));
        assert!(updater.chunk_update_queue.len() == 0);
        assert!(updater.voxel_update_queue.len() == 1);
    }
}
