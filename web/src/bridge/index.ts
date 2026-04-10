/**
 * Bridge to the Rust/WASM game engine.
 *
 * In dev, this will be loaded via wasm-pack output.
 * All game state lives in Rust — this module only provides
 * the JS-side interface to send commands and read snapshots.
 */

export interface GameBridge {
  send_command(json: string): string;
  tick(real_dt: number): string;
  interpolation_alpha(): number;
  get_hud_state(): string;
  get_tower_state(): string;
  get_journey_state(): string;
  get_hero_state(): string;
  get_validation_warnings(): string;
  save(): string;
  load(data: string): void;
}

// eslint-disable-next-line prefer-const -- will be assigned in initBridge once wasm-pack is wired up
let bridge: GameBridge | null = null;

export function getBridge(): GameBridge {
  if (!bridge) {
    throw new Error("Bridge not initialized — call initBridge() first");
  }
  return bridge;
}

export async function initBridge(_seed: number, _heroClass: string): Promise<GameBridge> {
  // TODO: load wasm-pack output, construct Bridge instance
  // const wasm = await import("../../pkg/supply_line_bridge");
  // bridge = new wasm.Bridge(seed, heroClass);
  throw new Error("WASM bridge not yet built — run wasm-pack first");
}
