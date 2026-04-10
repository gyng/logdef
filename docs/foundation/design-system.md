Project: SUPPLY LINE | Frontend Design System Specification

> **Status:** This document describes v1 only.

## I. Overview

The UI is split between two rendering layers:
- **Game canvas (Rust/wgpu):** tower cross-section, battlefield, combat, walking animation. Real-time, 60fps.
- **React overlay:** prep phase UI, map, inventory, companion management, menus. Turn-based, event-driven.

This document covers the React overlay layer. The game canvas follows the art direction doc for its visual language.

**This is a product design system for SUPPLY LINE, not a generic component library.** Many molecules and organisms below are game-specific (AmmoRack, BufferDisplay, PerkTree). That's intentional — the component taxonomy follows the game's domain, not abstract reusability. Atoms and tokens are reusable; molecules and above encode game meaning.

**Core constraint:** React is a VIEW LAYER. All game state lives in Rust. React renders snapshots received via wasm-bindgen and sends commands back. React owns only UI state (selections, hovers, open panels, animation transitions).

---

## II. Design Tokens

Tokens are the atomic values that ensure visual consistency across every component. No component should use a raw color, spacing value, or font size — always reference a token.

### Colors

```typescript
export const colors = {
  // --- Backgrounds ---
  // Warm dark palette: the UI should feel like parchment and wood in low lamplight,
  // not a blue-tinted dashboard. These are warm grays with brown undertones.
  bg: {
    base:      '#141210',  // deepest background — warm near-black
    surface:   '#1e1a16',  // panel/card backgrounds — dark walnut
    elevated:  '#2a2420',  // raised elements, overlays — lighter warm brown
    overlay:   'rgba(16, 12, 10, 0.85)', // overlay backdrop — warm dark
  },

  // --- Text ---
  text: {
    primary:   '#e8e0d4',  // main readable text — warm parchment white
    secondary: '#a89880',  // labels, descriptions — warm tan
    muted:     '#5c5244',  // disabled, placeholder — warm dim
    inverse:   '#141210',  // text on light backgrounds
    accent:    '#e6a23c',  // highlighted values, important numbers — warm gold
  },

  // --- Interactive ---
  interactive: {
    default:   '#4a3d30',  // button/control resting — warm brown
    hover:     '#5c4d3e',  // hover state — lighter warm brown
    active:    '#6e5d4c',  // pressed/active state
    focus:     '#c49040',  // focus ring — warm amber (not blue)
    disabled:  '#2a2420',  // disabled controls
  },

  // --- Resource types (logistics readability) ---
  resource: {
    arrows:    '#c87941',  // warm amber-brown
    bolts:     '#7a8fa3',  // steel blue-gray
    mana:      '#7c5cbf',  // deep purple
    wood:      '#8b6914',  // raw wood brown
    stone:     '#8a8a8a',  // neutral gray
    planks:    '#c4a35a',  // finished light tan
    fireOil:   '#d45722',  // flame orange
    potions:   '#2ea88a',  // teal-green
    gunpowder: '#3a3a3a',  // charcoal dark
    gold:      '#f0c240',  // bright gold
  },

  // --- Status ---
  status: {
    healthy:   '#3daa5e',  // green — full, working
    producing: '#5bb87a',  // lighter green — active animation
    warning:   '#d4922a',  // amber — buffer low, minor damage
    critical:  '#c9433a',  // red — breach, empty, critical damage
    starved:   '#6b5a8a',  // muted purple — waiting for input
    empty:     '#444c56',  // dim gray — nothing
  },

  // --- Rarity ---
  rarity: {
    common:    '#a0a8b4',  // silver-gray
    uncommon:  '#48b066',  // green
    rare:      '#4a8fd4',  // blue
    legendary: '#e8a830',  // warm gold
  },

  // --- Terrain (map) ---
  terrain: {
    plains:    '#c4b078',  // golden grass
    forest:    '#3a7a4a',  // deep green
    mountain:  '#7088a0',  // cool blue-gray
    swamp:     '#6a7a3a',  // murky yellow-green
    desert:    '#c8b888',  // sandy beige
    storm:     '#4a5568',  // dark blue-gray
  },

  // --- Borders & dividers ---
  border: {
    subtle:    '#2a2420',  // barely visible separation — warm
    default:   '#3a3028',  // standard border
    strong:    '#4a3d30',  // emphasized border
  },
} as const;
```

### Color semantic precedence

Several hues appear in multiple palettes (green = healthy + uncommon, gold = currency + legendary + accent, purple = mana + starved). The rule: **context determines which palette applies, and they never render on the same element.**

| Context | Palette | Applies to |
|---------|---------|------------|
| Physical objects in tower (crates, piles, racks) | `resource` | Fill colors of resource sprites |
| Infrastructure state (buildings, panels, transport) | `status` | Glow, outline, ambient lighting on infrastructure |
| Item cards, loot frames, shop items | `rarity` | Border/frame color on cards only — never on world objects |
| Map screen | `terrain` | Path lines and node backgrounds |
| UI controls, buttons, inputs | `interactive` | Button fills, focus rings |

