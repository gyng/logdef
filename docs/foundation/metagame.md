Project: SUPPLY LINE | Metagame Design

> **Status:** v1 and post-v1 vision. Check [v1-scope.md](v1-scope.md) before implementing any feature described here.

## I. Philosophy

**Unlocks expand options, not base power.** Each run starts at the same stat baseline regardless of how many runs you've completed. No "+5% damage forever." No meta-currency. What changes between runs is the toolkit available — more buildings, transport, weapons, companions, map nodes, hero classes.

**Honesty about what unlocks do:** unlocking Tier 2 buildings, cargo lifts, new weapon families, and additional classes doesn't increase your stats, but it absolutely widens your solution space. A player with 9 weapon types and 21 companions available can build towers that a first-run player cannot. That's power in the broadest sense — not stat inflation, but strategic capability. The distinction matters: a veteran and a newcomer with identical unlocks start on equal footing. But a veteran with full unlocks and a newcomer on run 1 do not have equal options. Knowledge is the primary advantage; toolkit breadth is the secondary one.

**Why this model, not permanent stat growth:**
- The game is about mastering systems. Meta-stat bonuses would undermine that — you'd win through accumulation, not skill.
- Slay the Spire, FTL, and Into the Breach use this model successfully. Each run is self-contained at the stat level.
- Failed runs still unlock content — you never feel a run was "wasted." You learned something AND you unlocked something.

---

## II. Content Unlock System

Content unlocks are gated by progress milestones. Each unlock expands the player's toolkit for future runs without making them more powerful.

### Production buildings

| Unlock | Requirement | What it adds |
|--------|------------|--------------|
| Tier 1 buildings (Fletcher, Forge, Quarry, Lumberyard) | Always available | Basic resource production |
| Tier 2 buildings (Sawmill, Alchemist, Weaponsmith) | Beat Chapter 1 boss | Refined resources, production chains begin |
| Tier 3 buildings (Enchanter, Siege Works, Artificer) | Beat Chapter 3 boss | Advanced resources, modifier crafting, deep chains |

### Transport infrastructure

| Unlock | Requirement | What it adds |
|--------|------------|--------------|
| Built-in stairs + ladders | Always available | Basic vertical transport |
| Dumbwaiters | Always available | Autonomous transport, no runner needed. Available from run 1 to show transport is a real system. |
| Chutes | Beat Chapter 2 boss | Fast downward transport |
| Cargo lifts | Complete 2 runs (any outcome) | Programmable transport (floor range, car count, departure mode) |
| Express lifts | Beat Chapter 3 boss | Long-distance fast transport with stop selection |
| Conveyor belts | Beat Chapter 3 boss | Horizontal transport within floors |
| Pneumatic tubes | Beat Chapter 4 boss | Very fast lightweight item transport |

### Weapon types

| Unlock | Requirement | What it adds |
|--------|------------|--------------|
| Bows | Always available | Starting ranged option |
| Crossbows | Always available | Flat trajectory, reload-based rhythm. Available from run 1 to show weapon variety matters. |
| Melee (as primary) | Always available | Zero-logistics option (also always available as sidearm) |
| Staves | Beat Chapter 2 boss | Easy aim, mana-consuming |
| Thrown | Beat Chapter 2 boss | Arc-based, diverse sub-types |
| Guns | Beat Chapter 3 boss | Gunpowder chain, recoil mechanic |
| Whips | Beat Chapter 3 boss | Extended melee, grab/pull |
| Instruments | Beat Chapter 4 boss | Rhythm mechanic, support/AoE |
| Shields | Beat Chapter 4 boss | Block/counter, defensive primary |

### Hero classes

| Unlock | Requirement | What it adds |
|--------|------------|--------------|
| Archer | Always available | Focus mechanic, precision playstyle |
| Engineer | Beat Chapter 2 boss | Overclock mechanic, tower-first playstyle |
| Commander | Complete 1 run (any class, any outcome past Ch2) | Rally mechanic, companion-focused playstyle |
| Scavenger | Complete 2 runs | Scrounging mechanic, economy playstyle |
| Berserker | Complete 3 runs OR reach Chapter 4 once | Rage mechanic, chaos playstyle |

### Map node types

