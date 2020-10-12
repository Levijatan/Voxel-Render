use legion::*;

#[derive(Clone, Debug, PartialEq)]
pub struct Ticket {
    pub start_time: u64,
    pub ttl: u64,
    pub priority: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PropagateTicket {
    pub poison: Option<super::util::Direction>,
    pub ticket: Ticket,
}

//TODO: add player position
pub fn add_ticket_system(schedule_builder: &mut systems::Builder) {
    use super::chunk::MarkedForGen;
    use super::chunk::Position;
    use crate::consts::RENDER_RADIUS;

    schedule_builder.add_system(
        SystemBuilder::new("AddPlayerTicketSystem")
            .with_query(<(
                Entity,
                Write<super::world::World>,
                Read<super::world::Active>,
            )>::query())
            .read_resource::<crate::Clock>()
            .build(move |cmd, ecs, clock, world_query| {
                if clock.cur_tick % 400 == 0 || clock.cur_tick == 1 {
                    world_query.iter_mut(ecs).for_each(|(entity, world, _)| {
                        let pos = Position {
                            x: 0,
                            y: 0,
                            z: 0,
                            world_id: entity.clone(),
                        };
                        let chunk_entity: Entity;
                        if let Some(chunk_entry) = world.chunk_map.get(&pos) {
                            chunk_entity = chunk_entry.value().clone();
                        } else {
                            chunk_entity = cmd.push((pos, MarkedForGen {}));
                            world.chunk_map.insert(pos, chunk_entity);
                        }
                        let ticket = Ticket {
                            start_time: clock.cur_tick,
                            ttl: 400,
                            priority: RENDER_RADIUS,
                        };
                        let prop_ticket = PropagateTicket {
                            poison: None,
                            ticket: ticket.clone(),
                        };
                        cmd.add_component(chunk_entity, ticket);
                        cmd.add_component(chunk_entity, prop_ticket);
                    })
                }
            }),
    );
}

pub fn update_tickets_system(schedule_builder: &mut systems::Builder) {
    schedule_builder.add_system(
        SystemBuilder::new("UpdateTickets")
            .with_query(<(Entity, Write<Ticket>)>::query())
            .read_resource::<crate::Clock>()
            .build(move |cmd, ecs, clock, ticket_query| {
                ticket_query.iter_mut(ecs).for_each(|(entity, ticket)| {
                    if ticket.start_time + ticket.ttl <= clock.cur_tick {
                        cmd.remove_component::<Ticket>(*entity);
                    }
                });
            }),
    );
}

pub fn propagate_tickets_system(schedule_builder: &mut systems::Builder) {
    use super::chunk::Position;
    use super::util::reverse_direction;
    use super::util::ALL_DIRECTIONS;
    use super::world::World;

    schedule_builder.add_system(
        SystemBuilder::new("PropagateTickets")
            .with_query(<(Entity, Read<Position>, Write<PropagateTicket>)>::query())
            .with_query(<Write<World>>::query())
            .build(move |cmd, ecs, _, (ticket_query, _)| {
                let (mut ticket_ecs, mut world_ecs) = ecs.split_for_query(ticket_query);
                ticket_query
                    .iter_mut(&mut ticket_ecs)
                    .for_each(|(entity, pos, prop_ticket)| {
                        let mut world_ref = world_ecs.entry_mut(pos.world_id).unwrap();
                        let mut world = world_ref.get_component_mut::<World>().unwrap();
                        if prop_ticket.ticket.priority > 0 {
                            if let Some(skip) = prop_ticket.poison.clone() {
                                ALL_DIRECTIONS.iter().for_each(|dir| {
                                    if *dir != skip {
                                        propagate_ticket(
                                            dir,
                                            pos.clone(),
                                            prop_ticket.clone(),
                                            &mut world,
                                            cmd,
                                        )
                                    }
                                });
                            } else {
                                ALL_DIRECTIONS.iter().for_each(|dir| {
                                    let mut dir_prop_ticket = prop_ticket.clone();
                                    dir_prop_ticket.poison = Some(reverse_direction(dir));
                                    propagate_ticket(
                                        dir,
                                        pos.clone(),
                                        dir_prop_ticket,
                                        &mut world,
                                        cmd,
                                    )
                                })
                            }
                        }
                        cmd.remove_component::<PropagateTicket>(entity.clone());
                    });
            }),
    );
}

fn propagate_ticket(
    dir: &super::util::Direction,
    pos: super::chunk::Position,
    prop_ticket: PropagateTicket,
    world: &mut super::world::World,
    cmd: &mut systems::CommandBuffer,
) {
    use super::chunk::MarkedForGen;
    use super::util::normals_i64;

    let norm = normals_i64(dir);
    let mut dir_pos = pos;
    dir_pos.x += norm.x;
    dir_pos.y += norm.y;
    dir_pos.z += norm.z;
    let dir_entity: Entity;
    if let Some(dir_entity_entry) = world.chunk_map.get(&dir_pos) {
        dir_entity = dir_entity_entry.value().clone();
    } else {
        dir_entity = cmd.push((dir_pos, MarkedForGen {}));
        world.chunk_map.insert(dir_pos, dir_entity);
    }

    let mut dir_prop_ticket = prop_ticket;
    dir_prop_ticket.ticket.priority -= 1;
    dir_prop_ticket.ticket.start_time += 1;
    let dir_ticket = dir_prop_ticket.ticket.clone();
    cmd.add_component(dir_entity, dir_prop_ticket);
    cmd.add_component(dir_entity, dir_ticket);
}
