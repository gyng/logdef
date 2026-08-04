# Renderer notes

What the custom WebGL2 renderer draws procedurally, what was tried and reverted, and why.
Engineering, not art direction — `docs/ART.md` is the artist's document and does not need any
of this.

The renderer is one instanced quad batch: a single VAO and one
`drawArraysInstanced(TRIANGLE_STRIP, 0, 4, count)` per frame, with position, size, two
colours, corner radius and softness as per-instance attributes. Everything below is more
instances in that batch — no textures, no second pass, no new state.

**Measured cost**: 16.6 ms p50 and 18 ms p95 over 240 frames on an RTX 3080, vsync-locked at
60, and a big busy tower measures the same as a bare one. The sim is 1.8–2.6 µs a tick and the
whole WASM bridge is under 0.1 ms a frame. See `make instruments`.

---

## What is drawn, and what was tried

Art is not the only way to make this look better. Three procedural jobs are worth more than
most of the manifest above, and one popular idea turns out to be blocked.

### Worth doing

**Burner smoke.** The burner's whole design point is that its smoke provokes
(`provocation_per_burn` 18, and provocation is the only difficulty dial in the game). It is
currently invisible. A plume from a running burner is the most `DECISIONS.md` §8-shaped thing
available — it puts the difficulty dial on screen as a fact about the world rather than a
number. A handful of soft quads on a noise-drifted path, opacity from the burn state.

**Water in the drowned city.** Region 2 is named for water and the renderer draws none. A
flooded ground plane with a slow shimmer and a reflected tower silhouette would do more for
that region's identity than any sprite.

**Not rain or weather.** Pretty, and it says nothing. This renderer's discipline is that a
visual carries information: the legs report the halt state, a crew member's step cadence
reports hunger, a stalled room goes quiet. Rain would be the first purely decorative system
in it.

### Planted feet and IK legs — done, and the fix was not in the legs

The tower's feet used to **slide**: `drawLegs` swung each foot sinusoidally around the hip
through the whole cycle, including the half it was supposed to be bearing weight on. Planting
the foot — holding it still in the world and letting it drift backwards across the screen at
the scroll rate — is the standard fix.

**It failed the first time, and the arithmetic is why.** At `GROUND_FRACTION` 0.86 the leg
spanned 91 px, so a two-bone joint could swing ±41 px — about half a slot. The ground scrolls
at `slotW * 0.5` per pace and the tower walks 18 paces a second, so a correctly planted foot
implied **nine steps a second**. Asking for a calmer two-slot stride was worse: ±82 px against
a ±41 px reach, both legs locked straight, and the tower skied.

**The fix was in `layout.ts`, not in the legs**, and it came in two parts that compound:

| | before | after |
|---|---|---|
| `GROUND_FRACTION` | 0.86 | **0.72** — leg span 91 px → 181 px |
| `HORIZON_FRACTION` | 0.52 | **0.40** — keeps the parallax band's depth |
| `paceW` | `slotW * 0.5` | **`slotW * 0.2`** — terrain crosses in 5.6 s, not 2.2 |
| Cadence | 8.7 steps/s | **1.7 steps/s** |

The leg room was *free*: `slotW` is bound by `byWidth` (80 at 1600×900) rather than `byHeight`
(95.6), so the tower could be given a quarter of the frame to stand in without getting any
smaller. 0.72 is the exact floor before it starts shrinking.

The general lesson, which is worth more than the legs: **a walk cycle is not an animation
problem, it is a units problem.** Foot reach, stride length and scroll rate are one equation,
and if any two are chosen independently the third is wrong. `STRIDE_SLOTS` in `scene.ts` now
derives the cadence from the other two so it cannot drift again.

### Smoke and water — done

Both are in, both are quads in the existing batch, and neither needed a texture.

**Burner smoke** vents from the roof above the burner's own column — not from the room, because
smoke draws after the tower and a plume started at the burner billows up through the bunks
above it. Eight puffs on one rising path, spaced along their own lives so the column is
continuous rather than pulsed, shearing sideways as they climb. Gated on the burner actually
*burning*: switched off, out of bamboo or wrecked draws nothing, so the plume reports what the
tower is doing rather than what it owns.

It also leans sideways harder than physics wants, and that is framing rather than fluid
dynamics: the roof sits about a floor's height below the HUD, so a plume that climbs three
floors spends most of its life off the top of the screen.

**Flood water** draws only where the band underfoot is drowned street: a sheet over the ground
darkening with depth, a bright waterline exactly on the ground line so the tower reads as
being *in* it, and six slow bands drifting against the stride. The bands run off `clock`
rather than distance, because water moves whether or not the tower does — a still tower on
still water is the one thing that would look wrong.

`web/e2e/effects.spec.ts` photographs both. Nothing else does: the capture harness never
builds a burner, and its drowned-city stills depend on where a nondeterministic script happens
to stop.

### The wake, the sails and the arm — done, and all three came free with the feet

**Splash and submersion.** A foot landing in the drowned street throws a ring and a little
spray, and everything below the waterline is re-tinted after the legs draw so the tower wades
rather than skating on the surface. **None of this was possible before the feet planted** — a
splash is an *event*, and a pendulum has no touchdown to hang one on. The foot solve is pulled
out into `feet()` so the water can ask where they are and how long since they landed.

**The sails fill and slacken with `exposure_pct`** — sun after terrain, which is the number
they are actually paid in. A sail room in dense canopy at 15% and one in open clearing at 100%
used to draw identically, so the tower's entire charge income was invisible on the one part of
it that earns the income. Walk into shade now and the canvas goes slack *before* the bank
starts falling, which is the §8 order: see it in the world first, read it off a gauge second.

**The cutter arm sweeps** while the tower is covering ground, and hangs still when it is not.
Off `distance` rather than the clock, because terrain intake is paid per pace — a halted tower
has an arm that has nothing to do, and a stalled one holds its rest angle, which is the same
silence a starved mill draws.

**Loads have their own colour**, keyed by item id in `palette.cargoOf`. Every crate used to be
the same green, so the cross-section reported *that* the tower was busy and never *what with* —
and which chain is winning the stairs is the entire question `DESIGN.md` insight 1 asks. The
crate also scales a little with the amount, so a full armful reads heavier than a single item;
`carry_capacity` is 3, so that is a three-step tell rather than a gauge. Kept in the renderer
rather than the content pack, because it is a presentation choice — `ItemInfo` carries a glyph
and an order because those are the only two the simulation ever needed.

The pattern in all of them: **the signal already existed in the snapshot and nothing was
drawing it.** That is where the cheap wins are, and it is a far better filter than "what would
look nice". What is left by that test is thin — lamps dimming in a brown-out would say a thing
the legs already say, and rain would say nothing at all.
