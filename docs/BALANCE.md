# Understory — Balance

Every tuning constant in the game, with where it came from.

Values live in `assets/data/balance.ron`; this file grades them. A test
(`crates/core/src/tests/balance_doc.rs`) fails if a constant exists in one and not the
other, in either direction — so this table cannot quietly rot.

## Grades

| Grade | Means |
|---|---|
| `DESIGNED` | Chosen from reasoning about the intended feel. Plausible, unproven. |
| `PLAYTESTED` | Someone played with it, and with neighbouring values, and this one won. |

M5 does not ship until every row reads `PLAYTESTED`. Being wrong fast is the point of
having the table; being wrong *quietly* is what the grade prevents.

---

## World

The tower's gait and how far it can see.

| Constant | Value | Grade | Reasoning |
|---|---:|---|---|
| `stride_paces_per_100_ticks` | 60 | DESIGNED | 0.6 paces/tick, 18 a second. Fast enough that the horizon changes while you watch a chain run; slow enough that terrain feels like somewhere rather than something flickering past. Becomes a player throttle in M3. |
| `band_min_paces` | 300 | DESIGNED | ~16 s in a band at 1×. Below this, terrain changes faster than a haul round-trip, so route choice could never bite. |
| `band_max_paces` | 900 | DESIGNED | ~50 s at 1×, ~12 s at 4×. Long enough that a rich band is worth noticing and a barren one is worth waiting out. |
| `stream_ahead_paces` | 900 | DESIGNED | One full maximum band beyond the tower, so the horizon is never visibly generated into existence. |
| `stream_behind_paces` | 300 | DESIGNED | Enough to keep what has just passed on screen for parallax; anything older is dropped. |

## Tower

Shape, height, and the price of both.

| Constant | Value | Grade | Reasoning |
|---|---:|---|---|
| `starting_floors` | 4 | DESIGNED | Enough vertical distance that the opening haul is a climb, few enough that the tower reads at a glance on the first frame. |
| `max_floors` | 14 | DESIGNED | The top of the cozy 8–14 range from `v2-plan.md` §0. Above this the cross-section stops fitting on one screen, which is the constraint the whole art direction rests on. |
| `floor_slots` | 8 | DESIGNED | Four two-wide rooms, or three plus a shaft column. Tight enough that a shaft is a real sacrifice — which is the entire point of shafts costing floor width. |
| `floor_cost` | 6 poles | DESIGNED | About two minutes of the opening chain's output. Growing taller should be a decision you save up for, not a button you press. |
| `stairs_capacity` | 1 | DESIGNED | One body at a time. With two crew this is already a queue, which puts the contention thesis on screen in the first minute rather than in M1. |
| `starting_stock` | 10 poles | DESIGNED | One floor's worth plus a room, so the first build is possible before the mill has ever run, and the second is not. |

## Crew

How fast the tower's pulse beats.

| Constant | Value | Grade | Reasoning |
|---|---:|---|---|
| `starting_crew` | 3 | PLAYTESTED | Was 2. Measured over 600 s (`cargo run --release -p understory-core --example throughput`): with two crew, adding an elevator changed crafted output by less than noise, so M1's sprint question could not be answered at all. At three, the same comparison shows **+90% crafts and 56% fewer ticks spent queueing**. Two crew make a queue; three make it matter. |
| `walk_ticks_per_slot` | 12 | DESIGNED | 0.4 s a slot, ~3 s to cross a floor. Horizontal distance is a real but minor cost, so layout matters without dominating. |
| `climb_ticks_per_floor` | 30 | DESIGNED | A full second a floor: two and a half times the cost of walking the same "distance". Vertical is expensive, which is why a dumbwaiter will be worth its slot in M1. |
| `load_ticks` | 15 | DESIGNED | Half a second at each end. Long enough to read as an action in the cross-section, short enough not to dominate a round trip. |
| `unload_ticks` | 15 | DESIGNED | Symmetric with loading; no reason yet for them to differ. |
| `carry_capacity` | 3 | DESIGNED | Sets haul throughput against production: two crew at three per trip roughly match the opening chain, so the tower runs but has no slack. That margin is the tension. |
| `stress_ticks` | 60 | DESIGNED | Two seconds blocked before a crew member tints red. Short enough to catch a real bottleneck, long enough that ordinary traffic doesn't cry wolf. |

## Clock

The day, and what reads it.

