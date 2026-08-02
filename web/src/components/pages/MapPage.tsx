import { useCallback, useEffect, useRef, useState } from "react";
import { getBridge } from "../../bridge";
import type {
  DrillSnapshot,
  EconomySnapshot,
  GamePhase,
  HeroSnapshot,
  JourneySnapshot,
  RunnerSnapshot,
  TowerSnapshot,
} from "../../bridge/types";
import { t } from "../../i18n";
import { BuildMenu } from "../organisms/BuildMenu";
import { TowerViz, type PlaceModeState, type PlacementChoice } from "../organisms/TowerViz";

interface Props {
  sendCommand: (cmd: Record<string, unknown> | string) => { Ok?: null; Error?: unknown };
  refreshPhase: () => void;
  initialPhase: GamePhase;
}

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
  const drillJson = bridge.get_drill_state();
  let drill: DrillSnapshot | null = null;
  try {
    drill = drillJson ? (JSON.parse(drillJson) as DrillSnapshot | null) : null;
  } catch {
    drill = null;
  }
  return {
    journey: JSON.parse(bridge.get_journey_state()) as JourneySnapshot,
    tower: JSON.parse(bridge.get_tower_state()) as TowerSnapshot,
    hero: JSON.parse(bridge.get_hero_state()) as HeroSnapshot,
    economy: JSON.parse(bridge.get_economy_state()) as EconomySnapshot,
    warnings: JSON.parse(bridge.get_validation_warnings()) as ValidationWarning[],
    gold: bridge.get_gold(),
    phase: bridge.get_phase() as GamePhase,
    drill,
  };
}

interface ValidationWarning {
  severity: "Info" | "Warning" | "Critical";
  message: string;
}

