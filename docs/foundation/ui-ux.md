Project: SUPPLY LINE | UI/UX Specification

> **Status:** This document describes v1 only.

## I. Philosophy

The game has two distinct UI modes with different densities:

**Combat (encounter phase):** Minimal, diegetic by default, abstracted when timing-sensitive decisions need precision. The game canvas IS the primary information — tower cross-section shows logistics visually, panel cracks show damage, arrows sticking in enemies show HP. But when a player needs exact values under time pressure (ability cooldowns, ammo counts, companion status), clean non-diegetic overlays are correct. The rule: diegetic for state awareness (glance and know roughly), abstracted for decision-critical precision (I need to know exactly). Your eyes should be on the battlefield 95% of the time.

**Prep (travel phase):** Rich, interactive, information-dense. The UI IS the game. You're making decisions, configuring, comparing. Think Into the Breach deployment screen — show everything, hide nothing, keep it clean.

### Reference games
- **Into the Breach** — show everything, hide nothing, keep it clean. Damage previews, enemy intentions visible. The gold standard for management UI clarity.
- **Slay the Spire** — clean card/relic/map UI. Information hierarchy is clear. Tooltips well-organized.
- **FTL** — ship cross-section IS the game. Crew are visible dots. Systems show power. This is what the tower cross-section should feel like.
- **Elona Shooter** — almost no UI during combat. Health bar, ammo, that's it.

### Rules
- **No modals during combat. Ever.** Combat is real-time. Nothing pauses or obscures the action.
- **No scrolling for spatial content.** The tower cross-section always fits on one screen — the player must see all floors simultaneously. For list-based content (inventory, weapon comparison, sell lists), constrained scrolling within a fixed-height panel is fine when filter/sort alone can't reduce the list to screen height. The principle: spatial layouts don't scroll, data lists can.
- **Spatial continuity, not literal persistence.** The tower should always be reachable within one action (close panel, press Escape). Transitions between views use slides, fades, or pans — never hard cuts. But "tower always visible" means the player never loses their sense of place, not that the tower cross-section must be literally rendered behind every overlay. Full-attention panels (map, perk tree, inventory) can dim or minimize the tower when the panel itself is the decision space.
- **Click to select → detail panel → click action.** No menus deeper than 2 levels. Click away to close.
- **Configuration is always free.** Equipping gear, changing orders, configuring caches/lifts costs zero ticks. Only physical construction costs ticks.

---

## II. Phase Transitions

The game has a heartbeat: fight → breathe → plan → march → fight. Each transition is smooth, no cuts.

### Encounter → Post-Combat
1. Final enemy dies. 1 second of stillness — the "did we survive?" beat.
2. Companions lower weapons.
3. If breaches happened: interior raider timer plays out (15-20 seconds). Unskippable on first occurrence per run. After the player has seen one full breach sequence, subsequent breaches can be fast-forwarded (click or spacebar to 2× speed). The first time must land emotionally; repetition shouldn't become friction.
4. Screen dims slightly. Post-combat summary slides up from bottom — NOT a full takeover. Tower visible behind, showing current damage.
5. Summary shows: kills, gold earned, accuracy %, ammo used, damage taken. Companion speech bubble with comment. All visible at once — no sequential reveals for stats.
6. Loot presented simultaneously as cards (Slay the Spire reward style). Click to take (→ inventory) or skip per item. No forced sequential reveal — show all loot cards at once, let the player flip/take at their pace.
7. "Continue" button at bottom. Alternatively, pressing Enter/Space skips straight to prep if no loot is pending.

**Pacing note:** This sequence has 5-7 beats before the player regains full control. In an encounter-heavy run (30+ encounters), that accumulates. The first 5 encounters should play every beat at full length — the rhythm is being established. After encounter 5, compress: stillness beat shortens to 0.5s, companion weapon-lower is instant, summary appears faster. The emotional beats (breach, loot) keep their weight; the connective tissue speeds up.

### Post-Combat → Prep
1. Click "Continue." Summary slides down.
2. Tower becomes fully interactive. Tick counter appears. Resource summary appears.
3. Damage visible — cracked panels, dimmed floors, empty racks.
4. Companion speech bubbles pop up with suggestions: "Floor 3 needs repairs." "I saw a merchant on the northern path."
5. No animation — UI shifts from read-only to interactive.

### Prep → Map
1. Click "MAP" (top bar) or press M.
2. Map slides in as full-screen overlay. Tower dims behind but remains faintly visible.
3. Full chapter map populated. Current node glows.
4. Browse, hover, compare, select next node.
5. Close (Escape or X) returns to prep. Map remembers selection.

