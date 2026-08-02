/**
 * The WASM bridge.
 *
 * Free functions over a thread-local engine rather than an exported
 * class: wasm-bindgen's handling of `Rc` under Rust 2024 makes the
 * class shape awkward, and there is only ever one engine anyway.
 *
 * Everything crosses as a JSON string. That is fine at this scale and
 * it keeps the boundary inspectable in DevTools; if it ever stops being
 * fine, `DECISIONS.md` §3 describes the replacement.
 */

import type {
  CatalogSnapshot,
  CommandResult,
  GameCommand,
  ReplayReport,
  SoundEvent,
  ViewSnapshot,
} from "./types";

interface WasmModule {
  default(): Promise<unknown>;
  init_game(seed: bigint): void;
  send_command(json: string): string;
  frame(elapsedUs: number): string;
  view(): string;
  catalog(): string;
  state_hash(): string;
  save(): string;
  load(json: string): void;
  export_replay(): string;
  verify_replay(json: string): string;
  verify_golden_replay(): string;
  debug_step(ticks: number): string;
}

export interface Bridge {
  send(cmd: GameCommand): CommandResult;
  /** Advance by wall-clock microseconds. Returns sounds produced. */
  frame(elapsedUs: number): SoundEvent[];
  view(): ViewSnapshot;
  catalog(): CatalogSnapshot;
  stateHash(): string;
  save(): string;
  load(json: string): void;
  exportReplay(): string;
  verifyReplay(json: string): ReplayReport;
  verifyGoldenReplay(): ReplayReport;
  /** Run N ticks regardless of speed. Test hook. */
  debugStep(ticks: number): void;
}

let bridge: Bridge | null = null;

export function getBridge(): Bridge {
  if (!bridge) {
    throw new Error("bridge not initialised — await initBridge() first");
  }
  return bridge;
}

export async function initBridge(seed: number): Promise<Bridge> {
  const wasm = (await import("../../pkg/understory_bridge")) as unknown as WasmModule;
  await wasm.default();
  wasm.init_game(BigInt(seed));

  bridge = {
    send: (cmd) => JSON.parse(wasm.send_command(JSON.stringify(cmd))) as CommandResult,
    frame: (elapsedUs) => JSON.parse(wasm.frame(elapsedUs)) as SoundEvent[],
    view: () => JSON.parse(wasm.view()) as ViewSnapshot,
    catalog: () => JSON.parse(wasm.catalog()) as CatalogSnapshot,
    stateHash: () => wasm.state_hash(),
    save: () => wasm.save(),
    load: (json) => {
      wasm.load(json);
    },
    exportReplay: () => wasm.export_replay(),
    verifyReplay: (json) => JSON.parse(wasm.verify_replay(json)) as ReplayReport,
    verifyGoldenReplay: () => JSON.parse(wasm.verify_golden_replay()) as ReplayReport,
    debugStep: (ticks) => {
      wasm.debug_step(ticks);
    },
  };

  installTestHooks(bridge);
  return bridge;
}

/**
 * Hooks the Playwright smoke test drives the engine through, so tests
 * can assert on simulation state without racing the render loop.
 */
function installTestHooks(active: Bridge): void {
  if (typeof window === "undefined") return;
  const target = window as unknown as Record<string, unknown>;
  target.__understory = {
    view: () => active.view(),
    catalog: () => active.catalog(),
    stateHash: () => active.stateHash(),
    step: (ticks: number) => {
      active.debugStep(ticks);
    },
    verifyGolden: () => active.verifyGoldenReplay(),
    exportReplay: () => active.exportReplay(),
  };
}

/** True when a command came back rejected, with the reason attached. */
export function commandFailed(result: CommandResult): result is { Error: unknown } {
  return result !== "Ok";
}
