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
| `starting_crew` | 2 | DESIGNED | One crew member proves the chain runs; two prove the stairs are contended. The second is what makes the tower feel inhabited rather than automated. |
| `walk_ticks_per_slot` | 12 | DESIGNED | 0.4 s a slot, ~3 s to cross a floor. Horizontal distance is a real but minor cost, so layout matters without dominating. |
| `climb_ticks_per_floor` | 30 | DESIGNED | A full second a floor: two and a half times the cost of walking the same "distance". Vertical is expensive, which is why a dumbwaiter will be worth its slot in M1. |
| `load_ticks` | 15 | DESIGNED | Half a second at each end. Long enough to read as an action in the cross-section, short enough not to dominate a round trip. |
| `unload_ticks` | 15 | DESIGNED | Symmetric with loading; no reason yet for them to differ. |
| `carry_capacity` | 3 | DESIGNED | Sets haul throughput against production: two crew at three per trip roughly match the opening chain, so the tower runs but has no slack. That margin is the tension. |
| `stress_ticks` | 60 | DESIGNED | Two seconds blocked before a crew member tints red. Short enough to catch a real bottleneck, long enough that ordinary traffic doesn't cry wolf. |

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
