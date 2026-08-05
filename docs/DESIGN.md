# Understory — Design

> The distilled design. For what is actually built, see `SYSTEMS.md`. For the milestone
> roadmap, sprint briefs, and the salvage audit of attempt #1, see `v2-plan.md` — that
> document is locked and remains the only whole-game plan. This one is the argument for why
> the plan says what it says.

---

## 1. The pitch

Understory is a walking garden-tower striding through the jungle that swallowed the old
world. It burns what it cuts to keep its lamps lit, strips bamboo and vines as it walks, salvages
alloy from drowned ruins, and crafts its way upward — while the jungle's territorial fauna
and the old world's feral machines test its walls. It is your factory, your fortress, and
your home. Logistics don't run on belts; they run on **stairs, dumbwaiters, and
elevators**. A belt never queues. An elevator does — and that queue is where the game
lives.

### The four parents

| Parent | What it contributes |
|---|---|
| **SimTower** | The cross-section. Vertical transport as shared, capacity-limited, queueing infrastructure. Elevator scheduling as a first-class game. Floor space as a scarce resource. |
| **Factorio** | Recipes, ratios, buffers, backpressure, tiers — the "one more chain" pull. |
| **Tower defence** | Waves as demand spikes on the same infrastructure the economy runs on. Enemies as spatial threats to specific rooms and shafts. Emplacements as the chain's top consumers. |
| **Ghibli (Nausicaä more than Howl)** | The tower *walks*. The streaming world is the resource input. The journey is the run. The tone is a home reclaiming a wild world, never a war machine. |

Each parent alone is a familiar genre. What makes Understory a different game than any one
of them is that they all draw on the same infrastructure at the same time — which is the
first of the three insights below.

### The three insights that fuse it

1. **Contention is the game.** In Factorio, transport is dedicated: a belt serves one lane
   forever, and once you've built it, it's solved. Here transport is *shared* — every new
   production chain adds load to the same stairs, dumbwaiters, and elevator shafts
   everyone else uses. Your factory's true capacity isn't its production rate; it's its
   circulation capacity. This is why the elevator, not a new resource tier, is the M1
   centerpiece: it's the one piece of infrastructure every other system will fight over.
2. **Combat is a load test.** A wave is a demand spike — darts to the batteries, repair
   crews to a breached panel — riding the same circulation that runs your economy at every
   other moment. You don't win a fight with faster reflexes, because there are no aimed
   weapons to aim. You win it earlier, at the moment you decided whether to place a second
   shaft. Or you simply keep walking: a creature only holds its grip while the tower is
   actually striding, so letting a wave lose its hold costs nothing beyond the charge you
   were already spending to travel — which is why standing still to work through a jam is a
   real risk, not a formality.
3. **The world streams past.** Intake is positional, not a menu choice: bamboo and fiber
   sit thick under dense canopy; sun and scrap sit in the open ruin-fields. The two are
   opposed — shade is biomass-rich and sun-poor, clearings are the reverse — so which
   terrain you walk through simultaneously sets your energy mix, your material mix, and
   your threat profile. Route choice is one decision with three consequences, which is what
   makes it a decision worth making rather than a preference.

---

## 2. The four structural calls

These are the decisions that make the three insights actually true in play, rather than
true in theory. Each one closes off a v1 failure mode by construction — see `v2-plan.md`
§2 and §7 for the full audit of what went wrong the first time.

1. **No hero.** There is no aimed weapon and no character to build. Player combat verbs are
   infrastructural: targeting priorities on emplacements, two or three tower-level cooldown
   abilities (lurch, vent), and logistics triage under fire. Skill expression lives in how
   you reroute the chain, not in how you aim. v1 spent most of its content budget on a hero
   layer — three classes, eleven weapons, ten trinkets — while the supply chain it was
   supposedly about ran on an auto-deliver shortcut until the last commit. Cutting the hero
   entirely is the only way to guarantee that doesn't happen again.
2. **Continuous sim.** Pause-and-plan plus 1×/2×/4×, RimWorld/Factorio-style. There is no
   prep phase and no combat phase — building during a fight is legal, just slow and
   exposed. This is a hard requirement of insight #2: "combat is a load test" is only true
   if the load-bearing infrastructure is live and reroutable *during* the load. v1's
   prep/combat phase split made the single best moment this design can produce —
   rerouting around a shaft that just took a hit, mid-fight — structurally impossible.
3. **Continuous world.** No node map, no chapter graph. Terrain bands stream past within a
   region; occasional route forks choose between a canopy passage and a ruin-field;
   enclaves punctuate as waystations. The map screen is a route overview, not a mode you
   enter. This is what makes insight #3 ("the world streams past") literal rather than a
   metaphor for a menu of node choices.
