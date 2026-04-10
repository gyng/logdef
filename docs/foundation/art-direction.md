Project: SUPPLY LINE | Art Direction & Visual Blueprint

> **Status:** v1 and post-v1 vision. Check [v1-scope.md](v1-scope.md) before implementing any feature described here.

## I. Vision & Principles

The Core Hook: A walking fortress crosses hostile territory. You build the supply chain inside it while defending its exterior. Howl's Moving Castle meets Factorio meets Elona Shooter.

**Principle 1: The Living Machine.** The tower is alive — it walks, it hums, it has an immune system. Every visual should reinforce that this is a living organism made of wood, stone, and iron, not a static building. Gears turn, bellows breathe, the chimney smokes, the legs step.

**Principle 2: Readable Logistics.** The supply chain must be visually legible at all times. You should be able to glance at the tower cross-section and instantly read: what's producing, what's flowing, what's stuck, what's empty, what's broken. Clarity over beauty. If it looks pretty but you can't tell the fletcher is starved, it's failed.

**Principle 3: Physical Materials.** Everything is made of something — wood, stone, iron, leather, rope. No abstract UI elements floating in space. Ammo racks are wooden shelves with visible crates. Buffers are physical piles. Transport is visible machinery. The UI chrome is the tower itself.

**Principle 4: Weight and Impact.** Arrows thud. Hammers crack. Crates land with a satisfying thump. Enemies tumble and scatter. Panels crumble. The walking tower's footsteps shake the ground. Every action has physical consequence and sensory feedback.

**Principle 5: Warmth Over Grit.** This is Howl's Moving Castle, not Dark Souls. The tone is warm, characterful, slightly whimsical — even when things are going badly. Companions joke during prep. The tower sways charmingly on chicken legs. Enemies are threatening but not horrifying. The forge glows orange. The alchemist's bottles bubble in jewel colors. There's life and warmth inside the tower, danger outside.

---

## II. Visual Style: Vector-Illustrated, Flat with Depth

**Not pixel art. Not photorealistic. Vector-illustrated with a hand-crafted feel.**

Think: clean lines, limited color palettes, slightly textured fills (paper grain, canvas texture), bold silhouettes. The aesthetic sits between FTL's clean UI diagrams and Slay the Spire's illustrated card art — mechanical clarity with artistic warmth.

**When goals clash, readability wins.** If a hand-crafted texture obscures a resource type, simplify the texture. If a charming animation makes state harder to parse, reduce the motion. If parallax depth confuses floor boundaries, flatten it. Warmth and charm are load-bearing — but never at the cost of a half-second glance read.

**Key visual characteristics:**
- **Clean outlines** with consistent line weight (foreground/interactive elements: 2-3px, midground detail: 1-1.5px, background: 0.5-1px at reference resolution)
- **Flat color fills** with subtle texture overlay (paper grain, canvas weave at 5-15% opacity) — not perfectly smooth, not noisy. Texture should be visible when zoomed in but nearly imperceptible during gameplay
- **Limited palette per element** — each building, character, resource type has 3-5 colors max
- **Strong silhouettes** — every element identifiable by shape alone, no reliance on color or label
- **Warm lighting bias** — interior tower scenes lean warm (forge glow, lantern light). Exterior lean cooler (daylight, moonlight, storm gray). The contrast between warm inside and cool outside reinforces "safety inside, danger outside"
- **Slight parallax depth** — the tower cross-section has 2-3 depth layers (background wall, midground machinery/buildings, foreground runners/crates). Subtle parallax when the tower sways gives a sense of interior space without going 3D

**What to avoid:**
- Smooth modern gradients or glow effects (too "mobile game")
- Overly detailed illustration that muddies readability
- Dark/grimy/horror aesthetic — this is warm and characterful
- Photorealistic textures or 3D rendering
- Excessive UI chrome that isn't diegetic (part of the tower world)

---

## III. The Tower — Visual Identity

The tower is the protagonist. Its visual design must communicate:

**From the outside:** A ramshackle, layered, characterful walking structure. Not symmetrical. Not uniform. Each floor looks slightly different based on what's inside — the forge floor has a chimney and orange glow, the alchemist floor has colored vapor, the fletcher floor has bundled arrows visible through windows. The tower is a vertical village on legs.