### Prep → March
1. Click "MARCH" button (bottom-right). Shows next node icon + terrain.
2. If validation warnings: flash briefly (2 seconds). "March Anyway" or fix.
3. Tower stands up — leg animation (chicken legs unfold, spider legs extend, treads engage). 2-3 seconds.
4. Landscape scrolls. Tower walks. 3-5 second atmospheric segment. Terrain changes visible.
5. Tower arrives. Planting animation (legs settle).
6. Node type determines what happens next:
   - **Combat:** enemies appear at right at max range. "ENCOUNTER" text flash. Combat begins immediately.
   - **Merchant:** merchant wagon walks up alongside. Dialogue bubble. Shop opens.
   - **Rest:** calm music shift. Rest UI appears.
   - **Mystery:** "?" appears, resolves into event with brief narrative text.
   - **Boss:** boss-specific entrance animation. Boss health bar appears. Combat begins.
   - **Recruitment:** settlement visible in background. Character walks to tower base. Recruitment panel opens.
   - **Forge site:** resource pickup animation. Material added to warehouse.

---

## III. Combat HUD

### Layout

```
┌──────────────────────────────────────────────────────────┐
│ WAVE 2/3 ▪▪▪▪░░░  [12 remaining]              ◉ 245g   │
│                                                          │
│                                                          │
│                                                          │
│     [tower cross-section]       [battlefield]            │
│     left third of screen        right two-thirds         │
│                                                          │
│                                                          │
│                                                          │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │ [Q] Charged Shot 8s  🏹🏹🏹░ 3/5   [E] Focus 12s │  │
│  │ ████████░░░░░░░░░░   arrows        ██░░░░░░░░░░░░ │  │
│  │                                                    │  │
│  │ [Varn ▪🦅▪██░] [Kael ▪🛡▪n/a] [Mira ▪⚔▪███]     │  │
│  └────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────┘
```

### Top bar (tiny, corner text)
- **Wave counter:** "WAVE 2/3" + thin progress bar (enemies remaining)
- **Enemy count:** just a number
- **Gold:** coin icon + number

### Bottom bar (compact, always visible)
- **Ability slots (left + right):** Weapon ability (Q hotkey) and hero skill (E hotkey). Each shows: icon, name, cooldown bar. Filling = on cooldown. Full + glow = ready. Grayed = no ammo for weapon ability.
- **Ammo rack (center):** Visual crate icons matching diegetic rack. 3 crate-shaped icons, filled/empty. Below: resource type icon + "3/5" text. This mirrors the physical rack on the balcony — same information, two places.
- **Companion row (bottom):** One entry per companion. Each: small portrait, targeting order icon, ammo vertical bar. Pulses when companion is in trouble (low ammo, panel at their position critical). Click does nothing during combat — read-only. (Portrait size TBD in implementation — small enough to fit 6 companions without crowding, large enough to identify at a glance.)

### What is NOT in the HUD
- Panel HP — read the cracks visually
- Runner status — see them in the cross-section
- Building production — see buffer piles visually
- Regular enemy HP — see arrows sticking in them
- Bosses: exception, get a health bar at the top of the screen

### Damage alerts
Brief flash near the affected floor: "FLOOR 4 CRITICAL" in red, fades after 2 seconds. The only proactive HUD notification. Means "something is about to breach."

### Priority flag (mid-combat)
Press Tab to cycle resource priority. Small icon near warehouse showing prioritized resource type. Fades after 3 seconds. One keypress, no menu. This is the ONLY combat interaction besides aim/shoot/ability.

---

## IV. Prep Phase

### Default view