If two palettes could both apply (e.g., a potion bottle in a shop has both `resource.potions` teal AND `rarity.uncommon` green), the rule is: the physical object uses `resource` color, the card border uses `rarity` color. They don't compete because they're on different elements.

### Spacing

An 4px base unit scale. All spacing uses these values.

```typescript
export const spacing = {
  '0':   0,
  '1':   4,    // xs — tight padding, icon gaps
  '2':   8,    // sm — inner padding, compact lists
  '3':   12,   // md — standard padding
  '4':   16,   // lg — section padding, card padding
  '5':   20,   // xl — between sections
  '6':   24,   // 2xl — major separations
  '8':   32,   // 3xl — panel margins
  '10':  40,   // 4xl — page-level spacing
  '12':  48,   // 5xl — major layout gaps
} as const;
```

### Typography

```typescript
export const typography = {
  family: {
    // Lora: a warm, slightly bookish serif with good readability at small sizes.
    // Supports the parchment/workshop feel better than a geometric sans like Inter.
    // Fallback chain goes warm serif → system serif → sans as last resort.
    ui:      '"Lora", "Georgia", "Palatino", serif',
    mono:    '"JetBrains Mono", "Fira Code", monospace',
    display: '"Cinzel", "Palatino", serif',
  },

  size: {
    xs:    10,   // tiny labels, subscripts
    sm:    12,   // secondary text, descriptions
    md:    14,   // body text, default
    lg:    16,   // emphasized body, input text
    xl:    20,   // section headers
    '2xl': 24,   // panel titles
    '3xl': 32,   // page titles
    '4xl': 40,   // display / hero text
  },

  weight: {
    normal:   400,
    medium:   500,
    semibold: 600,
    bold:     700,
  },

  lineHeight: {
    tight:  1.2,   // headings
    normal: 1.5,   // body
    loose:  1.8,   // readable paragraphs
  },

  letterSpacing: {
    tight:   '-0.01em',
    normal:  '0',
    wide:    '0.02em',  // uppercase labels
    wider:   '0.05em',  // display text
  },
} as const;
```

### Radii

```typescript
export const radii = {
  none:  0,
  sm:    2,    // subtle rounding (buttons, inputs)
  md:    4,    // cards, panels
  lg:    8,    // modals, large cards
  full:  9999, // pills, circular elements
} as const;
```

### Shadows

Minimal. Depth is communicated through background color stepping, not shadows. Shadows reserved for floating elements.

```typescript
export const shadows = {
  none:     'none',
  sm:       '0 1px 3px rgba(0,0,0,0.3)',               // tooltips
  md:       '0 4px 12px rgba(0,0,0,0.4)',               // dropdowns, popovers
  lg:       '0 8px 24px rgba(0,0,0,0.5)',               // dialogs, overlays
  glow: (color: string) => `0 0 8px ${color}40`,        // status glow (buffer full, etc.)
} as const;
```

### Animation

```typescript
export const animation = {
  duration: {
    instant:  '50ms',    // state toggles
    fast:     '120ms',   // hover states, micro-interactions
    normal:   '200ms',   // panel transitions, fades
    slow:     '400ms',   // page transitions, large reveals
    glacial:  '800ms',   // ambient loops, breathing effects
  },

  easing: {
    snap:     'cubic-bezier(0.25, 0.46, 0.45, 0.94)',   // quick, decisive
    smooth:   'cubic-bezier(0.4, 0, 0.2, 1)',            // standard motion
    bounce:   'cubic-bezier(0.68, -0.15, 0.265, 1.15)',  // slight overshoot
    decel:    'cubic-bezier(0, 0, 0.2, 1)',              // entering (slide in)
    accel:    'cubic-bezier(0.4, 0, 1, 1)',              // exiting (slide out)
  },
} as const;
```

### Z-Index

```typescript
export const zIndex = {
  canvas:     0,     // game canvas (wgpu)
  hud:        10,    // combat HUD overlay
  panel:      20,    // prep phase panels (slide-in)
  dropdown:   30,    // dropdowns, selectors
  tooltip:    40,    // tooltips
  dialog:     50,    // confirmation dialogs (prep-phase only, never combat)
  overlay:    60,    // full-screen overlays (map, inventory)
  toast:      70,    // notifications, alerts
} as const;
```

---

## III. Atoms

The smallest visual primitives. No business logic. Pure presentation. Each atom takes props for variant/size/state and maps them to tokens.

### Text

```typescript
interface TextProps {
  size?: keyof typeof typography.size;       // default: 'md'
  weight?: keyof typeof typography.weight;   // default: 'normal'
  color?: string;                            // default: colors.text.primary
  family?: keyof typeof typography.family;   // default: 'ui'
  transform?: 'uppercase' | 'capitalize';
  truncate?: boolean;
  align?: 'left' | 'center' | 'right';
}
// Variants: <Label>, <Value>, <Title>, <Caption> — preset combos
```

### Icon

