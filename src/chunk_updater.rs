use std::collections::{BinaryHeap, HashMap};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::thread;

use glm::Vec3;

use flamer::flame;

use super::chunk_gen::GenNode;
use super::geom::normals;
use super::geom::ChunkKey;
use super::SharedState;

#[derive(Debug, Ord, Eq, PartialEq, PartialOrd)]
pub struct ChunkTicket {
    key: ChunkKey,
    priority: u32,
    ttl: u32,
    propagated: bool,
    update_render: bool,
    reverse_poison: i32,
    world_id: u64,
    render: bool,
}

impl ChunkTicket {
    #[flame("ChunkTicket")]
    pub fn new(key: ChunkKey, priority: u32, ttl: u32, world_id: u64) -> ChunkTicket {
        ChunkTicket {
            key,
            priority,
            ttl,
            propagated: false,
            update_render: true,
            reverse_poison: 6,
            world_id,
            render: false,
        }
    }
}

#[derive(Debug, Ord, Eq, PartialEq, PartialOrd)]
struct TicketPriority {
    priority: u32,
    key: ChunkKey,
}

pub struct ChunkUpdater {
    ticket_queue: BinaryHeap<TicketPriority>,
    ticket_map: HashMap<ChunkKey, ChunkTicket>,
    state: SharedState,
    rx: Receiver<ChunkTicket>,
    tx: Sender<ChunkKey>,
    tx_chunk_gen: Sender<GenNode>,
    old_cam_chunk_pos: Vec3,
}

impl ChunkUpdater {
    #[flame("ChunkUpdater")]
    pub fn new(
        state: SharedState,
        rx: Receiver<ChunkTicket>,
        tx: Sender<ChunkKey>,
        tx_chunk_gen: Sender<GenNode>,
    ) -> ChunkUpdater {
        ChunkUpdater {
            ticket_queue: BinaryHeap::new(),
            ticket_map: HashMap::new(),
            state,
            rx,
            tx,
            tx_chunk_gen,
            old_cam_chunk_pos: Vec3::new(0.1, 0.1, 0.1),
        }
    }

    #[flame("ChunkUpdater")]
    pub fn init(
        rx: Receiver<ChunkTicket>,
        tx: Sender<ChunkKey>,
        tx_chunk_gen: Sender<GenNode>,
        state: SharedState,
    ) {
        thread::Builder::new()
            .name("ChunkUpdater".to_string())
            .spawn(move || {
                let mut updater = ChunkUpdater::new(state, rx, tx, tx_chunk_gen);
                updater.run();
            })
            .unwrap();
    }

    #[flame("ChunkUpdater")]
    pub fn run(&mut self) {
        let mut last_tick = 0;
        loop {
            let tick = self.state.tick.read().unwrap().clone();
            if tick != last_tick {
                last_tick = tick;
                match self.rx.try_recv() {
                    Ok(ticket) => self.add_ticket(ticket),
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => break,
                }

                self.process();
            }
        }
    }

    #[flame("ChunkUpdater")]
    pub fn add_ticket(&mut self, ticket: ChunkTicket) {
        if self.ticket_map.contains_key(&ticket.key) {
            self.ticket_map.insert(ticket.key, ticket);
        } else {
            self.ticket_queue.push(TicketPriority {
                key: ticket.key,
                priority: ticket.priority,
            });
            self.ticket_map.insert(ticket.key, ticket);
        }
    }

    #[flame("ChunkUpdater")]
    pub fn propagate_ticket(&mut self, key: &ChunkKey) {
        if self.ticket_map[key].priority > 1 {
            for i in 0..6 {
                if i != self.ticket_map[key].reverse_poison {
                    let norm = normals(i);
                    let mut n_key = key.clone();
                    n_key.x += norm.x as i32;
                    n_key.y += norm.y as i32;
                    n_key.z += norm.z as i32;
                    self.add_ticket(ChunkTicket {
                        key: n_key,
                        priority: self.ticket_map[key].priority - 1,
                        ttl: self.ticket_map[key].ttl,
                        propagated: false,
                        update_render: true,
                        render: false,
                        reverse_poison: self.ticket_map[key].reverse_poison,
                        world_id: self.ticket_map[key].world_id,
                    });
                }
            }
        }
        self.ticket_map.get_mut(key).unwrap().propagated = true;
    }

