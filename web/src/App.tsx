import { useState } from "react";
import type { GamePhase } from "./bridge/types";

export function App() {
  const [phase, _setPhase] = useState<GamePhase>("MainMenu");

  return (
    <div id="supply-line">
      {/* Game canvas managed by Rust/wgpu renders behind this */}
      <canvas id="game-canvas" />

      {/* React UI overlay */}
      <div id="ui-overlay">{phase === "MainMenu" && <MainMenu />}</div>
    </div>
  );
}

function MainMenu() {
  return (
    <div className="main-menu">
      <h1>Supply Line</h1>
      <p>A walking-fortress roguelike</p>
    </div>
  );
}