4. **Crew, not companions.** A small named crew — three to start, capped around eight — who
   *are* the porters, gunners, and repair hands, not separate stat-blocked units bolted on
   top of an automated economy. Jobs plus two needs (meals, sleep) plus a shift rota.
   Personality lives in barks and portraits; there is no relationship mechanic to
   maintain.

---

## 3. Design pillars

1. **The tower is the factory, the fortress, and the home.** One entity, one screen. If a
   feature only touches one of those three framings — say, a stat that affects combat but
   has no logistics footprint — it's worth asking whether it belongs.
2. **Vertical transport is the belt — shared, queued, scheduled, and powered.** Every
   transport decision (which shaft, what capacity, what dispatch policy) is a decision
   about contention, not just throughput.
3. **Combat is a load test.** You fight with what your chain can deliver, to wherever your
   shafts can deliver it. An emplacement with no darts is not a weaker weapon; it's a
   supply failure wearing a gun.
4. **The world streams past.** Terrain sets the input rates; the route is the strategy. A
   tower parked in one spot forever is not a valid way to play — the design assumes motion.
5. **Reclaim, don't conquer.** Solarpunk warmth, not gunmetal. Defence reads as thorns and
   seed-bombs, not artillery. Creatures defend their territory; you are the intruder
   passing through, not a soldier clearing ground. Breakage and silence are the interface —
   see `DECISIONS.md` §8 for how this becomes an enforceable rule rather than a mood board.

---

## 4. Core loop (minute to minute)

The tower strides through a sun-dappled clearing; the burner smokes; the cutter arms
strip bamboo and the mill hums. You place a thornwright on floor 4, add a dumbwaiter to
feed it, reprogram the elevator to skip floor 2. Dusk falls. Leapers drop from the
overstory onto the upper decks — batteries open up, dart racks drain, a shaft takes a hit,
the cells brown out and the elevator stalls mid-climb. You pause. Light the burner (the
smoke will draw more attention) or hold and route repairs up the stairs instead? Dawn
breaks; the wave scatters; the ruin-field — and its alloy — glints on the horizon. Stop to
salvage, or push on for the enclave?

Every clause in that paragraph is a system below, and every system is contending for the
same stairs.

---

## 5. Systems overview

This section is the shape of the game, not its numbers or its build order. Numbers live in
`BALANCE.md`; what's actually implemented versus still ahead lives in `SYSTEMS.md`
(currently M0 only); the milestone sequence lives in `v2-plan.md` §9.

### 5.1 Resources

Small and legible on purpose: **max two inputs per recipe, chain depth of three or less.**
If a chain can't be read at a glance in the cross-section, it's too deep — this is a hard
constraint, enforced by a content-validation check, not a style guide.

Raw materials (bamboo, fiber, produce, scrap) become Tier 1 goods (poles, rope, meals,
alloy, darts) become Tier 2 goods (mechanisms, seed bombs, charge cells). Bamboo is the
contested material, v1's wood tension reborn: it becomes poles for construction, repair,
and ammunition, or fuel for the burner when the sun isn't enough. Burn your building
material, or build with it — never both.

### 5.2 Charge — the keystone resource

Charge is a stored flux in cell banks, not a crate that a crew member carries. The only
source a tower builds is the burner: it eats bamboo — the same stalks the mill wants — and
its smoke raises provocation, so a tower that works hard announces itself. Underneath it the
Heartseed trickles a little for nothing, which is not an income but a floor: it means a tower
that runs dry crawls out slowly rather than dying where it stands.

There were canopy sails once, paying charge for sunlight scaled by terrain. They were cut
(`SYSTEMS.md` §6.10): sun-versus-shade asked the player to net two opposed numbers off
against each other in their head, and measurement showed they cancelled almost exactly — a
decision that cost nothing to get wrong. The route fork asks about danger, salvage and ground
richness now, which are things you can see out of the window. Sinks are striding (a
player-set throttle), powered production, powered transport (elevators and dumbwaiters
draw charge per trip; stairs, ladders, and chutes are free), and night lighting.

The tension: bank charge for the night — defence, lights, elevators under attack — or
spend it walking farther by day. A brown-out during a night assault, with the elevator
stalled and the forge dark, is meant to be as much of a signature emergency as a severed
shaft. Route choice closes the loop from insight #3: shaded jungle is biomass-rich and
sun-poor, open ruins are sun-rich, scrap-rich, and exposed. Your route is your power mix.

