//! Simulation systems, run in a fixed order every tick.
//!
//! The order is load-bearing twice over. It is load-bearing for
//! determinism — reordering invalidates every golden replay, so don't;
//! extend at the ends, or split a system in place. And it is
//! load-bearing for *charge priority*: consumers draw from a shared
//! pool as they run, so who runs first is who gets served when the pool
//! is thin (`state/power.rs`).
//!
//! 1. **clock** — advance the day.
//! 2. **power income** — recompute capacity; collect from sails and burners.
//! 3. **transport** — cars move. First claim on charge: a car freezing
//!    mid-shaft should be the last thing that happens, not the first.
//! 4. **intake** — intake rooms harvest the band underfoot.
//! 5. **production** — crafting rooms advance, consume, and emit.
//! 6. **siege** — creatures approach and attack; provocation decays.
//! 7. **defence** — emplacements fire at what siege just moved.
//! 8. **needs** — hunger rises, rest drains or refills, and the shift
//!    band decides who is awake.
//! 9. **haul** — crew advance their legs, then idle crew claim work,
//!    eat, sleep, or mend.
//! 10. **repair** — crew already at damage put hit points back.
//! 11. **lighting** — lamps, after dark.
//! 12. **stride** — the tower walks, if it can still afford to, and
//!     terrain streams in ahead of it.
//!
//! Haul runs after production so crew react to the buffers this tick
//! actually produced, and after transport so they see cars where those
//! cars really are. Stride runs last because walking is the first thing
//! a tower short of charge gives up.
//!
//! Needs runs *before* haul, because haul both reads the work
//! multiplier and executes every leg of going to eat and going to
//! sleep — a crew member's speed this tick and their decision this tick
//! should be about the same tick's hunger. It runs *after* production,
//! so a meal cooked this tick is available to eat this tick rather than
//! next. And it is not inside haul: haul's job is moving people, and
//! hanging counter accrual off the top of it would bury two needs
//! inside the most intricate system in the crate.
//!
//! Inserting in the middle is normally the one thing this list forbids,
//! and it is acceptable here only because `Crew` changed shape in the
//! same milestone — the golden fixture was going stale either way. What
//! must not happen is somebody moving `needs` after `haul` to avoid the
//! insertion: it would work, deterministically and imperceptibly, and
//! it would put the decision to go and eat a tick behind the hunger
//! that motivated it for no gain. Needs also draws no charge, and
//! neither does eating or sleeping, so nothing here joins the priority
//! order.

pub mod defence;
pub mod haul;
pub mod intake;
pub mod needs;
pub mod power;
pub mod production;
pub mod repair;
pub mod siege;
pub mod stride;
pub mod transport;

use crate::content::Content;
use crate::state::GameState;

/// Things that happened this tick, for the audio layer to voice. Sound
/// is fire-and-forget: emitted here, played or dropped by JS, never
/// read back into the simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SoundEvent {
    /// An intake room pulled something out of the terrain.
    Harvest,
    /// A crafting room finished a craft.
    Craft,
    /// A crew member picked up a load.
    Pickup,
    /// A crew member or a dumbwaiter set a load down.
    Deliver,
    /// A car opened its doors and somebody moved.
    CarStop,
    /// A burner consumed a load of fuel.
    Burn,
    /// The tower crossed into a new terrain band.
    BandChange,
    /// The tower has walked out of one region and into the next.
    RegionChange,
    /// The far edge of the journey. The run is over, and not badly.
    Arrived,
    /// A wave arrived on the horizon.
    WaveArrives,
    /// Something reached the tower and started work.
    EnemyContact,
    /// A hit landed on the tower.
    Impact,
    /// A panel gave way.
    Breach,
    /// A room stopped being a room.
    Wrecked,
    /// A shaft column was cut through. The signature emergency.
    Severed,
    /// An emplacement fired.
    Shot,
    /// Something was seen off.
    EnemyDown,
    /// A shift of repair work landed.
    Repair,
    /// The Heartseed is gone.
    HeartseedLost,
    /// Somebody sat down to a meal. The warmest moment in the tower,
    /// and the audible confirmation that the kitchen chain is alive.
    MealServed,
    /// The rota turned over. The tower's one daily ritual, and the only
    /// reliable way to *hear* what time it is.
    ShiftChange,
    /// Something took a load out of an outbox and left with it.
    ///
    /// Not `Impact`: nothing was hit and nothing needs mending. What it
    /// costs is a morning's work, and it should sound like a theft
    /// rather than like a blow.
    Steal,
    /// A load went down a chute and out of the tower.
    ///
    /// Deliberately distinct from `Deliver`: something the chain worked
    /// for has just been thrown away, and a tower that is spilling is
    /// telling you something about itself. Never celebratory, never an
    /// alarm — a thing falling a long way.
    Spill,
    /// Something lost its grip and walked away.
    ///
    /// Distinct from `EnemyDown` on purpose. `Leaving` and `Dying` are
    /// distinct in state and in the snapshot but sounded identical, so
    /// walking a wave off and shooting it down were indistinguishable —
    /// and `SYSTEMS.md` §2.2 refuses to count the first as repelled. The
    /// audio has to refuse too. Never triumphant.
    EnemyLeaves,
}

/// Run exactly one simulation tick.
pub fn tick(state: &mut GameState, content: &Content, sounds: &mut Vec<SoundEvent>) {
    state.clock.advance(content);
    power::income(state, content, sounds);
    transport::run(state, content, sounds);
    intake::run(state, content, sounds);
    production::run(state, content, sounds);
    siege::run(state, content, sounds);
    defence::run(state, content, sounds);
    needs::run(state, content, sounds);
    haul::run(state, content, sounds);
    repair::run(state, content, sounds);
    power::lighting(state, content);
    stride::run(state, content, sounds);
    state.tick += 1;
}
