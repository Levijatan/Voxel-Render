use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::thread;

use super::geom::Chunk;
use super::geom::ChunkKey;

use flamer::flame;

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
    #[flame("ChunkGen")]
    pub fn init(shared_state: super::SharedState, rx: Receiver<GenNode>) {
        let mut gen = ChunkGen::new(rx, shared_state);
        thread::Builder::new()
            .name("ChunkGenerator".to_string())
            .spawn(move || {
                gen.run();
            })
            .unwrap();
    }

    #[flame("ChunkGen")]
    fn new(rx: Receiver<GenNode>, shared_state: super::SharedState) -> Self {
        ChunkGen {
            rx,
            queue: BinaryHeap::new(),
            shared_state,
            in_queue: HashSet::new(),
        }
    }

    #[flame("ChunkGen")]
    fn run(&mut self) {
        loop {
            while !self.queue.is_empty() {
                let node = self.queue.pop().unwrap();
                println!("Generating: {:?}", node);
                let world_type;
                {
                    let world = self.shared_state.world_registry.world(&node.world_id);
                    world_type = self
                        .shared_state
                        .world_type_registry
                        .world_type_reg
                        .get(&world.world_type)
                        .unwrap();
                }
                let voxels = world_type.gen_chunk(&node.key, &self.shared_state.voxel_registry);
                let world = self.shared_state.world_registry.world(&node.world_id);
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

            match self.rx.try_recv() {
                Ok(node) => {
                    let world = self.shared_state.world_registry.world(&node.world_id);
                    if !world.pc.chunk_exists(&node.key) && !self.in_queue.contains(&node.key) {
                        self.queue.push(node);
                        self.in_queue.insert(node.key);
                    }
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => break,
            }
        }
    }
}