**Leg types have distinct visual personalities:**
- **Chicken legs:** organic, wobbly, birdlike joints. The tower bounces slightly with each step. Feathers and scales on the legs. Charming, Ghibli-energy. The starter feel — this is YOUR weird little walking house.
- **Spider legs:** mechanical, articulated, precise. Eight legs moving in coordinated sequence. Industrial feel — rivets, pistons, hydraulic joints. The tower is becoming a machine.
- **Mechanical treads:** wide rolling tracks, grinding gears, steam vents. Heavy and reliable — a working machine, not a war machine. The tower is a land-vehicle now. Dust clouds behind. Should still feel like a home on wheels, not an armored transport.
- **Magical hover:** the legs are gone. The tower floats on a shimmering energy field. Runes glow on the underside. Magical particles drift downward. Expensive, ethereal, slightly unsettling.

**The cross-section view:**
- Each floor is a room. Walls are visible (wood/stone/iron based on floor material). Interior details match the building: fletcher has a workbench with bow staves and fletching jigs, forge has an anvil and bellows, alchemist has bubbling flasks and colored liquids.
- Transport infrastructure is mechanical and visible: stairs are wooden planks with railings, lifts have chains and platforms, chutes are angled wooden slides, dumbwaiters are pulley-operated boxes on ropes, pneumatic tubes are brass pipes with glass viewing sections where you can see items whooshing through.
- Runners are small characters with crates on their backs, visibly climbing/descending/waiting.
- Buffers/storage are physical piles: crates stacked on shelves, barrels, sacks. The fill level is the pile height. An empty buffer is bare shelves. A full buffer is a towering stack.

**Exterior fighting positions:**
- Balconies and platforms mounted on the tower's right face. Each one is a small wooden/stone ledge with a railing or parapet. The hero and companions stand on these.
- Ammo racks are visible shelves on the exterior, stacked with crates. Depletion is visible (fewer crates).
- Wall panels are the surface between floors. They crack, chip, and crumble visually as HP decreases. A fully damaged panel shows the interior through the gap.

---

## IV. Color Language

**Every resource type has a distinct color.** This is critical for logistics readability — you must be able to tell arrow crates from bolt crates from mana crystal crates at a glance.

**Color hierarchy — where semantics apply:**
Colors serve different systems, and some hues overlap (green = healthy AND uncommon, gold = currency AND legendary). The rule: **context determines which palette is active.** Resource colors apply to physical objects and crate fills. Status colors apply to glows, outlines, and ambient lighting on infrastructure. Rarity colors apply only to item card borders and loot frames — never to world objects. Terrain colors apply only to the map screen. These scopes should never bleed across boundaries. If a green potion bottle sits on a shelf with a green "healthy" glow, the glow should be a distinct warm-green aura on the shelf edge, not on the bottle itself.

| Resource | Color | Visual |
|----------|-------|--------|
| Arrows | warm brown / amber | bundled wooden shafts, feathered tips visible |
| Bolts | steel gray / blue-gray | metal rods, darker than arrows |
| Mana crystals | deep purple / violet | glowing crystals in padded crate |
| Wood | natural brown | rough planks, bark visible |
| Stone | cool gray | squared blocks, chisel marks |
| Planks | light tan | smooth finished boards |
| Fire oil | orange-red | glass bottles with flame-colored liquid |
| Potions | teal / green | small bottles, bubbling |
| Gunpowder | dark charcoal / black | barrels with yellow warning marks |
| Gold | bright gold | coins, shiny |

**Status colors:**
- Healthy/full: warm green
- Producing/active: gentle pulse/glow animation
- Starved/waiting: dim, desaturated, slight pulse to draw attention
- Damaged: amber/orange
- Critical/breach: red, cracks visible
- Empty: desaturated, bare

**Rarity colors (for loot):**
- Common: silver/gray
- Uncommon: green
- Rare: blue
- Legendary: warm gold with subtle shimmer

**Terrain colors on map:**
- Plains: warm golden grass
- Forest: deep green canopy
- Mountain: cool gray-blue rock
- Swamp: murky yellow-green
- Desert: pale sandy beige
- Storm: dark blue-gray clouds

---

## V. Character Design

**The Hero:** visible on a balcony, small enough to fit a fighting position but with a strong silhouette. Weapon is the defining visual feature — a bow character is recognizably different from a hammer character at a glance. Armor/outfit reflects class:
- Archer: light leather, quiver visible, hood
- Engineer: heavy apron, goggles, tool belt
- Commander: cape, tabard, command baton
- Scavenger: ragged layers, bags and pouches, magpie aesthetic

**Companions:** each named companion has a distinct silhouette, color accent, and visual personality. They should be identifiable by shape alone on their balcony. Examples:
- Shield-bearer: wide, stocky silhouette with large shield dominating the shape
- Archer companion: slim, tall, bow visible
- Spotter: small, perched forward, monocle or spyglass
- Engineer companion: hunched, tools hanging
- Grenadier: bulky, bandolier of explosives visible

