//! Hunger, rest, and the tiredness that decides who lies down.
//!
//! **Nothing here adds an economy.** It adds *needs*, and a need is not
//! a new resource loop — it is a new customer for the loops already
//! running. Meals are bamboo the mill did not get; a sleeping crew
//! member is a pair of hands the stairs did not carry.
//!
//! The two needs fail differently on purpose, and are read differently
//! because of it. Hunger can be fixed by walking to a meal, so hunger
//! says *your chain broke*. Tiredness can only be fixed by lying down,
//! and how fast it clears is a fact about the bed, so tiredness says
//! *you have no beds*.
//!
//! **Sleep is need-driven, and this is where the rota used to be**
//! (`SYSTEMS.md` §6.32). Until M6 a crew member was awake if and only if
//! the current daypart belonged to the shift the player had assigned
//! them, `rested` was a work-rate tax and nothing else, and the whole
//! tower lay down and stood up on the same two ticks. Measured on
//! `examples/rota.rs`, every split of that rota cost the tower 28-44% of
//! its poles and the mixed splits — the ones a thoughtful player picks —
//! were the worst option on the board.
//!
//! What replaced it: somebody works until `rested` reaches
//! `tired_ticks`, goes to bed, and gets up when it is full again. The
//! cycle is `rested_max` awake against `rested_max / rest_gain` asleep,
//! which at the shipped constants is 8,640 and 4,320 — a 12,960-tick
//! loop against a 14,400-tick day, so crew drift round the clock by
//! themselves and the tower covers its own nights without anybody being
//! assigned to one.
//!
//! **Thirteen traits became behaviour the day this changed.**
//! `tired_pct`, `rested_max_pct`, `bunk_rest_pct` and `deck_rest_pct`
//! all fed that one hidden multiplier before and now set how long
//! somebody works, how long they sleep, and therefore *when* — so
//! `sleepless` and `quick_to_tire` visibly keep different hours.
//!
//! This system runs between `defence` and `haul`. Before haul, because
//! haul both reads the work multiplier and executes every leg of going
//! to eat and going to sleep — a crew member's speed this tick and their
//! decision this tick should be about the same tick's hunger. After
//! production, so a meal cooked this tick can be eaten this tick.

use crate::content::Content;
use crate::state::{Crew, CrewState, GameState, Job};

use super::SoundEvent;

pub fn run(state: &mut GameState, content: &Content, _sounds: &mut Vec<SoundEvent>) {
    let balance = &content.balance.crew;
    let mut asleep = 0u64;
    for member in &mut state.crew {
        // Hunger rises awake or asleep. You do not stop needing to eat
        // because you are in bed.
        member.hunger = member.hunger.saturating_add(1);

        // **Practice is a tick of doing the thing.** Not a reward paid
        // out on completing a haul, which would make a run of short
        // trips worth more than the same time spent on one long one and
        // hand the player a way to farm it. Time spent is time spent.
        if let Some(job) = Job::practised_by(&member.state) {
            member.practise(job, content);
        }
        if member.is_asleep() {
            asleep += 1;
        }

        // **Awake is "not asleep", and that is the whole rule now.**
        // It used to be "the current daypart belongs to my shift",
        // which meant somebody walking to a bunk at the end of their
        // shift was already refilling, and somebody exhausted in the
        // middle of one could not refill at all. Both were artefacts of
        // the rota; neither survives it.
        if !member.is_asleep() {
            member.rested = member.rested.saturating_sub(1);
            // **A push ends when the person does.** `post_until_tired`
            // is the temporary half of stationing: *everybody on the
            // mill, now*, which is a thing a player wants and which
            // would be a trap if it outlived their attention. It
            // expires on the one clock that already means "this person
            // has given what they have" — which is now the same clock
            // that sends them to bed, so a push lasts until they are
            // tired and no longer, and the tower goes back to hauling
            // without anybody having to remember.
            if member.post_until_tired && member.rested <= tired_ticks(member, content) {
                member.stationed = None;
                member.post_until_tired = false;
            }
            member.slept = 0;
        } else {
            member.slept = member.slept.saturating_add(1);
            // Actually asleep. Walking to a bunk is still walking, and
            // is caught by the branch above.
            // **And how well they sleep is a fact about them**
            // (`SYSTEMS.md` §6.25). A nocturnal crew member rests as
            // well on bare deck as most people do in a bed, which hands
            // the tower a bunk back; a light sleeper gets almost
            // nothing from the deck and makes the second bunk a
            // decision. Applied to the *rate*, not to a cap, and the
            // rate is now how long somebody is off the floor for — so a
            // light sleeper on bare deck is not merely worse rested,
            // they are away four times as long.
            let gain = match member.state {
                CrewState::Sleeping => {
                    if member.errand.is_some_and(|errand| errand.is_bunk()) {
                        scale(
                            balance.rest_gain_per_tick,
                            member.trait_pct(content, |t| t.bunk_rest_pct),
                        )
                    } else {
                        scale(
                            balance.no_bunk_rest_gain,
                            member.trait_pct(content, |t| t.deck_rest_pct),
                        )
                    }
                }
                _ => 0,
            };
            let ceiling = rested_max(member, content);
            member.rested = member.rested.saturating_add(gain).min(ceiling);
        }
    }
    state.stats.crew_ticks_asleep += asleep;
}