```
┌──────────────────────────────────────────────────────────┐
│  SUPPLY LINE    ⏱ 7/10 ticks    ◉ 342g    [MAP][COMP][⚙]│
│                                                          │
│  ┌──────────────────────────────────────────────────┐    │
│  │ F6 [Alchemist ████] |chute| |     | ⌂ Varn      │    │
│  │ F5 [Enchanter  ██░] |     | |lift | ⌂ HERO      │    │
│  │ F4 [Quarry    ████] |stair| |lift | ⌂ Kael      │    │
│  │ F3 [Fletcher  ███░] |stair| |     | ⌂ Mira      │    │
│  │ F2 [Forge      ██░] |stair| |     | ⌂ ---       │    │
│  │ F1 [Quarters    ▪▪] |stair| |     |             │    │
│  │ G  [====WAREHOUSE====] |legs|                    │    │
│  └──────────────────────────────────────────────────┘    │
│                                                          │
│  ┌────────────┐  ┌──────────────────────────────────┐    │
│  │ Resources  │  │  Selected: Floor 5               │    │
│  │ 🏹 24      │  │  Building: Enchanter (T3)        │    │
│  │ ⚙ 18      │  │  Inputs: arrows + mana crystals  │    │
│  │ 💎 8       │  │  Output buffer: ██░ 2/3          │    │
│  │ 🪵 12      │  │  Cache: mana [██░] 2/3           │    │
│  │ 🪨 6       │  │  Panel HP: ████████░░ 82%        │    │
│  │            │  │  Occupant: HERO (bow)             │    │
│  │ Total wage:│  │                                   │    │
│  │ 42g/enc    │  │  [Upgrade 2⏱+materials]           │    │
│  │ Oper cost: │  │  [Repair 1⏱+stone]                │    │
│  │ 28g/enc    │  │  [Build Cache 1⏱+planks]          │    │
│  └────────────┘  └──────────────────────────────────┘    │
│                                                          │
│  ⚠ Floor 7 has no transport │ ⚠ Hero rack mismatch      │
│                                                          │
│  Next: ☠☠ Elite [forest]                    [MARCH ▶]   │
└──────────────────────────────────────────────────────────┘
```

### Tower cross-section (center)
- All floors visible simultaneously. Never scrolls.
- Each floor row shows: floor number, building (with buffer fill), transport infrastructure passing through, balcony occupant (if any).
- Click any floor to select it — highlights, detail panel updates.
- Buildings show buffer as inline fill bar. Full = bright, producing = animated, starved = dim pulse, offline = dark.
- Transport shows as icons in dedicated columns (stairs always visible, lift/chute in separate column).
- Balconies show occupant name. Hero position has a crown icon. Empty balconies show "---".

### Resource panel (bottom-left)
- Grid of ResourceCount molecules. One row per resource type. Shows current warehouse stock.
- Below: total recurring costs (wages + operating costs) per encounter. Player can see their burn rate at a glance.

### Floor detail panel (bottom-right)
- Appears when a floor is selected. Shows everything about that floor.
- Building info: type, tier, inputs, outputs, buffer level, operating cost.
- Cache info: resource type, fill level. Clickable to change resource type (free).
- Panel HP: bar with percentage. Color-coded (green/amber/red).
- Occupant: who's on the balcony, their weapon type, cache compatibility check.
- Action buttons: each shows tick cost + material cost. Grayed if unaffordable. Click to execute immediately.

### Interaction flows

**Build a building:**
1. Click empty floor → detail panel shows "Empty Floor"
2. Panel shows "Build" button → click opens build menu
3. Build menu: filtered list of available buildings for this tier. Each entry: name, what it produces/consumes, tick cost, material cost. Grayed if unaffordable.
4. Click a building → it's built. Tick counter decrements. Building appears on floor.

**Build transport:**
1. Click between two floors (or click a floor and select "Add Transport")
2. Transport menu: available types (ladder, dumbwaiter, chute, lift). Each shows: tick cost, material cost, floor width consumed.
3. Click to build. For lifts: lift programmer opens immediately after building (configure floor range, car count, departure mode — free).

**Assign companion to position:**
1. Click an empty balcony slot → companion selector appears
2. Shows available companions (not assigned elsewhere). Portrait + name + passive + weapon type.
3. Click to assign. Companion appears on balcony.
4. OR: drag companion portrait from roster panel onto balcony slot.

**Configure cache:**
1. Click an existing cache in the floor detail → resource type dropdown
2. Select resource type. Instant, free.
3. If no cache exists: "Build Cache" button in floor detail (1 tick + planks).

**Configure lift:**
1. Click a lift in the cross-section → lift programmer panel
2. Floor range: drag endpoints or click numbers
3. Car count: 1/2/3 buttons
4. Departure mode: Immediate / Batch toggle
5. All changes free. Close panel when done.

**Repair:**
1. Damaged panels show amber/red in the cross-section. Cracked visual.
2. Click damaged floor → detail panel shows "Repair" with tick + material cost.
3. Click "Repair" → panel restored. Tick decrements.
4. OR: RepairQueue (accessible from top bar) shows all damaged floors sorted by severity. One-click repair from the queue.

### Tick spending feedback
- When a tick is spent: counter in top bar rolls down with animation + subtle clock tick sound.
- The action result is immediate — building appears, transport installs, panel repairs.
- No "apply" or "confirm" step. Actions are instant.
- No undo within prep phase — but you can rearrange (costs ticks) or demolish (costs 1 tick, no material refund). Demolish is not free: free demolish enables exploit-y rebuild loops where players tear down and reconstruct buildings to game buffer states or reposition production at no cost. 1 tick makes it a real decision.

