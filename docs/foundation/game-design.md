## Supply Line — Game Design Document

> **This document describes the full vision.** It includes both v1 and post-v1 features. Before implementing, check [v1-scope.md](v1-scope.md) for what ships first. When this document conflicts with [implementation-decisions.md](implementation-decisions.md), the decisions doc wins.

### The 30-second pitch

A walking fortress crosses hostile territory. You build the supply chain inside it — stacking production floors, connecting them with runners and lifts — while personally defending its exterior. The logistics IS the strategy layer; the shooting IS the action layer. They share one screen: your factory inside, the fight outside. Howl's Moving Castle meets Factorio meets Elona Shooter.

### The 5-minute pitch

You are a walker-keeper — not a soldier, closer to a lighthouse keeper. You found an empty walking tower on the outskirts of a failing town and decided to walk it somewhere better. The tower is alive — inhabited by a spirit that makes it walk and expels intruders. You're its caretaker, it's your home.

Inside the tower, you build production floors (fletcher, forge, alchemist), connect them with transport (stairs, lifts, chutes), and hire runners to carry goods. The supply chain flows down to a warehouse at the tower base, then out to ammo racks on the exterior where you and your companions fight.

Outside, enemies approach from the right. You're fixed on a balcony, aiming with your mouse, shooting with weapon-dependent physics (bows arc, crossbows fly flat, staves fire straight). Companions auto-fire from other balconies based on orders you set during prep. The tower stops walking during encounters. When all enemies are dead, it walks again.

Between encounters, you navigate a Slay-the-Spire-style branching map, choosing between combat, merchants, rest stops, companion recruitment, and mystery events. Each run is a journey across a continent toward a destination — The Harbor, The Crossroads, The Grove, or The Forge. Each destination has different terrain, bosses, and a unique ending.

The game is a roguelike. Runs take 30-60 minutes. Content unlocks between runs (new buildings, weapons, classes, companions) — no permanent stat inflation, but a wider toolkit means more strategic options. Every run starts at the same stat baseline. Difficulty increases through stackable ascension modifiers after your first completion.

---

## Core Loop

```
PREP (turn-based)                    COMBAT (real-time)
┌──────────────────────┐             ┌──────────────────────┐
│ Build floors         │             │ Aim and shoot        │
│ Place transport      │   March →   │ Companions auto-fire │
│ Assign companions    │             │ Runners deliver ammo │
│ Configure orders     │             │ Enemies climb tower  │
│ Equip gear           │   ← End    │ Panels take damage   │
│ Choose map route     │             │ Triage with abilities│
└──────────────────────┘             └──────────────────────┘
```

**Prep phase:** tower stops. Unlimited time, limited ticks (action points). Build, repair, configure. Choose next node on the map. Hit "March."

**Combat phase:** tower stopped, enemies approach from the right. Hero is fixed on a balcony, aiming freely. Companions fire autonomously. Runners carry ammo inside the tower. Encounter ends when all enemies are dead. Tower resumes walking.

**The hard boundary matters.** You can't rewire logistics during combat. Your prep decisions are a bet. The encounter is the reveal.

→ Full phase details: [ui-ux.md](ui-ux.md) §II-IV

---

## The Tower

### Structure

The tower is a vertical cross-section always visible on screen. Left side: interior (production, transport, runners). Right side: exterior (wall panels, balconies, battlefield).

```
Floor 6:  [Alchemist]  |chute|    ║  balcony — companion
Floor 5:  [Enchanter]  |lift |    ║  balcony — HERO
Floor 4:  [Quarry]     |stair|    ║  balcony — companion
Floor 3:  [Fletcher]   |stair|    ║  balcony — companion
Floor 2:  [Forge]      |stair|    ║
Floor 1:  [Quarters]   |stair|    ║
Ground:   [WAREHOUSE]  |legs |    ═══════════════
```

### Growth

- **Height** gated by foundation/legs. Upgrade foundation to unlock more floor slots.
- **Width** expandable via structural upgrades. Wider = more floor space for transport + stability on rough terrain, but slower.
- **Floors cost production resources** to build (planks, stone, iron) — competing with ammo supply.
- **Balconies** (exterior fighting positions) and **offices** (interior companion positions) are built separately on floors.

### Foundation / Legs

The tower walks. Leg type determines terrain compatibility, max floors, speed, and character:

