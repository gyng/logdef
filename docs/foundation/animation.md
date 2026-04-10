Project: SUPPLY LINE | Animation System Specification

> **Status:** v1 and post-v1 vision. Check [v1-scope.md](v1-scope.md) before implementing any feature described here.

## I. Approach: Tweens + Bones + Particles, Not Sprite Sheets

The game uses vector art (SVGs). Animation is achieved by moving, rotating, scaling, and fading static art pieces via code — not by drawing frame-by-frame sprite sheets. This means:

- **Tweens** handle most motion (position, rotation, scale, opacity over time with easing)
- **Bone rigs** handle character animation (body parts connected by joints, transforms propagate)
- **Procedural particles** handle effects (smoke, sparks, dust, magic — spawned and simulated by code)
- **State machines** govern which animation plays when (idle → walking → carrying, etc.)

No frame-by-frame sprite animation except possibly for complex VFX that can't be achieved procedurally.

**Essential motion pillars vs. full target:**
Not all animation in this document is equally load-bearing. The pillars that must ship for the game to feel right:
1. **Tower alive** — building production loops and transport motion. Without these, the tower is a static diagram.
2. **Combat readability** — weapon fire, hit reactions, enemy approach/climb. Without these, combat is illegible.
3. **Silence of absence** — when a building is destroyed or transport breaks, the animation stopping IS the feedback. This requires the above loops to exist first.

Everything else — personality idles, particle polish, boss phase transitions, secondary pendulum motion, terrain sway — is high-value enrichment that makes the game feel finished. Plan production in this order: pillars first, then layer enrichment.

---

## II. Animation Engine Architecture

### Tween system

```rust
pub struct Tween {
    pub target: TweenTarget,      // what property to animate
    pub from: f32,
    pub to: f32,
    pub duration: f32,            // seconds
    pub elapsed: f32,
    pub easing: EasingFn,
    pub state: TweenState,        // Running | Paused | Complete
    pub on_complete: Option<TweenCallback>,
    pub looping: LoopMode,        // None | Loop | PingPong
}

pub enum TweenTarget {
    PositionX(EntityId),
    PositionY(EntityId),
    Rotation(EntityId),
    ScaleX(EntityId),
    ScaleY(EntityId),
    Opacity(EntityId),
    Color(EntityId, ColorChannel),
    Custom(EntityId, PropertyId),  // for game-specific properties (buffer fill, HP bar width)
}

pub enum EasingFn {
    Linear,
    EaseIn,       // cubic-bezier(0.42, 0, 1, 1)
    EaseOut,      // cubic-bezier(0, 0, 0.58, 1)
    EaseInOut,    // cubic-bezier(0.42, 0, 0.58, 1)
    Bounce,       // overshoot and settle
    Snap,         // quick start, smooth end
    Step(u32),    // discrete steps (for "ticking" animations)
}

pub enum LoopMode {
    None,         // play once
    Loop,         // restart from beginning
    PingPong,     // forward then reverse, repeat
}
```

The tween system runs after the game simulation, before snapshot generation. It modifies visual-only properties that don't affect gameplay — positions are interpolated for display, not for collision.

**Simulation truth vs. visual expression:** Animation is always display-only. It never changes game state. When this doc describes terrain-dependent sway, runner stumbles, or chute misses in storms — those are visual expressions of mechanics that live in the simulation. The simulation decides "runner is 15% slower on mountain terrain" and "chute has a miss chance in storms." The animation system receives those facts and illustrates them. If an animation implies a gameplay consequence (a runner stumbling, a crate bouncing out of a chute), the underlying event MUST come from the simulation first. Never let animation create the impression of a mechanic that doesn't exist in the simulation layer.

### Bone system

```rust
pub struct BoneRig {
    pub root: BoneId,
    pub bones: Vec<Bone>,
}

pub struct Bone {
    pub id: BoneId,
    pub parent: Option<BoneId>,
    pub local_position: Vec2,     // offset from parent
    pub local_rotation: f32,      // radians
    pub local_scale: Vec2,
    pub sprite: Option<SpriteId>, // the art piece attached to this bone
    pub length: f32,              // for IK calculations
}
```

Bones form a tree. Transforms propagate parent → child. Animating a shoulder bone rotates the entire arm. Each bone can have its own tweens running on its local transforms.

### State machine

```rust
pub struct AnimationStateMachine {
    pub current_state: AnimStateId,
    pub states: HashMap<AnimStateId, AnimState>,
    pub transitions: Vec<AnimTransition>,
}

pub struct AnimState {
    pub id: AnimStateId,
    pub tweens: Vec<Tween>,       // tweens that play while in this state
    pub duration: Option<f32>,    // None = indefinite (until transition)
}

pub struct AnimTransition {
    pub from: AnimStateId,
    pub to: AnimStateId,
    pub condition: TransitionCondition,
    pub blend_time: f32,          // crossfade duration
}

pub enum TransitionCondition {
    GameEvent(GameEventType),     // e.g., "enemy reached position"
    StateComplete,                // current state's tweens finished
    PropertyThreshold {           // e.g., "buffer fill > 80%"
        property: PropertyId,
        threshold: f32,
        comparison: Comparison,
    },
}
```

---

## III. Entity Animation Specs

### Complexity budget per actor class

