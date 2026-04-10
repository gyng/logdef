Project: SUPPLY LINE | Class & Companion Design

> **Status:** v1 and post-v1 vision. Check [v1-scope.md](v1-scope.md) before implementing any feature described here.

## I. Hero Classes — Permanent Identity

Each class has an **exclusive mechanic** — a verb that only this class can do. It shapes combat feel, logistics relationship, and build decisions all run. Classes are not just starting conditions; they define how you play from start to finish.

**Scope note:** this document covers class design, companion roster (exterior + interior), and the relationship system. As the companion layer has grown, this file is carrying a lot of weight. When implementation begins, consider splitting into: `class-design.md` (hero classes only), `companion-roster.md` (exterior + interior companions, passives, scaling), and `companion-relationships.md` (affinity, bonuses, dialogue, paired endings). For now, keeping them together preserves the cross-references between class synergies and companion composition.

**Design maturity note:** specific numbers throughout (Focus 2x damage, Rally +30% fire rate, Scrap 40% drop rate, Scavenger "30-40% supply chain reduction") are design intent, not tuned results. All are exposed in the balance config and will shift during playtesting. Where this doc says "highest" or "safest," read "intended to be" — those claims need validation.

### The Archer — "Focus"

**Identity:** patience pays. The precision shooter who makes every shot count.

**Starting stats:** high Precision, moderate Draw. Low Grit, low Salvage.
**Starting weapon:** common shortbow.
**Class perk (always active):** Eagle Eye — critical hit zones on enemies are 50% larger.

**Exclusive mechanic: Focus mode.**

After not firing for 2 seconds, the Archer enters Focus mode. Visual: subtle glow on the hero, crosshair tightens, the world quiets slightly.

| Focus state | Effect |
|-------------|--------|
| Not in Focus | Normal combat. Aim, shoot, standard damage. |
| Focus active (2s no-fire) | Next shot: 2x damage, perfect accuracy (zero aim variation), guaranteed critical if hitting the weak spot. |
| Focus consumed | One Focus shot fired. 2-second timer restarts. |
| Focus broken | Any panel damage at your floor breaks Focus. Must re-wait 2s. |

**Why Focus shapes the whole run:**
- The Archer naturally gravitates toward slow, high-damage weapons (longbow, heavy crossbow, musket). Fast weapons waste Focus because you're never still long enough.
- High positions are ideal — more time between threats, more opportunities to Focus.
- The supply chain is barely stressed — the Archer fires infrequently but devastatingly. A single fletcher and small cache suffices.
- Focus rewards reading the battlefield — choosing WHEN to shoot and WHAT to shoot. Not reaction speed, not fire rate. Judgment.

**Focus perk amplifications (class-specific perks in the perk tree):**
- "Deep Focus" — Focus activates after 1.5s instead of 2s.
- "Unwavering" — Panel damage no longer breaks Focus.
- "Focus Chain" — If a Focus shot kills, the next shot is also a Focus shot (no timer needed). Chaining kills = sustained Focus.
- "Overwatch" — While in Focus, enemies approaching your floor are highlighted with a damage multiplier indicator. Information advantage.

### The Engineer — "Overclock"

**Identity:** the tower is the weapon. Building and boosting the machine matters more than personal DPS.

**Starting stats:** high Grit, moderate Salvage. Low Tempo, low Draw.
**Starting weapon:** common hand crossbow.
**Class perk (always active):** Blueprint Master — all construction costs 1 fewer tick (minimum 1).

**Exclusive mechanic: Overclock.**

Once per encounter (cooldown resets between encounters), the Engineer activates Overclock on one building or transport system visible in the tower cross-section. The target runs at 200% speed for 20 seconds.

| Overclock target | Effect |
|-----------------|--------|
| Production building | Produces at 2x rate for 20s. Fletcher outputs twice as fast. |
| Runner quarters | Runners from this quarters move at 2x speed for 20s. |
| Cargo lift | Lift moves at 2x speed for 20s. |
| Depot cache | Cache capacity temporarily doubles for 20s (holds more buffer). |

**Activation:** the Engineer presses a dedicated key (replaces the weapon ability key Q — Engineers use Q for Overclock instead of weapon ability). A targeting cursor appears on the tower cross-section (the interior, not the battlefield). Click the building or transport to Overclock. Brief activation animation (gears spin, steam vents, glow pulse on the target).

**Why Overclock shapes the whole run:**
- The Engineer thinks about the tower during combat, not just the battlefield. "Which system is the bottleneck RIGHT NOW?" is the core decision.
- Overclock timing matters — use it at the start when racks are full (wasted), or save it for wave 2 when caches are draining (maximum impact)?
- The Engineer builds a tower that's slightly under-capacity, knowing they can Overclock the bottleneck when it matters. This means a leaner, cheaper tower with strategic bursts of throughput.
- Personal DPS is lower (low Draw, low Tempo). The Engineer wins through tower efficiency, not gunplay.

**Overclock perk amplifications:**
- "Double Shift" — Overclock can be used twice per encounter.
- "Cascade" — Overclocking a building also Overclocks all transport connected to it.
- "Permanent Tuning" — Buildings you've Overclocked 3+ times in a run gain a permanent +10% speed bonus.
- "Emergency Repairs" — Overclock can target a damaged wall panel, instantly restoring 20% HP. Defensive Overclock.

### The Commander — "Rally"

**Identity:** the team is the weapon. Companions fight harder, smarter, and cheaper near the Commander.

**Starting stats:** moderate across all. No extremes.
**Starting weapon:** common staff (easy aim — Commander focuses on battlefield reading, not personal precision).
**Class perk (always active):** Tactician — companion accuracy grows 50% faster from combat experience. Extra targeting order: "synchronized" (fire when Commander fires, match target priority).

**Exclusive mechanic: Rally aura.**

Companions within 2 floors of the Commander receive significant bonuses (stronger than any perk or trinket could provide):

| Rally bonus | Effect |
|-------------|--------|
| Fire rate | +30% (companions shoot noticeably faster) |
| Accuracy | +20% (fewer misses = less ammo wasted) |
| Ammo efficiency | -15% ammo consumption (each shot costs less from the rack) |

**Rally is passive and always active.** No button to press, no cooldown. It's an aura — the Commander's presence IS the ability. The Commander's combat decision is POSITIONING: which floor to stand on determines which companions benefit.

**Why Rally shapes the whole run:**
- The Commander wants companions clustered near them. This means stacking 3-4 balconies on adjacent floors with the Commander in the middle. The "kill zone" is a concentrated section of the tower.
- This clustering creates logistics concentration — 4 people drawing ammo from a small section of the tower. The caches on those floors drain fast. The Commander needs a robust supply chain to that cluster.
- The Commander personally does moderate damage (staff is easy aim, not high DPS). But the companion output in the Rally zone is massive — effectively 4 companions firing at 130% rate with 120% accuracy. The total DPS of the zone is higher than any other class.
- The Commander wants high-accuracy veteran companions (Rally amplifies accuracy — a 70% accuracy companion at +20% is 90%, nearly perfect).

**Rally perk amplifications:**
- "Expanded Formation" — Rally extends to 3 floors instead of 2.
- "Shared Bounty" — Gold earned from companion kills within Rally counts as Commander kills (50% bonus).
- "Battle Hymn" — When the Commander uses a hero skill, all Rallied companions get a brief 100% fire rate burst (2 seconds). Synchronizes skill timing with team burst.
- "Field Promotion" — Companions who stay in Rally for an entire encounter gain +5% permanent accuracy bonus (stacks up to +15%). Long-term loyalty reward.

### The Scavenger — "Scrounging"

**Identity:** chaotic, self-sufficient, rich. Bypasses the supply chain through sheer aggression.

**Starting stats:** high Salvage, high Tempo. Low Precision, low Grit.
**Starting weapon:** common throwing knives.
**Class perk (always active):** Vulture — loot rolls to tower base 2x faster, enemies have 2x resource drop rate.

**Exclusive mechanic: Scrounging.**

When enemies die, they have a chance to drop **scrap ammo** — irregular arrows, bent bolts, broken crystals. Scrap ammo goes directly into the Scavenger's rack, bypassing the entire supply chain (no production, no runner, no cache).

| Scrap property | Value |
|----------------|-------|
| Drop chance per kill (hero kills) | 40% |
| Drop chance per kill (companion kills) | 15% |
| Damage vs. normal ammo | 70% |
| Ammo type | Matches current weapon (scrap arrows for bow, scrap bolts for crossbow, etc.) |
| Goes to | Scavenger's rack directly (no transport needed) |