| Leg type | Max floors | Terrain | Character |
|----------|-----------|---------|-----------|
| Chicken legs (starter) | 4 | Stumbles on mountains | Wobbly, charming, Ghibli energy |
| Spider legs *(post-v1)* | 6 | All terrain, no penalties | Precise, mechanical, steady |
| Mechanical treads *(post-v1)* | 6 | Fast on flat, can't do mountains | Heavy, industrial, powerful |
| Magical hover *(post-v1)* | 8 | Ignores terrain | Ethereal, expensive, anti-magic vulnerable |

→ Full tower mechanics: this document §Supply Chain, [software-architecture.md](software-architecture.md) §III

---

## The Supply Chain

### The flow

**Production floor → Runner → Warehouse → Runner → Cache → Rack → Hero/Companion shoots**

| Node | What it is | Key decision |
|------|-----------|-------------|
| Production floor | A building on a floor that converts gold (Tier 1) or inputs (Tier 2-3) into resources | Which buildings, which floors, which chains |
| Runner | Human porter carrying crates between floors via transport | How many runners, where is the quarters |
| Warehouse | Central reservoir at ground floor, pools all production output | How much to stockpile vs. consume |
| Cache | Player-placed buffer on a specific floor, configured for one resource type | Which floors get caches, which resource |
| Rack | Small ammo buffer on each exterior balcony, fed through wall pass-through | Auto-configured to match occupant's weapon |

### Production tiers

| Tier | Buildings | Inputs | Available |
|------|----------|--------|-----------|
| 1 — Raw producers | Fletcher, Forge, Quarry, Lumberyard | Gold (operating cost) | Run 1 |
| 2 — Refiners | Sawmill, Alchemist, Weaponsmith | Tier 1 outputs | After Ch1 boss |
| 3 — Specialists | Enchanter, Siege Works, Artificer | Two Tier 1/2 outputs | After Ch3 boss |

### Transport

Built-in stairs are always available (left edge of tower, no floor width cost, slow, one runner at a time). Everything else is built during prep and costs floor width:

| Transport | Speed | Direction | Runner needed? | Unlocked |
|-----------|-------|-----------|---------------|----------|
| Stairs (built-in) | Slow | Both | Yes | Always |
| Ladder | Slower | Both (2 floors) | Yes | Always |
| Dumbwaiter | Slow | Both | No (autonomous) | Always |
| Chute | Fast | Down only | No | After Ch1 boss |
| Cargo lift | Medium | Both (programmable range, car count, batch/immediate) | Yes (rides it) | After 2 runs |
| Express lift | Fast | Both (skips floors) | Yes | After Ch3 |
| Conveyor | Medium | Horizontal | No | After Ch3 |
| Pneumatic tube | Very fast | Both (light items only) | No | After Ch4 |

→ Full transport + buffer mechanics: [software-architecture.md](software-architecture.md) §III (data model), [controls.md](controls.md) §VI (prep interaction)

### Runners

Semi-automated. They see demand (buildings with full output, caches/racks that are empty) and route themselves. Player controls three things:
1. **Quarters placement** — which floor. Runners spawn here. Placement determines logistics speed.
2. **Runner count** — upgrade the quarters for more runners. More = more throughput but higher salary and stairwell congestion.
3. **Priority** — flag a resource type during combat. Runners weight it higher.

Runners automatically choose the fastest transport (chute > lift > stairs), avoid congestion, and queue visibly when infrastructure is busy.

→ Full runner AI: [software-architecture.md](software-architecture.md) §V (transport system)

---

## Combat

### Hero

Fixed position on a balcony. Free aim with mouse. The hero is the primary damage dealer (50-70% of kills). The hero is untouchable — the TOWER takes damage, not the hero.

**5 hero classes** (3 in v1, 2 post-v1), each with an exclusive mechanic that shapes combat and logistics all run:

| Class | Exclusive mechanic | Identity |
|-------|-------------------|----------|
| Archer | **Focus** — 2s pause → 2x damage, perfect accuracy | Patience pays. Lean supply chain. |
| Engineer | **Overclock** — boost one building/transport to 200% for 20s | The tower is the weapon. |
| Commander | **Rally** — passive aura, companions within 2 floors get +30% fire rate, +20% accuracy | The team is the weapon. |
| Scavenger *(post-v1)* | **Scrounging** — kills drop free scrap ammo (70% damage, bypasses supply chain) | Self-sufficient chaos. |
| Berserker *(post-v1)* | **Rage** — adversity fills a rage meter → escalating damage/fire rate bonuses | Stronger in ruins. |