    #[flame("ChunkUpdater")]
    fn update_chunk_render(&mut self, key: &ChunkKey) {
        let mut visible = false;
        let world = self
            .state
            .world_registry
            .world(&self.ticket_map[key].world_id);

        for i in 0..6 {
            let norm = super::geom::normals(i);
            let mut n_key = key.clone();
            n_key.x += norm.x as i32;
            n_key.y += norm.y as i32;
            n_key.z += norm.z as i32;

            if world.pc.chunk_is_transparent(&n_key, i) {
                visible = true;
                break;
            }
        }

        self.ticket_map.get_mut(key).unwrap().render = visible;

        if visible {
            let mut render_data = Vec::new();
            for idx in 0..world.pc.chunk_tot_size() {
                if !world
                    .pc
                    .voxel_in_chunk_transparency_idx(key, idx, &self.state.voxel_registry)
                {
                    let pos = super::geom::idx_to_pos(idx, world.chunk_size());
                    let mut render = false;
                    for i in 0..6 {
                        let norm = normals(i);
                        let n_pos = pos + norm;
                        if world.pc.voxel_pos_in_chunk(key, &n_pos) {
                            if world.pc.voxel_in_chunk_transparency(
                                key,
                                &n_pos,
                                &self.state.voxel_registry,
                            ) {
                                render = true;
                                break;
                            }
                        } else {
                            let mut n_key = key.clone();
                            n_key.x += norm.x as i32;
                            n_key.y += norm.y as i32;
                            n_key.z += norm.z as i32;
                            if world.pc.chunk_exists(&n_key) {
                                let n_world_pos = world.pc.voxel_to_world_pos(key, &n_pos);
                                if world.pc.voxel_transparency(
                                    &n_world_pos,
                                    &n_key,
                                    &self.state.voxel_registry,
                                    world.chunk_size(),
                                ) {
                                    render = true;
                                    break;
                                }
                            } else {
                                render = true;
                                break;
                            }
                        }
                    }

                    if render {
                        let world_pos = world.pc.voxel_to_world_pos(key, &pos);
                        render_data.push(world_pos.x);
                        render_data.push(world_pos.y);
                        render_data.push(world_pos.z);
                    }
                }
            }
            drop(world);
            if render_data.len() > 0 {
                let w = self
                    .state
                    .world_registry
                    .world(&self.ticket_map[key].world_id);
                w.pc.chunk_set_render_data(key, render_data);
            }
        }
    }

    #[flame("ChunkUpdater")]
    fn process_check_if_new_chunk(&mut self, key: &ChunkKey) -> bool {
        let ticket = self.ticket_map.get(key).unwrap();
        let world = self.state.world_registry.world(&ticket.world_id);
        if !world.pc.chunk_exists(&ticket.key) {
            self.tx_chunk_gen
                .send(GenNode {
                    priority: ticket.priority - 1,
                    world_id: ticket.world_id,
                    key: ticket.key,
                })
                .unwrap();
            true
        } else {
            false
        }
    }

    #[flame("ChunkUpdater")]
    pub fn process(&mut self) {
        println!("Chunk Ticket Queue Len: {}", self.ticket_queue.len());
        {
            let cam_chunk_pos = self.state.cam_chunk_pos.read().unwrap();
            if *cam_chunk_pos != self.old_cam_chunk_pos {
                let mut reset_render = self.state.clear_render.write().unwrap();
                *reset_render = true;
            }
        }

        if !self.ticket_queue.is_empty() {
            let mut next_queue = BinaryHeap::new();
            while !self.ticket_queue.is_empty() {
                let ticket_priority = self.ticket_queue.pop().unwrap();
                self.ticket_map.get_mut(&ticket_priority.key).unwrap().ttl -= 1;
                if !self.process_check_if_new_chunk(&ticket_priority.key) {
                    if !self.ticket_map[&ticket_priority.key].propagated {
                        self.propagate_ticket(&ticket_priority.key);
                    }

                    if self.ticket_map[&ticket_priority.key].update_render {
                        self.update_chunk_render(&ticket_priority.key);
                    }

                    if self.ticket_map[&ticket_priority.key].render {
                        self.tx.send(ticket_priority.key).unwrap();
                    }
                }

                if self.ticket_map[&ticket_priority.key].ttl > 0 {
                    next_queue.push(TicketPriority {
                        key: self.ticket_map[&ticket_priority.key].key,
                        priority: self.ticket_map[&ticket_priority.key].priority,
                    });
                } else {
                    self.ticket_map.remove(&ticket_priority.key);
                }
            }
            self.ticket_queue = next_queue;
        }

        if *self.state.clear_render.read().unwrap() {
            let mut reset_render = self.state.clear_render.write().unwrap();
            *reset_render = false;
        }
    }
}
