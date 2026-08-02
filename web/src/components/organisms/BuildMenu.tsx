import type { EconomySnapshot } from "../../bridge/types";

interface Props {
  economy: EconomySnapshot;
  drillActive: boolean;
  onBuildFloor: (material: "Wood" | "Stone") => void;
  onHireRunner: () => void;
  onRunDrill: () => void;
  /** Enter place mode for a building. The viz becomes a slot picker. */
  onStartPlaceBuilding: (buildingType: BuildingKey, widthSlots: number) => void;
  onStartPlaceCache: () => void;
  onStartPlaceLadder: () => void;
  onStartPlaceDumbwaiter: () => void;
}

/**
 * Static catalogue of every v1 building. Mirrors the registry data
 * files in `assets/data/entities/buildings/` so the UI can show what
 * each building does, what it costs, and which weapon ammo it feeds.
 *
 * If you add a new building to the registry, add it here too — the
 * registry is the source of truth for the simulation but this catalog
 * powers the build menu and serves as the on-screen production
 * reference for playtesters.
 */
type BuildingKey =
  | "Fletcher"
  | "Forge"
  | "Quarry"
  | "Lumberyard"
  | "Smelter"
  | "Alchemist"
  | "Enchanter";

interface BuildingCatalogEntry {
  key: BuildingKey;
  label: string;
  tier: "T1" | "T2";
  /** Resources this building consumes per craft (empty for raw producers). */
  inputs: { resource: string; icon: string }[];
  output: string;
  outputIcon: string;
  servesWeapon: string | null;
  buildTicks: number;
  buildResource: "Wood" | "Stone";
  buildResourceCost: number;
  rate: number;
  bufferMax: number;
  /** Floor slots the building occupies. T1 = 2, T2 = 3. */
  widthSlots: number;
  description: string;
}

const BUILDING_CATALOG: BuildingCatalogEntry[] = [
  {
    key: "Lumberyard",
    label: "Lumberyard",
    tier: "T1",
    widthSlots: 2,
    inputs: [],
    output: "Wood",
    outputIcon: "🪵",
    servesWeapon: null,
    buildTicks: 2,
    buildResource: "Stone",
    buildResourceCost: 1,
    rate: 4.0,
    bufferMax: 3,
    description: "Raw producer. Wood feeds Fletcher.",
  },
  {
    key: "Quarry",
    label: "Quarry",
    tier: "T1",
    widthSlots: 2,
    inputs: [],
    output: "Stone",
    outputIcon: "🪨",
    servesWeapon: null,
    buildTicks: 2,
    buildResource: "Wood",
    buildResourceCost: 1,
    rate: 4.0,
    bufferMax: 3,
    description: "Raw producer. Stone feeds Forge.",
  },
  {
    key: "Fletcher",
    label: "Fletcher",
    tier: "T1",
    widthSlots: 2,
    inputs: [{ resource: "Wood", icon: "🪵" }],
    output: "Arrows",
    outputIcon: "🏹",
    servesWeapon: "Bow",
    buildTicks: 2,
    buildResource: "Wood",
    buildResourceCost: 2,
    rate: 6.0,
    bufferMax: 4,
    description: "Consumes wood. Crafts arrows for bows.",
  },
  {
    key: "Forge",
    label: "Forge",
    tier: "T1",
    widthSlots: 2,
    inputs: [{ resource: "Stone", icon: "🪨" }],
    output: "Bolts",
    outputIcon: "⚒",
    servesWeapon: "Crossbow",
    buildTicks: 2,
    buildResource: "Stone",
    buildResourceCost: 2,
    rate: 5.0,
    bufferMax: 4,
    description: "Consumes stone. Forges bolts for crossbows.",
  },
  {
    key: "Smelter",
    label: "Smelter",
    tier: "T2",
    widthSlots: 3,
    inputs: [],
    output: "Iron",
    outputIcon: "🔩",
    servesWeapon: null,
    buildTicks: 3,
    buildResource: "Stone",
    buildResourceCost: 3,
    rate: 2.0,
    bufferMax: 3,
    description: "Smelts iron for advanced gear (WIP).",
  },
  {
    key: "Alchemist",
    label: "Alchemist",
    tier: "T2",
    widthSlots: 3,
    inputs: [],
    output: "Mana",
    outputIcon: "✨",
    servesWeapon: "Staff",
    buildTicks: 3,
    buildResource: "Stone",
    buildResourceCost: 3,
    rate: 1.5,
    bufferMax: 3,
    description: "Distills mana for staves (WIP).",
  },
  {
    key: "Enchanter",
    label: "Enchanter",
    tier: "T2",
    widthSlots: 3,
    inputs: [],
    output: "Planks",
    outputIcon: "🔮",
    servesWeapon: null,
    buildTicks: 4,
    buildResource: "Wood",
    buildResourceCost: 3,
    rate: 0.5,
    bufferMax: 2,
    description: "Enchants planks used by trinket recipes (WIP).",
  },
];

const FLOOR_COST_TICKS = 2;
const FLOOR_COST_WOOD = 3;
const FLOOR_COST_STONE = 4;
const CACHE_COST_TICKS = 2;
const HIRE_COST_GOLD = 10;
const LADDER_COST_TICKS = 1;
const LADDER_COST_WOOD = 1;
const DUMBWAITER_COST_TICKS = 2;
const DUMBWAITER_COST_WOOD = 2;

