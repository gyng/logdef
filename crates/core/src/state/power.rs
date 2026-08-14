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
//! defence, then lighting, then striding. So the tower stops walking
//! before the chain stalls, and a car freezing mid-shaft only happens
//! when things are genuinely dire — which is what makes that moment
//! land.
//!
//! **There are two constraints on a withdrawal, not one** (`SYSTEMS.md`
//! §6.36). The bank is a reservoir; the *rail* is the pipe out of it,
//! and a tower cannot spend faster than its burners and cell banks can
//! deliver however full the bank happens to be. That is what turns
//! charge from a stock into a supply: a stock only forces a choice at
//! the boundary of running out, by which point the tower is already in
//! a spiral, and a ranking that only matters in a spiral is a ranking
//! nobody gets to use.
//!
//! A failed draw never goes into debt. The consumer just does not act
//! this tick.

use crate::fx::Fx;
use serde::{Deserialize, Serialize};

/// The five things that spend charge, in the order they spend it.
///
/// **Charge priority used to *be* the tick order** — lifts drew first
/// because transport runs first, legs last because striding runs last —
/// and it was not a decision anybody could make. This is the same
/// draws with a ranking the player owns, which is the one piece of FTL's
/// reactor that Understory already had the wiring for.
///
/// The discriminants are the *tick* positions and never move; the
/// player's ranking is a separate list. Both are needed, because a class
/// that draws early in the tick but ranks low has to leave room for one
/// that draws later and ranks high.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerUse {
    /// Elevator and dumbwaiter cars, per floor travelled.
    Lifts,
    /// Every room with a `power_draw`: mills, forges, thornwrights.
    Works,
    /// Emplacements, per shot (`SYSTEMS.md` §6.36).
    ///
    /// **Its own circuit rather than part of `Works`, and the reason is
    /// the reserve rather than tidiness.** `defence` runs at step 7 of
    /// the tick and `production` at step 5, so a gun drawing under
    /// `Works`'s tick position would claim to have already spent when it
    /// had not, and every reservation computed against it would be
    /// wrong.
    ///
    /// It is also the one decision the rail exists to create: *shed the
    /// mill, keep the guns* — or the reverse, which is a real answer for
    /// a tower that would rather lose a panel than lose an afternoon's
    /// poles. Folded into `Works`, neither is expressible.
    Guns,
    /// Keeping the decks lit after dark.
    Lamps,
    /// Walking.
    Legs,
}

impl PowerUse {
    pub const ALL: [Self; 5] = [
        Self::Lifts,
        Self::Works,
        Self::Guns,
        Self::Lamps,
        Self::Legs,
    ];

    /// Where in the tick this one spends. Fixed by `systems::tick` and
    /// not something the player can change — reordering the tick would
    /// invalidate every replay (`DECISIONS.md` §1).
    #[must_use]
    pub const fn tick_position(self) -> usize {
        match self {
            Self::Lifts => 0,
            Self::Works => 1,
            Self::Guns => 2,
            Self::Lamps => 3,
            Self::Legs => 4,
        }
    }

