use log::info;
use std::time::{Duration, Instant};

use super::consts;

pub struct Clock {
    last_tick: u64,
    cur_tick: u64,
    last_tick_when: Instant,
    last_render: Instant,
    delta: Duration,
}

impl Clock {
    pub fn new() -> Self {
        Self {
            last_tick: 0,
            cur_tick: 0,
            last_tick_when: Instant::now(),
            last_render: Instant::now(),
            delta: Duration::default(),
        }
    }
    #[optick_attr::profile]
    pub fn tick(&mut self) {
        let now = Instant::now();

        self.delta = now.duration_since(self.last_render);
        self.last_render = now;
        let step = now.duration_since(self.last_tick_when);
        if step.as_secs_f32() >= consts::TICK_STEP {
            self.last_tick_when = now;
            self.cur_tick += 1;
            info!(
                "Tick: {} at tps: {}",
                self.cur_tick,
                1.0 / step.as_secs_f32()
            );
        }
    }

    pub const fn cur_tick(&self) -> u64 {
        self.cur_tick
    }

    pub const fn last_tick(&self) -> u64 {
        self.last_tick
    }

    pub const fn delta(&self) -> std::time::Duration {
        self.delta
    }

    pub fn tick_done(&mut self) {
        self.last_tick += 1;
    }
}
