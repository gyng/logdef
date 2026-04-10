Project: SUPPLY LINE | Game Telemetry & Balance Metrics

> **Status:** v1 and post-v1 vision. Check [v1-scope.md](v1-scope.md) before implementing any feature described here.

## I. Purpose

This document defines what the game measures about player behavior and performance, what the target values are, and how the data informs balance decisions.

**Three distinct systems, clearly separated:**

1. **Developer telemetry** (Sections II-VI, VIII-IX) — metrics the team reads to decide if systems are working. These drive balance config changes between releases. Never visible to players.
2. **Player suggestion system** (Section VII) — contextual tips delivered through companion dialogue. Shown to players, never mandatory, purely informational.
3. **Pacing adaptation** (Section X) — the game selects from within the existing encounter/event pools based on recent performance. This is NOT difficulty adjustment: the encounter pool, economy numbers, and balance tables remain constant. But it IS adaptation — the game chooses gentler or harder compositions from the same difficulty tier. We call this what it is: **pacing adaptation**, not "the game never adjusts anything." The player's skill is still the only path to victory; pacing adaptation ensures the *sequence* of challenges teaches effectively and maintains tension.

For developers: these metrics tell us if systems are working. For players: the suggestion system translates metrics into actionable advice. Pacing adaptation is invisible to both — it's a design tool, not a feature.

---

## II. Combat Metrics

### Hero performance

| Metric | Target range | If too low | If too high |
|--------|-------------|-----------|------------|
| Hero kill % | 50-70% | Hero weapons too weak, companions too strong. Hero doesn't feel like the star. | Companions feel useless. Reduce hero damage or buff companion accuracy. |
| Hero accuracy (bow) | 55-75% | Arcs too punishing, targets too small, or aim assist (Precision stat) too weak at low levels. | Too easy to hit. Increase enemy speed or reduce aim assist. |
| Hero accuracy (crossbow) | 70-85% | Bolt speed too slow or enemies too evasive. | Expected — crossbow is the forgiving weapon. |
| Hero accuracy (staff) | 85-95% | Homing/straight line not working. | Expected — staff is the easy-aim option. Cost is in mana logistics. |
| Ammo consumed per encounter | Varies by chapter | If way below expectation: encounters too short or hero too efficient. | If way above: encounters too long, hero inaccurate, or fire rate too high for the supply chain. |
| Weapon ability usage rate | 60-80% of available uses | Ability too weak or cooldown too long. Players forget it exists. | On cooldown every time = either the ability is core (fine) or the cooldown is too short (too spammable). |
| Hero skill pick distribution | ~25% each (4 skills) | Unpicked skills are too weak or too situational. Buff or redesign. | One skill dominates = too generically strong. Nerf or make alternatives more appealing. |

### Companion performance

| Metric | Target range | Meaning |
|--------|-------------|---------|
| Companion kill % (total) | 20-35% | They're support, not stars. |
| Companion accuracy (average) | 40-70% (scales with experience) | New recruits miss a lot (40%). Veterans are reliable (70%). |
| Companion ammo waste (misses × ammo cost) | < 20% of total ammo consumption | If companions waste too much ammo, players feel the supply chain is feeding bad AI. |
| Shield-bearer climber blocks per encounter | 3-8 | If 0: climbers aren't pathing through the shield-bearer. If 15+: too many climbers, or shield-bearer is doing too much of the defensive work. |

### Enemy metrics

