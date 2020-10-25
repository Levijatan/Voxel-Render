use super::chunk;
use super::ticket;
use super::util;
use super::voxel;

use anyhow::{anyhow, ensure, Result};
use bitvec::prelude::*;
use dashmap::DashMap;
use log::info;
use std::cmp::Ordering;
use std::collections::{HashMap, VecDeque};

#[derive(Debug)]
pub struct World {
    pub world_type: u32,
    chunk_map: DashMap<chunk::Position, ChunkMeta>,
    ticket_arena: generational_arena::Arena<ticket::Ticket>,
    ticket_queue: VecDeque<ticket::PropagateTicket>,
}

impl World {
    pub fn new(world_type: u32) -> Self {
        Self {
            world_type,
            chunk_map: DashMap::new(),
            ticket_arena: generational_arena::Arena::new(),
            ticket_queue: VecDeque::new(),
        }
    }

    pub fn chunk_is_transparent(
        &self,
        pos: &chunk::Position,
        from_direction: util::Direction,
    ) -> Option<bool> {
        let meta = self.chunk_map.get(pos)?;
        let to_direction = util::reverse_direction(from_direction);
        Some(meta.is_transparent(to_direction))
    }

    pub fn chunk_visibility(&self, pos: &chunk::Position) -> bool {
        let meta = self.chunk_map.get(pos).unwrap();
        meta.is_visible()
    }

    pub fn chunk_set_visible(&self, pos: &chunk::Position, value: bool) {
        let mut meta = self.chunk_map.get_mut(pos).unwrap();
        meta.set_visibilty(value);
    }

    pub fn chunk_set_transparency(&self, pos: &chunk::Position, value: u8) -> Result<()> {
        ensure!(value < 64, "Cannot use values higher than 0b00111111");
        let mut meta = self.chunk_map.get_mut(pos).unwrap();
        meta.set_transparency(value);
        Ok(())
    }

    pub fn chunk_add_ticket(
        &mut self,
        value: ticket::Ticket,
        pos: &chunk::Position,
        cmd: &mut legion::systems::CommandBuffer,
    ) {
        let mut prop = None;
        if let Some(mut meta) = self.chunk_map.get_mut(pos) {
            if let Some(ticket_id) = meta.ticket {
                if let Some(ticket) = self.ticket_arena.get_mut(ticket_id) {
                    if ticket.pos == *pos {
                        *ticket = value;
                    } else {
                        let idx = self.ticket_arena.insert(value);
                        meta.ticket = Some(idx);
                        prop = Some((None, *pos, crate::consts::RENDER_RADIUS, false, idx));
                    }
                } else {
                    let idx = self.ticket_arena.insert(value);
                    meta.ticket = Some(idx);
                    prop = Some((None, *pos, crate::consts::RENDER_RADIUS, false, idx));
                }
            } else {
                let idx = self.ticket_arena.insert(value);
                meta.ticket = Some(idx);
                prop = Some((None, *pos, crate::consts::RENDER_RADIUS, false, idx));
            }
        } else {
            let idx = self.ticket_arena.insert(value);
            prop = Some((None, *pos, crate::consts::RENDER_RADIUS, false, idx));
            let chunk = chunk::new(*pos);
            let entity = cmd.push(chunk);
            let mut meta = ChunkMeta::new(entity);
            meta.ticket = Some(idx);
            self.chunk_map.insert(*pos, meta);
        }

        if let Some(p) = prop {
            self.add_to_ticket_queue(p);
        }
    }

    pub fn chunk_set_ticket_idx(
        &self,
        pos: &chunk::Position,
        idx: generational_arena::Index,
        cmd: &mut legion::systems::CommandBuffer,
    ) {
        if let Some(mut meta) = self.chunk_map.get_mut(pos) {
            meta.ticket = Some(idx);
        } else {
            let chunk = chunk::new(*pos);
            let entity = cmd.push(chunk);
            let mut meta = ChunkMeta::new(entity);
            meta.ticket = Some(idx);
            self.chunk_map.insert(*pos, meta);
        }
    }

    pub fn chunk_has_ticket(&self, pos: &chunk::Position) -> bool {
        if let Some(meta) = self.chunk_map.get(pos) {
            meta.ticket.is_some()
        } else {
            false
        }
    }

    pub fn add_to_ticket_queue(&mut self, val: ticket::PropagateTicket) {
        self.ticket_queue.push_back(val);
    }
}

pub struct Active {}

#[derive(Debug, Copy, Clone)]
pub struct ChunkMeta {
    id: legion::Entity,
    //0-5 transparency, 6 visibility
    visibility: BitArray<LocalBits, [u8; 1]>,

    pub ticket: Option<generational_arena::Index>,
}

impl ChunkMeta {
    fn new(id: legion::Entity) -> Self {
        Self {
            id,
            visibility: BitArray::new([0; 1]),
            ticket: None,
        }
    }