| Actor | Max bones | Max simultaneous tweens | Personality idle? | Notes |
|-------|-----------|------------------------|-------------------|-------|
| Runner | 5 | 3 | No — runners are functional, not characterful | Many on screen at once; keep cheap |
| Hero | 8 | 6 | Yes — weapon-specific idle | Only one; can afford full expressiveness |
| Companion | 6 | 4 | Yes — one signature idle per named companion | Up to 6 on screen; simpler than hero but distinct |
| Enemy (grunt/runner/sapper) | 5 | 3 | No | Many on screen; keep cheap |
| Enemy (armored/climber) | 5 | 3 | No | Same budget as grunts, different timing |
| Enemy (flyer) | 4 | 3 | No | Wings + body, minimal rig |
| Enemy (siege/catapult/ram) | 4-6 | 4 | No | Fewer on screen; can afford slightly more |
| Boss | 8-12 | 8 | Yes — unique idle per boss | Only one at a time; full budget |

These budgets are the ceiling, not the target. Start at the minimum that reads well and add bones/tweens only when a specific animation requires them.

### Runner

**Art pieces:** body (torso+legs as one piece), head, arms, crate (optional, when carrying).

**Bone rig:**
```
root (body)
  ├── head
  ├── arm_left
  ├── arm_right
  └── crate (attached to back, visible when carrying)
```

**States:**

| State | Animation | Duration | Loop |
|-------|-----------|----------|------|
| idle | Body: subtle breathing (scaleY 1.0 → 1.02, PingPong, 2s). Head: occasional look around (rotation -5° → 5°, PingPong, 3s). Arms at sides. | Indefinite | PingPong |
| walking | Body: bob up-down (posY +2px → -2px, PingPong, 0.4s). Arms: swing (rotation ±15°, PingPong, 0.4s, offset 180° from each other). Position tweens along path. | Until destination | PingPong |
| carrying | Same as walking but slower (0.6s cycle). Crate visible on back. Body leans forward (rotation 5°). | Until destination | PingPong |
| climbing_stairs | Body: step motion (posY per stair step, EaseOut, 0.3s per step). Arms: holding railing (rotation fixed). Crate bobs with each step. | Until floor reached | Loop |
| climbing_ladder | Arms alternate reaching up (rotation ±30°, 0.3s, alternating). Body slides up. Slower than stairs. Crate visible but wobbling. | Until floor reached | Loop |
| loading | Body bends forward (rotation 10°, 0.3s). Arms reach forward. Crate fades in (opacity 0→1, 0.2s). | 0.5s | None |
| unloading | Reverse of loading. Crate fades out. | 0.5s | None |
| queuing | Idle variant. Weight shifts side to side (posX ±1px, PingPong, 1.5s). Occasional impatient gesture (arm taps hip, 0.3s, every 3-5s random). | Indefinite | PingPong |
| riding_lift | Standing still on lift platform. Body sways slightly with lift motion. Arms hold rail if available. | Until lift arrives | PingPong |

**Transitions:**
- idle → walking: when assigned a destination
- walking → loading: when reaching source floor
- loading → carrying: after load complete
- carrying → climbing_stairs/ladder: when reaching transport
- carrying → riding_lift: when boarding lift
- carrying → unloading: when reaching destination
- unloading → walking: after unload (returning empty)
- any → queuing: when transport is busy
- queuing → climbing/riding: when transport frees up

### Hero

**Art pieces:** torso, head, arm_weapon (holds weapon), arm_off (off-hand), legs, weapon_sprite (changes with weapon type).

**Bone rig:**
```
root (legs, fixed to balcony position)
  └── torso
      ├── head (faces aim direction)
      ├── arm_weapon
      │   └── weapon_sprite
      └── arm_off
```

**States:**

| State | Animation | Notes |
|-------|-----------|-------|
| idle | Breathing (torso scaleY PingPong 1.0→1.02, 2s). Weapon at rest position. Head slowly tracks nearest enemy (rotation tween toward target, 0.5s). | When no enemies in range |
| aiming | Head and arm_weapon rotate to follow mouse/aim direction. Weapon points at cursor. Torso rotates slightly toward aim. Responsive — rotation tweens are fast (0.05s). | Continuous during combat |
| firing_bow | arm_weapon pulls back (rotation + position offset, 0.15s EaseIn). On release: snap forward (0.05s Snap). Weapon string visual stretches and releases. Subtle recoil on torso (rotation -2°, 0.1s, bounce back). | Per shot |
| firing_crossbow | arm_weapon braces (0.1s). Bolt fires (instant). Sharp recoil (torso rotation -3°, 0.08s, bounce). Mechanical click visual on weapon (scale pulse). | Per shot |
| firing_staff | arm_weapon raises (0.2s EaseIn). Charge glow particle at staff tip (grows 0.2s). Release: arm swings forward (0.1s), projectile spawns, glow dissipates. | Per shot |
| throwing | arm_weapon winds up behind (rotation 45°, 0.2s EaseIn). Throws: arm swings forward fast (0.08s Snap). Body twists with throw (torso rotation 10°, 0.15s, return). | Per throw |
| melee_swing | arm_weapon arcs (rotation sweep 90°, 0.2s). Body follows (torso rotation 15°, 0.2s). Return to ready (0.15s). Hit: screen shake impulse (2px, 0.1s). | Per swing |
| blocking (shield) | arm_off raises shield (rotation + position, 0.1s). Body braces (torso leans back 5°). On block: impact flash + shield pushback (3px, 0.05s, bounce). | While blocking |
| weapon_swap | Both arms lower (0.1s). Weapon sprite crossfades (old fades, new fades in, 0.15s). Arms raise with new weapon (0.1s). | On swap, total ~0.35s |
| ability_activate | Depends on ability. General: brief glow/pulse on hero (scale 1.0→1.1→1.0, 0.2s). Ability-specific particle effect at source. | Per ability use |
| rack_empty | Weapon lowers slightly. Head looks toward rack (rotation toward rack position). Slight "frustrated" body language (torso sag, scaleY 0.98). | When ammo runs out |
| melee_fallback | Quick draw dagger (arm reaches to belt, 0.15s; dagger appears in hand). Stance shifts to defensive (legs widen, torso lowers). | Auto-transition when rack empty |