```typescript
interface IconProps {
  name: IconName;              // from icon set
  size?: 12 | 16 | 20 | 24;   // default: 16
  color?: string;              // default: currentColor
}
// Icon set includes: resource icons (arrow, bolt, mana, etc.),
// status icons (healthy, damaged, empty), UI icons (chevron, close, gear),
// node icons (skull, coin, tent, crown, etc.), weapon type icons
```

### ResourceIcon

```typescript
interface ResourceIconProps {
  resource: ResourceType;      // 'arrows' | 'bolts' | 'mana' | ...
  size?: 12 | 16 | 20 | 24;
}
// Renders the resource icon in its canonical color from tokens.
// Always consistent — arrows are ALWAYS warm brown, mana is ALWAYS purple.
```

### Badge

```typescript
interface BadgeProps {
  variant: 'rarity' | 'count' | 'status' | 'terrain';
  value: string | number;
  color?: string;   // auto-derived from variant + value if not specified
}
// Rarity badge: colored pill with rarity name
// Count badge: small number in circle (inventory counts)
// Status badge: colored dot + label
// Terrain badge: colored chip with terrain name
```

### ProgressBar

```typescript
interface ProgressBarProps {
  value: number;               // 0-1
  max?: number;                // for labeled bars
  size?: 'sm' | 'md' | 'lg';  // height: 4px, 8px, 12px
  color?: string;              // bar fill color
  bgColor?: string;            // track color
  showLabel?: boolean;         // show value/max text
  animated?: boolean;          // pulse when changing
  segments?: number;           // discrete segments instead of smooth fill
}
```

### VerticalBar

Same as ProgressBar but vertical. Used for cache fill levels, rack display.

```typescript
interface VerticalBarProps {
  value: number;
  size?: 'sm' | 'md' | 'lg';  // width: 4px, 8px, 12px
  height?: number;             // total bar height in px
  color?: string;
  segments?: number;           // crate-sized discrete segments
}
```

### Button

```typescript
interface ButtonProps {
  variant: 'primary' | 'secondary' | 'ghost' | 'danger';
  size?: 'sm' | 'md' | 'lg';
  disabled?: boolean;
  loading?: boolean;
  icon?: IconName;             // optional leading icon
  children: React.ReactNode;
}
// primary: solid fill, strong CTA
// secondary: bordered, softer CTA
// ghost: text only, minimal footprint
// danger: red tint, destructive actions (dismiss companion, sell gear)
```

### IconButton

Square icon-only button. Used in toolbars, compact controls.

```typescript
interface IconButtonProps {
  icon: IconName;
  size?: 'sm' | 'md' | 'lg';  // 24px, 32px, 40px
  variant?: 'default' | 'ghost';
  active?: boolean;
  disabled?: boolean;
  tooltip?: string;
}
```

### Tooltip

```typescript
interface TooltipProps {
  content: React.ReactNode;
  placement?: 'top' | 'bottom' | 'left' | 'right';
  delay?: number;              // ms before showing, default: 300
  children: React.ReactElement;
}
// Compact, dark background, subtle shadow.
// Used extensively for item stats, building details, resource explanations.
```

### Chip

Small inline tag. Used for modifiers, terrain types, orders.

```typescript
interface ChipProps {
  label: string;
  color?: string;
  size?: 'sm' | 'md';
  removable?: boolean;         // show X to remove
  icon?: IconName;
}
```

### Divider

```typescript
interface DividerProps {
  orientation?: 'horizontal' | 'vertical';
  color?: string;              // default: colors.border.subtle
  spacing?: keyof typeof spacing;  // margin above/below
}
```

---

## IV. Molecules

Combinations of atoms with specific game meaning. Each molecule represents a recognizable game concept.

### ResourceCount

Icon + number. The most common molecule — appears everywhere.

```typescript
interface ResourceCountProps {
  resource: ResourceType;
  count: number;
  size?: 'sm' | 'md' | 'lg';
  showChange?: number;         // +5 / -3 animated delta
}
// Renders: [arrow icon] 24
// With change: [arrow icon] 24 (+5 fading in green)
```

### ResourceBar

Icon + progress bar. Shows a fill level for a resource buffer.

```typescript
interface ResourceBarProps {
  resource: ResourceType;
  current: number;
  max: number;
  size?: 'sm' | 'md';
  showLabel?: boolean;         // "12/20"
  status?: 'normal' | 'low' | 'critical' | 'overflow';
}
// Status auto-derives from current/max ratio if not specified.
// Low (<25%) = warning color. Critical (<10%) = red pulse.
// Overflow (full) = glow.
```

### StatLine

Label + value, optionally with a bar and delta.

```typescript
interface StatLineProps {
  label: string;               // "Precision"
  value: number;
  maxValue?: number;           // for bar display
  delta?: number;              // +2 / -1 from equipment change
  icon?: IconName;
}
// Renders: Precision    8 ████████░░░░  (+2)
//          ^label       ^value ^bar     ^delta (green/red)
```

### WeaponCard

Compact weapon display for inventory/equipment screens.

