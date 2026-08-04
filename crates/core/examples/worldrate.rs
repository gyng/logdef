//! What the world actually does under the tower, per tick and per pace.
//!
//! ```text
//! cargo run --release -p understory-core --example worldrate
//! ```
//!
//! The World rows are the oldest arithmetic in `BALANCE.md` and the
//! least revisited: `stride_paces_per_100_ticks` 60 is "0.6 paces/tick,
//! 18 a second", `band_min_paces`/`band_max_paces` are "~16 s" and
//! "~50 s in a band", `stream_ahead_paces` is "one full maximum band
//! beyond the tower". Every one of those is a division somebody did once.
//!
//! The three sections that have had this treatment each turned up a row
//! that had quietly stopped being true — lighting 70% out, meals a third
//! out, a sails row built on a stride cost six times the current one. So
//! this counts the paces, the bands and the window rather than dividing
//! the constants by each other.

use understory_core::GameEngine;
use understory_core::command::GameCommand;
use understory_core::state::SimSpeed;

const TICKS: u32 = 60_000;

fn main() {
    println!("=== what does the world do under the tower? ===\n");

    let content = understory_core::content::Content::load_embedded().expect("pack");
    let mut game = GameEngine::new(0xB01D_FACE);
    game.set_speed(SimSpeed::X1);
    let _ = game.try_send(GameCommand::SetStriding { walking: true });

    let mut strode_ticks = 0u32;
    let mut band_lengths: Vec<i64> = Vec::new();
    let mut seen_band_start = 0i64;
    let mut last_band: Option<usize> = None;
    let mut window_min = 0i64;
    let mut window_max = 0i64;
    let mut bands_held = 0usize;

    for _ in 0..TICKS {
        if let Some(fork) = game.state().world.fork
            && fork.answer.is_none()
        {
            let _ = game.try_send(GameCommand::TakeFork { branch: 0 });
        }
        game.step(1);
        let state = game.state();
        if state.strode {
            strode_ticks += 1;
        }
        let here = state.world.distance;

        // Band boundaries, measured by watching the band under the tower
        // change rather than by reading the generator's intent.
        if let Some(band) = state.world.band_at(here) {
            let id = band.kind.get();
            if last_band != Some(id) {
                if last_band.is_some() {
                    band_lengths.push((here - seen_band_start) >> 8);
                }
                seen_band_start = here;
                last_band = Some(id);
            }
        }

        // How far the live window reaches in front of and behind the
        // tower. Pruning is what keeps a run of any length costing the
        // same memory, so this is the constant doing its job or not.
        // **Worst case, not best.** Taking the minimum measures the
        // instant just after a prune and says the window is zero; the
        // question memory cares about is how much is ever held at once.
        if let (Some(first), Some(last)) = (state.world.bands.first(), state.world.bands.last()) {
            window_min = window_min.max((here - first.start) >> 8);
            window_max = window_max.max((last.end() - here) >> 8);
            bands_held = bands_held.max(state.world.bands.len());
        }
    }

    let state = game.state();
    let paces = state.world.distance >> 8;
    println!(
        "paces per 100 ticks              {:.1}   (`stride_paces_per_100_ticks` says 60)",
        paces as f64 * 100.0 / f64::from(TICKS)
    );
    println!(
        "  ...of ticks actually strode    {:.1}   (the tower stood still for {}%)",
        paces as f64 * 100.0 / f64::from(strode_ticks.max(1)),
        100 - strode_ticks * 100 / TICKS
    );

    band_lengths.sort_unstable();
    let shortest = band_lengths.first().copied().unwrap_or(0);
    let longest = band_lengths.last().copied().unwrap_or(0);
    let mean = band_lengths.iter().sum::<i64>() / band_lengths.len().max(1) as i64;
    println!("\nbands crossed                    {}", band_lengths.len());
    println!(
        "band length, paces               {shortest} shortest / {mean} mean / {longest} longest\n\
         \x20                                 (`band_min_paces` {} / `band_max_paces` {})",
        content.balance.world.band_min_paces, content.balance.world.band_max_paces
    );
    println!(
        "  at 1x, seconds in a band       {:.0}s shortest / {:.0}s mean / {:.0}s longest",
        seconds(shortest),
        seconds(mean),
        seconds(longest),
    );

    println!(
        "\nmost ever held behind the tower  {window_min} paces (`stream_behind_paces` {})",
        content.balance.world.stream_behind_paces
    );
    println!(
        "most ever held ahead of it       {window_max} paces (`stream_ahead_paces` {})",
        content.balance.world.stream_ahead_paces
    );
    println!("most bands held at once           {bands_held}");
    println!(
        "\n  **Worst case, not typical**, and that matters: taking the minimum instead\n\
         measures the instant just after a prune and reports the window as zero paces\n\
         wide, which is true and useless. Memory cares about how much is ever held.\n\
         \n\
         Both figures are *supposed* to exceed their constants, and by a knowable\n\
         amount. Generation runs until it has reached `stream_ahead_paces` and then\n\
         finishes the band it is in; pruning drops what is wholly behind\n\
         `stream_behind_paces` and keeps the band straddling the line. So the ceilings\n\
         are 900 + 900 = 1,800 ahead and 300 + 900 = 1,200 behind, and a long run\n\
         touches 1,795 and 1,195. Neither grows with distance, which is the whole of\n\
         `prune_behind`'s promise: a run of any length costs the same memory."
    );
}

fn seconds(paces: i64) -> f64 {
    // 0.6 paces a tick at 30 Hz.
    paces as f64 / 0.6 / 30.0
}
