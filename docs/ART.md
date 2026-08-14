# Art manifest

Everything needed to generate the game's art.

For a compact inventory of every source, prompt sidecar, shipped derivative,
atlas cell order and live visual checkpoint, see
[`GENERATED-ART.md`](GENERATED-ART.md). It is generated from the manifest,
pipeline constants and files on disk; regenerate it with
`python scripts/generate_art_index.py` rather than editing it by hand.

**The prompts are not in this file.** They live in `docs/art-manifest.json`, which is the
source of truth for every fact about an asset: its prompt, its sizes, its format and where the
file goes. This file is the prose companion — why the art looks like this, and the house rules
a batch run has to follow.

That split is deliberate. A fact kept in two places drifts, and this repo has paid for it more
than once: `BALANCE.md`'s header records three rows whose prose claimed a grade their own
column contradicted, and the twelve-item icon list that used to sit in this file had been
missing three items since M6. **So do not copy a size, a path or a prompt body into this
document.** If you want to know what an asset is, read the manifest.

`crates/core/src/tests/art_manifest.rs` enforces the part that can be enforced: the sheets
have to list exactly the items, creatures and crew the content pack has.

---

## 1. What you are drawing

*Understory* is a walking garden-tower striding through a jungle that swallowed the old world.
You watch it from the side, in cross-section, like a doll's house — the rooms are open to you,
the crew are named people, and the whole thing walks. It is solarpunk and melancholy: nothing
here is a war machine, the creatures are animals defending their territory, and a run ends
with an arrival rather than a victory.

**The jungle grew over a city, not over nothing.** This is post-apocalyptic solarpunk with a
lot of salvaged late-20th-century computing in it: beige plastic gone yellow, CRT terminals,
ribbon cable and patch leads, perforated steel racks, all scuffed and patched and half
swallowed by growth. The tower is *built out of that* — bamboo and salvaged timber over
scavenged panelling, with cable runs threaded between its floors.

Keep it overgrown **tech**, not wilderness. The rule of thumb is in the manifest's `tech`
canon: where you would otherwise draw a bare rock or a plain tree trunk, prefer a concrete
edge, a dead server rack, or a bundle of cable under moss. The world content agrees with this
and the early prompts did not — the regions a run actually crosses are the Drowned City, the
Collapsed District, Ruin Field, Flooded Boulevard and Tide Road, and the renderer has named
colours for `salvage`, `stripped`, `wreck` and `verdigris`. A primeval jungle would be a
different game.

A lit screen glows as a flat amber or phosphor-green rectangle with faint scanlines — never
legible text and never a recognisable interface, which is also why the style block's "no text"
negative survives contact with a room full of terminals.

### The canon, and why it is in the manifest rather than here

`subject_canon` in the manifest holds the descriptions that must not vary between assets —
the tower, its roof, its hull, the crew, the creatures. Prompts reference them as `{tower}`,
`{crew}` and so on, and a test fails if a prompt uses a name the canon does not define.

They are there because **an error in the canon is an error in every asset at once**, and this
file got two of them wrong for a whole milestone:

- **The tower has six legs, not two or four.** They form three mechanical bogies, each with a
  near and rear limb, and carry the hull in an alternating tripod gait. Two reads as a person;
  four evenly spaced legs read as stilts in strict side view. Six produces the low, redundant
  walking-machine silhouette the live renderer now draws.
- **The joint is a mechanical linkage, not an animal knee.** Each limb forms a broad two-link
  Z with a visible circular servo. Rear-plane limbs sit slightly higher and darker. Never ask
  for a human knee or a bird's backward hock.
- **There are no sails.** M6 cut the canopy sails out of the game entirely (`SYSTEMS.md`
  §6.10) and the roof carries a row of planters now — a garden. Every mention of a sail in the
  code is a comment about their removal. The old cover prompt asked for "sail panels on the
  roof", which would have put a deleted mechanic on the itch.io page.

### The character direction