/// Has this person given what they have?
///
/// **The whole of the sleep decision** (`SYSTEMS.md` §6.32): work until
/// you would start slowing down, then go to bed. It is a rule rather
/// than a number — `tired_ticks` is already defined as the point where
/// somebody flags, and it is already what `post_until_tired` expires
/// on, so a push ending and a shift ending are one idea.
///
/// **Swept, because a threshold like this invites a fitted constant.**
/// On the chain-first tower of `tests::journey`, measured as when a
/// shaft first becomes affordable:
///
/// ```text
///   trigger              lift at
///   0 (empty)            never, inside 43 minutes
///   tired_ticks          32 minutes
///   tired_ticks * 2      41 minutes
/// ```
///
/// Both directions are worse and the reasons are different, which is
/// what makes the middle a rule rather than a fit. Later means working
/// the tail of every waking life at `tired_work_pct`. Earlier means a
/// shorter cycle, and a bed is a floor above the works by design
/// (`bunk.ron`), so every cycle is a trip on the stairs that the tower
/// does not get as haul time.
///
/// **The rota reached that shaft at 26 minutes and this does not.**
/// Recorded rather than tuned away: the twelve-seed walker floor is
/// unchanged at 31-36 minutes either way, so what moved is shaft
/// affordability specifically — which `AGENTS.md` already names as the
/// largest open balance question in the project, and which belongs to
/// the difficulty pass rather than to this change.
#[must_use]
pub fn wants_sleep(crew: &Crew, content: &Content) -> bool {
    crew.rested <= tired_ticks(crew, content)
}