    fn set_transparency(&mut self, value: u8) {
        self.visibility[..6].store(value);
    }

    fn set_visibilty(&mut self, value: bool) {
        self.visibility.set(6, value)
    }

    fn is_transparent(&self, dir: util::Direction) -> bool {
        let idx: usize = dir.into();
        *self.visibility.get(idx).unwrap()
    }

    fn is_visible(&self) -> bool {
        *self.visibility.get(6).unwrap()
    }
}

impl PartialEq for World {
    fn eq(&self, other: &Self) -> bool {
        self.world_type == other.world_type
    }
}

use legion::{component, Entity, IntoQuery, Write};

pub fn world_update_system(schedule_builder: &mut legion::systems::Builder) {
    schedule_builder.add_system(
        legion::SystemBuilder::new("WorldUpdateSystem")
            .with_query(<(Entity, Write<World>)>::query().filter(component::<Active>()))
            .read_resource::<crate::clock::Clock>()
            .build(|cmd, ecs, clock, world_query| {
                if clock.cur_tick() > clock.last_tick() {
                    info!("***Updating Worlds***");
                    world_query.for_each_mut(ecs, |(world_id, world)| {
                        info!("Upwdating world {:#?}", world_id);
                        ticket::add_ticket(*world_id, world, clock, cmd);

                        if let Some(prop) = world.ticket_queue.pop_front() {
                            let (_, pos, _, _, idx) = prop;
                            world.chunk_set_ticket_idx(&pos, idx, cmd);
                            if let Ok(Some(mut new_prop)) = ticket::propagate(prop) {
                                new_prop
                                    .drain(..)
                                    .for_each(|prop| world.ticket_queue.push_back(prop));
                            }
                        }

                        let mut remove = world
                            .ticket_arena
                            .iter()
                            .map(
                                |(idx, ticket)| match ticket::update(ticket, clock.cur_tick()) {
                                    Ok(_) => Err(anyhow!("woop")),
                                    Err(_) => Ok(idx),
                                },
                            )
                            .filter_map(Result::ok)
                            .collect::<Vec<_>>();
                        remove.drain(..).for_each(|idx| {
                            world.ticket_arena.remove(idx);
                        })
                    })
                }
            }),
    );
}

pub struct TypeRegistry {
    world_type_reg: HashMap<u32, Box<dyn WorldType>>,
    next_type_key: u32,
}

impl TypeRegistry {
    pub fn new() -> Self {
        Self {
            world_type_reg: HashMap::new(),
            next_type_key: 1,
        }
    }

    pub fn register_world_type(&mut self, world_type: Box<dyn WorldType>) -> u32 {
        let id = self.get_next_type_key();
        self.world_type_reg.insert(id, world_type);
        id
    }

    fn get_next_type_key(&mut self) -> u32 {
        let out = self.next_type_key;
        self.next_type_key += 1;
        out
    }

    pub fn world_type(&self, world_id: u32) -> Result<&dyn WorldType> {
        if let Some(world_type) = self.world_type_reg.get(&world_id) {
            Ok(&**world_type)
        } else {
            Err(anyhow!("{:?} is not a valid world type id", world_id))
        }
    }
}

pub trait WorldType: Send + Sync {
    fn gen_chunk(
        &self,
        pos: &chunk::Position,
        reg: &voxel::Registry,
        out: &mut arrayvec::ArrayVec<[u64; chunk::VOXELS_IN_CHUNK]>,
    ) -> Result<()>;
    fn world_type(&self) -> &'static str;
}

pub struct FlatWorldType {}

impl WorldType for FlatWorldType {
    #[optick_attr::profile]
    fn gen_chunk(
        &self,
        pos: &chunk::Position,
        reg: &voxel::Registry,
        out: &mut arrayvec::ArrayVec<[u64; chunk::VOXELS_IN_CHUNK]>,
    ) -> Result<()> {
        let transparent_voxel = reg
            .key_from_string_id(crate::consts::TRANSPARENT_VOXEL)
            .unwrap();
        let opaque_voxel = reg.key_from_string_id(crate::consts::OPAQUE_VOXEL).unwrap();
        *out = match pos.pos.y.partial_cmp(&0).unwrap() {
            Ordering::Greater => [transparent_voxel; chunk::VOXELS_IN_CHUNK].into(),
            Ordering::Less => [opaque_voxel; chunk::VOXELS_IN_CHUNK].into(),
            Ordering::Equal => {
                let mut tmp: arrayvec::ArrayVec<[u64; chunk::VOXELS_IN_CHUNK]> =
                    [transparent_voxel; chunk::VOXELS_IN_CHUNK].into();
                for idx in 0..crate::consts::CHUNK_SIZE_USIZE * crate::consts::CHUNK_SIZE_USIZE {
                    tmp[idx] = opaque_voxel;
                }
                tmp
            }
        };
        Ok(())
    }
    fn world_type(&self) -> &'static str {
        "FlatWorldType"
    }
}