export function MapPage({ sendCommand, refreshPhase, initialPhase }: Props) {
  const [state, setState] = useState(() => ({ ...readState(), phase: initialPhase }));
  const [placeMode, setPlaceMode] = useState<PlaceModeState | null>(null);
  const [pendingChoice, setPendingChoice] = useState<PlacementChoice | null>(null);

  const refresh = useCallback(() => {
    setState(readState());
  }, []);

  // When a drill is running, drive a real-time tick loop so production
  // and transport advance against the synthetic demand. The loop
  // pumps tick(dt) into the bridge and re-renders this page each frame
  // so the player can watch deliveries happen.
  const drillActive = state.drill !== null;
  const rafRef = useRef<number>(0);
  const lastTimeRef = useRef<number>(0);
  useEffect(() => {
    if (!drillActive) return;
    lastTimeRef.current = performance.now();
    const loop = () => {
      const bridge = getBridge();
      const now = performance.now();
      const dt = (now - lastTimeRef.current) / 1000;
      lastTimeRef.current = now;
      bridge.tick(dt);
      setState(readState());
      // Stop the loop the moment the drill ends.
      const drillJson = bridge.get_drill_state();
      const stillRunning = drillJson && drillJson !== "null";
      if (stillRunning) {
        rafRef.current = requestAnimationFrame(loop);
      }
    };
    rafRef.current = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(rafRef.current);
  }, [drillActive]);

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

          <div className="prep-budget">
            {t("map.tower.floors", { count: tower.floors.length })} ·{" "}
            {state.economy.ticks_remaining} ticks · {gold}g
          </div>

          {state.drill && (
            <div className="drill-banner">
              <div className="drill-banner-row">
                <span className="drill-banner-title">🛠 Drill running</span>
                <span className="drill-banner-time">
                  {state.drill.seconds_remaining.toFixed(1)}s /{" "}
                  {state.drill.seconds_total.toFixed(0)}s
                </span>
                <span className="drill-banner-deliveries">
                  📦 {state.drill.deliveries_during_drill} deliveries (
                  {(
                    (state.drill.deliveries_during_drill /
                      Math.max(0.1, state.drill.seconds_total - state.drill.seconds_remaining)) *
                    60
                  ).toFixed(1)}
                  /min)
                </span>
              </div>
              <div className="drill-banner-bar">
                <div
                  className="drill-banner-bar-fill"
                  style={{
                    width: `${
                      ((state.drill.seconds_total - state.drill.seconds_remaining) /
                        state.drill.seconds_total) *
                      100
                    }%`,
                  }}
                />
              </div>
            </div>
          )}

          <div className="prep-tower-row">
            <TowerViz
              tower={tower}
              heroBalconyId={hero.position}
              title="Tower"
              placeMode={placeMode}
              pendingChoice={pendingChoice}
              onSlotClick={(c) => setPendingChoice(c)}
              onConfirm={() => {
                if (placeMode && pendingChoice) {
                  if (placeMode.kind === "building" && placeMode.buildingKey) {
                    sendCommand({
                      PlaceBuilding: {
                        floor: pendingChoice.floor,
                        slot: pendingChoice.slot,
                        building_type: placeMode.buildingKey,
                      },
                    });
                  } else if (placeMode.kind === "cache") {
                    sendCommand({
                      PlaceCache: { floor: pendingChoice.floor, slot: pendingChoice.slot },
                    });
                  } else if (placeMode.kind === "ladder") {
                    sendCommand({
                      BuildLadder: {
                        low_floor: pendingChoice.floor,
                        slot: pendingChoice.slot,
                      },
                    });
                  } else if (placeMode.kind === "dumbwaiter") {
                    sendCommand({
                      BuildDumbwaiter: {
                        low_floor: pendingChoice.floor,
                        slot: pendingChoice.slot,
                      },
                    });
                  }
                }
                setPlaceMode(null);
                setPendingChoice(null);
                refresh();
              }}
              onCancel={() => {
                setPlaceMode(null);
                setPendingChoice(null);
              }}
            />
            <BuildMenu
              economy={state.economy}
              onBuildFloor={(material) => {
                sendCommand({ BuildFloor: { material } });
                refresh();
              }}
              onHireRunner={() => {
                sendCommand("HireRunner");
                refresh();
              }}
              drillActive={drillActive}
              onRunDrill={() => {
                sendCommand({ RunDrill: { seconds: 30 } });
                refresh();
              }}
              onStartPlaceBuilding={(buildingType, widthSlots) => {
                setPendingChoice(null);
                setPlaceMode({
                  kind: "building",
                  buildingKey: buildingType,
                  width: widthSlots,
                  label: buildingType,
                });
              }}
              onStartPlaceCache={() => {
                setPendingChoice(null);
                setPlaceMode({ kind: "cache", width: 1, label: "Cache" });
              }}
              onStartPlaceLadder={() => {
                setPendingChoice(null);
                setPlaceMode({ kind: "ladder", width: 1, label: "Ladder" });
              }}
              onStartPlaceDumbwaiter={() => {
                setPendingChoice(null);
                setPlaceMode({ kind: "dumbwaiter", width: 1, label: "Dumbwaiter" });
              }}
            />
          </div>

          <LogisticsPanel tower={tower} />

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

interface LogisticsPanelProps {
  tower: TowerSnapshot;
}

function LogisticsPanel({ tower }: LogisticsPanelProps) {
  return (
    <div className="logistics-panel">
      <div className="logistics-header">
        <h4>Runners</h4>
      </div>
      <ul className="runner-list">
        {tower.runners.map((runner) => (
          <li key={runner.id} className="runner-row">
            <span className="runner-id">R{runner.id}</span>
            <span className="runner-floor">F{runner.current_floor}</span>
            <span className="runner-state">{runnerStateLabel(runner)}</span>
            {runner.carried && <span className="runner-cargo">📦{runner.carried.resource}</span>}
          </li>
        ))}
      </ul>
    </div>
  );
}

function runnerStateLabel(runner: RunnerSnapshot): string {
  const s = runner.state;
  if ("Idle" in s) return "idle";
  if ("Moving" in s) {
    const pct = Math.round(s.Moving.progress * 100);
    return `→F${s.Moving.to} (${pct}%)`;
  }
  if ("Loading" in s) return `loading…`;
  if ("Unloading" in s) return `unloading…`;
  if ("Queued" in s) return `queued`;
  return "?";
}
