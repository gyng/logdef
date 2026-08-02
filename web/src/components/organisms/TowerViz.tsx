import type { CSSProperties, MouseEvent as ReactMouseEvent } from "react";

import type {
  BalconySnapshot,
  CompanionSnapshot,
  FloorSnapshot,
  RunnerSnapshot,
  TowerSnapshot,
  TransportInstanceSnapshot,
} from "../../bridge/types";

/** Active "place mode": player has clicked a build card and is
 *  picking a slot in the tower. The viz renders ghosts on every
 *  empty slot range that fits the request. */
export interface PlaceModeState {
  kind: "building" | "cache" | "ladder" | "dumbwaiter";
  /** Width of the ghost in slots. Cache/transport columns = 1. */
  width: number;
  /** For building placements, which building to place. */
  buildingKey?: string;
  /** Restrict to a specific floor (transports = low_floor). */
  floor?: number;
  /** Display label shown in the ghost popup. */
  label: string;
}

/** Result of clicking an empty slot in place mode. */
export interface PlacementChoice {
  floor: number;
  slot: number;
}

interface Props {
  tower: TowerSnapshot;
  heroBalconyId: number;
  title?: string;
  /** When set, the tower viz becomes a click target for slot picking. */
  placeMode?: PlaceModeState | null;
  /** Pending placement (the user clicked a slot, popup is showing). */
  pendingChoice?: PlacementChoice | null;
  onSlotClick?: (choice: PlacementChoice) => void;
  onConfirm?: () => void;
  onCancel?: () => void;
}

// Slot layout — must match SLOT_PX in transport.rs.
const SLOT_PX = 40;
const FLOOR_HEIGHT = 96;
const BALCONY_W = 60;
const FOUNDATION_H = 60;
const PADDING_X = 20;
// Right margin for floor index labels and breathing room.
const PADDING_RIGHT = 20;

function floorWidthPx(floor: FloorSnapshot): number {
  return floor.slots * SLOT_PX;
}

function maxFloorWidth(tower: TowerSnapshot): number {
  return tower.floors.reduce((max, f) => Math.max(max, floorWidthPx(f)), 0);
}

/** Convert (slot, width) into the x/width pair for an SVG rect. */
function slotRect(slot: number, width: number): { x: number; w: number } {
  return { x: PADDING_X + slot * SLOT_PX, w: width * SLOT_PX };
}

/**
 * Side-view of the tower with explicit slot grid. Each floor is
 * partitioned into N horizontal slots; buildings/caches/transports
 * occupy contiguous slot ranges. Runners walk L-shaped paths through
 * the grid using transport columns to climb between floors.
 */