→ Full class design: [class-design.md](class-design.md) §I

### Hero stats

5 stats, each with clear gameplay consequences:
- **Precision** — accuracy, crit chance, ammo efficiency
- **Draw/Power** — projectile speed, damage
- **Tempo** — fire rate (more DPS but burns ammo faster)
- **Grit** — wall panel durability near your position
- **Salvage** — loot quality, scavenge rate

Leveling from kills. Stat points allocated per level. Perk tree every 5 levels (3 branches: Combat, Logistics, Defensive — mix freely across branches).

→ Full stats + perk tree: [class-design.md](class-design.md) §I, §V

### Weapons

9 base types, ~35 sub-types (v1 ships 5 base types, 11 sub-types — see [v1-scope.md](v1-scope.md)). Each weapon type has a distinct input model, projectile physics, ammo type, and unique ability:

| Type | Input feel | Ammo | Distinctive quality |
|------|-----------|------|---------------------|
| Bow | Draw-hold-release, arc preview | Arrows | Gravity-dependent, skill ceiling in arc mastery |
| Crossbow | Click, auto-reload wait | Bolts | Flat trajectory, patience between shots |
| Staff | Hold to channel | Mana crystals | Easiest aim, most expensive ammo |
| Thrown | Click to throw | Varies | Heavy arc, diverse sub-types (javelins, bombs, bolas) |
| Melee | Click to swing | None | Zero logistics, only hits tower face |
| Gun *(post-v1)* | Click, recoil recovery | Gunpowder | Recoil control IS the skill |
| Whip *(post-v1)* | Click to lash | None | Extended melee (2-3 floor reach), grab/pull enemies |
| Instrument *(post-v1)* | Rhythm beats | Mana crystals | Beat-based input, combo → crescendo replaces ability |
| Shield *(post-v1)* | Hold to block, RMB counter | None | Defensive primary, no standard fire |

Weapons have **rarity** (common → legendary, affects base stats) and one **modifier** slot (flaming, frost, efficient, cursed, etc. — random on drop, craftable at Enchanter).