**Compact semi-chibi anime sprites, inside the same 1980s watercolour magazine style.** Roughly
2.5 to 2.75 heads tall, with shortened limbs and slightly larger hands and boots so a figure forty
pixels tall is still readable. Adult builds, jawlines and work-worn posture keep them from
becoming full chibi, children or mascots.

The thing to hold onto is that **semi-chibi is doing a readability job, not a comedy one.**
This is a melancholy game. The crew are adults at work: calm, tired, absorbed in a task. No
gag expressions, no sweat-drops, no wide grins, no heroic poses. `DECISIONS.md` §8 is a tone
guardrail — defenders rather than soldiers — and it binds the art as much as the code. A root
borer is a beetle doing a beetle thing, not a monster.

`crew_tone` and `creature` in the canon say this in prompt form; paste them, don't paraphrase.

### How the art enters the game

The first version of this document described a wholly procedural renderer. The art pass layers
painted skies, scenery, people, creatures, room interiors and a modular walker frame over that
deterministic geometry. **Generating a picture is still the cheap half**; the expensive half is
wiring it in without erasing the state the procedural layer communicates.

| Tier | Where it goes | Code needed |
|---|---|---|
| **0** | itch.io page, title screen, arrival card | none — a CSS background |
| **1** | Paper grain, wash, sky strips, over the game itself | one textured quad |
| **2** | Scenery props — trees, ferns, ruins | a texture atlas |
| **3** | Walker-frame and shaft components, room interiors, crew, creatures, item icons | a different renderer — **read §5 first** |

### Terrain paintings are atmosphere, not world state

Tier 2 now has one parallax painting for each authored terrain character: canopy, clearing,
ruin field, drowned street and the salt-flat coast. These are **finite paintings**, not
seamless wallpaper. The renderer cover-crops the appropriate painting into the available
terrain area; it must never hard-repeat it across the screen. A repeated ruin, pylon or tree
at a fixed interval turns a landscape into a texture strip, and a generated watercolour edge
is not reliably seamless even when a prompt asks for one.

The paintings provide distant atmosphere only. They may establish deep canopy, open clearing,
collapsed skyline, flooded concrete or pale coast, but they do not decide what is physically
present at a particular pace. The live terrain bands still choose the painting, and the
renderer still places the feature layer, ground plane and gameplay landmarks over it. Cover
cropping may discard either horizontal edge, so no unique or load-bearing subject belongs near
an edge and no painting may depend on its whole width being visible.

This boundary is especially strict around water and the journey ahead. Drowned-street water,
its surface motion and contact effects remain renderer-owned. So do the unresolved-fork mist,
the finite journey edge and arrival light, branch tracks and their selected state, enclave
approach and berth lighting, waypoint availability, labels and every other signal derived from
the snapshot. A painting can support those signals with atmosphere; it must not contain a
second baked fork, edge, settlement, path choice or interactive salvage site that can disagree
with them.

### The terrain-doodad atlas carries identity; the snapshot carries value

The replacement terrain-doodad sheet is a **4 by 4 atlas**. Its first half contains ordinary
terrain props; its second half contains four adjacent intact/stripped pairs for the salvageable
ruin families. The manifest owns the exact row-major cell names and order. Keep each prop
isolated, unlit and on the common baseline so the renderer can place, scale, tint and haze it
in any of the three live parallax layers.

Intact and stripped are semantic variants, not cosmetic randomisation. `FeatureView.salvage`
selects the member of the pair: a salvageable feature with value remaining uses its intact
cell, and the same feature at zero uses its stripped cell. The feature's terrain definition
still decides whether it is a ruin at all; zero does not turn an ordinary prop into a stripped
ruin. Seeded variation may choose a ruin family, but it must not override the intact/stripped
choice.

Do not paint a fixed salvage heap, glint, number, glow or berth marker into either cell. The
renderer draws the live quantity and interaction cues over the selected shell, which is how a
rich ruin, a lean ruin and the same place after extraction remain visibly different. The
painted cell owns material and silhouette; `FeatureView.salvage` and the procedural overlay own
the economic fact.

