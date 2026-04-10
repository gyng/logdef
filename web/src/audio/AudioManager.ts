import type { SoundEvent } from "../bridge/types";

/**
 * Consumes SoundEvents from the Rust simulation and plays audio
 * via the Web Audio API. Uses synthesized tones rather than audio
 * assets — every event maps to a short oscillator burst.
 *
 * This is MVP audio: it gives the game audible feedback without
 * shipping any sound files. Real sounds replace these later.
 */
export class AudioManager {
  private ctx: AudioContext | null = null;
  private masterGain: GainNode | null = null;
  private lastPlayTime: Map<string, number> = new Map();

  /** Safe to call eagerly; any failure leaves the manager as a no-op. */
  init() {
    if (this.ctx) return;
    try {
      this.ctx = new AudioContext();
      this.masterGain = this.ctx.createGain();
      this.masterGain.gain.value = 0.15;
      this.masterGain.connect(this.ctx.destination);
    } catch {
      // Audio unavailable (e.g., headless browser without user gesture).
      this.ctx = null;
      this.masterGain = null;
    }
  }

  play(events: SoundEvent[]) {
    if (!this.ctx || !this.masterGain) return;
    const now = this.ctx.currentTime;

    for (const ev of events) {
      const kind = eventKind(ev);
      if (!kind) continue;
      // Simple rate-limiter per kind — prevents audio overload when
      // dozens of projectiles hit in one tick.
      const last = this.lastPlayTime.get(kind) ?? 0;
      if (now - last < 0.05) continue;
      this.lastPlayTime.set(kind, now);

      const params = soundParams(kind);
      this.tone(params.freq, params.duration, params.type, params.gain);
    }
  }

  private tone(freq: number, duration: number, type: OscillatorType, gain: number) {
    if (!this.ctx || !this.masterGain) return;
    const osc = this.ctx.createOscillator();
    const env = this.ctx.createGain();
    osc.type = type;
    osc.frequency.value = freq;
    const now = this.ctx.currentTime;
    env.gain.setValueAtTime(0, now);
    env.gain.linearRampToValueAtTime(gain, now + 0.005);
    env.gain.exponentialRampToValueAtTime(0.0001, now + duration);
    osc.connect(env);
    env.connect(this.masterGain);
    osc.start(now);
    osc.stop(now + duration);
  }

  suspend() {
    this.ctx?.suspend();
  }

  resume() {
    this.ctx?.resume();
  }
}

/**
 * Convert a SoundEvent (serde tagged enum) to a short string key.
 * Rust's default serde serialization for enum variants produces
 * `{"VariantName": {...}}` or `"VariantName"` for unit variants.
 */
function eventKind(ev: SoundEvent): string | null {
  if (typeof ev === "string") return ev;
  if (typeof ev === "object" && ev !== null) {
    const keys = Object.keys(ev);
    return keys[0] ?? null;
  }
  return null;
}

interface SoundParams {
  freq: number;
  duration: number;
  type: OscillatorType;
  gain: number;
}

function soundParams(kind: string): SoundParams {
  switch (kind) {
    case "WeaponFire":
      return { freq: 440, duration: 0.08, type: "square", gain: 0.3 };
    case "ProjectileHit":
      return { freq: 300, duration: 0.1, type: "triangle", gain: 0.4 };
    case "ProjectileMiss":
      return { freq: 180, duration: 0.08, type: "sine", gain: 0.2 };
    case "EnemyDeath":
      return { freq: 130, duration: 0.25, type: "sawtooth", gain: 0.5 };
    case "EnemySpawn":
      return { freq: 200, duration: 0.15, type: "square", gain: 0.2 };
    case "PanelHit":
      return { freq: 90, duration: 0.12, type: "sawtooth", gain: 0.3 };
    case "PanelBreach":
      return { freq: 70, duration: 0.4, type: "sawtooth", gain: 0.5 };
    case "BuildingProduce":
      return { freq: 600, duration: 0.04, type: "triangle", gain: 0.2 };
    case "RunnerPickup":
    case "RunnerDeliver":
      return { freq: 550, duration: 0.05, type: "sine", gain: 0.15 };
    case "WaveStart":
      return { freq: 350, duration: 0.3, type: "square", gain: 0.4 };
    case "WaveComplete":
      return { freq: 500, duration: 0.2, type: "sine", gain: 0.3 };
    case "EncounterVictory":
      return { freq: 700, duration: 0.5, type: "triangle", gain: 0.5 };
    case "EncounterDefeat":
      return { freq: 100, duration: 0.7, type: "sawtooth", gain: 0.5 };
    default:
      return { freq: 440, duration: 0.05, type: "sine", gain: 0.2 };
  }
}
