use log::info;
use std::{time::{Duration, Instant}, sync::{Arc, RwLock}};

use super::consts;

pub type Tick = u64;

pub struct Clock {
    last_tick: Arc<RwLock<Tick>>,
    cur_tick: Tick,
    last_tick_when: Instant,
    last_render: Instant,
    delta: Duration,
}

impl Clock {
    pub fn new() -> Self {
        Self {
            last_tick: Arc::new(RwLock::new(0)),
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

    pub const fn cur_tick(&self) -> Tick {
        self.cur_tick
    }

    pub fn last_tick(&self) -> Tick {
        *self.last_tick.read().unwrap()
    }

    pub const fn delta(&self) -> std::time::Duration {
        self.delta
    }

    pub fn tick_done(&self) {
        if let Ok(mut last_tick) = self.last_tick.write() {
            *last_tick += 1;
        }
    }
}
