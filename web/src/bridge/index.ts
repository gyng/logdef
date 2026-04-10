/**
 * Bridge to the Rust/WASM game engine.
 * Uses free functions (thread_local engine) instead of a class
 * to avoid wasm-bindgen Rc issues with Rust 2024 edition.
 */

import type { BridgePerfSnapshot, PerfMetricSnapshot, SimPerfSnapshot } from "./types";

export interface GameBridge {
  send_command(json: string): string;
  tick(real_dt: number): string;
  interpolation_alpha(): number;
  get_phase(): string;
  get_hud_state(): string;
  get_tower_state(): string;
  get_journey_state(): string;
  get_economy_state(): string;
  get_gold(): number;
  get_hero_state(): string;
  get_merchant_state(): string;
  get_encounter_state(): string;
  get_validation_warnings(): string;
  get_perf_state(): string;
  save(): string;
  load(data: string): void;
}

let bridge: GameBridge | null = null;

type TimedBridgeMethod = keyof BridgePerfSnapshot;

function createPerfMetric(): PerfMetricSnapshot {
  return {
    calls: 0,
    last_ms: 0,
    avg_ms: 0,
    max_ms: 0,
  };
}

function createBridgePerfSnapshot(): BridgePerfSnapshot {
  return {
    send_command: createPerfMetric(),
    tick: createPerfMetric(),
    interpolation_alpha: createPerfMetric(),
    get_phase: createPerfMetric(),
    get_hud_state: createPerfMetric(),
    get_tower_state: createPerfMetric(),
    get_journey_state: createPerfMetric(),
    get_economy_state: createPerfMetric(),
    get_gold: createPerfMetric(),
    get_hero_state: createPerfMetric(),
    get_merchant_state: createPerfMetric(),
    get_encounter_state: createPerfMetric(),
    get_validation_warnings: createPerfMetric(),
    get_perf_state: createPerfMetric(),
    save: createPerfMetric(),
    load: createPerfMetric(),
  };
}

const bridgePerf = createBridgePerfSnapshot();

function recordPerf(metric: PerfMetricSnapshot, elapsedMs: number): void {
  metric.calls += 1;
  metric.last_ms = elapsedMs;
  if (metric.calls === 1) {
    metric.avg_ms = elapsedMs;
    metric.max_ms = elapsedMs;
    return;
  }
  const priorCalls = metric.calls - 1;
  metric.avg_ms = (metric.avg_ms * priorCalls + elapsedMs) / metric.calls;
  metric.max_ms = Math.max(metric.max_ms, elapsedMs);
}

function timeCall<T>(name: TimedBridgeMethod, fn: () => T): T {
  const start = performance.now();
  try {
    return fn();
  } finally {
    recordPerf(bridgePerf[name], performance.now() - start);
  }
}

export function getBridgePerfSnapshot(): BridgePerfSnapshot {
  return structuredClone(bridgePerf);
}

function installDebugHelpers(activeBridge: GameBridge): void {
  if (typeof window === "undefined") {
    return;
  }

  type DebugWindow = Window & {
    debug?: {
      metrics?: () => { bridge: BridgePerfSnapshot; core: SimPerfSnapshot };
    };
    __getPhase?: () => string;
    __getEncounter?: () => string;
    __getHud?: () => string;
    __tick?: (dt: number) => string;
  };

  const debugWindow = window as DebugWindow;
  debugWindow.debug ??= {};
  debugWindow.debug.metrics = () => ({
    bridge: getBridgePerfSnapshot(),
    core: JSON.parse(activeBridge.get_perf_state()) as SimPerfSnapshot,
  });
  // Test hooks: let Playwright drive the engine without React in the loop.
  debugWindow.__getPhase = () => activeBridge.get_phase();
  debugWindow.__getEncounter = () => activeBridge.get_encounter_state();
  debugWindow.__getHud = () => activeBridge.get_hud_state();
  debugWindow.__tick = (dt: number) => activeBridge.tick(dt);
}

interface WasmBridgeModule {
  default(): Promise<void>;
  init_game(seed: bigint, heroClass: string): void;
  send_command(json: string): string;
  tick(real_dt: number): string;
  interpolation_alpha(): number;
  get_phase(): string;
  get_hud_state(): string;
  get_tower_state(): string;
  get_journey_state(): string;
  get_economy_state(): string;
  get_gold(): number;
  get_hero_state(): string;
  get_merchant_state(): string;
  get_encounter_state(): string;
  get_validation_warnings(): string;
  get_perf_state(): string;
  save(): string;
  load(data: string): void;
}

export function getBridge(): GameBridge {
  if (!bridge) {
    throw new Error("Bridge not initialized — call initBridge() first");
  }
  return bridge;
}

export async function initBridge(seed: number, heroClass: string): Promise<GameBridge> {
  const wasm = (await import("../../pkg/supply_line_bridge")) as unknown as WasmBridgeModule;
  await wasm.default();
  wasm.init_game(BigInt(seed), heroClass);

  // Wrap free functions as a bridge object
  bridge = {
    send_command: (json: string) => timeCall("send_command", () => wasm.send_command(json)),
    tick: (real_dt: number) => timeCall("tick", () => wasm.tick(real_dt)),
    interpolation_alpha: () => timeCall("interpolation_alpha", () => wasm.interpolation_alpha()),
    get_phase: () => timeCall("get_phase", () => wasm.get_phase()),
    get_hud_state: () => timeCall("get_hud_state", () => wasm.get_hud_state()),
    get_tower_state: () => timeCall("get_tower_state", () => wasm.get_tower_state()),
    get_journey_state: () => timeCall("get_journey_state", () => wasm.get_journey_state()),
    get_economy_state: () => timeCall("get_economy_state", () => wasm.get_economy_state()),
    get_gold: () => timeCall("get_gold", () => wasm.get_gold()),
    get_hero_state: () => timeCall("get_hero_state", () => wasm.get_hero_state()),
    get_merchant_state: () => timeCall("get_merchant_state", () => wasm.get_merchant_state()),
    get_encounter_state: () => timeCall("get_encounter_state", () => wasm.get_encounter_state()),
    get_validation_warnings: () =>
      timeCall("get_validation_warnings", () => wasm.get_validation_warnings()),
    get_perf_state: () => timeCall("get_perf_state", () => wasm.get_perf_state()),
    save: () => timeCall("save", () => wasm.save()),
    load: (data: string) => timeCall("load", () => wasm.load(data)),
  };

  installDebugHelpers(bridge);

  return bridge;
}
