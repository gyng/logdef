/**
 * The tower, heard.
 *
 * M1's brief asked for "starvation/stall/brown-out all readable *and
 * audible* — silence = broken" and did not get it; `SoundEvent` has
 * been crossing the bridge since M0 with nothing on the other side. This
 * is the other side.
 *
 * **The boundary is fixed: Rust decides *that* something happened, JS
 * decides whether and how it sounds.** Nothing about the mix, the
 * volume, the voice count, or whether audio is enabled at all may reach
 * `GameState`. If a sound needs to know something it reads the
 * snapshot; it never asks the simulation to remember anything for it.
 *
 * **The audio layer reads two inputs, and telling them apart is the
 * whole design.** The eyes-closed test asks whether you can hear how the
 * tower is *doing*, which is continuous state — and `SoundEvent` is
 * punctuation. A starved mill going quiet is not an event at all; it is
 * the *absence* of a loop, and the fact driving it is `RoomView.stalled`
 * in the per-frame view. So:
 *
 * - `frame()`'s event list drives one-shots: things that happened.
 * - `view()`'s `ViewSnapshot` drives loops: things that are ongoing.
 *
 * Getting that split wrong is precisely how a project ends up with a
 * warning beep where a silence belonged.
 *
 * **The diegetic rule, stated once because everything else follows from
 * it: a starved production loop goes silent, it does not gain a warning
 * sound.** No alarm on a stalled room, no beep on a queue, no sting on a
 * full buffer. `wait_ticks` and the red tint are the bottleneck
 * instrument (`DECISIONS.md` §8) and audio's contribution to them is the
 * mill you can no longer hear. This is the rule that makes the
 * eyes-closed test winnable at all: a tower whose problems announce
 * themselves with tones is one where you hear the *alarms*, not the
 * tower. What keeps the mix from ever being silent — so that absence
 * reads against a floor rather than against nothing — is the beds
 * underneath: the jungle, the day and night soundscapes, and the
 * electrical hum thinning as the bank drains.
 *
 * Everything here is synthesised. No sample files, no fetches, nothing
 * to fail to load, and the whole soundscape is a few hundred lines of
 * oscillators and filtered noise — which is also what makes it possible
 * to render offline for the regression harness §4.9 sketches.
 */

import type { SoundEvent, ViewSnapshot } from "../bridge/types";

/** Master gain, so nothing here is ever startling on a first load. */
const MASTER = 0.5;

/**
 * How fast a loop's gain chases its target, per second.
 *
 * Slow on purpose: a room stalling should read as the sound *dying
 * away*, which is what an ear hears as "it stopped", rather than as a
 * cut, which an ear hears as a glitch.
 */
const LOOP_GLIDE = 2.6;

/**
 * The most times one kind of one-shot may fire in a second.
 *
 * `GameEngine::frame` runs up to `MAX_TICKS_PER_FRAME` ticks in one
 * call, so at 4x a single frame routinely contains several ticks' worth
 * of events. Three mills finishing on one tick must not be three times
 * as loud, and a busy frame must not machine-gun. Coalescing by kind
 * within a frame and rate-limiting each kind is a JS concern entirely
 * and needs no change in Rust.
 */
const RATE_LIMIT: Partial<Record<SoundEvent, number>> = {
  Harvest: 6,
  Craft: 6,
  Pickup: 8,
  Deliver: 8,
  CarStop: 5,
  Shot: 10,
  Impact: 8,
  Burn: 3,
  Repair: 4,
  EnemyDown: 6,
  EnemyLeaves: 4,
  EnemyContact: 5,
  // A jammed tower spills steadily, and a steady spill must not become
  // a drone. Slow enough that it reads as punctuation.
  Spill: 2,
  Steal: 4,
};
/** Anything not named above may fire at most this often a second. */
const DEFAULT_RATE = 3;

