//! Charge — the keystone.
//!
//! One pool, with a capacity summed from the cell banks the player has
//! built. Charge is never a crate: it is not hauled, does not sit on a
//! shelf, and cannot be carried up the stairs. That is what makes it a
//! different kind of pressure from everything else in the tower.
//!
//! **Priority is execution order.** Consumers call [`Power::draw`] from
//! inside their own system, and the tick's system order decides who
//! gets served when the pool is thin: transport, then production, then
//! lighting, then striding. So the tower stops walking before the chain
//! stalls, and a car freezing mid-shaft only happens when things are
//! genuinely dire — which is what makes that moment land.
//!
//! A failed draw never goes into debt. The consumer just does not act
//! this tick.

use serde::{Deserialize, Serialize};

use crate::fx::Fx;

/// Which prepaid meter a block purchase belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Credit {
    Stride,
    Light,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Power {
    pub charge: i64,
    /// Summed from cell banks each tick. Zero banks means zero storage,
    /// and income that arrives with nowhere to go is lost.
    pub capacity: i64,
    /// Sub-unit solar accumulation, so a trickle of sun still adds up.
    pub solar_acc: Fx,
    /// Charge added last tick. Presentation only.
    pub income_last: i64,
    /// Charge drawn last tick. Presentation only.
    pub spent_last: i64,
    /// Some draw failed last tick. What the cross-section reads to dim
    /// the tower.
    pub brownout: bool,
    /// Lamps are lit — it is dark enough to need them and there was
    /// charge to spare.
    pub lit: bool,
    /// Ticks of striding already paid for. See [`Self::buy_block`].
    pub stride_credit: u32,
    /// Ticks of lighting already paid for.
    pub light_credit: u32,
}

impl Power {
    #[must_use]
    pub const fn new(starting_charge: i64) -> Self {
        Self {
            charge: starting_charge,
            capacity: starting_charge,
            solar_acc: Fx::ZERO,
            income_last: 0,
            spent_last: 0,
            brownout: false,
            lit: false,
            stride_credit: 0,
            light_credit: 0,
        }
    }

    /// Pay for a hundred ticks of something up front, then spend that
    /// credit a tick at a time.
    ///
    /// The obvious alternative — charge `rate / 100` every tick — is
    /// wrong in a way that hides: a rate of 20 per 100 ticks truncates
    /// to zero charge on eighty ticks out of every hundred, so a tower
    /// with an empty bank would keep walking for free most of the time.
    /// Buying in blocks means running out actually stops you, and it
    /// makes the draw visible as a periodic bite rather than a trickle.
    pub fn buy_block(&mut self, per_100: i64, credit: Credit) -> bool {
        let held = match credit {
            Credit::Stride => &mut self.stride_credit,
            Credit::Light => &mut self.light_credit,
        };
        if *held > 0 {
            *held -= 1;
            return true;
        }
        // Free is free: a zero rate never needs buying.
        if per_100 <= 0 {
            return true;
        }
        if !self.draw(per_100) {
            return false;
        }
        match credit {
            Credit::Stride => self.stride_credit = 99,
            Credit::Light => self.light_credit = 99,
        }
        true
    }

    /// Take `amount` if the pool can cover it. Returns whether it could.
    /// A refusal sets `brownout`, which is the only place that flag is
    /// raised.
    pub fn draw(&mut self, amount: i64) -> bool {
        if amount <= 0 {
            return true;
        }
        if self.charge < amount {
            self.brownout = true;
            return false;
        }
        self.charge -= amount;
        self.spent_last += amount;
        true
    }

    /// Add charge, discarding anything past capacity. Overflow is not
    /// an error — it is the signal that you need another cell bank.
    pub fn add(&mut self, amount: i64) {
        if amount <= 0 {
            return;
        }
        let room = (self.capacity - self.charge).max(0);
        let taken = amount.min(room);
        self.charge += taken;
        self.income_last += taken;
    }

    /// Fraction of capacity currently stored, in per-mille. Zero
    /// capacity reads as empty rather than dividing by zero.
    #[must_use]
    pub fn fill_permille(&self) -> i64 {
        if self.capacity <= 0 {
            return 0;
        }
        self.charge * 1000 / self.capacity
    }