| Metric | Target | Meaning |
|--------|--------|---------|
| Grunt survival time | 1-3 seconds from first shot | They're chaff. Should die fast. |
| Armored survival time | 8-15 seconds | Ammo sinks. Should feel tanky. |
| Climber reach-rate (% that reach a position) | 20-40% | 60%+ means ranged DPS is too low or climbing is too fast. <10% means climbers are irrelevant. |
| Flyer kill time | 3-6 seconds | Harder than grunts but not as tanky as armored. |
| Sapper reach-rate (% that reach infrastructure) | 10-20% | Sappers should mostly die but occasionally succeed. If 50%+ reach, they're too fast or too tanky. |
| Ram reach-rate | < 5% | Rams reaching the foundation should be rare. If 20%+, they're too fast or the game isn't communicating priority. |
| Panel breach rate per encounter | 15-30% of encounters (Ch3+) | Breaches should be uncommon but real. If 60%, walls are too weak. If 5%, climbers/siege are irrelevant. |

---

## III. Economy Metrics

### Gold flow

| Metric | Target | Notes |
|--------|--------|-------|
| Gold balance trend | Gradual accumulation with dips | Player should slowly get richer, with dips after big purchases or bad encounters. Flat = too tight. Exponential growth = too loose. |
| Income vs. expenses ratio | 1.1-1.3x (income slightly exceeds costs) | Player should be able to afford operations + slow savings. <1.0 = death spiral. >2.0 = no pressure. |
| Gold at run end (victory) | Moderate surplus (50-100g) | Player shouldn't end rich (economy too loose) or broke (economy too tight). |
| Merchant visit spend | 40-70% of available gold | If players spend 100%, prices are too low or items too good. If 10%, stock is unappetizing or prices too high. |

### Tick economy

| Metric | Target | Notes |
|--------|--------|-------|
| Tick utilization per stop | 70-85% | Below 50% = too many ticks. Above 95% = too few ticks, no room for decisiveness. |
| Unused ticks → gold conversion | 15-30% of stops | Players should sometimes bank ticks. If never banking, ticks are too scarce. If always banking, ticks are too generous. |
| Most common tick spend | Build > Repair > Upgrade | Building should be the exciting spend. If repair dominates, damage is too frequent. If upgrade dominates, building options are exhausted too early. |

### Material economy

| Metric | Target | Notes |
|--------|--------|-------|
| Warehouse utilization | 40-70% full on average | If always full (80%+), production exceeds consumption — player is overbuilding production. If always empty (<20%), production can't keep up. |
| Material diversion rate | 10-20% of production going to construction | Most production should feed combat (ammo). Construction should feel like a real diversion, not free. |
| Time to Tier 2 | Encounters 10-14 | First Tier 2 building. If earlier, progression is too fast. If later, T2 feels gated too long. |
| Time to Tier 3 | Encounters 22-28 | Same principle. |

---

## IV. Logistics Metrics

### Runner efficiency

| Metric | Target | Notes |
|--------|--------|-------|
| Runner utilization (% time carrying) | 50-70% | Below 30% = too many runners or too little production. Above 85% = runners are overwhelmed, queuing constantly. |
| Runner queue time (% time waiting in queue) | < 15% | If 30%+, congestion is severe. Player needs better transport infrastructure. |
| Average delivery time (warehouse → rack) | 8-15 seconds | If 5s: transport is so fast it's trivial. If 25s+: racks run dry regularly, supply chain is too slow. |

### Cache / rack performance

| Metric | Target | Notes |
|--------|--------|-------|
| Hero rack empty duration per encounter | < 10% of encounter time | If 20%+: supply chain can't sustain hero's fire rate. Player should feel pressure but not constant ammo starvation. |
| Cache empty duration | < 20% of encounter time | Caches are buffers — they should smooth demand spikes. If empty 40%+, cache is too small or runner delivery is too slow. |
| Rack restock count per encounter | 3-6 | Number of crate deliveries to the hero's rack per encounter. If 1-2: hero barely shoots. If 10+: hero sprays ammo. |

### Transport infrastructure

| Metric | Target | Notes |
|--------|--------|-------|
| Transport type distribution | Varies but should see diversity | If 90% of builds are stairs + lifts and nobody builds chutes/dumbwaiters, the alternatives aren't compelling enough. |
| Lift car utilization | 50-80% | If all lifts run 1 car, car count isn't a meaningful decision. If everyone runs 3 cars, 1-2 cars are never viable. |
| Batch vs. immediate mode split | ~50/50 | Both should be viable for different situations. If 90% use one mode, the other isn't compelling. |

