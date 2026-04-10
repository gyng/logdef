Project: SUPPLY LINE | Procedural Generation

> **Status:** v1 and post-v1 vision. Check [v1-scope.md](v1-scope.md) before implementing any feature described here.

## I. What's Generated Per Run

**Scope:** this document covers both procedural generation algorithms and the balance parameters they consume. The two are tightly coupled — changing a drop rate changes what the generator produces — so they live together. Sections I-IX define generation rules. Sections X-XIII define balance calibration, config exposure, and the telemetry feedback loop.

Every run is unique. The following are procedurally generated at run start or during the journey:

| System | When generated | Seed source |
|--------|---------------|-------------|
| Chapter maps (node layout, paths, terrain) | Run start (all 5 chapters) | Run seed |
| Encounter compositions (enemy types, counts, waves) | When entering a combat node | Run seed + node index |
| Loot drops (weapon type, rarity, modifier) | On enemy death / encounter end | Run seed + encounter index + kill index |
| Merchant stock | When entering a merchant node | Run seed + node index |
| Mystery events | When entering a mystery node | Run seed + node index |
| Companion placement on map | Run start | Run seed |
| Forge site materials | Run start | Run seed |
| Boss variants (within type) | Run start per chapter | Run seed + chapter |

**Everything is seeded.** The run seed determines all randomness. Same seed = same run (deterministic). Seeds are displayed on the run summary screen for sharing/reproducing.

**All numerical parameters are exposed in the balance config** (see [tech-performance.md](tech-performance.md) — balance values as runtime config). No recompile needed to adjust budgets, costs, drop rates, wave timings, or map structure. Edit the config file, reload, test. Telemetry metrics (see [telemetry-balance.md](telemetry-balance.md)) validate whether changes produce the target outcomes — e.g., adjusting grunt HP should move the "grunt survival time" metric toward the 1-3 second target.

---

## II. Map Generation

### Chapter layout algorithm

Each chapter has a defined structure (columns, node count, branching factor) but the specific layout is generated.

**Input parameters per chapter:**

| Chapter | Columns | Nodes per column | Total nodes | Branching | Boss |
|---------|---------|-----------------|-------------|-----------|------|
| 1 | 3 | 1-2 | 4-5 | Minimal (1-2 paths) | Fixed type per destination |
| 2 | 4 | 2-3 | 6-8 | Moderate | Fixed type per destination |
| 3 | 5 | 2-3 | 8-10 | Full | Fixed type per destination |
| 4 | 5 | 2-3 | 8-10 | Full | Fixed type per destination |
| 5 | 4 | 2-3 | 6-8 | Converging | Fixed type per destination |

**Generation steps:**

1. **Place columns left to right.** Each column has N node slots (from the table above). Slots are vertically distributed with even spacing + slight random offset (±15% of spacing) for organic feel.

2. **Assign node types.** Each chapter has a node type budget:

| Chapter | Combat | Elite | Merchant | Rest | Mystery | Recruitment | Forge | Toll |
|---------|--------|-------|----------|------|---------|-------------|-------|------|
| 1 | 2-3 | 0 | 0-1 | 0-1 | 0-1 | 0-1 | 0 | 0 |
| 2 | 3-4 | 0-1 | 1 | 1 | 0-1 | 0-1 | 0-1 | 0 |
| 3 | 3-5 | 1-2 | 1 | 1-2 | 1-2 | 0-1 | 0-1 | 0-1 |
| 4 | 4-6 | 1-2 | 1-2 | 1 | 1-2 | 0-1 | 0-1 | 0-1 |
| 5 | 3-4 | 1-2 | 0-1 | 1 | 0-1 | 0 | 0-1 | 0 |

Node types are assigned to slots using weighted random selection from the budget. Rules:
- Boss is always the final node.
- First column is always combat (ease into the chapter).
- Rest never appears in the first column.
- Elite never appears in column 1 of chapters 1-2.
- Recruitment nodes show a specific companion from the available pool (determined by run seed).
- No two rest nodes adjacent (same column or connected path).
- At least one path through the chapter must include a merchant OR rest (guaranteed breathing room).

3. **Connect nodes.** Edges connect nodes in adjacent columns. Rules:
- Every node must have at least 1 incoming and 1 outgoing edge (except start and boss).
- Edges go left-to-right only (no backtracking).
- Edges prefer connecting vertically nearby nodes (short vertical distance) with 70% probability, and occasionally cross to distant nodes (30%) for variety.
- The boss node receives edges from ALL nodes in the second-to-last column (all paths converge).
- No orphaned nodes (every node is reachable from start).

4. **Assign terrain to edges.** Each edge (path between nodes) gets a terrain type. Terrain distribution depends on the chapter's setting and the journey destination:

| Destination | Ch1 terrain | Ch2 terrain | Ch3 terrain | Ch4 terrain | Ch5 terrain |
|-------------|------------|------------|------------|------------|------------|
| Harbor | plains, forest | plains, swamp | swamp, forest | storm, swamp | coast (new) |
| Crossroads | plains | plains, forest | mountain, forest | mountain, desert | plains (roads) |
| Grove | plains, forest | forest | forest, swamp | dying forest | living forest |
| Forge | plains | plains, mountain | mountain | mountain, storm | industrial (new) |

