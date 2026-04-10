Project: SUPPLY LINE | Controls & Input Specification

> **Status:** v1 and post-v1 vision. Check [v1-scope.md](v1-scope.md) before implementing any feature described here.

## I. Input Targets

**Primary: mouse + keyboard.** Desktop browser and Tauri. Mouse for aiming, keyboard for hotkeys.

**Future: gamepad.** Not in v1. The input layer is abstract (all inputs mapped through an `InputAction` enum), which helps but doesn't make gamepad trivial. Several current interactions are deeply mouse-native: bow arc preview following cursor, gun recoil cursor displacement, drag-and-drop companion assignment, and precise click targeting in dense prep panels. Gamepad support will require aim assist tuning, a cursor-substitute for prep navigation, and likely some interaction redesign — not just a button mapping pass.

```rust
pub enum InputAction {
    // Combat
    AimAt(Vec2),          // mouse position → world direction
    PrimaryFire,          // LMB / trigger
    PrimaryFireRelease,   // LMB release (for draw-release weapons)
    SecondaryAction,      // RMB / shoulder button (weapon-specific)
    WeaponAbility,        // Q / face button
    HeroSkill,            // E / face button
    SwapWeapon,           // Tab / bumper
    SetPriority,          // F / d-pad (cycle priority resource)

    // Prep
    Click(Vec2),          // LMB on UI / tower
    RightClick(Vec2),     // RMB for tooltip/context
    DragStart(Vec2),
    DragMove(Vec2),
    DragEnd(Vec2),
    OpenMap,              // M
    OpenInventory,        // I
    OpenCompanions,       // C
    OpenHero,             // H
    March,                // Enter
    Cancel,               // Escape

    // System
    Pause,                // Escape (combat) / P
    ToggleDebug,          // F1-F10 (dev only)
}
```

---

## II. Combat Controls — Weapon-Dependent Input Models

Each weapon type has its own input feel. The mouse does different things depending on what the hero is holding.

**Shared foundation, distinct feel:** all weapons share the same core verbs — LMB to attack, RMB for secondary, Q for ability. The distinction is in *how* LMB behaves (hold-release vs. click vs. hold-continuous), not in learning entirely new controls. A player who has mastered the bow already knows the input vocabulary; swapping to crossbow changes the rhythm, not the grammar. If onboarding testing reveals too much friction between weapon types, the first simplification should be collapsing hold-release and click weapons into a single "click to fire" model with optional hold-for-aim, rather than adding tutorials per weapon.

### Bow — draw, aim, release

```
LMB press:   begin drawing bow (string pulls back, arrow appears)
LMB hold:    aim freely (trajectory arc preview shown while holding)
LMB release: fire arrow along aimed trajectory

Longer hold does NOT increase damage — it's purely aim time.
The arc preview disappears once released.
Minimum draw time: 0.15s (prevents accidental taps from firing)
```

**Feel:** each shot is a mini commitment. You draw, settle your aim, release. The draw-hold-release rhythm is THE bow experience. Skilled players draw quickly and release accurately. New players hold longer to line up the arc. The trajectory preview teaches the arc physics.

**Arc preview:** a dotted line showing the projectile path from the hero's position through the cursor. Updates every frame while holding. Shows where the arrow will go including gravity drop. The preview fades out at max range. Color: white, semi-transparent, thin.

### Crossbow — click to fire, auto-reload

```
LMB click:   fire bolt instantly (pre-loaded, no draw)
             reload begins automatically (visual: crank animation)
             cannot fire again until reload completes
LMB hold:    no effect (single shot per click)

Reload time: weapon-dependent (hand crossbow: 0.5s, heavy: 1.5s)
```

**Feel:** instant gratification on click. Then patience during reload. You aim during the reload and fire the instant it's ready. The rhythm is: BANG-wait-aim-BANG-wait-aim. The wait makes each shot feel more impactful.

**Reload indicator:** small circular progress bar near the crossbow, fills during reload. When full, brief glow = ready to fire.

### Staff / Wand — hold to channel

```
LMB hold:    continuous fire at weapon's fire rate
             mana drains while firing
LMB release: stop firing

Fire rate is automatic while holding — you just aim.
```