---

## V. Map & Journey Metrics

| Metric | Target | Notes |
|--------|--------|-------|
| Route choice diversity | No single path taken by >40% of players | If one path dominates, it's a superior strategy. Buff alternatives. |
| Node type preference (when choosing) | Rough parity across combat/merchant/rest/elite | If everyone avoids elites: risk isn't worth the reward. If everyone avoids rest: rest stops aren't valuable enough. |
| Chapter completion rate (Ch1) | >90% | Tutorial chapter. Almost everyone should survive. |
| Chapter completion rate (Ch2) | 70-80% | First real challenge. |
| Chapter completion rate (Ch3) | 50-60% | Mid-game filter. |
| Chapter completion rate (Ch4) | 30-40% | Hard. Reaching this is an achievement. |
| Chapter completion rate (Ch5) | 20-30% | Final chapter. Completion is victory. Not everyone should win. |
| Game over cause: foundation destroyed | 30-40% of game overs | Should be the dramatic, memorable way to lose. |
| Game over cause: voluntary abandon | 40-50% of game overs | Player recognizes the run is doomed. This is fine — knowing when to quit is a skill. |
| Game over cause: economic collapse | 10-20% of game overs | Can't afford operations. Should be uncommon but real. |
| Average run length (encounters) | 20-25 | A run that doesn't complete should still be a satisfying session. |

---

## VI. Class & Build Metrics

| Metric | Target | Notes |
|--------|--------|-------|
| Class pick rate | ~25% each | All classes should be appealing. If one is 5%, it's either too weak or too niche. |
| Class win rate | Within 10% of each other | A 50% win rate class vs. 15% is broken. Small differences are fine (some classes are harder). |
| Perk pick diversity | No single perk taken by >50% of players of any class | Perks should be build-dependent, not universally optimal. |
| Stat allocation patterns by class | Correlated with class identity | Archers SHOULD pump Precision. If they pump Tempo instead, the class design isn't guiding correctly. |
| Weapon type diversity | All 9 base types used | If 3 weapon types are never used, they're not compelling. |
| Melee build viability | Win rate within 15% of ranged builds | Melee is a deliberately restrictive choice. It should be viable but harder, not impossible. |

**Segmentation note:** the targets above are population-level dashboards, not balancing mandates. Many of them (hero kill %, class win rate, merchant spend, chapter completion) will look different when segmented by player skill cohort (runs completed), class, or progression state. Before adjusting balance based on any metric:

1. **Segment first.** A 35% overall chapter 3 completion rate might be 80% for experienced players and 15% for first-timers. Those require different interventions (or none — first-timer attrition is expected).
2. **Asymmetry is intentional.** Archer and Commander are *supposed* to play differently. 25% pick rate is a health signal, not a target to force. If Commander has 15% pick rate and 45% win rate, it's a niche class that works — not a problem.
3. **Dashboard, not governor.** These metrics trigger investigation, not automatic balance changes. A metric outside its range means "look at this," not "change the config."

---

## VII. Player Feedback System — Suggestions

The game provides contextual feedback to help players improve. Suggestions are informational only — they never change game state, encounter composition, or economy values.

### Post-combat tips (shown in PostCombatSummary)

Triggered by specific metric thresholds from the encounter just completed:

| Trigger condition | Suggestion |
|-------------------|-----------|
| Hero rack empty > 15% of encounter | "Your ammo ran low. Consider a second fletcher or a closer cache." |
| Hero accuracy < 40% | "Many shots missed. Try slowing your fire rate, or switch to a crossbow for easier aim." |
| Companion on wrong orders (e.g., air focus with no flyers) | Companion says: "I was watching the skies but nothing flew. Maybe ground focus next time?" |
| Panel breach occurred | "Floor [N] was breached. Consider a balcony on an adjacent floor, or repair the panel before the next fight." |
| Runners queued > 20% of encounter | "Your runners were stuck in traffic. A second staircase or a lift would speed things up." |
| Cache empty > 30% of encounter | "Your cache on floor [N] couldn't keep up. Upgrade it or add a faster transport link." |
| Gold trending negative for 3+ encounters | "You're spending more than you're earning. Consider dismissing a companion or reducing operating costs." |
| Ticks fully spent 3+ stops in a row | "You're using every tick. That's fine, but banking a few ticks earns bonus gold." |
| Weapon ability unused for entire encounter | "You didn't use [Charged Shot] this fight. It costs ammo but deals massive damage — try it against armored enemies." |
| No damage taken (flawless) | Companion says: "Not a scratch. I could get used to this." (Positive reinforcement.) |

### Companion hint system (shown during prep)

Companions comment based on upcoming encounter info + recent performance:

| Context | Hint |
|---------|------|
| Next encounter has flyers | "I hear there are flyers ahead. Someone should be on air focus." |
| Next encounter is elite | "Tough fight coming. Make sure racks are full before we march." |
| Floor was breached last fight, unrepaired | "[Companion name]: That hole in floor 4 is still open. We should fix it before something crawls in." |
| Player has unused gold + merchant available | "There's a merchant ahead. We've got gold to spend." |
| Player hasn't built a lift yet (past Ch2) | "These stairs are killing my legs. Ever think about installing a lift?" |
| Companion accuracy has improved significantly | "[Name]: I'm getting better at this. Barely miss now." (Positive, tracks growth.) |

### Suggestion prioritization

A single bad encounter can trigger 4-5 suggestions simultaneously (ammo ran low + accuracy was bad + panel breached + runners queued + gold trending down). Showing all of them is noise, not help.

**Priority rules:**
1. **Max 2 suggestions per post-combat screen.** One combat tip, one logistics tip. If multiple triggers fire, pick the highest-severity one per category.
2. **Severity ranking:** breach > rack empty > runner congestion > accuracy > economy > unused ability. (Closer to "you almost died" = higher priority.)
3. **Root cause over symptom.** If runners queued AND rack was empty, the root cause is transport — show the transport tip, not the rack tip.
4. **Companion hints during prep: max 2.** One about the upcoming encounter, one about the tower state. Don't stack 4 observations.
5. **Cooldown per tip category:** a tip about the same system (e.g., transport) doesn't repeat for 5 encounters even if the condition persists. The player heard it.

### What suggestions DON'T do

- Never tell the player the "right" answer. "Consider a second fletcher" not "build a second fletcher on floor 4."
- Never pressure the player. Suggestions are always ignorable. No "you MUST do this" energy.
- Never repeat. A tip shown once doesn't repeat for 5 encounters. The player heard it.
- Never suggest during combat. All suggestions are prep-phase or post-combat only.

---

## VIII. Developer Balance Dashboard

For the developer (not shown to players). Aggregated across all playtests/runs:

### Run overview
- Win rate by class, by chapter, by destination
- Average run length (encounters)
- Game over cause distribution (pie chart)
- Gold balance curve (average across runs, with min/max bands)

### Per-encounter breakdown
- Kill % distribution (hero vs. companion)
- Breach rate by encounter number (should increase through the journey)
- Ammo consumption by weapon type
- Average rack empty time by encounter number

### Economy health
- Gold balance histogram at each chapter transition
- Tick utilization histogram
- Material stockpile curves over a run
- Construction timing (when are buildings/transport/balconies built)

### Logistics performance
- Runner utilization distribution
- Transport type adoption curve (when are lifts/chutes first built)
- Cache empty duration trends

### Map analytics
- Path choice heatmap per chapter
- Node visit frequency
- Elite attempt rate vs. elite completion rate