| Unlock | Requirement | What it adds |
|--------|------------|--------------|
| Combat, Rest, Boss | Always available | Core encounter loop |
| Merchant | Always available | Buy/sell, economy decisions. Run 1 needs the full economy loop visible. |
| Mystery | Complete 1 run | Random events, narrative moments |
| Companion recruitment | Always available | Named companion roster. Companions are the game's emotional core — don't hide them behind a gate. |
| Forge site | Beat Chapter 2 boss | Specialty construction materials |
| Toll gate | Beat Chapter 3 boss | Economy gamble, premium routes |

### Companion pool

| Stage | Pool size | How |
|-------|-----------|-----|
| Run 1 | 5-6 companions available | Starting roster (mix of exterior + interior) |
| Run 2 | 8-9 | +2-3 unlocked by completing first run |
| Run 3 | 11-12 | +2-3 more |
| Run 4 | 14-16 | +2-3 more |
| Run 5+ | 18-21 | Full roster available. +1-2 per run until complete. |

Each completed run (victory OR reaching a new chapter milestone) adds new companions to the recruitment pool for all future runs. Which companions unlock is fixed (not random) — the roster reveals in a designed order that introduces different passive types gradually.

**Unlock order design:** early unlocks are simpler companions (Drift, Kael, Mira — clear passives, easy to use). Later unlocks are more complex (Dreamer, Smuggler, Wren — unique mechanics, build-around potential). Interior companions unlock later than exterior (the player needs to understand logistics before managing it).

### Leg types

| Unlock | Requirement | What it adds |
|--------|------------|--------------|
| Chicken legs | Always available | Starter. Moderate speed, stumbles on mountains. |
| Spider legs | Beat Chapter 2 boss | All-terrain generalist. Slow but steady. |
| Mechanical treads | Beat Chapter 3 boss | Fast on flat terrain. Can't handle mountains. |
| Magical hover | Complete the game (any destination) | Ignores terrain. Expensive. Vulnerable to anti-magic. |

---

## III. Journey Destinations

Multiple endpoints on the continent. Each destination has a different Chapter 5 and final boss. Choose destination at run start. Unlocking destinations provides replay variety through different endgames, different map layouts, and different narrative contexts.

| Destination | Theme | Ch5 terrain | Final boss | Unlock requirement |
|-------------|-------|-------------|------------|--------------------|
| The Harbor | Coastal trade city. Sea, salt, commerce. | Coast | Leviathan (flying boss, sea-storm) | Always available |
| The Crossroads | Legendary road junction. Maintained by road-keepers. | Plains/roads | Road Spirit (climbing boss, corrupted) | Complete The Harbor |
| The Grove | Forest sanctuary. Healthy land spirits. | Living forest | Forest Guardian (ground boss, ancient) | Complete The Crossroads |
| The Forge | Legendary workshop. First walking towers built here. | Industrial ruins | Forge Guardian (ground boss, automaton) | Complete The Grove |

Each destination changes:
- **Chapter terrain distribution** — The Harbor has more swamp/coast. The Forge has more mountain/industrial.
- **Boss sequence** — each chapter has a boss tailored to that destination's terrain and narrative.
- **Map structure** — node layouts differ. The Harbor has more merchant nodes (trade route). The Forge has more forge sites (rare materials).
- **Narrative context** — companion dialogue references the destination. "The Harbor? I hear you can still buy fresh fish there." "The Forge is just a myth." "The Grove... I've heard the trees still sing."
- **Victory ending** — each destination has unique arrival text and companion paired endings.

**Decision: sequential unlock, leaning into it.** Requiring Harbor → Crossroads → Grove → Forge reads as an escalating campaign. That's fine — each destination assumes familiarity with the previous one's terrain types, enemy archetypes, and logistics patterns. Not harder stats, but more system experience expected. The sequence is a teaching progression, not a difficulty ladder.

Each destination has unique challenges, not strictly harder ones:
- Harbor: water terrain, weather encounters, sea creatures
- Crossroads: open terrain, large enemy groups, rival walkers
- Grove: dense terrain, spirit enemies, environmental healing (enemies too)
- Forge: industrial hazards, mechanical enemies, extreme heat events

The difficulty comes from ascension modifiers, not destination choice.

---

## IV. Ascension System

After completing any destination once, the player unlocks **ascension modifiers** — optional difficulty increases that can be stacked for harder runs.

### Modifier list

Each modifier adds to a cumulative **ascension score**. Higher score = harder run. Leaderboards track highest score completed per destination per class.

**Modifier categories:** modifiers fall into two types, and the distinction matters for readability:

- **Scalar modifiers** (Iron Enemies, Dull Blades, Glass Tower, Inflation, etc.): change numbers. The game plays the same way with harder values. These are safe to stack and easy to understand.
- **Ruleset modifiers** (Fog of War, Early Sappers, No Rest): change what systems exist or when they appear. These alter how the game teaches and what strategies are viable. They're qualitatively different from "enemies have more HP" — they distort the rules, not just the difficulty curve.

Ruleset modifiers should be capped: max 2 active simultaneously. Stacking 4+ ruleset modifiers risks making the game unrecognizable rather than harder. Scalar modifiers can stack freely.

| Modifier | Score | Effect |
|----------|-------|--------|
| **Iron Enemies** | +10 | All enemies +20% HP |
| **Lean Rations** | +10 | 1 fewer tick per prep stop |
| **Inflation** | +10 | Merchant prices +50% |
| **Early Sappers** | +15 | Sappers appear from Chapter 1 (normally Chapter 3) |
| **No Rest** | +15 | Rest nodes removed from all maps |
| **Rough Roads** | +15 | All terrain penalties doubled |
| **Dull Blades** | +20 | Hero damage -15% |
| **Glass Tower** | +20 | All wall panels have -25% HP |
| **Green Recruits** | +15 | Companions start with -20% accuracy |
| **Skeleton Crew** | +25 | Maximum 4 companions (instead of 6) |
| **Famine** | +20 | Production buildings produce 20% slower |
| **No Mercy** | +30 | Foundation HP does not regenerate between encounters |
| **The Long Road** | +20 | Each chapter has +2 nodes (more encounters per chapter) |
| **Fog of War** | +15 | Map only shows 1 node ahead instead of full chapter |

### Ascension score thresholds

| Score | Tier name | Meaning |
|-------|-----------|---------|
| 0 | Standard | Base difficulty. The intended first-clear experience. |
| 10-25 | Ascension I | "I know the systems. Challenge me." |
| 30-50 | Ascension II | "I've mastered the basics. Break the rules." |
| 55-80 | Ascension III | "Make me suffer." |
| 85+ | Ascension IV | Theoretical maximum stacking everything. Bragging rights only. |

### Ascension rewards

No mechanical rewards for ascension — it's pure bragging rights. Possible non-mechanical rewards:
- **Cosmetic tower skins** — visual variations on the tower exterior at certain score thresholds
- **Title/badge** — shown on the run summary and shared with seed
- **Leaderboard entry** — seed + class + destination + score + completion status

---

## V. Run Structure

### New game flow

1. **Title screen** → New Game
2. **Class select** — choose from unlocked classes. Each shows: starting stats, starting weapon, class perk, exclusive mechanic description, playstyle summary.
3. **Destination select** — choose from unlocked destinations. Each shows: theme, chapter terrains, final boss silhouette, estimated difficulty notes.
4. **Ascension select** (if any destinations completed) — toggle modifiers on/off. Shows cumulative score.
5. **Seed** (optional) — enter a specific seed or accept random. Displayed for sharing.
6. **Run begins** — tower stands up, Chapter 1 map reveals.

### Run end states

| Outcome | When | What carries over |
|---------|------|-------------------|
| **Victory** | Tower reaches destination. Final boss defeated. | Unlock progress. Run telemetry saved. Destination counts as "completed." Seed + stats shareable. |
| **Foundation destroyed** | Foundation HP reaches 0 during encounter. | Unlock progress earned up to this point. Run telemetry saved. |
| **Abandon** | Player chooses to quit at a settlement/rest node. | Unlock progress earned. Framed as "the tower rests here." Not a failure — a choice. |

**Failed runs still unlock content — but not everything.** The unlock table uses two types of milestones:

- **Reach milestones** (persistence): "reach Chapter 3," "complete 2 runs (any outcome)." These unlock through persistence. Getting there is enough — you've demonstrated you can survive that long, and you've seen the systems.
- **Beat milestones** (mastery): "beat Chapter 3 boss," "complete The Harbor." These require winning a specific fight or finishing a destination. They gate the most impactful unlocks (Tier 3 buildings, exotic weapons, new destinations, ascension system).

This split means: a player who keeps dying in Chapter 3 will still unlock new companions, some transport, and new classes through persistence. But they won't unlock Tier 3 buildings, guns, or The Crossroads until they actually beat a boss. The content ladder has both a gentle slope (persistence) and real gates (mastery).

### Between runs

After a run ends (victory, death, or abandon):