### Validation warnings
- Yellow bar above the "March" button.
- Each warning is one line: icon + description. Clickable — jumps to the problem floor.
- Examples: "Floor 7 has no transport connection," "Hero rack mismatch — weapon uses arrows, cache has mana," "Enchanter has no input supply."
- Warnings are not blocking — click "March" to depart anyway.

### The "March" button
- Bottom-right, always visible during prep.
- Shows: next node icon + node name + terrain type.
- Click to depart.

---

## V. Map

### Layout

Full-screen overlay on top of dimmed tower view.

```
┌──────────────────────────────────────────────────────────┐
│  Chapter 3: The Frontier                        [CLOSE] │
│                                                          │
│      ☠──[plains]──── ☠☠──[forest]──── 💰                │
│     /                                   \                │
│    ●                                     ♛               │
│     \                                   /                │
│      ☠──[swamp]────  ⛺──[plains]──── ☠☠                │
│                                                          │
│    ● current   ☠ combat   ☠☠ elite   💰 merchant        │
│    ⛺ rest      ♛ boss     ? mystery  👤 companion       │
│                                                          │
│  ┌──────────────────────────────────────────────────┐    │
│  │  Hovering: Elite Combat (☠☠)                     │    │
│  │  Difficulty: ●●●     Modifier: "Frenzied"        │    │
│  │  Terrain: [forest] Reduced visibility at range    │    │
│  │  Leg penalty: none (spider legs)                  │    │
│  │  Prep ticks: 6 (standard)                         │    │
│  │  Rewards: Rare+ loot guaranteed, 1.5x gold        │    │
│  └──────────────────────────────────────────────────┘    │
│                                                          │
│  💬 Varn: "The northern forest is thick with flyers."    │
│                                                          │
│  [SELECT THIS PATH]                                      │
└──────────────────────────────────────────────────────────┘
```

### Visual style
- Parchment/paper texture background. Illustrated map style.
- Path lines colored and textured by terrain: plains = golden, forest = dark green, mountain = gray jagged, swamp = murky green, storm = dark blue with lightning marks.
- Nodes are stamped icons (skull, coin, tent, crown, etc.) with ink-style strokes.
- Visited paths are solid lines. Future paths are dotted. Current node glows with a warm pulse.
- Boss node at chapter end is always visible, larger than other nodes.

### Interactions

**Hover a node:** Detail panel appears at bottom. Shows: type, difficulty, terrain modifier, rewards, leg penalty for the path terrain, prep ticks for that path distance. Panel updates instantly on hover — rapid comparison by flicking between nodes.

**Click a node:** Selects it as next destination. Path from current node highlights. Nodes on other branches that become unreachable dim (but don't disappear — you can still read what you're giving up). Click a different node to change selection.

**Path terrain indicators:** Path lines show terrain type through color/texture. If your legs have a penalty for that terrain, a small ⚠ icon appears on the path segment. Hover ⚠ for details ("Chicken legs on swamp: -15% runner speed").

**Companion hints:** Speech bubbles from companions appear near the map edges. Contextual: "I've heard the northern pass has flyers" (if the northern path has air-heavy encounters). Hints are based on the next 1-2 nodes, not the full path — the game helps you without fully spoiling.

**Select path button:** Bottom of screen. Confirms your next node choice. Returns to prep view with the destination shown. You can reopen the map to change your mind before marching.

**Pre-chapter moment:** When entering a new chapter, the FULL chapter map is revealed at once. Brief "unveiling" animation — the parchment unrolls or fades in region by region. You survey everything before your first step. Boss identity is visible at the end. This is the strategic planning moment.

---

## VI. Merchant

### Layout

Shop panel slides in from the right. Tower visible (dimmed) on the left. Merchant character visible in their wagon alongside the tower base.