**Why Scrounging shapes the whole run:**
- The Scavenger partially self-supplies. With 40% scrap drop rate on hero kills and high Tempo (fast fire rate), the Scavenger generates a steady stream of free ammo. Not enough to fight entirely on scrap — but enough to reduce supply chain dependency by 30-40%.
- This means the Scavenger can build a SMALLER factory. One fewer fletcher, one fewer cache. Those floor slots and runner trips can go to other things (tower growth, companion supply, construction materials).
- BUT: scrap does 70% damage. The Scavenger trades efficiency for volume. High Tempo + low Precision = lots of shots, many misses, some scrap recovery. It's messy but it WORKS. The economy funds the waste through Vulture (2x drops = 2x income).
- The Scavenger is the class that can afford to spray-and-pray because their economy can absorb the waste. Other classes are punished for inaccuracy. The Scavenger turns inaccuracy into a playstyle.
- Low position is ideal — near the ground where kills drop loot quickly (gravity collection) and where scrap ammo is most likely (more enemies at lower floors, faster kill-to-scrap cycle).

**Scrounging perk amplifications:**
- "Resourceful" — Scrap ammo does 85% damage instead of 70%.
- "Scrap Magnet" — Scrap ammo from companion kills also goes to the Scavenger's rack (not just hero kills).
- "Recycler" — Missed shots have a 20% chance to generate scrap ammo (you even profit from missing).
- "Junk Bomb" — Once per encounter, the Scavenger can convert 10 scrap ammo into a massive AoE explosion (clears a wave, costs no real ammo).

---

### The Berserker — "Rage"

**Identity:** desperation becomes determination. When the tower is breaking, you fight harder to save it. The hero who converts disaster into power.

**Tonal note:** Berserker is the class most at risk of drifting from the project's "protective home" emotional promise. The mechanic rewards damage and breaches, which can read as *wanting* your home destroyed rather than *enduring* despite destruction. The framing must stay defensive: the Berserker isn't a chaos agent who enjoys ruin — they're the person who fights hardest when everything is falling apart. Rage is adrenaline born from protectiveness, not bloodlust. Companion dialogue should reinforce this: "The wall's down and they're still coming? Good. Now I'm angry." not "Finally, some real damage." If playtesting shows Berserker players deliberately sabotaging their tower for Rage, the mechanic needs guardrails (e.g., Rage only from *enemy-caused* damage, not from skipping repairs).

**Starting stats:** high Tempo, high Draw. Low Precision, low Grit. Zero Salvage.
**Starting weapon:** common sword (melee — Berserker wants enemies close).
**Class perk (always active):** Bloodrage — base damage increases by 2% for every 10% of total panel HP lost across all tower floors. Tower at 50% integrity = +10% damage.

**Exclusive mechanic: Rage meter (0-100).**

Rage fills from adversity and decays slowly (-5/sec if nothing bad is happening). Resets between encounters.

| Event | Rage gain |
|-------|-----------|
| Panel breach on any floor | +25 |
| Rack empties at hero position | +20 |
| Companion displaced | +30 |
| Runner killed | +15 |
| Infrastructure destroyed | +10 |
| Foundation takes a hit | +40 |

| Rage tier | Threshold | Effect |
|-----------|-----------|--------|
| Simmering | 20+ | +15% fire rate, +10% damage |
| Burning | 50+ | +30% fire rate, +20% damage, attacks pierce 1 target |
| Berserk | 80+ | +50% fire rate, +30% damage, attacks pierce ALL targets in line, weapon ability has no cooldown |

At Berserk tier: hero glows red, weapon crackling, fire rate is insane, screen edges pulse red. But you're in Berserk because your tower is WRECKED.

**Why Berserker shapes the whole run:**
- The Berserker WANTS a tower that takes damage. Might CHOOSE not to repair a panel. "That breach on floor 4? Leave it. I'm stronger with it open."
- Low Precision + high Tempo = spray-and-pray, wasting ammo. But at high Rage, piercing means stray shots hit something eventually.
- Low Grit = panels break easier = more Rage = more power. Self-feeding cycle. The question: how much damage can the tower sustain before collapse?
- Melee preference at peak (zero logistics, no rack needed). At Berserk with a sword, the Berserker is a one-person army standing in a ruin.
- The "gambling" class. Others control the game. The Berserker rides the edge of disaster.

**Berserker perk amplifications:**
- "Thrill of Ruin" — Rage decays 50% slower.
- "Pain is Fuel" — Rack emptying gives +35 Rage instead of +20.
- "Undying Fury" — at Berserk tier, wall panel at your position regenerates 3 HP/sec. The tower's spirit focuses healing on your section.
- "Scorched Earth" — when a floor is breached, ALL enemies on the tower face take 10% of their max HP. The tower's destruction hurts them too.

---

### Class scope awareness

Not all classes reshape the game equally. Engineer and Commander are "system-warping" — they change how you think about both combat and tower planning. Archer and Scavenger are "self-warping" — they change how the hero plays but leave the tower broadly similar. Berserker warps the risk calculus. This asymmetry is intentional (different classes appeal to different player types), but it means Engineer/Commander players may feel they're playing a different game than Archer players. If that gap feels too wide in playtesting, the fix is to give Archer and Scavenger more tower-level expression (e.g., Archer's Focus could reduce ammo consumption on the floor, creating a logistics signature), not to flatten Engineer/Commander.

---

## II. Class Comparison

| Aspect | Archer | Engineer | Commander | Scavenger | Berserker |
|--------|--------|----------|-----------|-----------|-----------|
| One-line fantasy | "One shot. One kill." | "I build machines and they win for me." | "My team is the weapon." | "I take what I need." | "Everything is on fire and I've never been stronger." |
| Core verb | Wait, then kill | Boost the machine | Empower the team | Scavenge and spray | Feed on chaos |
| Combat feel | Slow, precise, devastating | Moderate, strategic | Moderate, supportive | Fast, chaotic, self-feeding | Escalating, reckless, explosive |
| Exclusive mechanic | Focus (2x damage after pause) | Overclock (2x production burst) | Rally (passive team aura) | Scrounging (free scrap ammo) | Rage (adversity → power) |
| Supply chain demand | Very low (fires rarely) | Low-medium (boosts throughput) | High (clustered companions drain) | Medium (partially self-supplied) | Variable — low at peak Rage (melee), high otherwise |
| Ideal position | High (range, Focus time) | Mid (near critical buildings) | Mid (centered in companion cluster) | Low (near ground for loot/scrap) | Low-mid (near threats, wants breaches nearby) |
| Best weapons | Longbow, heavy crossbow, musket | Any (Overclock is the weapon) | Staff, instruments (easy aim, support) | Throwing knives, pistol, shortbow (fast) | Sword, hammer, flail (melee at Rage peak); shortbow when building Rage |
| Companion strategy | Independent — spread companions for coverage | Independent — companions where needed | Clustered — stack 3-4 near Commander | Independent — companions anywhere | Minimal — Berserker carries, companions cover what Berserker can't |
| Economy pressure | Low overhead, modest income | Low overhead, efficient growth | High wages (many clustered companions) | High income, moderate overhead | Low overhead (melee = no ammo), low income (no Salvage) |
| Risk profile | Safest (high position, strong panels) | Moderate (tower-dependent) | Risky (single cluster = single point of failure) | Risky (low position, weak panels) | Highest risk — deliberately lets tower take damage. Rewards and consequences are both extreme. |

---

## II-B. Perk Tree (v1)

> **Status:** v1 spec. 3 branches (Combat, Logistics, Defensive) × 3 tiers = 9 perks per class. Players unlock one perk every 5 hero levels. Perks from any branch can be mixed freely. All values are in the balance config.
>
> **These are generic perks, not class-specific amplifications.** Class-specific perk amplifications (Focus Chain, Overclock Cascade, etc.) described in §I are post-v1. The generic perk tree below is sufficient for v1 — it provides meaningful build differentiation without requiring per-class perk design.

### Archer Perks

| Branch | Tier | Name | Effect |
|--------|------|------|--------|
| Combat | 1 | Steady Hand | +15% accuracy after not moving aim for 1s |
| Combat | 2 | Vital Strike | Critical hits deal 2.5x damage instead of 2x |
| Combat | 3 | Deadeye | Weak spot (crit zone) size +25% (stacks with Eagle Eye class perk) |
| Logistics | 1 | Fletcher's Friend | Arrows that miss have 20% chance to be recovered (returned to rack) |
| Logistics | 2 | Lean Supply | Personal ammo reserve increased from 10 to 20 |
| Logistics | 3 | Recycler | Kills have 30% chance to refund the ammo spent on the killing shot |
| Defensive | 1 | Fortified Position | Wall panel at hero's floor has +20% max HP |
| Defensive | 2 | Patch Work | Hero kills repair 1 HP to the nearest damaged panel |
| Defensive | 3 | Last Stand | When any panel is breached, hero gains +40% fire rate for 10s (once per encounter) |

