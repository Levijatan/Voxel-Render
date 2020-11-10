use geom::chunk::PositionTrait as _;
use geom::chunk::Meta as _;
use geom::world;
use geom::voxel;
use super::clock;

use legion::{SystemBuilder, Read, Write, IntoQuery, component};
use building_blocks::prelude::{PointN, ForEachMut};
use gfx::state::Instance;
use log::info;

/// A struct for defining what chunk area that are active.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Ticket {
    ///How many ticks a Ticket should live
    ttl: clock::Tick,
    ///The area that the ticket holds active
    extent: geom::chunk::Extent,
    ///The maximum radius of the area
    max_radius: i32,
    ///The current radius of the area
    cur_radius: i32,
}

impl Ticket {
    ///Returns a Ticket with how many ticks it should exist, its maximum radius and at what posistion in the world the center of the area is.
    /// 
    /// # Arguments
    /// 
    /// * `ttl` - A amount of `clock::Tick` that the Ticket should exist
    /// * `max_radius` - The maximum radius of the effected area in i32
    /// * `pos` - A position in a chunk that is the center of the effected area`
    /// 
    /// # Examples
    /// 
    /// ```
    /// use engine::ticket;
    /// use building_blocks::prelude::PointN;
    /// let pos: geom::chunk::Position = PointN([0, 0, 0]);
    /// let ticket = ticket::Ticket::new(40, 5, &pos);
    /// ```
    #[allow(clippy::must_use_candidate)]
    pub fn new(ttl: clock::Tick, max_radius: i32, pos: &geom::chunk::Position) -> Self {
        assert!(max_radius % 2 != 0, "max_radius has to be an odd number!!");
        Self{
            ttl,
            extent: geom::chunk::Extent::from_min_and_shape(pos.key(), geom::chunk::CHUNK_SHAPE),
            max_radius,
            cur_radius: 0,
        }
    }

    pub fn propagate(&mut self, pos: geom::chunk::Position) {
        if self.cur_radius < self.max_radius {
            self.cur_radius += 1;
            self.extent = geom::chunk::Extent::from_min_and_shape((pos - PointN([(self.cur_radius - 1); 3])).key(), PointN([geom::chunk::CHUNK_SIZE_I32 * ((self.cur_radius * 2) - 1); 3]));
        }
    }

    #[allow(clippy::must_use_candidate)]
    pub const fn done_propagating(&self) -> bool {
        self.cur_radius == self.max_radius
    }

    #[allow(clippy::must_use_candidate)]
    pub const fn extent(&self) -> geom::chunk::Extent {
        self.extent
    }

    #[allow(clippy::must_use_candidate)]
    pub fn create(world_id: world::Id, pos: geom::chunk::Position, ttl: clock::Tick, max_radius: i32) -> (world::Id, geom::chunk::Position, Ticket) {
        (world_id, pos, Self::new(ttl, max_radius, &pos))
    }

    pub fn systems(schedule_builder: &mut legion::systems::Builder, render_radius: i32) {
        assert!(render_radius > 0, "render_radius cannot be a number lower that 1");
        Self::add_system(schedule_builder, render_radius);
        Self::update_system(schedule_builder);
        Self::render_system(schedule_builder);
    }