There is no tick currency of the kind v1 used to gate prep actions. Trade tokens exist only
at enclaves. The loss condition is the Heartseed — the tower's living core, pre-placed —
being destroyed.

### 5.3 Rooms

Slot-based floors, multiple rooms per floor. v1's one-building-per-floor cap does not exist
in the data model at all, on any milestone — see `DECISIONS.md` §9 for why fixing this on
day one mattered. Scale is cozy on purpose: 8–14 floors, the whole tower on one screen,
always.

Categories: intake (harvests the terrain underfoot), production (crafts on a timer),
energy (burner, cell banks — the burner wants a floor above the works, so the chain has a
direction and a tower has a reason to haul upward), logistics (storerooms,
caches), defence (dart batteries, seed-bomb mortars, repair workshops), crew (bunks,
canteen), and the Heart (the Heartseed, unique and pre-placed).

### 5.4 Transport

The centerpiece, per pillar 2. Stairs are free, always present, slow, and crew-only —
the baseline everyone queues on. A ladder is a cheap two-floor crew-only shortcut. A
dumbwaiter is item-only, autonomous, short-range, and sips charge — Understory's inserter.
The elevator is the machine: a real car simulation with a shaft spanning chosen floors
(costing a slot column on every floor it spans), car capacity, a stop queue, boarding time,
charge draw per trip, and a player-programmable dispatch policy switchable per time of day.
The dispatch model isn't invented from scratch — it ports the trace-verified SimTower
specs from `phulin/tower-together` (bidirectional sweep, dispatch threshold, dwell timing,
fixed floor queues) and extends them for freight, which boards like a passenger with
different dwell and capacity weights. A chute is down-only, fast, item-only, and free
(gravity still works).

Visible queue stress is the load-bearing feedback device, borrowed from SimTower's best
idea: crew and crates waiting too long tint toward red in the cross-section. The bottleneck
diagnoses itself. There is no dashboard, and per pillar 5, there will not be one.

### 5.5 Combat

Enemies approach on the terrain layer and damage infrastructure — panels, rooms, shafts —
not a health bar abstracted away from the tower. Each type teaches one lesson: skitters
teach ammo-drain economics, canopy leapers drop onto *upper* decks (a jungle-native
inversion of v1's climb-from-below, since here the canopy is above you), root-borers gnaw
shafts and legs to teach transport redundancy, spitters bombard from cover to teach
priority targeting, feral wardens are armored old-world machines that wake when you salvage
ruins (provocation given a face), and night predators are the reason you bank charge.

Emplacements auto-fire by player-set priority, consuming ammo from local racks — the
player's verb is triage, not aim. Repair consumes poles, rope, and crew time, so defence is
itself a chain sink, not a separate system bolted on. Creatures do not hold on forever: each
type's grip counts down only while the tower is actually striding, so continuing to walk is
a free, always-available answer to a wave and stopping to work mid-assault is a real risk,
not a formality — losing a wave's grip is not the same as driving it off, and only a kill
counts as the latter. Provocation is one knob: aggressive harvesting, burner smoke, and
ruin-salvaging all raise local threat, which keeps the tone guardrail in pillar 5 true by
construction — they defend their home; you are the one passing through.

### 5.6 Journey and run structure

Roguelike runs: two to four hours, permadeath, seeded, with shareable seeds. Three regions
en route to the Refugia (name provisional): deep jungle, the drowned city (a ruin belt),
and the coast approach. Each region is a terrain palette plus a resource/sun mix plus a
threat table. Enclaves sit between regions for trade, recruiting, and repair, with route
forks inside them.

Meta-progression widens the toolkit — new rooms, transports, route options — and never
raises baselines. A first-run tower and a fifty-run tower start identical; the veteran has
more tools, not bigger numbers. This is deliberately unbuilt until M5; deciding the
delivery mechanism (enclave gifts, Heartseed cultivars, something else) before there's a
toolkit to widen would be designing in a vacuum.

### 5.7 Crew

Jobs — haul, operate, gun, repair — assigned as role priorities per crew member,
RimWorld-lite and kept to one screen. Two needs: meals (a kitchen chain; hungry crew slow
down) and sleep (bunks plus a shift rota keyed to the day/night clock). Night operations
need light, which is a charge sink, which ties crew needs back into the charge tension in
§5.2 rather than leaving them as a separate minigame. Crew are the tower's pulse: their
commutes load the elevators, their shifts shape demand, and their stress-red queues are
part of the same bottleneck alarm that flags a starved production room. Personality lives
in barks and portraits, not in mechanics — there is no relationship system to balance
against the rest of the game.
