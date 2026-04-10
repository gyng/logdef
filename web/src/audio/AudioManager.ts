import type { SoundEvent } from "../bridge/types";

/**
 * Consumes SoundEvents from the Rust simulation and plays audio
 * via the Web Audio API. Fire-and-forget — if too many sounds are
 * playing, lowest-priority ones are dropped.
 *
 * Priority: weapon fire > enemy death > building production > ambient.
 */
export class AudioManager {
  private ctx: AudioContext | null = null;

  init() {
    this.ctx = new AudioContext();
  }

  play(_events: SoundEvent[]) {
    if (!this.ctx) return;
    // TODO: map SoundEvent types to audio buffers, play with spatial panning
  }

  suspend() {
    this.ctx?.suspend();
  }

  resume() {
    this.ctx?.resume();
  }
}