### Companion analytics
- Recruitment frequency per companion
- Companion displacement rate (breached at their position)
- Companion accuracy progression curves

### Alert thresholds (flags for developer attention)
- Any class win rate diverges >15% from average → balance issue
- Any weapon type used by <5% of players → design issue
- Any chapter completion rate outside target range → difficulty curve issue
- Any node type consistently avoided (>70% of players skip when available) → reward/risk issue
- Any perk taken by >60% of a class's players → perk is too universally strong

---

## IX. Data Collection

### What's collected per run (stored with save)

```rust
#[derive(Serialize, Deserialize)]
pub struct RunTelemetry {
    pub class: HeroClass,
    pub destination: DestinationId,
    pub final_chapter: usize,
    pub encounters_completed: usize,
    pub outcome: RunOutcome,  // Victory | Abandon | FoundationDestroyed | EconomicCollapse
    pub total_gold_earned: u32,
    pub total_gold_spent: u32,
    pub final_gold: u32,
    pub per_encounter: Vec<EncounterTelemetry>,
    pub perks_chosen: Vec<Perk>,
    pub stat_allocation: HeroStats,
    pub final_tower_floors: usize,
    pub companions_recruited: Vec<CompanionId>,
    pub companions_dismissed: Vec<CompanionId>,
    pub route_taken: Vec<NodeId>,
    pub route_alternatives_available: Vec<Vec<NodeId>>,  // what paths were available at each choice point
    pub session_duration_seconds: f32,
    pub session_count: u32,                              // how many play sessions this run spanned
    pub quit_encounter: Option<usize>,                   // which encounter the player quit at (if abandoned)
}

#[derive(Serialize, Deserialize)]
pub struct EncounterTelemetry {
    pub encounter_number: usize,
    pub node_type: NodeType,
    pub terrain: Option<TerrainType>,
    pub duration_seconds: f32,
    pub hero_kills: u32,
    pub companion_kills: u32,
    pub hero_accuracy: f32,
    pub ammo_consumed: HashMap<ResourceType, u32>,
    pub rack_empty_seconds: f32,
    pub cache_empty_seconds: f32,
    pub runner_queue_seconds: f32,
    pub panels_breached: Vec<usize>,
    pub foundation_damage_taken: f32,
    pub gold_earned: u32,
    pub ticks_spent: u32,
    pub ticks_available: u32,
    pub buildings_built: Vec<BuildingType>,
    pub transport_built: Vec<TransportType>,
    pub weapon_ability_uses: u32,
    pub hero_skill_uses: u32,
    pub loot_found: Vec<ItemId>,

    // Combat feel metrics
    pub inputs_per_second_avg: f32,        // hero fire rate + ability use rate
    pub inputs_per_second_stddev: f32,     // variance (steady = flow, spiky = panic)
    pub closest_call_panel_hp_pct: f32,    // lowest panel HP reached (0-1)
    pub comeback_from_danger: bool,        // recovered from 2+ critical panels or rack empty >10s

    // Logistics detail
    pub transport_utilization: HashMap<TransportType, f32>,  // % time active per transport type
    pub runner_trips_completed: u32,
    pub runner_trips_per_transport: HashMap<TransportType, u32>,

    // Economy feel
    pub gold_at_encounter_start: u32,
    pub near_miss_purchases: u32,          // items browsed but <10g short of affording

    // Pacing
    pub prep_duration_seconds: f32,        // time from post-combat "Continue" to "March"
    pub encounters_since_last_breach: u32, // recovery tracking after bad encounters
    pub hero_accuracy_trend: f32,          // slope of accuracy over last 5 encounters
}
```

### Privacy

- All telemetry is LOCAL by default. Stored with the save file.
- Optional opt-in anonymous upload for aggregate balance data.
- No personal information. No tracking. Just game state numbers.
- Developer dashboard reads aggregated anonymous data only.

