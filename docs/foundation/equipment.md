Project: SUPPLY LINE | Equipment Design

> **Status:** v1 and post-v1 vision. Check [v1-scope.md](v1-scope.md) before implementing any feature described here.

## I. Overview

Equipment has two slots per character: **weapon** (primary) and **trinket** (secondary). Weapons define how you fight. Trinkets create situational advantages and build interactions.

**Sidearm clarification:** every hero starts with a dagger in weapon slot 2. It auto-activates via standard weapon-swap (Tab) when the primary weapon's rack is empty. The dagger is a normal weapon occupying a real slot — it can be sold, replaced at a merchant, or swapped for any other weapon. There is no hidden third slot. If both weapon slots hold ranged weapons and ammo runs dry, the hero has no melee fallback. See [implementation-decisions.md](implementation-decisions.md) §14.

**Design principles:**
- Every weapon ability is a VERB that creates a distinct moment. No two abilities should feel like reskins of each other.
- Trinkets create CHOICES or MOMENTS, not stat buffs. Each trinket should change a decision or create an interaction that wouldn't exist without it. (In practice, some trinkets — Berserker's Tooth, Heartstring, Copycat — are build-defining passives rather than situational prompts. That's acceptable for Wild-tier trinkets; common/uncommon trinkets should stay closer to the "moment" end.)
- Weapon choice = logistics choice. Each weapon type consumes a specific resource (or none). Choosing a weapon commits you to a production chain. Zero-ammo weapons (melee, whip, shield) deliberately bypass this — they trade logistics freedom for combat limitations (range, coverage). If playtesting shows zero-ammo builds dominating because they sidestep the supply chain entirely, add a maintenance cost (whetstone crates, repair kits) to keep them in the logistics economy.
- Modifiers are the spice — they twist a weapon's identity without replacing it.

**Scope note:** this document describes the full aspirational weapon roster, not the v1 playable set. For first playable, prioritize: bows (3 sub-types), crossbows (2), staves (2), thrown (2), melee (2), one gun, one whip. Shields and instruments are deferred (instruments already marked as future work in controls.md). The full roster grows through updates. This keeps the initial balance surface tractable and the input-model teaching load manageable.

**Exception budget:** several items below create unique rules (Eldritch bow hits interior raiders, Phase Amulet shoots through walls, Time Crystal rewinds combat, Copycat clones companion passives). Each is exciting individually, but collectively they create a large special-case implementation and teaching burden. **Rule: no more than 3 "unique exception" items should be obtainable in any single run.** Legendary items are the natural home for exceptions; common/uncommon items should follow the standard rules of their weapon type.

---

## II. Weapon Base Types

