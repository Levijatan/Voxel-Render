use super::chunk;
use super::util;
use super::world;

use anyhow::{Result, Error};
use thiserror::Error;
use legion::Entity;



#[derive(Error, Debug)]
pub enum TicketError {
    #[error("Out of life")]
    OutsideTTL,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Ticket {
    pub start_time: u64,
    pub ttl: u64,
    pub pos: chunk::Position,
}

//TODO: add player position
pub fn add_ticket(
    world_id: Entity,
    world: &mut world::World,
    clock: &crate::clock::Clock,
    cmd: &mut legion::systems::CommandBuffer,
) {
    if clock.cur_tick() % 20 == 0 || clock.cur_tick() == 9 {
        let pos = chunk::Position {
            pos: glm::vec3(0, 0, 0),
            world_id,
        };

        let ticket = Ticket {
            start_time: clock.cur_tick(),
            ttl: 40,
            pos,
        };

        world.chunk_add_ticket(ticket, &pos, cmd);
    }
}

pub fn update(ticket: &Ticket, cur_tick: u64) -> Result<()> {
    if ticket.start_time + ticket.ttl <= cur_tick {
        Err(Error::new(TicketError::OutsideTTL))
    } else {
        Ok(())
    }
}

//direction to propagate (None for all), pos to move from, how many steps left of prop, is a prop branch, ticket index
pub type PropagateTicket = (Option<util::Direction>, chunk::Position, u32, bool, generational_arena::Index);

//the vec is the new commands for the ticket queue.
type PropagateReturn = Option<Vec<PropagateTicket>>;

pub fn propagate(
    prop: PropagateTicket,
) -> Result<PropagateReturn> {
    use util::Direction::{Down, Up};

    Ok(match prop {
        (_, _, 0, _, _) => None,
        (Some(Down), pos, priority, _, idx) => Some(propagate_up_or_down(pos, priority, idx, false)?),
        (Some(Up), pos, priority, _, idx) => Some(propagate_up_or_down(pos, priority, idx, true)?),
        (Some(dir), pos, priority, branch, idx) => Some(propagate_rest(pos, priority, idx, branch, dir)?),
        (None, pos, priority, _, idx) => Some(propagate_start(pos, priority, idx)?),
    })
}

fn propagate_rest(
    pos: chunk::Position,
    priority: u32,
    idx: generational_arena::Index,
    branch: bool,
    dir: util::Direction,
) -> Result<Vec<PropagateTicket>> {
    let mut out = Vec::new();
    out.push(propagate_ticket(dir, pos, priority, branch, idx)?);
    if !branch {
        let b_dir = util::go_left(dir)?;
        out.push(propagate_ticket(b_dir, pos, priority, true, idx)?);
    }
    Ok(out)
}

fn propagate_up_or_down(
    pos: chunk::Position,
    priority: u32,
    idx: generational_arena::Index,
    up: bool,
) -> Result<Vec<PropagateTicket>> {
    let skip = if up {
        util::Direction::Down
    } else {
        util::Direction::Up
    };
    Ok(util::ALL_DIRECTIONS.iter().filter(|&dir| *dir != skip).map(|dir| {
        propagate_ticket(*dir, pos, priority, false, idx)
    }).filter_map(Result::ok).collect::<Vec<_>>())
}

fn propagate_start(
    pos: chunk::Position,
    priority: u32,
    idx: generational_arena::Index,
) -> Result<Vec<PropagateTicket>> {
    Ok(util::ALL_DIRECTIONS.iter().map(|dir| {
        propagate_ticket(*dir, pos, priority, false, idx)
    }).filter_map(Result::ok).collect::<Vec<_>>())
}

#[optick_attr::profile]
fn propagate_ticket(
    dir: util::Direction,
    pos: chunk::Position,
    mut priority: u32,
    branch: bool,
    idx: generational_arena::Index,
) -> Result<PropagateTicket> {
    let dir_pos = pos.neighbor(dir)?;
    priority -= 1;
    Ok((Some(dir), dir_pos, priority, branch, idx))
}