export function TowerViz({
  tower,
  heroBalconyId,
  title,
  placeMode,
  pendingChoice,
  onSlotClick,
  onConfirm,
  onCancel,
}: Props) {
  const floorCount = Math.max(tower.floors.length, 1);
  const innerW = maxFloorWidth(tower);
  const svgW = PADDING_X + innerW + BALCONY_W + PADDING_RIGHT;
  const svgH = floorCount * FLOOR_HEIGHT + FOUNDATION_H + 12;

  return (
    <div className="tower-viz">
      {title && <div className="tower-viz-title">{title}</div>}
      <svg
        viewBox={`0 0 ${svgW} ${svgH}`}
        width="100%"
        style={{ maxWidth: svgW, display: "block" }}
        role="img"
        aria-label="Tower cross-section"
      >
        {/* Floors (top of svg = top floor) */}
        {tower.floors.map((floor) => {
          const yTop = (floorCount - 1 - floor.index) * FLOOR_HEIGHT;
          const balcony = tower.balconies.find((b) => b.floor === floor.index);
          const isHeroFloor = balcony?.id === heroBalconyId;
          const companionOnBalcony = balcony
            ? tower.companions.find((c) => c.position === balcony.id)
            : undefined;
          return (
            <FloorView
              key={floor.index}
              floor={floor}
              balcony={balcony}
              yTop={yTop}
              isHeroFloor={isHeroFloor}
              companion={companionOnBalcony}
              innerW={innerW}
            />
          );
        })}

        {/* Foundation depot strip */}
        <g transform={`translate(0, ${floorCount * FLOOR_HEIGHT})`}>
          <rect
            x={PADDING_X}
            y={0}
            width={innerW}
            height={FOUNDATION_H}
            className="foundation-rect"
          />
          <text x={PADDING_X + 8} y={14} className="depot-label">
            Depot (overflow):
          </text>
          {tower.warehouse.slots.length === 0 ? (
            <text x={PADDING_X + 8} y={28} className="depot-empty">
              empty
            </text>
          ) : (
            tower.warehouse.slots.map((slot, i) => (
              <text key={i} x={PADDING_X + 8 + i * 70} y={28} className="depot-slot">
                {resourceIcon(slot.resource)} {slot.current}/{slot.max}
              </text>
            ))
          )}
          <text
            x={PADDING_X + innerW - 8}
            y={FOUNDATION_H - 6}
            textAnchor="end"
            className="foundation-label"
          >
            chicken legs
          </text>
        </g>

        {/* Transport columns spanning their floor range */}
        {tower.transports.map((t) => (
          <TransportColumn key={t.id} transport={t} floorCount={floorCount} />
        ))}

        {/* Runners — sit at facility positions, walk L-shaped paths */}
        {tower.runners.map((runner) => (
          <RunnerDot key={runner.id} runner={runner} tower={tower} floorCount={floorCount} />
        ))}

        {/* Slot grid overlay (place mode) — drawn LAST so it sits on
            top of buildings/transports, with high contrast so the
            player can see exactly which slots are valid. */}
        {placeMode &&
          tower.floors.map((floor) => (
            <SlotGrid
              key={`grid-${floor.index}`}
              floor={floor}
              floorCount={floorCount}
              tower={tower}
              placeMode={placeMode}
              onSlotClick={onSlotClick}
            />
          ))}

        {/* Pending placement ghost + popup */}
        {placeMode && pendingChoice && (
          <PlacementGhost
            placeMode={placeMode}
            choice={pendingChoice}
            floorCount={floorCount}
            onConfirm={onConfirm}
            onCancel={onCancel}
          />
        )}
      </svg>
    </div>
  );
}

interface FloorViewProps {
  floor: FloorSnapshot;
  balcony: BalconySnapshot | undefined;
  yTop: number;
  isHeroFloor: boolean;
  companion: CompanionSnapshot | undefined;
  innerW: number;
}

