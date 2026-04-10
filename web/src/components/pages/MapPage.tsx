import { useCallback, useState } from "react";
import { getBridge } from "../../bridge";
import type { GamePhase, HeroSnapshot, JourneySnapshot, TowerSnapshot } from "../../bridge/types";
import { t } from "../../i18n";

interface Props {
  sendCommand: (cmd: Record<string, unknown> | string) => { Ok?: null; Error?: unknown };
  refreshPhase: () => void;
  initialPhase: GamePhase;
}

const BUILDING_TYPES = [
  "Fletcher",
  "Forge",
  "Quarry",
  "Lumberyard",
  "Smelter",
  "Alchemist",
  "Enchanter",
] as const;

type BuildingType = (typeof BUILDING_TYPES)[number];

const STAT_TYPES: Array<{
  key: "Precision" | "DrawPower" | "Tempo" | "Grit" | "Salvage";
  labelKey: "stat.precision" | "stat.draw_power" | "stat.tempo" | "stat.grit" | "stat.salvage";
}> = [
  { key: "Precision", labelKey: "stat.precision" },
  { key: "DrawPower", labelKey: "stat.draw_power" },
  { key: "Tempo", labelKey: "stat.tempo" },
  { key: "Grit", labelKey: "stat.grit" },
  { key: "Salvage", labelKey: "stat.salvage" },
];

function readState() {
  const bridge = getBridge();
  return {
    journey: JSON.parse(bridge.get_journey_state()) as JourneySnapshot,
    tower: JSON.parse(bridge.get_tower_state()) as TowerSnapshot,
    hero: JSON.parse(bridge.get_hero_state()) as HeroSnapshot,
    warnings: JSON.parse(bridge.get_validation_warnings()) as ValidationWarning[],
    gold: bridge.get_gold(),
    phase: bridge.get_phase() as GamePhase,
  };
}

interface ValidationWarning {
  severity: "Info" | "Warning" | "Critical";
  message: string;
}

