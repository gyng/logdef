Project: SUPPLY LINE | v1 Scope Definition

> **Status:** Canonical, defines what ships. Check this file before implementing any feature from other foundation docs.

Every feature in the foundation docs is tagged here as `v1`, `post-v1`, or `prototype-only`. Implementation should ONLY build v1 features. Post-v1 features are designed but deferred. Prototype-only features exist in docs for context but may never ship.

When a foundation doc describes a feature, check this file for its scope tag. If untagged, assume post-v1.

---

## Hero Classes

| Feature | Scope | Notes |
|---------|-------|-------|
| Archer (Focus) | v1 | Starting class. First playable. |
| Engineer (Overclock) | v1 | Second class. Unlock after Ch2 boss. |
| Commander (Rally) | v1 | Third class. Unlock after 1 run. |
| Scavenger (Scrounging) | post-v1 | Design complete. Defer to first content update. |
| Berserker (Rage) | post-v1 | Design complete. Defer to second content update. |
| Hero stat system (5 stats) | v1 | Precision, Draw, Tempo, Grit, Salvage. |
| Perk tree (3 branches, 3 tiers) | v1 | Full tree for 3 classes. Class-specific perk amplifications are post-v1. |
| Class-specific perk amplifications | post-v1 | (Focus Chain, Overclock cascade, etc.) Defer. Generic perk tree sufficient for v1. |

## Weapons

| Feature | Scope | Notes |
|---------|-------|-------|
| Bows (3 sub-types: shortbow, longbow, composite) | v1 | Core ranged. Available from run start. |
| Crossbows (2 sub-types: hand crossbow, heavy crossbow) | v1 | Alternate ranged. Available from run start. |
| Staves (2 sub-types: wand, staff) | v1 | Easy-aim option. **Gated: unlocks after Ch2 boss.** |
| Thrown (2 sub-types: javelins, bombs) | v1 | Arc + AoE. **Gated: unlocks after Ch2 boss.** |
| Melee (2 sub-types: dagger sidearm, sword) | v1 | Zero-logistics. Available from run start. Dagger occupies weapon slot 2 (can be sold/replaced). See [implementation-decisions.md](implementation-decisions.md) §14. |
| Pistol (companion-only: Rust) | v1 | Not available to hero. Fixed companion equipment. See [implementation-decisions.md](implementation-decisions.md) §15. |
| Hammer (companion-only: Stone) | v1 | Not available to hero. Fixed companion equipment. Melee, no ammo. See [implementation-decisions.md](implementation-decisions.md) §15. |
| Additional bow sub-types (recurve, greatbow, hunting bow, eldritch bow) | post-v1 | |
| Additional crossbow sub-types (repeating, siege, clockwork) | post-v1 | |
| Additional staff sub-types (scepter, orb, necromancer, storm rod) | post-v1 | |
| Additional thrown sub-types (knives, flasks, boomerang, bolas, chakram) | post-v1 | |
| Additional melee sub-types (spear, hammer, flail, gauntlets, scythe) | post-v1 | |
| Guns (all sub-types) | post-v1 | Requires gunpowder production chain. |
| Whips (all sub-types) | post-v1 | |
| Instruments (all sub-types + rhythm mechanic) | post-v1 | Rhythm system is a major feature. Defer entirely. |
| Shields (all sub-types) | post-v1 | Block/counter input model is distinct from all other weapons. Defer. |
| Weapon modifiers (6 combat + 3 economy + 3 utility) | v1 | 12 modifiers. Full system. |
| Drawback modifiers (cursed, heavy, fragile, bloodthirsty) | v1 | |
| Modifier crafting at Enchanter | v1 | Reroll + apply specific. |
| Weapon rarity (4 tiers) | v1 | |
| Legendary unique names | v1 | ~15 for v1 launch. Pool grows post-v1. |

## Trinkets

