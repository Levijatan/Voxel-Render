use super::chunk;
use super::util;
use super::world;

use anyhow::Result;
use legion::{maybe_changed, systems, Entity, IntoQuery, Read, SystemBuilder, Write};

#[derive(Clone, Debug, PartialEq)]
pub struct Ticket {
    pub start_time: u64,
    pub ttl: u64,
    pub priority: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PropagateComponent {
    dir: Option<util::Direction>,
    ticket: Ticket,
    branch: bool,
}

//TODO: add player position
pub fn add_ticket_system(schedule_builder: &mut systems::Builder) {
    schedule_builder.add_system(
        SystemBuilder::new("AddPlayerTicketSystem")
            .with_query(<(Entity, Write<world::World>, Read<world::Active>)>::query())
            .read_resource::<crate::Clock>()
            .build(move |cmd, ecs, clock, world_query| {
                optick::event!();
                if clock.cur_tick % 400 == 0 || clock.cur_tick == 1 {
                    world_query.iter_mut(ecs).for_each(|(entity, world, _)| {
                        let pos = chunk::Position {
                            pos: glm::vec3(0, 0, 0),
                            world_id: entity.clone(),
                        };
                        let chunk_entity: Entity;
                        if let Some(chunk_entry) = world.chunk_map.get(&pos) {
                            chunk_entity = chunk_entry.clone();
                        } else {
                            let chunk = chunk::new(pos.clone());
                            chunk_entity = cmd.push(chunk);
                            world.chunk_map.insert(pos, chunk_entity);
                        }
                        let ticket = Ticket {
                            start_time: clock.cur_tick,
                            ttl: 400,
                            priority: crate::consts::RENDER_RADIUS,
                        };
                        let prop_ticket = PropagateComponent {
                            dir: None,
                            ticket: ticket.clone(),
                            branch: false,
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
                optick::event!();
                ticket_query.iter_mut(ecs).for_each(|(entity, ticket)| {
                    if ticket.start_time + ticket.ttl <= clock.cur_tick {
                        cmd.remove_component::<Ticket>(*entity);
                    }
                });
            }),
    );
}

pub fn propagate_tickets_system(schedule_builder: &mut systems::Builder) {
    use util::Direction::*;

    schedule_builder.add_system(
        SystemBuilder::new("PropagateComponents")
            .with_query(<(Write<world::World>, Read<world::Active>)>::query())
            .with_query(
                <(Entity, Read<chunk::Position>, Write<PropagateComponent>)>::query()
                    .filter(maybe_changed::<PropagateComponent>()),
            )
            .build(move |cmd, ecs, _, (world_query, ticket_query)| {
                optick::event!();
                let (mut world_ecs, mut ticket_ecs) = ecs.split_for_query(world_query);
                world_query.iter_mut(&mut world_ecs).for_each(|(world, _)| {
                    ticket_query.iter_mut(&mut ticket_ecs).for_each(
                        |(entity, pos, prop_ticket)| {
                            if prop_ticket.ticket.priority > 0 {
                                match prop_ticket.dir {
                                    Some(Up) => {
                                        propagate_up_or_down(prop_ticket, pos, cmd, world, true)
                                    }
                                    Some(Down) => {
                                        propagate_up_or_down(prop_ticket, pos, cmd, world, false)
                                    }
                                    Some(_) => propagate_rest(prop_ticket, pos, cmd, world),
                                    None => propagate_start(prop_ticket, pos, cmd, world),
                                };
                            }
                            cmd.remove_component::<PropagateComponent>(entity.clone());
                        },
                    );
                });
            }),
    );
}

fn propagate_rest(
    prop_ticket: &PropagateComponent,
    pos: &chunk::Position,
    cmd: &mut systems::CommandBuffer,
    world: &mut world::World,
) {
    let dir = prop_ticket.dir.clone().unwrap();
    propagate_ticket(&dir, pos, prop_ticket.clone(), world, cmd).unwrap();
    if !prop_ticket.branch {
        let mut branch = prop_ticket.clone();
        branch.branch = false;
        let p_dir = prop_ticket.dir.clone().unwrap();
        let dir = util::go_left(&p_dir).unwrap();
        propagate_ticket(&dir, pos, branch, world, cmd).unwrap();
    }
}

fn propagate_up_or_down(
    prop_ticket: &PropagateComponent,
    pos: &chunk::Position,
    cmd: &mut systems::CommandBuffer,
    world: &mut world::World,
    up: bool,
) {
    let skip: util::Direction;
    if up {
        skip = util::Direction::Down;
    } else {
        skip = util::Direction::Up;
    }
    for dir in util::ALL_DIRECTIONS.iter() {
        if *dir != skip {
            propagate_ticket(dir, pos, prop_ticket.clone(), world, cmd).unwrap();
        }
    }
}

fn propagate_start(
    prop_ticket: &PropagateComponent,
    pos: &chunk::Position,
    cmd: &mut systems::CommandBuffer,
    world: &mut world::World,
) {
    for dir in util::ALL_DIRECTIONS.iter() {
        propagate_ticket(dir, pos, prop_ticket.clone(), world, cmd).unwrap();
    }
}

#[optick_attr::profile]
fn propagate_ticket(
    dir: &util::Direction,
    pos: &chunk::Position,
    prop_ticket: PropagateComponent,
    world: &mut world::World,
    cmd: &mut systems::CommandBuffer,
) -> Result<()> {
    let dir_pos = pos.neighbor(&dir)?;
    let dir_entity: Entity;
    if let Some(dir_entity_entry) = world.chunk_map.get(&dir_pos) {
        dir_entity = dir_entity_entry.clone();
    } else {
        let chunk = chunk::new(dir_pos);
        dir_entity = cmd.push(chunk);
        world.chunk_map.insert(dir_pos, dir_entity);
    }
    let mut prop_ticket = prop_ticket;
    prop_ticket.dir = Some(dir.clone());
    prop_ticket.ticket.priority -= 1;
    prop_ticket.ticket.start_time += 1;
    let dir_ticket = prop_ticket.ticket.clone();
    cmd.add_component(dir_entity, prop_ticket);
    cmd.add_component(dir_entity, dir_ticket);
    Ok(())
}
