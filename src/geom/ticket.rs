use super::chunk;
use super::util;
use super::world;

use anyhow::{Result, Error};
use thiserror::Error;
use legion::Entity;
use std::{collections::VecDeque, sync::RwLock};
use log::info;

pub type Arena = generational_arena::Arena<Ticket>;

pub struct Queue {
    queue: RwLock<VecDeque<Propagate>>,
}

impl Queue {
    pub fn new() -> Self {
        Self{queue: RwLock::new(VecDeque::new())}
    }

    pub fn pop(&self) -> Option<Propagate> {
        let mut queue = self.queue.write().unwrap();
        queue.pop_front()
    }

    pub fn push(&self, value: Propagate) {
        let mut queue = self.queue.write().unwrap();
        queue.push_back(value);
    }
}

//direction to propagate (None for all), pos to move from, how many steps left of prop, is a prop branch, ticket index
pub type Propagate = (Option<util::Direction>, chunk::Position, u32, bool, generational_arena::Index);

//the vec is the new commands for the ticket queue.
type PropagateReturn = Option<Vec<Propagate>>;

#[derive(Error, Debug)]
pub enum TError {
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
pub fn add(
    world_id: Entity,
    world: &world::Map,
    clock: &crate::clock::Clock,
    cmd: &mut legion::systems::CommandBuffer,
    arena: &mut Arena,
    queue: &Queue,
) {
    info!("***Adding Player Ticket***");
    if clock.cur_tick() % 20 == 0 || clock.cur_tick() == 1 {
        let pos = chunk::Position {
            pos: glm::vec3(0, 0, 0),
        };

        let ticket = Ticket {
            start_time: clock.cur_tick(),
            ttl: 40,
            pos,
        };

        info!("Adding {:#?} at {:#?}", ticket, pos);

        world.chunk_add(ticket, world_id, &pos, cmd, arena, queue);
    }
}

pub fn update(ticket: &Ticket, cur_tick: u64) -> Result<()> {
    if ticket.start_time + ticket.ttl <= cur_tick {
        Err(Error::new(TError::OutsideTTL))
    } else {
        Ok(())
    }
}

pub fn propagate(
    prop: Propagate,
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
) -> Result<Vec<Propagate>> {
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
) -> Result<Vec<Propagate>> {
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
) -> Result<Vec<Propagate>> {
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
) -> Result<Propagate> {
    let dir_pos = pos.neighbor(dir)?;
    priority -= 1;
    Ok((Some(dir), dir_pos, priority, branch, idx))
}
