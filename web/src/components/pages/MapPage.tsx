import { useCallback, useState } from "react";
import { getBridge } from "../../bridge";
import type { GamePhase, JourneySnapshot, TowerSnapshot } from "../../bridge/types";

interface Props {
  sendCommand: (cmd: Record<string, unknown> | string) => { Ok?: null; Error?: unknown };
  refreshPhase: () => void;
  initialPhase: GamePhase;
}

function readState() {
  const bridge = getBridge();
  return {
    journey: JSON.parse(bridge.get_journey_state()) as JourneySnapshot,
    tower: JSON.parse(bridge.get_tower_state()) as TowerSnapshot,
    hud: JSON.parse(bridge.get_hud_state()) as { gold: number },
    phase: bridge.get_phase() as GamePhase,
  };
}

export function MapPage({ sendCommand, refreshPhase, initialPhase }: Props) {
  const [state, setState] = useState(() => ({ ...readState(), phase: initialPhase }));

  const refresh = useCallback(() => {
    setState(readState());
  }, []);

  const { journey, tower, hud, phase } = state;
  const chapter = journey.chapters[journey.current_chapter - 1];
  if (!chapter) return <div>No chapter data</div>;

  const reachableNodes = chapter.edges
    .filter((e) => e.from === journey.current_node)
    .map((e) => e.to);

  const currentNode = chapter.nodes.find((n) => n.id === journey.current_node);

  return (
    <div className="map-page">
      <div className="map-header">
        <h2>Chapter {journey.current_chapter}</h2>
        <div className="resources">
          <span className="gold">Gold: {hud.gold}</span>
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
              <div className="node-type">{node.node_type}</div>
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
          <h3>Prep (Node: {currentNode?.node_type})</h3>

          <div className="tower-info">
            <p>Floors: {tower.floors.length}</p>
            {tower.floors.map((floor) => (
              <div key={floor.index} className="floor-row">
                Floor {floor.index}: {floor.material}
                {floor.building ? ` [${floor.building.building_type}]` : " [empty]"}
                {" | HP: "}
                {Math.round(floor.panel_hp_fraction * 100)}%
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
              Build Wood Floor (2t, 3 wood)
            </button>
            <button
              onClick={() => {
                sendCommand({ BuildFloor: { material: "Stone" } });
                refresh();
              }}
            >
              Build Stone Floor (2t, 4 stone)
            </button>
            {tower.floors.map(
              (floor) =>
                !floor.building && (
                  <button
                    key={`build-${floor.index}`}
                    onClick={() => {
                      sendCommand({
                        PlaceBuilding: { floor: floor.index, building_type: "Fletcher" },
                      });
                      refresh();
                    }}
                  >
                    Place Fletcher on F{floor.index} (2t, 2 wood)
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
            March!
          </button>
        </div>
      )}
    </div>
  );
}
