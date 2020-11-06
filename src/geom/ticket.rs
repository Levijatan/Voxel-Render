use super::chunk;
use super::chunk::PositionTrait;
use super::world;
use super::voxel;
use crate::clock;

use building_blocks::prelude::PointN;
use log::info;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Ticket {
    ttl: clock::Tick,
    extent: chunk::Extent,
    max_radius: u8,
    cur_radius: u8,
}

impl Ticket {
    pub fn new(ttl: clock::Tick, max_radius: u8, pos: &chunk::Position) -> Self {
        assert!(max_radius % 2 != 0, "max_radius has to be an odd number!!");
        Self{
            ttl,
            extent: chunk::Extent::from_min_and_shape(pos.key(), chunk::CHUNK_SHAPE),
            max_radius,
            cur_radius: 0,
        }
    }

    pub fn propagate(&mut self, pos: chunk::Position) {
        if self.cur_radius < self.max_radius {
            self.cur_radius += 1;
            self.extent = chunk::Extent::from_min_and_shape((pos - PointN([(self.cur_radius as i32 - 1); 3])).key(), PointN([crate::consts::CHUNK_SIZE_I32 * ((self.cur_radius as i32 * 2) - 1); 3]));
        }
    }

    pub const fn done_propagating(&self) -> bool {
        self.cur_radius == self.max_radius
    }

    pub const fn extent(&self) -> chunk::Extent {
        self.extent
    }
}

pub type TicketComponents = (world::Id, chunk::Position, Ticket);

pub fn create(world_id: world::Id, pos: chunk::Position, ttl: clock::Tick, max_radius: u8) -> TicketComponents {
    (world_id, pos, Ticket::new(ttl, max_radius, &pos))
}

use legion::{SystemBuilder, Read, Write, IntoQuery, component};
pub fn systems(schedule_builder: &mut legion::systems::Builder) {
    add_system(schedule_builder);
    update_system(schedule_builder);
    render_system(schedule_builder);
}

fn add_system(schedule_builder: &mut legion::systems::Builder) {
    schedule_builder.add_system(SystemBuilder::new("TicketAddSystem")
        .with_query(<(Read<world::Id>, Read<chunk::Position>, Read<Ticket>)>::query())
        .with_query(<world::Id>::query().filter(component::<world::Active>()))
        .read_resource::<clock::Clock>()
        .build(|cmd, ecs, clock, (ticket_query, world_query)| {
            if clock.cur_tick() > clock.last_tick() {
                info!("***Add Ticket System***");
                let (world_ecs, ticket_ecs) = ecs.split_for_query(world_query);
                world_query.for_each(&world_ecs, |world_id| {
                    let mut has_active_ticket = false;
                    ticket_query.for_each(&ticket_ecs, |(ticket_world_id, _pos, _ticket)| {
                        if ticket_world_id == world_id {
                            has_active_ticket = true;
                        }
                    });
                    if !has_active_ticket {
                        let ticket = create(*world_id, PointN([0; 3]), 40, crate::consts::RENDER_RADIUS);
                        info!("Adding new ticket {:#?}", ticket);
                        cmd.push(ticket);
                    }
                });
            }
        })
    );
}

fn update_system(schedule_builder: &mut legion::systems::Builder) {
    schedule_builder.add_system(SystemBuilder::new("TicketUpdateSyste")
        .with_query(<(Read<world::Id>, Write<chunk::Position>, Write<Ticket>)>::query())
        .with_query(<(world::Id, Write<world::Map>)>::query().filter(component::<world::Active>()))
        .read_resource::<clock::Clock>()
        .read_resource::<world::TypeRegistry>()
        .read_resource::<voxel::Registry>()
        .build(|_, ecs, (clock, world_type_reg, voxel_reg), (ticket_query, world_query)| {
            if clock.cur_tick() > clock.last_tick() {
                info!("***Update Ticket System***");
                let (mut world_ecs, mut ticket_ecs) = ecs.split_for_query(world_query);
                world_query.for_each_mut(&mut world_ecs, |(world_id, map)| {
                    let world_type = world_type_reg.world_type(map.type_id()).unwrap();
                    ticket_query.for_each_mut(&mut ticket_ecs, |(ticket_world_id, pos, ticket)| {
                        if world_id == ticket_world_id && !ticket.done_propagating() {
                            ticket.propagate(*pos);
                            info!("Propagating ticket {:#?}", ticket);
                            for key in map.chunk_map.key_iter(&ticket.extent()) {
                                let _chunk = map.chunk_map.get_mut_chunk_or_insert_with(
                                    key,
                                    |point, extent| {
                                        world_type.gen_chunk(point, extent, voxel_reg)
                                    }
                                );
                            }
                        }
                    });
                });
            }
        })
    );
}



use building_blocks::prelude::ForEachMut;
use crate::render::state::Instance;
fn render_system(schedule_builder: &mut legion::systems::Builder) {
    schedule_builder.add_system(SystemBuilder::new("RenderTicketSystem")
        .with_query(<(Read<world::Id>, Read<Ticket>)>::query())
        .with_query(<(world::Id, Write<world::Map>)>::query().filter(component::<world::Active>()))
        .read_resource::<clock::Clock>()
        .read_resource::<crate::render::chunk::State>()
        .read_resource::<voxel::Registry>()
        .write_resource::<crate::render::chunk::Renderer>()
        .read_resource::<wgpu::Queue>()
        .build(|_, ecs, (clock, chunk_state, vox_reg, chunk_renderer, render_queue), (ticket_query, world_query)| {
            if clock.cur_tick() > clock.last_tick() {
                info!("***Render Ticket System***");
                let (mut world_ecs, mut ticket_ecs) = ecs.split_for_query(world_query);
                world_query.for_each_mut(&mut world_ecs,  |(world_id, map)| {
                    ticket_query.for_each_mut(&mut ticket_ecs, |(ticket_world_id, ticket)| {
                        if world_id == ticket_world_id {
                            let mut cnt = 0;
                            for key in map.chunk_map.key_iter(&ticket.extent()) {
                                let extent = map.chunk_map.extent_for_chunk_at_key(&key);
                                if let Some(chunk) = map.chunk_map.get_mut_chunk(key) {
                                    if chunk.metadata.is_visible() && !chunk.metadata.has_render_offset() {
                                        info!("Loading chunk to gpu: {:#?}", chunk.metadata);
                                        if let Some(render_offset) = chunk_renderer.fetch_offset() {
                                            cnt += 1;
                                            let mut instances = Vec::new();
                                            let c_map = &mut chunk.map;
                                            let c_meta = &mut chunk.metadata;
                                            c_map.for_each_mut(&extent, |point: chunk::Position, value| {
                                                if c_meta.voxel_is_visible(point-key) && !vox_reg.is_transparent(*value).unwrap() {
                                                    let rotation = voxel::rotation();
                                                    let position = voxel::calc_pos(point);
                                                    instances.push(Instance{position, rotation})
                                                }
                                            });
                                            crate::render::state::set_instance_buffer(render_queue, chunk_state, &instances, render_offset as u64);
                                            c_meta.set_render_offset(Some(render_offset));
                                            c_meta.render_amount = instances.len() as u16;
                                        }
                                    }
                                }
                                if cnt == 7 {
                                    break;
                                }
                            }
                        }
                    })
                });
            }
        })
    );
}