### Companion

**Art pieces:** same structure as hero but simpler (fewer bones, less expressive). Each companion has unique body proportions and color scheme for silhouette distinction.

**Bone rig:** same as hero but with 1 fewer arm bone (simpler rig).

**States:** simplified subset of hero states:
- idle, aiming (auto-aim toward target), firing (weapon-type dependent), rack_empty
- No weapon swap animation (companions don't swap mid-combat)
- **Passive-specific animations:**
  - Shield-bearer: shield raise/brace animation instead of firing. Impact animation when blocking climber.
  - Spotter: brief scope/monocle glint animation when marking a target.
  - Grenadier: larger throwing animation (heavier weapon).
  - Engineer: idle includes tinkering gesture (hands fidget with tools).

**Companion personality in animation:** each named companion has a subtle idle variation — Varn adjusts his monocle, Kael shifts his shield weight, Mira tosses a bomb and catches it. These are small touches that communicate character without dialogue.

### Enemy — Grunt

**Art pieces:** body (simple blob/shape), limbs (2 arms, 2 legs, simple).

**Bone rig:**
```
root (body)
  ├── arm_left
  ├── arm_right
  ├── leg_left
  └── leg_right
```

**States:**

| State | Animation |
|-------|-----------|
| approaching | Walk cycle: legs alternate (rotation ±20°, PingPong, 0.5s). Arms swing opposite legs. Body bobs slightly. Position tweens toward tower. |
| at_base | Stops walking. Arms raise to attack position (0.3s). Begin panel attack cycle. |
| climbing | Limbs splay outward (gecko pose). Arms alternate reaching up (0.4s each). Body slides upward. Slight left-right sway. |
| attacking_panel | Arms swing forward repeatedly (0.5s cycle). Each swing triggers impact particle on panel. |
| hit_reaction | Body flinches backward (posX +3px, 0.05s, bounce back 0.1s). Opacity flash (1.0→0.7→1.0, 0.1s). Arrow/bolt sprite attaches to body at hit point. |
| dying | Body tumbles — rotation increases gently, gravity pulls down (posY accelerates downward). Fades to 50% opacity over 0.5s. Collapses on ground or slides off tower face. Not violent — more "gave up" than "destroyed." |

### Enemy — Armored

Same rig as grunt but bulkier proportions. Hit reaction is smaller (barely flinches — conveys toughness). Walking is slower (0.8s cycle). Attacking is heavier (arms swing slower, bigger impact particle).

### Enemy — Climber (fast)

Same rig but with longer limbs. Climbing animation is twice as fast as grunt (0.2s per reach). Body is leaner. Hit reaction includes "grip slip" (slides down 0.5 floors then re-grips, 0.3s).

### Enemy — Flyer (hoverer)

**Art pieces:** body, wing_left, wing_right, projectile_arm (for ranged attack).

**Bone rig:**
```
root (body, floating)
  ├── wing_left
  ├── wing_right
  └── projectile_arm
```

**States:**

| State | Animation |
|-------|-----------|
| flying | Wings flap (rotation ±30°, PingPong, 0.3s). Body bobs slightly (posY ±2px, PingPong, 0.6s). Position tweens toward tower at target height. |
| hovering | Wings flap slower (0.5s). Body near-stationary. Slight drift (posX ±1px, PingPong, 2s). |
| attacking | projectile_arm extends forward (0.2s). Flash at tip. Projectile spawns. Arm retracts (0.15s). Recoil pushback on body (posX +2px, 0.1s). |
| hit_reaction | Wings stutter (skip a flap beat). Body lurches. Arrow attaches. |
| dying | Wings stop. Body tumbles (rotation accelerates, posY accelerates down). Falls to ground with impact dust particle. |

### Enemy — Dive bomber

Same rig as hoverer but wings are swept back during dive. Dive state: body rotates nose-down (45°), wings tuck (rotation collapses), position accelerates toward target. Impact: explosion particle, wings detach (rotation spin outward), body bounces off panel.

### Enemy — Catapult

**Art pieces:** base (wheeled platform), arm (large throwing arm), projectile (boulder), crew (2-3 tiny figures).

**Bone rig:**
```
root (base, stationary)
  ├── arm (pivots on base)
  │   └── projectile (attached to arm tip)
  └── crew (background, simple idle)
```

**States:**

| State | Animation |
|-------|-----------|
| positioning | Base rolls into position (posX tween, 1s). Crew animates setup (arms move, 0.5s). |
| loading | Arm pulls back (rotation -60°, 0.8s EaseIn). Projectile appears at arm tip. Crew pulls ropes (arm animations). |
| firing | Arm releases (rotation -60° → 90°, 0.15s Snap). Projectile detaches and follows arc path (parabolic tween). Recoil: base rocks (rotation ±3°, 0.3s, damped). |
| hit_reaction | Base shudders. Crew flinches. |
| destroyed | Arm snaps (rotation wild). Base collapses (scaleY shrinks). Crew scatters (tiny figures run in random directions). Wood debris particles. |

### Enemy — Ram

**Art pieces:** body (massive beast or machine), head (battering surface), legs (4-6, heavy).

**States:**
- approaching: heavy stomping walk (0.8s cycle, screen shake on each step)
- charging: speed increases, head lowers, dust trail particles behind
- impact: head hits foundation — massive screen shake (8px, 0.3s), impact flash, dust explosion particle burst
- hit_reaction: barely flinches (conveys mass). Arrows look tiny sticking in it.
- dying: slow topple (rotation 90°, 2s). Ground shake on collapse. Dust cloud.

### Enemy — Sapper

Same rig as grunt but smaller, with tool in one hand. Climbing animation includes tool swinging. When reaching infrastructure: rapid tool animation (0.2s cycle, sparks particle from target). Distinct from grunt visually — player must learn to spot them.

### Boss enemies

Each boss has a unique rig and animation set. Detailed per boss:

**Ground boss (ram archetype):** massive, multi-limbed. Walking shakes the ground subtly (3px, per step). Has phase transitions — at 50% HP, posture changes (stands taller, different attack pattern). Transition: body shifts (scale 1.0→1.1, 0.5s, hold 0.5s, settle). The transition should feel like the creature becoming more alert, not a cinematic power-up.

**Flying boss (dragon archetype):** large wingspan. Wing flaps are slow and powerful (0.8s cycle). Strafe attack: body banks, breath/projectile sweeps across multiple floors (position tween horizontally while particles stream from mouth). Phase transition: lands (wings fold, 1s descent), stance shifts to grounded (legs extend).

**Climbing boss (giant):** enormous. Fills multiple floors visually. Each handhold is a dramatic grab (arm reaches, hand clamps, body pulls up, 1.5s per floor). Panel at each floor visibly strains when giant grips it (panel deforms, cracks radiate from grip point).

---

## IV. Tower & Infrastructure Animation

### Building production

Each building has a "working" animation loop that plays while producing:

| Building | Working animation |
|----------|-------------------|
| Fletcher | Arm bone moves back and forth (shaving motion, 0.6s loop). Tiny arrow sprites accumulate in output area. |
| Forge | Bellows compress/expand (scaleX PingPong 0.8→1.2, 1s). Anvil: arm swings down periodically (0.4s). Orange glow pulses (opacity PingPong 0.6→1.0, 1.5s). |
| Quarry | Pickaxe arm swings (rotation 0→-30°, 0.5s loop). Rock fragments particle from impact point. |
| Lumberyard | Saw arm moves back-forth (posX PingPong, 0.6s). Wood chips particle. |
| Sawmill | Circular saw element rotates (continuous rotation, 1 revolution per 0.5s). Plank slides through (posX tween). |
| Alchemist | Flask bubbles (small circle particles rising from flask, continuous). Liquid color shifts (hue tween, 3s loop). Occasional steam puff (opacity particle, 0.5s). |
| Weaponsmith | Similar to forge but lighter, more precise hammering (0.3s cycle). Grinding wheel rotates (continuous). |
| Enchanter | Crystal at center pulses (scale PingPong 0.9→1.1, 2s). Rune particles orbit crystal (circular path, 3s loop). Ambient glow (opacity PingPong 0.5→0.8, 2s, offset from crystal pulse). |
| Siege Works | Large arm swings hammer (slow, 1.2s cycle). Wood frame creaks (subtle rotation PingPong ±1°, 2s). |
| Artificer | Small mechanical ticking (tiny arm moves rapidly, 0.15s loop). Occasional spark (particle). Spring tension (element compresses/releases, 0.8s). |

**Building state visual changes:**

| State | Visual |
|-------|--------|
| Producing (active) | Working animation plays. Normal brightness. |
| Full buffer (idle) | Working animation stops. Worker idle pose (arms at sides, occasional idle gesture). Output area visually full (crates stacked high). |
| Starved (waiting for input) | Working animation stops. Worker looks around (head rotation PingPong, 2s). Input area visibly empty. Slight dim (opacity 0.85). Pulsing dim (opacity PingPong 0.8→0.9, 1.5s) to draw eye. |
| Destroyed | Building sprites are scattered/broken (rotation offsets, position offsets). Smoke/dust particles. Dark. No animation. |

### Buffer / crate visualization

Crates in buffers, caches, racks, and warehouse are physical sprites that stack:

- **Adding a crate:** crate sprite slides into position (posX/Y tween, 0.2s EaseOut) and settles (subtle bounce, 0.1s).
- **Removing a crate:** crate sprite lifts out (posY -3px, 0.1s) then moves toward whoever took it (runner, companion). Brief "grab" particle (small flash).
- **Buffer warning (nearly full):** top crate pulses glow (opacity PingPong, 1s). Visual: "we're almost full."
- **Buffer empty:** empty shelf/rack visible. Subtle shadow where crates used to be. The absence IS the visual.

### Transport animation

| Transport | Animation |
|-----------|-----------|
| Built-in stairs | No ambient animation. Animate only when runner uses them (footstep dust particles, stair creaks — audio only). |
| Ladder | Slight sway when runner climbs (rotation PingPong ±2°, 0.5s). |
| Dumbwaiter | Platform moves up/down (posY tween between floors, speed based on dumbwaiter speed stat). Rope/chain moves with platform (texture scroll or position tween). Crate visible on platform. Pulley wheel at top rotates (continuous during movement). |
| Chute | Crate slides down (posY tween, fast, EaseIn). Brief dust/spark particle at entry and exit points. The crate is visible sliding through the chute's transparent section. |
| Cargo lift | Platform moves (posY tween). Chains move (texture scroll). Runner visible standing on platform with crate. Arrival: platform decelerates (EaseOut), subtle thud (posY overshoot 1px, bounce). |
| Express lift | Same as cargo lift but faster tween speed. Blur lines while moving (stretch effect: scaleY increases slightly during motion). |
| Conveyor | Belt segments move continuously (texture scroll or repeating position tween). Crates ride along (posX tween matching belt speed). |
| Pneumatic tube | Item shoots through (very fast posY tween). Visible through glass viewing sections (item sprite appears briefly in each section). Entry/exit: puff particle (air burst). |

**Transport breakdown:** when a lift/dumbwaiter breaks, the animation stutters (tween pauses, jitters, then stops). The element visually "locks up" — chains freeze, platform stops mid-floor. Spark particle at the break point. Visual: something is clearly wrong.

### Tower walking (legs)

**Chicken legs:**
```
Bone rig:
  tower_body
    ├── leg_left
    │   ├── thigh
    │   ├── shin
    │   └── foot
    └── leg_right
        ├── thigh
        ├── shin
        └── foot
```

Walk cycle (1s per stride):
- Leg alternation: left lifts while right pushes, then swap
- Thigh rotation: ±25° (PingPong, offset between legs)
- Shin rotation: follows thigh with slight delay (secondary motion)
- Foot: maintains ground contact via simple IK (foot always points down relative to ground)
- Tower body: bobs up-down (posY ±3px, following leg cycle). Slight tilt (rotation ±2°) matching stance leg.
- Personality: slightly irregular timing (not perfectly metronomic — add ±5% random variation to stride timing)

**Spider legs:**
8 legs, each a 2-bone chain (upper + lower). Walk cycle is precise and synchronized:
- Legs move in groups of 4 (alternating sets), 0.3s per step
- Tower body stays level (minimal bob — posY ±1px). This communicates stability.
- Mechanical feel: each step is exactly timed, no variation

**Mechanical treads:**
- Tread texture scrolls continuously (texture UV offset tween, speed proportional to tower speed)
- Body vibrates slightly (posX ±0.5px, fast PingPong, 0.05s — machinery shake)
- Exhaust particles from rear (small puffs, continuous)
- Road/ground particles underneath (dust/gravel kicked up)

**Magical hover:**
- No legs. Tower floats above ground.
- Gentle bobbing (posY ±2px, PingPong, 3s — slow, dreamy)
- Magical particles drift downward from tower base (continuous, slow, ethereal)
- Rune glow on underside pulses (opacity PingPong, 2s)
- Ground beneath has a subtle glow/shadow (projection effect)

### Tower sway (terrain-dependent)

On rough terrain, the tower sways more:

| Terrain | Sway amplitude | Sway period | Effect on internals |
|---------|---------------|-------------|---------------------|
| Plains | ±1° rotation | 4s | None |
| Forest | ±2° rotation | 3s | None |
| Mountain | ±4° rotation | 2s | Runner speed penalty visual (stumble animation every ~5s) |
| Swamp | ±3° rotation, irregular | 2.5s | Slight sink (posY -2px, slow) |
| Storm | ±5° rotation, gusting | 1.5s (irregular) | Chute items can miss (visual: item bounces out of chute entrance) |

Sway applies to the entire tower body. Interior elements (runners, crates, buildings) move WITH the tower. But loose items (hanging signs, chains, dangling tools) have secondary motion — they sway OPPOSITE to the tower with a slight delay (pendulum effect). This sells the physical weight of the tower.

---

## V. Combat Visual Effects

### Projectile trails

| Weapon type | Trail |
|-------------|-------|
| Bow/arrow | Subtle white streak behind arrow (fading opacity line, ~2-3 frames). Arrow sprite rotates to match arc (rotation = velocity angle). |
| Crossbow/bolt | Sharper, shorter trail (~1 frame). Bolt doesn't rotate (flat trajectory, always pointing forward). |
| Staff/magic | Colored glow trail matching element (fire = orange, frost = blue, arcane = purple). Trail persists slightly longer (~4-5 frames, wider). Particles along path. |
| Thrown/javelin | No trail. Object rotates end-over-end (continuous rotation during flight). |
| Thrown/bomb | Short fuse particle (spark, trailing). Bomb tumbles (rotation). |
| Gun/bullet | Nearly invisible trail (single-frame flash line). Impact is the main visual (spark, dust). |
| Whip | Whip sprite extends from arm to target (bezier curve tween, 0.05s). Snaps back (reverse, 0.05s). Crack particle at tip. |

### Hit effects

| Surface | Effect |
|---------|--------|
| Flesh (unarmored enemy) | Blood-free impact: dust puff + stagger animation on enemy. Arrow/bolt attaches to body at hit point (random slight offset for variety). |
| Armor (armored enemy) | Spark particle + metallic flash (white opacity spike, 0.05s). Arrow bounces off (if deflected) or embeds at angle. Smaller stagger. |
| Panel (wall) | Stone/wood chips particle burst (direction away from impact). Small crack appears at impact point (sprite overlay, persists). Dust puff. |
| Ground (miss) | Dirt puff. Arrow/bolt sticks in ground (stays as visual debris for 2s, then fades). |
| Shield (blocked) | Large flash, ring particle (expanding circle, 0.2s). Blocked projectile reflects away (rotation spin, posY arc upward then down). |

### Kill effects (weapon-dependent)

**Tone check:** enemies are driven off or collapse, not brutalized. Kill animations should feel like "threat removed" not "enemy destroyed." The tower is a home being protected, not a war machine scoring kills. Keep deaths quick, clean, and slightly understated.

| Weapon | Kill animation |
|--------|---------------|
| Arrow | Enemy stumbles backward (gentle impulse in arrow direction). Body collapses. Arrows remain stuck. Settles on ground. |
| Bolt | Enemy staggers (sharp pushback, 0.05s, then slumps forward for 0.3s before collapsing). Bolt protrudes visibly. |
| Staff | Enemy dissolves: scale shrinks (1.0→0.3, 0.3s) while opacity drops and particle burst (colored sparkles expanding outward). Body fades away. |
| Thrown/javelin | Enemy knocked back (moderate impulse). Javelin protrudes. |
| Thrown/bomb | Explosion: expanding circle (scale 0→2.0, 0.2s, fading opacity). Screen shake (3px, 0.1s). Debris particles. Nearby enemies stagger. |
| Melee/sword | Slash arc visual (curved line, 0.1s, fading). Enemy knocked sideways off tower face (if climber) or tumbles backward (if ground). |
| Melee/hammer | Impact: screen shake (4px, 0.15s). Enemy knocked back. Crack particle at impact. |
| Whip | Enemy pulled off tower face (if climber) — slides toward whip user, then falls. |

### Critical hit

All critical hits add a bonus layer on top of the normal kill/hit effect:
- Screen freeze: 2-frame pause (33ms at 30hz — subtle but noticeable "hit stop")
- Flash: bright white overlay on the enemy (opacity 0.8, 0.05s, then fade)
- Particle: sharp starburst at impact point (white/gold, 0.15s)
- Camera: very subtle zoom pulse (scale 1.0→1.01→1.0, 0.15s) — barely perceptible but adds weight

### Screen shake

Used sparingly for high-impact events. Shake is applied to the camera, not individual elements.

| Trigger | Amplitude | Duration | Frequency |
|---------|-----------|----------|-----------|
| Normal hit | None | — | — |
| Critical hit | 2px | 0.05s | 60hz |
| Panel breach | 6px | 0.3s | 30hz (heavy) |
| Foundation hit | 6px | 0.3s | 20hz (deep, slow) |
| Bomb explosion | 3px | 0.1s | 40hz |
| Boss footstep | 2px | 0.1s | 30hz |
| Ram impact | 8px | 0.4s | 15hz (massive, slow) |

Shake decays exponentially (amplitude × e^(-t/decay)). Multiple shakes stack additively (capped at 10px). Shake direction is random per frame within amplitude.

**Shake restraint:** these values are tuning seeds, not commitments — test in context and reduce if cumulative shake during busy encounters becomes fatiguing. The tower is a home, and excessive shaking undermines the feeling of stability. A player should feel "that was a big hit" from shake, not "the screen won't stop moving." If in doubt, halve the amplitude.

---

## VI. UI Animation

### Panel transitions (React, CSS)

| Transition | Animation | Duration | Easing |
|------------|-----------|----------|--------|
| Panel slide in (from left) | translateX(-100% → 0) | 200ms | decelerate |
| Panel slide out | translateX(0 → -100%) | 150ms | accelerate |
| Overlay fade in | opacity 0→1, scale 0.95→1.0 | 200ms | smooth |
| Overlay fade out | opacity 1→0, scale 1.0→0.95 | 150ms | smooth |
| Tooltip appear | opacity 0→1, translateY(4px→0) | 120ms | snap |
| Tooltip disappear | opacity 1→0 | 80ms | linear |

### Tick spending

When a tick is spent:
- Tick counter number rolls down (translateY animation, 150ms)
- Brief pulse on the counter (scale 1.0→1.15→1.0, 200ms, bounce)
- Subtle clock tick particle (small gear or hourglass sand grain, at counter position)

### Loot reveal (card flip)

Slay the Spire style:
1. Card starts face-down (backside visible). Slight wobble (rotation ±2°, 0.3s).
2. Card flips (rotateY 0→180°, 0.4s, with scale squeeze at midpoint: scaleX 1.0→0.1→1.0). Rarity glow appears during flip.
3. Card lands face-up. Rarity-dependent particle burst:
   - Common: nothing
   - Uncommon: green sparkle (few particles)
   - Rare: blue shimmer (ring of particles)
   - Legendary: gold explosion (many particles, screen glow pulse, dramatic)

### Ability cooldown

Circular radial wipe on ability icon:
- On use: icon grays out. Radial overlay (dark, clockwise sweep from 100%→0% over cooldown duration).
- Ready: overlay disappears. Icon returns to full color. Brief glow pulse (0.3s). Subtle particle effect (ready sparkle).

### Damage alert

"FLOOR 4 CRITICAL" text:
- Appears with scale 1.2→1.0 (bounce, 0.15s) and opacity 0→1
- Holds for 1.5s
- Fades out (opacity 1→0, 0.3s)
- Red tint pulse on the edge of the screen nearest the damaged floor (0.5s, fading)

---

## VII. Particle System

Procedural particle system — no sprite sheets for particles. Each particle is a simple shape (circle, square, triangle, line) with properties:

```rust
pub struct Particle {
    pub position: Vec2,
    pub velocity: Vec2,
    pub acceleration: Vec2,     // gravity, wind
    pub rotation: f32,
    pub rotation_speed: f32,
    pub scale: f32,
    pub scale_rate: f32,        // grow or shrink over lifetime
    pub color: Color,
    pub color_end: Color,       // lerp toward this over lifetime
    pub opacity: f32,
    pub opacity_rate: f32,      // fade per second
    pub lifetime: f32,
    pub elapsed: f32,
    pub shape: ParticleShape,   // Circle | Square | Triangle | Line
}

pub struct ParticleEmitter {
    pub position: Vec2,
    pub rate: f32,              // particles per second
    pub burst: Option<u32>,     // one-time burst count
    pub particle_template: ParticleTemplate,
    pub randomization: ParticleRandomization,  // variance in velocity, lifetime, color, etc.
    pub active: bool,
}
```

### Particle presets

| Name | Shape | Color | Behavior | Used for |
|------|-------|-------|----------|----------|
| dust_puff | Circle | Tan→transparent | Expand + fade, 0.3s | Footsteps, impacts, landings |
| wood_chips | Square | Brown variations | Launch outward + gravity, 0.5s | Wood panel damage, building destruction |
| stone_chips | Triangle | Gray variations | Launch outward + gravity, 0.5s | Stone panel damage |
| sparks | Line | Orange→yellow | Launch upward, fast, 0.2s | Metal impacts, forge, transport breakdown |
| smoke | Circle (large) | Dark gray→transparent | Rise slowly, expand, 0.8s-2s | Chimney, fire, destruction aftermath |
| magic_glow | Circle (soft) | Spell color→transparent | Drift, slow fade, 0.5s | Staff attacks, enchanter ambient, rune effects |
| blood_free_impact | Circle (small) | White→transparent | Burst outward, 0.15s | Hit impacts (no actual blood — stylized) |
| fire | Circle | Orange→red→transparent | Rise, flicker (random scale), 0.4s | Flaming modifier, fire flasks, forge |
| frost | Square (tiny) | Light blue→white→transparent | Drift downward slowly, 0.6s | Frost modifier, frost enemies |
| loot_sparkle | Circle (tiny) | Gold/white | Rise + drift, 0.3s | Loot drops, rare item reveals |
| enchantment | Circle (soft, large) | Teal→transparent | Expand from center, 0.5s | Tower expelling raiders, enchanted floor effect |

### Particle budget

Maximum 200 particles on screen simultaneously. If limit reached, oldest particles despawn first. Most effects use 5-20 particles. A large explosion might use 30-40. Ambient effects (smoke, enchanter glow) use 3-5 continuously.

---

## VIII. Animation Priority & Performance

When the frame budget is tight, animation quality degrades gracefully:

| Priority | What | Degradation |
|----------|------|-------------|
| 1 (never cut) | Hero animations, projectile positions | Always full quality |
| 2 (reduce last) | Enemy hit reactions, kill effects | Reduce particle count |
| 3 (reduce early) | Building production loops, transport ambient | Reduce to simpler idle (no secondary motion) |
| 4 (cut first) | Stuck arrows on enemies, resource flow particles, loot sparkles | Remove entirely |

Particle system respects frame budget — if particles are causing slowdown, emission rate halves automatically. If still slow, particle lifetime halves (they disappear faster, fewer on screen).

Bone rig evaluation: if many characters are on screen, off-screen or distant characters skip bone evaluation and render at their last computed pose. Only characters within the player's immediate view get full bone animation updates.

---

## IX. AI Animation Generation Spec

This section specifies how to generate animation assets with AI tools so that results are consistent, layerable, and match the game's tween-based architecture. Unlike audio or static art, animation generation requires decomposing motion into parameterized primitives that the engine can drive — AI tools generate the component art and motion descriptions, not baked frame sequences.

### What AI generates vs. what the engine owns

| AI generates | Engine owns |
|-------------|-------------|
| SVG body part art (torso, head, arm, leg, weapon, crate) | Bone rig assembly and hierarchy |
| Suggested tween parameters (duration, easing, amplitude) | Actual tween execution and state machine transitions |
| Particle shape sprites (circle, shard, puff, spark) | Particle emitter behavior, lifetime, physics |
| Color palettes per entity | Runtime color application and state-based shifts |

**AI should never generate spritesheet frame sequences.** The engine uses tweens and bones. AI output must be decomposed static parts that the engine animates programmatically. A "walking runner" from AI is 4 SVG pieces (body, head, arm_left, arm_right) plus a motion description — not 8 frames of a walk cycle.

### SVG part generation constraints

All character art is decomposed into bone-attachable parts. Each part is a separate SVG.

| Constraint | Value | Why |
|-----------|-------|-----|
| Art style | Vector, flat color, clean outlines per art-direction.md | Consistency with visual identity |
| Outline weight | Match art-direction.md ranges (2-3px foreground, 1-1.5px detail) | Cross-entity consistency |
| Pivot point | Marked in SVG as a named element or metadata. Every part needs a defined rotation origin | Bone attachment requires knowing where the joint is |
| Neutral pose | All parts generated in a neutral/rest pose (arms down, legs straight, weapon at side) | Engine applies all rotation/position from neutral |
| Color count | 3-5 colors per part, from the entity's palette | Matches art direction's limited palette rule |
| Canvas size | Consistent per actor class (e.g., runner parts on 64×64, hero parts on 96×96) | Parts must scale consistently when assembled |
| Symmetry | Left/right limbs generated as mirrored copies with separate SVG files | Engine needs independent control per bone |

### Prompt template for character parts

```
ENTITY: [runner / hero_archer / companion_varn / grunt / etc.]
PART: [torso / head / arm_left / arm_right / leg_left / leg_right / weapon / crate]
POSE: neutral (rest position — arms at sides, legs straight, weapon lowered)
STYLE: vector illustration, flat color with 5-15% paper texture, clean outline
  (foreground weight: 2-3px, detail weight: 1-1.5px)
PALETTE: [3-5 specific hex colors from entity's color spec]
PIVOT: [description — e.g., "shoulder joint at top-center of arm piece"]
CANVAS: [size]px square, transparent background
SILHOUETTE: must be identifiable by shape alone at 50% scale
WARMTH: this is a Ghibli-warm game, not grimdark. Even enemies should feel
  like creatures, not monsters. Rounded shapes, slight asymmetry, handcrafted feel.
AVOID: sharp edges, horror aesthetic, photorealism, gradients, glow effects,
  perfectly symmetric shapes (slight imperfection = life)
```

**Example — Runner torso:**
```
ENTITY: runner (logistics worker inside the tower)
PART: torso (includes chest and hips as one piece)
POSE: neutral, upright, slight forward lean suggesting readiness
STYLE: vector, flat warm brown overalls with cream shirt underneath,
  clean outline 2px, paper texture at 10% opacity
PALETTE: #8B6914 (overalls), #F5E6C8 (shirt), #5C4A1E (belt/straps),
  #3D2B0F (outline)
PIVOT: center-top (neck attachment point for head bone)
CANVAS: 64×64px, transparent background
SILHOUETTE: stocky, slightly hunched, clearly a worker not a fighter
AVOID: military clothing, armor, weapons, anything that reads "soldier"
```

### Motion description format

For each animation state, provide a structured motion description that translates directly to tween parameters. AI can suggest these; the engine implements them.

```
ENTITY: runner
STATE: walking
DURATION: 0.4s per cycle
LOOP: PingPong

BONES:
  body:
    property: positionY
    from: 0px (relative to rest)
    to: -2px
    easing: EaseInOut
    note: gentle bob, not bouncy

  arm_left:
    property: rotation
    from: -15°
    to: +15°
    easing: EaseInOut
    phase_offset: 0° (starts at back swing)

  arm_right:
    property: rotation
    from: +15°
    to: -15°
    easing: EaseInOut
    phase_offset: 180° (opposite arm)

FEEL: purposeful, steady, a worker on their route — not hurried unless carrying
```

### Particle sprite generation

Particle shapes are simple SVGs that the engine colors, scales, rotates, and fades programmatically.

| Particle type | SVG shape | Size | Colors (AI generates neutral; engine recolors) |
|--------------|-----------|------|-----------------------------------------------|
| dust_puff | Soft circle, slightly irregular edge | 8×8px | Warm tan base |
| wood_chip | Small irregular rectangle | 4×6px | Brown base |
| stone_chip | Small irregular triangle | 4×4px | Gray base |
| spark | Thin line with bright center | 2×8px | Orange-white base |
| smoke | Large soft circle, very irregular edge | 16×16px | Medium gray base |
| magic_glow | Soft circle, perfectly smooth | 12×12px | White base (engine applies spell color) |
| snowflake/frost | Tiny square, slight rotation | 3×3px | Light blue base |
| fire_lick | Teardrop shape, pointed top | 6×10px | Orange base |
| loot_sparkle | 4-point star | 6×6px | Gold base |

Generate each as a simple SVG with the base color. The engine handles: color tinting per context, opacity animation, scale animation, rotation, velocity, and lifetime. AI-generated particle sprites should be maximally simple — complexity comes from the emitter behavior, not the particle shape.

### Batch generation strategy

Like audio, generate in families for consistency:

1. **All parts for one entity in one session.** Generate runner body, head, arms, crate together so proportions, line weight, and color treatment match. Then assemble and verify the bone rig looks correct at neutral pose.
2. **All enemies in one session.** Grunt, armored, climber, sapper should share a visual family — similar construction, different proportions. Generate together so silhouettes are distinct but stylistically cohesive.
3. **All particle sprites in one session.** They're simple shapes, but they need consistent line weight (or no line) and compatible scale.
4. **All building worker sprites in one session.** The fletcher worker, forge worker, alchemist, etc. should look like the same species of NPC.

### Naming convention

```
{entity}_{part}_{variant}_v{version}.svg
```

Examples:
- `runner_torso_default_v2.svg`
- `hero_archer_arm_weapon_default_v1.svg`
- `grunt_body_default_v3.svg`
- `particle_dust_puff_v1.svg`
- `companion_varn_head_default_v1.svg`

### Post-processing checklist

After AI generation, verify each part against:

- [ ] Outline weight matches art direction ranges
- [ ] Pivot point is marked or documented
- [ ] Neutral pose is genuinely neutral (no pre-baked rotation or lean that fights the engine)
- [ ] Color count is within 3-5 per part
- [ ] Transparent background, no baked shadows
- [ ] Silhouette test: recognizable at 50% scale in grayscale
- [ ] Assembly test: all parts for an entity compose correctly at neutral pose
- [ ] No baked animation frames — this is a static part, not a sequence
