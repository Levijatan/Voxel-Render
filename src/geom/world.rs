use super::chunk;
use super::ticket;
use super::util;
use super::voxel;

use anyhow::{anyhow, ensure, Result};
use bitvec::prelude::*;
use dashmap::DashMap;
use log::info;
use std::cmp::Ordering;
use std::collections::HashMap;

pub type TypeId = u32;
pub type Id = legion::Entity;

pub fn new_world(world_type: TypeId) -> (TypeId, Map, ticket::TicketArena, ticket::TicketQueue) {
    (
        world_type,
        Map::new(),
        ticket::TicketArena::new(),
        ticket::TicketQueue::new(),
    )
}

pub struct Map {
    chunk_map: DashMap<chunk::Position, ChunkMeta>,
}

impl Map {
    pub fn new() -> Self {
        Self{ chunk_map: DashMap::new() }
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
        &self,
        value: ticket::Ticket,
        id: Id,
        pos: &chunk::Position,
        cmd: &mut legion::systems::CommandBuffer,
        arena: &mut ticket::TicketArena,
        queue: &ticket::TicketQueue,
    ) {
        let mut prop = None;
        if let Some(mut meta) = self.chunk_map.get_mut(pos) {
            if let Some(ticket_id) = meta.ticket {
                if let Some(ticket) = arena.get_mut(ticket_id) {
                    if ticket.pos == *pos {
                        *ticket = value;
                    } else {
                        let idx = arena.insert(value);
                        meta.ticket = Some(idx);
                        prop = Some((None, *pos, crate::consts::RENDER_RADIUS, false, idx));
                    }
                } else {
                    let idx = arena.insert(value);
                    meta.ticket = Some(idx);
                    prop = Some((None, *pos, crate::consts::RENDER_RADIUS, false, idx));
                }
            } else {
                let idx = arena.insert(value);
                meta.ticket = Some(idx);
                prop = Some((None, *pos, crate::consts::RENDER_RADIUS, false, idx));
            }
        } else {
            let idx = arena.insert(value);
            prop = Some((None, *pos, crate::consts::RENDER_RADIUS, false, idx));
            let chunk = chunk::new(id, *pos);
            let entity = cmd.push(chunk);
            let mut meta = ChunkMeta::new(entity);
            meta.ticket = Some(idx);
            self.chunk_map.insert(*pos, meta);
        }

        if let Some(p) = prop {
            queue.push(p);
        }
    }

    pub fn chunk_set_ticket_idx(
        &self,
        id: Id,
        pos: &chunk::Position,
        idx: generational_arena::Index,
        cmd: &mut legion::systems::CommandBuffer,
    ) {
        if let Some(mut meta) = self.chunk_map.get_mut(pos) {
            meta.ticket = Some(idx);
        } else {
            let chunk = chunk::new(id, *pos);
            let entity = cmd.push(chunk);
            let mut meta = ChunkMeta::new(entity);
            meta.ticket = Some(idx);
            self.chunk_map.insert(*pos, meta);
        }
    }

    pub fn chunk_has_ticket(&self, pos: &chunk::Position, arena: &ticket::TicketArena) -> bool {
        if let Some(meta) = self.chunk_map.get(pos) {
            if let Some(ticket_id) = meta.ticket {
                arena.contains(ticket_id)
            } else {
                false
            }
        } else {
            false
        }
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

use legion::{component, Entity, IntoQuery, Read, Write};

pub fn ticket_queue(schedule_builder: &mut legion::systems::Builder) {
    schedule_builder.add_system(
        legion::SystemBuilder::new("WorldTicketQueue")
            .with_query(
                <(Entity, Read<Map>, Read<ticket::TicketQueue>)>::query().filter(component::<Active>()),
            )
            .build(|cmd, ecs, _, world_query| {
                world_query.for_each_mut(ecs, |(id, map, queue)| {
                    if let Some(prop) = queue.pop() {
                        let (_, pos, _, _, idx) = prop;
                        map.chunk_set_ticket_idx(*id, &pos, idx, cmd);
                        if let Ok(Some(mut new_prop)) = ticket::propagate(prop) {
                            new_prop.drain(..).for_each(|prop| queue.push(prop));
                        }
                    }
                })
            }),
    );
}

pub fn add_player_ticket_system(schedule_builder: &mut legion::systems::Builder) {
    schedule_builder.add_system(
        legion::SystemBuilder::new("WorldUpdateSystem")
            .with_query(
                <(
                    Entity,
                    Read<Map>,
                    Write<ticket::TicketArena>,
                    Read<ticket::TicketQueue>,
                )>::query()
                .filter(component::<Active>()),
            )
            .read_resource::<crate::clock::Clock>()
            .build(|cmd, ecs, clock, world_query| {
                if clock.cur_tick() > clock.last_tick() {
                    world_query.for_each_mut(ecs, |(world_id, world, arena, queue)| {
                        ticket::add_ticket(*world_id, world, clock, cmd, arena, queue);
                    })
                }
            }),
    );
}

pub fn world_update_system(schedule_builder: &mut legion::systems::Builder) {
    schedule_builder.add_system(
        legion::SystemBuilder::new("WorldUpdateSystem")
            .with_query(
                <(Entity, Write<ticket::TicketArena>)>::query().filter(component::<Active>()),
            )
            .read_resource::<crate::clock::Clock>()
            .build(|_, ecs, clock, world_query| {
                if clock.cur_tick() > clock.last_tick() {
                    info!("***Updating Worlds***");
                    world_query.for_each_mut(ecs, |(world_id, arena)| {
                        info!("Upwdating world {:#?}", world_id);

                        let mut remove = arena
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
                            arena.remove(idx);
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
        out: &mut arrayvec::ArrayVec<[voxel::Id; chunk::VOXELS_IN_CHUNK]>,
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
        out: &mut arrayvec::ArrayVec<[voxel::Id; chunk::VOXELS_IN_CHUNK]>,
    ) -> Result<()> {
        let transparent_voxel = reg
            .key_from_string_id(crate::consts::TRANSPARENT_VOXEL)
            .unwrap();
        let opaque_voxel = reg.key_from_string_id(crate::consts::OPAQUE_VOXEL).unwrap();
        *out = match pos.pos.y.partial_cmp(&0).unwrap() {
            Ordering::Greater => [transparent_voxel; chunk::VOXELS_IN_CHUNK].into(),
            Ordering::Less => [opaque_voxel; chunk::VOXELS_IN_CHUNK].into(),
            Ordering::Equal => {
                let mut tmp: arrayvec::ArrayVec<[voxel::Id; chunk::VOXELS_IN_CHUNK]> =
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
