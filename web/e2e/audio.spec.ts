import { test, type Page } from "@playwright/test";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";

/**
 * The audio counterpart of `capture.spec.ts`, and the honest version of
 * M4's eyes-closed test (`SYSTEMS.md` §4.9).
 *
 * Playwright cannot listen. What it *can* do is exactly what §4.9
 * sketched and left unbuilt: step a real run headlessly, capture each
 * frame's `SoundEvent[]` plus the snapshot behind it, then drive **the
 * shipping `AudioManager`** from that log under an `OfflineAudioContext`
 * and render a WAV. That makes the soundscape deterministic, diffable,
 * and reviewable without a browser.
 *
 * And it turns the criterion into something that can be *checked* rather
 * than only felt. The eyes-closed test asks whether a listener can
 * recover four binaries from sound alone — day or night, chain running
 * or stalled, something attacking, walking or stopped. That is first of
 * all a question about whether the information is present in the signal
 * at all. So this renders three unlabelled sixty-second excerpts,
 * measures per-second energy in five frequency bands, and reports both
 * the measurements and what was actually true, so the two can be
 * compared. A mix that cannot separate a working mill from a starved one
 * has found that the loops are decorating rather than reporting — the
 * exact failure §4.9 names.
 *
 * **What this does not replace is a person.** A signal can carry a fact
 * and still not be legible to an ear, and nothing here can say whether
 * the tower sounds *good*. It answers "is it in there", which is the
 * half that was previously unanswerable.
 *
 * Everything happens inside the page and only the rendered audio comes
 * back. The first version passed the frame log out across the bridge and
 * died on it: 3,600 frames each carrying a whole `ViewSnapshot` is a
 * hundred megabytes, which is the CDP message ceiling exactly.
 */

const OUT = "capture/audio";
/** Sixty seconds, as the criterion asks for. */
const SECONDS = 60;
/**
 * Half CD rate. Everything in this mix lives under 6 kHz, and halving
 * the rate halves what has to cross the bridge as base64.
 */
const RATE = 22_050;

/** What was true while an excerpt was recorded, as a share of seconds. */
interface Truth {
  clip: string;
  night: number;
  chainStalled: number;
  underAttack: number;
  /**
   * Anything out at all, in contact or still closing.
   *
   * Reported alongside `underAttack` because the two are different
   * questions and the audio answers the wider one: `WaveArrives` fires
   * when a wave appears on the horizon, which a listener hears as "the
   * jungle has noticed us" well before anything is biting. Scoring only
   * against contact marks that correct report as a false positive.
   */
  enemiesOut: number;
  stopped: number;
}

interface Rendered {
  wav: string;
  bands: number[][];
  truth: Omit<Truth, "clip">;
  /** How many of each one-shot fired, so a spike can be attributed. */
  events: Record<string, number>;
}

declare global {
  interface Window {
    __audio?: {
      run(plan: string, seconds: number, rate: number): Promise<Rendered>;
    };
  }
}