| Constant | Value | Grade | Reasoning |
|---|---:|---|---|
| `ticks_per_day` | 14400 | DESIGNED | 8 minutes at 1x (480 s x 30 ticks/s), ~2 minutes at 4x — about 20 day cycles across a 3-hour run, enough for the sun/route tension to repeat many times without a single day feeling like the whole session. |
| `sun_curve` | 8 anchors, 0-0-40-90-100-60-10-0 | DESIGNED | A smooth rise and fall rather than a staircase, so no daypart boundary reads as a lighting bug. Dark for the first tenth of the day, peaking at midday, dark again by the last fifteenth — integrating the curve over a full day (see Power below) gives an average exposure of ~46.5% on neutral terrain, the number the sail and cost figures below are sized against. |
| `night_light_threshold` | 20 | DESIGNED | Crossing this against the curve puts the "lights must be lit" span at ~330 permille of the day (~4,750 ticks, ~2.6 min) — long enough that unlit floors are a real cost, short enough that a competent tower shrugs off most nights. |

## Power

Charge income, storage, and draw. See `SYSTEMS.md` §1.2.

| Constant | Value | Grade | Reasoning |
|---|---:|---|---|
| `starting_charge` | 800 | DESIGNED | Tick 0 is predawn, so a fresh run starts in the dark. A starting tower's base draw (striding + lighting 4 floors, no thornwright yet) is ~0.28 charge/tick; 800 outlasts the ~2,200 ticks until the curve clears the light threshold, with room to spare, but is nowhere near enough to also run a thornwright unattended — the tower starts tight, not comfortable. |
| `stride_charge_per_100_ticks` | 20 | DESIGNED | Continuous striding costs 2,880 charge across a full day-cycle — real money next to a two-sail "good sun" day's ~20,000 charge income (see Content constants below), but not so much that walking is unaffordable. Visible and ongoing, which is what makes `SetStriding{false}` a genuine choice rather than a button nobody presses. |
| `light_charge_per_100_ticks_per_floor` | 2 | DESIGNED | Small per floor, but multiplies with height: an 8-floor tower lit through the ~4,750-tick night spends ~760 charge; a 4-floor starting tower spends ~380. Enough that a brown-out reads as "the tower outgrew what it can light," not as noise. |

## Transport

Dwell, dispatch, and the estimates crew use to pick a shaft. See `SYSTEMS.md` §1.3.

| Constant | Value | Grade | Reasoning |
|---|---:|---|---|
| `dwell_base_ticks` | 10 | DESIGNED | A third of a second of doors-and-landing overhead before anyone moves — enough to read as a stop in the cross-section, not enough to make every trip feel like a wait. |
| `dwell_per_unit_ticks` | 6 | DESIGNED | Cheaper than a full `load_ticks`/`unload_ticks` (15) — boarding a car is simpler than working a shelf — but it still adds up: a full 4-unit car adds 24 ticks to a stop, visibly slower than an empty one. |
| `dispatch_threshold` | 2 | DESIGNED | Matches `starting_crew`. With two crew, a car that leaves the instant the first caller arrives would never show contention at all, which is the exact thing M1's sprint question ("is elevator contention actually fun?") is testing. |
| `dispatch_max_wait_ticks` | 90 | DESIGNED | 3 seconds. A lone caller on a quiet floor still gets served eventually — batching is meant to create the queue a player can see, not to strand anyone waiting for a second caller who never comes. |
| `queue_penalty_ticks` | 60 | DESIGNED | A floor of 2 seconds per person ahead of you in a stairs estimate. Note the estimate multiplies this by the *queue length* rather than applying it once: a flat penalty had crew cheerfully joining a five-deep line because "occupied" cost the same whether one person or five were ahead. |
| `elevator_base_wait_ticks` | 45 | DESIGNED | 1.5 seconds, a rough midpoint between "the car is right here" and "the car is at the far end of its run." The estimate crew fall back on before the car's actual position and direction sharpen it. |

---

## Content constants

Room and item numbers live next to their definitions in `assets/data/rooms/` and are
graded here as a group, since they are tuned as a set rather than individually.