```
┌──────────────────────────────────────────────────────────┐
│  ⏱ 8/10 (7+1 merchant)  ◉ 342g     THE ARMS DEALER     │
│                                                          │
│  ┌──────────┐  ┌──────────────────────────────────────┐  │
│  │          │  │  FOR SALE                            │  │
│  │  Tower   │  │                                      │  │
│  │  cross   │  │  WEAPONS                             │  │
│  │  section │  │  ┌──────────────────────────────┐    │  │
│  │  (dim)   │  │  │ ★★ Flaming Composite Bow   │85g │  │
│  │          │  │  │ DMG 18  SPD 1.4/s  RNG 130  │    │  │
│  │          │  │  │ [Flaming] 3s burn           │    │  │
│  │          │  │  │ Ability: Split Shot          │    │  │
│  │          │  │  │              [BUY] [COMPARE] │    │  │
│  │          │  │  └──────────────────────────────┘    │  │
│  │          │  │  ┌──────────────────────────────┐    │  │
│  │          │  │  │ ★ Heavy Crossbow            │45g │  │
│  │          │  │  │ DMG 32  SPD 0.6/s  RNG 160  │    │  │
│  │          │  │  │ Ability: Snipe               │    │  │
│  │          │  │  │              [BUY] [COMPARE] │    │  │
│  │          │  │  └──────────────────────────────┘    │  │
│  │          │  │                                      │  │
│  │          │  │  MATERIALS            TRINKETS       │  │
│  │          │  │  🪨 Stone ×5    15g   ★ Spyglass    │  │
│  │          │  │  🪵 Planks ×5   12g   +15% range    │  │
│  │          │  │  ⚙ Bolts ×5    18g   40g [BUY]     │  │
│  │          │  │  [BUY]                               │  │
│  │          │  │                                      │  │
│  │          │  │  ─── SELL ───                        │  │
│  │          │  │  [SELL MATERIALS]  [SELL GEAR]        │  │
│  └──────────┘  └──────────────────────────────────────┘  │
│                                                          │
│  "Fine weaponry for a fine fortress!"                    │
│                                                          │
│  [LEAVE MERCHANT]                                        │
└──────────────────────────────────────────────────────────┘
```

### Buy flow
1. Browse weapons/trinkets in the shop grid. Each card shows: rarity badge, name, key stats, modifier chip, price.
2. Click [COMPARE] on a weapon → side-by-side comparison opens:

```
┌────────────────────────┬────────────────────────┐
│  CURRENT                │  COMPARING              │
│  Composite Bow          │  Flaming Composite Bow  │
│  ★ Uncommon             │  ★★ Rare                │
│                         │                         │
│  DMG  14                │  DMG  18     ▲ +4       │
│  SPD  1.2/s             │  SPD  1.4/s  ▲ +0.2    │
│  RNG  120               │  RNG  130    ▲ +10     │
│  ARC  medium            │  ARC  medium            │
│                         │                         │
│  Modifier: none         │  Modifier: Flaming      │
│                         │  3s burn DOT            │
│                         │                         │
│  Ability: Split Shot    │  Ability: Split Shot    │
│                         │                         │
│  Ammo: arrows           │  Ammo: arrows           │
│  Cache: compatible ✓    │  Cache: compatible ✓    │
└────────────────────────┴────────────────────────┘
```

3. Green ▲ = better, red ▼ = worse. Instant visual read.
4. Click [BUY] → gold deducts, item goes to inventory. "Ka-ching" sound.
5. Items you can't afford: price in red, [BUY] grayed.

### Sell flow
1. Click [SELL MATERIALS] → warehouse resource list opens in a panel below the shop. Each resource type shows: current stock, quantity selector (+/- or type a number), sell price per unit (50% of buy price), and total sale value. "SELL" confirms the batch. No per-item confirmation — speed matters.
2. Click [SELL GEAR] → inventory list filtered to unequipped items. Each shows: name, rarity, sell price. Click an item to sell immediately (single click, no confirmation for unequipped gear). Equipped items require one confirmation ("This is equipped on Varn. Sell anyway?").
3. After selling, gold counter in the top bar updates immediately. Merchant reacts with a dialogue line.

### Compare flow (weapons)
1. Click [COMPARE] on any weapon in the shop → side-by-side panel opens (see wireframe above).
2. Left column: currently equipped weapon. Right column: shop item. Delta arrows (▲▼) show per-stat difference.
3. If the hero has two weapon slots, a toggle at the top of the compare panel switches which equipped weapon is being compared.
4. Close compare panel to return to browse. Compare does not consume a tick or cost gold.

### Merchant archetypes
Shown on the map node before arriving. Stock composition differs:
- **Arms Dealer:** 3 weapons (higher rarity), 1 trinket, limited materials.
- **Supplier:** 1 weapon, lots of materials, 1-2 specialty items (repair kits, runner boots).
- **Collector:** 1-2 unique/legendary items at premium. Buys your stuff at 75% instead of 50%.
- **Traveler:** random mix. Cheapest prices. Unpredictable.

### Merchant personality
Speech bubble at bottom with character dialogue. Changes when you buy/sell. "Ah, fine choice!" / "Selling that? Bold move." / "I'll take it off your hands." Adds character without slowing interaction.

---

## VII. Inventory & Equipment

Accessible from prep phase via top bar button or keyboard shortcut (I).