async function arm(page: Page): Promise<void> {
  await page.evaluate(() => {
    const hooks = window.__understory!;

    const answerFork = (): void => {
      const fork = hooks.view().journey.fork;
      if (fork && fork.answer === null) hooks.send({ TakeFork: { branch: 0 } });
    };

    const buy = (id: string, wide: number, tries = 30): boolean => {
      for (let attempt = 0; attempt < tries; attempt += 1) {
        for (const floor of hooks.view().tower.floors) {
          for (let slot = 0; slot + wide <= floor.slots; slot += 1) {
            if (hooks.send({ PlaceRoom: { room: id, floor: floor.index, slot } }) === "Ok") {
              return true;
            }
          }
        }
        answerFork();
        hooks.step(600);
      }
      return false;
    };

    const walkTo = (permille: number, budget: number): void => {
      for (let attempt = 0; attempt < budget; attempt += 1) {
        if (Math.abs(hooks.view().clock.permille - permille) < 25) return;
        answerFork();
        hooks.step(60);
      }
    };

    window.__audio = {
      async run(plan, seconds, rate) {
        // **One tick per frame, at the tick rate.** The engine runs at
        // 30 Hz, so sixty seconds of audio is 1,800 ticks — sixty
        // seconds of play at 1×, which is what the criterion asks a
        // listener to sit through. The first version stepped two ticks
        // per display frame and rendered four minutes of simulation into
        // one minute of sound: every rhythm in the mix was wrong by 4×,
        // and a "daytime" excerpt walked into the night halfway through.
        const FRAME_MS = 1000 / 30;
        const TICKS_PER_FRAME = 1;

        // Walk each excerpt to the state it is an excerpt *of* before
        // any of it is recorded. An unlabelled recording still has to be
        // a recording of something.
        buy("room.canteen", 2);
        buy("room.bunk", 2);
        if (plan === "day") {
          walkTo(450, 4000);
        }
        if (plan === "stalled") {
          walkTo(450, 4000);
          // Switch the chain off rather than waiting for it to break.
          // Every production loop silent is the state the criterion asks
          // a listener to hear, and this is the fastest honest way there.
          for (const floor of hooks.view().tower.floors) {
            for (const room of floor.rooms) {
              hooks.send({
                SetRoomActive: { floor: floor.index, slot: room.slot, active: false },
              });
            }
          }
          hooks.send({ SetStriding: { walking: false } });
        }
        if (plan === "siege") {
          buy("room.cutter_arm", 2);
          for (let attempt = 0; attempt < 8000; attempt += 1) {
            if (hooks.view().siege.enemies.some((enemy) => enemy.state === "attack")) break;
            answerFork();
            hooks.step(60);
          }
        }

        const frames = Math.round((seconds * 1000) / FRAME_MS);
        const ctx = new OfflineAudioContext(1, Math.ceil(seconds * rate), rate);
        // The *shipping* mixer, imported by a non-literal specifier so
        // the spec's typechecker does not try to resolve a browser path
        // from Node. A harness that rendered a copy of the mixer would
        // keep passing while the game drifted away from it.
        const specifier = "/src/engine/AudioManager.ts";
        const mod = (await import(/* @vite-ignore */ specifier)) as {
          AudioManager: new () => {
            start(ctx: BaseAudioContext): void;
            update(view: unknown, events: unknown, deltaMs: number, at: number): void;
          };
        };
        const audio = new mod.AudioManager();
        audio.start(ctx);

        let night = 0;
        let stalled = 0;
        let attacked = 0;
        let out = 0;
        let stopped = 0;
        let sampled = 0;
        let at = 0;
        // What fired, and how often. Without this a burst in the
        // rendered audio is a mystery: the analysis can see that
        // *something* was loud in a band and has no way to say what.
        const fired: Record<string, number> = {};
        for (let i = 0; i < frames; i += 1) {
          answerFork();
          const events = hooks.step(TICKS_PER_FRAME);
          for (const event of events) fired[event] = (fired[event] ?? 0) + 1;
          const view = hooks.view();
          // Schedule ahead of the clock: an offline context renders
          // faster than real time and its `currentTime` stays at zero,
          // so a mixer reading the clock stacks the whole run at t=0.
          audio.update(view, events, FRAME_MS, at);
          at += FRAME_MS / 1000;

          if (i % 30 === 0) {
            sampled += 1;
            if (view.clock.sun_pct < 30) night += 1;
            const rooms = view.tower.floors
              .flatMap((floor) => floor.rooms)
              .filter((room) => !room.wrecked);
            const working = rooms.filter((room) => !room.stalled && room.active).length;
            if (rooms.length > 0 && working / rooms.length < 0.34) stalled += 1;
            if (view.siege.enemies.some((enemy) => enemy.state === "attack")) attacked += 1;
            if (view.siege.enemies.length > 0) out += 1;
            if (view.journey.halt !== "walking") stopped += 1;
          }
        }

        const buffer = await ctx.startRendering();
        const samples = buffer.getChannelData(0);

        // Per-second energy in five bands, by Goertzel at a spread of
        // probes per band. Crude on purpose: a fact only recoverable by
        // a sophisticated analysis is not recoverable by an ear either.
        //
        // **Twenty-four probes a band, not six.** A Goertzel bin sees a
        // pure tone sitting exactly on it as roughly N times louder than
        // broadband noise of the same amplitude, so with few probes a
        // single sine dominates a band that is mostly noise — which is
        // a property of the ruler rather than of the sound. More probes
        // dilutes that: a tone still shows, but it no longer decides the
        // band. This mattered: a 50 Hz hum was reading as the loudest
        // thing in the tower over legs that were twenty times its
        // amplitude.
        const EDGES = [0, 200, 700, 2000, 4500, rate / 2];
        const bands: number[][] = [];
        for (let second = 0; second * rate < samples.length; second += 1) {
          const from = second * rate;
          const to = Math.min(samples.length, from + rate);
          const row: number[] = [];
          for (let band = 0; band + 1 < EDGES.length; band += 1) {
            const lo = EDGES[band]!;
            const hi = EDGES[band + 1]!;
            let energy = 0;
            const PROBES = 24;
            for (let probe = 0; probe < PROBES; probe += 1) {
              const freq = lo + ((hi - lo) * (probe + 0.5)) / PROBES;
              const w = (2 * Math.PI * freq) / rate;
              const coeff = 2 * Math.cos(w);
              let s1 = 0;
              let s2 = 0;
              for (let i = from; i < to; i += 2) {
                const s0 = samples[i]! + coeff * s1 - s2;
                s2 = s1;
                s1 = s0;
              }
              energy += s1 * s1 + s2 * s2 - coeff * s1 * s2;
            }
            row.push((Math.sqrt(Math.max(0, energy)) / ((to - from) / 2)) * 1000);
          }
          bands.push(row.map((value) => Math.round(value * 1000) / 1000));
        }

        // 16-bit mono WAV, written by hand and base64'd for the trip
        // out. Nothing here needs a library.
        const header = 44;
        const raw = new Uint8Array(header + samples.length * 2);
        const dv = new DataView(raw.buffer);
        const ascii = (offset: number, text: string): void => {
          for (let i = 0; i < text.length; i += 1) dv.setUint8(offset + i, text.charCodeAt(i));
        };
        ascii(0, "RIFF");
        dv.setUint32(4, 36 + samples.length * 2, true);
        ascii(8, "WAVEfmt ");
        dv.setUint32(16, 16, true);
        dv.setUint16(20, 1, true);
        dv.setUint16(22, 1, true);
        dv.setUint32(24, rate, true);
        dv.setUint32(28, rate * 2, true);
        dv.setUint16(32, 2, true);
        dv.setUint16(34, 16, true);
        ascii(36, "data");
        dv.setUint32(40, samples.length * 2, true);
        for (let i = 0; i < samples.length; i += 1) {
          const clipped = Math.max(-1, Math.min(1, samples[i]!));
          dv.setInt16(header + i * 2, Math.round(clipped * 32_767), true);
        }
        let binary = "";
        for (let i = 0; i < raw.length; i += 8192) {
          binary += String.fromCharCode(...raw.subarray(i, i + 8192));
        }

        const share = (n: number): number => Math.round((n / Math.max(1, sampled)) * 100);
        return {
          wav: btoa(binary),
          bands,
          events: fired,
          truth: {
            night: share(night),
            chainStalled: share(stalled),
            underAttack: share(attacked),
            enemiesOut: share(out),
            stopped: share(stopped),
          },
        };
      },
    };
  });
}

