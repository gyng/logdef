/**
 * Bridge to the Rust/WASM game engine.
 * Uses free functions (thread_local engine) instead of a class
 * to avoid wasm-bindgen Rc issues with Rust 2024 edition.
 */

export interface GameBridge {
  send_command(json: string): string;
  tick(real_dt: number): string;
  interpolation_alpha(): number;
  get_phase(): string;
  get_hud_state(): string;
  get_tower_state(): string;
  get_journey_state(): string;
  get_hero_state(): string;
  get_encounter_state(): string;
  get_validation_warnings(): string;
  save(): string;
  load(data: string): void;
}

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
  wasm.init_game(BigInt(seed), heroClass);

  // Wrap free functions as a bridge object
  bridge = {
    send_command: (json: string) => wasm.send_command(json),
    tick: (real_dt: number) => wasm.tick(real_dt),
    interpolation_alpha: () => wasm.interpolation_alpha(),
    get_phase: () => wasm.get_phase(),
    get_hud_state: () => wasm.get_hud_state(),
    get_tower_state: () => wasm.get_tower_state(),
    get_journey_state: () => wasm.get_journey_state(),
    get_hero_state: () => wasm.get_hero_state(),
    get_encounter_state: () => wasm.get_encounter_state(),
    get_validation_warnings: () => wasm.get_validation_warnings(),
    save: () => wasm.save(),
    load: (data: string) => wasm.load(data),
  };

  return bridge;
}
