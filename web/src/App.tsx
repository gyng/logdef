import { useCallback, useEffect, useState } from "react";
import { initBridge, getBridge } from "./bridge";
import type { GamePhase } from "./bridge/types";
import { MapPage } from "./components/pages/MapPage";
import { CombatPage } from "./components/pages/CombatPage";
import { PostCombatPage } from "./components/pages/PostCombatPage";
import { MerchantPage } from "./components/pages/MerchantPage";
import { t } from "./i18n";

type AppState = "loading" | "error" | "ready";

export function App() {
  const [appState, setAppState] = useState<AppState>("loading");
  const [error, setError] = useState("");
  const [phase, setPhase] = useState<GamePhase>("MainMenu");

  useEffect(() => {
    initBridge(Date.now(), "archer")
      .then(() => {
        setPhase(getBridge().get_phase() as GamePhase);
        setAppState("ready");
      })
      .catch((e: unknown) => {
        setError(String(e));
        setAppState("error");
      });
  }, []);

  const refreshPhase = useCallback(() => {
    if (appState === "ready") {
      setPhase(getBridge().get_phase() as GamePhase);
    }
  }, [appState]);

  const sendCommand = useCallback(
    (cmd: Record<string, unknown> | string) => {
      const bridge = getBridge();
      const json = typeof cmd === "string" ? `"${cmd}"` : JSON.stringify(cmd);
      const result = bridge.send_command(json);
      refreshPhase();
      return JSON.parse(result) as { Ok?: null; Error?: unknown };
    },
    [refreshPhase],
  );

  if (appState === "loading") {
    return (
      <div className="loading">
        <p>{t("app.loading")}</p>
      </div>
    );
  }
  if (appState === "error") {
    return (
      <div className="error">
        <h1>{t("app.error.title")}</h1>
        <pre>{error}</pre>
      </div>
    );
  }

  return (
    <div id="supply-line">
      {(phase === "MapView" || phase === "Travel") && (
        <MapPage sendCommand={sendCommand} refreshPhase={refreshPhase} initialPhase={phase} />
      )}
      {phase === "Encounter" && (
        <CombatPage sendCommand={sendCommand} refreshPhase={refreshPhase} />
      )}
      {phase === "PostCombat" && <PostCombatPage sendCommand={sendCommand} />}
      {phase === "Merchant" && (
        <MerchantPage sendCommand={sendCommand} refreshPhase={refreshPhase} />
      )}
      {phase === "GameOver" && <GameOverPage />}
      {phase === "Victory" && <VictoryPage />}
    </div>
  );
}

function GameOverPage() {
  return (
    <div className="end-screen">
      <h1>{t("end.game_over.title")}</h1>
      <p>{t("end.game_over.body")}</p>
      <button onClick={() => window.location.reload()}>{t("end.new_game")}</button>
    </div>
  );
}

function VictoryPage() {
  return (
    <div className="end-screen">
      <h1>{t("end.victory.title")}</h1>
      <p>{t("end.victory.body")}</p>
      <button onClick={() => window.location.reload()}>{t("end.play_again")}</button>
    </div>
  );
}
