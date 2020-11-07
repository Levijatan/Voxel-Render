use log::info;
use std::time::{Duration, Instant};

///A tick of `clock::Clock`
pub type Tick = u64;

///A engine Clock
pub struct Clock {
    last_tick: Tick,
    cur_tick: Tick,
    last_tick_when: Instant,
    last_render: Instant,
    delta: Duration,
    ticks_per_sec: f32,
    tick_step: f32,
}

impl Default for Clock {
    fn default() -> Self {
        Self::new(20.0)
    }
}

impl Clock {
    ///Initialized a new `clock::Clock`
    /// 
    /// # Arguments
    /// 
    /// * `ticks_per_sec` - a f32 of how many ticks that is expected to happen in 1 sec of time
    #[must_use = "To use a Clock it has to be initialized"]
    pub fn new(ticks_per_sec: f32) -> Self {
        let tick_step = 1.0 / ticks_per_sec;
        Self {
            last_tick: 0,
            cur_tick: 1,
            last_tick_when: Instant::now(),
            last_render: Instant::now(),
            delta: Duration::default(),
            ticks_per_sec,
            tick_step,
        }
    }

    ///Calculates delta since last time used and time since last time a tick happened.
    ///If time since last tick happened is greater or equal to expected tick step then a tick happens.
    pub fn tick(&mut self) {
        let now = Instant::now();

        self.delta = now.duration_since(self.last_render);
        self.last_render = now;
        let step = now.duration_since(self.last_tick_when);
        if step.as_secs_f32() >= self.tick_step {
            self.last_tick_when = now;
            self.cur_tick += 1;
            info!(
                "Tick: {} at tps: {}",
                self.cur_tick,
                1.0 / step.as_secs_f32()
            );
        }
    }

    ///Returns current tick.
    #[allow(clippy::must_use_candidate)]
    pub const fn cur_tick(&self) -> Tick {
        self.cur_tick
    }

    ///Returns last tick
    #[allow(clippy::must_use_candidate)]
    pub const fn last_tick(&self) -> Tick {
        self.last_tick
    }


    /// Returns the delta time between each call of tick()
    #[allow(clippy::must_use_candidate)]
    pub const fn delta(&self) -> std::time::Duration {
        self.delta
    }

    ///Returns true if a tick is in progress
    #[allow(clippy::must_use_candidate)]
    pub const fn do_tick(&self) -> bool {
        self.cur_tick() > self.last_tick()
    }

    ///Signal to clock that all tick releated functions are completed.
    #[must_use = "To signal that a clock tick is finished this function has be used"]
    pub fn tick_done(&mut self) -> Tick {
        self.last_tick += 1;
        self.last_tick
    }

    ///Returns how many ticks per second expected.
    #[allow(clippy::must_use_candidate)]
    pub const fn ticks_per_sec(&self) -> f32 {
        self.ticks_per_sec
    }

    ///Returns the time expected between each tick.
    #[allow(clippy::must_use_candidate)]
    pub const fn tick_step(&self) -> f32 {
        self.tick_step
    }
}