    #[must_use]
    pub const fn index(self) -> usize {
        self.tick_position()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Power {
    pub charge: i64,
    /// Summed from cell banks each tick. Zero banks means zero storage,
    /// and income that arrives with nowhere to go is lost.
    pub capacity: i64,
    /// Sub-unit accumulation of the Heartseed's trickle, so six per
    /// hundred ticks still adds up instead of truncating to nothing.
    pub trickle_acc: Fx,
    /// Hundredths of a charge owed to the bank, and by it. Rates run in
    /// charge per 100 ticks (see [`PER_TICK`]), so the whole-charge
    /// remainder has to be carried between ticks or a tower spending
    /// 0.2 a tick would spend nothing at all.
    #[serde(default)]
    pub spend_acc: i64,
    #[serde(default)]
    pub fill_acc: i64,
    /// Charge added last tick. Presentation only.
    pub income_last: i64,
    /// Charge drawn last tick. Presentation only.
    pub spent_last: i64,
    /// Something ran short this tick. What the cross-section reads to
    /// dim the tower.
    pub brownout: bool,
    /// Lamps are lit — it is dark enough to need them and the tower
    /// could serve them.
    pub lit: bool,
    /// **Generation capacity: the most charge the tower could deliver
    /// this tick**, from burners, cell banks and the Heartseed.
    ///
    /// Factorio's number, and it is a *ceiling* rather than an income —
    /// a burner only consumes fuel for what is actually drawn.
    #[serde(default)]
    pub rail: i64,
    /// What the player wants kept running when charge is short, best
    /// first. Defaults to the order the tick already spent in, so an
    /// untouched tower behaves exactly as it did before this existed.
    pub priority: Vec<PowerUse>,
    /// What each circuit asked for this tick, indexed by
    /// [`PowerUse::index`].
    pub demand: Vec<i64>,
    /// **How well each circuit was served, in per-mille**, indexed by
    /// [`PowerUse::index`]. 1000 is everything it asked for.
    ///
    /// **The whole of `SYSTEMS.md` §6.39.** A draw used to be binary —
    /// paid in full or refused — which made the smallest expressible
    /// `power_draw` an enormous commitment, forced block purchases for
    /// anything cheaper than a charge a tick, and meant a tower one
    /// joule short stopped dead rather than slowing down.
    ///
    /// Now a circuit is served a fraction and everything on it runs at
    /// that fraction: the mill turns slower, the legs walk slower, the
    /// guns reload slower. Proportionality lives *inside* a circuit;
    /// [`Self::priority`] orders *between* circuits, which is how a
    /// switchboard sheds load.
    #[serde(default)]
    pub satisfaction: Vec<i64>,
    /// Stored charge physically attached to each floor's automatic bus.
    /// Keeping it by floor means severing a riser forms honest islands:
    /// a bank remains with the deck it was built on rather than being
    /// silently available through the global total.
    #[serde(default)]
    pub floor_charge: Vec<i64>,
    /// Per-floor circuit service, indexed `[floor][PowerUse]`.
    #[serde(default)]
    pub floor_satisfaction: Vec<Vec<i64>>,
    #[serde(default)]
    pub floor_spend_acc: Vec<i64>,
    #[serde(default)]
    pub floor_fill_acc: Vec<i64>,
}

/// Full service, in the per-mille this module works in.
pub const FULL: i64 = 1000;

/// **Rates are carried in charge per 100 ticks**, which is the unit the
/// balance pack authors them in ("rates are written the way a designer
/// thinks about them") and the only one that keeps them whole numbers.
///
/// Striding is 20 per 100 ticks — 0.2 a tick — and a demand table in
/// charge-per-tick would round that to nothing. Every figure in
/// [`Power::demand`], and the supply handed to [`Power::allocate`], is
/// in these units; [`Power::settle`] converts back to whole charge with
/// the remainder carried, so nothing is lost to truncation.
pub const PER_TICK: i64 = 100;

impl Power {
    #[must_use]
    pub fn new(starting_charge: i64) -> Self {
        Self {
            charge: starting_charge,
            capacity: starting_charge,
            trickle_acc: Fx::ZERO,
            spend_acc: 0,
            fill_acc: 0,
            income_last: 0,
            spent_last: 0,
            brownout: false,
            lit: false,
            rail: 0,
            priority: vec![
                PowerUse::Lifts,
                PowerUse::Works,
                PowerUse::Guns,
                PowerUse::Lamps,
                PowerUse::Legs,
            ],
            demand: vec![0; PowerUse::ALL.len()],
            satisfaction: vec![FULL; PowerUse::ALL.len()],
            floor_charge: Vec::new(),
            floor_satisfaction: Vec::new(),
            floor_spend_acc: Vec::new(),
            floor_fill_acc: Vec::new(),
        }
    }

    /// How well this circuit is being served, in per-mille.
    #[must_use]
    pub fn served(&self, use_: PowerUse) -> i64 {
        self.satisfaction
            .get(use_.index())
            .copied()
            .unwrap_or(FULL)
            .clamp(0, FULL)
    }

    /// Service on one deck. Old saves and unit tests without topology
    /// state fall back to the tower-wide value.
    #[must_use]
    pub fn served_at(&self, floor: u8, use_: PowerUse) -> i64 {
        self.floor_satisfaction
            .get(floor as usize)
            .and_then(|row| row.get(use_.index()))
            .copied()
            .unwrap_or_else(|| self.served(use_))
            .clamp(0, FULL)
    }

    /// Was this circuit given less than it asked for?
    #[must_use]
    pub fn short(&self, use_: PowerUse) -> bool {
        self.served(use_) < FULL
    }

    /// Divide this tick's supply between the circuits and record how
    /// well each one came out of it.
    ///
    /// **Served in the player's order, each in full until the supply
    /// runs out.** Everything above the cut runs at its full rate, one
    /// circuit runs partially, and anything below gets nothing — which
    /// is a switchboard shedding load rather than a pool being raced
    /// for. Inside a circuit the fraction applies to every room on it
    /// equally, so a half-served Works is every mill turning at half
    /// speed rather than half the mills stopped.
    ///
    /// Returns how much was actually taken, which is what the bank and
    /// the fuel are charged for. **Nothing is spent on a circuit that
    /// was not served**, so a tower short of charge burns less fuel
    /// rather than burning the same amount for less work.
    pub fn allocate(&mut self, supply: i64) -> i64 {
        if self.satisfaction.len() != PowerUse::ALL.len() {
            self.satisfaction = vec![FULL; PowerUse::ALL.len()];
        }
        let mut left = supply.max(0);
        let mut taken = 0;
        for at in 0..self.priority.len() {
            let index = self.priority[at].index();
            let want = self.demand.get(index).copied().unwrap_or(0).max(0);
            if want <= 0 {
                // Nothing asked for is fully served by definition. Saying
                // otherwise would light the brown-out flag for circuits a
                // tower does not even have.
                self.satisfaction[index] = FULL;
                continue;
            }
            let got = want.min(left);
            left -= got;
            taken += got;
            self.satisfaction[index] = got * FULL / want;
        }
        self.brownout = PowerUse::ALL.iter().any(|use_| self.short(*use_));
        self.spent_last = taken;
        taken
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
        self.charge * FULL / self.capacity
    }

    /// Turn a rate in charge-per-100-ticks into whole charge, carrying
    /// the remainder so nothing is lost to truncation.
    fn take_whole(acc: &mut i64, rate: i64) -> i64 {
        *acc += rate.max(0);
        let whole = *acc / PER_TICK;
        *acc -= whole * PER_TICK;
        whole
    }

    /// Move this tick's allocated rates into the bank: what the tower
    /// drew out of storage, and what its spare generation put back.
    pub fn settle(&mut self, from_bank_rate: i64, to_bank_rate: i64) {
        let drawn = Self::take_whole(&mut self.spend_acc, from_bank_rate);
        self.charge = (self.charge - drawn).max(0);
        let filled = Self::take_whole(&mut self.fill_acc, to_bank_rate);
        self.add(filled);
    }

    /// Clear the per-tick counters. Called once at the top of the tick,
    /// before anything is allocated.
    pub fn begin_tick(&mut self) {
        self.income_last = 0;
        self.spent_last = 0;
        self.brownout = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pool(charge: i64, demand: &[(PowerUse, i64)]) -> Power {
        let mut power = Power::new(charge);
        for (use_, want) in demand {
            power.demand[use_.index()] = *want;
        }
        power
    }

    #[test]
    fn a_supply_that_covers_everything_serves_everything() {
        let mut power = pool(100, &[(PowerUse::Works, 10), (PowerUse::Legs, 5)]);
        assert_eq!(power.allocate(100), 15, "only what was asked for is taken");
        assert_eq!(power.served(PowerUse::Works), FULL);
        assert_eq!(power.served(PowerUse::Legs), FULL);
        assert!(!power.brownout);
    }

    #[test]
    fn a_circuit_that_asks_for_nothing_is_not_a_brown_out() {
        // A tower with no shaft has no Lifts demand at all, and reporting
        // that as an unserved circuit would light the flag permanently.
        let mut power = pool(100, &[]);
        power.allocate(0);
        assert!(!power.brownout);
        for use_ in PowerUse::ALL {
            assert_eq!(power.served(use_), FULL, "{use_:?}");
        }
    }

    #[test]
    fn a_short_supply_is_served_in_the_players_order() {
        // Lifts and Works fully, Guns half, and nothing below it — a
        // switchboard shedding load rather than a pool being raced for.
        let mut power = pool(
            100,
            &[
                (PowerUse::Lifts, 10),
                (PowerUse::Works, 10),
                (PowerUse::Guns, 20),
                (PowerUse::Lamps, 10),
            ],
        );
        assert_eq!(power.allocate(30), 30, "everything available is used");
        assert_eq!(power.served(PowerUse::Lifts), FULL);
        assert_eq!(power.served(PowerUse::Works), FULL);
        assert_eq!(power.served(PowerUse::Guns), 500, "half of what it asked");
        assert_eq!(power.served(PowerUse::Lamps), 0);
        assert!(power.brownout);
    }

    #[test]
    fn reordering_moves_which_circuit_goes_short() {
        // The whole point of the ranking, and now it is continuous:
        // the same tower under a different order serves different
        // fractions rather than flipping something on or off.
        let demand = [(PowerUse::Lifts, 20), (PowerUse::Legs, 20)];
        let mut first = pool(100, &demand);
        assert_eq!(first.allocate(30), 30);
        assert_eq!(first.served(PowerUse::Lifts), FULL);
        assert_eq!(first.served(PowerUse::Legs), 500);

        let mut second = pool(100, &demand);
        second.priority = vec![
            PowerUse::Legs,
            PowerUse::Lifts,
            PowerUse::Works,
            PowerUse::Guns,
            PowerUse::Lamps,
        ];
        assert_eq!(second.allocate(30), 30);
        assert_eq!(second.served(PowerUse::Legs), FULL);
        assert_eq!(second.served(PowerUse::Lifts), 500);
    }

    #[test]
    fn nothing_is_spent_on_a_circuit_that_was_not_served() {
        // **The Factorio property, and the reason a poor tower is not
        // also a wasteful one.** A refused draw used to cost nothing but
        // achieve nothing; an unserved circuit now simply is not paid
        // for, so the fuel bill falls with the work done.
        let mut power = pool(100, &[(PowerUse::Works, 50), (PowerUse::Legs, 50)]);
        assert_eq!(
            power.allocate(50),
            50,
            "the supply is spent, not the demand"
        );
        assert_eq!(power.served(PowerUse::Legs), 0);
    }

    #[test]
    fn a_tower_with_no_supply_serves_nothing_and_says_so() {
        let mut power = pool(0, &[(PowerUse::Legs, 20)]);
        assert_eq!(power.allocate(0), 0);
        assert_eq!(power.served(PowerUse::Legs), 0);
        assert!(power.brownout);
    }

    #[test]
    fn income_is_capped_at_capacity_and_the_overflow_is_lost() {
        // Losing the overflow is the design: it is the signal that the
        // tower needs another cell bank.
        let mut power = Power::new(90);
        power.capacity = 100;
        power.add(50);
        assert_eq!(power.charge, 100);
        assert_eq!(power.income_last, 10);
    }

    #[test]
    fn fill_reads_zero_rather_than_dividing_by_zero() {
        let mut empty = Power::new(0);
        empty.capacity = 0;
        assert_eq!(empty.fill_permille(), 0);
        let mut half = Power::new(50);
        half.capacity = 100;
        assert_eq!(half.fill_permille(), 500);
    }
}
