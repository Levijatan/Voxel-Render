use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};
use std::sync::mpsc::Receiver;
use std::thread;

use super::geom::Chunk;
use super::geom::ChunkKey;

#[derive(Copy, Clone, Debug)]
pub struct GenNode {
    pub priority: u32,
    pub world_id: u64,
    pub key: ChunkKey,
}

impl Ord for GenNode {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority.cmp(&other.priority)
    }
}

impl PartialOrd for GenNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for GenNode {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl Eq for GenNode {}

pub struct ChunkGen {
    rx: Receiver<GenNode>,
    queue: BinaryHeap<GenNode>,
    shared_state: super::SharedState,
    in_queue: HashSet<ChunkKey>,
}

impl ChunkGen {
    pub fn init(shared_state: super::SharedState, rx: Receiver<GenNode>) {
        thread::Builder::new()
            .name("ChunkGenerator".to_string())
            .spawn(move || {
                let mut gen = ChunkGen::new(rx, shared_state);
                gen.run();
            })
            .unwrap();
    }

    fn new(rx: Receiver<GenNode>, shared_state: super::SharedState) -> Self {
        ChunkGen {
            rx,
            queue: BinaryHeap::new(),
            shared_state,
            in_queue: HashSet::new(),
        }
    }

    fn run(&mut self) {
        loop {
            while !self.queue.is_empty() {
                let node = self.queue.pop().unwrap();
                println!("Generating: {:?}", node);
                let world_type;
                {
                    let world_reg = self.shared_state.world_registry.read().unwrap();
                    let world = world_reg.world(&node.world_id);
                    world_type = self
                        .shared_state
                        .world_type_registry
                        .world_type_reg
                        .get(&world.world_type)
                        .unwrap();
                }
                let voxels = world_type.gen_chunk(&node.key, &self.shared_state.voxel_registry);
                let mut world_reg = self.shared_state.world_registry.write().unwrap();
                let world = world_reg.world_mut(&node.world_id);
                world.pc.insert_chunk(
                    node.key,
                    Chunk::new(
                        world.chunk_size(),
                        &node.key,
                        voxels,
                        &self.shared_state.voxel_registry,
                    ),
                );
                self.in_queue.remove(&node.key);
            }
            let world_reg = self.shared_state.world_registry.read().unwrap();
            for node in self.rx.try_iter() {
                let world = world_reg.world(&node.world_id);
                if !world.pc.chunk_exists(&node.key) && !self.in_queue.contains(&node.key) {
                    self.queue.push(node);
                    self.in_queue.insert(node.key);
                }
            }
        }
    }
}