Within a chapter, terrain is distributed by weighted random selection from the chapter's terrain pool. Adjacent edges on the same path tend toward the same terrain (70% chance of repeating) for geographic coherence.

5. **Assign path distances.** Each edge gets a distance (Short / Medium / Long) which determines the tick budget for that stop:
- Short: base ticks - 1
- Medium: base ticks (standard for the chapter)
- Long: base ticks + 2

Distribution: 30% short, 50% medium, 20% long. Paths leading to rest/merchant nodes tend toward medium/long (more prep time for shopping/building).

6. **Assign combat difficulty.** Each combat node gets a difficulty (1-3 skulls):
- Chapter 1: all difficulty 1
- Chapter 2: 70% difficulty 1, 30% difficulty 2
- Chapter 3: 30% difficulty 1, 50% difficulty 2, 20% difficulty 3
- Chapter 4: 10% difficulty 1, 40% difficulty 2, 50% difficulty 3
- Chapter 5: all difficulty 3 (no easy encounters in the final chapter)

Elite nodes are always difficulty 3. Difficulty also scales slightly based on column position (later columns within a chapter are +0.5 difficulty on average).

### Map validation

After generation, the map is validated:
- Every node is reachable from start
- Every node has a path to the boss
- At least one "breathing room" path exists (merchant or rest accessible without elite)
- No more than 3 combat nodes in a row on any single path
- If validation fails: regenerate with a modified seed (seed + 1) until valid. This retry is part of the canonical seed derivation — the "run seed" displayed to the player is the *original* seed, and the retry count is deterministic (same seed always retries the same number of times and settles on the same valid map). Seed sharing works correctly because the retry logic is pure.

---

## III. Encounter Generation

When the player enters a combat node, the encounter composition is generated.

### Encounter parameters

Each encounter is defined by:
- **Difficulty** (1-3, from the map node)
- **Chapter** (determines available enemy archetypes)
- **Terrain modifier** (from the path terrain)
- **Elite modifier** (if elite node — Shielded, Frenzied, etc.)
- **Wave count** (1-3 waves per encounter)

### Enemy budget system

Each encounter has a **threat budget** (points to spend on enemies). Budget scales with difficulty and chapter:

```
base_budget = (chapter - 1) * 10 + difficulty * 12
elite_multiplier = 1.5 (if elite node)
budget = base_budget * elite_multiplier
```

| Example | Budget | ~Enemies |
|---------|--------|----------|
| Ch1 D1 | 12 | 5-6 grunts |
| Ch3 D2 | 44 | mixed mid-game wave |
| Ch5 D3 | 76 | heavy late-game |
| Ch5 D3 elite | 114 | endgame gauntlet |

### Enemy costs

| Archetype | Cost | Available from |
|-----------|------|---------------|
| Grunt | 2 | Chapter 1 |
| Runner | 3 | Chapter 1 |
| Armored | 8 | Chapter 2 |
| Climber | 4 | Chapter 2 |
| Flyer (hoverer) | 5 | Chapter 2 |
| Flyer (dive bomber) | 6 | Chapter 3 |
| Catapult | 12 | Chapter 3 |
| Siege tower | 15 | Chapter 3 |
| Ram | 20 | Chapter 4 |
| Sapper | 6 | Chapter 3 |

### Wave composition algorithm

1. **Determine wave count.** Difficulty 1: 1 wave. Difficulty 2: 1-2 waves (50/50). Difficulty 3: 2-3 waves (30/70). Elite: always 2-3 waves.

2. **Distribute budget across waves.** Escalating: Wave 1 gets 30%, Wave 2 gets 35%, Wave 3 gets 35%. The player's supplies deplete over waves, so later waves feel harder even at similar budget — the natural "things get tighter" arc emerges from ammo depletion, not from budget escalation alone.

3. **Fill each wave from the enemy pool.** For each wave:
   a. Start with the wave's budget portion.
   b. Randomly select an enemy archetype from available pool (chapter-gated).
   c. If it fits the remaining budget, add it. If not, try a cheaper archetype.
   d. Repeat until budget is spent (±5 points tolerance).
   e. Ensure at least 3 enemies per wave (minimum viable wave).

4. **Apply composition rules:**
   - No more than 2 catapults per wave (visual clutter, too punishing)
   - No more than 1 ram per encounter (run-ender should be singular)
   - Rams only appear in wave 2+ (never the first thing you see)
   - At least 50% of wave 1 should be grunts/runners (familiar enemies first)
   - Sappers never appear alone — always mixed with other climbers (they sneak in)
   - Siege towers always come with 3-5 "free" climbers (disgorged on dock — these are on top of the budget)