    fn add_system(schedule_builder: &mut legion::systems::Builder, render_radius: i32) {
        schedule_builder.add_system(SystemBuilder::new("TicketAddSystem")
            .with_query(<(Read<world::Id>, Read<geom::chunk::Position>, Read<Self>)>::query())
            .with_query(<world::Id>::query().filter(component::<world::Active>()))
            .read_resource::<clock::Clock>()
            .build(move |cmd, ecs, clock, (ticket_query, world_query)| {
                if clock.do_tick() {
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
                            let ticket = Self::create(*world_id, PointN([0; 3]), 40, render_radius);
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
            .with_query(<(Read<world::Id>, Write<geom::chunk::Position>, Write<Self>)>::query())
            .with_query(<(world::Id, Read<world::TypeId>, Write<world::Map<super::chunk::MetaU64>>)>::query().filter(component::<world::Active>()))
            .read_resource::<clock::Clock>()
            .read_resource::<world::TypeRegistry<super::chunk::MetaU64>>()
            .read_resource::<voxel::Registry>()
            .build(|_, ecs, (clock, world_type_reg, voxel_reg), (ticket_query, world_query)| {
                if clock.do_tick() {
                    info!("***Update Ticket System***");
                    let (mut world_ecs, mut ticket_ecs) = ecs.split_for_query(world_query);
                    world_query.for_each_mut(&mut world_ecs, |(world_id, type_id, map)| {
                        let world_type = world_type_reg.world_type(*type_id).unwrap();
                        ticket_query.for_each_mut(&mut ticket_ecs, |(ticket_world_id, pos, ticket)| {
                            if world_id == ticket_world_id && !ticket.done_propagating() {
                                ticket.propagate(*pos);
                                info!("Propagating ticket {:#?}", ticket);
                                map.chunk_keys_for_extent(&ticket.extent()).for_each(|key|{ 
                                    let _chunk = map.get_mut_chunk_or_insert_with(
                                        key,
                                        |point, extent| {
                                            world_type.gen_chunk(point, extent, voxel_reg)
                                        }
                                    );
                                })
                            }
                        });
                    });
                }
            })
        );
    }


    #[allow(clippy::cast_possible_truncation)]
    fn render_system(schedule_builder: &mut legion::systems::Builder) {
        schedule_builder.add_system(SystemBuilder::new("RenderTicketSystem")
            .with_query(<(Read<world::Id>, Read<Self>)>::query())
            .with_query(<(world::Id, Write<world::Map<super::chunk::MetaU64>>)>::query().filter(component::<world::Active>()))
            .read_resource::<clock::Clock>()
            .read_resource::<gfx::chunk::State>()
            .read_resource::<voxel::Registry>()
            .write_resource::<gfx::buffer::OffsetControllerU32>()
            .read_resource::<wgpu::Queue>()
            .build(|_, ecs, (clock, chunk_state, vox_reg, chunk_renderer, render_queue), (ticket_query, world_query)| {
                if clock.do_tick() {
                    info!("***Render Ticket System***");
                    let (mut world_ecs, mut ticket_ecs) = ecs.split_for_query(world_query);
                    world_query.for_each_mut(&mut world_ecs,  |(world_id, map)| {
                        ticket_query.for_each_mut(&mut ticket_ecs, |(ticket_world_id, ticket)| {
                            if world_id == ticket_world_id {
                                let mut cnt = 0;
                                for key in map.chunk_keys_for_extent(&ticket.extent()) {
                                    let extent = map.extent_for_chunk_at_key(&key);
                                    if let Some(chunk) = map.get_mut_chunk(key) {
                                        if chunk.metadata.is_visible() && !chunk.metadata.has_render_offset() {
                                            info!("Loading chunk to gpu: {:#?}", chunk.metadata);
                                            if let Some(render_offset) = chunk_renderer.fetch_offset() {
                                                cnt += 1;
                                                let mut instances = Vec::new();
                                                let c_map = &mut chunk.array;
                                                let c_meta = &mut chunk.metadata;
                                                c_map.for_each_mut(&extent, |point: geom::chunk::Position, value| {
                                                    if c_meta.voxel_is_visible(point-key) && !vox_reg.is_transparent(*value).unwrap() {
                                                        let rotation = voxel::rotation();
                                                        let position = voxel::calc_pos(point);
                                                        instances.push(Instance{position, rotation})
                                                    }
                                                });
                                                gfx::state::set_instance_buffer(render_queue, chunk_state, &instances, render_offset.into());
                                                c_meta.set_render_offset(Some(render_offset));
                                                assert!(instances.len() <= std::u16::MAX as usize, "instance length can never be larger than {}", std::u16::MAX);
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
}