### Layout

```
┌──────────────────────────────────────────────────────────┐
│  INVENTORY                                       [CLOSE] │
│                                                          │
│  [WEAPONS] [TRINKETS] [MATERIALS]                        │
│  Filter: [All ▼]  Sort: [Rarity ▼]                      │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │ ★★ Flaming Composite Bow         EQUIPPED (Hero)  │  │
│  │ DMG 18  SPD 1.4/s  Ability: Split Shot             │  │
│  │ [Flaming]                          [EQUIP] [SELL]  │  │
│  ├────────────────────────────────────────────────────┤  │
│  │ ★★ Frost Heavy Crossbow           EQUIPPED (Varn)  │  │
│  │ DMG 32  SPD 0.6/s  Ability: Snipe                  │  │
│  │ [Frost]                            [EQUIP] [SELL]  │  │
│  ├────────────────────────────────────────────────────┤  │
│  │ ★ Recurve Bow                     in inventory     │  │
│  │ DMG 12  SPD 1.6/s  Ability: Quick Volley           │  │
│  │ (no modifier)                      [EQUIP] [SELL]  │  │
│  ├────────────────────────────────────────────────────┤  │
│  │ ★ Bombs                           in inventory     │  │
│  │ DMG 28  SPD 0.8/s  Ability: Cluster Bomb           │  │
│  │ (no modifier)                      [EQUIP] [SELL]  │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  Enchanter available (Floor 5)           [ENCHANT ▶]    │
└──────────────────────────────────────────────────────────┘
```

### Tabs
- **Weapons:** all weapons in inventory + equipped. Equipped items pinned at top. Filter by base type (bow, crossbow, etc.). Sort by rarity, damage, speed.
- **Trinkets:** same format. Equipped trinkets pinned.
- **Materials:** warehouse resource grid (ResourceCount per type). Shows production rate trend (▲ rising, ─ stable, ▼ falling).

### Equip flow
1. Click [EQUIP] on a weapon → "Equip on whom?" selector. Shows hero + all companions. Each entry shows current weapon for comparison.
2. Select recipient → weapon equipped. Previous weapon goes to inventory. Free (no ticks).

### Enchanter access
If an Enchanter building exists in the tower, a link appears at the bottom: "Enchanter available (Floor 5) [ENCHANT ▶]". Click opens the enchanter panel:

```
┌──────────────────────────────────────────┐
│  ENCHANTER — Modify Weapon               │
│                                          │
│  Select weapon: [Composite Bow ▼]        │
│  Current modifier: Flaming               │
│                                          │
│  ○ REROLL (random new modifier)          │
│    Cost: 3 💎 mana crystals              │
│    Warehouse: 8 💎 available ✓           │
│                                          │
│  ○ APPLY SPECIFIC:                       │
│    [Flaming]    3💎 + 2 fire oil    ✓    │
│    [Frost]      3💎 + 2🪨           ✓    │
│    [Efficient]  5💎                  ✓    │
│    [Explosive]  4💎 + 3🪨           ✗    │
│    [Silent]     3💎 + 2🪵           ✓    │
│                                          │
│  [REROLL]    [APPLY SELECTED]            │
│                                          │
│  ⚠ Materials consumed immediately.       │
│  ⚠ Reroll result is random.              │
└──────────────────────────────────────────┘
```

Each option shows materials needed with ✓/✗ for affordability. Reroll is cheap but random. Apply is expensive but deterministic. Both consume warehouse materials (competing with ammo production).

---

## VIII. Companion Management

Accessible from prep phase via top bar [COMP] button or keyboard shortcut (C).

### Roster view

```
┌──────────────────────────────────────────────────────────┐
│  COMPANIONS (3/6)                                [CLOSE] │
│                                                          │
│  ┌──────────────────────────────────────────────────┐    │
│  │ [portrait] VARN THE SPOTTER            Floor 6 ⌂ │    │
│  │            Passive: Mark (+20% dmg from all)     │    │
│  │            Weapon: ★★ Frost Heavy Crossbow       │    │
│  │            Accuracy: ████████░░ 78%              │    │
│  │            Orders: Air Focus / Conservative      │    │
│  │            Wage: 14g/encounter                   │    │
│  │            [DETAIL] [EQUIP] [ORDERS] [POSITION]  │    │
│  ├──────────────────────────────────────────────────┤    │
│  │ [portrait] KAEL THE SHIELD-BEARER      Floor 4 ⌂ │    │
│  │            Passive: Wall (blocks climbers)       │    │
│  │            Weapon: (none)                        │    │
│  │            Orders: n/a                           │    │
│  │            Wage: 10g/encounter                   │    │
│  │            [DETAIL] [POSITION]                   │    │
│  ├──────────────────────────────────────────────────┤    │
│  │ [portrait] MIRA THE GRENADIER          Floor 3 ⌂ │    │
│  │            Passive: Splash (AoE, 2x ammo)       │    │
│  │            Weapon: ★ Bombs                       │    │
│  │            Accuracy: ██████░░░░ 58%              │    │
│  │            Orders: Ground Focus / Free Fire      │    │
│  │            Wage: 18g/encounter                   │    │
│  │            [DETAIL] [EQUIP] [ORDERS] [POSITION]  │    │
│  └──────────────────────────────────────────────────┘    │
│                                                          │
│  Total wages: 42g/encounter                              │
└──────────────────────────────────────────────────────────┘
```

