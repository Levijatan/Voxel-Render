use super::chunk;
use super::ticket;
use super::util;
use super::voxel;
use super::chunk::PositionTrait as _;

use anyhow::{anyhow, Result};

use log::info;
use std::{collections::HashMap, cmp::Ordering};
use building_blocks::{
    storage::{
        Array3,
        chunk_map::{
            ChunkMap3, LocalChunkCache, Chunk
        },
        FastLz4
    }
};

pub type TypeId = u32;
pub type Id = legion::Entity;

pub fn new(world_type: TypeId) -> (Map, ticket::Arena, ticket::Queue) {
    (
        Map::new(world_type),
        ticket::Arena::new(),
        ticket::Queue::new(),
    )
}

pub struct Map {
    pub chunk_map: ChunkMap3<voxel::Id, chunk::Meta>,
    type_id: TypeId,
}

impl Map {
    pub fn new(type_id: TypeId) -> Self {
        let ambient_value = 1;
        let default_chunk_metadata = chunk::Meta::new();
        Self{ type_id, chunk_map: ChunkMap3::new(chunk::CHUNK_SHAPE, ambient_value, default_chunk_metadata, FastLz4 { level: 10 }) }
    }
    pub fn chunk_is_transparent(
        &self,
        pos: &chunk::Position,
        from_direction: util::Direction,
        local_cache: &LocalChunkCache<[i32; 3], voxel::Id, chunk::Meta>
    ) -> Option<bool> {
        let chunk = self.chunk_map.get_chunk(pos.key(), local_cache)?;
        let to_direction = util::reverse_direction(from_direction);
        Some(chunk.metadata.is_transparent(to_direction))
    }

    pub fn chunk_visibility(&self, pos: &chunk::Position, local_cache: &LocalChunkCache<[i32; 3], voxel::Id, chunk::Meta>) -> bool {
        self.chunk_map.get_chunk(pos.key(), local_cache).map_or(false, |chunk| chunk.metadata.is_visible())
    }

    pub fn chunk_set_visible(&mut self, pos: &chunk::Position, value: bool) {
        if let Some(chunk) = self.chunk_map.get_mut_chunk(pos.key()) {
            chunk.metadata.set_visibilty(value);
        }
    }

    pub fn chunk_set_transparency(&mut self, pos: &chunk::Position, value: u8) {
        assert!(value < 64, "Cannot use values higher than 0b00111111");
        if let Some(chunk) = self.chunk_map.get_mut_chunk(pos.key()) {
            chunk.metadata.set_transparency(value);
        }
    }

    pub fn chunk_mut(
        &mut self,
        world_id: legion::Entity,
        pos: &chunk::Position,
        cmd: &mut legion::systems::CommandBuffer,
        type_register: &TypeRegistry,
        voxel_reg: &voxel::Registry,
    ) -> &mut Chunk<[i32; 3], voxel::Id, chunk::Meta>{
        let type_id = self.type_id;
        
        let chunk = self.chunk_map.get_mut_chunk_or_insert_with(pos.key(), |point, extent| {
            let world_type = type_register.world_type(type_id).unwrap();
            world_type.gen_chunk(point, extent, voxel_reg)
        });
        if chunk.metadata.has_id() {
            chunk
        } else {
            let id = cmd.push(chunk::new(world_id, *pos));
            chunk.metadata.set_id(id);
            chunk
        }
    }

    pub fn chunk_set_ticket_idx(
        &mut self,
        world_id: Id,
        pos: &chunk::Position,
        idx: generational_arena::Index,
        cmd: &mut legion::systems::CommandBuffer,
        type_register: &TypeRegistry,
        voxel_reg: &voxel::Registry,
    ) {

        let chunk = self.chunk_mut(world_id, pos, cmd, type_register, voxel_reg);
        chunk.metadata.ticket = Some(idx);
    }

    pub fn chunk_has_ticket(&self, pos: &chunk::Position, arena: &ticket::Arena, local_cache: &LocalChunkCache<[i32; 3], voxel::Id, chunk::Meta>) -> bool {
        self.chunk_map.get_chunk(pos.key(), local_cache)
            .map_or(false, |chunk| chunk.metadata.ticket
            .map_or(false, |ticket_id| arena.contains(ticket_id)))
    }
}

