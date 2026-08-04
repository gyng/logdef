//! Hunger, rest, and the shift band that decides who is awake.
//!
//! **Nothing here adds an economy.** It adds *needs*, and a need is not
//! a new resource loop — it is a new customer for the loops already
//! running. Meals are bamboo the mill did not get; a sleeping crew
//! member is a pair of hands the stairs did not carry.
//!
//! The two needs fail differently on purpose, and are read differently
//! because of it. Hunger can be fixed by walking to a meal, so hunger
//! says *your chain broke*. Tiredness cannot be fixed at all except by
//! being off shift, so tiredness says *your rota is wrong, or you have
//! no beds*.
//!
//! This system runs between `defence` and `haul`. Before haul, because
//! haul both reads the work multiplier and executes every leg of going
//! to eat and going to sleep — a crew member's speed this tick and their
//! decision this tick should be about the same tick's hunger. After
//! production, so a meal cooked this tick can be eaten this tick.

use crate::content::{Content, Shift};
use crate::state::{Crew, CrewState, GameState};

use super::SoundEvent;

pub fn run(state: &mut GameState, content: &Content, sounds: &mut Vec<SoundEvent>) {
    let balance = &content.balance.crew;
    let awake_shift = shift_now(state, content);

    // The handover is the tower's one daily ritual, and the only
    // reliable way to *hear* what time it is. Emitted once for the
    // tower, not once per crew member: three people going off shift on
    // the same tick is one handover.
    if state.shift_now != awake_shift {
        state.shift_now = awake_shift;
        sounds.push(SoundEvent::ShiftChange);
    }

    let mut asleep = 0u64;
    for member in &mut state.crew {
        // Hunger rises awake or asleep. You do not stop needing to eat
        // because you are in bed.
        member.hunger = member.hunger.saturating_add(1);
        if member.is_asleep() {
            asleep += 1;
        }

        if member.shift == awake_shift {
            member.rested = member.rested.saturating_sub(1);
        } else {
            // Asleep, or on the way to bed. Only actual sleep refills:
            // walking to a bunk is still walking.
            let gain = match member.state {
                CrewState::Sleeping => {
                    if member.errand.is_some_and(|errand| errand.is_bunk()) {
                        balance.rest_gain_per_tick
                    } else {
                        balance.no_bunk_rest_gain
                    }
                }
                _ => 0,
            };
            member.rested = member
                .rested
                .saturating_add(gain)
                .min(balance.rested_max_ticks);
        }
    }
    state.stats.crew_ticks_asleep += asleep;
}

/// Which shift the current daypart belongs to.
///
/// Awake is "the current daypart belongs to my shift" and nothing else.
/// That single definition is what makes re-shifting a sleeping day
/// worker to `Night` in the middle of the night an all-hands lever with
/// a real price — they wake immediately, unrested and slow, and come
/// morning they are off shift and will sleep through the day you needed
/// them for. Built out of nothing but the definition.
#[must_use]
pub fn shift_now(state: &GameState, content: &Content) -> Shift {
    content
        .dayparts
        .get(state.clock.daypart(content).0 as usize)
        .map_or(Shift::Day, |part| part.shift)
}

/// Whether this crew member is awake right now.
#[must_use]
pub fn is_awake(crew: &Crew, state: &GameState, content: &Content) -> bool {
    crew.shift == shift_now(state, content)
}

/// How fast this crew member works, in percent of normal.
///
/// Three causes — starving, tired, and working unlit — compose
/// multiplicatively, and **never exceed 100**. A fed, rested crew member
/// in a lit tower is the baseline, not a buff: being cared for is
/// normal and neglect is what costs you. A food that made people
/// *faster* would turn the crew into a throughput stat to optimise,
/// which is the one thing `DESIGN.md`'s fourth structural call exists to
/// prevent.
///
/// Floored at 1 so the worst case is crawling rather than stopped. A
/// need that halts the tower is a death spiral rather than a pressure.
#[must_use]
pub fn work_pct(crew: &Crew, content: &Content, lit: bool) -> u32 {
    let balance = &content.balance.crew;
    let mut pct = 100u32;
    if crew.hunger >= balance.starving_ticks {
        pct = pct * balance.hungry_work_pct / 100;
    }
    if crew.rested <= balance.tired_ticks {
        pct = pct * balance.tired_work_pct / 100;
    }
    if !lit {
        // **Unless they are carrying a light.** `dark_work_pct` is the
        // penalty for working by feel; a hand lamp is the tower saying
        // "not this one". It does not end the brown-out — the lamps are
        // still out and everybody else is still slow — it makes one
        // person able to work through it, which is the triage a siege
        // asks for.
        let has_light = crew
            .kit
            .and_then(|item| content.item(item).kit.as_ref())
            .is_some_and(|kit| kit.lights_the_dark);
        if !has_light {
            pct = pct * balance.dark_work_pct / 100;
        }
    }
    pct.clamp(1, 100)
}

/// How long an action actually takes for this crew member.
///
/// **The percentage scales the duration of an action, never the
/// fixed-point step that advances it**, and that is not a stylistic
/// preference. `haul::advance` steps position by
/// `Fx::ratio(1, walk_ticks_per_slot)`, and `Fx::ratio(1, 12)` is
/// already `Fx(21)` — 1.6% off the authored rate. Scaling *that* by a
/// percentage is precisely the arithmetic that broke intake before M3:
/// `Fx::ratio(60, 1200)` truncates to `Fx(12)`, a 21-tick slot rather
/// than the 20 the constants describe, and the errors compound per
/// penalty. See `intake::terrain_effort` for the authoritative account
/// of what that class of mistake cost the game the first time.
///
/// Scaling the tick count keeps a single integer division, leaves the
/// Fx precision exactly where it already is, and makes the penalties
/// inspectable as tick counts.
#[must_use]
pub fn effective_ticks(ticks: u32, pct: u32) -> u32 {
    let pct = pct.clamp(1, 100);
    // Saturating, because a u32 tick count times 100 can overflow and a
    // wrapped duration would read as an instant action.
    ticks.saturating_mul(100) / pct
}