**Trinkets:** one slot per character. Each creates a choice or moment, not a stat buff. Examples: "Last Arrow" (3x damage when rack nearly empty), "Time Crystal" (rewind 5s of combat, once per encounter), "Copycat" (weapon ability becomes nearest companion's passive).

→ Full equipment: [equipment.md](equipment.md)

### Companions

**Exterior companions** stand on balconies. Auto-fire with configurable targeting orders (ground/air/climbers/siege/boss) and fire discipline (free fire/conservative/hold). Named characters with personalities, passives, and dialogue that warms over encounters.

15 exterior companions, each with a VERB (v1 ships 8 — see [v1-scope.md](v1-scope.md)):

| Companion | Verb | What they do |
|-----------|------|-------------|
| Ren | AMPLIFY | Marks targets for +20% damage from all |
| Kael | BLOCK | Physically stops climbers at his position |
| Mira | EXPLODE | AoE shots, 2x ammo consumption |
| Yuki | HEAL | Shots repair wall panels instead of damaging enemies |
| Drift | SLOW | Hit enemies slowed |
| Volt | CHAIN | Shots chain between nearby enemies |
| Thorn | SNARE | Hit enemies can't climb |
| Bell | HARMONIZE | Accuracy aura + instrument rhythm mechanic |
| Sable | PROFIT | Shots generate gold instead of max damage |
| Ash | SCAVENGE | Loot rolls to base faster |
| Rust | SUPPRESS | Hit enemies act slower |
| Kit | FIX | Temp-repairs broken infrastructure mid-encounter |
| Shade | HIDE | Position treated as unoccupied by enemies |
| Stone | ANCHOR | Reduces tower sway, slows climbers at position |
| Wren | REDIRECT | Sends climbing enemies to empty floors |

**Interior companions** work inside the tower from offices. Don't fight. Affect logistics (v1 ships 2: Broth + Cog):

| Companion | Verb | What they do |
|-----------|------|-------------|
| Broth | FEED | Cooks meals from resources → companion buffs |
| Cog | PROTECT | Transport on floor immune to breakdown/sappers |
| Tinker | BUILD | One-use gadgets placed per floor each prep stop |
| Smuggler | ACQUIRE | Free resources from outside the system, uncertain quality |
| Dreamer | CHANNEL | Mid-combat tower spirit action (expel, seal, surge, tremble) |
| Saboteur | TRAP | Interior traps that damage/confuse breach enemies |
| Cartographer | MAP | Permanent hidden shortcuts accumulate over encounters |
| Forge | OPTIMIZE | Production speed boost to own + adjacent floors |

→ Full companion design: [class-design.md](class-design.md) §III-VI, [narrative.md](narrative.md) §V

### Companion relationships *(post-v1)*

Companions develop relationships through proximity (adjacent floors) and time (encounters together). An **affinity system** (Precision, Force, Guard, Cunning, Spirit) determines what bonuses each pair gets — different pairs give different bonuses. Relationships evolve through dialogue tiers (Stranger → Acquaintance → Friend → Close → Bonded). Bonded pairs unlock paired endings on victory. Romantic undertones are ambiguous by design. Relationship bonuses are strategically meaningful on higher difficulties, not merely narrative.

→ Full relationship system: [class-design.md](class-design.md) §VII

### Enemies

Enemies approach from the right. The tower stops during encounters. Enemy archetypes defined by behavior:

| Archetype | Behavior |
|-----------|----------|
| Grunt | Walks to base, attacks ground panel or climbs |
| Runner | Fast ground approach |
| Armored | Slow, high HP, soaks ammo |
| Climber | Scales tower face toward nearest defended position |
| Flyer (hoverer) | Hovers at range, shoots at panels |
| Flyer (dive bomber) | Dives at position, burst damage |
| Catapult | Long-range panel damage, splash to adjacent floors |
| Ram | Attacks foundation directly. Run-ender threat. |
| Siege tower | Docks at height, disgorges climbers bypassing ground defenses |
| Sapper | Climbs to destroy infrastructure (racks, caches, transport) |
| Boss | Unique per encounter, multi-phase |

### Damage model

**The tower IS the HP bar.** No hero HP, no companion HP.

- Each floor has a **wall panel** with HP. Enemies attack panels. Panel breaks = **breach** → enemies enter interior for 15 seconds (tower spirit expels them), wreck infrastructure. See [implementation-decisions.md](implementation-decisions.md) §12.
- **Foundation** has a separate HP pool. Rams target it. Foundation at 0 = tower collapses, run over.
- Breaches that aren't repaired stay open — next encounter enemies walk straight in.
- Interior enemies destroy transport infrastructure → logistics cascade (upper floors cut off from supply).

→ Full damage model: [implementation-decisions.md](implementation-decisions.md) §12 (raider expulsion timer), [telemetry-balance.md](telemetry-balance.md) §II (enemy metric targets), [balance-config.md](balance-config.md) (concrete stat values)

---

## The Journey

### Map

Slay-the-Spire branching map. Full chapter visible at once. 5 chapters per destination. ~35-45 nodes total, player visits ~20-25.

**Terrain is on the paths, not the nodes.** Legs affect difficulty on terrain, not access — any legs can take any path with varying penalties.

### Node types

| Node | What happens |
|------|-------------|
| Combat | Standard encounter (1-3 difficulty skulls) |
| Elite | Harder, guaranteed rare+ loot, has a modifier (shielded, frenzied, etc.) |
| Merchant | Buy/sell weapons, trinkets, materials. 4 merchant archetypes. |
| Rest | No combat. Free panel repair. +2 bonus ticks. |
| Mystery | Random event (~60% positive, ~25% risk/reward, ~15% negative) |
| Boss | Mandatory chapter end. Guaranteed legendary loot. Multi-phase. |
| Companion recruitment | Named companion visible on map before choosing path |
| Forge site | Rare construction material (enchanted stone, living wood, star iron, cloudsilk) |
| Toll gate | Pay gold for a better route, or fight for free passage |

→ Full map + node details: [procedural-generation.md](procedural-generation.md) §II, §IV-VII

### Encounter progression

| Chapter | Enemies | Teaches | Tower state |
|---------|---------|---------|-------------|
| 1 "The Road" | Grunts, runners, first climbers | Aiming, target priority, ammo awareness | 2-3 floors, basic |
| 2 "The Wilds" | Flyers, catapults, multi-wave | Fire discipline, orders, siege priority | 4-5 floors, first chain |
| 3 "The Frontier" | Sappers, siege towers, attrition | Infrastructure protection, sustained throughput | 6-7 floors, Tier 2 |
| 4 "The Siege" | Rams, combined arms, no-lull gauntlets | Combined threat management | 7-8 floors, full system |
| 5 "The End" | Everything combined, multi-phase bosses | Full mastery test | 8 floors, Tier 3 |

→ Full encounter progression: [procedural-generation.md](procedural-generation.md) §XI (tuning)

---

## Economy

### Three currencies

| Currency | Role | Constrains | When it bites |
|----------|------|-----------|---------------|
| **Ticks** | Pacing — how many actions per prep stop | What you CAN do | Early game (few ticks) |
| **Materials** | Production tradeoff — diverting output from ammo to construction | What you SHOULD do | Late game (chains at capacity) |
| **Gold** | Operating costs + shopping | What you can SUSTAIN | Mid-late game (burn rate) |

No action requires all three. Construction = ticks + materials. Hiring = ticks + gold. Configuration = free.

### Income

Kill bounties (hero earns 50% more), scavenged drops (sell vs. use), encounter completion bonuses, unused ticks → gold, merchant selling.

### Expenses

Tier 1 operating costs, runner salaries, companion wages, leg maintenance, repairs, merchant purchases.

→ Full economy: [procedural-generation.md](procedural-generation.md) §XII-XIII (balance config + telemetry loop)

---

## Metagame

**No permanent stat inflation; expanding toolkit.** Each run starts at the same stat baseline. Unlocking new weapon families, building tiers, and transport types widens your solution space — that's strategic capability, not raw power. Knowledge is the primary advantage; toolkit breadth is the secondary one.

- **Content unlocks** across runs: buildings, transport, weapons, classes, companions, map nodes, legs. Failed runs still unlock.
- **4 journey destinations:** Harbor → Crossroads → Grove → Forge. Different terrain, bosses, and endings.
- **Ascension system:** 14 stackable difficulty modifiers after first completion. Score-tracked.
- **5 hero classes:** unlocked across runs 1-3.
- **~21 companions:** pool grows across runs 1-8.
- **Seed system:** shareable 64-bit seed for community challenges.

→ Full metagame: [metagame.md](metagame.md)

---

## Narrative

**Tone:** Ghibli fantasy. Warm, whimsical, melancholic. The tower is a home under threat, not a war machine. Enemies are territorial creatures and desperate bandits, not an evil army. The journey is about finding belonging.

**The tower's spirit:** a presence, not a character. Makes the tower walk, expels intruders, communicates through rare "Tower Dream" events. Companions may attribute feelings to it (Bell hears it "pulling south," Yuki says the walls are "stressed"). Whether the spirit truly has preferences is an open question.

**Narrative delivery:** companion dialogue, tower visual/audio state, landscape, loot flavor text, mystery events. No cutscenes. No lore codex. Show, don't tell.

**Victory:** a quiet arrival. The tower stops walking. Companions step outside. Brief paired ending text. The journey was the game. The arrival is the exhale.

→ Full narrative: [narrative.md](narrative.md)

---

## Detailed Design Documents

| Document | Covers |
|----------|--------|
| [art-direction.md](art-direction.md) | Visual style, color language, character design, emotional reads |
| [design-system.md](design-system.md) | UI tokens, atoms, molecules, organisms, pages, state management |
| [software-architecture.md](software-architecture.md) | Rust/React stack, GameState, command pattern, systems, rendering, WASM bridge |
| [ui-ux.md](ui-ux.md) | Combat HUD, prep flow, map interaction, merchant, inventory, companions |
| [audio-direction.md](audio-direction.md) | Tower soundscape, combat audio, music, spatial audio, silence as design |
| [animation.md](animation.md) | Tween/bone/particle systems, per-entity animation specs |
| [controls.md](controls.md) | Weapon-dependent input models, rhythm mechanic, input abstraction |
| [equipment.md](equipment.md) | All weapons, modifiers, trinkets, rarity, melee mechanics |
| [class-design.md](class-design.md) | Hero classes, companion passives/synergies/scaling, relationships |
| [narrative.md](narrative.md) | World, tower spirit, hero, 21 named companions, destinations, emotional arc |
| [procedural-generation.md](procedural-generation.md) | Map gen, encounter gen, loot tables, merchant stock, seeds, anti-frustration |
| [metagame.md](metagame.md) | Unlocks, destinations, ascension, run structure, first 10 runs |
| [telemetry-balance.md](telemetry-balance.md) | Metrics, cognitive science heuristics, suggestions, balance feedback loop |
| [tech-performance.md](tech-performance.md) | Frame budgets, tick rates, profiling, dev workflow, CI/CD, asset pipeline |