function FloorView({ floor, balcony, yTop, isHeroFloor, companion, innerW }: FloorViewProps) {
  const hp = Math.max(0, Math.min(1, floor.panel_hp_fraction));
  const wallWidth = floorWidthPx(floor);
  return (
    <g transform={`translate(0, ${yTop})`}>
      {/* Floor base (full inner width) */}
      <rect
        x={PADDING_X}
        y={0}
        width={Math.max(wallWidth, innerW)}
        height={FLOOR_HEIGHT}
        className={`floor-wall floor-wall-${floor.material.toLowerCase()}`}
      />
      {/* Panel HP fill on the leftmost wall edge */}
      <rect
        x={PADDING_X + 2}
        y={2 + (FLOOR_HEIGHT - 4) * (1 - hp)}
        width={4}
        height={(FLOOR_HEIGHT - 4) * hp}
        className="floor-wall-hp"
      />

      {/* Building at its slot range */}
      {floor.building && <BuildingChain building={floor.building} />}

      {/* Cache at its slot */}
      {floor.cache && floor.cache.slots[0] && <CacheBox cache={floor.cache} />}

      {/* Balcony jutting to the right + rack */}
      {balcony && (
        <g transform={`translate(${PADDING_X + innerW}, ${FLOOR_HEIGHT - 60})`}>
          <rect
            x={0}
            y={0}
            width={BALCONY_W - 8}
            height={60}
            className={`balcony-rect ${balcony.rack.destroyed ? "destroyed" : ""}`}
          />
          {!balcony.rack.destroyed && (
            <rect
              x={4}
              y={4 + 52 * (1 - rackFraction(balcony))}
              width={BALCONY_W - 16}
              height={52 * rackFraction(balcony)}
              className="rack-fill"
            />
          )}
          <text x={(BALCONY_W - 8) / 2} y={-3} textAnchor="middle" className="rack-label">
            🏹{balcony.rack.current}/{balcony.rack.max}
          </text>
          {isHeroFloor && (
            <g transform={`translate(${(BALCONY_W - 8) / 2}, 30)`}>
              <circle r={6} className="hero-icon" />
              <text x={0} y={-9} textAnchor="middle" className="hero-label">
                hero
              </text>
            </g>
          )}
          {!isHeroFloor && companion && (
            <g transform={`translate(${(BALCONY_W - 8) / 2}, 30)`}>
              <circle r={5} className={`companion-icon ${companion.injured ? "injured" : ""}`} />
              <text x={0} y={-7} textAnchor="middle" className="companion-label">
                {companion.name}
              </text>
            </g>
          )}
        </g>
      )}

      {/* Floor index label outside the wall on the left */}
      <text x={PADDING_X - 6} y={FLOOR_HEIGHT - 6} textAnchor="end" className="floor-num">
        F{floor.index}
      </text>
    </g>
  );
}

function rackFraction(balcony: BalconySnapshot): number {
  if (balcony.rack.max === 0) return 0;
  return Math.min(1, balcony.rack.current / balcony.rack.max);
}

interface BuildingChainProps {
  building: NonNullable<FloorSnapshot["building"]>;
}

/** A building rendered into its (slot, width_slots) footprint with
 *  inbox(es) on the left, body+progress in the middle, outbox on the
 *  right — all proportional to its width. */