---

## 2. Running a batch

Compose each prompt by concatenating the fields the manifest's `compose` array names, in that
order, separated by blank lines. Substitute `{...}` placeholders from `subject_canon`.

**There are exactly two style blocks and there must never be a third.** `style` is for
illustrations seen at full screen; `style_sprite` is for anything cut out and drawn small. An
asset picks one with `style_use`, and textures use neither.

The second exists because one clause of the first is actively wrong for a sprite: "let
everything else dissolve into wash" leaves a figure with no silhouette, and silhouette is the
entire readability budget of something drawn at 40 pixels. The chiaroscuro clause has to go
for the same reason — a sprite lit from low-left on the sheet fights whatever light the scene
later puts it in.

**Never reword either block between assets.** The single biggest reason a generated set looks
like five different artists is somebody tidying the style wording each time. Storing them once
and composing is what makes that impossible rather than merely discouraged.

**Generate at `generate`, downscale to `ship`, never upscale.** Downscaling hides the softness
AI images have at 100% and tightens the watercolour edges. Two assets ship larger than they
generate and say so in their own notes; everything else downscales.

**Do sheets, and do not split a coherent set into individual objects.** Where the manifest gives
a `sheet`, generate its cells together and cut them apart. One painting pass matches itself far
better than eight separate ones. The 24-room set is the documented exception: three eight-cell
source sheets keep the prompt legible to the generator, then pack into one shipping atlas. All
three use the same references, style block and session.

**Write a sidecar.** Beside every original, a `<name>.json` with the model, the full composed
prompt, the seed and the date. You will want a matching asset in six months and will not
remember which model or which wording produced it.

---

## 3. Sizes, formats, and where files go

Per-asset sizes and paths are in the manifest. The rules behind them:

| Use | Format | Why |
|---|---|---|
| Backgrounds, title, itch page | **WebP**, quality 82–88 | Half the bytes of PNG at the same look |
| Anything with transparency | **PNG** | Simpler to debug than WebP alpha |
| Paper grain, vignette | **PNG** greyscale | Gets multiplied over colour |
| Atlas sheets | **PNG**, one sheet | One texture bind beats twenty |

**Transparency:** if your generator can do a transparent background, ask for it. If not,
generate on **flat magenta `#FF00FF`** and key it out. Do *not* generate on white —
watercolour edges are semi-transparent and white fringing will follow you forever.

**Where files go:** `web/public/art/`. Vite serves that folder as-is, so
`web/public/art/title.webp` is reachable at `/art/title.webp` with no import and no config.
Keep the full-size originals *outside* `web/public/` so you can re-derive a different size
later without regenerating.

**Budget: 40 MiB hard ceiling; stay lean by default.** An itch.io HTML5 game downloads in full
before it starts, so the higher ceiling is headroom for art that proves its value in live
comparison, not a target. Keep opaque paintings as quality-tuned WebP, alpha sheets indexed
and optimized, and full-resolution sources outside `web/public/`. `make release` prints the
size; only ship a larger derivative when the smaller one visibly loses material readability.

**Screenshots for the itch page are free** — `cd web && npx playwright test capture` writes
real stills to `web/capture/`. Use those rather than generating fake ones.

---

## 4. If this is your first time

**Consistency beats quality.** Twenty individually-lovely images in twenty slightly different
styles look worse than twenty mediocre ones that match. To get it: never edit a style block,
generate a set in one sitting (models drift between versions, sometimes between days), do
sheets, and fix a reference early — generate your favourite, then attach it to later prompts
with "match the style, palette and linework of the attached image".

**On the prompts themselves:**

- **Be concrete about the subject, vague about the art.** "Six legs in an alternating tripod
  gait, three always carrying the hull" is useful. "Beautiful, highly detailed, 8k,
  masterpiece" is noise, and
  actively pushes toward the glossy digital-painting look you are trying to avoid.
