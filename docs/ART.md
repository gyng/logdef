# Art manifest

Everything needed to generate the game's art.

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

### The canon, and why it is in the manifest rather than here

`subject_canon` in the manifest holds the descriptions that must not vary between assets —
the tower, its roof, its hull, the crew, the creatures. Prompts reference them as `{tower}`,
`{crew}` and so on, and a test fails if a prompt uses a name the canon does not define.

They are there because **an error in the canon is an error in every asset at once**, and this
file got two of them wrong for a whole milestone:

- **The tower has four legs, not two.** This document used to say "two legs, not six … it is
  the silhouette the whole game reads by", which is the opposite of what the game draws.
  `scene.ts` has `const LEGS = 4`, and `RENDERER.md`'s section is titled "Four legs, and the
  joint that would not go where it was told". The reason matters for the art: two legs "reads
  as a *person* however it is drawn", so the change was made to stop it doing that.
- **The joint is not a knee.** It is a shallow inverted V riding *above* the hip — a wading
  insect, not a bird. RENDERER.md spent three attempts on this and says a knee "is the one
  thing this must not look like". The old prompt asked for "backward-bending knees like a
  bird's", which is the failure mode named in the renderer's own notes.
- **There are no sails.** M6 cut the canopy sails out of the game entirely (`SYSTEMS.md`
  §6.10) and the roof carries a row of planters now — a garden. Every mention of a sail in the
  code is a comment about their removal. The old cover prompt asked for "sail panels on the
  roof", which would have put a deleted mechanic on the itch.io page.

### The character direction

**Semi-chibi anime sprites, inside the same 1980s watercolour magazine style.** Roughly three
and a half heads tall — softened and slightly large-headed so a figure forty pixels tall is
still a person, but not full chibi and not a mascot.

The thing to hold onto is that **semi-chibi is doing a readability job, not a comedy one.**
This is a melancholy game. The crew are adults at work: calm, tired, absorbed in a task. No
gag expressions, no sweat-drops, no wide grins, no heroic poses. `DECISIONS.md` §8 is a tone
guardrail — defenders rather than soldiers — and it binds the art as much as the code. A root
borer is a beetle doing a beetle thing, not a monster.

`crew_tone` and `creature` in the canon say this in prompt form; paste them, don't paraphrase.

### The game has no art at all today

Every pixel is drawn procedurally — coloured shapes, no images anywhere, and `web/public/` is
empty. So this is a manifest for *new* art, and **generating a picture is the cheap half**;
the expensive half is wiring it in. The manifest is ordered so the early assets need almost no
code and the later ones need progressively more.

| Tier | Where it goes | Code needed |
|---|---|---|
| **0** | itch.io page, title screen, arrival card | none — a CSS background |
| **1** | Paper grain, wash, sky strips, over the game itself | one textured quad |
| **2** | Scenery props — trees, ferns, ruins | a texture atlas |
| **3** | Crew, creatures, item icons | a different renderer — **read §5 first** |

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

**Do sheets, and do not split them.** Where the manifest gives a `sheet`, generate all its
cells in one image and cut them apart. One painting pass matches itself far better than eight
separate ones, and the crew sheet is the clearest case — eight separately-generated characters
will not look like the same crew.

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

**Budget: 3–5 MB of art, total.** The shipped bundle is ~1.9 MB today, 1.6 MB of which is the
wasm, and an itch.io HTML5 game downloads *in full* before it starts. Past 5 MB the first
thing a stranger experiences is a wait. `make release` prints the size. If the set does not
fit, cut assets rather than quality.

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

- **Be concrete about the subject, vague about the art.** "Four legs in a wave gait, two
  always planted" is useful. "Beautiful, highly detailed, 8k, masterpiece" is noise, and
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

The exception worth doing on its own is the **item icons**, because those are emoji today and
emoji render differently on every machine — a real bug rather than a preference. The crew
sheet is the next most valuable, since it is where the semi-chibi direction actually becomes
visible; but it is also the one that needs the most renderer work behind it.

`docs/RENDERER.md` lists what the renderer already draws and animates by itself — smoke,
water, legs planting, loads coloured by what they are — which is more than most people expect,
and worth reading before deciding a picture is needed. Note that its "the sails fill and
slacken" section is stale in the same way this file was: the sails are gone.