function BuildingChain({ building }: BuildingChainProps) {
  const isStalled =
    building.input_buffers.length > 0 && building.input_buffers.some((inb) => inb.current === 0);
  const progressPct = Math.min(1, Math.max(0, building.production_progress));
  const { x, w } = slotRect(building.slot, building.width_slots);
  // Sub-regions inside the building footprint.
  const sectionH = 56;
  const yTop = FLOOR_HEIGHT - 64;
  const inboxW = building.input_buffers.length > 0 ? w * 0.22 : w * 0.18;
  const outboxW = w * 0.22;
  const bodyX = inboxW;
  const bodyW = w - inboxW - outboxW;
  const outboxX = w - outboxW;

  return (
    <g transform={`translate(${x}, ${yTop})`}>
      {/* Inbox(es) */}
      {building.input_buffers.length === 0 ? (
        <g>
          <rect x={2} y={0} width={inboxW - 4} height={sectionH} rx={3} className="inbox-none" />
          <text x={inboxW / 2} y={sectionH / 2 + 4} textAnchor="middle" className="inbox-label">
            raw
          </text>
        </g>
      ) : (
        building.input_buffers.map((inb, i) => {
          const slotH = sectionH / building.input_buffers.length - 2;
          const yOff = i * (sectionH / building.input_buffers.length);
          const fillFrac = inb.max === 0 ? 0 : inb.current / inb.max;
          return (
            <g key={i} transform={`translate(2, ${yOff})`}>
              <rect x={0} y={0} width={inboxW - 4} height={slotH} rx={2} className="inbox-rect" />
              <rect
                x={1}
                y={1 + (slotH - 2) * (1 - fillFrac)}
                width={inboxW - 6}
                height={(slotH - 2) * fillFrac}
                className="inbox-fill"
              />
              <text
                x={(inboxW - 4) / 2}
                y={slotH / 2 + 4}
                textAnchor="middle"
                className="inbox-label"
              >
                {resourceIcon(inb.resource)} {inb.current}/{inb.max}
              </text>
            </g>
          );
        })
      )}

      {/* Body */}
      <g transform={`translate(${bodyX}, 0)`}>
        <rect
          x={0}
          y={0}
          width={bodyW}
          height={sectionH}
          rx={3}
          className={`building-body ${isStalled ? "stalled" : ""}`}
        />
        <text x={bodyW / 2} y={18} textAnchor="middle" className="building-name">
          {buildingShortLabel(building.building_type)}
        </text>
        <rect x={6} y={26} width={bodyW - 12} height={6} className="progress-track" />
        <rect
          x={6}
          y={26}
          width={(bodyW - 12) * progressPct}
          height={6}
          className={isStalled ? "progress-fill stalled" : "progress-fill"}
        />
        <text x={bodyW / 2} y={45} textAnchor="middle" className="building-substatus">
          {isStalled
            ? "needs input"
            : building.is_active
              ? `${Math.round(progressPct * 100)}%`
              : "idle"}
        </text>
      </g>

      {/* Outbox */}
      <g transform={`translate(${outboxX}, 0)`}>
        <rect x={2} y={0} width={outboxW - 4} height={sectionH} rx={3} className="outbox-rect" />
        <rect
          x={3}
          y={
            1 +
            (sectionH - 2) *
              (1 -
                (building.output_buffer.max === 0
                  ? 0
                  : building.output_buffer.current / building.output_buffer.max))
          }
          width={outboxW - 6}
          height={
            (sectionH - 2) *
            (building.output_buffer.max === 0
              ? 0
              : building.output_buffer.current / building.output_buffer.max)
          }
          className="outbox-fill"
        />
        <text x={outboxW / 2} y={sectionH / 2 - 2} textAnchor="middle" className="outbox-label">
          {resourceIcon(building.output_buffer.resource)}
        </text>
        <text x={outboxW / 2} y={sectionH / 2 + 12} textAnchor="middle" className="outbox-label">
          {building.output_buffer.current}/{building.output_buffer.max}
        </text>
      </g>
    </g>
  );
}

interface CacheBoxProps {
  cache: NonNullable<FloorSnapshot["cache"]>;
}

function CacheBox({ cache }: CacheBoxProps) {
  if (!cache.slots[0]) return null;
  const { x, w } = slotRect(cache.slot, 1);
  const yTop = FLOOR_HEIGHT - 64;
  return (
    <g transform={`translate(${x}, ${yTop})`}>
      <rect x={2} y={0} width={w - 4} height={56} rx={3} className="cache-icon" />
      <text x={w / 2} y={18} textAnchor="middle" className="cache-label">
        cache
      </text>
      <text x={w / 2} y={36} textAnchor="middle" className="cache-label">
        📦{cache.slots[0].current}/{cache.slots[0].max}
      </text>
    </g>
  );
}

interface TransportColumnProps {
  transport: TransportInstanceSnapshot;
  floorCount: number;
}

function TransportColumn({ transport, floorCount }: TransportColumnProps) {
  const yTop = (floorCount - 1 - transport.high_floor) * FLOOR_HEIGHT;
  const yBottom = (floorCount - transport.low_floor) * FLOOR_HEIGHT;
  const { x } = slotRect(transport.slot, 1);
  const colW = SLOT_PX - 6;
  const colX = x + 3;
  const stripH = yBottom - yTop;
  const dotSize = 5;
  const dots = Array.from({ length: transport.capacity }, (_, i) => {
    const cy = yTop + 14 + i * (dotSize * 2 + 2);
    return (
      <circle
        key={i}
        cx={colX + colW / 2}
        cy={cy}
        r={dotSize}
        className={`transport-dot ${i < transport.occupancy ? "filled" : ""}`}
      />
    );
  });
  const kindClass = `transport-strip transport-${transport.kind.toLowerCase()}`;
  return (
    <g>
      <rect x={colX} y={yTop} width={colW} height={stripH} className={kindClass} />
      {dots}
      <text x={colX + colW / 2} y={yBottom - 4} textAnchor="middle" className="transport-label">
        {transportShortLabel(transport.kind)}
      </text>
    </g>
  );
}