### When is telemetry analyzed?

- **Post-combat:** EncounterTelemetry is generated, checked against suggestion thresholds, tips shown if triggered.
- **End of run:** RunTelemetry is finalized and stored.
- **Developer dashboard:** reads aggregated data from opt-in uploads or local test runs.

---

## X. Cognitive Science Heuristics — Is the Game Fun?

Fun isn't one thing. It's several psychological states that the game needs to produce in the right sequence and proportion.

**Epistemic status:** the heuristics below are *hypotheses*, not established measurements. Telemetry can observe behavior (inputs per second, session length, prep time) but cannot reliably infer internal states (flow, dread, boredom). "Long prep time" might mean dread, careful planning, or the player went to get coffee. Each signal below is a starting point for investigation — a reason to watch a replay or run a playtest interview, not a confident conclusion. Treat these as lenses to look through, not dashboards to trust at face value.

### Flow state (Csikszentmihalyi) — challenge matches skill

**Signal:** steady input rate during combat + quick march timing in prep + high tick utilization + long sessions.

**Anti-signal:** erratic inputs (panic/boredom) + long prep times (dread) + low utilization (nothing to do) + short sessions (frustration or disinterest).

**Measurable:**
- Inputs per second during combat: steady = flow, spiking = anxiety, declining = boredom
- Time between encounter end and "March" click: <30s with high tick use = eager flow. >2min = hesitation/dread
- Session length: >30min = engaged. <15min = something's wrong

### Competence (Self-Determination Theory) — feeling improvement

**Signal:** accuracy, rack management, and breach rate all improve over the run even as difficulty increases.

**Measurable:**
- Hero accuracy slope: should trend upward across encounters
- Rack empty time slope: should trend downward
- Breach rate relative to difficulty: breaches ÷ encounter difficulty rating should decrease
- Prep speed: should increase (player learns what to build faster)

**Heuristic:** if all improvement curves are flat for 5+ encounters, the player has stopped learning. Either the skill ceiling is reached (fine for late game) or the game isn't teaching effectively (problem in early-mid game).

### Autonomy — choices feel real

**Signal:** high build diversity across runs + frequent configuration changes within runs.

**Measurable:**
- Tower layout variance between runs by same player (how different are builds?)
- Order/cache reconfiguration frequency per stop (are players adapting or set-and-forget?)
- Stat/perk diversity within a class (multiple viable builds, not one dominant path)

**Heuristic:** if build diversity across 5+ runs is <20% variance, there's a dominant strategy. Something needs rebalancing to open alternatives.

### Optimal arousal (Yerkes-Dodson) — the sweet spot of tension

**Signal:** moderate win margins, resource oscillation, occasional comebacks.

**Measurable:**
- Closest call per encounter: lowest panel HP reached, longest rack empty streak
- Comeback frequency: % of encounters where player recovered from "danger state" (2+ panels critical or rack empty >10s)
- Resource oscillation: rack level variance during combat (high variance = healthy tension, flat = no tension)
- Panel HP oscillation across the run: should cycle up (repairs) and down (damage), not monotonically decrease