pub struct Active {}



use legion::{component, Entity, IntoQuery, Read, Write};

pub fn ticket_queue(schedule_builder: &mut legion::systems::Builder) {
    schedule_builder.add_system(
        legion::SystemBuilder::new("WorldQueue")
            .with_query(
                <(Entity, Write<Map>, Read<ticket::Queue>)>::query().filter(component::<Active>()),
            ).read_resource::<TypeRegistry>()
            .read_resource::<voxel::Registry>()
            .build(|cmd, ecs, (type_reg, voxel_reg), world_query| {
                world_query.for_each_mut(ecs, |(id, map, queue)| {
                    if let Some(prop) = queue.pop() {
                        let (_, pos, _, _, idx) = prop;
                        map.chunk_set_ticket_idx(*id, &pos, idx, cmd, type_reg, voxel_reg);
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
                    Write<Map>,
                    Write<ticket::Arena>,
                    Read<ticket::Queue>,
                )>::query()
                .filter(component::<Active>()),
            )
            .read_resource::<crate::clock::Clock>()
            .read_resource::<TypeRegistry>()
            .read_resource::<voxel::Registry>()
            .build(|cmd, ecs, (clock, type_reg, voxel_reg), world_query| {
                if clock.cur_tick() > clock.last_tick() {
                    world_query.for_each_mut(ecs, |(world_id, world, arena, queue)| {
                        ticket::add((*world_id, world), clock, cmd, arena, queue, type_reg, voxel_reg);
                    })
                }
            }),
    );
}

pub fn update_system(schedule_builder: &mut legion::systems::Builder) {
    schedule_builder.add_system(
        legion::SystemBuilder::new("WorldUpdateSystem")
            .with_query(
                <(Entity, Write<ticket::Arena>)>::query().filter(component::<Active>()),
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
    world_type_reg: HashMap<u32, Box<dyn TypeTrait>>,
    next_type_key: u32,
}

impl TypeRegistry {
    pub fn new() -> Self {
        Self {
            world_type_reg: HashMap::new(),
            next_type_key: 1,
        }
    }

    pub fn register_world_type(&mut self, world_type: Box<dyn TypeTrait>) -> u32 {
        let id = self.get_next_type_key();
        self.world_type_reg.insert(id, world_type);
        id
    }

    fn get_next_type_key(&mut self) -> u32 {
        let out = self.next_type_key;
        self.next_type_key += 1;
        out
    }

    pub fn world_type(&self, world_id: u32) -> Result<&dyn TypeTrait> {
        if let Some(world_type) = self.world_type_reg.get(&world_id) {
            Ok(&**world_type)
        } else {
            Err(anyhow!("{:?} is not a valid world type id", world_id))
        }
    }
}

pub trait TypeTrait: Send + Sync {
    fn gen_chunk(
        &self,
        pos: &chunk::Position,
        extent: &chunk::Extent,
        vox_reg: &voxel::Registry,
    ) -> chunk::CType;
    fn world_type(&self) -> &'static str;
}

pub struct FlatWorldType {}

impl TypeTrait for FlatWorldType {
    #[optick_attr::profile]
    fn gen_chunk(
        &self,
        pos: &chunk::Position,
        extent: &chunk::Extent,
        vox_reg: &voxel::Registry,
    ) -> chunk::CType {
        let transparent_voxel = vox_reg.key_from_string_id(crate::consts::TRANSPARENT_VOXEL).unwrap();
        let opaque_voxel = vox_reg.key_from_string_id(crate::consts::OPAQUE_VOXEL).unwrap();
        let map = match pos.y().partial_cmp(&0).unwrap() {
            Ordering::Greater => Array3::fill(*extent, transparent_voxel),
            Ordering::Less => Array3::fill(*extent, opaque_voxel),
            Ordering::Equal => Array3::fill_with(*extent, |pos| {
                if pos.y() == 0 {
                    opaque_voxel
                } else {
                    transparent_voxel
                }
            }),
        };
        chunk::CType{metadata: chunk::Meta::new(), map}
    }
    fn world_type(&self) -> &'static str {
        "FlatWorldType"
    }
}