export function MapPage({ sendCommand, refreshPhase, initialPhase }: Props) {
  const [state, setState] = useState(() => ({ ...readState(), phase: initialPhase }));
  const [selectedBuilding, setSelectedBuilding] = useState<BuildingType>("Fletcher");

  const refresh = useCallback(() => {
    setState(readState());
  }, []);

  const { journey, tower, hero, gold, phase } = state;
  const chapter = journey.chapters[journey.current_chapter - 1];
  if (!chapter) return <div>{t("map.no_chapter")}</div>;

  const reachableNodes = chapter.edges
    .filter((e) => e.from === journey.current_node)
    .map((e) => e.to);

  const currentNode = chapter.nodes.find((n) => n.id === journey.current_node);

  return (
    <div className="map-page">
      <div className="map-header">
        <h2>{t("map.chapter", { chapter: journey.current_chapter })}</h2>
        <div className="resources">
          <span className="gold">{t("map.gold", { gold })}</span>
        </div>
      </div>

      <div className="map-nodes">
        {chapter.nodes.map((node) => {
          const isReachable = reachableNodes.includes(node.id);
          const isCurrent = node.id === journey.current_node;
          const isVisited = journey.visited_nodes.includes(node.id);

          return (
            <button
              key={node.id}
              className={`map-node ${isCurrent ? "current" : ""} ${isVisited ? "visited" : ""} ${isReachable ? "reachable" : ""}`}
              disabled={!isReachable || phase !== "MapView"}
              onClick={() => {
                sendCommand({ SelectNode: { node: node.id } });
                refresh();
                refreshPhase();
              }}
            >
              <div className="node-type">{nodeTypeLabel(node.node_type)}</div>
              {node.difficulty != null && (
                <div className="node-diff">{"⚔".repeat(node.difficulty)}</div>
              )}
              {isCurrent && <div className="node-marker">▶</div>}
            </button>
          );
        })}
      </div>

      {phase === "Travel" && (
        <div className="prep-controls">
          <h3>{t("map.prep.title", { node: nodeTypeLabel(currentNode?.node_type) })}</h3>

          {state.warnings.length > 0 && (
            <div className="validation-warnings">
              {state.warnings.map((w, i) => (
                <div key={i} className={`warning warning-${w.severity.toLowerCase()}`}>
                  {w.message}
                </div>
              ))}
            </div>
          )}

          <p className="hero-line">
            {t("map.hero.label", {
              class: hero.class,
              level: hero.level,
              xp: hero.xp,
              points: hero.stats.unspent_points,
            })}
          </p>
          {hero.stats.unspent_points > 0 && (
            <div className="stat-allocate">
              {STAT_TYPES.map((s) => (
                <button
                  key={s.key}
                  onClick={() => {
                    sendCommand({ AllocateStat: { stat: s.key } });
                    refresh();
                  }}
                >
                  {t("map.hero.allocate", { stat: t(s.labelKey) })}
                </button>
              ))}
            </div>
          )}

          <div className="tower-info">
            <p>{t("map.tower.floors", { count: tower.floors.length })}</p>
            {tower.floors.map((floor) => (
              <div key={floor.index} className="floor-row">
                {t("map.floor.line", {
                  index: floor.index,
                  material: materialLabel(floor.material),
                  building: floor.building
                    ? t("map.floor.building", {
                        building: buildingLabel(floor.building.building_type),
                      })
                    : t("map.floor.empty"),
                  hp: Math.round(floor.panel_hp_fraction * 100),
                })}
              </div>
            ))}
          </div>

          <div className="prep-actions">
            <button
              onClick={() => {
                sendCommand({ BuildFloor: { material: "Wood" } });
                refresh();
              }}
            >
              {t("map.build.wood_floor")}
            </button>
            <button
              onClick={() => {
                sendCommand({ BuildFloor: { material: "Stone" } });
                refresh();
              }}
            >
              {t("map.build.stone_floor")}
            </button>

            <select
              value={selectedBuilding}
              onChange={(e) => setSelectedBuilding(e.target.value as BuildingType)}
            >
              {BUILDING_TYPES.map((b) => (
                <option key={b} value={b}>
                  {buildingLabel(b)}
                </option>
              ))}
            </select>

            {tower.floors.map(
              (floor) =>
                !floor.building && (
                  <button
                    key={`build-${floor.index}`}
                    onClick={() => {
                      sendCommand({
                        PlaceBuilding: {
                          floor: floor.index,
                          building_type: selectedBuilding,
                        },
                      });
                      refresh();
                    }}
                  >
                    {t("map.build.place_on", {
                      building: buildingLabel(selectedBuilding),
                      floor: floor.index,
                    })}
                  </button>
                ),
            )}
          </div>

          <button
            className="march-button"
            onClick={() => {
              sendCommand("March");
              refreshPhase();
            }}
          >
            {t("map.march")}
          </button>
        </div>
      )}
    </div>
  );
}

function nodeTypeLabel(
  nodeType: JourneySnapshot["chapters"][number]["nodes"][number]["node_type"] | undefined,
) {
  switch (nodeType) {
    case "Combat":
      return t("map.node.combat");
    case "Boss":
      return t("map.node.boss");
    case "Merchant":
      return t("map.node.merchant");
    case "Camp":
      return t("map.node.camp");
    case "Recruit":
      return t("map.node.recruit");
    case "Event":
      return t("map.node.event");
    default:
      return "";
  }
}

function materialLabel(material: TowerSnapshot["floors"][number]["material"]) {
  switch (material) {
    case "Wood":
      return t("map.material.wood");
    case "Stone":
      return t("map.material.stone");
    default:
      return material;
  }
}

function buildingLabel(buildingType: string): string {
  switch (buildingType) {
    case "Fletcher":
      return t("map.building.fletcher");
    case "Forge":
      return t("map.building.forge");
    case "Quarry":
      return t("map.building.quarry");
    case "Lumberyard":
      return t("map.building.lumberyard");
    case "Smelter":
      return t("map.building.smelter");
    case "Alchemist":
      return t("map.building.alchemist");
    case "Enchanter":
      return t("map.building.enchanter");
    default:
      return buildingType;
  }
}