**Feel:** the "garden hose." Point and spray. The easiest aim model — just track targets with your cursor while holding LMB. The cost is in mana crystals, not skill. Stopping releases the button. You're managing your mana budget by controlling how long you hold.

**Visual:** continuous beam or rapid projectile stream (depends on staff sub-type). Wand = rapid small projectiles. Staff = slower but with AoE splash. Scepter = slow heavy shots. Orb = homing stream.

### Thrown — click to throw

```
LMB click:   throw one projectile at cursor position
             throwing animation plays (0.2-0.3s)
             can click again after animation completes
LMB hold:    no effect (one throw per click)
```

**Feel:** quick, responsive, chunky. Each click = one throw. The fire rate is limited by the animation, not by input. Rapid clicking = rapid throwing (up to the animation cap). Javelins feel heavy and slow. Throwing knives feel fast and light. Bombs feel weighty with a satisfying arc.

**Aim:** thrown weapons use a landing indicator instead of an arc preview — a circle/X on the ground showing where the projectile will land (for bombs and flasks). Javelins show a short arc line.

### Melee — click to swing

```
LMB click:   swing weapon once
             swing arc covers the weapon's reach zone
             cannot swing again until animation completes
LMB hold:    no effect (one swing per click)

Timing matters — swing when a climber is in range.
```

**Feel:** deliberate, weighty, committal. Each click is one attack. You choose WHEN to strike. The swing animation has a brief wind-up (0.1s) and a longer recovery (0.2-0.3s). Button mashing is slower than timed swings because you can't swing until the recovery ends. Skilled melee players time their swings to hit climbers the instant they enter range.

**Reach indicator:** a subtle zone highlight showing the weapon's reach area (floors covered). Enemies within the zone are highlighted. Helps the player learn melee range without guessing.

### Gun — click to fire, recoil recovery

```
LMB click:   fire gun (instant, loud)
             recoil kicks aim direction (cursor offset up/sideways)
             aim drifts back to center over 0.3-0.5s
             can fire again anytime (but accuracy suffers if you don't let recoil settle)
LMB hold:    no effect (one shot per click)
```

**Feel:** the skill IS recoil control. Click, BANG, aim jumps. You re-center and click again. Fast clicking = lots of shots but wild accuracy (wasted ammo). Slow, paced clicking = every shot lands. The optimal rhythm is weapon-specific: pistol is fast, musket is very slow (huge recoil, long settle time).

**Recoil visual:** the cursor itself kicks on fire (jumps in the recoil direction, then smoothly drifts back to the mouse position). The hero's arm also kicks. Crosshair widens during recoil (accuracy cone expands), tightens as it settles.

### Whip — click to lash

```
LMB click:   lash whip in aim direction
             whip extends instantly, hits everything in arc
             brief recovery before next lash (0.4-0.6s)
LMB hold:    no effect (one lash per click)
```

**Feel:** snappy, instant feedback. Click and a crack. The whip is the fastest visual response of any weapon — the lash extends in one frame. But the recovery prevents spam. Each lash is a deliberate choice.

**Aim:** the whip hits in a cone/arc toward the cursor. Not a precise single-target — it sweeps the area. The cursor determines the center of the arc, not a specific target.

### Instrument — FUTURE WORK

Instruments are deferred to a post-v1 update. The rhythm-based input model (BPM timing, combo crescendos, tempo per instrument type) is a fundamentally different input paradigm from the aim-based weapons and adds significant implementation complexity (beat tracking, timing windows, audio sync, unique UI indicator). The design vision is preserved in full below for when this becomes active work, but instruments should not block v1 weapon implementation or balance.

<details>
<summary>Instrument design (deferred — click to expand)</summary>

Instruments don't use the standard fire model. They have their own input system based on timed beats.

```
Game provides:   a tempo (BPM) with a visual/audio pulse
LMB click:       play a note/beat

On-beat (±100ms):   full power pulse. 100% effect. Strong visual + sound.
Close (±200ms):     partial power. 60% effect. Weaker visual.
Off-beat (>200ms):  whiff. No effect. Mana still consumed. Discordant sound.
Perfect (±30ms):    150% power. Bonus visual (golden note, sparkle).

No hold mechanic. Each click is one beat.
Stop clicking = stop playing.
```