**Runners:** small, industrious characters with crates on their backs. They should look like workers — overalls, caps, sturdy boots. When carrying, the crate is visible and clearly colored by resource type. When empty-handed, they move faster (longer stride animation). In a queue, they stand with hands on hips, impatient.

**Enemies (archetypes, specific creatures TBD):**
- Grunts: small, numerous, simple silhouette. Should read as "expendable."
- Runners (enemy): similar to grunts but leaner, legs longer, posture forward. Should read as "fast."
- Armored: large, square silhouette, heavy. Should read as "I need to shoot this a lot."
- Climbers: gecko-like posture, limbs splayed, clinging to the tower face. Should read as "on my wall."
- Flyers: wings/hovering, elevated, distinct from ground enemies immediately.
- Siege: massive, lumbering (living battering beasts, animated stone constructs). Should read as "big problem" — but natural or magical, not military hardware.
- Sappers: small and fast like grunts, but carrying tools (wrench, hammer). Should read as "heading for something specific."
- Bosses: enormous, unique, screen-filling. Each boss should be a visual event.

**Combat domesticity check:** even in the most intense encounters, the tower interior should still read as a home. Forge glow, hanging laundry, a companion's personal items on a shelf — these details shouldn't disappear when combat starts. The visual contrast between the warm, lived-in interior and the hostile exterior is the emotional engine. If a combat asset makes the tower feel like a military installation, soften it.

---

## VI. UI Philosophy: Diegetic Where Possible

**The tower IS the UI.** During combat, you're looking at the tower cross-section and the battlefield. The "HUD" should be information that's physically part of the world:

- **Ammo count:** look at the rack. Crates are physical. The visual pile IS the approximate read. Exact numbers appear only on hover/focus — small, muted, on the rack shelf. Never competing with the physical state.
- **Buffer levels:** look at the building. Pile of crates = full. Bare shelves = empty. The building's visual state IS the UI. No overlay at rest; exact values on hover.
- **Panel HP:** look at the wall. Cracks = damage. No HP bar at rest. A thin, semi-transparent HP bar fades in on hover or when the panel enters critical state.

**The diegetic rule:** physical state is the default read. Numeric overlays exist for precision but are hidden until requested (hover, focus, or critical threshold). If a team member is unsure whether to add an overlay, the answer is: make the physical state clearer first, add the number second.
- **Runner status:** look at the runners. They're carrying or idle. They're in a queue or moving. You can read logistics health by watching the people.