```typescript
interface WeaponCardProps {
  weapon: WeaponData;
  selected?: boolean;
  equipped?: boolean;
  compact?: boolean;           // single-line for lists vs. expanded for detail
  compareBase?: WeaponData;    // show stat deltas vs. current weapon
  onClick?: () => void;
}
// Renders:
// [bow icon] Flaming Longbow          [rare badge]
// DMG 24  SPD 1.2/s  RNG 180
// [flaming chip] Ignites, 3s burn DOT
// Ability: Charged Shot — piercing line
```

### TrinketCard

Similar to WeaponCard but for trinkets.

```typescript
interface TrinketCardProps {
  trinket: TrinketData;
  selected?: boolean;
  equipped?: boolean;
  compact?: boolean;
  onClick?: () => void;
}
```

### CompanionPortrait

Face/silhouette + name + key info. Used in roster, assignment, status row.

```typescript
interface CompanionPortraitProps {
  companion: CompanionData;
  size?: 'sm' | 'md' | 'lg';  // 32px, 48px, 64px
  showName?: boolean;
  showPassive?: boolean;       // passive ability icon
  showAccuracy?: boolean;      // accuracy bar underneath
  showStatus?: 'normal' | 'injured' | 'displaced';
  selected?: boolean;
  onClick?: () => void;
}
```

### PanelHealth

Floor label + HP bar. For the repair queue and damage overview.

```typescript
interface PanelHealthProps {
  floorIndex: number;
  currentHP: number;
  maxHP: number;
  material: FloorMaterial;     // affects color/icon
  breached?: boolean;          // special red state
}
```

### CooldownTimer

Circular cooldown overlay on an ability icon.

```typescript
interface CooldownTimerProps {
  icon: IconName;
  cooldownRemaining: number;   // 0-1 (0 = ready, 1 = full cooldown)
  ready?: boolean;             // glow when ready
  size?: 'sm' | 'md' | 'lg';
  hotkey?: string;             // "Q" / "E" label
}
```

### TickCounter

Planning ticks remaining.

```typescript
interface TickCounterProps {
  current: number;
  max: number;
  animated?: boolean;          // tick down animation when spending
}
// Renders as hourglass icon + "7 / 10" with segmented bar
```

### WaveIndicator

Current wave in an encounter.

```typescript
interface WaveIndicatorProps {
  current: number;
  total: number;
  inLull?: boolean;            // show "RESUPPLY" label during lull
  enemyCount?: number;         // remaining enemies in current wave
}
```

### AmmoRack

Visual representation of the hero's ammo rack.

```typescript
interface AmmoRackProps {
  crates: number;              // current crate count
  maxCrates: number;
  resource: ResourceType;
  depleting?: boolean;         // animate depletion during combat
}
// Renders as stacked crate icons (visual, not a bar).
// 3 crates = 3 icons stacked. 1 crate = 1 icon. 0 = empty shelf.
```

### NodeMarker

Map node display.

```typescript
interface NodeMarkerProps {
  type: NodeType;              // combat, elite, merchant, rest, mystery, boss, etc.
  difficulty?: 1 | 2 | 3;
  terrain?: TerrainType;
  visited?: boolean;
  current?: boolean;
  selectable?: boolean;
  onClick?: () => void;
}
```

### BufferDisplay

Building output buffer — mini vertical bar with state indication.

```typescript
interface BufferDisplayProps {
  current: number;
  max: number;
  resource: ResourceType;
  state: 'producing' | 'full' | 'starved' | 'offline';
}
// Producing: animated fill. Full: glow. Starved: dim pulse. Offline: dark.
```

### OrderSelector

Dropdown for companion targeting / fire discipline.

```typescript
interface OrderSelectorProps {
  type: 'target' | 'discipline';
  value: TargetOrder | FireDiscipline;
  onChange: (value: any) => void;
  compact?: boolean;           // for status row (icon only)
}
// Target: Nearest, Ground, Air, Climbers, Siege, Boss
// Discipline: Free Fire, Conservative, Hold Fire
```

### LootItem

Item in loot/reward display with compare indicators.

```typescript
interface LootItemProps {
  item: WeaponData | TrinketData;
  isUpgrade?: boolean;         // green up arrow
  isDowngrade?: boolean;       // red down arrow
  isSidegrade?: boolean;       // yellow lateral arrow
  isNew?: boolean;             // "NEW" badge
  onClick?: () => void;
}
```

---

## V. Organisms

Complex UI sections composed of molecules and atoms. Each organism owns a specific domain of the game interface.

### Combat Domain

**CombatHUD** — the entire combat overlay. Minimal, edge-tucked, semi-transparent.
```
┌─────────────────────────────────────────────┐
│ [wave 2/3]                    [gold: 245]   │  ← top bar
│                                             │
│                                             │
│                                             │
│                     (game canvas)           │
│                                             │
│                                             │
│ [ability1][ability2]  [ammo rack]            │  ← bottom bar
│ [companion row: portrait|ammo|status ...]   │
└─────────────────────────────────────────────┘
```