export function BuildMenu({
  economy,
  drillActive,
  onBuildFloor,
  onHireRunner,
  onRunDrill,
  onStartPlaceBuilding,
  onStartPlaceCache,
  onStartPlaceLadder,
  onStartPlaceDumbwaiter,
}: Props) {
  const woodAvailable = economy.materials.find((m) => m.resource === "Wood")?.current ?? 0;
  const stoneAvailable = economy.materials.find((m) => m.resource === "Stone")?.current ?? 0;

  const canBuild = (entry: BuildingCatalogEntry) => {
    if (economy.ticks_remaining < entry.buildTicks) return false;
    const stock = entry.buildResource === "Wood" ? woodAvailable : stoneAvailable;
    return stock >= entry.buildResourceCost;
  };

  const canBuildFloor = (material: "Wood" | "Stone") => {
    if (economy.ticks_remaining < FLOOR_COST_TICKS) return false;
    const need = material === "Wood" ? FLOOR_COST_WOOD : FLOOR_COST_STONE;
    const stock = material === "Wood" ? woodAvailable : stoneAvailable;
    return stock >= need;
  };

  return (
    <div className="build-menu">
      {/* Top bar: floor + cache + runner — the global actions */}
      <div className="build-menu-top">
        <button
          type="button"
          className="build-action"
          disabled={!canBuildFloor("Wood")}
          onClick={() => onBuildFloor("Wood")}
        >
          + Wood Floor
          <span className="cost">
            {FLOOR_COST_TICKS}t / {FLOOR_COST_WOOD} 🪵
          </span>
        </button>
        <button
          type="button"
          className="build-action"
          disabled={!canBuildFloor("Stone")}
          onClick={() => onBuildFloor("Stone")}
        >
          + Stone Floor
          <span className="cost">
            {FLOOR_COST_TICKS}t / {FLOOR_COST_STONE} 🪨
          </span>
        </button>
        <button
          type="button"
          className="build-action"
          disabled={economy.gold < HIRE_COST_GOLD}
          onClick={onHireRunner}
        >
          + Hire Runner
          <span className="cost">{HIRE_COST_GOLD}g</span>
        </button>
        <button
          type="button"
          className="build-action drill-button"
          disabled={drillActive || economy.ticks_remaining < 1}
          onClick={onRunDrill}
          title="Run a 30s logistics drill in prep mode. Hero rack drains 1/s while runners feed it. XP per delivery."
        >
          🛠 Run Drill
          <span className="cost">1t · 30s · +xp</span>
        </button>
      </div>

      {/* Transport + cache placement — these all enter place mode and
          let the player click the slot grid in the tower viz. */}
      <div className="cache-row">
        <span className="cache-row-label">Place:</span>
        <button
          type="button"
          className="build-action"
          disabled={economy.ticks_remaining < CACHE_COST_TICKS}
          onClick={onStartPlaceCache}
          title="Cache: 1 slot wide. Click an empty slot in the tower viz to place."
        >
          + Cache
          <span className="cost">{CACHE_COST_TICKS}t</span>
        </button>
        <button
          type="button"
          className="build-action"
          disabled={economy.ticks_remaining < LADDER_COST_TICKS || woodAvailable < LADDER_COST_WOOD}
          onClick={onStartPlaceLadder}
          title="Ladder: spans 2 floors at the chosen column. Speed 0.7x, capacity 1."
        >
          + Ladder
          <span className="cost">
            {LADDER_COST_TICKS}t / {LADDER_COST_WOOD} 🪵
          </span>
        </button>
        <button
          type="button"
          className="build-action"
          disabled={
            economy.ticks_remaining < DUMBWAITER_COST_TICKS || woodAvailable < DUMBWAITER_COST_WOOD
          }
          onClick={onStartPlaceDumbwaiter}
          title="Dumbwaiter: spans 2 floors at the chosen column. Speed 0.8x, autonomous."
        >
          + Dumbwaiter
          <span className="cost">
            {DUMBWAITER_COST_TICKS}t / {DUMBWAITER_COST_WOOD} 🪵
          </span>
        </button>
      </div>

      {/* Building catalogue: click a card to enter place mode, then
          pick a slot in the tower viz. */}
      <div className="build-catalog">
        <h4 className="build-catalog-title">
          Buildings <span className="build-catalog-hint">click a card → pick a slot</span>
        </h4>
        <div className="build-cards">
          {BUILDING_CATALOG.map((entry) => {
            const buildable = canBuild(entry);
            return (
              <button
                key={entry.key}
                type="button"
                disabled={!buildable}
                className={`build-card tier-${entry.tier.toLowerCase()} ${buildable ? "" : "unaffordable"}`}
                onClick={() => onStartPlaceBuilding(entry.key, entry.widthSlots)}
              >
                <span className="build-card-header">
                  <span className="build-card-name">{entry.label}</span>
                  <span className="build-card-tier">{entry.tier}</span>
                </span>
                <span className="build-card-flow">
                  {entry.inputs.length > 0 && (
                    <>
                      <span className="build-card-input">
                        {entry.inputs.map((inp) => `${inp.icon} ${inp.resource}`).join(" + ")}
                      </span>
                      <span className="build-card-arrow">→</span>
                    </>
                  )}
                  <span className="build-card-output">
                    {entry.outputIcon} {entry.output}
                  </span>
                  {entry.servesWeapon && (
                    <span className="build-card-weapon">→ {entry.servesWeapon}</span>
                  )}
                </span>
                <span className="build-card-stats">
                  <span>{entry.rate}/min</span>
                  <span>buf {entry.bufferMax}</span>
                  <span>w {entry.widthSlots}</span>
                </span>
                <span className="build-card-cost">
                  {entry.buildTicks}t /{" "}
                  {entry.buildResource === "Wood"
                    ? `${entry.buildResourceCost} 🪵`
                    : `${entry.buildResourceCost} 🪨`}
                </span>
                <span className="build-card-desc">{entry.description}</span>
              </button>
            );
          })}
        </div>
      </div>
    </div>
  );
}