**Non-diegetic HUD (overlays for information that can't be physical):**
- Ability cooldown timers (circular radial on ability icons)
- Wave counter ("Wave 2/3" — small, corner)
- Gold count (small, corner)
- Companion status row (bottom — portraits with small status indicators)
- Resource priority flag indicator

**Non-diegetic should be minimal, tucked to edges, semi-transparent.** The game canvas IS the information. HUD is the footnotes.

**Prep phase UI (React overlays):**
- Clean panels that slide in from edges. Not fullscreen takeovers — the tower is always visible behind.
- Build menus, inventory, companion management panels should feel like physical documents/blueprints pulled out of a desk drawer. Paper texture, tabbed edges, ink-style text.
- Tick counter is a physical hourglass or clock in the corner.
- Map is an illustrated parchment/travel map. Paths are drawn lines. Nodes are stamped icons.

---

## VII. Animation Principles

**The tower is always in motion.** Even when stopped, something should be moving inside:
- Forge bellows breathing
- Alchemist flasks bubbling
- Fletcher's arms working
- Dumbwaiter rope creaking
- Smoke from the chimney
- Runner pacing when idle

**Locomotion animation (legs):**
- Chicken legs: bouncy, irregular, slightly comedic. 6-8 frame walk cycle. The tower body bobs up and down.
- Spider legs: precise, mechanical, synchronized. 12-16 frame cycle. Smooth and unsettling.
- Treads: continuous grinding. The tower slides forward. Dust particles behind.
- Hover: gentle float. Slight bobbing like a boat on calm water. Particles drift down.

**Combat animation priorities:**
1. Weapon fire (arrow release, bolt snap, spell cast) — must feel responsive, <100ms from input to visual
2. Enemy hit reactions (flinch, knockback, arrow stick) — immediate, crunchy
3. Enemy death (tumble, fall, dissolve) — satisfying, weapon-specific. Enemies are driven off or collapse, not brutalized
4. Panel damage (crack appears, dust falls) — visceral
5. Panel breach (crumble, enemies pour in) — dramatic
6. Runner movement (climb, carry, queue) — smooth, readable
7. Transport operation (lift moves, chute slides, pulley turns) — continuous, ambient

**Tween-based animation over frame-based.** Vector art tweens naturally — position, rotation, scale, opacity. Character animations can be skeletal (body parts move independently) rather than frame-by-frame sprite sheets. This is faster to produce AND looks smoother for vector art. Frame-by-frame only for complex special effects (explosions, magic bursts, breach crumble).

---

## VIII. Emotional Reads by Game State

Like MAVICA's archetype fantasies, each game state should have an instant emotional read:

**"My tower is healthy":** warm interior glow, steady ambient sounds, runners moving smoothly, buffers full, racks stacked. The cross-section looks ALIVE and busy. Comforting.

**"My tower is under pressure":** buffer levels dropping, rack depleting, runners rushing, amber warnings on panels. The interior sounds speed up. Slightly anxious.

**"My tower is breaking":** red cracks on panels, floors dimming (production stopped), runners stuck in queues, breach sounds from inside. Alarm. The warm interior is cooling.

**"My tower is wrecked":** silent floors, broken infrastructure visible, empty racks, few runners. The cross-section is dark and still. The tower is limping. Post-battle quiet.

**"My tower is recovering":** repair animations, buildings restarting (sounds fading back in), runners moving again, buffers slowly refilling. Warmth returning. The rebuilding soundscape.

The player should know their tower's health STATE from the visual/audio atmosphere without checking any numbers.

---

## IX. Assets Needed (by priority)

**Phase 0 (prototype):**
- Placeholder tower cross-section (simple rectangles for floors, basic colors)
- Hero silhouette on balcony
- Arrow projectile
- 2-3 enemy silhouettes (grunt, climber, flyer)
- Basic UI elements (buttons, bars, text)

**Phase 1 (logistics + readability core):**
- Production building interiors (fletcher, forge, quarry, lumberyard — 4 buildings)
- Runner character (idle, walking, carrying, queuing — 4 states)
- Crate sprites per resource type (color-coded — 6-8 types)
- Warehouse visual (ground floor, crates stacking)
- Staircase visual
- Buffer/shelf visual (empty to full)
- Core transport visuals (stairs, lifts, chutes — the logistics backbone must be readable before combat polish)
- Wall panel damage states (3-4 stages) — damage readability is as foundational as production readability

**Phase 2 (combat):**
- Hero character per class (4 classes, distinct silhouettes)
- Weapon sprites (9 base types minimum)
- Companion characters (5-6 unique named characters)
- Enemy types (8 archetypes)
- Balcony/platform exterior
- Hit effects, death effects per weapon type
- Projectile sprites per weapon type

**Phase 3 (journey):**
- Leg animations (4 types)
- Walking animation per leg type
- Terrain background art (6-7 terrain types)
- Map illustration (parchment style, terrain regions)
- Node icons (9 types)
- Merchant character + wagon

**Phase 4 (depth):**
- Remaining transport infrastructure visuals (dumbwaiters, pneumatic tubes, conveyors)
- All Tier 2/3 building interiors
- Equipment icons (weapons, trinkets — ~50 total)
- Perk tree visual
- All companion characters
- Boss designs (5+)
- Loot/reward visuals

**Phase 5 (polish):**
- Particle effects (smoke, sparks, dust, magic)
- Weather effects (rain, snow, storm, fog)
- Ambient animations (bellows, bubbles, grinding)
- UI refinement (paper texture, blueprint style, parchment map)
- Title screen / key art
- Ending illustrations

---

## X. Tone Guardrails

**Core principle: this is a home under threat, not a war machine.** Every system — combat, audio, animation, class design — must produce protective tension, not martial aggression. The tower is a village of people defending their journey home. If a feature makes the game feel like commanding an assault platform, it has drifted off tone.

These guardrails exist because individual systems, designed in isolation, naturally drift toward escalation. A class designer adds "let the tower burn" incentives. An audio designer reaches for war drums. An animator cranks screen shake to 11. Each choice is locally reasonable and globally corrosive. The Ghibli-warm, Howl's-Moving-Castle tone described in Section I doesn't survive death by a thousand martial cuts.

**Guardrail 1: Defenders, not soldiers.**
Characters on the tower are protecting a home, not waging a campaign. Their language, animations, and abilities should read as people standing their ground, not troops on the attack.
- **Do:** A companion braces behind a parapet and calls "They're coming up the east wall!" A shield-bearer steps forward to cover a breach. The archer steadies a shot from a balcony railing.
- **Avoid:** War cries, battle roars, "charge" animations. Characters should never look like they're enjoying the violence or seeking it out. No fist-pumping after kills, no taunting downed enemies.

**Guardrail 2: No incentive to let the tower suffer.**
Class designs and ability systems must never reward the player for allowing the tower to take damage. If a mechanic makes "everything is on fire" the optimal state, it converts the tower from a home worth protecting into a resource to spend. That inverts the emotional contract.
- **Do:** Berserker-style classes gain power from personal risk — the hero is exposed on a lower balcony, or has charged ahead of the defensive line. Reward proximity to danger, not tower degradation.
- **Avoid:** Abilities that scale with broken panels, burning floors, or injured runners. "Rage" meters that fill from tower damage. Any design where the player thinks "good, another wall broke" instead of "no, my wall."

**Guardrail 3: Soundscape of a hearth under siege, not a battlefield.**
Audio should reinforce the contrast between warm interior and hostile exterior. The interior ambience — forge, bubbling flasks, creaking wood, runner footsteps — should persist even during combat. Combat audio intensifies around the tower but never overwhelms the sense that there are people working inside.
- **Do:** Tension strings, low rumbling percussion, wind picking up. The tower's own sounds (groaning wood, rattling shutters) as the signature of pressure. A cracked panel lets exterior sound bleed in — that's the audio threat.
- **Avoid:** War drums, military horns, war cries, triumphant brass fanfares on wave clear. These belong to games about armies. This game is about a home. Victory audio should feel like relief and quiet pride, not conquest.

**Guardrail 4: Combat spectacle serves readability, not power fantasy.**
Hit reactions, knockbacks, and screen effects exist to communicate game state, not to make the player feel like a wrecking ball. Every combat effect should pass the question: "Does this help the player read what just happened, or does it just feel violent?"
- **Do:** Enemies flinch and stagger to show damage registered. A well-placed arrow pins an enemy to a surface briefly. Screen shake is subtle and short (2-4 frames, small amplitude) to confirm a heavy hit or panel breach. Ragdolls are minimal — enemies crumple or tumble off the tower, not launched into orbit.
- **Avoid:** "Massive" screen shake that rattles the whole cross-section. Ragdoll physics played for spectacle. Exaggerated knockback that sends enemies flying across the screen. Gore, dismemberment, or excessive particle violence. If a combat moment would look at home in a character-action game, it's too much for this tone.

**Guardrail 5: The tower always reads as architecture, not arsenal.**
Exterior fighting positions are balconies and platforms — places where people stand. They are not gun ports, turret mounts, or weapon bays. The tower's silhouette should always read as "building with people on it," not "vehicle bristling with armaments."
- **Do:** Wooden railings, hanging laundry near a balcony, a companion's personal effects visible at their station. Weapon racks look like workshop storage — tools of the trade, not an armory. Defensive additions (reinforced panels, barricades) look improvised and practical, like boarding up windows.
- **Avoid:** Symmetrical weapon arrays. Turret-like rotating mounts. Military-grade fortification aesthetic (arrow slits in uniform rows, crenellations, murder holes). If the tower exterior could be mistaken for a siege engine or warship, soften it with domestic details.

**Guardrail 6: Enemies are a threat, not a target gallery.**
Enemies should feel like a natural hazard — something the world throws at you — not cannon fodder lined up for destruction. Their deaths should register as "threat removed" not "kill scored." This keeps the emotional frame on survival rather than body count.
- **Do:** Enemies arrive in organic, uneven groups. They have self-preserving behaviors (flinching from fire, retreating when wounded). Death animations are quick and understated — collapse, slide off the tower, dissolve. The focus after a wave ends is the state of the tower, not a kill tally.
- **Avoid:** Kill counters, combo meters, or scoring systems that celebrate volume of destruction. Enemies that exist only to die in satisfying ways. Gratuitous death animations that linger. Post-wave screens that rank performance primarily by kills rather than by tower health, resources preserved, or journey progress.

---

## XI. Motivation for Future Work

When creating or updating visuals, ask first: **"Can the player read this at a glance during combat?"**

The answer must be yes. If it takes more than half a second to parse — what resource is in that crate, which floor is being attacked, where the runner is heading — the art has failed, no matter how beautiful it is.

Secondary question: **"Does this feel warm, physical, and alive?"**

The tower is a living home full of people making things and carrying things and defending each other. It should feel like a workshop, not a war machine. Cozy danger. The warmth inside makes the danger outside matter more.

Embrace: mechanical warmth, visible craftsmanship, readable logistics, Ghibli charm, physical weight, lived-in texture.
Avoid: sterile perfection, dark grit, magic without mechanism, UI that floats detached from the world, beauty at the cost of clarity.
