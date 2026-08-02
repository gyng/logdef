/**
 * Hosts the canvas and the label overlay, and owns the `Game`
 * lifetime.
 *
 * React's only job here is to put a canvas in the DOM and hand it over
 * once. Everything after that — the frame loop, the snapshot reads, the
 * draw calls — happens outside the reconciler. React finds out what is
 * going on through the UI digest the game publishes ten times a second.
 */

import { useEffect, useRef, useState } from "react";

import { Game, type UiState } from "../engine/Game";
import type { Bridge } from "../bridge";
import { Chrome } from "./Chrome";

interface Props {
  bridge: Bridge;
}

export function GameStage({ bridge }: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const labelsRef = useRef<HTMLDivElement>(null);
  const [game, setGame] = useState<Game | null>(null);
  const [ui, setUi] = useState<UiState | null>(null);
  const [failure, setFailure] = useState<string | null>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    const labels = labelsRef.current;
    if (!canvas || !labels) return;

    let instance: Game;
    try {
      instance = new Game(canvas, labels, bridge);
    } catch (error) {
      setFailure(error instanceof Error ? error.message : String(error));
      return;
    }

    const unsubscribe = instance.subscribe(setUi);
    instance.start();
    setGame(instance);

    return () => {
      unsubscribe();
      instance.dispose();
      setGame(null);
    };
  }, [bridge]);

  if (failure !== null) {
    return (
      <div className="boot-fail">
        <h1>The tower could not be drawn</h1>
        <pre>{failure}</pre>
        <p>Understory needs WebGL2. Check that hardware acceleration is enabled.</p>
      </div>
    );
  }

  return (
    <div className="stage">
      <canvas
        ref={canvasRef}
        className={ui?.placing ? "stage-canvas placing" : "stage-canvas"}
        onPointerMove={(event) => game?.handlePointerMove(event.clientX, event.clientY)}
        onPointerLeave={() => game?.handlePointerLeave()}
        onClick={(event) => game?.handleClick(event.clientX, event.clientY)}
        data-testid="game-canvas"
      />
      <div ref={labelsRef} className="stage-labels" />
      {game && ui && <Chrome game={game} ui={ui} />}
    </div>
  );
}