**Build archetypes:**
- *Precision sniper:* Combat 1-2-3. Maximize crit damage with Focus.
- *Ammo miser:* Logistics 1-2-3. Fire rarely but never run dry. Pairs with lean tower builds.
- *Hybrid defender:* Combat 1, Logistics 1, Defensive 1-2. Balanced early picks, hero contributes to tower durability.

### Engineer Perks

| Branch | Tier | Name | Effect |
|--------|------|------|--------|
| Combat | 1 | Turret Synergy | +10% hero damage when on a floor with an active production building |
| Combat | 2 | Overcharge | Overclock also boosts hero fire rate by +25% for its duration |
| Combat | 3 | Chain Reaction | Hero kills within 2 floors of an active building trigger a spark dealing 50% of kill damage to nearest enemy |
| Logistics | 1 | Efficient Design | All production buildings consume 15% less gold in operating costs |
| Logistics | 2 | Express Delivery | Runners move 20% faster |
| Logistics | 3 | Automation | Dumbwaiters operate at 2x speed; chutes carry 2 crates per trip |
| Defensive | 1 | Reinforced Panels | All wall panels gain +10% max HP |
| Defensive | 2 | Quick Repair | Panel repair during prep costs 1 fewer tick (min 1) |
| Defensive | 3 | Blast Doors | Breached panels auto-seal after 10s instead of waiting for spirit expulsion at 15s. Raiders inside are expelled when the seal completes. |

**Build archetypes:**
- *Tower optimizer:* Logistics 1-2-3. Maximum efficiency, cheapest tower possible. Overclock fills the gaps.
- *Combat engineer:* Combat 1-2, Logistics 1. Turret Synergy + Overcharge turns Overclock windows into personal damage spikes.
- *Fortress builder:* Defensive 1-2-3, Logistics 1. Hardened tower, cheap repairs, auto-sealing breaches.

### Commander Perks

