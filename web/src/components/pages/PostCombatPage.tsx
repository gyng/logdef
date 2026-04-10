import { useState } from "react";
import { getBridge } from "../../bridge";
import type { HudSnapshot } from "../../bridge/types";
import { t } from "../../i18n";

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
      <h2>{t("post.title")}</h2>
      <div className="summary">
        <p>{t("post.gold", { gold: hud.gold })}</p>
        <p>{t("post.tower_hp", { hp: Math.round(hud.tower_hp_fraction * 100) })}</p>
      </div>
      <button className="continue-button" onClick={() => sendCommand("ContinueJourney")}>
        {t("post.continue")}
      </button>
    </div>
  );
}