1. **Run summary screen** — stats, telemetry highlights, companion roster, tower final state. Seed displayed.
2. **Unlock notification** — any new content unlocked is shown with brief description. "Unlocked: Cargo Lifts — programmable transport with floor ranges and car count settings."
3. **Return to title** — start a new run or review unlocks.

There is no "home base," no persistent currency, no permanent upgrades. The between-run experience is: summary → unlocks → new run. Clean, fast, no bloat.

**Run archive (light history, not a hub):** the game has strong found-family themes and tower-as-home attachment. Losing all trace of a run feels at odds with that. A simple run archive — accessible from the title screen — stores a one-page summary per completed run: tower silhouette, companion roster, destination, outcome, seed, and any Bonded companion pair endings. No gameplay function, no currency, no upgrades. Just a shelf of memories. "I remember the run where Drift and Ren bonded and we barely survived The Forge." This is implementation-cheap (the data already exists in RunTelemetry) and emotionally high-value.

---

## VI. Seed System & Community

### Seed sharing

Every run has a 64-bit seed that determines all procedural generation (maps, encounters, loot, events). Same seed + same class + same destination + same ascension modifiers = same experience (minus player skill).

**Use cases:**
- **Challenge runs:** "Try seed 847291 as Engineer on The Forge with Iron Enemies + Glass Tower. I barely survived Chapter 4."
- **Speedruns:** specific seeds become known "good" seeds for speedrun categories.
- **Balance testing:** developers can reproduce exact run conditions from a seed.
- **Bug reports:** seed included automatically — developers can replay the exact run.

### Seed display

- Shown on pause menu during run
- Shown on run summary at end
- Copiable to clipboard with one click
- Included in screenshot/share export

---

## VII. Unlock Pacing — The First 10 Runs

The unlock system should feel like a steady drip of new toys. Each run should unlock SOMETHING. Here's the designed unlock curve:

**Run 1 (first attempt, any outcome):**
- Available: Archer class, bows + crossbows + melee (sidearm always available, but also as primary choice), Tier 1 buildings, stairs + ladders + dumbwaiters, combat/rest/merchant/boss nodes, chicken legs, 5-6 companions (including 1 interior), The Harbor destination
- Run 1 should be a true sample of the game's promise, not a stripped tutorial. The player needs to experience logistics depth (dumbwaiters show transport isn't just stairs), economy decisions (merchant nodes show the gold loop), and weapon variety (3 input models show weapons feel different). If run 1 is too bare, players quit before discovering what the game actually is.
- Even a failed run (died in Ch1-2) unlocks content for run 2.

**What run 1 withholds:** advanced production chains (Tier 2+), programmable transport (lifts), exotic weapons (staves, guns, instruments), non-Archer classes, alternate destinations, and the deeper companion roster. These are "more of the same systems but richer," not "the real game that was hiding."

**After Run 1 (regardless of outcome):**
- Unlock: mystery nodes, +2-3 new companions
- The player's second run adds narrative variety and roster depth.

**After Run 1 (if beat Ch1 boss):**
- Also unlock: Tier 2 buildings, chutes
- The production chain deepens.

**Run 2-3:**
- Playing with new weapons, new nodes, new companions.
- Beat Ch2 boss → unlock: staves, thrown, spider legs, forge sites, Engineer class, chutes
- Complete a run (any outcome past Ch2) → unlock: Commander class

**Run 3-5:**
- Full weapon variety emerging. Transport options expanding.
- Beat Ch3 boss → unlock: guns, whips, treads, toll gates, Tier 3 buildings, express lifts, conveyors
- Complete 2 runs → unlock: Scavenger class
- Complete The Harbor → unlock: The Crossroads destination

**Run 5-8:**
- Deep mechanics unlocking.
- Beat Ch4 boss → unlock: instruments, shields, pneumatic tubes
- Complete 3 runs → unlock: Berserker class
- Complete The Crossroads → unlock: The Grove

**Run 8+:**
- Full content palette available. Every weapon, every building, every transport, every class, every companion.
- Complete The Grove → unlock: The Forge (hardest destination)
- Complete any destination → unlock: ascension system
- Beat the game → unlock: magical hover legs

**By run 10, everything is unlocked.** Further runs are about: trying new classes, new destinations, ascension challenges, seed challenges, perfecting builds.

---

## VIII. What the Game Does NOT Have

Explicitly excluded from the metagame:

- **No permanent stat bonuses.** No "+5% damage forever after run 3."
- **No meta-currency.** No "souls" or "cells" or "orbs" spent at a hub.
- **No home base / hub world.** No persistent space between runs. Run summary → new run.
- **No daily challenges.** (Could be added post-launch but not in v1.)
- **No multiplayer leaderboards.** (Could be added, but seeds are the community feature, not competition.)
- **No battle pass / seasonal content.** One-time purchase, all content included.
- **No save scumming.** One save per run, overwritten on each encounter. Can't reload to undo a bad fight. (Abandon is always available as the "I don't want to continue" option.)

---

## IX. Balance Config

```toml
[meta.unlocks]
# Milestone → unlock mapping
# Format: "milestone_id" = ["unlock_1", "unlock_2", ...]
beat_ch1_boss = ["tier2_buildings"]
beat_ch2_boss = ["staves", "thrown", "spider_legs", "forge_sites", "engineer_class", "chutes"]
beat_ch3_boss = ["guns", "whips", "treads", "toll_gates", "tier3_buildings", "express_lifts", "conveyors"]
beat_ch4_boss = ["instruments", "shields", "pneumatic_tubes"]
complete_1_run = ["mystery_nodes", "commander_class"]
complete_2_runs = ["cargo_lifts", "scavenger_class"]
complete_3_runs = ["berserker_class"]
complete_harbor = ["crossroads_destination"]
complete_crossroads = ["grove_destination"]
complete_grove = ["forge_destination"]
complete_any_destination = ["ascension_system"]
beat_game = ["hover_legs"]

[meta.companions]
# Companion unlock order (fixed, not random)
# Early: simple passives. Late: complex mechanics.
run_1_pool = ["Drift", "Kael", "Mira", "Ren", "Broth"]
run_2_adds = ["Yuki", "Stone", "Tinker"]
run_3_adds = ["Rust", "Bell", "Cog"]
run_4_adds = ["Thorn", "Ash", "Forge"]
run_5_adds = ["Sable", "Volt", "Smuggler"]
run_6_adds = ["Kit", "Shade", "Cartographer"]
run_7_adds = ["Wren", "Dreamer", "Saboteur"]
run_8_adds = ["Anthem_if_kept_or_replacement"]

[meta.ascension]
modifiers = [
    { id = "iron_enemies", score = 10, label = "Iron Enemies", desc = "All enemies +20% HP" },
    { id = "lean_rations", score = 10, label = "Lean Rations", desc = "1 fewer tick per prep stop" },
    { id = "inflation", score = 10, label = "Inflation", desc = "Merchant prices +50%" },
    { id = "early_sappers", score = 15, label = "Early Sappers", desc = "Sappers appear from Chapter 1" },
    { id = "no_rest", score = 15, label = "No Rest", desc = "Rest nodes removed from maps" },
    { id = "rough_roads", score = 15, label = "Rough Roads", desc = "Terrain penalties doubled" },
    { id = "dull_blades", score = 20, label = "Dull Blades", desc = "Hero damage -15%" },
    { id = "glass_tower", score = 20, label = "Glass Tower", desc = "Panel HP -25%" },
    { id = "green_recruits", score = 15, label = "Green Recruits", desc = "Companions start -20% accuracy" },
    { id = "skeleton_crew", score = 25, label = "Skeleton Crew", desc = "Max 4 companions" },
    { id = "famine", score = 20, label = "Famine", desc = "Production -20% speed" },
    { id = "no_mercy", score = 30, label = "No Mercy", desc = "Foundation HP no regen between encounters" },
    { id = "long_road", score = 20, label = "The Long Road", desc = "+2 nodes per chapter" },
    { id = "fog_of_war", score = 15, label = "Fog of War", desc = "Map shows only 1 node ahead" },
]

[meta.save]
saves_per_run = 1          # one autosave, overwritten each encounter
allow_manual_save = false  # no save scumming
save_on_encounter_end = true
save_on_prep_complete = true
```

**Telemetry cross-reference:**
- Track unlock pacing: are players seeing new content each run? If 80% of players quit before run 3 and never unlock Tier 2, the first-run experience needs more hooks. See [telemetry-balance.md](telemetry-balance.md) §V.
- Track ascension adoption: what % of players who complete the game try ascension? Which modifiers are most/least popular? If "No Rest" is never picked, rest stops are too valuable (or the modifier is too punishing).
- Track destination completion rates: if The Forge is completed by <5% of players who attempt it, it may be too hard or the earlier destinations aren't preparing players well enough.