| Feature | Scope | Notes |
|---------|-------|-------|
| Combat trinkets (Last Arrow, Returner's Coin, Twin Fang, Berserker's Tooth) | v1 | 4 trinkets. |
| Logistics trinkets (The Leech, Smuggler's Pouch, Magnet Stone) | v1 | 3 trinkets. Echo Chamber deferred (depends on companion rack interaction). |
| Defensive trinkets (Phase Amulet, Decoy Charm, Tower Heart) | v1 | 3 trinkets. Scarecrow deferred. |
| Wild trinkets (Heartstring, Time Crystal, Resonance Bell, Copycat) | post-v1 | All 4 are complex special cases. Ship after core is stable. |

## Companions — Exterior

| Feature | Scope | Notes |
|---------|-------|-------|
| Ren (Mark / AMPLIFY) | v1 | |
| Kael (Wall / BLOCK) | v1 | |
| Mira (Splash / EXPLODE) | v1 | |
| Yuki (Field Medic / HEAL) | v1 | New mechanic — shots repair panels. |
| Drift (Pinning / SLOW) | v1 | Simple, reliable. |
| Rust (Suppression / SUPPRESS) | v1 | |
| Stone (Anchor / ANCHOR) | v1 | |
| Kit (Jury-Rig / FIX) | v1 | |
| Volt (Arc / CHAIN) | post-v1 | |
| Thorn (Snare / SNARE) | post-v1 | |
| Bell (Harmony / HARMONIZE) | post-v1 | Depends on instrument/rhythm system. |
| Sable (Profiteer / PROFIT) | post-v1 | |
| Ash (Salvage / SCAVENGE) | post-v1 | |
| Shade (Vanish / HIDE) | post-v1 | Enemy pathfinding exception. |
| Wren (Redirect / REDIRECT) | post-v1 | Enemy pathfinding manipulation. |

**v1 exterior companions: 8.** Post-v1 adds 7 more.

## Companions — Interior

| Feature | Scope | Notes |
|---------|-------|-------|
| Offices (buildable positions) | v1 | Infrastructure. |
| Broth (Cook / FEED) | v1 | Meal system is a new loop but contained. |
| Cog (Mechanic / PROTECT) | v1 | Simple — transport immunity. |
| Tinker (Gadgeteer / BUILD) | post-v1 | 6 gadget types is a subsystem. Defer. |
| Smuggler (Black Marketeer / ACQUIRE) | post-v1 | Quality rolls + hot goods events. Defer. |
| Dreamer (Spirit-Touched / CHANNEL) | post-v1 | 4 channel options + mid-combat activation. Defer. |
| Saboteur (Trap-Layer / TRAP) | post-v1 | Interior trap mechanics. Defer. |
| Cartographer (Route-Maker / MAP) | post-v1 | Permanent shortcut system. Defer. |
| Forge (Consultant / OPTIMIZE) | post-v1 | |

**v1 interior companions: 2** (Broth, Cog). Post-v1 adds 6 more.

## Companion Systems

| Feature | Scope | Notes |
|---------|-------|-------|
| Targeting orders (6 types) | v1 | Nearest, Ground, Air, Climbers, Siege, Boss. |
| Fire discipline (3 types) | v1 | Free Fire, Conservative, Hold Fire. |
| Companion accuracy (scales with experience) | v1 | Core progression. |
| Companion dialogue (prep hints, post-combat comments) | v1 | Text lines, state-triggered. |
| Affinity system (5 types) | post-v1 | Relationship bonuses need the full roster to be interesting. |
| Relationship levels (Stranger → Bonded) | post-v1 | |
| Relationship bonuses (affinity-dependent) | post-v1 | |
| Paired endings | post-v1 | |
| Romantic undertones | post-v1 | |
| Companion passive scaling (base → veteran → elite) | v1 | Simple 2-tier scaling (base + veteran at 10 encounters). Elite tier is post-v1. |

## Tower & Supply Chain

| Feature | Scope | Notes |
|---------|-------|-------|
| Floors (wood, stone) | v1 | Iron floors are post-v1. |
| Foundation / legs (chicken legs only) | v1 | Spider, treads, hover are post-v1 unlocks. |
| Tower width upgrades | post-v1 | |
| Warehouse | v1 | |
| Caches | v1 | |
| Ammo racks | v1 | |
| Balconies (buildable) | v1 | |
| Offices (buildable) | v1 | |
| Runners (semi-automated) | v1 | |
| Runner quarters | v1 | |
| Production Tier 1 (fletcher, forge, quarry, lumberyard) | v1 | |
| Production Tier 2 (sawmill, alchemist, weaponsmith) | v1 | Unlocked after Ch1 boss. |
| Production Tier 3 (enchanter, siege works, artificer) | post-v1 | Unlocked after Ch3 boss. Defer to content update. |
| Built-in stairs | v1 | |
| Ladders | v1 | |
| Dumbwaiters | v1 | |
| Chutes | v1 | Unlocked after Ch1 boss. (Registries.md is canonical.) |
| Cargo lifts (with programming: floor range, car count, departure mode) | post-v1 | Lift programming is a subsystem. Defer. |
| Express lifts | post-v1 | |
| Conveyors | post-v1 | |
| Pneumatic tubes | post-v1 | |
| Crate-based logistics (visual crates, buffer overflow/starvation) | v1 | |
| Runner queuing + route choice | v1 | |

## Combat

| Feature | Scope | Notes |
|---------|-------|-------|
| Fixed-position hero | v1 | |
| Free aim (mouse) | v1 | |
| Weapon-dependent projectile physics (bow arc, crossbow flat, staff straight) | v1 | For v1 weapon types only. |
| Weapon abilities (Q key) | v1 | For v1 weapons. |
| Hero skill (E key, from perk tree) | v1 | |
| Weapon swap (Tab) | v1 | Two weapon slots. |
| Priority flag (F key) | v1 | |
| Melee sidearm fallback | v1 | |
| Personal ammo reserve (10-15 shots) | v1 | |
| Every-shot-visible (arrows stick in enemies, misses on ground) | v1 | Core feel. Non-negotiable. |
| Critical hits (weak spots) | v1 | |
| Screen shake | v1 | |
| Kill effects (per weapon type) | v1 | For v1 weapons. |

## Enemies

| Feature | Scope | Notes |
|---------|-------|-------|
| Grunts | v1 | |
| Runners (fast) | v1 | |
| Armored | v1 | |
| Climbers | v1 | |
| Flyers (hoverers) | v1 | |
| Flyers (dive bombers) | post-v1 | |
| Catapults | v1 | |
| Rams | v1 | Chapter 4+. |
| Siege towers | post-v1 | Complex docking mechanic. |
| Sappers | v1 | |
| Bosses (ground ram type, climbing giant type) | v1 | 2 boss types. |
| Bosses (flying dragon type) | post-v1 | |
| Boss phases (behavior changes at HP thresholds) | v1 | 2 phases per boss (66% HP transition). |

## Damage & Tower HP

| Feature | Scope | Notes |
|---------|-------|-------|
| Per-floor wall panel HP | v1 | |
| Panel breach → interior raiders (timed, expelled by spirit) | v1 | |
| Interior raiders destroy transport infrastructure | v1 | |
| Foundation HP (run-ender) | v1 | |
| Unrepaired breaches stay open | v1 | |
| Panel damage visibility (cracks) | v1 | |
| Companion displacement on breach | v1 | |

## Map & Journey

| Feature | Scope | Notes |
|---------|-------|-------|
| Slay the Spire branching map | v1 | |
| Full chapter visibility | v1 | |
| Combat nodes (3 difficulty tiers) | v1 | |
| Elite nodes (with modifiers) | v1 | |
| Merchant nodes | v1 | |
| Rest nodes | v1 | |
| Mystery nodes (6-8 events from pool) | v1 | Reduced event pool. |
| Boss nodes | v1 | |
| Companion recruitment nodes | v1 | |
| Forge site nodes | post-v1 | Specialty materials. |
| Toll gate nodes | post-v1 | |
| Terrain on paths | v1 | Plains, forest, mountain (3 types). Swamp, desert, storm, night are post-v1. |
| Leg terrain penalties | post-v1 | Only chicken legs in v1, so no penalty system needed. |
| 3 chapters per run | v1 | Not 5. Shorter runs for v1. Victory at end of Ch3. |
| Encounter modifiers (night, storm, etc.) | post-v1 | |

**v1 journey: 3 chapters, ~20 nodes, 1 destination (The Harbor).** Post-v1 adds chapters 4-5 and destinations 2-4.

## Economy

| Feature | Scope | Notes |
|---------|-------|-------|
| Ticks (action budget per prep stop) | v1 | |
| Materials (construction costs) | v1 | |
| Gold (operating costs, bounties, merchants) | v1 | |
| Material stockpiling in warehouse | v1 | |
| Tick scaling by chapter | v1 | |
| Merchant buying/selling | v1 | |
| Merchant archetypes (4 types) | post-v1 | v1 has one generic merchant. |

## Metagame

| Feature | Scope | Notes |
|---------|-------|-------|
| Content unlocks (buildings, transport, weapons) | v1 | Gated by Ch1-Ch3 boss progression within a run, NOT across runs. v1 has no between-run unlock persistence. |
| Between-run unlock persistence | post-v1 | v1 = each run starts with full v1 content available. Unlock gating is WITHIN the run (beat Ch1 boss → Tier 2 unlocks for this run). |
| Hero class unlocks across runs | post-v1 | v1: all 3 v1 classes available from start. |
| Companion pool growth across runs | post-v1 | v1: all 10 v1 companions available every run. |
| Journey destinations (Harbor only) | v1 | |
| Additional destinations (Crossroads, Grove, Forge) | post-v1 | |
| Ascension system | post-v1 | |
| Seed system (display + share) | v1 | Cheap to implement, high community value. |
| Leaderboards | post-v1 | |

## UI/UX

| Feature | Scope | Notes |
|---------|-------|-------|
| Combat HUD (ammo, cooldowns, wave, companion row) | v1 | |
| Prep phase tower editor | v1 | |
| Map screen | v1 | |
| Merchant screen | v1 | |
| Inventory/equipment screen | v1 | |
| Companion management screen | v1 | |
| Post-combat summary | v1 | |
| Hero stats/perk screen | v1 | |
| Pre-departure validation warnings | v1 | |
| Loot card-flip reveal | post-v1 | v1 uses simple list. |
| Companion dialogue system | v1 | Text lines, state-triggered. |

## Audio

| Feature | Scope | Notes |
|---------|-------|-------|
| Web Audio API implementation | v1 | |
| Production building ambient loops (for v1 buildings) | v1 | |
| Weapon fire + hit sounds (for v1 weapons) | v1 | |
| Enemy approach + death sounds (for v1 enemies) | v1 | |
| Panel damage + breach sounds | v1 | |
| Tower walking sound (chicken legs) | v1 | |
| Music (prep + combat, 2 tracks minimum) | v1 | |
| Adaptive music stems | post-v1 | |
| Terrain-specific ambient | post-v1 | |
| Companion voice barks | post-v1 | |
| Spatial audio (left/right panning) | v1 | Basic stereo. Advanced height-based reverb is post-v1. |

## Animation

| Feature | Scope | Notes |
|---------|-------|-------|
| Tween system | v1 | Core engine. |
| Runner animation (walk, carry, queue) | v1 | |
| Hero animation (aim, fire per v1 weapon type) | v1 | |
| Enemy animation (approach, climb, attack, die) | v1 | |
| Building production loops (for v1 buildings) | v1 | |
| Transport animation (stairs, dumbwaiter, chute) | v1 | |
| Tower walking (chicken legs) | v1 | |
| Bone rig system | post-v1 | v1 uses simpler sprite-swap animation. |
| Particle system | v1 | Basic: dust, sparks, hit impacts. Full preset library is post-v1. |
| Screen shake | v1 | |
| Panel crack progression | v1 | |

## Telemetry

| Feature | Scope | Notes |
|---------|-------|-------|
| Per-encounter telemetry (kills, accuracy, ammo, breaches) | v1 | Must collect. |
| Per-run telemetry (outcome, chapters reached, gold) | v1 | Must collect. |
| Player suggestions (post-combat tips) | v1 | 5-6 trigger conditions. Not the full 10+. |
| Companion prep hints | v1 | |
| Pacing adaptation (encounter composition selection) | post-v1 | v1 uses pure random from the difficulty pool. |
| Cognitive science heuristics (flow, competence, arousal) | post-v1 | Research, not v1 implementation. |
| Developer dashboard | post-v1 | v1 uses raw telemetry logs. |

## Procedural Generation

| Feature | Scope | Notes |
|---------|-------|-------|
| Seeded RNG (deterministic) | v1 | |
| Map generation (3 chapters) | v1 | |
| Encounter composition from threat budget | v1 | |
| Loot generation (weapon + trinket drops) | v1 | |
| Merchant stock generation | v1 | |
| Mystery event selection (6-8 events) | v1 | Reduced pool. |
| Boss generation (fixed per chapter) | v1 | |
| Companion placement on map | v1 | |
| Forge site placement | post-v1 | |
| Anti-frustration rules | v1 | Core set (breathing room, no 3+ combat chains, first mystery positive). |

---

## v1 Summary

**What ships:**
- 3 hero classes (Archer, Engineer, Commander)
- 11 weapon sub-types across 5 base types (bow, crossbow, staff, thrown, melee)
- 16 weapon modifiers + Enchanter crafting
- 10 trinkets
- 8 exterior companions + 2 interior companions
- Companion orders + accuracy scaling + dialogue
- 3-chapter journey to The Harbor
- Slay the Spire map with 7 node types
- 4 terrain types (plains, forest, mountain + coast for Ch3)
- Full tower building: floors, transport (stairs, ladders, dumbwaiters, chutes), warehouse, caches, racks, runners
- Tier 1 + Tier 2 production chains
- Chicken legs only
- Panel HP, breach mechanics, foundation HP
- 8 enemy archetypes + 2 boss types
- Full economy (ticks, materials, gold)
- Seed system
- Web Audio
- wgpu rendering + React UI

**What doesn't ship (post-v1):**
- Scavenger + Berserker classes
- Guns, whips, instruments, shields
- 7 additional exterior companions + 6 interior companions
- Relationship/affinity system
- Cargo lifts, express lifts, conveyors, pneumatic tubes
- Tier 3 production
- Spider legs, treads, hover
- Chapters 4-5, Crossroads/Grove/Forge destinations
- Ascension system
- Encounter modifiers (night, storm, etc.)
- Forge sites, toll gates
- Wild trinkets
- Adaptive music, spatial audio depth
- Pacing adaptation
- Bone rig animation
- Between-run unlock persistence

**Estimated v1 complexity: ~60% of the full design.** This is a shippable game, not a demo. The 40% deferred is expansion content that layers onto a complete core.
