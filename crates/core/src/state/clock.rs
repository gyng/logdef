//! The day.
//!
//! What reads this: the sun, which sets the garden's rate and decides
//! when the lamps come on, and the elevator, which can run a different
//! program after dark. Everything is derived from a single counter, so
//! the clock costs one increment a tick and nothing else.
//!
//! **Nothing about sleep reads it any more** (`SYSTEMS.md` §6.32).
//! Crew lie down when they are tired and get up when they are rested,
//! so the day no longer decides who is awake — it decides what the
//! light is like while they work.

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
    /// A run begins at permille 0, which is predawn.
    ///
    /// **It used to begin at the handover onto the day shift**, and the
    /// reason was the rota: every crew member defaulted to `Day`, so a
    /// run opened at predawn was a run that opened with the entire crew
    /// asleep for 2,592 ticks — about 86 seconds at 1x of nothing
    /// moving, which reads as a broken game to somebody who has not yet
    /// been taught what a rota is.
    ///
    /// M6 cut the rota (`SYSTEMS.md` §6.32) and that reason went with
    /// it. Crew now come aboard rested and stay up until they are
    /// tired, so what time the tower sets out no longer decides whether
    /// anybody is standing. Starting at 0 is the simpler statement, and
    /// it is the one `power.starting_charge` was already tuned against:
    /// its note reads "tick 0 is predawn" and says 800 covers the
    /// ~2,200 ticks until real daylight.
    #[must_use]
    pub fn new(_content: &Content) -> Self {
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

// No `Default`: where a run starts in the day is a fact about the
// content pack (see `Clock::new`), and a `Clock` conjured without one
// would silently start at predawn with the whole crew asleep.