test("render the tower to a WAV, and see what is recoverable from it", async ({ page }) => {
  test.setTimeout(900_000);
  await page.setViewportSize({ width: 1280, height: 720 });

  const truths: Truth[] = [];
  const features: Record<string, number[][]> = {};
  const events: Record<string, Record<string, number>> = {};

  // Three excerpts, deliberately different in the ways the criterion
  // asks about, and written to disk as `a`, `b`, `c` so the filenames
  // give nothing away.
  const plans = [
    { plan: "day", seed: 909, as: "a" },
    { plan: "stalled", seed: 909, as: "b" },
    { plan: "siege", seed: 4242, as: "c" },
  ];

  for (const { plan, seed, as } of plans) {
    await page.goto(`/?seed=${seed}`);
    await page.waitForFunction(() => window.__understory !== undefined, null, { timeout: 20_000 });
    await arm(page);

    const out = await page.evaluate(
      async (args: { which: string; seconds: number; rate: number }) =>
        await window.__audio!.run(args.which, args.seconds, args.rate),
      { which: plan, seconds: SECONDS, rate: RATE },
    );

    const path = join(OUT, `${as}.wav`);
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, Buffer.from(out.wav, "base64"));
    features[as] = out.bands;
    events[as] = out.events;
    truths.push({ clip: as, ...out.truth });
    // Picked by repeated max rather than by sorting: `toSorted` is
    // ES2023 and this project targets ES2022, so its callback arguments
    // come back untyped, and `sort` mutates. Eight items does not need
    // either.
    const counts = new Map<string, number>(Object.entries(out.events));
    const loudest: string[] = [];
    while (loudest.length < 8 && counts.size > 0) {
      let best = "";
      let most = -1;
      for (const [name, count] of counts) {
        if (count > most) {
          best = name;
          most = count;
        }
      }
      counts.delete(best);
      loudest.push(`${best}x${most}`);
    }
    console.log(`rendered ${path} — ${out.bands.length}s — ${loudest.join(" ")}`);
  }

  writeFileSync(join(OUT, "features.json"), JSON.stringify({ features, events, truths }, null, 2));
  console.log("\n=== what was actually true, percent of sampled seconds ===");
  console.table(truths);
});