**Combo system:** consecutive on-beat hits build a combo counter. At 4 consecutive beats, you hit a **crescendo** — a free super-powered pulse unique to the instrument type. Combo resets on any off-beat. The crescendo replaces the weapon ability (Q) — instruments don't have a separate Q ability. The crescendo IS the ability, earned through rhythm mastery.

**Tempo per instrument:**

| Instrument | BPM | Beat interval | Feel | Crescendo (at 4 combo) |
|------------|-----|---------------|------|------------------------|
| War drum | 80 | 0.75s | Slow, heavy, deliberate | Massive shockwave: damage + stun all enemies on screen |
| Battle horn | 120 | 0.5s | Martial, driving | All companions +100% fire rate for 5 seconds |
| Lute | 160 | 0.375s | Fast, melodic, demanding | Charm 2 enemies simultaneously (fight for you, 10s) |
| Bell | 60 | 1.0s | Very slow, massive tolls | Triple-damage toll + all panels regenerate 5 HP |

**Rhythm indicator:** minimal, non-intrusive. A pulsing ring around the hero that expands on each beat. Click when the ring reaches a target size.

```
  ○  ring starts small
  ◯  ring growing
  ◎  click HERE (on the beat line) — full effect
  ○  ring passed, shrinks, next beat starts
```

As the player internalizes the tempo, they stop looking at the indicator and play by feel. The AUDIO is the real guide — the tower's ambient sounds synchronize with the instrument's tempo, and on-beat hits harmonize while off-beat hits sound discordant.

**Why rhythm works:**
- Completely different skill axis from aiming (bow = spatial skill, instrument = temporal skill)
- A player bad at aiming can excel at rhythm
- The instrument's sound integrates with the tower's soundscape (good playing = harmony, bad = discord)
- Crescendo earned through skill replaces the cooldown-based weapon ability
- Mana consumed per beat regardless of timing — bad rhythm = wasted mana (same ammo economy as other weapons)

**Visual:** each on-beat hit produces a ring/wave/note particle radiating from the hero. The battlefield during instrument play looks completely different from bow play — musical energy instead of arrows. Combo counter visible as small glowing notes stacking (1, 2, 3, CRESCENDO flash).

</details>

### Shield — hold to block, click to counter

```
LMB hold:    raise shield (blocking stance)
             blocks incoming projectiles toward your position
             cannot attack while blocking
LMB release: lower shield (return to normal)

RMB click:   counter-attack (while shield is raised)
             reflects blocked projectile or shield bash

If no shield raised, RMB does nothing.
```

**Feel:** the anti-shooter. You're not attacking, you're defending. Holding the shield is a commitment — you can't shoot while blocking. The skill is timing: when to block (incoming flyer projectile) and when to lower (gap between attacks). Counter-attacking with RMB during a block is satisfying — turning their attack back on them.

**Visual:** shield raises in front of hero. Incoming projectiles that would hit you hit the shield instead (impact particle on shield). RMB counter: shield glows, reflected projectile is highlighted, flies back.

---

## III. Weapon Ability & Hero Skill

Two active abilities with dedicated keys:

### Weapon ability (Q)

```
Q press:     activate weapon's unique ability
             consumes ammo (amount depends on ability)
             enters cooldown (weapon-dependent, shown on HUD)

Some abilities are instant (Snipe, Cleave)
Some require aim (Charged Shot — Q to activate, then aim + LMB to fire)
Some are toggle (Full Burst — Q to start, auto-fires until magazine empty)
```

**Activated abilities (Q → instant effect):** Snipe, Cleave, Nova, Barrage, Shield Bash, Thunderclap, Shockwave. Press Q, effect happens immediately in current aim direction.

**Aimed abilities (Q → aim → LMB):** Charged Shot (hold Q to charge, LMB to release), Meteor (Q to summon, cursor shows landing zone, LMB to confirm), Skyfall (Q to fire upward, click landing zone). After pressing Q, a special aim indicator appears. LMB confirms. Escape cancels.

