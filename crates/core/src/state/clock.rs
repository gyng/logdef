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
    /// A run begins at the handover onto the day shift.
    ///
    /// **Not at permille 0**, which is predawn, which is the night band.
    /// Until M4 the clock started there and nothing minded: it meant a
    /// tower that set out in the dark with its sails idle for the first
    /// minute and a half, which was atmospheric and cost nothing. The
    /// rota made it cost something. Every crew member defaults to the
    /// day shift — the tower a player who never opens the roster gets —
    /// so starting at predawn would open every run with the entire crew
    /// asleep for 2,592 ticks, about 86 seconds at 1x. Nothing moves,
    /// nothing is hauled, and the only reading available to somebody
    /// who has not yet been taught what a rota is, is that the game is
    /// broken.
    ///
    /// So the tower sets out in the morning. Derived from the pack
    /// rather than hardcoded, because which daypart opens the day shift
    /// is a designer's decision (`content::validate_rota`).
    #[must_use]
    pub fn new(content: &Content) -> Self {
        Self {
            tick_of_day: content.day_shift_start_tick(),
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
