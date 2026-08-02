//! The day.
//!
//! Two things read this: the sun, which decides how much charge the
//! sails make, and the elevator, which can run a different program on
//! the night shift. Everything is derived from a single counter, so the
//! clock costs one increment a tick and nothing else.

use serde::{Deserialize, Serialize};

use crate::content::Content;
use crate::ids::DaypartIdx;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Clock {
    /// Ticks elapsed in the current day.
    pub tick_of_day: u32,
    /// Days completed since the run began.
    pub day: u32,
}

impl Clock {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            tick_of_day: 0,
            day: 0,
        }
    }

    pub fn advance(&mut self, content: &Content) {
        let length = content.balance.clock.ticks_per_day.max(1);
        self.tick_of_day += 1;
        if self.tick_of_day >= length {
            self.tick_of_day = 0;
            self.day += 1;
        }
    }

    /// How far through the day we are, in per-mille.
    #[must_use]
    pub fn permille(&self, content: &Content) -> i64 {
        let length = i64::from(content.balance.clock.ticks_per_day.max(1));
        i64::from(self.tick_of_day) * 1000 / length
    }

    #[must_use]
    pub fn daypart(&self, content: &Content) -> DaypartIdx {
        content.daypart_at(self.permille(content))
    }

    /// Sunlight before terrain, as a percentage.
    #[must_use]
    pub fn sun_pct(&self, content: &Content) -> i64 {
        content.sun_pct_at(self.permille(content))
    }
}

impl Default for Clock {
    fn default() -> Self {
        Self::new()
    }
}