/// Has this person had enough — either because they are full, or
/// because the night is over?
///
/// **Nobody is ever woken by an event** — not by a wave, not by a
/// stall, not by a stalled chain. That is the one piece of the rota
/// worth keeping: if the simulation roused people when things got bad,
/// the beds would be decorative.
///
/// **But a night has a length**, and that is the second clause. A
/// bunked sleeper reaches `rested_max` in exactly
/// `rested_max_ticks / rest_gain_per_tick` ticks, so the cap never
/// binds on them. It binds on somebody who could not get a bed, and
/// what it does is turn a bed shortage back into a *tired* crew rather
/// than an absent one — which is what `bunk.ron` means by "visibly
/// degrading, never fatal". Without it a deck sleeper is off the floor
/// for twice as long as a bunked one and the shortage compounds.
///
/// The cap is the pack's figure, unscaled by traits, because it is a
/// fact about the night rather than about the person. Somebody with a
/// big tank (`tireless`, `sleepless`) simply never tops it up, and runs
/// on a partial charge for far longer — which is what those traits say
/// on the label.
#[must_use]
pub fn is_rested(crew: &Crew, content: &Content) -> bool {
    let balance = &content.balance.crew;
    let night = balance.rested_max_ticks / balance.rest_gain_per_tick.max(1);
    crew.rested >= rested_max(crew, content) || crew.slept >= night
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
///
/// **Practice is deliberately not in here.** Being fed and rested is
/// the baseline and neglect is what costs you; being *good at your job*
/// is a separate multiplier that stacks on top (`practice_pct`), and
/// keeping them apart is what stops a veteran's rank quietly cancelling
/// out an empty pantry. A starving expert is still starving.
#[must_use]
pub fn work_pct(crew: &Crew, content: &Content, lit: bool) -> u32 {
    let balance = &content.balance.crew;
    let mut pct = 100u32;
    if crew.hunger >= starving_ticks(crew, content) {
        pct = pct * balance.hungry_work_pct / 100;
    }
    if crew.rested <= tired_ticks(crew, content) {
        pct = pct * balance.tired_work_pct / 100;
    }
    if !lit {
        // **Unless they are carrying a light.** `dark_work_pct` is the
        // penalty for working by feel; a hand lamp is the tower saying
        // "not this one". It does not end the brown-out — the lamps are
        // still out and everybody else is still slow — it makes one
        // person able to work through it, which is the triage a siege
        // asks for.
        // A lent lamp or a knack for the dark — the kit can be taken
        // back and the knack came aboard with the person, and either
        // answers `dark_work_pct` the same way (`SYSTEMS.md` §6.25).
        let has_light = crew
            .kit
            .and_then(|item| content.item(item).kit.as_ref())
            .is_some_and(|kit| kit.lights_the_dark)
            || crew.traits.iter().any(|idx| {
                content
                    .traits
                    .get(idx.get())
                    .is_some_and(|def| def.sees_in_the_dark)
            });
        if !has_light {
            pct = pct * balance.dark_work_pct / 100;
        }
    }
    pct.clamp(1, 100)
}

/// How long this person goes before wanting a meal.
///
/// The pack's constant scaled by their traits (`SYSTEMS.md` §6.25).
/// Floored at one tick: a trait that took it to zero would be somebody
/// permanently at the canteen, which is a deadlock rather than an
/// appetite.
#[must_use]
pub fn hungry_ticks(crew: &Crew, content: &Content) -> u32 {
    scale(
        content.balance.crew.hungry_ticks,
        crew.trait_pct(content, |t| t.hunger_pct),
    )
}

/// The point at which this person starts working slowly for want of
/// rest. The pack's `tired_ticks` scaled by their traits.
#[must_use]
pub fn tired_ticks(crew: &Crew, content: &Content) -> u32 {
    scale(
        content.balance.crew.tired_ticks,
        crew.trait_pct(content, |t| t.tired_pct),
    )
}

/// How much work a full night buys this person.
#[must_use]
pub fn rested_max(crew: &Crew, content: &Content) -> u32 {
    scale(
        content.balance.crew.rested_max_ticks,
        crew.trait_pct(content, |t| t.rested_max_pct),
    )
}

/// The same, for the point at which hunger starts slowing somebody down.
#[must_use]
pub fn starving_ticks(crew: &Crew, content: &Content) -> u32 {
    scale(
        content.balance.crew.starving_ticks,
        crew.trait_pct(content, |t| t.hunger_pct),
    )
}

/// A tick count times a percentage, floored at one.
fn scale(ticks: u32, pct: i64) -> u32 {
    let scaled = i64::from(ticks) * pct.max(0) / 100;
    u32::try_from(scaled).unwrap_or(ticks).max(1)
}

/// How much faster this person is at a job for having done it before.
///
/// 100 at no practice, rising by `rank_bonus_pct` a rank. This is the
/// one multiplier in the game allowed above 100 — see `work_pct` for
/// why every other one is not.
#[must_use]
pub fn practice_pct(crew: &Crew, job: Job, content: &Content) -> u32 {
    100 + u32::from(crew.rank(job, content)) * content.balance.crew.rank_bonus_pct
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
///
/// The ceiling is 1,000 rather than 100 because `practice_pct` composes
/// into the figure passed in and is allowed to push it past par. It is
/// still a ceiling: an action that took a tenth of its authored time
/// would be an action the player cannot see happening.
#[must_use]
pub fn effective_ticks(ticks: u32, pct: u32) -> u32 {
    let pct = pct.clamp(1, 1000);
    // Saturating, because a u32 tick count times 100 can overflow and a
    // wrapped duration would read as an instant action.
    ticks.saturating_mul(100) / pct
}