| Constant | Value | Grade | Reasoning |
|---|---:|---|---|
| cutter arm `ticks_per_item` | 90 | DESIGNED | One stalk per 3 s at ordinary yield, 2.1 s under canopy, 6 s in a ruin-field. Terrain should change the rhythm audibly. |
| cutter arm `buffer_max` | 8 | DESIGNED | About 24 s of unattended output before the arm goes quiet — enough slack that a single delayed haul isn't a stall, little enough that a *persistent* transport shortfall is. |
| mill `craft_ticks` | 120 | DESIGNED | 4 s a pole. Deliberately slower than the cutter arm so the first thing a player learns is that one mill cannot keep up with one arm. |
| mill buffers | 6 in, 6 out | DESIGNED | 24 s of runway on each side. Big enough to ride out a queue, small enough that backpressure reaches the arm. |
| storeroom `shelves` × `per_shelf` | 4 × 20 | DESIGNED | 80 items across four independent shelves, so surplus bamboo and milled poles never compete for the same space. |
| terrain `yield_pct` | 140 / 100 / 50 | DESIGNED | Canopy nearly triples a ruin-field's output. The gap has to be large enough to feel, because in M1 it will be opposed by sun — and the opposition is the choice. |
| terrain `weight` | 40 / 35 / 25 | DESIGNED | Jungle-first mix for region 1. Ruin-fields are the rarer, more interesting stretch; M3 flips these weights per region. |
| terrain `sun_pct` | 35 / 100 / 130 | DESIGNED | Strictly opposed to `yield_pct`: canopy (140 yield) is the poorest sun at 35, ruin-field (50 yield) is the richest at 130, clearing sits neutral on both. No band is allowed to be good at both axes — that opposition is the whole of "your route is your power mix" (`SYSTEMS.md` §1.1). |
| thornwright `craft_ticks` | 150 | DESIGNED | 5 s a dart — deliberately slower than the mill's 4 s, so a single thornwright can never out-consume a single mill's output (mill supplies 1 pole/120 ticks; a thornwright only demands 1/150 ticks) and the chain stays visibly fed rather than backed up at the mill. |
| thornwright `power_draw` | 1 | DESIGNED | The first production charge sink (`RoomDef.power_draw`, drawn per tick while crafting). Modest per tick, but a thornwright that never stops crafting costs 14,400 charge across a full day-cycle — on the order of two sail rooms' full-day income, which is what "a couple of sails should roughly cover one thornwright" is sized against. |
| thornwright `build_cost` / buffers | 5 poles; 6 in, 6 out | DESIGNED | Priced a notch above the mill (4 poles) for the added complexity of being powered; buffers match the mill's 6/6 so both links of the chain hold equivalent runway. |
| canopy sails `charge_per_100_ticks` | 150 | DESIGNED | The tower's main charge income. Two sail rooms over a full "good sun" (clearing-terrain) day-cycle net ~20,000 charge against ~17,700 of continuous striding, night lighting on a 4-floor tower, and one continuously-crafting thornwright — a ~13% surplus, not a landslide. Under canopy terrain (sun_pct 35) the same two rooms net only ~7,000, comfortably short of that ~17,700 cost — the exact bind "your route is your power mix" is meant to create. |
| canopy sails `build_cost` | 6 poles | DESIGNED | The tower's main income source costs a little more than the production rooms it powers, in line with everything else depending on it. |
| burner `fuel_per_burn` / `charge_per_burn` / `burn_ticks` | 2 bamboo / 100 charge / 100 ticks | DESIGNED | 1 charge/tick sustained — close to a sail room's full-day average of ~0.7/tick in good sun (150 x 66.96 / 14400), but paid for in bamboo at roughly 2.4x the mill's own appetite for the same stalks (0.02/tick burned vs 0.0083/tick milled). Available regardless of sun or hour; expensive in the one material every other chain wants. |
| burner `build_cost` | 5 poles | DESIGNED | Same tier as the mill — the burner is meant to be an easy fallback to stand up, not a late-game investment. |
| cell bank `capacity` | 1500 | DESIGNED | A full night's worst case — striding, lighting, and one continuous thornwright, over the ~4,750-tick span below `night_light_threshold`, with no sun income — costs on the order of 6,000 charge. About four cell banks' worth: "charge capacity is built, not found" should cost real floor space, not one room. |
| cell bank `build_cost` | 8 poles | DESIGNED | The priciest of the small Energy rooms — pure capacity with no throughput of its own, so it should feel like an infrastructure investment rather than an impulse build. |
| stairs `ticks_per_floor` / `capacity` | 30 / 1 | DESIGNED | Deliberately mirrors `climb_ticks_per_floor` and `stairs_capacity` above exactly — the shaft content doesn't get to quietly redefine a number `balance.ron` already owns. |
| dumbwaiter `ticks_per_floor` / `charge_per_floor` / `batch` | 15 / 1 / 4 | DESIGNED | Half the stairs' time per floor for a small sip of charge — fast enough to be worth its slot column, cheap enough to run constantly. A batch of 4 sits a little above one crew member's `carry_capacity` (3), so it beats a hauler on volume without trivializing the trip. |
| dumbwaiter `build_cost` | 8 poles | DESIGNED | Priced above a single production room — it's replacing a standing haul route, which is worth more than one link in the chain. |
| elevator `ticks_per_floor` / `charge_per_floor` / `capacity` | 8 / 5 / 4 | DESIGNED | Nearly 4x the stairs' pace per floor, and by far the largest charge draw of any shaft — the fastest way up costs the most to run every time it moves. Capacity 4 seats roughly two laden crew (1 unit each, +1 for a carried load), the "small crowd" scale the elevator earns its slot column for. |
| elevator `build_cost` | 18 poles | DESIGNED | Three times a floor's own cost (6 poles) — the elevator is a mid-game investment the player saves for, not an opening move. |