**Sustained abilities (Q → runs until finished):** Full Burst (fires all bolts rapidly), Lash Storm (whip strikes for 3s), Rapid Fire (5 arrows fast). Q starts it, it runs for its duration, no further input needed. You can still aim normally during it.

### Hero skill (E)

```
E press:     activate hero's chosen skill
             instant effect
             enters cooldown (longer than weapon ability)
```

All hero skills are instant — press E, effect happens. Focus Fire (2x fire rate starts), War Cry (companions volley), Called Shot (next shot crits), Fortify (defense buff). No aiming required. One keypress.

---

## IV. Weapon Swap

```
Tab press:   swap to other weapon slot
             brief animation (0.35s for Quick Draw perk, 0.5s otherwise)
             weapon ability changes to new weapon's ability
             rack resource type auto-reconfigures if cache supports both types

Scroll wheel: same as Tab (swap weapons)
```

**Feel:** the swap should feel decisive, not fiddly. Tab → brief visual swap → you're in a different mode. Bow to sword: you go from ranged to melee. Crossbow to staff: you go from single-shot to sustained fire. The swap animation is short enough to be reactive (climber reaches you, Tab to melee) but long enough to be a commitment (can't strobe between weapons).

---

## V. Priority Flag (mid-combat)

```
F press:     cycle priority resource type
             small icon appears near tower base showing current priority
             runners adjust behavior to favor prioritized resource
             icon fades after 3 seconds
```

One keypress, no menu, no mode change. Press F, priority cycles: arrows → bolts → mana → stone → (none) → arrows. The current priority shows as a colored resource icon at the tower base. Runners respond by weighting that resource's cache delivery higher.

This is the ONLY logistics interaction during combat besides aiming and abilities.

---

## VI. Prep Phase Controls

Prep is mouse-driven with keyboard shortcuts. No combat inputs active during prep.

### Mouse

```
LMB click:      select floor / building / balcony / companion / transport
                 click action buttons in detail panels
                 click map nodes
LMB drag:        drag companion portraits onto balcony slots
                 drag slider handles (lift floor range)
RMB click:       open tooltip/context info for any element
                 close current detail panel (if one is open)
Scroll wheel:    no function (tower always fits on screen, no scrolling)
```

### Keyboard shortcuts

| Key | Action | Context |
|-----|--------|---------|
| M | Open/close map | Anytime during prep |
| I | Open/close inventory | Anytime during prep |
| C | Open/close companion roster | Anytime during prep |
| H | Open/close hero stats/perks | Anytime during prep |
| Enter | March (depart to next node) | When map node selected |
| Escape | Close current panel / cancel action | Anytime |
| 1-8 | Select floor by number | Quick floor selection |
| R | Repair selected floor | When a damaged floor is selected |
| B | Build menu for selected floor | When a floor is selected |
| U | Upgrade selected building | When a building is selected |

### Interaction patterns

**Click to select → detail panel → click action:**
1. Click floor 5 in the tower cross-section
2. Floor 5 highlights, detail panel opens at bottom
3. Panel shows: building info, cache, panel HP, actions
4. Click "Build Cache" → cache placed, tick decrements
5. Click floor 3 → detail panel switches to floor 3
6. Click empty space → detail panel closes

**Drag to assign companion:**
1. Click a companion portrait in the roster panel (or on a balcony)
2. Drag to a balcony slot in the cross-section
3. Release → companion assigned to that position
4. If slot occupied → companions swap positions

