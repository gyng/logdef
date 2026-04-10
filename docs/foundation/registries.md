Project: SUPPLY LINE | Content Registries

> **Status:** Canonical content registry. When companion names, passives, weapon abilities, or unlock conditions differ between docs, this file is the source of truth.

---

## 1. Companion Registry

### Exterior Companions (15)

| Name | Title | Passive Name | Verb | Affinity | Weapon Preference | Scope |
|------|-------|-------------|------|----------|-------------------|-------|
| Ren | the Spotter | Mark | AMPLIFY | Precision | Crossbow | v1 |
| Kael | the Shield-Bearer | Wall | BLOCK | Guard | None (doesn't shoot) | v1 |
| Mira | the Grenadier | Splash | EXPLODE | Force | Bombs, thrown | v1 |
| Yuki | the Healer | Field Medic | HEAL | Guard | Staff | v1 |
| Drift | the Archer | Pinning Shots | SLOW | Precision | Bow | v1 |
| Volt | — | Arc | CHAIN | — | — | post-v1 |
| Thorn | the Whip-Fighter | Snare | SNARE | Force | Whip | post-v1 |
| Bell | the Musician | Harmony | HARMONIZE | Spirit | Any instrument | post-v1 |
| Sable | the Merchant | Profiteer | PROFIT | Spirit | Thrown | post-v1 |
| Ash | the Scavenger | Salvage | SCAVENGE | Cunning | Thrown (javelins, bolas) | post-v1 |
| Rust | the Gunner | Suppression | SUPPRESS | Precision | Guns | v1 |
| Kit | the Inventor | Jury-Rig | FIX | Cunning | Any (keeps changing) | v1 |
| Shade | the Ghost | Vanish | HIDE | Cunning | Throwing knives | post-v1 |
| Stone | the Giant | Anchor | ANCHOR | Force | Hammer | v1 |
| Wren | the Misdirector | Redirect | REDIRECT | Cunning | Wand | post-v1 |

**v1 exterior companions (8):** Ren, Kael, Mira, Yuki, Drift, Rust, Stone, Kit.

### Interior Companions (8)

| Name | Title | Passive Name | Verb | Affinity | Scope |
|------|-------|-------------|------|----------|-------|
| Broth | the Cook | Meals | FEED | Spirit | v1 |
| Cog | the Mechanic | Maintain | PROTECT | Guard | v1 |
| Tinker | the Gadgeteer | Gadgets | BUILD | Cunning | post-v1 |
| Smuggler | the Black Marketeer | Connections | ACQUIRE | Cunning | post-v1 |
| Dreamer | the Spirit-Touched | Commune | CHANNEL | Spirit | post-v1 |
| Saboteur | the Trap-Layer | Booby Trap | TRAP | Cunning | post-v1 |
| Cartographer | the Route-Maker | Shortcuts | MAP | Precision | post-v1 |
| Forge | the Consultant | Optimize | OPTIMIZE | Guard | post-v1 |

**v1 interior companions (2):** Broth, Cog.

### Companion Unlock Order (across runs) — post-v1

> **v1 note:** This unlock order is a post-v1 feature (between-run persistence). In v1, all 10 v1 companions (8 exterior + 2 interior) are available every run from the start. This table is preserved for the post-v1 progression design.

| Run | Companions Added |
|-----|-----------------|
| 1 | Drift, Kael, Mira, Ren, Broth |
| 2 | Yuki, Stone, Tinker |
| 3 | Rust, Bell, Cog |
| 4 | Thorn, Ash, Forge |
| 5 | Sable, Volt, Smuggler |
| 6 | Kit, Shade, Cartographer |
| 7 | Wren, Dreamer, Saboteur |

---

## 2. Hero Class Registry

| Class | Exclusive Mechanic | Starting Stats | Starting Weapon | Class Perk | Scope |
|-------|-------------------|----------------|-----------------|------------|-------|
| Archer | Focus (2s pause, 2x damage, perfect accuracy) | High Precision, moderate Draw. Low Grit, low Salvage. | Common shortbow | Eagle Eye (crit zones 50% larger) | v1 |
| Engineer | Overclock (one building at 200% speed for 20s, once per encounter) | High Grit, moderate Salvage. Low Tempo, low Draw. | Common hand crossbow | Blueprint Master (construction costs -1 tick, min 1) | v1 |
| Commander | Rally (passive aura: +30% fire rate, +20% accuracy, -15% ammo consumption within 2 floors) | Moderate across all. No extremes. | Common staff | Tactician (companion accuracy grows 50% faster, extra "synchronized" targeting order) | v1 |
| Scavenger | Scrounging (enemies drop scrap ammo on kill: 40% hero, 15% companion; scrap does 70% damage) | High Salvage, high Tempo. Low Precision, low Grit. | Common throwing knives | Vulture (loot rolls 2x faster, enemies 2x resource drop rate) | post-v1 |
| Berserker | Rage (meter 0-100, fills from adversity, 3 tiers of escalating damage/fire rate/pierce) | High Tempo, high Draw. Low Precision, low Grit. Zero Salvage. | Common sword | Bloodrage (+2% base damage per 10% total panel HP lost) | post-v1 |

**v1 classes (3):** Archer, Engineer, Commander.

---

## 3. Weapon Registry

### Base Types (9)

| Base Type | Ammo Type | Input Model | Scope |
|-----------|-----------|-------------|-------|
| Bow | Arrows (fletcher) | Draw-hold-release | v1 |
| Crossbow | Bolts (forge) | Click, auto-reload | v1 |
| Staff/Wand | Mana crystals (alchemist) | Hold to channel | v1 |
| Thrown | Thrown supply (varies) | Click to throw | v1 |
| Melee | None | Click to swing | v1 |
| Gun | Gunpowder (forge + alchemist) | Click, recoil recovery | post-v1 |
| Whip | None | Click to lash | post-v1 |
| Instrument | Mana crystals (alchemist) | Rhythm beats | post-v1 |
| Shield | None | Hold to block, RMB to counter | post-v1 |

### Sub-Types

| Base Type | Sub-Type | Ability Name | Scope |
|-----------|----------|-------------|-------|
| Bow | Shortbow | Rain | v1 |
| Bow | Longbow | Charged Shot | v1 |
| Bow | Composite | Trick Shot | v1 |
| Bow | Recurve | Suppressing Volley | post-v1 |
| Bow | Greatbow | Skyfall | post-v1 |
| Bow | Hunting bow | Marked Prey | post-v1 |
| Bow | Eldritch bow | Soul Arrow | post-v1 |
| Crossbow | Hand crossbow | Grapple Bolt | v1 |
| Crossbow | Heavy crossbow | Snipe | v1 |
| Crossbow | Repeating crossbow | Overdrive | post-v1 |
| Crossbow | Siege crossbow | Scorpion Bolt | post-v1 |
| Crossbow | Clockwork crossbow | Dual Fire | post-v1 |
| Staff | Wand | Chain Lightning | v1 |
| Staff | Staff | Barrier | v1 |
| Staff | Scepter | Meteor | post-v1 |
| Staff | Orb | Leash | post-v1 |
| Staff | Necromancer's staff | Army of the Dead | post-v1 |
| Staff | Storm rod | Tempest | post-v1 |
| Thrown | Javelins | Impale | v1 |
| Thrown | Bombs | Cluster Bomb | v1 |
| Thrown | Throwing knives | Mark for Death | post-v1 |
| Thrown | Fire flasks | Wall of Fire | post-v1 |
| Thrown | Boomerang | Whirlwind | post-v1 |
| Thrown | Bolas | Net Toss | post-v1 |
| Thrown | Chakram | Ricochet Storm | post-v1 |
| Melee | Dagger (sidearm) | Flurry | v1 |
| Melee | Sword | Riposte | v1 |
| Melee | Spear | Thrust | post-v1 |
| Melee | Hammer | Shockwave | post-v1 |
| Melee | Flail | Whirlwind | post-v1 |
| Melee | Gauntlets | Falcon Punch | post-v1 |
| Melee | Scythe | Reap | post-v1 |
| Gun | Pistol | Deadeye | post-v1 |
| Gun | Musket | Overloaded Shot | post-v1 |
| Gun | Blunderbuss | Point Blank | post-v1 |
| Gun | Hand cannon | Siege Breaker | post-v1 |
| Whip | Leather whip | Crack | post-v1 |
| Whip | Chain whip | Grapple | post-v1 |
| Whip | Thorn whip | Entangle | post-v1 |
| Whip | Fire whip | Ring of Fire | post-v1 |
| Instrument | War drum | Thunderclap (crescendo) | post-v1 |
| Instrument | Battle horn | War Song (crescendo) | post-v1 |
| Instrument | Lute | Siren Song (crescendo) | post-v1 |
| Instrument | Bell | Death Knell (crescendo) | post-v1 |
| Shield | Buckler | Riposte | post-v1 |
| Shield | Tower shield | Phalanx | post-v1 |
| Shield | Spiked shield | Shield Bash | post-v1 |
| Shield | Mirror shield | Dazzle | post-v1 |

---

## 4. Trinket Registry

| Trinket | Category | Effect Summary | Scope |
|---------|----------|---------------|-------|
| Last Arrow | Combat | Rack at 1 crate: next shot does 3x damage | v1 |
| Returner's Coin | Combat | 25% of misses ricochet for 50% damage | v1 |
| Twin Fang | Combat | Every 5th shot fires bonus projectile at nearest untargeted enemy | v1 |
| Berserker's Tooth | Combat | Panel below 50% HP: +30% fire rate, +20% damage | v1 |
| The Leech | Logistics | Companion below's kills deposit 1 ammo to your rack | v1 |
| Echo Chamber | Logistics | Ability cooldown halved, but draws ammo from companion above's rack | post-v1 |
| Smuggler's Pouch | Logistics | Once per encounter, produces 1 crate of most-needed resource | v1 |
| Magnet Stone | Logistics | Loot within 2 floors rolls at 3x speed, +10% chance of item drop | v1 |
| Phase Amulet | Defensive | Once per encounter, 5 shots pass through own wall panels to hit interior raiders | v1 |
| Decoy Charm | Defensive | Phantom defender at position; climbers slower, sappers 50% chance to target phantom | v1 |
| Tower Heart | Defensive | Interior raiders expelled in 8s instead of 15-20s (5s with Dreamer aboard) | v1 |
| Scarecrow | Defensive | Flyers avoid your floor, redirect to other floors | post-v1 |
| Heartstring | Wild | Weapon ability heals nearest companion for 5% of damage dealt | post-v1 |
| Time Crystal | Wild | Once per encounter, rewind 3s of combat (ammo not reverted) | post-v1 |
| Resonance Bell | Wild | Adjacent to Bell: weapon hits generate half-beats toward crescendo | post-v1 |
| Copycat | Wild | Weapon ability matches nearest companion's passive | post-v1 |

**v1 trinkets (10):** Last Arrow, Returner's Coin, Twin Fang, Berserker's Tooth, The Leech, Smuggler's Pouch, Magnet Stone, Phase Amulet, Decoy Charm, Tower Heart.

---

## 5. Resource Registry

| Resource | Color | Production Source | Consumed By |
|----------|-------|-------------------|-------------|
| Arrows | Warm amber-brown (#c87941) | Fletcher (Tier 1, gold) | Bows |
| Bolts | Steel blue-gray (#7a8fa3) | Forge (Tier 1, gold) | Crossbows |
| Mana crystals | Deep purple (#7c5cbf) | Alchemist (Tier 2, Tier 1 inputs) | Staves, instruments |
| Wood | Raw wood brown (#8b6914) | Lumberyard (Tier 1, gold) | Construction (floors, buildings) |
| Stone | Neutral gray (#8a8a8a) | Quarry (Tier 1, gold) | Construction (floors, panels, repair) |
| Planks | Finished light tan (#c4a35a) | Sawmill (Tier 2, wood) | Construction (advanced buildings, offices, balconies) |
| Fire oil | Flame orange (#d45722) | Alchemist (Tier 2, Tier 1 inputs) | Modifier crafting, fire weapons |
| Potions | Teal-green (#2ea88a) | Alchemist (Tier 2, Tier 1 inputs) | Consumable effects |
| Gunpowder | Charcoal dark (#3a3a3a) | Forge + Alchemist (Tier 2 chain) | Guns |
| Gold | Bright gold (#f0c240) | Kill bounties, loot sales, encounter bonuses | Operating costs, merchants, hiring, Tier 1 building input |

---

## 6. Map Node Registry

| Node Type | What Happens | Scope |
|-----------|-------------|-------|
| Combat | Standard encounter (1-3 difficulty skulls) | v1 |
| Elite | Harder encounter, guaranteed rare+ loot, has a modifier | v1 |
| Merchant | Buy/sell weapons, trinkets, materials | v1 |
| Rest | No combat. Free panel repair. +2 bonus ticks. | v1 |
| Mystery | Random event (~60% positive, ~25% risk/reward, ~15% negative) | v1 |
| Boss | Mandatory chapter end. Guaranteed legendary loot. Multi-phase. | v1 |
| Companion recruitment | Named companion visible on map before choosing path | v1 |
| Forge site | Rare construction material (enchanted stone, living wood, star iron, cloudsilk) | post-v1 |
| Toll gate | Pay gold for a better route, or fight for free passage | post-v1 |

**v1 node types (7):** Combat, Elite, Merchant, Rest, Mystery, Boss, Companion recruitment.

---

## 7. Unlock Registry

> **v1 scope note:** v1 has no between-run unlock persistence. All v1-scoped content is available every run. The "within-run" milestones below (Beat Ch1/Ch2/Ch3 boss) DO apply in v1 — they gate content progression within a single run. The "across-run" milestones (Complete N runs, Beat Ch4+ boss) are post-v1 and are listed here for completeness only. See [implementation-decisions.md](implementation-decisions.md) §9 and [v1-scope.md](v1-scope.md) §Metagame.

### Production Buildings

| Milestone | Unlocks | Scope |
|-----------|---------|-------|
| Always available | Fletcher, Forge, Quarry, Lumberyard (Tier 1) | v1 |
| Beat Ch1 boss | Sawmill, Alchemist, Weaponsmith (Tier 2) | v1 |
| Beat Ch3 boss | Enchanter, Siege Works, Artificer (Tier 3) | post-v1 |

### Transport

| Milestone | Unlocks | Scope |
|-----------|---------|-------|
| Always available | Built-in stairs, ladders, dumbwaiters | v1 |
| Beat Ch1 boss | Chutes | v1 |
| Complete 2 runs | Cargo lifts | post-v1 |
| Beat Ch3 boss | Express lifts, conveyors | post-v1 |
| Beat Ch4 boss | Pneumatic tubes | post-v1 |

### Weapons

| Milestone | Unlocks | Scope |
|-----------|---------|-------|
| Always available | Bows, crossbows, melee | v1 |
| Beat Ch2 boss | Staves, thrown | v1 |
| Beat Ch3 boss | Guns, whips | post-v1 |
| Beat Ch4 boss | Instruments, shields | post-v1 |

> **Companion-only weapons (v1):** Pistol (Rust) and Hammer (Stone) exist in v1 as companion-fixed equipment. They do not appear in loot, merchants, or hero inventory. See [implementation-decisions.md](implementation-decisions.md) §15.

### Hero Classes

| Milestone | Unlocks | Scope |
|-----------|---------|-------|
| Always available | Archer | v1 |
| Beat Ch2 boss | Engineer | v1 (within-run gate) |
| Complete 1 run (any class, any outcome past Ch2) | Commander | v1 (within-run gate) |
| Complete 2 runs | Scavenger | post-v1 |
| Complete 3 runs OR reach Ch4 once | Berserker | post-v1 |

> **v1 note:** In v1 (no between-run persistence), all 3 v1 classes are available from run start. The within-run gates for Engineer and Commander listed above are the post-v1 unlock path. v1 treats them as always available.

### Map Nodes

| Milestone | Unlocks | Scope |
|-----------|---------|-------|
| Always available | Combat, Rest, Boss, Merchant, Companion recruitment | v1 |
| Complete 1 run | Mystery | v1 (available from run start in v1, no between-run gate) |
| Beat Ch2 boss | Forge site | post-v1 |
| Beat Ch3 boss | Toll gate | post-v1 |

### Leg Types

| Milestone | Unlocks | Scope |
|-----------|---------|-------|
| Always available | Chicken legs | v1 |
| Beat Ch2 boss | Spider legs | post-v1 |
| Beat Ch3 boss | Mechanical treads | post-v1 |
| Beat the game (any destination) | Magical hover | post-v1 |

### Journey Destinations

| Milestone | Unlocks | Scope |
|-----------|---------|-------|
| Always available | The Harbor | v1 |
| Complete The Harbor | The Crossroads | post-v1 |
| Complete The Crossroads | The Grove | post-v1 |
| Complete The Grove | The Forge | post-v1 |

### Other

| Milestone | Unlocks | Scope |
|-----------|---------|-------|
| Complete any destination | Ascension system | post-v1 |