/** One continuous bed, with a gain that follows the tower's state. */
interface Loop {
  gain: GainNode;
  target: number;
  current: number;
  /** Optional per-frame hook, for loops that move more than their gain. */
  tune?: (view: ViewSnapshot, now: number) => void;
}

export class AudioManager {
  private ctx: BaseAudioContext | null = null;
  private master: GainNode | null = null;
  private readonly loops = new Map<string, Loop>();
  private readonly lastFired = new Map<SoundEvent, number>();
  private started = false;
  private enabled = true;

  /**
   * Wire up the graph.
   *
   * **Audio cannot start without a gesture.** Browser autoplay policy
   * means the `AudioContext` is suspended until the player clicks, so
   * the opening frames are silent and the Playwright smoke test never
   * hears anything. Neither is a bug; both need to be true on purpose
   * rather than discovered as a mystery. This is called from a click
   * handler, and calling it twice is harmless.
   *
   * `into` exists for the offline harness (`web/e2e/audio.spec.ts`),
   * which drives this same graph under an `OfflineAudioContext` to
   * render a run to a WAV. Taking the context as an argument rather
   * than reaching for `window.AudioContext` is the entire difference
   * between a soundscape that can be reviewed without a browser and one
   * that can only be listened to live — and the point of the harness is
   * that it drives the *shipping* mixer, not a copy of it that has
   * drifted.
   */
  start(into?: BaseAudioContext): void {
    if (this.started) {
      void (this.ctx as AudioContext | null)?.resume?.();
      return;
    }
    let ctx: BaseAudioContext | undefined = into;
    if (!ctx) {
      const Ctor: typeof AudioContext | undefined =
        window.AudioContext ??
        (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
      if (!Ctor) return;
      ctx = new Ctor();
    }

    this.started = true;
    this.ctx = ctx;
    const master = ctx.createGain();
    master.gain.value = this.enabled ? MASTER : 0;
    master.connect(ctx.destination);
    this.master = master;

    this.buildBeds(ctx, master);
    // Only a live context may be resumed — an `OfflineAudioContext`
    // throws, because it has not started and starting it is
    // `startRendering`'s job.
    //
    // **Feature-detecting `resume` does not tell them apart.** It lives
    // on `BaseAudioContext`, so an offline context has one and it
    // rejects; the test harness's console filled with an unhandled
    // `InvalidStateError` on every run. `startRendering` is the property
    // that is actually exclusive to the offline one.
    if (!("startRendering" in ctx) && ctx.state === "suspended") {
      void (ctx as AudioContext).resume().catch(() => {
        // A context the browser will not resume until the player clicks
        // is the normal case, not an error: the first gesture calls
        // `start` again.
      });
    }
  }

  /** Whether the player wants to hear anything at all. */
  setEnabled(on: boolean): void {
    this.enabled = on;
    if (this.master && this.ctx) {
      this.master.gain.setTargetAtTime(on ? MASTER : 0, this.ctx.currentTime, 0.05);
    }
  }

  isEnabled(): boolean {
    return this.enabled;
  }

  isRunning(): boolean {
    return this.ctx !== null && this.ctx.state === "running";
  }

  /**
   * One frame of sound: the loops follow the snapshot, the one-shots
   * fire from the event list.
   */
  update(view: ViewSnapshot, events: readonly SoundEvent[], deltaMs: number, at?: number): void {
    const ctx = this.ctx;
    if (!ctx || !this.master) return;
    // `at` is the offline harness scheduling ahead of the clock. An
    // `OfflineAudioContext` renders faster than real time and its
    // `currentTime` stays at zero until `startRendering` runs, so a
    // mixer that reads the clock schedules the whole run on top of
    // itself at t=0 and renders one instant of noise. Live, this is
    // `currentTime` and nothing changes.
    const now = at ?? ctx.currentTime;

    this.tuneBeds(view, now);

    // Glide every loop toward its target rather than jumping.
    const step = Math.min(1, (deltaMs / 1000) * LOOP_GLIDE);
    for (const loop of this.loops.values()) {
      loop.current += (loop.target - loop.current) * step;
      loop.gain.gain.setTargetAtTime(loop.current, now, 0.03);
    }

    // Coalesce by kind within the frame — three mills finishing on one
    // tick is one craft sound, not three.
    const seen = new Set<SoundEvent>();
    for (const event of events) seen.add(event);
    for (const event of seen) this.fire(event, now);
  }

  dispose(): void {
    void (this.ctx as AudioContext | null)?.close?.();
    this.ctx = null;
    this.master = null;
    this.loops.clear();
    this.started = false;
  }

  // -------------------------------------------------------------------
  // Beds — the continuous half, driven by the snapshot
  // -------------------------------------------------------------------

  private buildBeds(ctx: BaseAudioContext, master: GainNode): void {
    const noise = makeNoiseBuffer(ctx);

    // **Every bed that carries state owns a band, and the ones that do
    // not carry state stay out of those bands.** This is the whole of
    // why the mix is laid out the way it is, and it was not obvious
    // until it was measured.
    //
    // The first version had the jungle on a `lowpass` at 620, which
    // sounds reasonable and is not: white noise through a lowpass keeps
    // *everything* under the corner, so the bed piled up the entire
    // sub-200 Hz range at eight times the level of every state-carrying
    // loop put together. Rendered and measured (`e2e/audio.spec.ts`), a
    // fully working tower and a completely dead one came out 6% apart
    // in the room band, and a stopped tower read *louder* in the leg
    // band than a walking one. Three of the eyes-closed test's four
    // binaries were unrecoverable, and the fault was the mix rather
    // than the wiring: the loops were reporting faithfully and nobody
    // could hear them over the weather.
    //
    // So the beds are bandpassed and pushed up out of the way, and the
    // bottom of the spectrum is left to the two things that mean
    // something down there — the legs and the hum.
    //
    //   0–200 Hz   the legs, and the electrical hum
    //   200–700    rooms working, and a car running
    //   700–2k     the jungle floor, and footsteps on the stairs
    //   2k–4.5k    daytime insects, and the canopy overhead
    //   4.5k+      night insects
    this.addNoiseLoop("jungle", ctx, master, noise, "bandpass", 1100, 0.9);
    this.addNoiseLoop("insects_day", ctx, master, noise, "bandpass", 3200, 4);
    this.addNoiseLoop("insects_night", ctx, master, noise, "bandpass", 6200, 6);
    this.addNoiseLoop("wind_night", ctx, master, noise, "bandpass", 900, 0.7);

    // The tower's own working noise.
    this.addToneLoop("rooms", ctx, master, 132, "sawtooth", 620);
    this.addToneLoop("hum", ctx, master, 50, "triangle", 160);
    this.addNoiseLoop("legs", ctx, master, noise, "lowpass", 150, 1.2);
    this.addNoiseLoop("footsteps", ctx, master, noise, "bandpass", 1500, 3);
    this.addToneLoop("car", ctx, master, 220, "triangle", 620);
    // **The canopy, not the sails.** M6 cut the sail deck; the bed
    // stays because what it was really voicing is bright open air,
    // which `exposure_pct` still measures — it just no longer buys
    // anybody any charge.
    this.addNoiseLoop("canopy", ctx, master, noise, "bandpass", 2600, 2.5);
  }

  /**
   * What each bed is reading, and when it goes silent.
   *
   * | Loop | Read from | Silent when |
   * |---|---|---|
   * | a room working | `RoomView.stalled` false | starved, backed up, unpowered, wrecked — all of which sound identical, because from outside they are |
   * | the legs | `journey.halt`, **not** `power.walking` | stopped, halted at a fork, arrived, or browned out |
   * | a car running | `ShaftView` car state | idle |
   * | footsteps | crew on the stairs | nobody on them |
   * | the canopy | `clock.exposure_pct` | shaded, or after dark |
   * | the electrical hum | `power.fill_permille` | thins as the bank drains, drops out on `brownout` |
   * | day/night beds | the sun curve | crossfaded, never stepped |
   * | the jungle | always | never |
   *
   * **The legs are the one loop with a trap in it, and the snapshot
   * already contains the answer.** `power.walking` is the player's
   * *intent*; whether the legs actually ran is `journey.halt`, which
   * §3.3 made distinguishable precisely because the renderer needed
   * telling which kind of standing still it was drawing. Audio inherits
   * that for free and uses all five: a tower that stopped and a tower
   * that cannot afford to move must not sound the same, and the
   * brown-out case has a treatment to match already — `drawLegs` gives
   * it a stuttering lift that never becomes a step, and the sound of
   * that is a motor asking and not being answered.
   */
  private tuneBeds(view: ViewSnapshot, now: number): void {
    // The crossfade follows the sun curve rather than the daypart
    // index, for the same reason the sun curve is a curve: a step
    // change at a boundary reads as a bug (§1.1).
    const day = clamp01(view.clock.sun_pct / 60);
    // **Levels here are per-band-width, not per-taste, and the numbers
    // look wrong until you measure them.** A biquad bandpass at Q 6
    // passes a sliver of white noise; one at Q 0.9 passes most of the
    // spectrum with gentle skirts. So the insect beds need gains an
    // order of magnitude above the jungle's to be *equally* audible,
    // and the jungle needs holding down or its skirts leak across every
    // band and flatten the day/night difference to nothing — measured,
    // a midday excerpt and a midnight one came out 1.5% apart in the
    // band the night insects have to themselves. Anything changed here
    // gets re-rendered (`e2e/audio.spec.ts`), not eyeballed.
    this.setTarget("jungle", 0.03 + day * 0.012);
    this.setTarget("insects_day", day * 1.1);
    this.setTarget("insects_night", (1 - day) * 1.5);
    this.setTarget("wind_night", (1 - day) * 0.04);

    // Rooms: how much of the tower is actually working. Not "is
    // anything wrong" — the loop is the work, and its absence is the
    // problem.
    let working = 0;
    let rooms = 0;
    for (const floor of view.tower.floors) {
      for (const room of floor.rooms) {
        if (room.wrecked) continue;
        rooms += 1;
        if (!room.stalled && room.active) working += 1;
      }
    }
    const busy = rooms > 0 ? working / rooms : 0;
    // The loudest thing in its band by a wide margin when the tower is
    // working, and gone when it is not. "A starved mill goes quiet" is
    // only a rule if the difference is audible.
    this.setTarget("rooms", busy * 0.09);

    // The legs, off `halt` rather than off intent.
    const halt = view.journey.halt;
    this.setTarget("legs", halt === "walking" ? 0.6 : 0);
    // A brown-out is a motor asking and not being answered: the hum
    // drops out entirely and the legs stutter rather than run.
    //
    // Kept *thin*. The hum is a pure 50 Hz tone and the legs are
    // broadband noise under 150 Hz, so at anything like equal gain the
    // tone owns the whole bottom of the spectrum and the legs vanish
    // underneath it — measured, a stopped tower and a walking one came
    // out within half a percent of each other down there, which makes
    // "is the tower walking" unanswerable. A hum should be something
    // you notice when it *changes*, not a floor.
    const drained = clamp01(view.power.fill_permille / 1000);
    this.setTarget("hum", view.power.brownout ? 0 : 0.004 + drained * 0.008);
    this.tuneNode("hum", (loop) => {
      // Pitch sags with the bank, so a tower running down is audible
      // before it is dark.
      const osc = (loop as Loop & { osc?: OscillatorNode }).osc;
      osc?.frequency.setTargetAtTime(44 + drained * 14, now, 0.4);
    });

    const onStairs = view.crew.filter(
      (member) => member.state === "climb" || member.state === "board",
    ).length;
    this.setTarget("footsteps", Math.min(0.14, onStairs * 0.05));

    const cars = view.tower.shafts.some((shaft) =>
      shaft.cars.some((car) => car.state === "moving"),
    );
    this.setTarget("car", cars ? 0.05 : 0);

    // The canopy overhead — bright air rather than income, since M6,
    // said in a register you can hear without looking.
    this.setTarget("canopy", clamp01(view.clock.exposure_pct / 100) * 0.09);
  }

  // -------------------------------------------------------------------
  // One-shots — the punctuation half, driven by the event list
  // -------------------------------------------------------------------

  private fire(event: SoundEvent, now: number): void {
    const ctx = this.ctx;
    const master = this.master;
    if (!ctx || !master) return;

    const limit = RATE_LIMIT[event] ?? DEFAULT_RATE;
    const last = this.lastFired.get(event) ?? -Infinity;
    if (now - last < 1 / limit) return;
    this.lastFired.set(event, now);

    switch (event) {
      // The chain. Small, wooden, unglamorous — these fire constantly
      // and are the sounds most likely to grate, so they are the
      // quietest things here.
      case "Harvest":
        this.blip(now, 320, 0.06, 0.05, "triangle");
        break;
      case "Craft":
        this.blip(now, 210, 0.09, 0.06, "square");
        break;
      case "Pickup":
        this.blip(now, 430, 0.04, 0.03, "triangle");
        break;
      case "Deliver":
        this.blip(now, 300, 0.05, 0.035, "triangle");
        break;
      case "CarStop":
        this.blip(now, 520, 0.07, 0.04, "sine");
        break;
      case "Burn":
        this.hiss(now, 0.22, 0.05, 900);
        break;

      // The world changing under the tower.
      case "BandChange":
        this.hiss(now, 0.5, 0.045, 1800);
        break;
      case "RegionChange":
        this.chime(now, [196, 262, 330], 1.6, 0.07);
        break;
      case "Arrived":
        this.chime(now, [262, 330, 392, 523], 3.2, 0.09);
        break;

      // The siege. Nothing here is a weapon report and nothing is
      // triumphant — `DECISIONS.md` §8, and §2.2's refusal to grade a
      // creature that walked away.
      //
      // **Loud, and that took measuring.** These were first written at
      // roughly the chain's level, which is wrong twice over: the chain
      // sounds fire constantly and these do not, and something biting
      // your home should be the loudest thing in the frame. Rendered
      // and measured (`e2e/audio.spec.ts`), a minute with a creature on
      // the tower for a fifth of it was *indistinguishable* from a quiet
      // minute — the impacts never rose above the clip's own median,
      // which is the eyes-closed test's third question failing outright.
      // A bite is now several times the beds it lands on.
      case "WaveArrives":
        this.chime(now, [147, 175], 1.4, 0.16);
        break;
      case "EnemyContact":
        this.thud(now, 120, 0.18, 0.2);
        break;
      case "Impact":
        this.thud(now, 90, 0.24, 0.3);
        break;
      case "Breach":
        this.thud(now, 62, 0.6, 0.42);
        break;
      case "Wrecked":
        this.thud(now, 48, 0.9, 0.46);
        break;
      case "Severed":
        this.thud(now, 40, 1.3, 0.5);
        break;
      case "Shot":
        this.blip(now, 640, 0.06, 0.13, "square");
        break;
      case "EnemyDown":
        // Falling, not a kill sting. A pitch that drops and stops.
        this.sweep(now, 300, 120, 0.28, 0.16);
        break;
      case "EnemyLeaves":
        // **Closes M2's deferral.** `Leaving` and `Dying` are distinct
        // in state and in the snapshot but sounded identical, so walking
        // a wave off and shooting it down were indistinguishable. §2.2
        // refuses to count the first as repelled; the audio refuses too.
        // A rustle going away from you, and nothing else.
        this.hiss(now, 0.65, 0.035, 2600);
        break;
      case "HeartseedLost":
        this.chime(now, [147, 110, 87], 4.5, 0.12);
        break;

      case "Repair":
        this.blip(now, 180, 0.11, 0.05, "square");
        break;

      // M4's three.
      case "MealServed":
        // The warmest moment in the tower, and the audible confirmation
        // that the kitchen chain is alive. A small warm third.
        this.chime(now, [392, 494], 0.9, 0.05);
        break;
      case "Spill":
        // Something the chain worked for, falling a long way and not
        // coming back. Deliberately not `Deliver`'s little wooden knock
        // — a descending sweep, so a tower that is spilling is audibly
        // losing something rather than putting it away.
        this.sweep(now, 420, 90, 0.5, 0.07);
        break;
      case "Steal":
        // A rustle and a wingbeat going away from you. Not `Impact`:
        // nothing was hit and nothing needs mending, and it should not
        // sound like a blow.
        this.hiss(now, 0.35, 0.07, 3200);
        this.blip(now + 0.06, 900, 0.07, 0.05, "triangle");
        break;
      case "Daybreak":
        // A new day. The tower's one daily ritual, and the only
        // reliable way to *hear* what time it is. It used to be the
        // rota's handover, twice a day; M6 cut the rota and crew now
        // sleep when they are tired, so there is no handover to sound.
        this.chime(now, [262, 392], 2.4, 0.06);
        break;
      default:
        break;
    }
  }

  // -------------------------------------------------------------------
  // Synthesis primitives
  // -------------------------------------------------------------------

  private blip(
    at: number,
    freq: number,
    length: number,
    peak: number,
    shape: OscillatorType,
  ): void {
    const ctx = this.ctx;
    const master = this.master;
    if (!ctx || !master) return;
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();
    osc.type = shape;
    osc.frequency.value = freq;
    gain.gain.setValueAtTime(0, at);
    gain.gain.linearRampToValueAtTime(peak, at + 0.008);
    gain.gain.exponentialRampToValueAtTime(0.0001, at + length);
    osc.connect(gain).connect(master);
    osc.start(at);
    osc.stop(at + length + 0.02);
  }

  private sweep(at: number, from: number, to: number, length: number, peak: number): void {
    const ctx = this.ctx;
    const master = this.master;
    if (!ctx || !master) return;
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();
    osc.type = "triangle";
    osc.frequency.setValueAtTime(from, at);
    osc.frequency.exponentialRampToValueAtTime(Math.max(20, to), at + length);
    gain.gain.setValueAtTime(peak, at);
    gain.gain.exponentialRampToValueAtTime(0.0001, at + length);
    osc.connect(gain).connect(master);
    osc.start(at);
    osc.stop(at + length + 0.02);
  }

  private thud(at: number, freq: number, length: number, peak: number): void {
    const ctx = this.ctx;
    const master = this.master;
    if (!ctx || !master) return;
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();
    const filter = ctx.createBiquadFilter();
    filter.type = "lowpass";
    filter.frequency.value = freq * 4;
    osc.type = "sine";
    osc.frequency.setValueAtTime(freq * 1.6, at);
    osc.frequency.exponentialRampToValueAtTime(freq, at + length * 0.6);
    gain.gain.setValueAtTime(peak, at);
    gain.gain.exponentialRampToValueAtTime(0.0001, at + length);
    osc.connect(filter).connect(gain).connect(master);
    osc.start(at);
    osc.stop(at + length + 0.02);
  }

  private chime(at: number, freqs: readonly number[], length: number, peak: number): void {
    freqs.forEach((freq, i) => {
      const start = at + i * 0.09;
      const ctx = this.ctx;
      const master = this.master;
      if (!ctx || !master) return;
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.type = "sine";
      osc.frequency.value = freq;
      gain.gain.setValueAtTime(0, start);
      gain.gain.linearRampToValueAtTime(peak, start + 0.02);
      gain.gain.exponentialRampToValueAtTime(0.0001, start + length);
      osc.connect(gain).connect(master);
      osc.start(start);
      osc.stop(start + length + 0.05);
    });
  }

  private hiss(at: number, length: number, peak: number, cutoff: number): void {
    const ctx = this.ctx;
    const master = this.master;
    if (!ctx || !master) return;
    const src = ctx.createBufferSource();
    src.buffer = makeNoiseBuffer(ctx);
    src.loop = true;
    const filter = ctx.createBiquadFilter();
    filter.type = "bandpass";
    filter.frequency.value = cutoff;
    filter.Q.value = 1.4;
    const gain = ctx.createGain();
    gain.gain.setValueAtTime(peak, at);
    gain.gain.exponentialRampToValueAtTime(0.0001, at + length);
    src.connect(filter).connect(gain).connect(master);
    src.start(at);
    src.stop(at + length + 0.02);
  }

  // -------------------------------------------------------------------
  // Loop plumbing
  // -------------------------------------------------------------------

  private addToneLoop(
    key: string,
    ctx: BaseAudioContext,
    master: GainNode,
    freq: number,
    shape: OscillatorType,
    cutoff: number,
  ): void {
    const osc = ctx.createOscillator();
    const filter = ctx.createBiquadFilter();
    const gain = ctx.createGain();
    osc.type = shape;
    osc.frequency.value = freq;
    filter.type = "lowpass";
    filter.frequency.value = cutoff;
    gain.gain.value = 0;
    osc.connect(filter).connect(gain).connect(master);
    osc.start();
    const loop: Loop & { osc?: OscillatorNode } = { gain, target: 0, current: 0 };
    loop.osc = osc;
    this.loops.set(key, loop);
  }

  private addNoiseLoop(
    key: string,
    ctx: BaseAudioContext,
    master: GainNode,
    buffer: AudioBuffer,
    type: BiquadFilterType,
    freq: number,
    q: number,
  ): void {
    const src = ctx.createBufferSource();
    src.buffer = buffer;
    src.loop = true;
    const filter = ctx.createBiquadFilter();
    filter.type = type;
    filter.frequency.value = freq;
    filter.Q.value = q;
    const gain = ctx.createGain();
    gain.gain.value = 0;
    src.connect(filter).connect(gain).connect(master);
    src.start();
    this.loops.set(key, { gain, target: 0, current: 0 });
  }

  private setTarget(key: string, value: number): void {
    const loop = this.loops.get(key);
    if (loop) loop.target = Math.max(0, value);
  }

  private tuneNode(key: string, fn: (loop: Loop) => void): void {
    const loop = this.loops.get(key);
    if (loop) fn(loop);
  }
}

/** Two seconds of white noise, reused by every noise loop and hiss. */
let noiseBuffer: AudioBuffer | null = null;
function makeNoiseBuffer(ctx: BaseAudioContext): AudioBuffer {
  if (noiseBuffer && noiseBuffer.sampleRate === ctx.sampleRate) return noiseBuffer;
  const frames = ctx.sampleRate * 2;
  const buffer = ctx.createBuffer(1, frames, ctx.sampleRate);
  const data = buffer.getChannelData(0);
  // Deterministic, so two runs of the offline harness §4.9 sketches
  // would render the same WAV. `Math.random` here would make the one
  // reviewable artefact un-diffable.
  let seed = 0x9e37_79b9;
  for (let i = 0; i < frames; i += 1) {
    seed = (seed * 1_664_525 + 1_013_904_223) >>> 0;
    data[i] = (seed / 0xffff_ffff) * 2 - 1;
  }
  noiseBuffer = buffer;
  return buffer;
}

function clamp01(value: number): number {
  return Math.max(0, Math.min(1, value));
}