function transportShortLabel(kind: TransportInstanceSnapshot["kind"]): string {
  switch (kind) {
    case "Stairs":
      return "stair";
    case "Ladder":
      return "ladr";
    case "Dumbwaiter":
      return "dwtr";
    case "Chute":
      return "chut";
  }
}

interface RunnerDotProps {
  runner: RunnerSnapshot;
  tower: TowerSnapshot;
  floorCount: number;
}

function RunnerDot({ runner, floorCount }: RunnerDotProps) {
  // Where the runner is, in (floor, slot) space — a fractional point
  // when mid-Moving on a leg.
  let virtFloor = runner.current_floor;
  let virtSlot = runner.current_slot as number;
  let stateLabel = "";
  const s = runner.state;
  if ("Idle" in s) {
    virtFloor = s.Idle.at_floor;
    stateLabel = "idle";
  } else if ("Loading" in s) {
    virtFloor = s.Loading.at_floor;
    stateLabel = "load";
  } else if ("Unloading" in s) {
    virtFloor = s.Unloading.at_floor;
    stateLabel = "drop";
  } else if ("Moving" in s) {
    const t = s.Moving.progress;
    if (s.Moving.from === s.Moving.to) {
      // Horizontal leg: y stays, x interpolates between slots.
      virtFloor = s.Moving.from;
      virtSlot = s.Moving.from_slot + (s.Moving.to_slot - s.Moving.from_slot) * t;
    } else {
      // Vertical leg: x stays at the column slot, y interpolates floors.
      virtSlot = s.Moving.from_slot;
      virtFloor = s.Moving.from + (s.Moving.to - s.Moving.from) * t;
    }
    stateLabel = "→";
  } else if ("Queued" in s) {
    stateLabel = "que";
  }

  const x = PADDING_X + virtSlot * SLOT_PX + SLOT_PX / 2;
  const y = (floorCount - 1 - virtFloor) * FLOOR_HEIGHT + FLOOR_HEIGHT - 22;

  return (
    <g transform={`translate(${x}, ${y})`}>
      <circle r={7} className={`runner-dot ${runner.carried ? "carrying" : "empty"}`} />
      <text x={0} y={-10} textAnchor="middle" className="runner-dot-label">
        R{runner.id}
        {runner.carried ? "📦" : ""}
      </text>
      {stateLabel && (
        <text x={0} y={16} textAnchor="middle" className="runner-dot-state">
          {stateLabel}
        </text>
      )}
    </g>
  );
}

interface SlotGridProps {
  floor: FloorSnapshot;
  floorCount: number;
  tower: TowerSnapshot;
  placeMode: PlaceModeState;
  onSlotClick?: (choice: PlacementChoice) => void;
}

/** Renders a faint slot grid over a floor and makes empty slots
 *  clickable in place mode. */
