import { useEffect, useState } from "react";
import { initBridge, getBridge } from "./bridge";

type AppState = "loading" | "error" | "ready";

export function App() {
  const [state, setState] = useState<AppState>("loading");
  const [error, setError] = useState<string>("");

  useEffect(() => {
    initBridge(Date.now(), "archer")
      .then(() => setState("ready"))
      .catch((e: unknown) => {
        setError(String(e));
        setState("error");
      });
  }, []);

  if (state === "loading") return <Loading />;
  if (state === "error") return <Error message={error} />;
  return <Game />;
}

function Loading() {
  return (
    <div className="loading">
      <p>Loading...</p>
    </div>
  );
}

function Error({ message }: { message: string }) {
  return (
    <div className="error">
      <h1>Failed to load</h1>
      <pre>{message}</pre>
    </div>
  );
}

function Game() {
  const bridge = getBridge();
  const phase = JSON.parse(bridge.get_hero_state()).class as string;
  const tower = JSON.parse(bridge.get_tower_state());

  return (
    <div id="supply-line">
      <canvas id="game-canvas" />
      <div id="ui-overlay">
        <MainMenu heroClass={phase} floorCount={tower.floors.length} />
      </div>
    </div>
  );
}

function MainMenu({ heroClass, floorCount }: { heroClass: string; floorCount: number }) {
  return (
    <div className="main-menu">
      <h1>Supply Line</h1>
      <p>A walking-fortress roguelike</p>
      <p className="bridge-status">
        Bridge connected — class: {heroClass}, floors: {floorCount}
      </p>
    </div>
  );
}
