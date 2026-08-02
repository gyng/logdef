/**
 * Boot: load the wasm core, then hand off to the game stage.
 *
 * There is exactly one screen. v1 routed between a map page, a prep
 * page, a combat page and a post-combat page, and that phase split is
 * precisely what made rerouting logistics under pressure impossible.
 * The tower, the world it walks through, and everything you can do to
 * it live on one surface, always.
 */

import { useEffect, useState } from "react";

import { initBridge, type Bridge } from "./bridge";
import { GameStage } from "./ui/GameStage";

type Boot =
  | { status: "loading" }
  | { status: "ready"; bridge: Bridge }
  | { status: "failed"; error: string };

export function App() {
  const [boot, setBoot] = useState<Boot>({ status: "loading" });

  useEffect(() => {
    let cancelled = false;
    initBridge(chooseSeed())
      .then((bridge) => {
        if (!cancelled) setBoot({ status: "ready", bridge });
      })
      .catch((error: unknown) => {
        if (!cancelled) {
          setBoot({
            status: "failed",
            error: error instanceof Error ? (error.stack ?? error.message) : String(error),
          });
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  if (boot.status === "loading") {
    return <div className="boot">the tower is waking</div>;
  }
  if (boot.status === "failed") {
    return (
      <div className="boot-fail">
        <h1>Understory failed to start</h1>
        <pre>{boot.error}</pre>
      </div>
    );
  }
  return <GameStage bridge={boot.bridge} />;
}

/**
 * `?seed=` if given, otherwise the clock.
 *
 * Runs are seeded and shareable, so an explicit seed has to be able to
 * come from outside — that is how a reproduction case gets handed over,
 * and how the smoke test gets a run it can make assertions about.
 */
function chooseSeed(): number {
  const provided = new URLSearchParams(window.location.search).get("seed");
  if (provided !== null) {
    const parsed = Number.parseInt(provided, 10);
    if (Number.isFinite(parsed)) return parsed;
  }
  return Date.now() % Number.MAX_SAFE_INTEGER;
}