- **Pick one light per image and say which.** The illustration block offers firelight *or*
  overcast. Asking for both gets you neither.
- **Ask for less detail, not more.** Reinforce "let the far trees dissolve into flat wash".
- **Do not name the creatures.** A generator given "Thicket Mother" draws a monster. The
  manifest describes each as an animal instead, on purpose.
- The negatives are already in the style blocks — leave them in. Models put text and
  signatures into illustrations constantly.

---

## 5. Before you commission tier 3

`DECISIONS.md` §10 chose a procedural renderer deliberately, and the cross-section's
readability comes from flat colour and silhouette. Replacing it with painted sprites is not a
reskin — it is a different renderer, and it would fight a documented decision.

If you want the watercolour look *inside* the tower, **tier 1 gets you most of the way**: a
paper grain and a wash multiplied over the existing shapes reads as painted and costs one
quad. Try that before commissioning a full sprite set.

Room art is **interior dressing, not replacement room state**. The atlas provides the machinery
and furniture that makes a mill unlike a ropery and a Root Ward unlike a Thorn Gun. The renderer
still owns active motion and warmth, idle dimming, backed-up stock, buffer fills, damage, the
destroyed silhouette, hammocks, hearth fire, smoke and exterior crowns. That split is what lets
the rooms stop looking like labelled blocks without turning a live factory into 24 static cards.

The walker frame follows the same rule more strictly: it is **a component kit, never a fixed
walker silhouette**. Its twelve cells are ordered and load-bearing: `roof-lip`, `flank-wall`,
`deck-plate`, `hull-rib`, `underside-beam`, `stair-frame`, `leg-strut`, `leg-joint`, `leg-foot`,
`planter-box`, `plating-strip`, `moss-vine-ledge`. The renderer repeats those components to the
current number of slots and floors and composes the strut, joint and foot into the authoritative
six-leg gait. A whole painted tower would only match one tower height and one instant of that
gait, which makes it the wrong asset however attractive the still image is.

The art owns material — perforated rack steel, patched computer casings, brass linkages, cable,
verdigris and the human repairs over them. The renderer still owns structure and state: hull
dimensions, six leg positions and rotations, planted feet, panel damage and breaches, missing
stair treads, plating amount, planter and overgrowth placement, room and deck light, water
contact and every other live effect. In particular, `leg-strut` is generated horizontal so the
renderer can rotate it; `stair-frame` has no treads so damage can take the procedural ones out;
and none of the twelve cells may bake in a glow, break, contact splash or fixed pose.

The roof garden has its own optional eight-cell `roof-flora` kit rather than asking the tiny
frame-atlas planter cell to carry the whole silhouette. Six upright grasses, ferns, herbs and a
sapling repeat behind the roof lip; two trailing vine and sedge cells spill over it. The selection
is derived from hull dimensions and slot number, so it remains stable while the walker moves and
works at every live width. The frame still owns the metal planter boxes and roof lip. If the flora
sheet is absent, the procedural planter tufts return independently.

Stairs and elevators use their own six-cell shaft kit rather than stretching the frame's old
generic `stair-frame` through every transport type. One-floor stair and elevator bays repeat up
the live span; a separate tread, lift cage, overhead drive and counterweight give each mechanism
a distinct silhouette. The renderer still chooses which damaged treads are missing, masks a
severed gap, moves the car, opens its doors at a stop, and draws queue glow, load, freight and
crew above the paintings. A missing shaft atlas therefore falls back independently without
disabling room or walker-frame art.

The item icons are also worth doing on their own because platform emoji render differently on
every machine. Crew and creature sheets supply the small readable silhouettes while procedural
poses and state overlays remain authoritative.

`docs/RENDERER.md` lists what the renderer already draws and animates by itself — smoke,
water, legs planting, loads coloured by what they are — which is more than most people expect,
and worth reading before deciding a picture is needed. Note that its "the sails fill and
slacken" section is stale in the same way this file was: the sails are gone.