    /// Clear the per-tick counters. Called once at the top of the tick,
    /// before anything draws.
    pub fn begin_tick(&mut self) {
        self.income_last = 0;
        self.spent_last = 0;
        self.brownout = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pool(charge: i64, capacity: i64) -> Power {
        let mut power = Power::new(charge);
        power.capacity = capacity;
        power
    }

    #[test]
    fn a_draw_within_budget_succeeds_and_is_counted() {
        let mut power = pool(100, 100);
        assert!(power.draw(30));
        assert_eq!(power.charge, 70);
        assert_eq!(power.spent_last, 30);
        assert!(!power.brownout);
    }

    #[test]
    fn a_draw_over_budget_takes_nothing_and_raises_brownout() {
        // Partial payment would be worse than refusal: half a floor of
        // elevator travel is not a thing, and a consumer that spent
        // charge without acting is charge that vanished.
        let mut power = pool(10, 100);
        assert!(!power.draw(11));
        assert_eq!(power.charge, 10);
        assert_eq!(power.spent_last, 0);
        assert!(power.brownout);
    }

    #[test]
    fn drawing_exactly_the_balance_is_allowed() {
        let mut power = pool(10, 100);
        assert!(power.draw(10));
        assert_eq!(power.charge, 0);
        assert!(!power.brownout);
    }

    #[test]
    fn a_zero_draw_is_free_and_never_browns_out() {
        let mut power = pool(0, 100);
        assert!(power.draw(0));
        assert!(!power.brownout);
        assert_eq!(power.spent_last, 0);
    }

    #[test]
    fn income_is_capped_at_capacity_and_the_overflow_is_lost() {
        // Losing the overflow is the design: it is the signal that the
        // tower needs another cell bank.
        let mut power = pool(90, 100);
        power.add(50);
        assert_eq!(power.charge, 100);
        assert_eq!(power.income_last, 10);
    }

    #[test]
    fn income_into_a_full_pool_counts_nothing() {
        let mut power = pool(100, 100);
        power.add(25);
        assert_eq!(power.charge, 100);
        assert_eq!(power.income_last, 0);
    }

    #[test]
    fn fill_reads_zero_rather_than_dividing_by_zero() {
        assert_eq!(pool(0, 0).fill_permille(), 0);
        assert_eq!(pool(50, 100).fill_permille(), 500);
        assert_eq!(pool(100, 100).fill_permille(), 1000);
    }

    #[test]
    fn a_block_is_bought_once_and_then_spent_a_tick_at_a_time() {
        // The whole reason blocks exist: a rate of 20 per 100 ticks
        // must actually cost 20, not round to nothing eighty times.
        let mut power = pool(100, 100);
        assert!(power.buy_block(20, Credit::Stride));
        assert_eq!(power.charge, 80, "the first tick pays for the block");

        for _ in 0..99 {
            assert!(power.buy_block(20, Credit::Stride));
        }
        assert_eq!(power.charge, 80, "the next 99 ticks ride on the credit");

        assert!(power.buy_block(20, Credit::Stride));
        assert_eq!(power.charge, 60, "tick 101 buys the next block");
    }

    #[test]
    fn an_unaffordable_block_stops_the_consumer_dead() {
        let mut power = pool(5, 100);
        assert!(!power.buy_block(20, Credit::Stride));
        assert_eq!(power.charge, 5);
        assert!(power.brownout);
    }

    #[test]
    fn the_two_meters_are_independent() {
        // Striding and lighting must not spend each other's credit, or
        // halting the legs would silently buy free lamps.
        let mut power = pool(100, 100);
        assert!(power.buy_block(20, Credit::Stride));
        assert!(power.buy_block(10, Credit::Light));
        assert_eq!(power.charge, 70);
        assert_eq!(power.stride_credit, 99);
        assert_eq!(power.light_credit, 99);
    }

    #[test]
    fn a_free_rate_needs_no_purchase() {
        let mut power = pool(0, 0);
        assert!(power.buy_block(0, Credit::Light));
        assert!(!power.brownout);
    }
}