- **AmmoPanel** — hero rack visual + cache fill level + resource type label
- **AbilityBar** — two CooldownTimer molecules (weapon ability + hero skill) with hotkey labels
- **CompanionStatusRow** — horizontal row of CompanionPortrait (sm) + VerticalBar (ammo) + OrderSelector (compact icon) per companion. Read left-to-right = bottom-to-top of tower.
- **DamageOverlay** — flash warnings when panels take heavy damage. "[Floor 4] UNDER ATTACK" with directional indicator. Fades after 2 seconds.

### Prep Domain

**TowerEditor** — the main prep phase view. Interactive tower cross-section rendered as a React overlay atop the game canvas.
```
┌─────────┬───────────────────────────┐
│         │                           │
│  Build  │   Tower cross-section     │
│  Menu   │   (interactive floors)    │
│         │                           │
│  -----  │   [floor 6] [+balcony]    │
│  Tick   │   [floor 5] [building]    │
│  Counter│   [floor 4] [building]    │
│         │   [floor 3] [building]    │
│         │   [floor 2] [quarters]    │
│         │   [floor 1] [warehouse]   │
│         │   [legs]                  │
└─────────┴───────────────────────────┘
```

- **FloorPanel** — single floor row. Shows: floor material icon, building (or empty slot), transport passing through (stair/lift/chute icons), cache (if present), balcony (if present), panel HP bar. Clickable to select. Click empty slot to build.
- **BuildMenu** — left panel. Lists what can be built, filtered by selected floor. Shows: tick cost, material cost, current resources available. Grayed-out if can't afford. Grouped by category (buildings, transport, infrastructure).
- **TransportPanel** — sub-panel when selecting transport. Shows available transport types for this floor, where they connect, tick/material cost.
- **LiftProgrammer** — opens when clicking a lift. Floor range slider, car count selector (1-3), departure mode toggle (immediate/batch). Changes are free (no ticks).
- **CompanionAssigner** — drag-and-drop companion portraits onto balcony slots. Shows companion weapon type + passive icon. Highlights compatible positions.
- **OrdersPanel** — per-companion: two OrderSelector dropdowns (target + discipline). Free to change.
- **RepairQueue** — list of damaged floors sorted by severity. Each entry is a PanelHealth molecule + repair button (tick + material cost). Priority ordering.
- **ValidationWarnings** — pre-departure warning list. Yellow/red items. Non-blocking. "Floor 7 has no transport." "Hero rack resource mismatch."

### Map Domain

**ChapterMap** — full Slay the Spire branching map.
```
┌─────────────────────────────────────┐
│  Chapter 3: The Frontier            │
│                                     │
│  ○──[plains]──○──[forest]──○        │
│ /             │              \      │
│START          ○──[mountain]── BOSS  │
│ \             │              /      │
│  ○──[swamp]──○──[plains]──○        │
│                                     │
│  [node detail panel when hovered]   │
│  [route summary when path selected] │
└─────────────────────────────────────┘
```

- **MapNode** — NodeMarker molecule + connection lines. Hover shows NodeDetail popup.
- **PathLine** — SVG line between nodes, colored/textured by terrain type. Dashed if leg penalty applies.
- **RoutePreview** — when a path is selected, shows: total encounters, terrain types, tick budget, estimated difficulty. Bottom panel.
- **NodeDetail** — popup on node hover. Shows full node info: type, difficulty, terrain modifier, rewards, merchant type, companion available, etc.

### Inventory Domain

**WeaponList** — grid of WeaponCard molecules within a constrained-height panel. Filter by type (bow, crossbow, etc.), sort by rarity/damage/name. Equipped items pinned at top. Per the UI/UX doc: spatial layouts (tower cross-section) never scroll, but data lists like this one scroll within their fixed panel when filter/sort can't reduce the list to screen height.

**WeaponCompare** — side-by-side: current weapon vs. selected weapon. StatLine molecules for each stat with deltas highlighted (green = better, red = worse). Shows ability comparison.

**EnchanterPanel** — modifier crafting UI.
```
┌──────────────────────────────┐
│  Current: Flaming Longbow    │
│  Modifier: [Flaming]        │
│                              │
│  ○ Reroll (random)           │
│    Cost: 3 mana crystals     │
│                              │
│  ○ Apply specific:           │
│    [Efficient] 5 mana + ...  │
│    [Frost] 3 mana + 2 stone  │
│    [Vampiric] 4 mana + ...   │
│                              │
│  [Reroll] [Apply]            │
└──────────────────────────────┘
```

**MaterialsPanel** — warehouse resource counts as a grid of ResourceCount molecules. Shows production rate per resource (arrows/min). Stockpile trend indicator (rising, stable, falling).

### Merchant Domain

**ShopView** — grid of items for sale. WeaponCard / TrinketCard / ResourceCount molecules. Price tag on each. Items you can't afford are dimmed.

**BuySellPanel** — transaction confirmation. Shows: item, price, your gold, gold after purchase. Buy/sell toggle. Quantity selector for materials.

**MerchantDialogue** — merchant personality text in a speech bubble. Appears when entering merchant node and when buying/selling. "Ah, a fellow arms enthusiast! I've got something special today..."

### Companion Domain