**Target ranges:**
- Closest call panel HP: 20-40% range (came close but pulled through)
- Comeback frequency: 15-25% of encounters should involve a comeback moment
- Rack level variance: standard deviation > 0.5 crates (it's going up and down, not sitting at max or min)

**Pacing adaptation (arousal):** if closest calls are consistently >80% (too comfortable) for 3+ encounters, the encounter generator biases toward the higher end of the difficulty tier's composition range. If consistently <10% (too desperate) for 3+ encounters, it biases toward the lower end. Both compositions are valid outputs of the same encounter pool and difficulty tier — no rules are broken, no budget is changed. The adaptation shifts *where within the range* the generator lands, not the range itself.

### Learning rate — new information at the right pace

**Signal:** new mechanics are mastered in 3-5 encounters after introduction.

**Measurable:**
- Time from first encounter with a new enemy type to the first encounter where that type is handled without breach/damage
- Time from first lift installation to effective lift utilization (>50% car usage)
- Deaths within 2 encounters of a new mechanic introduction

**Pacing adaptation (learning):** the encounter AFTER a new mechanic introduction biases toward simpler compositions (fewer enemy types, more familiar chaff) to give learning space. This is not a player-specific adjustment — it's a structural rule: "introduction encounters are followed by breathing room." The encounter difficulty tier doesn't change.

**Developer heuristic:** if >20% of players die within 2 encounters of a new mechanic, the introduction encounter's budget or enemy mix needs review (a balance config change, not a runtime adaptation).

### Loss aversion (Kahneman/Tversky) — losses hurt more than gains feel good

**Signal:** quit rate after losses, repair behavior, emotional momentum.

**Measurable:**
- Quit rate after first breach: should be <10%. If >25%, first breach punishment is too harsh.
- Quit rate after 2+ breaches in one encounter: acceptable if higher (~20%).
- Repair priority: which floors are repaired first? (Tells you what players VALUE most.)
- Run continuation after bad encounters: does the player "tilt" (play worse for 2-3 encounters) or recover?

**Heuristic:** the first breach should happen in Chapter 2. Too early (Ch1) = player hasn't invested enough to learn from the loss. Too late (Ch3+) = player is blindsided by a mechanic they should have encountered earlier.

### Incompleteness motivation (Zeigarnik effect) — "one more turn"

**Signal:** session-ending behavior, near-miss frequency.

**Measurable:**
- Where in the journey do players stop playing? After chapter completion (clean exit) vs. mid-chapter (hooked, will return)?
- Near-miss frequency: how often is the player 1-2 ticks or 5-10 gold short of something they want?
- Return rate: % of players who start a new run within 24h of ending one.

**Heuristic:** if most players stop at chapter boundaries, the game needs more mid-chapter hooks (upcoming companion visible on map, "almost enough for that lift" moments). If most stop mid-session, the game's natural stopping points are well-placed.

### Composite fun signal

A weighted heuristic combining all signals. Not a single number — a dashboard of indicators:

```
FLOW:       [████████░░] 78%  — steady input, good sessions
COMPETENCE: [██████░░░░] 62%  — improving but plateau on logistics
AUTONOMY:   [███████░░░] 72%  — good build variety, orders could vary more
AROUSAL:    [█████████░] 88%  — great tension, close calls frequent
LEARNING:   [███████░░░] 74%  — new mechanics learned in 3-4 encounters
RETENTION:  [████████░░] 82%  — long sessions, high return rate
```

**What pacing adaptation looks like in practice:**

| Signal | Adaptation type | What changes |
|--------|----------------|-------------|
| Competence low | **Suggestion** (player-visible) | More explicit companion tips. "Try leading your shots." |
| Arousal too low | **Suggestion** (player-visible) | Companions suggest harder paths. "The elite path has better loot." |
| Arousal too high | **Suggestion** (player-visible) | Companions suggest rest stops. "We should rest." |
| Arousal sustained low | **Composition bias** (invisible) | Generator picks from higher end of difficulty tier's range |
| Arousal sustained high | **Composition bias** (invisible) | Generator picks from lower end of difficulty tier's range |
| Autonomy low | **Event selection** (invisible) | Mystery events weighted toward build-altering options |
| Learning overloaded | **Structural rule** (invisible) | Post-introduction encounters use simpler compositions |
| Flow breaking | **Presentation** (invisible) | Dim non-essential UI, make critical info more prominent |

**What never changes at runtime:** encounter difficulty tier, threat budget formula, economy values, drop rates, boss HP, enemy stats. The balance config is constant within a run. Pacing adaptation selects *where within the existing range* the generator lands and *what the game says to the player*. It does not create encounters that couldn't have appeared with a neutral seed.
