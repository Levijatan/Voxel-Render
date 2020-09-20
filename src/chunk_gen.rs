use glm::Vec3;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::sync::{mpsc, Arc};
use std::thread;

use super::geom::Chunk;

#[derive(Copy, Clone, Debug)]
pub struct GenNode {
    pub priority: i32,
    pub world_id: u64,
    pub world_type: u64,
    pub pos: Vec3,
}

#[derive(Debug)]
pub struct ChunkNode {
    pub chunk: Chunk,
    pub world_id: u64,
}

impl Ord for GenNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other.priority.cmp(&self.priority)
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
    tx: Sender<ChunkNode>,
    rx: Receiver<GenNode>,
    queue: BinaryHeap<GenNode>,
    shared_state: Arc<super::SharedState>,
}

impl ChunkGen {
    pub fn init(shared_state: Arc<super::SharedState>) -> (Sender<GenNode>, Receiver<ChunkNode>) {
        let (tx, rx) = mpsc::channel();
        let (tx2, rx2) = mpsc::channel();
        thread::spawn(move || {
            let mut gen = ChunkGen::new(tx2, rx, shared_state);
            gen.run();
        });
        (tx, rx2)
    }

    fn new(
        tx: Sender<ChunkNode>,
        rx: Receiver<GenNode>,
        shared_state: Arc<super::SharedState>,
    ) -> Self {
        ChunkGen {
            tx,
            rx,
            queue: BinaryHeap::new(),
            shared_state,
        }
    }

    fn run(&mut self) {
        let mut running = true;
        while running {
            while !self.queue.is_empty() {
                let node = self.queue.pop().unwrap();
                //println!("Generating Node: {:?}", node);
                let world_type = self
                    .shared_state
                    .world_type_registry
                    .world_type_reg
                    .get(&node.world_type)
                    .unwrap();
                let chunk = world_type.gen_chunk(node.pos, &self.shared_state.voxel_registry);
                match self.tx.send(ChunkNode {
                    chunk,
                    world_id: node.world_id,
                }) {
                    Ok(_) => (),
                    Err(_) => running = false,
                };
            }

            for node in self.rx.try_iter() {
                //println!("Pushing node: {:?}", node);
                self.queue.push(node);
            }
        }
    }
}