**CompanionRoster** — grid of CompanionPortrait (lg) molecules. Shows all companions. Click for detail. Hired vs. available indicated.

**CompanionDetail** — full companion view:
```
┌──────────────────────────────────┐
│ [portrait]  Varn the Spotter     │
│             "Mark" — enemies     │
│             take 20% more dmg    │
│                                  │
│  Accuracy: ████████░░ (78%)      │
│  Weapon: [crossbow card]         │
│  Trinket: [trinket card]         │
│  Position: Floor 5 balcony       │
│  Orders: [target] [discipline]   │
│  Wage: 12 gold/encounter         │
│                                  │
│  "Put me up high — I can mark    │
│   targets for everyone below."   │
│                                  │
│  [Equip] [Assign] [Dismiss]     │
└──────────────────────────────────┘
```

**RecruitPanel** — recruitment node UI. Shows candidate companion with full stats, passive, personality quote. Cost to hire. Compare to current roster.

### Hero Domain

**HeroStats** — stat allocation. Five StatLine molecules (Precision, Draw, Tempo, Grit, Salvage). Unspent points counter. +/- buttons per stat. Tooltip on each stat explaining its effect.

**PerkTree** — visual tree with three branches, tiered.
```
┌─────────────────────────────────────┐
│        COMBAT    LOGISTICS   DEFENSE│
│                                     │
│  T3    [perk]    [perk]     [perk]  │
│          |          |          |    │
│  T2    [perk]    [perk]     [perk]  │
│        [perk]    [perk]     [perk]  │
│          |          |          |    │
│  T1    [perk]    [perk]     [perk]  │
│        [perk]    [perk]     [perk]  │
│                                     │
│  Selected perks highlighted.        │
│  Available perks glowing.           │
│  Locked perks dimmed.               │
│  Hover shows perk description.      │
└─────────────────────────────────────┘
```

Each perk node is an IconButton with tooltip. Selected = filled + highlighted border. Available = outlined + subtle glow. Locked = dimmed, shows "Requires N tier picks" on hover. Lines connect tiers.

**HeroSummary** — compact hero overview for quick reference. Class icon + level + key stats + weapon + trinket + perk icons. Used in the HUD and prep header.

### Meta Domain

**ClassSelect** — hero class picker at run start. 4 class cards with: class art, starting stats, starting weapon, class perk description, playstyle summary. Click to select. Selected card enlarges.

**DestinationSelect** — journey destination picker. Map showing 3-4 endpoints with descriptions. Locked destinations show unlock requirement.

**UnlocksGrid** — grid showing all unlockable content. Unlocked = full color + checkmark. Locked = silhouette + unlock requirement text. Categories: buildings, transport, weapons, companions, map nodes, legs.

**AscensionPanel** — difficulty modifier toggles. Each modifier is a Chip with toggle. Active modifiers glow. Total difficulty score counter.

### Shared

**PostCombatSummary** — post-encounter report:
```
┌──────────────────────────────────┐
│  ENCOUNTER SURVIVED              │
│                                  │
│  Enemies killed: 34    Gold: +62 │
│  Hero kills: 24 (71%)            │
│  Companion kills: 10 (29%)       │
│  Ammo used: 38 arrows, 12 bolts │
│  Accuracy: 82%                   │
│                                  │
│  Damage taken:                   │
│  Floor 4 panel: 100% → 35%      │
│  Floor 2 panel: 100% → 78%      │
│                                  │
│  Loot:                           │
│  [weapon card]  [trinket card]   │
│  [resource counts]               │
│                                  │
│  Companion says: "..."           │
│                                  │
│  [Continue]                      │
└──────────────────────────────────┘
```

**ConfirmDialog** — generic overlay dialog for destructive actions. Title + description + cancel/confirm buttons. Used for: dismiss companion, sell gear, spend rare materials. Prep-phase only — never appears during combat.

**ResourceSummary** — compact warehouse overview. Row of ResourceCount molecules. Always visible in prep header. Shows gold + key materials at a glance.

---

## VI. Pages (Templates)

Each page is a full-screen composition of organisms. Pages manage layout, not logic.

| Page | When | Key organisms |
|------|------|---------------|
| CombatPage | Encounter phase | Game canvas + CombatHUD overlay |
| PrepPage | Travel/prep phase | TowerEditor + BuildMenu + CompanionAssigner + TickCounter |
| MapPage | Route selection | ChapterMap + RoutePreview + NodeDetail |
| MerchantPage | Merchant node | ShopView + BuySellPanel + MerchantDialogue |
| InventoryPage | Accessible from prep | WeaponList + WeaponCompare + EnchanterPanel + MaterialsPanel |
| CompanionPage | Accessible from prep | CompanionRoster + CompanionDetail + OrdersPanel |
| PostCombatPage | After encounter | PostCombatSummary + loot + companion dialogue |
| HeroPage | Accessible from prep | HeroStats + PerkTree + HeroSummary |
| MenuPage | Main menu | Settings, credits, save/load |
| MetaPage | Run start | ClassSelect + DestinationSelect + UnlocksGrid + AscensionPanel |

---

## VII. Layout Patterns

