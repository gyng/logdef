/**
 * Bridge to the Rust/WASM game engine.
 *
 * All game state lives in Rust — this module only provides
 * the JS-side interface to send commands and read snapshots.
 */

import type { Bridge as WasmBridge } from "../../pkg/supply_line_bridge";

export type GameBridge = WasmBridge;

let bridge: GameBridge | null = null;

export function getBridge(): GameBridge {
  if (!bridge) {
    throw new Error("Bridge not initialized — call initBridge() first");
  }
  return bridge;
}

export async function initBridge(seed: number, heroClass: string): Promise<GameBridge> {
  const wasm = await import("../../pkg/supply_line_bridge");
  await wasm.default();
  bridge = new wasm.Bridge(BigInt(seed), heroClass);
  return bridge;
}
