import { useCallback, useEffect, useState } from "react";
import { initBridge, getBridge } from "./bridge";
import type { GamePhase } from "./bridge/types";
import { MapPage } from "./components/pages/MapPage";
import { CombatPage } from "./components/pages/CombatPage";
import { PostCombatPage } from "./components/pages/PostCombatPage";

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
        <p>Loading...</p>
      </div>
    );
  }
  if (appState === "error") {
    return (
      <div className="error">
        <h1>Failed to load</h1>
        <pre>{error}</pre>
      </div>
    );
  }

  return (
    <div id="supply-line">
      {(phase === "MapView" || phase === "Travel") && (
        <MapPage sendCommand={sendCommand} refreshPhase={refreshPhase} phase={phase} />
      )}
      {phase === "Encounter" && (
        <CombatPage sendCommand={sendCommand} refreshPhase={refreshPhase} />
      )}
      {phase === "PostCombat" && <PostCombatPage sendCommand={sendCommand} />}
      {phase === "GameOver" && <GameOverPage />}
      {phase === "Victory" && <VictoryPage />}
    </div>
  );
}

function GameOverPage() {
  return (
    <div className="end-screen">
      <h1>Game Over</h1>
      <p>Your tower has fallen.</p>
      <button onClick={() => window.location.reload()}>New Game</button>
    </div>
  );
}

function VictoryPage() {
  return (
    <div className="end-screen">
      <h1>Victory!</h1>
      <p>You reached The Harbor.</p>
      <button onClick={() => window.location.reload()}>Play Again</button>
    </div>
  );
}