9 base weapon types. Each has a distinct input model (see [controls.md](controls.md)), a distinct projectile behavior (see that doc's combat section), and a distinct ammo type.

| Base type | Ammo consumed | Input model | Aim feel | Logistics demand |
|-----------|--------------|-------------|----------|-----------------|
| Bow | Arrows (fletcher) | Draw-hold-release | Arc preview, gravity | Low-medium |
| Crossbow | Bolts (forge) | Click, auto-reload | Flat trajectory, wait | Medium |
| Staff/Wand | Mana crystals (alchemist) | Hold to channel | Straight line / homing | High (Tier 2 chain) |
| Thrown | Thrown supply (varies) | Click to throw | Landing indicator / arc | Medium |
| Melee | None | Click to swing | Reach zone highlight | Zero |
| Gun | Gunpowder (forge + alchemist) | Click, recoil recovery | Flat + recoil | High (Tier 2 chain) |
| Whip | None | Click to lash | Arc sweep | Zero |
| Instrument | Mana crystals (alchemist) | Rhythm beats | Pulse / wave / beam | High (Tier 2 chain) |
| Shield | None | Hold to block, RMB to counter | Block zone | Zero |

---

## III. Weapon Sub-Types & Abilities

### Bows (consume arrows)

| Sub-type | Fire rate | Damage | Range | Arc | Ability |
|----------|----------|--------|-------|-----|---------|
| Shortbow | Fast | Low | Short | Medium | **Rain** — fire 5 arrows in a high arc that rain down on an aimed landing zone. Anti-ground AoE from height. |
| Longbow | Slow | High | Long | Heavy | **Charged Shot** — hold to charge, piercing bolt passes through ALL enemies in a line. The sniper's burst. |
| Composite | Medium | Medium | Medium | Medium | **Trick Shot** — next arrow ricochets off the first enemy hit, strikes a second target behind cover or on the tower face. Precision + geometry. |
| Recurve | Fast | Medium | Medium | Light | **Suppressing Volley** — 3 rapid shots that each slow hit enemies by 50% for 3s. Not about damage — about locking a wave in place. |
| Greatbow | Very slow | Very high | Extreme | Very heavy | **Skyfall** — fire arrow straight up, rains down on a targeted area 3s later. Massive AoE. Can hit siege equipment at extreme range. |
| Hunting bow | Medium | Medium | Medium | Medium | **Marked Prey** — tag an enemy. All shots against it crit for 5s. Killed enemies always drop loot (passive). The economy bow. |
| Eldritch bow | Medium | Medium | Long | Homing | **Soul Arrow** — spectral arrow passes through the tower's own wall panels, hitting interior raiders. The only ranged weapon that can reach enemies inside. Consumes mana crystals instead of arrows. |

### Crossbows (consume bolts)

| Sub-type | Fire rate | Damage | Range | Trajectory | Ability |
|----------|----------|--------|-------|-----------|---------|
| Hand crossbow | Fast | Low | Medium | Nearly flat | **Grapple Bolt** — bolt with chain pulls hit enemy 1 floor toward the hero. Drags climbers down, drags flyers lower. Repositioning tool. |
| Heavy crossbow | Very slow | Very high | Long | Flat | **Snipe** — instant kill on any non-boss enemy below a HP threshold (design intent: kills anything up to armored tier; rams and siege may resist). 30s cooldown. The delete button. Threshold is a balance config value, not literally "any HP." |
| Repeating crossbow | 3-bolt burst | Medium | Medium | Flat | **Overdrive** — fire rate doubles for 5s (6 bolts per trigger). Burns ammo fast. Devastating sustained output. |
| Siege crossbow | Very slow | High | Long | Flat | **Scorpion Bolt** — pins enemy to the ground/wall with a chained bolt. Tethered for 10s (can't move). Siege equipment hit by Scorpion Bolt is disabled for the duration. |
| Clockwork crossbow | Auto + manual | Low + Medium | Medium | Flat | **Dual Fire** — weapon auto-fires at the nearest enemy while you manually aim a second bolt. Passive + active fire simultaneously. Ability: auto-fire triples speed for 5s. |

### Staves (consume mana crystals)

| Sub-type | Fire rate | Damage | Range | Trajectory | Ability |
|----------|----------|--------|-------|-----------|---------|
| Wand | Fast | Low | Medium | Straight line | **Chain Lightning** — projectile bounces to 3 additional enemies, each bounce at 80% previous damage. Multi-target tool. |
| Staff | Medium | Medium | Medium | Straight, slight AoE on impact | **Barrier** — place a magical wall on one floor's exterior. Blocks enemy projectiles (flyer shots, catapult rocks) for 10s. Defensive magic. |
| Scepter | Slow | High | Long | Straight, large AoE | **Meteor** — aim at a ground point, 2s delay, then massive AoE impact. Rewards prediction — aim where enemies WILL be. |
| Orb | Medium | Medium | Medium | Slight homing | **Leash** — tether to one enemy. For 8s, every shot auto-targets that enemy regardless of aim. Lock-on. Guaranteed hits, but only on one target. |
| Necromancer's staff | Medium | Low | Medium | Straight | **Army of the Dead** — all enemies killed in the last 10s rise as spectral allies (fight for you, 15s). Passive: kills raise individual spectral allies. Damage through necromancy. |
| Storm rod | Slow | Medium | Long | Drifting cloud | **Tempest** — summon a storm covering the entire right side of the screen. All enemies take continuous zap damage for 8s. The screen-clear. Normal fire: launches slow-moving thunderclouds that drift and zap nearby enemies. |

### Thrown (consume thrown supply)

| Sub-type | Fire rate | Damage | Range | Arc | Ability |
|----------|----------|--------|-------|-----|---------|
| Javelins | Medium | High | Long | Heavy | **Impale** — pin target to ground/wall (immobile 5s). Climbers pinned to wall face. Siege equipment crews pinned (weapon stops functioning). |
| Throwing knives | Fast (3 per throw) | Low each | Short | Light | **Mark for Death** — marked knife makes target take double damage from ALL sources for 8s. The setup tool — mark the armored enemy, let everyone focus fire. |
| Bombs | Medium | High (AoE) | Medium | Heavy, bounces once | **Cluster Bomb** — splits into 3 bomblets mid-air, each landing in a different spot. Triple AoE coverage. |
| Fire flasks | Medium | Medium (area DOT) | Medium | Medium | **Wall of Fire** — throw a line of fire along the tower base. Ground enemies are heavily slowed and take burn DOT crossing it for 8s. Rams and siege may push through at reduced speed. The panic button for ground waves. |
| Boomerang | Medium | Medium | Medium | Returns on hit | **Whirlwind** — throw 3 boomerangs in a spread, all return. Ammo not consumed on successful return (passive). The self-sustaining thrown weapon. |
| Bolas | Medium | Low | Medium | Heavy | **Net Toss** — large net captures all enemies in an area (immobile 3s). Climbers caught in the net fall 1 floor. Anti-climber specialist. |
| Chakram | Fast | Medium | Medium | Bounces between enemies | **Ricochet Storm** — chakram bounces up to 8 times between enemies. Each bounce does full damage. The ping-pong projectile. |

### Melee (no ammo)

| Sub-type | Speed | Damage | Reach | Special | Ability |
|----------|-------|--------|-------|---------|---------|
| Dagger | Very fast | Low | At position only | Always available as sidearm | **Flurry** — 5 rapid stabs in 1s. Each hit has 30% chance to produce 1 scrap ammo (Scavenger synergy). |
| Sword | Medium | Medium | At position | — | **Riposte** — counter-stance for 3s. Next climber that attacks your panel is instantly killed (any HP) + counter hits all climbers on the same floor. |
| Spear | Slow | High | 1 floor below | Hits climbing enemies below you | **Thrust** — stab downward with extreme reach (3 floors below). Hits enemies no other weapon can reach from this height. |
| Hammer | Very slow | Very high | At position | Knockback on every hit | **Shockwave** — slam the tower wall. ALL climbers on the entire tower face take damage and are knocked down 2 floors. Costs: minor panel damage to your own floor. The "reset the wall" button. |
| Flail | Slow | High | 1 floor above/below | Inaccurate (can miss in melee) | **Whirlwind** — spin the flail for 3s, damaging everything within 2 floors constantly. Area denial. |
| Gauntlets | Very fast | Low | At position | Can grab + throw small enemies off tower | **Falcon Punch** — one-shot any non-boss enemy and send them flying off the tower face. Extremely satisfying. Long cooldown. |
| Scythe | Slow | Very high | At position | Natural cleave (hits all enemies at position) | **Reap** — for 5s, every kill heals 10 wall panel HP. Offense becomes defense. The vampiric melee. |

### Guns (consume gunpowder)

Gunpowder is a Tier 2 resource (Forge output + Alchemist output → gunpowder). Expensive to produce but devastating.

| Sub-type | Fire rate | Damage | Range | Recoil | Ability |
|----------|----------|--------|-------|--------|---------|
| Pistol | Fast | Medium | Medium | Moderate | **Deadeye** — next 3 shots have zero recoil, each hit refunds 1 gunpowder. Rewards accuracy under pressure. |
| Musket | Very slow | Very high | Long | Massive | **Overloaded Shot** — double powder charge, pierces ALL enemies in a line AND the first wall panel it hits repairs 10 HP. Offense becomes defense. |
| Blunderbuss | Medium | High (cone) | Short | Heavy | **Point Blank** — next shot at a climber ON your position does 5x damage and knocks them down 3 floors. Only works on enemies that have reached you. "Get off my wall." |
| Hand cannon | Slow | High (explosive arc) | Medium | Heavy | **Siege Breaker** — fires a heavy round at siege equipment. 3x damage to catapults, siege towers, rams. 1x to normal enemies. The anti-siege specialist. |

### Whips (no ammo)

Extended melee with 2-3 floor reach. Can grab/pull enemies off the tower face.

| Sub-type | Speed | Damage | Reach | Special | Ability |
|----------|-------|--------|-------|---------|---------|
| Leather whip | Fast | Low | 2 floors | — | **Crack** — sonic boom stuns ALL enemies within reach for 2s AND disrupts one flyer (forces it to land for 5s, becoming a ground target). Crowd control that reaches the sky. |
| Chain whip | Medium | Medium | 3 floors | Longest melee reach | **Grapple** — grab one enemy and pull them to your position. Small enemies = instant kill. Large enemies = repositioned to your floor. You choose which threat to bring close. |
| Thorn whip | Medium | Medium | 2 floors | Bleed DOT on hit | **Entangle** — thorny vines spread across 2 floors of the tower face. Enemies climbing through take continuous bleed AND climb at 50% speed for 10s. Area denial on the wall. |
| Fire whip | Slow | High | 2 floors | Ignites enemies on hit | **Ring of Fire** — burning ring on tower face centered on position (2 floor radius). Enemies inside take fire DOT, entering enemies ignited. 8s duration. Turns your wall section into a furnace. |

### Instruments (consume mana crystals — rhythm mechanic)

Instruments do NOT use standard fire. They use the beat-based rhythm system (see [controls.md](controls.md)). Click on the beat = effect, miss = whiff. 4 consecutive on-beats = **crescendo** (replaces Q weapon ability).

| Sub-type | BPM | Beat interval | Per-beat effect | Crescendo (at 4 combo) |
|----------|-----|---------------|----------------|------------------------|
| War drum | 80 | 0.75s | AoE pulse damages all enemies within 3 floors | **Thunderclap** — massive pulse + 2s stun, all enemies on screen |
| Battle horn | 120 | 0.5s | Sonic blast in aimed direction, knockback | **War Song** — all companions within 3 floors +100% fire rate for 5s |
| Lute | 160 | 0.375s | Note chains into beam on target (breaks if you miss a beat) | **Siren Song** — charm 2 non-boss enemies (fight for you, 10s) |
| Bell | 60 | 1.0s | Single devastating toll, AoE centered on position | **Death Knell** — triple-power toll + all panels regen 5 HP + damages siege towers |

### Shields (no ammo — defensive/counter-attack)

Shields use a unique input model: hold LMB to block, RMB to counter-attack while blocking. No standard fire.

| Sub-type | Block speed | Coverage | Counter | Ability |
|----------|-----------|----------|---------|---------|
| Buckler | Fast | Own position only | Reflects projectiles back at 2x damage | **Riposte** — next 3 reflected projectiles deal 3x damage. Turns enemy ranged attacks against them. |
| Tower shield | Slow | Own position + 1 floor above/below | No counter — pure defense | **Phalanx** — panels within 2 floors take 80% reduced damage for 5s. (Not zero — siege equipment and rams should still dent. The shield is powerful, not absolute.) |
| Spiked shield | Medium | Own position | Damages melee climbers on contact | **Shield Bash** — knock ALL climbers at position down 3 floors. Crowd control. |
| Mirror shield | Medium | Own position | Reflects magic projectiles only | **Dazzle** — flash blinds all enemies on screen for 3s (stop moving, stop attacking). Panic button. |

---

## IV. Weapon Modifiers

Each weapon has one modifier slot. Modifiers are randomly rolled on loot drops and can be rerolled or applied specifically at the Enchanter production building (see [procedural-generation.md](procedural-generation.md) for drop rules).

### Combat modifiers

| Modifier | Effect | Best on |
|----------|--------|---------|
| Flaming | Shots ignite enemies, 3s burn DOT. Bypasses armor. | Slow weapons (burn does work between shots) |
| Frost | Shots slow enemies 30% for 2s | Fast weapons (keep the slow rolling) |
| Explosive | AoE damage on impact (small radius) | Single-target weapons that lack AoE |
| Piercing | Projectiles pass through first target, can hit second | Ranged weapons with clear line-of-sight |
| Vampiric | Kills repair 5 panel HP at your position | Melee and close-range (most kills near your panel) |
| Venomous | Hit enemies deal 50% less panel damage for 3s | Defensive builds focused on panel survival |

### Economy modifiers

| Modifier | Effect | Best on |
|----------|--------|---------|
| Efficient | Uses 0.5 ammo per shot (doubles effective rack life) | High fire rate weapons (doubles the value) |
| Gilded | Kills generate +20% gold bounty | High-kill-volume weapons |
| Scavenging | Kills guarantee a resource drop (normally chance-based) | Any — pure economy |

### Utility modifiers

| Modifier | Effect | Best on |
|----------|--------|---------|
| Silent | Climbers don't path toward your position (treated as empty) | High positions where you don't want to be targeted |
| Beacon | Companions within 2 floors get +10% accuracy | Central positions near companion cluster |
| Magnetic | Missed projectiles curve slightly toward nearest enemy | Low-Precision builds, Scavenger class |

### Drawback modifiers (powerful + costly)

| Modifier | Upside | Downside | Risk/reward |
|----------|--------|----------|-------------|
| Cursed | +50% damage | Rack drains 10% faster (phantom ammo consumption) | Big damage, faster ammo depletion |
| Heavy | +80% damage | -30% fire rate | Massive hits, slow pace |
| Fragile | +40% damage | Weapon breaks after 2 encounters (needs Enchanter repair) | Temporary power spike |
| Bloodthirsty | +30% damage | Critical hits consume 3 ammo instead of 1 | Crits are expensive — avoid crit builds, or embrace the cost |

### Modifier crafting at the Enchanter

The Enchanter (Tier 3 production building) enables modifier manipulation:

| Action | Cost | Result |
|--------|------|--------|
| **Reroll** | 3 mana crystals | Random new modifier (removes old one). Gambling. |
| **Apply specific** | Varies (see below) | Deterministic. Choose the modifier you want. |

**Specific modifier costs:**

| Modifier | Materials required |
|----------|-------------------|
| Flaming | 3 mana crystals + 2 fire oil |
| Frost | 3 mana crystals + 2 stone |
| Efficient | 5 mana crystals |
| Explosive | 4 mana crystals + 3 stone |
| Piercing | 3 mana crystals + 2 bolts |
| Silent | 3 mana crystals + 2 planks |
| Cursed | 3 mana crystals (cheap — the curse IS the cost) |
| Vampiric | 4 mana crystals + 2 fire oil |

Crafting consumes warehouse materials — competing with ammo production. Enchanting a weapon mid-run diverts resources from combat supply. The logistics tension extends to gear progression.

---

## V. Trinkets

One trinket slot per character (hero + companions). Each trinket creates a **choice, moment, or interaction** — not a stat bump.

### Combat trinkets

| Trinket | Effect | The decision it creates |
|---------|--------|------------------------|
| **"Last Arrow"** | When rack drops to 1 crate, next shot does 3x damage | Do you shoot faster to trigger the spike, or conserve ammo? |
| **"Returner's Coin"** | 25% of missed shots ricochet off ground, hit nearest enemy for 50% damage | Makes low-Precision builds less punishing. Misses become grazing hits. |
| **"Twin Fang"** | Every 5th shot fires a bonus projectile at the nearest enemy you're NOT targeting | Passive multi-target. Free damage on flanking enemies without splitting attention. |
| **"Berserker's Tooth"** | When your panel drops below 50% HP: +30% fire rate, +20% damage | Fight harder when your home is breaking. Berserker class fantasy as a trinket for any class. |

### Logistics trinkets

| Trinket | Effect | The decision it creates |
|---------|--------|------------------------|
| **"The Leech"** | Companion below's kills deposit 1 ammo to your rack | Parasitic resupply. Put a high-kill companion below you. Placement synergy. |
| **"Echo Chamber"** | Weapon ability cooldown halved, but ability draws ammo from companion ABOVE's rack | Powerful abilities, but you're stealing from your neighbor. Who sits above you matters. |
| **"Smuggler's Pouch"** | Once per encounter, produces 1 crate of whichever resource your rack needs most | Emergency supply, no logistics required. A lifeline, not a strategy. |
| **"Magnet Stone"** | Loot from kills within 2 floors rolls to base at 3x speed, +10% chance of being an item instead of resources | The loot-hunter's trinket. Pair with Ash (Salvage) for maximum economy. |

### Defensive trinkets

| Trinket | Effect | The decision it creates |
|---------|--------|------------------------|
| **"Phase Amulet"** | Once per encounter, 5 shots pass through your own wall panels, hitting interior raiders | The only way to damage enemies INSIDE the tower from outside. A unique breach-response moment. |
| **"Decoy Charm"** | Projects a phantom defender at your position. Climbers approach slower (confused), sappers target phantom (50% chance) instead of real infrastructure | Misdirection. The phantom does nothing but absorbs attention and protects infrastructure. |
| **"Tower Heart"** | Interior raiders on your floor expelled in 8s instead of 15-20. If Dreamer companion is aboard, drops to 5s | The tower's spirit reinforces YOUR section. Synergy with Dreamer companion. |
| **"Scarecrow"** | Flyers avoid your floor (won't target your panel from range). They redirect to other floors | You're safe from flyer damage — but you're pushing the problem to companions. Coverage vs. safety. |

### Wild trinkets (rare, build-warping)

| Trinket | Effect | Why it changes everything |
|---------|--------|--------------------------|
| **"Heartstring"** | Weapon ability heals nearest companion for 5% of damage dealt | Offensive ability becomes partly restorative. With Vampiric weapon + Heartstring = combat healer. |
| **"Time Crystal"** | Once per encounter, rewinds 3 seconds of combat. Enemy positions, panel HP revert. Your ammo does NOT revert. (Legendary only — this is the highest-complexity exception item in the game. Implementation requires state snapshots every tick. Defer to post-v1 if snapshot cost is prohibitive.) | The tactical undo. Catapult just breached your enchanter? Rewind, kill it this time. |
| **"Resonance Bell"** | If Bell companion is adjacent, your weapon's projectile hits generate half-beats in the rhythm system. Non-instrument weapons can build toward crescendo (at 2x normal beat requirement) | Blurs the line between weapon types. A bow user adjacent to Bell can earn crescendo through combat rhythm. |
| **"Copycat"** | Weapon ability changes to match the NEAREST companion's passive. Adjacent to Ren = Mark. Adjacent to Kael = brief Wall. Adjacent to Shade = brief Vanish | Placement IS the build. Your ability changes based on who's next to you. Infinite flexibility, requires planning. |

---

## VI. Weapon Rarity & Naming

### Rarity tiers

| Rarity | Stat multiplier | Modifier | Drop source | Visual |
|--------|----------------|----------|-------------|--------|
| Common | 1.0x | No modifier (clean weapon, no twist) | Regular enemies, early chapters | Gray border |
| Uncommon | 1.15x | 50% chance | Regular enemies, merchants | Green border |
| Rare | 1.3x | Always has one | Elite encounters, Ch3+ merchants | Blue border |
| Legendary | 1.5x | Always has one (from "good" pool, no drawbacks) | Boss drops, rare elites | Gold border + shimmer |

### Name generation

Composed from: [Modifier] [SubType]

```
"Flaming Longbow"
"Efficient Hand Crossbow"
"Cursed War Drum"
"Frost Gauntlets"
```

Legendary items have unique hand-written names (~50 in pool):

```
"The Silence" — a Silent longbow with extreme range
"Heartstring" — a Vampiric lute with healing properties
"Old Reliable" — an Efficient composite bow with no flashy modifier but exceptional base stats
"Thunder's Echo" — a storm rod that fires TWO thunderclouds per shot
"The Last Word" — a heavy crossbow whose Snipe has no cooldown (but costs 3 ammo)
```

---

## VII. Companion Equipment

Companions equip **weapon + trinket**, same item pool as the hero.

**Key differences from hero equipment:**
- Companions use hit/miss based on accuracy stat — no physics simulation for their projectiles
- Companions can't use the rhythm mechanic for instruments (instruments are deferred to future work and should remain hero-only even then — the rhythm identity requires player input)
- Companions' weapon abilities fire automatically when off cooldown (targeting based on their orders)
- Shield companions auto-block when enemies are within range

**Identity note:** some weapons lose their distinctive feel in companion hands. Bow draw-hold-release, gun recoil management, and aimed abilities (Meteor, Charged Shot) all depend on player skill — companions bypass this via auto-fire. That's fine for most weapons (the *stats* matter more than the *feel* for companions), but weapons whose identity IS the input model (rhythm instruments, shields) should be restricted to hero-only or given companion-specific variants. As a rule: if a weapon's fantasy collapses without player input, it's a hero weapon.

**Equipment assignment is a prep-phase decision.** The companion's weapon determines their ammo type → which cache they need → which production chain feeds them. Equipping a companion with a crossbow means building a forge for bolts.

---

## VIII. Melee in a Fixed-Position Game

Melee weapons hit enemies ON the tower face at or near the hero's position.

### Hit zones

1. **At your floor** — climbers who've reached your position. Point-blank. They hack the panel, you swing at them.
2. **Within reach above/below** — weapons with reach (spear = 1 floor below, chain whip = 3 floors, flail = 1 floor up/down) can hit climbers still climbing. Active defense zone.
3. **Ground enemies at base** — only from floor 1-2. Can swing down at grunts. Floor 3+ can't reach ground.

### What melee CAN'T hit

Ground enemies at range. Flyers. Siege equipment. Melee heroes are specialist anti-climber defenders who NEED ranged companions for everything else.

### Melee positioning

| Position | Pros | Cons |
|----------|------|------|
| Low (floor 1-2) | Hit ground enemies + climbers. Broad coverage. | Climbers reach you fast. Less time to react. |
| Mid (floor 3-4) | Watch climbers approach, kill on arrival. Sweet spot. | Can't help ground or air. |
| High (floor 5+) | Few climbers reach you (companions/defenses filter them). | The ones that DO reach are the tough survivors. |

### Melee as a logistics choice

Zero ammo. No fletcher, no cache, no rack on your floor. Entire production chain freed for companions. The "zero logistics footprint" build. Pairs with Kael (shield-bearer blocks climbers below, melee hero kills what gets through).

### Melee sidearm

Every hero carries a dagger regardless of primary weapon. Auto-switches when ammo rack empties. Short reach, low damage — functional but desperate. The fallback, not a build.

---

## IX. Balance Config

All equipment values are in the runtime balance config (no recompile to adjust).

```toml
[weapons]
# Base stats per sub-type: damage, fire_rate, range, arc_gravity
# Rarity multipliers
common_multiplier = 1.0
uncommon_multiplier = 1.15
rare_multiplier = 1.3
legendary_multiplier = 1.5

# Ability cooldowns (seconds)
[weapons.abilities]
shortbow_rain_cooldown = 12
longbow_charged_shot_cooldown = 8
heavy_crossbow_snipe_cooldown = 30
# ... etc

[weapons.modifiers]
flaming_dot_duration = 3.0
flaming_dot_dps = 5.0
frost_slow_percent = 0.30
frost_slow_duration = 2.0
efficient_ammo_multiplier = 0.5
cursed_damage_bonus = 0.50
cursed_drain_rate = 0.10
# ... etc

[weapons.instruments]
war_drum_bpm = 80
battle_horn_bpm = 120
lute_bpm = 160
bell_bpm = 60
rhythm_on_beat_window_ms = 100
rhythm_close_window_ms = 200
rhythm_perfect_window_ms = 30
rhythm_on_beat_power = 1.0
rhythm_close_power = 0.6
rhythm_perfect_power = 1.5
crescendo_combo_required = 4

[trinkets]
last_arrow_damage_multiplier = 3.0
last_arrow_threshold_crates = 1
returners_coin_ricochet_chance = 0.25
returners_coin_damage = 0.50
time_crystal_rewind_seconds = 5.0
# ... etc

[loot]
# See procedural-generation.md for full drop tables
weapons_per_run_target = [12, 16]
legendaries_per_run_target = 4
```

**Telemetry cross-reference:** weapon type diversity, ability usage rates, modifier pick distribution, trinket pick rates. See [telemetry-balance.md](telemetry-balance.md) §VI.