5. **Apply terrain modifier to composition:**
   - Forest: +30% climber weight, -50% flyer weight (dense canopy grounds flyers)
   - Mountain: +30% flyer weight (open skies), +catapult weight
   - Swamp: +30% grunt weight (slowed, more numerous), -siege weight (too heavy for terrain)
   - Storm: 0% flyers (grounded by storm), +30% ground weight
   - Night: +30% runner weight (they sneak in the dark), -catapult weight (can't aim)

6. **Apply elite modifier:**
   - Shielded: all enemies +50% HP (applied post-generation)
   - Frenzied: all enemies +30% speed (applied post-generation)
   - Regenerating: all enemies regenerate 1% HP/sec (applied post-generation)
   - Armored vanguard: wave 1 is 100% armored enemies (override wave 1 composition)
   - Sapper swarm: add 3-4 sappers spread across waves (on top of budget)

### Lull timing

Between waves: 10-15 second lull (randomized). During the lull, no new enemies spawn. Existing enemies continue fighting (climbers on the wall don't pause). The lull is for cache/rack refilling, not for full safety.

If the encounter is aggressive (difficulty 3, chapter 4+): 30% chance the lull is shortened to 5-8 seconds. 10% chance there's no lull at all (waves overlap — the next wave spawns while the previous is still active).

### Enemy spawn positions

Ground enemies spawn at x positions between 800-1200px to the right of the tower (off-screen or screen edge). Staggered spawn times within a wave (not all at once — 0.5-2s between spawn groups).

Flyers spawn at x=1000-1400px at random heights within the tower's floor range.

Climbers spawn at the tower base (they run in from the right first, then climb).

Siege towers and catapults spawn far right (x=1200+) and approach slowly.

---

## IV. Loot Generation

### Drop chance per enemy

| Enemy | Drop chance | Drop quality |
|-------|------------|-------------|
| Grunt | 5% | Common only |
| Runner | 5% | Common only |
| Armored | 15% | Common-Uncommon |
| Climber | 8% | Common only |
| Flyer | 10% | Common-Uncommon |
| Catapult | 30% | Uncommon-Rare |
| Siege tower | 25% | Uncommon |
| Ram | 50% | Rare |
| Sapper | 20% | Common-Uncommon |
| Boss | 100% | Legendary guaranteed |
| Elite encounter bonus | 100% | Rare+ guaranteed (on top of enemy drops) |

### Resource drops

Every killed enemy has a separate chance to drop raw resources (independent of gear drops):

| Chapter | Resource drop chance | Resource types |
|---------|---------------------|----------------|
| 1-2 | 20% per kill | wood, stone, arrows |
| 3-4 | 15% per kill | wood, stone, arrows, bolts, planks |
| 5 | 10% per kill | all types |

Resource amount: 1-3 units per drop, weighted toward 1. Higher-value enemies (armored, siege) drop more.

### Weapon generation

When a weapon drops, its properties are generated:

1. **Base type.** Weighted by chapter and what the player uses:

```
base_weights = {
    bow: 20, crossbow: 15, staff: 10, thrown: 10,
    melee: 10, gun: 8, whip: 8, instrument: 5, shield: 5
}

// Increase weight for types the player hasn't found yet this run
// Decrease weight for types the player has 3+ of in inventory
// Apply chapter gating (guns not available until unlocked, etc.)
```

2. **Sub-type.** Random within the base type. Equal weight across sub-types. Higher chapters unlock later sub-types (greatbow only from Chapter 3+, clockwork crossbow from Chapter 4+).

3. **Rarity.** Determined by enemy/encounter quality:

| Source | Common | Uncommon | Rare | Legendary |
|--------|--------|----------|------|-----------|
| Regular enemy | 70% | 25% | 5% | 0% |
| Armored/siege enemy | 40% | 40% | 18% | 2% |
| Elite encounter bonus | 0% | 20% | 65% | 15% |
| Boss | 0% | 0% | 30% | 70% |

Rarity affects base stats (damage, speed, range multiplied by rarity tier: common 1.0x, uncommon 1.15x, rare 1.3x, legendary 1.5x).

4. **Modifier.** Uncommon: 50% chance of modifier. Rare: 100% chance. Legendary: 100% chance + modifier is drawn from a weighted "good" pool (no drawback modifiers on legendary).

Modifier selection is weighted random from the modifier pool:

```
combat_modifiers = { flaming: 10, frost: 10, explosive: 8, piercing: 8, vampiric: 6, venomous: 6 }
economy_modifiers = { efficient: 8, gilded: 6, scavenging: 6 }
utility_modifiers = { silent: 5, beacon: 5, magnetic: 5 }
drawback_modifiers = { cursed: 4, heavy: 4, fragile: 3, bloodthirsty: 3 }  // excluded from legendary
```

5. **Name generation.** Composed from: [Modifier] [SubType]:

```
"Flaming Longbow"
"Efficient Hand Crossbow"
"Cursed War Drum"
"Frost Gauntlets"
```

Legendary items get unique names from a hand-written pool (~50 names):

```
"The Silence" (a silent longbow)
"Heartstring" (a vampiric lute)
"Old Reliable" (an efficient composite bow with no special properties but very high base stats)
```

### Trinket generation

Trinkets drop less frequently than weapons. When one drops:

1. **Category.** Equal weight: combat, logistics, defensive, weird.
2. **Specific trinket.** Random within category. No duplicates of currently equipped trinkets (re-roll if duplicate).
3. **Rarity.** Same table as weapons. Higher rarity trinkets have stronger effects (+15% range on uncommon → +25% on rare).

---

## V. Merchant Stock Generation

When the player enters a merchant node, the merchant's inventory is generated.

### Merchant archetype

Determined by the map node (assigned during map generation). Archetype affects stock weighting:

| Archetype | Weapons | Trinkets | Materials | Specialty |
|-----------|---------|----------|-----------|-----------|
| Arms Dealer | 3 | 1 | limited | 0 |
| Supplier | 1 | 0-1 | full stock | 1-2 |
| Collector | 1-2 (higher rarity) | 1-2 | limited | 0-1 (unique) |
| Traveler | 2 | 1 | moderate | 0-1 |

### Weapon stock

Generated like loot drops but with controlled rarity:
- Chapter 1-2 merchants: 60% common, 30% uncommon, 10% rare
- Chapter 3-4 merchants: 30% common, 40% uncommon, 25% rare, 5% legendary
- Chapter 5 merchants: 10% common, 30% uncommon, 40% rare, 20% legendary

Collector merchants shift the pool upward (+1 rarity tier on average).

### Pricing

```
base_price = {
    common: 30, uncommon: 60, rare: 120, legendary: 250
}

// Modifier adjustments
price += modifier_value * 10  // good modifiers cost more
price -= drawback_discount * 15  // drawback modifiers are cheaper

// Archetype adjustments
if collector: price *= 1.3  // premium
if traveler: price *= 0.8   // discount
```

Material prices are fixed per unit: wood 2g, stone 3g, planks 4g, bolts 5g, mana crystals 8g.

Specialty items have hand-set prices: repair kit 25g, runner boots 40g, enchanted stone 60g, compass 35g.

### Material stock

Materials available at merchants come from a fixed set but quantities vary:
- Each material type: 3-10 units available (random)
- Supplier merchants: double quantities
- Materials the player is LOW on (warehouse < 5 units) have +50% chance of appearing in higher quantities (the game subtly helps without being obvious)

---

## VI. Mystery Event Selection

When the player enters a mystery node, an event is selected from the pool.

### Event pool

~15 events total. Weighted by category:
- Positive: 60%
- Risk/reward: 25%
- Negative: 15%
- Weird: 5% (rare, rolled separately — if weird triggers, it overrides the category roll)

### Selection rules

- No event repeats within the same run (once seen, removed from pool)
- Events requiring specific conditions are filtered:
  - "Abandoned companion" only appears if roster < 6
  - "The Collector" (trade companion for legendary) only appears if roster ≥ 2
  - "Toll bridge" only appears if player has ≥ 30 gold
  - "Gambling den" only appears if player has ≥ 20 gold
  - "Time loop" only appears after Chapter 2 (needs a previous encounter to replay)
- Ambush events are weighted lower in Chapter 1 (5%) and higher in Chapter 4+ (20%)
- The "Tower Dream" event can only appear once per run

### Event outcome randomization

Events with random outcomes use the run seed + event index:
- Gambling den: 50/50 win/lose
- Cursed shrine: buff/debuff values are fixed (not random per instance)
- Hidden cache: resource types and amounts randomized (2-5 crates of 1-3 resource types)
- Wandering enchanter: always offers exactly one free reroll
- Abandoned companion: selected from the companion pool (companions not already on the map as recruitment nodes)
- Earthquake: floor selection is random (weighted toward floors WITH buildings — empty floors are boring to damage)

---

## VII. Companion Placement

At run start, the companion pool is populated and distributed across the map.

### Pool population

1. Start with all unlocked companions (~4-15 depending on meta progression)
2. Remove companions that don't fit the destination (some companions are destination-specific — future feature)
3. Select 6-8 companions for this run's pool (random, weighted by diversity — ensure at least one of each passive type if possible)
4. Assign 2-4 to recruitment nodes on the map (distributed across chapters, max 1 per chapter)
5. Remaining 2-4 are available as abandoned companion mystery events

### Recruitment node placement

- Chapter 1: 0-1 recruitment nodes
- Chapter 2: 0-1 recruitment nodes
- Chapter 3: 1 recruitment node (guaranteed — player needs companions by now)
- Chapter 4: 0-1 recruitment nodes
- Chapter 5: 0 recruitment nodes

Recruitment nodes are placed on non-dominant paths (not the "easy" route — you have to go slightly out of your way to recruit).

---

## VIII. Boss Generation

Boss type is fixed per chapter per destination (not random). This ensures the player can see the boss on the map at chapter start and plan for it.

| Destination | Ch1 Boss | Ch2 Boss | Ch3 Boss | Ch4 Boss | Ch5 Boss |
|-------------|----------|----------|----------|----------|----------|
| Harbor | Ground (ram) | Flying (flock) | Climbing (kraken) | Ground (siege engine) | Flying (leviathan) |
| Crossroads | Ground (beast) | Climbing (giant) | Flying (dragon) | Ground (ram horde) | Climbing (road spirit) |
| Grove | Climbing (treant) | Ground (beast pack) | Flying (swarm) | Climbing (corruption) | Ground (forest guardian) |
| Forge | Ground (automaton) | Ground (siege engine) | Climbing (colossus) | Flying (storm) | Ground (forge guardian) |

Boss HP, phase transitions, and attack patterns are hand-designed per boss (not generated). The procedural element is limited to slight stat scaling based on the player's tower size and companion count at that point.

### Boss scaling

```
hp_multiplier = 1.0 + (tower_floors - 4) * 0.1 + (companion_count - 2) * 0.05
damage_multiplier = 1.0 + (chapter - 1) * 0.15
```

A player with 8 floors and 5 companions faces a boss with 1.55x HP compared to a player with 4 floors and 2 companions. This prevents over-leveled towers from trivializing bosses.

**Communicating scaling to the player:** boss scaling must not feel like hidden punishment for building well. The boss HP bar on the map preview screen should show the scaled HP value (not the base), and companion hints can reference the boss's perceived strength: "That thing looks tougher than the last one — it's sizing us up." The player should understand intuitively that a bigger tower attracts bigger threats, not discover through frustration that their growth was partially self-canceling. If playtesting shows players feeling punished, cap the multiplier at 1.4x (currently uncapped) and increase base boss HP to compensate.

---

## IX. Seed System

### Run seed

- 64-bit integer
- Generated randomly on new game, or entered manually by the player
- Displayed on the run summary screen and in the pause menu
- Determines ALL randomness for the run (map, encounters, loot, events, merchants)

### Deterministic RNG

```rust
pub struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    pub fn new(seed: u64) -> Self { Self { state: seed } }

    pub fn next(&mut self) -> u64 {
        // xorshift64 or similar fast PRNG
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }

    pub fn range(&mut self, min: u32, max: u32) -> u32 {
        min + (self.next() as u32 % (max - min + 1))
    }

    pub fn float(&mut self) -> f32 {
        (self.next() % 10000) as f32 / 10000.0
    }

    /// Fork a new RNG for a subsystem (map gen, encounter gen, loot gen)
    /// so they don't interfere with each other's sequences
    pub fn fork(&mut self, domain: u64) -> Self {
        Self::new(self.next() ^ domain)
    }
}
```

**Forking** is critical: map generation, encounter generation, and loot generation each get their own forked RNG. This means changing the loot table doesn't change the map layout, and vice versa. Subsystems are independent.

### Seed sharing

Players can share seeds for challenge runs: "Try seed 847291 — the Chapter 3 elite is brutal." Same seed + same class + same destination = same experience (minus player skill). This enables community challenges, speedruns, and balance testing.

---

## X. Anti-Frustration Measures

Procedural generation can produce unfair or unfun results. These rules prevent the worst outcomes.

**Guardrail philosophy:** each rule below protects against a specific failure state. They should feel like the world is coherent, not like an invisible hand is correcting the player's experience. The threshold for adding a new guardrail: would its absence produce a run that feels *broken* (not just hard)? If the answer is "it would feel unfair," add it. If the answer is "it would feel challenging," let it stand. Too many guardrails make the world feel curated rather than alive.

**Transparency rule:** guardrails that modify content the player can see (merchant stock, loot drops) should be subtle — weight adjustments, not guarantees. Guardrails that constrain structure (map layout, combat caps) can be strict because the player never sees what was rejected.

| Rule | Why |
|------|-----|
| At least one "breathing room" path per chapter (merchant or rest) | Player always has access to recovery |
| No more than 3 combat nodes in a row on any path | Prevents exhaustion |
| Merchant stock subtly favors resources the player is low on | Prevents feel-bad "nothing I need" shops |
| Mystery events never repeat within a run | Prevents fatigue |
| Ambush events capped at 1 per chapter | Surprise is fun once, not three times |
| First mystery event in a run is always positive | First impression shouldn't be punishment |
| Boss scaling prevents trivialization but is capped | Over-leveled players still feel strong, just not invincible |
| Loot slightly favors weapon types the player hasn't found yet | Encourages build diversity, prevents "all bows all the time" |
| No rams before Chapter 4 | Foundation threat introduced after player has learned all other threats |
| Companion recruitment guaranteed in Chapter 3 | Player won't reach mid-game with an empty roster |
| Max 40 enemies per encounter | Visual clarity + performance ceiling |
| Common weapons stop dropping after Chapter 3 | Late-game drops feel meaningful, not vendor trash |
| Total weapons per run target: 12-16 | Each weapon found feels significant |

---

## XI. Tuning Decisions & Rationale

Surfaced during design review. These are calibration targets — adjust with playtesting.

### Encounter length

| Chapter | Target encounter length | Notes |
|---------|------------------------|-------|
| 1 | 20-30 seconds | Brief. Tutorial-length. Player learns the loop quickly. |
| 2 | 30-45 seconds | First multi-wave. Supply chain has time to matter. |
| 3 | 45-60 seconds | Sustained. 2-3 cache refill cycles happen. |
| 4 | 60-90 seconds | Long. Attrition tests logistics throughput. |
| 5 | 60-90 seconds (regular), 90-120s (boss) | Endgame intensity. |

Encounter length = (enemy count × avg kill time) + spawn stagger + lull time. Not directly controlled — emerges from budget and enemy stats. If encounters run long, reduce budget or increase hero DPS. If too short, increase budget or add waves.

### Lull duration scaling

Lull between waves scales with tower height (taller tower = runner trips take longer = needs more recovery time):

```
lull_duration = 8 + tower_floors * 1 seconds
```

| Tower floors | Lull duration |
|-------------|---------------|
| 3 floors | 11 seconds |
| 5 floors | 13 seconds |
| 8 floors | 16 seconds |

Aggressive encounters (Ch4+ difficulty 3): 30% chance lull shortened by 40%. 10% chance no lull (waves overlap).

### Loot pacing

Target weapons found per run: 12-16 total (loot drops + merchants + guaranteed).

| Source | Weapons per run |
|--------|----------------|
| Enemy loot drops | 4-6 |
| Merchant stock (purchased) | 2-3 |
| Elite encounter guaranteed | 2-3 |
| Boss encounter guaranteed | 4-5 |
| **Total** | **12-16** |

**Rarity curve across the run:**

| Encounters | Rarity pool available |
|------------|----------------------|
| 1-10 | Common (60%), Uncommon (35%), Rare (5%) |
| 11-20 | Common (20%), Uncommon (45%), Rare (30%), Legendary (5%) |
| 21-30 | Uncommon (30%), Rare (50%), Legendary (20%) |
| 30+ | Uncommon (15%), Rare (45%), Legendary (40%) |

Common items naturally phase out. By endgame, every drop is uncommon+. The player never sees a "common shortbow" drop in Chapter 5 — that would feel like a punishment.

**Legendary target: ~4 per run.** Primarily from bosses (5 bosses × 70% = ~3.5). Elites occasionally. Regular enemies almost never. Each legendary should feel like a run-defining moment.

### Map node balance

**Path differentiation rule:** when generating two or more paths from the same node, paths must differ on at least TWO axes:
- Different node types (one has merchant, other has combat)
- Different difficulty (one is D1, other is D2)
- Different terrain (one is plains, other is mountain)
- Different rewards (one has recruitment, other has forge site)

Two paths that are both "D2 combat on plains" is a non-choice. The generator re-rolls one path until they differ.

**Ch4 capped at 10 nodes.** Prevents analysis paralysis. 10 nodes across 5-6 columns = 2 per column average. Readable at a glance.

**Ch5 all combat is difficulty 3.** No easy encounters in the final chapter. You've earned your way here. Rest and merchant nodes are the only relief.

### Boss encounter calibration

Boss HP is calibrated so that:
- A well-built tower kills the boss with ~15-20% ammo reserves remaining (close but comfortable)
- A mediocre tower barely kills the boss (runs out of ammo, fights melee at the end)
- A poorly built tower can't kill the boss before it deals fatal foundation damage

Phase transitions at 66% and 33% HP (3 phases). Each phase lasts 20-40 seconds of sustained fire. Total boss fight: 60-120 seconds.

### Enemy count validation

After budget spending, validate enemy counts:

| Metric | Minimum | Maximum |
|--------|---------|---------|
| Enemies per wave | 3 | 20 |
| Total enemies per encounter | 5 | 40 |
| Catapults per wave | 0 | 2 |
| Rams per encounter | 0 | 1 |
| Siege towers per encounter | 0 | 2 |
| Sappers per wave | 0 | 3 |

If budget math produces counts outside these ranges, clamp and redistribute remaining budget.

### Encounter cleanup

When the last enemy dies:
- Climbers on the tower face: brief death animation (fall off, 0.5s), don't despawn instantly
- Projectiles in flight: complete their arc and land/miss
- Interior raiders: continue their timer (15-20s). Post-combat phase shows this playing out.
- Loot on ground: begins rolling toward tower base. Anything remaining after 5 seconds auto-collects.

### Hero participation requirement

No explicit enforcement. Self-enforcing through math:
- Companions do 20-35% of total required DPS
- If hero does 0%, only 20-35% of enemies die per wave
- Enemies pile up, panels breach, economy collapses
- The game is unwinnable past Chapter 2 without hero shooting

This is verified by the balance tests (simulate AI run with hero doing 0 damage — should fail by mid-Chapter 2).

---

## XII. Balance Config — Exposed Parameters

All procedural generation parameters live in the runtime balance config (JSON/TOML, loaded at startup, no recompile). Organized by system.

### Encounter parameters (balance_config.encounters)

```toml
[encounters]
budget_chapter_base = 10           # points per (chapter - 1)
budget_difficulty_multiplier = 12  # points per difficulty level
# formula: (chapter - 1) * budget_chapter_base + difficulty * budget_difficulty_multiplier
elite_budget_multiplier = 1.5

[encounters.enemy_costs]
grunt = 2
runner = 3
armored = 8
climber = 4
flyer_hoverer = 5
flyer_divebomber = 6
catapult = 12
siege_tower = 15
ram = 20
sapper = 6

[encounters.wave_budget_split]
wave_1_pct = 0.30
wave_2_pct = 0.35
wave_3_pct = 0.35

[encounters.wave_count]
difficulty_1_waves = [1, 1]            # [min, max]
difficulty_2_waves = [1, 2]
difficulty_3_waves = [2, 3]
elite_waves = [2, 3]

[encounters.enemy_caps]
per_wave_min = 3
per_wave_max = 20
per_encounter_max = 40
catapults_per_wave_max = 2
rams_per_encounter_max = 1
siege_towers_per_encounter_max = 2
sappers_per_wave_max = 3

[encounters.lull]
base_seconds = 8
per_floor_seconds = 1
aggressive_shorten_chance = 0.30      # Ch4+ D3
aggressive_shorten_factor = 0.60
no_lull_chance = 0.10                 # Ch4+ D3

[encounters.terrain_weights]
# Multipliers applied to enemy type weights per terrain
forest_climber_bonus = 0.30
forest_flyer_penalty = -0.50
mountain_flyer_bonus = 0.30
swamp_grunt_bonus = 0.30
storm_flyer_weight = 0.0             # grounded
night_runner_bonus = 0.30
```

**Telemetry feedback loop:** if the "panel breach rate per encounter" metric (target: 15-30%) is consistently at 50%+, reduce `budget_difficulty_multiplier` or increase enemy costs. If breach rate is 5%, increase budgets. See [telemetry-balance.md](telemetry-balance.md) §II Combat Metrics.

### Loot parameters (balance_config.loot)

```toml
[loot.drop_chances]
grunt = 0.05
runner = 0.05
armored = 0.15
climber = 0.08
flyer = 0.10
catapult = 0.30
siege_tower = 0.25
ram = 0.50
sapper = 0.20
boss = 1.0

[loot.rarity_weights_by_source]
# [common, uncommon, rare, legendary]
regular_enemy = [0.70, 0.25, 0.05, 0.00]
armored_siege = [0.40, 0.40, 0.18, 0.02]
elite_bonus = [0.00, 0.20, 0.65, 0.15]
boss = [0.00, 0.00, 0.30, 0.70]

[loot.rarity_phase_out]
# Rarity pool shift by encounter range
# Common weight multiplier by encounter number bracket
common_weight_enc_1_10 = 1.0
common_weight_enc_11_20 = 0.33
common_weight_enc_21_30 = 0.0
common_weight_enc_30_plus = 0.0

[loot.resource_drops]
chance_ch1_2 = 0.20
chance_ch3_4 = 0.15
chance_ch5 = 0.10
amount_min = 1
amount_max = 3

[loot.weapons_per_run_target]
from_enemy_drops = [4, 6]
from_merchants_purchased = [2, 3]
from_elite_guaranteed = [2, 3]
from_boss_guaranteed = [4, 5]
legendaries_per_run_target = 4

[loot.modifier_weights]
flaming = 10
frost = 10
explosive = 8
piercing = 8
vampiric = 6
venomous = 6
efficient = 8
gilded = 6
scavenging = 6
silent = 5
beacon = 5
magnetic = 5
cursed = 4          # excluded from legendary pool
heavy = 4
fragile = 3
bloodthirsty = 3
```

**Telemetry feedback loop:** if "weapon type diversity" metric shows 3+ types unused, increase the `loot.base_weights` bias toward unfound types. If "legendary frequency" feels too generous, reduce boss legendary chance. See [telemetry-balance.md](telemetry-balance.md) §VI Class & Build Metrics.

### Map parameters (balance_config.map)

```toml
[map.chapter_structure]
# [columns, min_nodes_per_col, max_nodes_per_col, total_min, total_max]
chapter_1 = [3, 1, 2, 4, 5]
chapter_2 = [4, 2, 3, 6, 8]
chapter_3 = [5, 2, 3, 8, 10]
chapter_4 = [5, 2, 3, 8, 10]     # capped at 10
chapter_5 = [4, 2, 3, 6, 8]

[map.difficulty_distribution]
# [pct_d1, pct_d2, pct_d3] per chapter
chapter_1 = [1.0, 0.0, 0.0]
chapter_2 = [0.7, 0.3, 0.0]
chapter_3 = [0.3, 0.5, 0.2]
chapter_4 = [0.1, 0.4, 0.5]
chapter_5 = [0.0, 0.0, 1.0]      # all D3 — matches prose in §II

[map.path_distance]
short_tick_modifier = -1
medium_tick_modifier = 0
long_tick_modifier = 2
distribution = [0.30, 0.50, 0.20]  # [short, medium, long]

[map.terrain_repeat_chance]
adjacent_same_terrain = 0.70        # geographic coherence

[map.edge_connection]
prefer_vertical_nearby = 0.70
allow_vertical_distant = 0.30
max_combat_chain = 3                # no more than 3 combat in a row
```

**Telemetry feedback loop:** if "route choice diversity" metric shows one path dominant (>40% players take it), adjust node type budgets to make alternative paths more appealing. If "chapter completion rate" is outside target range, adjust difficulty distribution. See [telemetry-balance.md](telemetry-balance.md) §V Map & Journey Metrics.

### Merchant parameters (balance_config.merchant)

```toml
[merchant.stock]
# Weapon count per archetype
arms_dealer_weapons = 3
supplier_weapons = 1
collector_weapons = [1, 2]
traveler_weapons = 2

[merchant.rarity_by_chapter]
# [common, uncommon, rare, legendary]
chapter_1_2 = [0.60, 0.30, 0.10, 0.00]
chapter_3_4 = [0.30, 0.40, 0.25, 0.05]
chapter_5 = [0.10, 0.30, 0.40, 0.20]

[merchant.pricing]
common_base = 30
uncommon_base = 60
rare_base = 120
legendary_base = 250
modifier_value_multiplier = 10
drawback_discount = 15
collector_markup = 1.3
traveler_discount = 0.8

[merchant.materials]
price_wood = 2
price_stone = 3
price_planks = 4
price_bolts = 5
price_mana = 8
quantity_range = [3, 10]
supplier_quantity_multiplier = 2
low_stock_bias = 0.50              # +50% quantity for resources player is low on
```

**Telemetry feedback loop:** if "merchant visit spend" is consistently <20% (players aren't buying), prices are too high or stock is unappealing. If >90%, prices are too low. Target: 40-70%. See [telemetry-balance.md](telemetry-balance.md) §III Economy Metrics.

### Boss parameters (balance_config.bosses)

```toml
[bosses]
hp_floor_scaling = 0.10            # +10% HP per floor above 4
hp_companion_scaling = 0.05        # +5% HP per companion above 2
phase_transitions = [0.66, 0.33]   # HP thresholds for phase changes

[bosses.target_fight_duration]
well_built_seconds = [60, 90]
mediocre_seconds = [90, 120]
# Boss HP calculated backward from target duration × expected DPS
```

**Telemetry feedback loop:** if "boss fight duration" metric is consistently <30s (too easy) or >180s (grind), adjust HP scaling. See [telemetry-balance.md](telemetry-balance.md) §II Combat Metrics.

### Economy parameters (balance_config.economy)

```toml
[economy]
tick_budget_chapter_1 = [3, 4]
tick_budget_chapter_2 = [5, 6]
tick_budget_chapter_3 = [7, 8]
tick_budget_chapter_4 = [9, 10]
tick_budget_chapter_5 = [11, 12]
rest_bonus_ticks = 2
merchant_bonus_ticks = 1
unused_tick_gold_value = 5

[economy.construction_costs]
# [ticks, {material: amount}]
floor_wood = [2, {planks = 3}]
floor_stone = [3, {stone = 5}]
floor_iron = [4, {bolts = 4}]
balcony = [1, {planks = 2}]
cache = [1, {planks = 1}]
dumbwaiter = [1, {planks = 2, bolts = 1}]
chute = [2, {planks = 3}]
cargo_lift = [3, {stone = 3, bolts = 2}]
pneumatic_tube = [3, {mana = 2, bolts = 2}]
repair_panel = [1]  # + matching floor material
rebuild_floor = [3] # + matching floor material
widen_tower = [3, {stone = 4, planks = 3}]
upgrade_foundation = [4]  # + significant leg-type-dependent materials

[economy.recurring_costs]
fletcher_operating = 8
forge_operating = 10
quarry_operating = 6
lumberyard_operating = 6
sawmill_operating = 0     # T2, consumes inputs not gold
alchemist_operating = 0
weaponsmith_operating = 0
enchanter_operating = 0   # T3, same
siege_works_operating = 0
artificer_operating = 0
runner_salary = 5
companion_base_wage = 10
companion_elite_multiplier = 1.5
chicken_legs_maintenance = 5
spider_legs_maintenance = 12
treads_maintenance = 8
hover_maintenance = 20

[economy.income]
kill_bounty_grunt = 2
kill_bounty_runner = 3
kill_bounty_armored = 6
kill_bounty_climber = 3
kill_bounty_flyer = 4
kill_bounty_catapult = 10
kill_bounty_siege_tower = 12
kill_bounty_ram = 15
kill_bounty_sapper = 5
hero_kill_bonus_multiplier = 1.5  # hero kills pay 50% more
encounter_completion_base = 20
encounter_completion_no_breach_bonus = 15
encounter_completion_no_runner_death_bonus = 10
```

**Telemetry feedback loop:** if "gold balance trend" shows persistent decline (economy too tight), increase bounties or reduce operating costs. If persistent surplus (no pressure), increase wages or reduce bounties. Target: income/expense ratio of 1.1-1.3x. See [telemetry-balance.md](telemetry-balance.md) §III Economy Metrics.

---

## XIII. Telemetry → Balance Config Feedback Loop

The procedural generation system and the telemetry system form a closed loop:

```
Balance Config (parameters)
    ↓
Procedural Generation (produces encounters, loot, maps)
    ↓
Player Experience (gameplay)
    ↓
Telemetry (measures outcomes against targets)
    ↓
Developer reads telemetry dashboard
    ↓
Developer adjusts Balance Config
    ↓
(loop repeats)
```

**Key metric → parameter mappings:**

| Telemetry metric | Target | If out of range, adjust | Config key |
|-----------------|--------|------------------------|------------|
| Panel breach rate | 15-30% | Enemy budget, enemy HP, panel HP | encounters.budget_*, enemy stat tables |
| Hero kill % | 50-70% | Companion accuracy, companion damage | companion base stats |
| Grunt survival time | 1-3s | Grunt HP, hero damage | enemy_stats.grunt_hp |
| Rack empty duration | <10% encounter | Production rates, runner speed, cache size | economy.production_rates, transport speeds |
| Gold income/expense ratio | 1.1-1.3x | Kill bounties, operating costs, wages | economy.income.*, economy.recurring_costs.* |
| Tick utilization | 70-85% | Tick budgets, construction costs | economy.tick_budget_*, economy.construction_costs.* |
| Chapter completion rates | 90/70/50/35/25% | Difficulty distribution, enemy budgets | map.difficulty_distribution, encounters.budget_* |
| Weapons per run | 12-16 | Drop chances, merchant stock | loot.drop_chances.*, merchant.stock.* |
| Route diversity | No path >40% | Node type budgets, path differentiation | map.node_type_budgets.* |
| Boss fight duration | 60-120s | Boss HP scaling | bosses.hp_*_scaling |

Every parameter in the balance config has a corresponding telemetry metric that validates it. No parameter exists without a way to measure whether it's correct.
