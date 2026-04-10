import { useState } from "react";
import { getBridge } from "../../bridge";
import type { HudSnapshot } from "../../bridge/types";

interface Props {
  sendCommand: (cmd: Record<string, unknown> | string) => { Ok?: null; Error?: unknown };
}

function readHud(): HudSnapshot {
  return JSON.parse(getBridge().get_hud_state());
}

export function PostCombatPage({ sendCommand }: Props) {
  const [hud] = useState(readHud);

  return (
    <div className="post-combat-page">
      <h2>Encounter Complete</h2>
      <div className="summary">
        <p>Gold: {hud.gold}</p>
        <p>Tower HP: {Math.round(hud.tower_hp_fraction * 100)}%</p>
      </div>
      <button className="continue-button" onClick={() => sendCommand("ContinueJourney")}>
        Continue
      </button>
    </div>
  );
}