### Sub-flows

**[DETAIL]:** Full companion view. Large portrait, name, personality quote ("Put me up high — I can mark targets for everyone below."), all stats, combat history (kills, accuracy trend over last 5 encounters, total encounters survived). Close to return.

**[EQUIP]:** Weapon selector filtered to compatible types. Shows inventory weapons. Click to equip with side-by-side comparison to current weapon. Trinket equip is a separate tab in the same panel. Free (no ticks).

**[ORDERS]:** Two dropdowns inline:
- Target: Nearest / Ground / Air / Climbers / Siege / Boss
- Discipline: Free Fire / Conservative / Hold Fire
Hover each option for tooltip explaining behavior. Companion reacts to changes: "Air focus? Got it — I'll watch the skies." Free (no ticks).

**[POSITION]:** Highlights balcony slots in the tower cross-section (visible behind the companion panel). Occupied slots show current occupant. Click empty slot → companion moves there. Click occupied slot → companions swap positions. Interactive tower view during positioning.

### Recruitment (at recruitment nodes)

```
┌──────────────────────────────────────────────────────────┐
│  A TRAVELER APPROACHES                                   │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │  [large portrait]                                  │  │
│  │                                                    │  │
│  │  VARN THE SPOTTER                                  │  │
│  │  "I've been watching your tower walk past for      │  │
│  │   a mile. Impressive machine. But your top floor   │  │
│  │   is undefended — I can fix that. Give me a        │  │
│  │   crossbow and a high perch."                      │  │
│  │                                                    │  │
│  │  Passive: MARK                                     │  │
│  │  Enemies hit take +20% damage from all sources     │  │
│  │  for 3 seconds.                                    │  │
│  │                                                    │  │
│  │  Weapon preference: Crossbow                       │  │
│  │  Starting accuracy: 45%                            │  │
│  │  Wage: 14g/encounter                               │  │
│  │  Hiring cost: 35g + 1 tick                         │  │
│  │                                                    │  │
│  │  [HIRE VARN]              [DECLINE]                │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  Your companions: 3/6 — room available                   │
│  Gold: 342g — can afford ✓                               │
│  Ticks: 7/10 — can afford ✓                              │
└──────────────────────────────────────────────────────────┘
```

- Companion introduces themselves in character. Dialogue hints at preferred position/playstyle.
- Stats and costs clearly shown. Affordability indicated below.
- If roster is full (6/6): "DISMISS TO MAKE ROOM" option shows current roster for dismissal selection.
- Decline: node becomes minor rest (+1 tick, no other benefit).

---

## IX. Post-Combat Summary

```
┌──────────────────────────────────────────────────────────┐
│  ENCOUNTER SURVIVED                                      │
│                                                          │
│  Enemies killed: 34         Gold earned: +62             │
│  Hero kills: 24 (71%)      Accuracy: 82%                │
│  Companion kills: 10        Ammo used: 38🏹 12⚙         │
│                                                          │
│  Damage taken:                                           │
│  Floor 4 panel: 100% → 35%   ⚠ needs repair             │
│  Floor 2 panel: 100% → 78%                              │
│  Floor 4 staircase: DESTROYED (breach)                   │
│                                                          │
│  LOOT                                                    │
│  ┌──────────────────────┐  ┌──────────────────────┐     │
│  │ ★★ Frost Recurve Bow │  │ ★ Runner's Whistle  │     │
│  │ [TAKE]     [SKIP]    │  │ [TAKE]     [SKIP]   │     │
│  └──────────────────────┘  └──────────────────────┘     │
│                                                          │
│  💬 Varn: "That was close. Floor 4 took a beating —     │
│           we should patch that staircase before the      │
│           next fight."                                   │
│                                                          │
│  [CONTINUE]                                              │
└──────────────────────────────────────────────────────────┘
```