| Branch | Tier | Name | Effect |
|--------|------|------|--------|
| Combat | 1 | Coordinated Fire | Companions within Rally aura deal +10% damage to targets the hero has hit in the last 2s |
| Combat | 2 | War Cry | Activating Hero Skill triggers a bonus volley from all companions within Rally range |
| Combat | 3 | Focus Fire | When hero hits an enemy, companions within 2 floors prioritize that target for 3s |
| Logistics | 1 | Inspiring Presence | Runners within 2 floors of hero move 15% faster |
| Logistics | 2 | Shared Supplies | Companion ammo consumption within Rally aura reduced by 10% (stacks with Rally's -15%) |
| Logistics | 3 | Field Requisition | Once per encounter, instantly fill all racks within 2 floors from warehouse reserves |
| Defensive | 1 | Shield Wall | Panels on floors within Rally aura take 15% less damage |
| Defensive | 2 | Hold the Line | When a panel is breached, companions within 2 floors gain +25% fire rate for 5s |
| Defensive | 3 | Unbreakable | Foundation takes 20% less damage while hero is alive and Rally aura is active |

**Build archetypes:**
- *Kill commander:* Combat 1-2-3. Hero marks targets, companions execute. Maximum team DPS.
- *Supply officer:* Logistics 1-2-3. The Rally cluster's ammo hunger is managed through efficiency and emergency resupply.
- *Bastion:* Defensive 1-2-3. The Rally zone becomes a nearly indestructible section of the tower.

### Perk Balance Config

```toml
[perks.archer]
steady_hand_accuracy_bonus = 0.15
steady_hand_delay = 1.0
vital_strike_crit_multiplier = 2.5
deadeye_crit_zone_bonus = 0.25
fletchers_friend_recovery_chance = 0.20
lean_supply_reserve = 20
recycler_refund_chance = 0.30
fortified_position_hp_bonus = 0.20
patch_work_heal_per_kill = 1.0
last_stand_fire_rate_bonus = 0.40
last_stand_duration = 10.0

[perks.engineer]
turret_synergy_damage_bonus = 0.10
overcharge_fire_rate_bonus = 0.25
chain_reaction_damage_ratio = 0.50
chain_reaction_range_floors = 2
efficient_design_cost_reduction = 0.15
express_delivery_speed_bonus = 0.20
automation_dumbwaiter_speed = 2.0
automation_chute_capacity = 2
reinforced_panels_hp_bonus = 0.10
quick_repair_tick_reduction = 1
blast_doors_seal_time = 10.0

[perks.commander]
coordinated_fire_damage_bonus = 0.10
coordinated_fire_window = 2.0
war_cry_volley_count = 1
focus_fire_duration = 3.0
inspiring_presence_speed_bonus = 0.15
inspiring_presence_range_floors = 2
shared_supplies_ammo_reduction = 0.10
field_requisition_range_floors = 2
field_requisition_uses = 1
shield_wall_damage_reduction = 0.15
hold_the_line_fire_rate_bonus = 0.25
hold_the_line_duration = 5.0
unbreakable_foundation_reduction = 0.20
```

---

## III. Companion Passives — Synergies & Scaling

**Complexity budget.** The companion system stacks many layers: exterior passives, interior passives, positional synergies, anti-synergies, experience scaling (3 tiers), class synergies, affinity types, relationship ranks (5 levels), affinity pair bonuses, and paired narrative endings. Each is interesting individually, but together they create a large optimization surface that risks overwhelming players and making balance intractable.

**Implementation priority (layer by layer):**
1. **Core passives + experience scaling** — ship first. This is the minimum for companions to feel distinct and to grow.
2. **Positional synergies** — ship second. Adjacent-floor bonuses create interesting placement decisions without new UI.
3. **Affinity types + relationship levels** — ship third. Narrative warmth is the primary reward; bonuses are secondary.
4. **Interior companions** — ship as a post-launch expansion or late-game unlock. They introduce a new building type (offices), new roster category, and new logistics loops. High value, high complexity.
5. **Affinity pair bonuses (the full matrix)** — defer or simplify. The 15-entry affinity combination table is the highest-complexity, hardest-to-balance layer. Consider shipping with a simpler rule: "adjacent companions with matching affinity get a flat +5% to their shared stat." The full matrix can come later if the simpler version feels too flat.

**Player-facing readability:** no matter how many layers exist internally, the player should never need to consult a matrix. The prep UI should handle it: drag a companion to a position, and synergy/anti-synergy icons appear automatically with tooltips. The system's depth lives in placement experimentation, not in memorizing tables.

### Positional synergies

When specific companions are on adjacent floors, they gain a bonus. This is communicated during prep: when you drag a companion to a position, compatible companions on adjacent floors show a "synergy" icon and tooltip.

| Companion A | Companion B | Synergy (adjacent floors) |
|------------|------------|---------------------------|
| Ren (Mark) | Any high-DPS companion or hero | Mark amplifies concentrated fire. Ren wants to be adjacent to your biggest damage dealer. +5% bonus Mark damage when adjacent to hero. |
| Kael (Wall) | Any ranged companion above | Kael blocks climbers, giving the companion above uninterrupted shooting time. Companion above Kael gets +10% accuracy (no pressure, calm shooting). |
| Drift (Pinning) | Mira (Splash) | Slow + AoE = combo. Drift's pinned enemies cluster, Mira's splash hits more. Mira's splash damage +15% against pinned enemies. |
| Yuki (Patch) | Forge (Reinforce) | Yuki heals exterior, Forge boosts production. Floor between them is the tower's stronghold. Both bonuses stack: the floor between gets +15% production AND +20% repair rate. |
| Bell (Harmony) | Any companion | Bell's accuracy aura is universal. But adjacent to another instrument user (hero or companion), the aura doubles (+10% accuracy instead of +5%). |
| Sable (Discount) | At a merchant node | Sable's discount is always active, but if she's on the ground floor (closest to the merchant wagon), the discount increases to 15% instead of 10%. Positioning for economy. |

### Anti-synergies (avoid, not penalize)

These aren't penalties — just situations where placement is suboptimal. Companion dialogue hints at it: "Mira and I both pull from the same cache. Maybe spread us out?"

| Situation | Why it's suboptimal |
|-----------|---------------------|
| Two splash companions on adjacent floors | Both drain ammo fast from nearby caches. Double the logistics pressure on one section. |
| Two shield-bearers | Redundant. Climbers stopped by the first never reach the second. |
| Ren (Mark) on top floor, all companions below | Nobody benefits from Mark — the hero is the only one who can shoot Mark's targets from above. Wasted if hero is also above. |

### Passive scaling with experience

Passives grow stronger as the companion gains combat experience. Thresholds at 10 and 20 encounters survived.

| Companion | Base (0-9 encounters) | Veteran (10-19) | Elite (20+) |
|-----------|----------------------|-----------------|-------------|
| Ren (Mark) | +20% damage on marked, 3s duration | +25% damage, 4s duration | +30% damage, 5s duration, marks visible through walls |
| Kael (Wall) | Blocks climbers | Blocks climbers, takes 25% less panel damage at position | Blocks climbers, reflects 10% damage back to blocked enemies |
| Mira (Splash) | AoE shots, 2x ammo | AoE 15% larger, 1.8x ammo | AoE 25% larger, 1.5x ammo (becomes more efficient with experience) |
| Yuki (Field Medic) | Shots repair wall panels (+3 HP per hit) instead of damaging enemies. Targets most damaged panel. | +4 HP per hit, can target hero's panel specifically | +5 HP per hit, heals adjacent panels for half, panel at Yuki's position auto-repairs 1 HP/sec |
| Drift (Pinning) | 1s slow | 1.5s slow | 2s slow + slowed enemies take 10% more damage from all sources |
| Volt (Arc) | Shots chain to 1 additional enemy within 1 floor (60% damage) | Chains to 2 enemies (70% damage) | Chains to 3 (80% damage), chained targets stunned 0.5s |
| Sable (Profiteer) | Shots generate gold on hit (50% damage, +2g per hit) | 60% damage, +3g per hit, kills reveal loot table | 70% damage, +4g per hit, can "appraise" 1 enemy/encounter (shows loot before killing) |
| Thorn (Snare) | 2s can't climb | 3s can't climb | 3s can't climb + snared enemies drop from current position (fall down 1 floor) |
| Bell (Harmony) | +5% accuracy, 2 floor range | +8% accuracy, 2 floors | +10% accuracy, 3 floors + slight fire rate bonus (+5%) in aura |
| Ash (Salvage) | Loot rolls 50% faster near position | 75% faster | 100% faster + chance for bonus loot (duplicate drop, 10%) |
| Rust (Suppression) | Hit enemies 20% slower, 2s | 25% slower, 2.5s | 30% slower, 3s + suppressed enemies deal 15% less panel damage |
| Kit (Jury-Rig) | 1 temp repair per encounter | 2 temp repairs | 2 temp repairs + permanent repair if infrastructure survives the encounter |
| Shade (Vanish) | Position treated as unoccupied | Same + Shade's shots don't reveal position | Same + once per encounter, Shade can "vanish" an adjacent companion's position for 10s (enemies skip them too) |
| Stone (Anchor) | 50% less tower sway, climbers 30% slower at position | 60% less sway, 40% slower | 70% less sway, 50% slower + Stone's floor panel has +30% HP |

**Why scaling matters:**
- Makes veteran companions meaningfully better than fresh recruits
- Creates genuine pain when a veteran is displaced by a breach (they're not dead but they lose their position — and you might need to put a new recruit there instead)
- Connects to the "competence" metric — players FEEL their team growing
- Creates a hard decision: dismiss a 15-encounter veteran with a "meh" passive for a fresh recruit with an amazing passive? The veteran's scaled bonuses might outweigh the recruit's base ability.

---

## IV. Class × Companion Composition

Each class naturally gravitates toward certain companion compositions:

### Archer compositions

The Archer fights independently. Companions provide coverage, not amplification. Best companions for Archer:

- **Kael (Wall)** below the Archer — blocks climbers, gives the Archer Focus time
- **Ren (Mark)** adjacent — Archer's Focus shots on marked targets are devastating (2x Focus × 1.2 Mark = 2.4x damage)
- **Drift or Rust** on distant floors — independent defenders who don't need babysitting
- **Yuki** anywhere — passive repair reduces the Archer's main vulnerability (panel damage breaking Focus)

### Engineer compositions

The Engineer cares about the TOWER more than the team. Companions are positioned for logistics, not combat synergy.

- **Forge (Reinforce)** adjacent to critical production — stacking Engineer's Overclock with Forge's production bonus
- **Kit (Jury-Rig)** on a floor with critical infrastructure — auto-repairs the stuff Engineer can't protect during combat
- **Any ranged companion** in other positions — covering sectors Engineer doesn't personally defend well
- **Yuki** on a floor that takes heavy damage — Engineer handles throughput, Yuki handles durability

### Commander compositions

The Commander NEEDS companions clustered. The Run depends on Rally.

- **3-4 ranged companions** on floors adjacent to the Commander — forming the "kill zone"
- **Ren (Mark)** in the zone — Mark + Rally fire rate = absurd burst damage from the whole cluster
- **Bell (Harmony)** in the zone — stacking accuracy bonuses (Rally +20% + Harmony +5-10% = companions rarely miss)
- **Kael (Wall)** below the zone — protecting the entire cluster from climbers
- **One independent companion** outside the zone — covering the section Rally doesn't reach

The Commander's tower is lopsided: one section is incredibly powerful, the rest is undefended. The supply chain must feed the zone heavily.

### Scavenger compositions

The Scavenger doesn't rely on companions for damage — they're self-sufficient. Companions provide utility.

- **Ash (Salvage)** on a nearby low floor — stacking scavenge bonuses. Both near the ground = maximum loot income.
- **Shade (Vanish)** on any floor — Shade's position is ignored by enemies, reducing total threat surface
- **Kael (Wall)** on a floor above — protects the Scavenger's high-value position from climbers
- **Any grenadier or splash companion** to create more kills (more kills = more scrap ammo for the Scavenger)

---

## V. Balance Config — Class Parameters

All class-exclusive mechanic values are in the runtime balance config:

```toml
[classes.archer]
focus_activation_time = 2.0        # seconds of no-fire to enter Focus
focus_damage_multiplier = 2.0
focus_accuracy_override = 1.0      # 1.0 = perfect accuracy
focus_break_on_panel_damage = true
eagle_eye_crit_zone_multiplier = 1.5

[classes.engineer]
overclock_duration = 20.0          # seconds
overclock_speed_multiplier = 2.0
overclock_cooldown = "per_encounter"  # resets between encounters
overclock_uses_per_encounter = 1

[classes.commander]
rally_range_floors = 2             # floors above and below
rally_fire_rate_bonus = 0.30
rally_accuracy_bonus = 0.20
rally_ammo_efficiency = 0.15       # 15% less ammo consumed
tactician_xp_multiplier = 1.5     # 50% faster companion accuracy growth

[classes.scavenger]
scrap_drop_chance_hero_kill = 0.40
scrap_drop_chance_companion_kill = 0.15
scrap_damage_multiplier = 0.70    # 70% of normal damage
vulture_loot_speed_multiplier = 2.0
vulture_drop_rate_multiplier = 2.0

[companion_scaling]
veteran_threshold = 10             # encounters for first scaling tier
elite_threshold = 20               # encounters for second scaling tier
```

**Telemetry feedback loop:** if Archer win rate diverges >15% from Commander, adjust Focus damage multiplier or Rally bonuses. If Scavenger is 50% pick rate, Scrounging is too good — reduce scrap drop chance or damage multiplier. Track class-specific metrics: Focus shots per encounter, Overclock targets, Rally uptime, scrap ammo generated. See [telemetry-balance.md](telemetry-balance.md) §VI.

---

## VI. New Companion Types

### Exterior companion: Wren, the Misdirector (Trickster fantasy)

**"I play a different game than you."**

- Position: EXTERIOR (balcony, like normal companions)
- Passive: **Redirect** — once every 15 seconds, Wren redirects one climbing enemy to a different floor. The enemy changes target. If redirected to an unoccupied floor, the enemy climbs all the way up and off the tower — removed from the encounter without killing it. No bounty, no loot, but no ammo spent.
- Weapon preference: wand (doesn't need high damage — just touches enemies with redirect effect)
- Affinity: Cunning
- Personality: playful, mischievous. Talks to enemies during combat: "Wrong way, little guy. Try floor 7. Nothing there."
- Story: was a guide who led travelers through dangerous territory. Learned the best way to survive isn't fighting — it's making threats go somewhere else.
- Quote: "Fighting is what you do when you've run out of better ideas."
- Scaling:
  - Base: redirect 1 enemy per 15s
  - Veteran (10 enc): 1 per 12s, redirected enemies slowed 30% for 5s
  - Elite (20 enc): 1 per 10s, can redirect flyers too, 20% chance redirected enemies attack other enemies for 5s

**Why Wren matters:** pure utility. Never the highest kill count, but encounters with Wren have fewer breaches. Saves ammo (enemies removed without shooting). Anti-synergy with Scavenger (redirected enemies don't drop scrap). Synergy with Kael (Kael blocks, Wren redirects what gets past).

### Interior companions — a new category

Normal companions stand on balconies and shoot. **Interior companions** are assigned to a tower floor's interior and affect logistics during combat. They don't fight, don't need ammo racks, don't have targeting orders. They manage.

Interior companions require an **office** built on a floor (like balconies for exterior companions). Offices cost ticks + materials to build, take floor width. Same investment model as balconies but for logistics instead of combat.

Interior companions cost a companion slot + an office. No balcony, no cache, no ammo infrastructure. The tradeoff: combat coverage vs. supply chain efficiency.

### Interior companion: Tinker, the Gadgeteer

**"Give me some scrap and ten minutes. I'll make something that shouldn't work but does."**

- Position: INTERIOR (any floor)
- Passive: **Gadgets** — each prep phase, Tinker builds ONE temporary gadget from 1 crate of resources and places it on a specific floor. The gadget activates during the next encounter, then is consumed. A new micro-decision every prep stop.
- Affinity: Cunning
- Personality: hyperactive, hands always moving, talks through problems out loud. Surrounded by half-finished projects.
- Story: was apprenticed to a clockmaker who made beautiful, useless things. Tinker makes ugly, essential things. Got kicked out for "lacking artistry." Found that walking towers don't care about artistry — they care about whether the thing works when the wall is breaking.
- Quote: "It's not pretty. But it'll save your life exactly once."

**Gadget types (choose one per prep stop, costs 1 crate of any resource):**

| Gadget | Placed on | Effect during encounter |
|--------|-----------|------------------------|
| Spring Trap | Any floor exterior | First climber to reach this floor is launched off the tower (instant kill on small, staggers large). One-use. |
| Smoke Screen | Any floor interior | Sappers skip this floor entirely — can't see infrastructure inside. |
| Ammo Funnel | Any floor with cache | Cache auto-refills from warehouse at 2x speed for the encounter. No runner needed for that cache. |
| Alarm Wire | Any floor exterior | When enemies climb past, ALL companions get +5% accuracy for 10 seconds (early warning focus). |
| Panic Hatch | Any floor interior | If breached, goods from the building's output buffer are dumped to warehouse instead of being destroyed. Saves production output from breach. |
| Jury-Rig Armor | Any balcony | One companion's panel gets +30% HP for the encounter. |

**Scaling:**
- Base: 1 gadget per prep stop, 4 types available (Spring Trap, Smoke Screen, Ammo Funnel, Alarm Wire)
- Veteran: all 6 types available, gadget effects +25% stronger
- Elite: 2 gadgets per prep stop (on different floors). The tower gets two surprises.

### Interior companion: Cog, the Mechanic

**"The lift on floor 4 is making a noise. I don't like the noise."**

- Position: INTERIOR
- Passive: **Maintain** — transport infrastructure on Cog's floor never breaks down and is immune to sapper damage. Cog also auto-repairs any transport damaged in the previous encounter (free, no tick cost).
- Affinity: Guard
- Personality: quiet, focused, covered in grease. Talks to machinery more than people.
- Story: worked in the engine room of a stationary fort's generator. When the fort fell, Cog walked away with a wrench and nowhere to use it. The tower's machinery sings to Cog the way the generator used to.
- Quote: *sound of wrench tightening*
- Scaling:
  - Base: transport immune to failure/sapper on Cog's floor
  - Veteran: adjacent floors' transport also protected
  - Elite: all transport on Cog's floors runs 15% faster (well-maintained = efficient)

### Interior companion: Broth, the Cook

**"You fight on full stomachs or you don't fight at all."** (Caretaker / sustain fantasy)

- Position: INTERIOR (needs own office — the kitchen)
- Passive: **Meals** — Broth produces meals from raw materials diverted from the supply chain. During prep, the player assigns a resource input (any production output: arrows, bolts, stone, wood, planks — anything). Broth consumes 1 crate per encounter and produces 2 meals. During prep, meals are distributed to individual companions as a buff for the next encounter.
- Affinity: Spirit
- Personality: gruff, caring, territorial about the kitchen. Opinions about everything. Feeds people whether they want it or not.
- Story: cooked for a garrison that disbanded. Then a caravan that broke up. Then a tavern that closed. Broth keeps losing the people they cook for. The tower is the first home where the people stay and the meals keep coming.
- Quote: "Eat. You're too thin and your aim is shaky."

**Meal types (player chooses which to prepare each encounter):**

| Meal | Input cost | Effect (one companion, one encounter) |
|------|-----------|---------------------------------------|
| Hearty Stew | 1 crate of any raw resource | +15% fire rate for the encounter |
| Sharp Brew | 1 crate of mana crystals | +10% accuracy + crit chance for the encounter |
| Iron Rations | 1 crate of bolts or stone | Companion's panel section takes 20% less damage |
| Trail Mix | 1 crate of wood or planks | Companion's rack holds +1 crate for the encounter |
| Mystery Soup | 1 crate of anything | Random buff (one of the above, 1.5x strength). Broth won't tell you what's in it. |

**2 meals per encounter** (from 1 crate input) — distributed to 2 different companions during prep. Choose which companions get fed and what they eat.

**Why Broth is interesting:**
- Broth CONSUMES from the supply chain. Every crate going to the kitchen is a crate NOT going to an ammo rack. Feeding companions means slightly fewer arrows for the hero. Real tension.
- Meal distribution is a prep decision: who benefits most? The companion on the most dangerous floor (Iron Rations)? The one with the worst accuracy (Sharp Brew)? The one whose rack runs dry (Trail Mix)?
- Mystery Soup is the gamble — stronger but random. Do you trust Broth's cooking?
- Broth creates a new logistics loop: production → warehouse → kitchen → meal → companion buff. A loop that didn't exist before. The tower's factory now has a cafeteria.
- Companions who receive meals have unique reactions: "Broth's stew again. ...actually, it's good." / "I fight better on a full stomach. Don't tell anyone I said that." / Rust just nods.

**Scaling:**
- Base: 1 crate → 2 meals, 4 meal types + Mystery Soup
- Veteran: 1 crate → 3 meals (can feed 3 companions per encounter), meal effects +20% stronger
- Elite: 1 crate → 3 meals, meal effects +40% stronger, NEW meal unlocked: "Feast" (costs 2 crates, ALL exterior companions get a weaker version of any meal buff — mass-feed). Broth cooks for the whole tower.

**Relationship with Broth:** Broth has some of the best relationship dialogue because feeding people is intimate.
- Acquaintance: "Here. Eat." (shoves bowl at companion)
- Friend: Broth starts remembering preferences. "Drift likes more salt. Noted."
- Close: "I noticed you skipped your meal before the last fight. Don't do that again."
- Bonded: "I'll cook for you as long as you let me. That's not a contract. It's a promise."

### Interior companion: Smuggler, the Black Marketeer

**"Don't ask where I got it. Just be glad I did."**

- Position: INTERIOR (any floor — the less visible, the better)
- Passive: **Connections** — between encounters, Smuggler acquires 2-3 crates of resources "from somewhere." No production building needed, no gold cost. FREE resources that appear in the warehouse. But the quality is uncertain.
- Affinity: Cunning
- Personality: evasive, cheerful, morally flexible. Always has a story about "a guy I know." Pockets full of things that fell off other towers.
- Story: nobody knows. Smuggler has three different origin stories and tells a different one each time. All that's certain: they have access to supply lines that shouldn't exist in a world where the roads are closed.
- Quote: "Found this behind a rock. What rock? A rock. Don't worry about it."

**How Connections works:**

Each prep stop, 2-3 crates appear in the warehouse labeled "acquired by Smuggler." But each crate has a quality roll:

| Quality | Chance | Effect |
|---------|--------|--------|
| Premium | 15% | Resource is 50% more effective (premium arrows do bonus damage, premium stone repairs more HP) |
| Normal | 55% | Standard resource, no modifier |
| Dodgy | 25% | Resource is 75% effective (dodgy arrows do slightly less damage, dodgy planks repair less) |
| Hot | 5% | Resource works fine BUT a mystery event triggers next stop (the "owner" comes looking for their stuff — could be a fight, could be a trade opportunity, could be a shakedown for gold) |

**Why Smuggler is interesting:**
- Free resources bypass the entire production chain. Smuggler is literally injecting value from outside the system.
- The quality uncertainty creates a gambling element — most of the time it's fine, sometimes it's premium (amazing), sometimes it's dodgy (waste), rarely it's hot (consequence).
- Smuggler makes lean towers viable — you don't need as many production buildings if free crates keep appearing.
- Anti-synergy with Broth: Broth consumes crates, Smuggler provides them. Smuggler feeds the kitchen.
- Hot goods create unexpected narrative moments. The "owner" showing up is a mini-event that doesn't exist without Smuggler aboard.

**Scaling:**
- Base: 2 crates per stop, quality roll as above
- Veteran: 3 crates, premium chance increases to 20%, dodgy decreases to 20%
- Elite: 3 crates, can CHOOSE one crate's resource type (the others are still random), hot goods events become profitable (the "owner" always offers a trade, never a fight)

### Interior companion: Dreamer, the Spirit-Touched

**"The tower is trying to tell you something. Let me listen."**

- Position: INTERIOR (any floor — but the tower's spirit resonates strongest on the ground floor near the foundation)
- Passive: **Commune** — Dreamer connects with the tower's enchantment and can channel it once per encounter to do something extraordinary. During combat, the player presses a dedicated key (like the priority flag) to activate Dreamer's channel. One use per encounter.
- Affinity: Spirit
- Personality: serene, distant, slightly unsettling. Hears things nobody else does. Often found touching the walls, listening. Bell (the exterior companion) senses the tower too, but Dreamer SPEAKS to it.
- Story: was born in a walking tower that collapsed when its spirit departed. Dreamer survived because the spirit paused before leaving — touched the child's mind — then left. Dreamer has been looking for another spirit to talk to ever since. Your tower's spirit is the first that listened back.
- Quote: "Shh. It's speaking." (hand on wall, eyes closed)

**Channel options (choose one when activated mid-encounter):**

| Channel | Effect |
|---------|--------|
| **Expel** | Instantly expel all interior raiders from ONE breached floor (normally 15-20 second timer). The tower's enchantment surges at that floor. |
| **Seal** | Temporarily seal one breach — wall panel reforms at 30% HP for 20 seconds. Enemies can't enter. Buys time. |
| **Surge** | The tower's spirit energizes ALL runners — every runner moves at 2x speed for 15 seconds. A logistics burst channeled through the tower itself. |
| **Tremble** | The tower shakes violently. ALL climbers on the exterior face lose grip and fall down 2 floors. Environmental knockback from the building itself. |

**Why Dreamer is interesting:**
- Dreamer is the tower spirit given a human voice. The tower already has an enchantment (expels raiders, legs walk). Dreamer amplifies that enchantment into a player-controlled action.
- One use per encounter = high-stakes decision. Which channel? When? Using Expel early wastes it if no breach happens. Saving Tremble for the boss is smart but risky if climbers overwhelm you first.
- Dreamer is the only companion that gives the player a MID-COMBAT action beyond aiming and abilities. The channel is a fourth button during combat (alongside weapon, ability, skill).
- Connects to the Ghibli narrative: the tower is alive, the spirit is real, and Dreamer is the proof.
- Bell (exterior) and Dreamer (interior) adjacent = the two spirit-touched companions create a resonance. Synergy: channel cooldown reduced by 30% when Bell is on an adjacent exterior floor.

**Scaling:**
- Base: 1 channel per encounter, 2 options available (Expel + Tremble)
- Veteran: 3 options (adds Seal), channel effect +25% stronger
- Elite: all 4 options, 2 channels per encounter (can use two different ones). The tower's spirit trusts Dreamer completely.

### Interior companion: Saboteur, the Trap-Layer

**"They always look up at the walls. They never look down at the floor."**

- Position: INTERIOR (assigned to a specific floor)
- Passive: **Booby Trap** — Saboteur rigs the interior of their floor with traps. If enemies breach at this floor, instead of freely wrecking infrastructure for 15-20 seconds, they face:
  1. First 5 seconds: enemies are DAMAGED by traps (caltrops, tripwires, falling shelves). Takes 30% of their HP.
  2. Next 5 seconds: enemies are CONFUSED — they wreck things slower (50% destruction rate instead of 100%).
  3. Then the tower's enchantment expels them as normal.

  Net result: breach at Saboteur's floor is MUCH less damaging than a normal breach. Infrastructure takes ~40% of normal breach damage instead of 100%.
- Affinity: Cunning
- Personality: paranoid, prepared, always thinking about worst-case scenarios. Has trapped every surface in their office. Companions learn not to touch anything on Saboteur's floor.
- Story: grew up in a border town that was raided every season. Learned that you don't stop raiders by fighting them — you stop them by making the cost of entry too high. Every door, every window, every floorboard can be a weapon if you think about it long enough.
- Quote: "Don't step there. Or there. Actually, just don't step anywhere on this floor."

**Why Saboteur is interesting:**
- Saboteur doesn't prevent breaches — but makes them SURVIVABLE. This is proactive interior defense that no other companion provides.
- Placement decision: which floor is most critical to protect? Put Saboteur on your enchanter floor and a breach there is a setback, not a catastrophe.
- Anti-synergy with Berserker: the Berserker WANTS breaches to fuel Rage. Saboteur reduces breach damage, which reduces Rage gain. The Berserker might deliberately NOT place Saboteur near critical floors.
- Synergy with Cog: Cog protects transport from sappers, Saboteur punishes breach enemies. Together they make one floor nearly immune to interior damage.

**Scaling:**
- Base: traps on own floor only, 30% enemy HP damage, 50% destruction rate
- Veteran: traps also affect adjacent floors (partial protection spreads), trap damage 40%
- Elite: traps on own floor + 2 adjacent floors, trap damage 50%, confused enemies have 20% chance to accidentally repair infrastructure instead of wrecking it (the traps redirect their destructive energy)

### Interior companion: Cartographer, the Route-Maker

**"There's a faster way through here. I've mapped it."**

- Position: INTERIOR (any floor — but moves between floors between encounters to map)
- Passive: **Shortcuts** — Cartographer discovers hidden pathways inside the tower that runners can use. Each encounter the Cartographer has been aboard, ONE permanent shortcut is added to the tower's interior. A shortcut connects two non-adjacent floors directly — runners can use it as an alternative to stairs/lifts. Shortcuts are FREE transport that doesn't take floor width.
- Affinity: Precision
- Personality: meticulous, quiet, always drawing. Has mapped every tower they've been in. Walls in their office are covered with annotated blueprints. Knows the tower's interior better than anyone, including the tower.
- Story: was a surveyor whose province was fully mapped. Every hill, every creek, every tree — documented. Then the roads closed and the maps became meaningless. Cartographer realized: the map that matters isn't the land. It's the INSIDE of the thing you live in. Started mapping tower interiors and finding paths the builders never intended.
- Quote: "There's a gap behind the sawmill on floor 4. A runner could squeeze through and reach the warehouse in half the time."

**How Shortcuts work:**

Each prep stop (while Cartographer is aboard), a new shortcut is permanently added:

- Shortcut connects floor A to floor B (Cartographer chooses — semi-random based on where congestion is worst)
- Runners can use the shortcut as transport — same as a chute or lift but free, no floor width, no construction cost
- Shortcuts are one-directional (only down, like a hidden slide) or bi-directional (a crawlway) depending on the floors connected
- Shortcuts can't be destroyed by sappers or breaches (they're hidden paths, not infrastructure)
- Maximum 5 shortcuts per tower (Cartographer eventually finishes mapping)

**Why Cartographer is interesting:**
- Cartographer's value GROWS over time. Encounter 1: no shortcuts. Encounter 10: 5 shortcuts = the tower has a hidden express network that bypasses congested stairs and can't be sabotaged.
- The long-term investment model: Cartographer does almost nothing in their first encounter. By mid-run, they've transformed the tower's logistics permanently. Recruit early for maximum value.
- Shortcuts are permanent and indestructible — the only transport that can't be broken. Insurance against the worst breaches.
- Synergy with Berserker: the Berserker's tower gets wrecked, transport gets destroyed, but shortcuts survive. Cartographer is the Berserker's logistics lifeline.
- Anti-synergy with Engineer: the Engineer Overclocks transport. Shortcuts don't benefit from Overclock (they're just paths, not machines). Engineer prefers buildable transport they can boost.

**Scaling:**
- Base: 1 shortcut per encounter (up to 5 max). One-directional (downward only).
- Veteran: shortcuts become bi-directional. Shortcut travel is 20% faster than stairs.
- Elite: max 7 shortcuts. Shortcuts are FASTER than lifts (the hidden paths are the best routes in the tower). Cartographer can choose which floors to connect (player picks during prep instead of semi-random).

### Interior companion: Forge, the Consultant (moved from exterior)

**"Who put the sawmill on floor 6? Gravity exists, you know."**

- Position: INTERIOR (on a production floor)
- Passive: **Optimize** — the building on Forge's floor produces 15% faster AND adjacent buildings (one floor above, one below) produce 10% faster. Forge doesn't just boost one building — they tune the entire section of the production chain, aligning timings and reducing waste.
- Affinity: Guard
- Personality: gruff, competent, critical. Constantly tells you what you're doing wrong (helpfully). The consultant you didn't hire but can't fire.
- Story: built walkers for a living. Retired. Got bored. Now "consults" by joining towers and telling people what they're doing wrong.
- Quote: "Your lift is badly programmed. Let me show you."
- Scaling:
  - Base: own floor +15% production, adjacent floors +10%
  - Veteran: own floor +20%, adjacent +15%, transport on own floor runs 10% faster
  - Elite: own floor +25%, adjacent +20%, adjacent transport +15%, Forge automatically reconfigures caches on adjacent floors to optimal resource type (free, no player action needed)

**Why Forge moved interior:** "+X% production" is a logistics verb, not a combat verb. Forge's character (criticizes your layout, gives build advice) works better inside the tower than on a balcony shooting. As interior, Forge becomes the "production chain optimizer" — the companion who makes your factory hum.

### Class × Interior companion synergies

| Class | Best interior companions | Why |
|-------|------------------------|-----|
| **Archer** | Tinker + Broth | Tinker's Alarm Wire + Broth's Sharp Brew on the Archer = early warning accuracy + crit boost on top of Focus. Devastating precision with information advantage. |
| **Engineer** | Cog + Forge | Cog protects transport. Forge optimizes production. Engineer Overclocks the whole section = factory perfection. Triple logistics synergy. |
| **Commander** | Broth + Cartographer | Broth feeds the Rally cluster. Cartographer builds shortcuts for cache access through hidden paths. |
| **Scavenger** | Smuggler + Broth | Smuggler provides free resources. Broth turns them into companion meals. Economy runs on scavenged goods and mystery stew. |
| **Berserker** | Cartographer + Saboteur | Cartographer's indestructible shortcuts = logistics lifeline in a ruined tower. Saboteur reduces breach damage. The tower falls apart but still functions. |

**The office investment question:** each interior companion needs an office (floor width + ticks + materials). A tower with 3 interior companions has 3 offices eating floor width that could be production buildings or transport. The player decides: how much of my tower is factory, how much is management?

**Total companion roster: ~21 named companions.** 15 exterior (balcony), 8 interior (office). Player sees 8-10 per run, recruits 4-6. The interior/exterior split is another composition decision layered on top of positioning, affinity, and relationship.

**Roster continuity note:** this document has evolved beyond the original companion roster in narrative.md. Several companions here have new or reworked passives (Yuki's "Field Medic" is sharper than the earlier "Patch," Forge moved from exterior to interior, Sable's passive renamed to "Profiteer"). New companions (Volt, Wren, Dreamer, Broth, Cog, Tinker, Smuggler, Saboteur, Cartographer) expand the cast significantly. This doc is now the canonical companion reference — narrative.md's roster section should be updated to match on the next cleanup pass.

**Exterior companion roster — verb audit:**

| Companion | Verb | What makes them distinct |
|-----------|------|-------------------------|
| Ren (Mark) | AMPLIFY | Marks targets for +20% damage from all sources. Creates target priority decisions. |
| Kael (Wall) | BLOCK | Physically stops climbers. Only companion that prevents enemy movement. Placement = chokepoint. |
| Mira (Splash) | EXPLODE | AoE shots at 2x ammo cost. High damage, expensive logistics. |
| Yuki (Field Medic) | HEAL | Shots repair wall panels instead of damaging enemies. The only companion who shoots AT the tower. |
| Drift (Pinning) | SLOW | Hit enemies slowed. Simple, reliable. The dependable option. Combo with Mira. |
| Volt (Arc) | CHAIN | Shots chain between nearby enemies (up to 3 targets). AoE through chaining, not splash. |
| Thorn (Snare) | SNARE | Hit enemies can't climb. Freezes them on the wall. Distinct from slow — enemies are stuck, not sluggish. |
| Bell (Harmony) | HARMONIZE | Accuracy aura + instrument rhythm mechanic. Combat identity IS the rhythm, the aura is the bonus. |
| Sable (Profiteer) | PROFIT | Shots generate gold instead of max damage. Turns combat into income. Can appraise enemies for loot info. |
| Ash (Salvage) | SCAVENGE | Loot rolls to base faster near position. Economy companion for ground-level play. |
| Rust (Suppression) | SUPPRESS | Hit enemies act slower (attack, climb, fire). Reduces enemy effectiveness, not just movement. |
| Kit (Jury-Rig) | FIX | Temp-repairs broken infrastructure mid-encounter. Only companion who interacts with the supply chain during combat. |
| Shade (Vanish) | HIDE | Position treated as unoccupied. Enemies skip Shade. Changes enemy pathing entirely. |
| Stone (Anchor) | ANCHOR | Reduces tower sway + climbers slower at position. The immovable object. |
| Wren (Redirect) | REDIRECT | Sends climbing enemies to empty floors. Removes enemies without killing. |

**Interior companion roster — verb summary:**

| Companion | Verb | What they introduce |
|-----------|------|---------------------|
| Broth (Cook) | FEED | New resource loop: production → kitchen → companion buffs. Consumes to empower. |
| Cog (Mechanic) | PROTECT | Transport immunity. The only defense against sapper infrastructure damage. |
| Tinker (Gadgeteer) | BUILD | One-use tactical devices placed per floor per prep stop. Micro-decisions. |
| Smuggler (Black Marketeer) | ACQUIRE | Free resources from outside the system. Uncertain quality. Hot goods = surprise events. |
| Dreamer (Spirit-Touched) | CHANNEL | Mid-combat tower spirit action (expel, seal, surge, tremble). The tower fights back through Dreamer. |
| Saboteur (Trap-Layer) | TRAP | Interior breach defense. Damages and confuses breach enemies. Makes breaches survivable. |
| Cartographer (Route-Maker) | MAP | Permanent hidden shortcuts that accumulate over encounters. Indestructible logistics. |
| Forge (Consultant) | OPTIMIZE | Production speed boost to own + adjacent floors. The chain tuner. (Moved from exterior — this is a logistics verb.) |

Every companion has a VERB. The simplest (Drift: SLOW, Stone: ANCHOR) are kept intentionally as reliable, easy-to-understand options. Not everything needs to be complex — a roster needs dependable picks alongside exotic ones.

### Office construction costs

```toml
[construction.office]
ticks = 1
materials = {planks = 2}
floor_width = "medium"  # less than a building, more than a cache
```

Same cost structure as balconies. Built during prep. Can be destroyed by breach (interior companions on a breached floor are displaced, same as exterior companions).

---

## VII. Companion Relationships

Companions develop relationships with the hero and with each other over time. Relationships are primarily narrative (dialogue evolves), with mechanical bonuses that are meaningful but not decisive. The player doesn't manage relationships directly — they emerge from proximity (adjacent floors) and shared time (encounters on the same tower).

**Honesty about bonus impact:** the "What this system does NOT do" section says "no gameplay-critical bonuses." But a Bonded Guard+Guard pair giving +30% panel HP, or a Bonded Spirit+Spirit pair giving +20% ammo efficiency, IS strategically meaningful — enough to influence where experienced players place companions. That's fine. The accurate framing: relationships are a *bonus* layer that rewards long-term investment, not a *requirement* for winning. You can ignore them and still complete the game. But optimizers will use them, and that's intentional.

### Relationship levels

| Level | Name | Threshold | How |
|-------|------|-----------|-----|
| 0 | Stranger | Just recruited | — |
| 1 | Acquaintance | 3 encounters together | On the same tower (hero↔companion) or adjacent floors (companion↔companion) |
| 2 | Friend | 8 encounters | Same conditions |
| 3 | Close | 15 encounters | Same conditions |
| 4 | Bonded | 25 encounters | Same conditions |

"Together" = on the same tower for hero↔companion. Adjacent floors for companion↔companion. Non-adjacent companions don't build relationship (too far apart — they don't interact in the tower).

### Affinity system

Each companion (and the hero) has an affinity type. The COMBINATION of two units' affinities determines what bonuses their relationship provides — different pairs give different bonuses, creating meaningful pairing decisions.

| Affinity | Companions | Hero class | Bonus focus |
|----------|-----------|------------|-------------|
| **Precision** | Ren, Drift, Rust | Archer | Accuracy, crit chance |
| **Force** | Mira, Thorn, Stone | — | Damage, AoE, knockback |
| **Guard** | Kael, Yuki, Forge | Engineer | Panel HP, repair, damage reduction |
| **Cunning** | Shade, Kit, Ash | Scavenger | Loot, scrap, infrastructure |
| **Spirit** | Bell, Sable | Commander | Aura range, economy, ammo efficiency |

### Relationship bonuses (affinity-dependent)

Bonuses apply when the paired units are on adjacent floors. Different affinity combinations give different bonuses, scaling with rank. This makes WHO you pair and WHERE you place them a real decision.

**Precision + Precision** (e.g., Ren + Drift, or Archer hero + Ren):

| Rank | Bonus |
|------|-------|
| Friend | +3% accuracy for both |
| Close | +6% accuracy, +5% crit chance |
| Bonded | +10% accuracy, +10% crit chance |

**Precision + Force** (e.g., Ren + Mira):

| Rank | Bonus |
|------|-------|
| Friend | +2% accuracy, +5% damage |
| Close | +4% accuracy, +10% damage |
| Bonded | +7% accuracy, +15% damage |

**Precision + Guard** (e.g., Drift + Kael):

| Rank | Bonus |
|------|-------|
| Friend | +2% accuracy, +5% panel HP at shared section |
| Close | +4% accuracy, +10% panel HP |
| Bonded | +7% accuracy, +15% panel HP |

**Precision + Cunning** (e.g., Ren + Shade):

| Rank | Bonus |
|------|-------|
| Friend | +2% accuracy, +10% loot roll speed |
| Close | +4% accuracy, +15% loot speed |
| Bonded | +7% accuracy, +20% loot speed |

**Precision + Spirit** (e.g., Rust + Bell):

| Rank | Bonus |
|------|-------|
| Friend | +2% accuracy, +5% ammo efficiency |
| Close | +4% accuracy, +10% ammo efficiency |
| Bonded | +7% accuracy, +15% ammo efficiency |

**Force + Force** (e.g., Mira + Stone):

| Rank | Bonus |
|------|-------|
| Friend | +8% damage for both |
| Close | +15% damage |
| Bonded | +20% damage, +10% AoE radius |

**Force + Guard** (e.g., Stone + Kael):

| Rank | Bonus |
|------|-------|
| Friend | +5% damage, +5% panel HP |
| Close | +10% damage, +10% panel HP |
| Bonded | +15% damage, +15% panel HP |

**Force + Cunning** (e.g., Thorn + Kit):

| Rank | Bonus |
|------|-------|
| Friend | +5% damage, +10% loot speed |
| Close | +10% damage, +15% loot speed |
| Bonded | +15% damage, +1 scrap ammo per kill near pair |

**Force + Spirit** (e.g., Mira + Bell):

| Rank | Bonus |
|------|-------|
| Friend | +5% damage, +5% ammo efficiency |
| Close | +10% damage, +10% ammo efficiency |
| Bonded | +15% damage, +15% ammo efficiency |

**Guard + Guard** (e.g., Kael + Yuki):

| Rank | Bonus |
|------|-------|
| Friend | +10% panel HP at shared section |
| Close | +20% panel HP |
| Bonded | +30% panel HP, panel auto-repairs 1 HP/min during combat |

**Guard + Cunning** (e.g., Forge + Kit):

| Rank | Bonus |
|------|-------|
| Friend | +5% panel HP, +10% loot speed |
| Close | +10% panel HP, infrastructure repair 20% faster |
| Bonded | +15% panel HP, infrastructure on shared floor immune to 1 sapper hit per encounter |

**Guard + Spirit** (e.g., Yuki + Sable):

| Rank | Bonus |
|------|-------|
| Friend | +5% panel HP, +5% ammo efficiency |
| Close | +10% panel HP, +10% ammo efficiency |
| Bonded | +15% panel HP, cache on shared floor +1 crate capacity |

**Cunning + Cunning** (e.g., Shade + Ash):

| Rank | Bonus |
|------|-------|
| Friend | +15% loot roll speed |
| Close | +25% loot speed, +10% resource drop rate |
| Bonded | +30% loot speed, +20% resource drops, +5% scrap ammo chance |

**Cunning + Spirit** (e.g., Kit + Bell):

| Rank | Bonus |
|------|-------|
| Friend | +10% loot speed, +5% ammo efficiency |
| Close | +15% loot speed, +10% ammo efficiency |
| Bonded | +20% loot, +15% ammo efficiency, runners serving shared floors +10% speed |

**Spirit + Spirit** (e.g., Bell + Sable, or Commander hero + Bell):

| Rank | Bonus |
|------|-------|
| Friend | +8% ammo efficiency for both |
| Close | +15% ammo efficiency |
| Bonded | +20% ammo efficiency, +5% companion fire rate in shared aura |

### Notable pairings and why they matter

- **Ren + Drift (Precision×Precision):** pure accuracy stack. Both become near-perfect shots. Incredible ammo economy.
- **Ren + Mira (Precision×Force):** Ren marks, Mira blasts. Accuracy + damage = devastating on marked targets.
- **Kael + Yuki (Guard×Guard):** fortress section. +30% panel HP + auto-repair + Yuki's passive. Nearly unbreachable.
- **Shade + Ash (Cunning×Cunning):** loot engine. Everything near them generates more, faster. Economy goes orbital.
- **Stone + Kael (Force×Guard):** immovable defense. Stone's knockback bounces climbers into Kael's wall. +15% damage AND +15% panel HP.
- **Commander hero + Bell (Spirit×Spirit):** aura stacking. Rally + Harmony + Spirit bond = companions barely consume ammo and fire fast. The entire section runs on fumes.
- **Scavenger hero + Ash (Cunning×Cunning):** maximum scrap, maximum loot. Self-sustaining economy engine.

### Paired endings (narrative reward)

At Bonded rank, companion pairs unlock a unique ending line during the victory screen — a brief text about what they do after reaching the destination. The Fire Emblem "paired ending" model.

Examples:

> **Ren + Drift:** "Ren took up cartography again. Drift stayed to help — they said the maps needed someone who'd actually walked the roads. The atlas they produced was the first new one in a decade."

> **Kael + Yuki:** "Kael became the gate guard of the new settlement. Yuki opened a clinic next to the gate. Neither could explain why, but the gate never needed repairing."

> **Thorn + Mira:** "Thorn and Mira opened a theater. The reviews were mixed. The explosions were not."

> **Forge + Kit:** "They opened a workshop together. Forge made things that worked. Kit made things that shouldn't work but did. Between them, the settlement never wanted for anything — or for excitement."

> **Rust + hero:** "Rust stayed. Didn't say why. Didn't need to."

> **Bell + hero:** "Bell said the tower's spirit was content. Then she said something else, too quiet to hear. The tower hummed a note it had never hummed before."

Only the highest-ranked pair per companion gets an ending (like Fire Emblem — if Ren is Bonded with both Drift and the hero, the one with more "points" determines the ending).

### Dialogue evolution

Relationships are primarily experienced through dialogue that warms over time:

**Stranger → Acquaintance:** polite, formal, about the tower/road.
> Kael to Mira: "Grenadier. Try not to blow up the wall I'm standing on."

**Acquaintance → Friend:** warmer, personal details, using names.
> Drift to hero: "Most keepers fire too fast. You've got patience. I like that."

**Friend → Close:** personal, worried, references shared history.
> Forge to Kit: "That jury-rig on the lift... I hate to admit it, but it's holding better than my original design."
> Ren to Drift: "You're getting more accurate. Not that I'm watching. ...I'm watching."

**Close → Bonded:** deep, meaningful, the lines players screenshot.
> Kael to hero: "I've guarded gates, walls, towns. This is the first time I'm guarding a home. YOUR home."
> Drift to hero: "I said I'd leave when the road called. The road's been calling for three chapters. I'm still here."
> Mira to Kael: "You block them. I blow them up. We're a terrible pair and I wouldn't change a thing."

### Romantic undertones (ambiguous by design)

Some pairs develop romantic subtext at Close/Bonded. The game never confirms or denies — the player reads into it what they want. Ambiguity IS the design.

**Pairs with romantic potential:**

| Pair | Flavor |
|------|--------|
| Ren + Drift | Precision meets wanderlust. "Drift talks about leaving. But they aim like someone who plans to stay." |
| Kael + Yuki | Protector meets healer. "Kael never asks for help. I help anyway." |
| Thorn + Mira | Drama meets explosion. "Thorn called my bombs 'inelegant.' I'm going to name the next one after him." |
| Bell + anyone | Mystical connection. "Your heartbeat and the tower's are syncing. I can hear it." |
| Rust + hero | Silence as love language. *Rust quietly places an extra crate at your rack before the encounter starts.* No words. |

**Hero ↔ Companion romantic potential** exists for several companions at Bonded level. Never explicit. Always deniable. The player ships who they want.

### Visual indicators

- Small bond icons between companion portraits in the roster screen (heart stages: empty → quarter → half → full → gold)
- During prep: subtle warm-colored connection line between adjacent companions who have a relationship (visible in the tower cross-section)
- Post-combat: paired dialogue exchanges instead of individual comments when bonded companions are both present

### What this system does NOT do

- No relationship management mini-game. No gifts, no dialogue choices.
- No jealousy, anger, or conflict between companions. This is a warm game about found family.
- No forced romantic outcomes. Everything is ambiguous.
- No bonuses required to win. You can ignore relationships entirely and still complete the game. The dialogue is the primary reward; the mechanical bonuses reward long-term investment without gating progress.
- No relationship decay. Once earned, levels don't decrease.

### Balance config

```toml
[relationships]
acquaintance_threshold = 3
friend_threshold = 8
close_threshold = 15
bonded_threshold = 25
friend_accuracy_bonus = 0.03
close_accuracy_bonus = 0.05
close_kill_bonus_damage = 0.10
bonded_displacement_fire_rate = 0.15
bonded_ammo_share_crates = 1
```

**Telemetry:** track average relationship levels reached per run. If most runs end with only Acquaintance levels, encounters threshold is too high (players don't have enough encounters for bonds to develop). Target: 1-2 Friend relationships per run, 0-1 Close, Bonded only on long successful runs.
