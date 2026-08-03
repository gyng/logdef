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
//! 8. **haul** — crew advance their legs, then idle crew claim work.
//! 9. **repair** — crew already at damage put hit points back.
//! 10. **lighting** — lamps, after dark.
//! 11. **stride** — the tower walks, if it can still afford to, and
//!     terrain streams in ahead of it.
//!
//! Haul runs after production so crew react to the buffers this tick
//! actually produced, and after transport so they see cars where those
//! cars really are. Stride runs last because walking is the first thing
//! a tower short of charge gives up.

pub mod defence;
pub mod haul;
pub mod intake;
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
    haul::run(state, content, sounds);
    repair::run(state, content, sounds);
    power::lighting(state, content);
    stride::run(state, content, sounds);
    state.tick += 1;
}