function SlotGrid({ floor, floorCount, tower, placeMode, onSlotClick }: SlotGridProps) {
  const yTop = (floorCount - 1 - floor.index) * FLOOR_HEIGHT;
  const slotH = FLOOR_HEIGHT - 12;
  // Restrict to the requested floor when transports
  if (placeMode.floor !== undefined && placeMode.floor !== floor.index) return null;

  const slotIndices = Array.from({ length: floor.slots }, (_, i) => i);
  return (
    <g>
      {slotIndices.map((s) => {
        const occupied = isSlotOccupied(tower, floor.index, s);
        const fits = canFitAt(tower, floor.index, s, placeMode.width);
        const cellX = PADDING_X + s * SLOT_PX;
        return (
          <rect
            key={s}
            x={cellX}
            y={yTop + 6}
            width={SLOT_PX - 1}
            height={slotH}
            className={`slot-cell ${occupied ? "occupied" : ""} ${fits ? "fits" : ""}`}
            onClick={(e: ReactMouseEvent<SVGRectElement>) => {
              e.stopPropagation();
              if (fits && onSlotClick) onSlotClick({ floor: floor.index, slot: s });
            }}
            style={{ cursor: fits ? "pointer" : "not-allowed" }}
          />
        );
      })}
    </g>
  );
}

function isSlotOccupied(tower: TowerSnapshot, floorIdx: number, slot: number): boolean {
  const floor = tower.floors[floorIdx];
  if (!floor) return true;
  if (floor.building) {
    const b = floor.building;
    if (slot >= b.slot && slot < b.slot + b.width_slots) return true;
  }
  if (floor.cache && floor.cache.slot === slot) return true;
  for (const t of tower.transports) {
    if (t.low_floor <= floorIdx && t.high_floor >= floorIdx && t.slot === slot) {
      return true;
    }
  }
  return false;
}

function canFitAt(
  tower: TowerSnapshot,
  floorIdx: number,
  startSlot: number,
  width: number,
): boolean {
  const floor = tower.floors[floorIdx];
  if (!floor) return false;
  if (startSlot + width > floor.slots) return false;
  for (let s = startSlot; s < startSlot + width; s += 1) {
    if (isSlotOccupied(tower, floorIdx, s)) return false;
  }
  return true;
}

interface PlacementGhostProps {
  placeMode: PlaceModeState;
  choice: PlacementChoice;
  floorCount: number;
  onConfirm?: () => void;
  onCancel?: () => void;
}

function PlacementGhost({
  placeMode,
  choice,
  floorCount,
  onConfirm,
  onCancel,
}: PlacementGhostProps) {
  const { x, w } = slotRect(choice.slot, placeMode.width);
  const yTop = (floorCount - 1 - choice.floor) * FLOOR_HEIGHT + FLOOR_HEIGHT - 64;
  return (
    <g>
      <rect x={x + 2} y={yTop} width={w - 4} height={56} rx={3} className="placement-ghost" />
      <text x={x + w / 2} y={yTop + 30} textAnchor="middle" className="placement-ghost-label">
        {placeMode.label}
      </text>
      {/* Popup placed just above the ghost */}
      <foreignObject x={x - 30} y={yTop - 50} width={w + 60} height={48}>
        <div className="placement-popup" style={{ width: "100%" } as CSSProperties}>
          <button type="button" className="placement-ok" onClick={onConfirm}>
            ✓ Place
          </button>
          <button type="button" className="placement-cancel" onClick={onCancel}>
            ✕ Cancel
          </button>
        </div>
      </foreignObject>
    </g>
  );
}

function buildingShortLabel(type: string): string {
  switch (type) {
    case "Fletcher":
      return "Fletch";
    case "Forge":
      return "Forge";
    case "Quarry":
      return "Quarry";
    case "Lumberyard":
      return "Wood";
    case "Smelter":
      return "Smelt";
    case "Alchemist":
      return "Alch";
    case "Enchanter":
      return "Ench";
    default:
      return type;
  }
}

function resourceIcon(resource: string): string {
  switch (resource) {
    case "Wood":
      return "🪵";
    case "Stone":
      return "🪨";
    case "Iron":
      return "🔩";
    case "Arrows":
      return "🏹";
    case "Bolts":
      return "⚒";
    case "Mana":
      return "✨";
    case "Thrown":
      return "💣";
    case "Planks":
      return "🪟";
    case "Gold":
      return "🪙";
    default:
      return "?";
  }
}