- Slides up from bottom. Tower visible behind (showing damage).
- Stats: kills, accuracy, ammo consumption, gold. Tells you how efficient you were.
- Damage: which floors took hits, what was destroyed. Links between damage and "what to repair."
- Loot: card-flip reveal. Take or skip per item.
- Companion comment: contextual, often pointing at what needs fixing.
- "Continue" enters prep phase.

---

## X. Hero Screen

Accessible from prep via top bar or keyboard (H).

### Stats

```
┌──────────────────────────────────────────────────────────┐
│  HERO — The Archer (Lv. 14)                      [CLOSE] │
│                                                          │
│  ┌──────────────────────────────────────────────────┐    │
│  │  [class art]  Weapon 1: ★★ Flaming Composite Bow│    │
│  │               Weapon 2: ★ Sword                  │    │
│  │               Trinket: Sharpshooter's Monocle    │    │
│  │               Skill: Focus Fire                  │    │
│  └──────────────────────────────────────────────────┘    │
│                                                          │
│  STATS (3 unspent points)              [ALLOCATE ▶]     │
│  Precision  12 ████████████░░░░ (+)                      │
│  Draw        8 ████████░░░░░░░░ (+)                      │
│  Tempo       6 ██████░░░░░░░░░░ (+)                      │
│  Grit        4 ████░░░░░░░░░░░░ (+)                      │
│  Salvage     5 █████░░░░░░░░░░░ (+)                      │
│                                                          │
│  PERKS                                  [PERK TREE ▶]   │
│  ✓ Steady Hand (T1 Combat)                               │
│  ✓ Scrap Collector (T1 Logistics)                        │
│  ✓ Headhunter (T2 Combat)                                │
│  ▶ 1 perk available — next at Lv. 15                     │
│                                                          │
│  XP: 2450/3000 to next level                             │
└──────────────────────────────────────────────────────────┘
```

### Stat allocation
- Click (+) next to a stat to spend a point. Stat bar extends. Point counter decrements.
- Hover a stat name for tooltip explaining exactly what it does.
- Points are permanent once spent (no respec in v1).

### Perk tree (separate view)

```
┌──────────────────────────────────────────────────────────┐
│  PERK TREE                                       [CLOSE] │
│                                                          │
│       COMBAT           LOGISTICS         DEFENSE         │
│                                                          │
│  T3   [Overcharge]     [War Cry]        [Last Stand]     │
│       [Executioner]    [Supply Master]  [Living Fort.]   │
│            │                │                │           │
│  T2   [Headhunter ✓]  [Inspiration]    [Iron Curtain]   │
│       [Double Tap]     [Quartermaster]  [Thick Walls]    │
│       [Piercing]       [Eff. Prod.]     [Reinf. Rack]   │
│            │                │                │           │
│  T1   [Steady Hand ✓] [Scrap Coll. ✓]  [Fortified]     │
│       [Quick Draw]     [Pack Rat]       [Watchman]       │
│                                                          │
│  ✓ = selected   ● = available   ○ = locked               │
│                                                          │
│  Tier 2 requires: 1 perk from any branch ✓               │
│  Tier 3 requires: 2 perks from Tier 2 (have 1/2)        │
│                                                          │
│  Hover any perk for full description.                    │
│  Click an available (●) perk to select.                  │
└──────────────────────────────────────────────────────────┘
```

- ✓ Selected perks: filled, highlighted border, connected by lines to show path.
- ● Available perks: outlined, subtle glow. Click to select (permanent).
- ○ Locked perks: dimmed. Hover shows unlock requirement ("Requires 2 Tier 2 perks from any branch").
- Lines connect tiers visually. The tree reads bottom-to-top (T1 at bottom, T3 at top).
- Selecting a perk: click → confirmation popup ("This is permanent. Choose Headhunter?") → confirm or cancel.

---

## XI. Responsive & Layout Rules

- **Primary target:** 1920×1080 desktop. 16:9 aspect ratio.
- **Game canvas:** fixed aspect ratio, letterboxes on non-16:9 displays. Never stretches.
- **React UI:** scales with viewport. Panels have min-width (300px) and max-width (500px).
- **Tower cross-section:** always visible during prep, always fits vertically. Target: 8 floors fitting within ~640px of vertical space at 1080p, leaving room for top/bottom bars. Exact floor height is an implementation variable — test with 8 populated floors before locking.
- **Font scaling:** user setting (+/- 2 steps). All typography sizes multiply by a user scale factor.
- **Touch support:** not a priority for v1 (desktop/browser focus), but buttons are minimum 32×32px touch targets for future consideration.