**Right-click for details:**
- Right-click any building → full production details (input/output rates, operating cost, buffer state)
- Right-click any transport → throughput stats, congestion info
- Right-click any companion → full stats, combat history, equipment
- Right-click any enemy on the map → archetype info (if Architect's Lens reveals them)

---

## VII. Map Controls

Map is a full-screen overlay with its own controls:

```
Hover:           highlight node, show detail panel at bottom
LMB click:       select node as next destination
                 path highlights, unreachable nodes dim
Enter:           confirm selected node (same as "Select This Path" button)
RMB / Escape:    close map, return to prep
```

Primarily mouse-driven. Keyboard support within the map: arrow keys to move between connected nodes, Enter to confirm selection, Escape to close. The map is a visual puzzle you solve by hovering, reading, and clicking — but keyboard navigation exists for accessibility.

---

## VIII. Input Abstraction Layer

All raw input goes through an abstraction layer before reaching game logic. This enables future gamepad support and keybinding customization.

```rust
pub struct InputState {
    // Raw (from winit/browser events)
    pub mouse_position: Vec2,     // screen coordinates
    pub mouse_world: Vec2,        // converted to game world coordinates
    pub mouse_buttons: MouseButtons,
    pub keys_down: HashSet<KeyCode>,
    pub keys_just_pressed: HashSet<KeyCode>,
    pub keys_just_released: HashSet<KeyCode>,

    // Processed (mapped to game actions)
    pub actions: Vec<InputAction>,
}

pub struct InputMapper {
    pub bindings: HashMap<RawInput, InputAction>,
    // Default bindings, customizable in settings
}

impl InputMapper {
    pub fn process(&self, raw: &RawInputState) -> Vec<InputAction> {
        // Map raw keys/mouse to InputActions
        // Handle weapon-dependent LMB behavior
        // Handle press/hold/release distinction
    }
}
```

**Weapon-dependent LMB mapping:** the InputMapper knows the current weapon type and maps LMB press/hold/release to the correct InputAction for that weapon. The game logic never checks "is LMB down?" — it checks "is PrimaryFire active?" The mapper does the translation.

```rust
// Example: bow LMB mapping
// All weapons use the same InputAction variants — the mapper translates
// raw mouse events into weapon-agnostic actions. Game logic reads
// PrimaryFire/PrimaryFireRelease and applies weapon-specific behavior.
match weapon_type {
    Bow => {
        if lmb_just_pressed { emit(InputAction::PrimaryFire) }       // begin draw
        if lmb_held { emit(InputAction::AimAt(mouse_world)) }
        if lmb_just_released { emit(InputAction::PrimaryFireRelease) } // release arrow
    }
    Crossbow => {
        if lmb_just_pressed { emit(InputAction::PrimaryFire) }
        // hold does nothing, release does nothing
    }
    Staff => {
        if lmb_held { emit(InputAction::PrimaryFire) }
        if lmb_just_released { emit(InputAction::PrimaryFireRelease) }
    }
    // ...
}
```

### Keybinding customization

All bindings are stored in a config file and editable in settings. Players can remap any key except mouse aiming (always mouse position).

```json
{
    "weapon_ability": "Q",
    "hero_skill": "E",
    "swap_weapon": "Tab",
    "priority_flag": "F",
    "open_map": "M",
    "open_inventory": "I",
    "open_companions": "C",
    "open_hero": "H",
    "march": "Enter",
    "cancel": "Escape",
    "floor_1": "1",
    "floor_2": "2",
    ...
}
```

### Future gamepad mapping

When gamepad support is added, the same `InputAction` enum is used:

| Gamepad input | InputAction |
|---------------|-------------|
| Right stick | AimAt (converted to direction) |
| Right trigger | PrimaryFire / hold |
| Right trigger release | PrimaryFireRelease |
| Left trigger | SecondaryAction (block/counter) |
| Face button A | WeaponAbility |
| Face button B | HeroSkill |
| Left bumper | SwapWeapon |
| Right bumper | SetPriority |
| D-pad | Floor selection (prep) |
| Start | Pause |

Aim assist would be increased for gamepad (higher Precision stat effect, wider hit zones) to compensate for stick vs. mouse precision. This is a presentation-layer change, not a simulation change.

---

## IX. Input Responsiveness Targets

| Action | Maximum input latency | Notes |
|--------|----------------------|-------|
| Aim tracking (mouse → crosshair) | < 1 frame (< 7ms at 144fps) | Crosshair follows mouse at render rate, not sim rate |
| Fire/shoot (click → projectile spawns) | < 1 sim tick (< 33ms at 30hz) | Processed at next simulation tick |
| Weapon ability (Q → effect) | < 1 sim tick (< 33ms) | Same as fire |
| Weapon swap (Tab → new weapon visible) | < 1 sim tick + animation (33ms + 350-500ms) | Swap animation is intentional delay |
| UI interaction (click → panel response) | < 1 frame (< 7ms) | React handles UI at render rate |
| Prep action (build → visual result) | < 100ms | Command sent, validated, applied, re-rendered |

**Aim tracking is special.** The crosshair/cursor follows the mouse at the RENDER rate (144hz), even though shots fire at the SIM rate (30hz). This means aiming feels perfectly responsive — no input lag on cursor movement. The slight delay between click and projectile spawn (up to 33ms) is imperceptible for this game's pace.

---

## X. Accessibility

### Remappable controls
All keyboard bindings customizable. No hardcoded keys except mouse aim.

### Aim assist options (settings)
- **Trajectory preview** (default on for bows, toggleable for all weapons): shows where the projectile will go
- **Target snap** (optional): when aiming near an enemy, cursor subtly pulls toward the nearest target. Adjustable strength (off / light / medium / strong).
- **Auto-fire** (optional): hold LMB to auto-fire at the weapon's fire rate for ALL weapon types (overrides the per-weapon input model). Accessibility option for players who can't click rapidly. **Note:** this collapses weapon identity significantly — bow draw-release, crossbow click-wait, and gun recoil-recovery all become "hold to shoot." That's the correct tradeoff for accessibility, but balance should assume the default (non-auto-fire) input model. Auto-fire may need slightly reduced fire rates or accuracy to compensate for the ergonomic advantage it provides.

### Prep phase accessibility
- All prep actions achievable with keyboard only:
  - **Tab / Shift+Tab:** cycle focus between interactive elements (floors, action buttons, panel controls). Focus order: tower floors top-to-bottom → detail panel actions → resource panel → top bar buttons.
  - **1-8:** jump focus directly to a floor.
  - **Enter:** activate the focused element (same as LMB click).
  - **Arrow keys:** navigate within a panel (e.g., between build menu options, companion list entries, inventory items).
  - **Escape:** close current panel, return focus to tower.
  - Companion assignment via keyboard: focus a companion in the roster, press Enter → floor selector activates, use 1-8 to pick floor, Enter to confirm.
- Tooltip on every interactive element
- High-contrast mode for UI elements (in settings)

### Color-blind support
- Resource types identifiable by icon shape, not just color
- Enemy types identifiable by silhouette, not just color
- Status indicators have text labels, not just color

---

## XI. AI Implementation Spec

This section structures the controls system so AI coding tools can implement weapon input models, the input abstraction layer, and prep interactions with minimal ambiguity.

### Core architecture principle

The input system has exactly three layers. AI implementations must maintain this separation:

```
Layer 1: Raw Input      → browser/platform events (MouseDown, KeyPress, etc.)
Layer 2: InputMapper     → translates raw events into InputAction variants
                           using current weapon type + game phase as context
Layer 3: Game Logic      → consumes InputAction variants only, never raw events
```

**Hard rule:** game logic (Layer 3) must NEVER reference mouse buttons, key codes, or platform events. If a new weapon needs a new input behavior, it is expressed by how the InputMapper translates raw events into existing InputAction variants — not by adding raw input checks to game logic.

### Weapon input model table

Each weapon type maps the same raw inputs (LMB press/hold/release) to different InputAction sequences. This table is the complete specification for the InputMapper's weapon-dependent behavior:

| Weapon | LMB press | LMB hold | LMB release | RMB | Notes |
|--------|-----------|----------|-------------|-----|-------|
| Bow | `PrimaryFire` (begin draw) | `AimAt(pos)` continuously | `PrimaryFireRelease` (fire) | — | Min draw time 0.15s; release before min = cancel, not fire |
| Crossbow | `PrimaryFire` (fire instantly) | No action | No action | — | Cannot fire again until reload completes (game logic enforces) |
| Staff | `PrimaryFire` continuously at fire rate | Same (hold = sustained fire) | `PrimaryFireRelease` (stop) | — | Fire rate gated by game logic, not input |
| Thrown | `PrimaryFire` (throw one) | No action | No action | — | Rate limited by animation duration (game logic) |
| Melee | `PrimaryFire` (swing once) | No action | No action | — | Rate limited by animation recovery (game logic) |
| Gun | `PrimaryFire` (fire once) | No action | No action | — | Recoil cursor displacement is visual-only; game logic applies accuracy penalty |
| Whip | `PrimaryFire` (lash once) | No action | No action | — | Rate limited by recovery (game logic) |
| Shield | `PrimaryFire` (raise shield) | Shield stays raised | `PrimaryFireRelease` (lower) | `SecondaryAction` (counter) | RMB only active while shield is raised |

**For AI implementers:** implement the InputMapper as a `match` on `(weapon_type, raw_event)` that emits `Vec<InputAction>`. Each row above is one match arm. The game logic handler for `PrimaryFire` then dispatches on weapon type for behavior.

### State machine per weapon

Each weapon's combat behavior is a small state machine. AI should implement these as explicit enums, not ad-hoc boolean flags:

```
Bow:       Idle → Drawing(elapsed) → Aimed → [on release] Firing → Idle
Crossbow:  Loaded → [on fire] Firing → Reloading(elapsed) → Loaded
Staff:     Idle → Channeling → [on release] Idle
Thrown:    Ready → [on fire] Throwing(anim_elapsed) → Ready
Melee:     Ready → [on fire] Swinging(anim_elapsed) → Recovering(elapsed) → Ready
Gun:       Ready → [on fire] Fired → Recoiling(elapsed) → Ready
Whip:      Ready → [on fire] Lashing(anim_elapsed) → Recovering(elapsed) → Ready
Shield:    Lowered → [on hold] Raised → [on RMB] Countering → Raised
                                      → [on release] Lowered
```

**For AI implementers:** each weapon state machine is a `match` on `(current_state, input_action)` → `(new_state, side_effects)`. Side effects are: spawn projectile, apply damage, start animation, consume ammo, enter cooldown. Keep these state machines small (3-5 states max per weapon).

### Prep input routing

Prep phase input is simpler — no weapon-dependent behavior. The routing rule:

```
1. If a panel is open, input routes to the panel (click = panel interaction)
2. If no panel is open, input routes to the tower cross-section (click = select floor)
3. Keyboard shortcuts are global (M, I, C, H work regardless of panel state)
4. Escape always closes the topmost panel
5. Number keys (1-8) always select a floor, even if a panel is open
```

**For AI implementers:** implement prep input as a priority stack. The topmost open panel captures click/drag events. Keyboard shortcuts bypass the stack. This avoids the common bug where clicks "fall through" a panel to the tower behind it.

### Test matrix

When implementing or modifying weapon input, verify against this matrix:

| Test | Expected |
|------|----------|
| Tap LMB with bow (< 0.15s) | No arrow fires (below min draw time) |
| Hold LMB with crossbow | Only one bolt fires (hold has no effect) |
| Rapid-click crossbow during reload | No extra bolts (reload blocks fire) |
| Hold LMB with staff, release, hold again | Fire starts, stops, starts — no queued shots |
| Tab to swap weapon mid-draw (bow) | Draw cancels, no arrow fires, swap begins |
| Q during weapon swap animation | Queued and fires after swap completes, OR rejected (decide and document) |
| F in prep phase | No effect (priority flag is combat-only) |
| Click tower floor while inventory panel is open | Click goes to inventory panel, not tower |
| Escape with nested panels (inventory → equip) | Closes equip first, then inventory on second press |

### Error handling

| Situation | Behavior |
|-----------|----------|
| Fire when no ammo | Weapon click sound (dry fire), no projectile, no cooldown consumed |
| Q ability when on cooldown | Brief flash on cooldown indicator, no effect |
| Q ability when no ammo for ability cost | Same as no ammo — dry click, no effect |
| Click "Build" when insufficient ticks | Button stays grayed, click produces error sound, no tick consumed |
| March with validation warnings | Warnings flash, "March Anyway" appears, player confirms or fixes |