### Panel slide-in

Prep phase panels slide in from the left edge over the tower cross-section. The tower remains visible (dimmed) behind. Panels are max 40% screen width. Multiple panels can stack (inventory over tower editor) but no more than 2 deep.

```
Animation: slide from left, 200ms, decel easing
Close: slide out left, 150ms, accel easing
Backdrop: none (tower visible behind, slightly dimmed)
```

### Overlay / dialog

For confirmations, detail views, full-screen takeovers (map, meta screens). Per the UI/UX doc: **no modals during combat, ever.** These overlays are prep-phase and menu only. The term "overlay" is preferred over "modal" to avoid implying combat-blocking behavior.

```
Animation: fade in 200ms + scale from 95% to 100%
Backdrop: colors.bg.overlay (85% opacity dark)
Close: fade out 150ms + scale to 95%
Max width: 600px for dialogs, full-screen for map/meta
```

Full-screen overlays (map, perk tree, inventory) dim the tower but maintain spatial continuity — the tower is reachable via Escape, never a hard screen cut.

### Tooltip positioning

Tooltips avoid overlapping the game canvas center (where the hero fights). Prefer positioning toward screen edges. Auto-flip if clipped.

### Responsive considerations

Primary target: 1920x1080 desktop. The game canvas is fixed-aspect (16:9). React UI scales with the viewport but the game canvas doesn't stretch — it letterboxes. UI panels have min/max widths to stay readable at different resolutions.

---

## VIII. State Management

```typescript
// React UI state — NOT game state (that's in Rust)
interface UIState {
  // Navigation
  currentPage: PageType;
  panelStack: PanelType[];        // open panels (max 2)

  // Selection
  selectedFloor: number | null;
  selectedCompanion: CompanionId | null;
  selectedWeapon: WeaponId | null;
  selectedNode: NodeId | null;
  hoveredNode: NodeId | null;

  // Transient
  tooltipData: TooltipData | null;
  confirmDialog: ConfirmDialogData | null;
  notification: NotificationData | null;

  // Cached from Rust (refreshed on state change)
  towerSnapshot: TowerState;
  combatSnapshot: CombatRenderState;
  journeySnapshot: JourneyState;
  heroSnapshot: HeroState;
  inventorySnapshot: InventoryState;
}
```

**State manager: plain React state/context.** No external state library. UI state (selections, panel open/closed, hover targets, cached Rust snapshots) is managed via `useState` and `useContext`. See [implementation-decisions.md §18](implementation-decisions.md) for the rationale.

```typescript
// React context for UI state — never authoritative game state
const UIContext = createContext<UIState>(initialState);

function UIProvider({ children }: { children: React.ReactNode }) {
  const [panelStack, setPanelStack] = useState<string[]>([]);
  const [selectedFloor, setSelectedFloor] = useState<number | null>(null);
  // ... UI-only state

  const value = useMemo(() => ({
    panelStack, selectedFloor,
    openPanel: (p: string) => setPanelStack(s => [...s, p].slice(-2)),
    closePanel: () => setPanelStack(s => s.slice(0, -1)),
    setSelectedFloor,
  }), [panelStack, selectedFloor]);

  return <UIContext.Provider value={value}>{children}</UIContext.Provider>;
}
```

**Rust snapshot refresh pattern:**

React syncs with Rust state differently depending on game phase:

```typescript
// Hook that syncs Rust state into React state
function useRustSync(phase: 'prep' | 'combat') {
  const rafRef = useRef<number>();
  const [combatSnapshot, setCombatSnapshot] = useState<HudSnapshot | null>(null);

  useEffect(() => {
    if (phase === 'combat') {
      // Combat: sync every frame via requestAnimationFrame.
      // React HUD elements (ammo, cooldowns, wave) must stay in sync.
      const tick = () => {
        const snapshot = JSON.parse(bridge.get_hud_state());
        setCombatSnapshot(snapshot);
        rafRef.current = requestAnimationFrame(tick);
      };
      rafRef.current = requestAnimationFrame(tick);
      return () => { if (rafRef.current) cancelAnimationFrame(rafRef.current); };
    } else {
      // Prep: sync on command, not on interval. Each prep action
      // sends a command to Rust and reads the updated snapshot.
      // No polling needed — avoids wasted JSON serialization.
      return () => {};
    }
  }, [phase]);
}
```

**Why not setInterval:** polling at a fixed interval wastes serialization during prep (nothing changed) and is too slow during combat (100ms is 6 missed frames at 60fps). Frame-sync during combat and command-response during prep are both more correct and cheaper.

---

## IX. Accessibility

- **Keyboard navigation** for all prep phase controls. Tab order follows tower floor order (bottom to top). Arrow keys navigate within panels.
- **Color is never the only indicator.** Resource types have distinct icons AND colors. Status has text labels AND color. Rarity has text AND color.
- **Tooltips on everything interactive.** No unlabeled icon buttons.
- **Font sizes respect user settings.** Typography scale can be adjusted in settings (+/- 2 steps).
- **Reduced motion setting.** Disables: panel slide animations, combat HUD transitions, ambient pulses. Does NOT affect: game canvas animations (those are in Rust).
- **Contrast ratio.** All text meets WCAG AA (4.5:1) against its background. Use `colors.text.primary` on `colors.bg.surface` = ~12:1 ratio.

