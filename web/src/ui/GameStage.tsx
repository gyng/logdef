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
import { registerGameTools } from "../mcp/provider";

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

    // **The page declares what it can do** (`SYSTEMS.md` §6.31). Torn
    // down with the engine, because a tool closing over a disposed
    // `Game` is worse than no tool.
    const unregister = registerGameTools(instance);
    const unsubscribe = instance.subscribe(setUi);
    instance.start();
    setGame(instance);

    return () => {
      unregister();
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
        // Drag to lasso a group of crew; a short drag is a click and
        // `handleClick` keeps it.
        onPointerDown={(event) =>
          game?.handlePointerDown(event.clientX, event.clientY, event.button)
        }
        onPointerUp={(event) => game?.handlePointerUp(event.clientX, event.clientY)}
        onClick={(event) => game?.handleClick(event.clientX, event.clientY)}
        // **Right-click puts the placement cursor down.** Picking a room
        // and changing your mind used to mean finding the same card
        // again and clicking it off — a lot of travel to undo a decision
        // you have not made yet. The browser menu is suppressed only
        // over the canvas, so text elsewhere still behaves.
        // **Right-click means three things, in this order.** Push the
        // people you have picked at the room under the pointer; failing
        // that, put the placement cursor down; failing that, let go of
        // the selection. Each is the "undo the thing I am in the middle
        // of" gesture for whichever thing that is, which is why they
        // can share a button without ambiguity — only one of them is
        // ever in progress.
        onContextMenu={(event) => {
          event.preventDefault();
          if (game?.pushSelectedTo(event.clientX, event.clientY)) return;
          if (ui?.placing) {
            game?.cancelPlacement();
            return;
          }
          game?.releaseSelected();
        }}
        // A wheel notch is ~100 deltaY, so this is about 10% a notch and
        // multiplicative — see `Game.zoomBy` for why it is not additive.
        onWheel={(event) => game?.zoomBy(Math.exp(-event.deltaY * 0.001))}
        data-testid="game-canvas"
      />
      <div ref={labelsRef} className="stage-labels" />
      {ui?.marquee && (
        <div
          className="marquee"
          data-testid="marquee"
          style={{
            left: Math.min(ui.marquee.x0, ui.marquee.x1),
            top: Math.min(ui.marquee.y0, ui.marquee.y1),
            width: Math.abs(ui.marquee.x1 - ui.marquee.x0),
            height: Math.abs(ui.marquee.y1 - ui.marquee.y0),
          }}
        />
      )}
      {game && ui && <Chrome game={game} ui={ui} />}
    </div>
  );
}
