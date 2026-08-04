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
  /**
   * Step without the frame loop, and hand back what it sounded like.
   *
   * The events were always there — `debug_step` has serialised them
   * since M0 and this wrapper threw them away, which is why the offline
   * audio harness could not exist. Returning them costs nothing and is
   * the whole of what it needed.
   */
  debugStep(ticks: number): SoundEvent[];
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
    debugStep: (ticks) => JSON.parse(wasm.debug_step(ticks)) as SoundEvent[],
  };

  wasmView = () => wasm.view();
  wasmFrame = (elapsedUs) => wasm.frame(elapsedUs);
  installTestHooks(bridge);
  return bridge;
}

/**
 * Hooks the Playwright smoke test drives the engine through, so tests
 * can assert on simulation state without racing the render loop.
 */
let wasmView: () => string = () => "";
let wasmFrame: (elapsedUs: number) => string = () => "";

function installTestHooks(active: Bridge): void {
  if (typeof window === "undefined") return;
  const target = window as unknown as Record<string, unknown>;
  target.__understory = {
    view: () => active.view(),
    catalog: () => active.catalog(),
    // Anything that drives the engine without a player has to be able
    // to answer a fork, or it walks into one and measures a parked
    // tower with total confidence (`SYSTEMS.md` §3.3). The harnesses
    // need a command channel for exactly that.
    send: (cmd: GameCommand) => active.send(cmd),
    stateHash: () => active.stateHash(),
    step: (ticks: number) => active.debugStep(ticks),
    verifyGolden: () => active.verifyGoldenReplay(),
    exportReplay: () => active.exportReplay(),
  };
  // **Unparsed, for the profiler only.** Every other caller wants a
  // `ViewSnapshot`; `e2e/profile.spec.ts` wants to know how much of a
  // frame is the WASM call and how much is `JSON.parse`, and it cannot
  // tell them apart through the parsed accessor above. Kept beside the
  // other test hooks rather than exported, because nothing in the game
  // should ever want the raw string.
  target.__understoryRaw = {
    view: () => wasmView(),
    frame: (elapsedUs: number) => wasmFrame(elapsedUs),
  };
}

/** True when a command came back rejected, with the reason attached. */
export function commandFailed(result: CommandResult): result is { Error: unknown } {
  return result !== "Ok";
}