---

## X. File Structure

```
src/
├── ui/
│   ├── tokens/
│   │   ├── colors.ts
│   │   ├── spacing.ts
│   │   ├── typography.ts
│   │   ├── radii.ts
│   │   ├── shadows.ts
│   │   ├── animation.ts
│   │   ├── zIndex.ts
│   │   └── index.ts          // re-exports all tokens
│   │
│   ├── atoms/
│   │   ├── Text.tsx
│   │   ├── Icon.tsx
│   │   ├── ResourceIcon.tsx
│   │   ├── Badge.tsx
│   │   ├── ProgressBar.tsx
│   │   ├── VerticalBar.tsx
│   │   ├── Button.tsx
│   │   ├── IconButton.tsx
│   │   ├── Tooltip.tsx
│   │   ├── Chip.tsx
│   │   ├── Divider.tsx
│   │   └── index.ts
│   │
│   ├── molecules/
│   │   ├── ResourceCount.tsx
│   │   ├── ResourceBar.tsx
│   │   ├── StatLine.tsx
│   │   ├── WeaponCard.tsx
│   │   ├── TrinketCard.tsx
│   │   ├── CompanionPortrait.tsx
│   │   ├── PanelHealth.tsx
│   │   ├── CooldownTimer.tsx
│   │   ├── TickCounter.tsx
│   │   ├── WaveIndicator.tsx
│   │   ├── AmmoRack.tsx
│   │   ├── NodeMarker.tsx
│   │   ├── BufferDisplay.tsx
│   │   ├── OrderSelector.tsx
│   │   ├── LootItem.tsx
│   │   └── index.ts
│   │
│   ├── organisms/
│   │   ├── combat/
│   │   │   ├── CombatHUD.tsx
│   │   │   ├── AmmoPanel.tsx
│   │   │   ├── AbilityBar.tsx
│   │   │   ├── CompanionStatusRow.tsx
│   │   │   └── DamageOverlay.tsx
│   │   ├── prep/
│   │   │   ├── TowerEditor.tsx
│   │   │   ├── FloorPanel.tsx
│   │   │   ├── BuildMenu.tsx
│   │   │   ├── TransportPanel.tsx
│   │   │   ├── LiftProgrammer.tsx
│   │   │   ├── CompanionAssigner.tsx
│   │   │   ├── OrdersPanel.tsx
│   │   │   ├── RepairQueue.tsx
│   │   │   └── ValidationWarnings.tsx
│   │   ├── map/
│   │   │   ├── ChapterMap.tsx
│   │   │   ├── MapNode.tsx
│   │   │   ├── PathLine.tsx
│   │   │   ├── RoutePreview.tsx
│   │   │   └── NodeDetail.tsx
│   │   ├── inventory/
│   │   │   ├── WeaponList.tsx
│   │   │   ├── WeaponCompare.tsx
│   │   │   ├── EnchanterPanel.tsx
│   │   │   ├── TrinketList.tsx
│   │   │   └── MaterialsPanel.tsx
│   │   ├── merchant/
│   │   │   ├── ShopView.tsx
│   │   │   ├── BuySellPanel.tsx
│   │   │   └── MerchantDialogue.tsx
│   │   ├── companion/
│   │   │   ├── CompanionRoster.tsx
│   │   │   ├── CompanionDetail.tsx
│   │   │   └── RecruitPanel.tsx
│   │   ├── hero/
│   │   │   ├── HeroStats.tsx
│   │   │   ├── PerkTree.tsx
│   │   │   └── HeroSummary.tsx
│   │   ├── meta/
│   │   │   ├── ClassSelect.tsx
│   │   │   ├── DestinationSelect.tsx
│   │   │   ├── UnlocksGrid.tsx
│   │   │   └── AscensionPanel.tsx
│   │   └── shared/
│   │       ├── PostCombatSummary.tsx
│   │       ├── ConfirmDialog.tsx
│   │       └── ResourceSummary.tsx
│   │
│   ├── pages/
│   │   ├── CombatPage.tsx
│   │   ├── PrepPage.tsx
│   │   ├── MapPage.tsx
│   │   ├── MerchantPage.tsx
│   │   ├── InventoryPage.tsx
│   │   ├── CompanionPage.tsx
│   │   ├── PostCombatPage.tsx
│   │   ├── HeroPage.tsx
│   │   ├── MenuPage.tsx
│   │   └── MetaPage.tsx
│   │
│   ├── hooks/
│   │   └── useGameCommand.ts  // sends commands to Rust
│   │
│   ├── audio/
│   │   └── AudioManager.ts    // Web Audio API, consumes SoundEvents
│   │
│   ├── context/               // React context (UI state only, see impl-decisions §18)
│   │
│   └── styles/
│       ├── tokens.css          // CSS custom properties (design tokens)
│       └── global.css          // reset, font imports, base styles
```
