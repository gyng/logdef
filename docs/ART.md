# Art manifest

What to generate, at what size, in what format, for what — and how to prompt for it so the
set looks like one artist made it.

---

## 0. Read this first: the game has no art at all today

The tower, the terrain, the creatures and the crew are **drawn procedurally** — coloured
quads through `web/src/engine/QuadBatch.ts`, tinted from `palette.ts`. There is not one
image file in the project. Items are emoji.

That matters more than it sounds, because **generating a picture is the cheap half.** The
expensive half is wiring it in, and the cost is wildly different depending on where it lands:

| Tier | Where it goes | Code needed | Do it? |
|---|---|---|---|
| **0** | itch.io page, title screen, arrival card, UI icons | None, or a CSS `background-image` / `<img>` | **Yes, start here** |
| **1** | Full-screen overlays: paper grain, watercolour wash, vignette | One textured quad + a sampler in the shader | **Yes, best value per hour** |
| **2** | Parallax scenery props — trees, ferns, ruins, wrecks | A texture atlas, UV coords in `QuadBatch`, atlas loading | Probably |
| **3** | Room / creature / crew sprites in the cross-section | Atlas + per-room art + animation states + a lot of layout rework | **Probably not — see below** |

**On tier 3.** `DECISIONS.md` §10 chose a custom procedural renderer deliberately, and the
cross-section's readability comes from flat colour and silhouette. Replacing it with painted
sprites is not a reskin; it is a different renderer, and it would fight a documented decision.
If you want the watercolour look *in* the tower, tier 1 gets you most of the way — a paper
grain and a wash over procedural shapes reads as painted, and it costs one quad.

So: **generate tiers 0 and 1 first.** They are 12 images, they need almost no code, and they
change the whole feel of the thing.

---

## 1. The style block

Paste this **verbatim** into every prompt. Never reword it between assets — the single
biggest reason a generated set looks incoherent is the artist tweaking the style wording
each time.

```
Melancholic 1980s anime watercolour magazine illustration. Watercolour wash with visible
paper grain, soft feathered edges, muted desaturated palette, subtle ink linework kept thin
and broken. Low-key chiaroscuro firelight OR soft overcast daylight — one or the other, never
both. Simplified background, selective detail: render one focal area crisply and let
everything else dissolve into wash. Solarpunk, overgrown, quiet. No text, no logos, no
watermark, no signature, no UI, no border or frame.
```

Then add a **subject block** — one or two sentences, concrete, present tense — and a
**format block** (size, background). Full example:

```
Melancholic 1980s anime watercolour magazine illustration. Watercolour wash with visible
paper grain, soft feathered edges, muted desaturated palette, subtle ink linework kept thin
and broken. Low-key chiaroscuro firelight OR soft overcast daylight — one or the other, never
both. Simplified background, selective detail: render one focal area crisply and let
everything else dissolve into wash. Solarpunk, overgrown, quiet. No text, no logos, no
watermark, no signature, no UI, no border or frame.

Subject: a vast walking tower of bamboo and salvaged timber striding through dense jungle
canopy, seen from the side at a distance. Two heavy chicken-jointed legs mid-stride — knee bending backwards, one foot planted and one lifting. Lamps lit in a few
windows. Soft overcast light, mist between the trunks.

Landscape 3:2. Fill the frame edge to edge.
```

### The palette, if you want the art to sit with the renderer

Paste these into the prompt when it matters (backgrounds especially):

```
Palette anchors: deep teal sky #2b6f74, pale sage haze #b7cbb0, dark canopy green #12301f,
bark brown #3a2f26, warm brass #a8834d, lamp gold #e0c07a, bone #ecdfba.
```

---

## 2. Sizes and formats, generally

**Generate large, ship small.** Always. Downscaling hides the soft mush AI images have at
100%, and it tightens the watercolour edges. Never upscale.

- Generate at your tool's largest supported size — usually **1024×1024**, **1536×1024**
  (landscape) or **1024×1536** (portrait).
- Ship at **2× the display size** for anything on a screen (retina), so a 200 px icon ships
  at 400 px.

| Use | Format | Why |
|---|---|---|
| Big backgrounds, title art, itch page | **WebP**, quality 82–88 | Half the bytes of PNG at the same look; the whole build is currently ~530 KB, so a 2 MB PNG background would quadruple it |
| Anything with transparency (props, icons) | **PNG-8 or PNG-24** | Alpha; WebP also supports alpha and is smaller, but PNG is simpler to debug |
| Paper grain / wash overlays | **PNG** greyscale, or WebP | Tiled or stretched; needs to survive being multiplied over colour |
| Anything in a WebGL atlas | **PNG**, packed into one sheet | One texture bind beats twenty |

**Transparency:** if your generator supports a transparent background, ask for it. If it does
not, generate the subject on a **flat, saturated colour that appears nowhere in the art** —
`#FF00FF` magenta is traditional — and key it out afterwards. Do not generate on white and
try to cut it; watercolour edges are semi-transparent and white fringing will follow you
around forever.

**Where files go:** `web/public/art/` — Vite serves that directory as-is, so a file at
`web/public/art/title.webp` is reachable at `/art/title.webp` with no import and no bundler
config. Name files after the content id they belong to: `room.mill` → `art/rooms/mill.png`.

---

## 3. The manifest

### Tier 0 — no code needed

| # | Asset | Purpose | Ship size | Format | Subject block |
|---|---|---|---|---|---|
| 0.1 | `itch-cover` | itch.io game cover. **Required** by itch, shown in every listing | **630×500** exactly | WebP | The tower small against a huge overgrown horizon, seen from below-left. Two legs mid-stride, knees bending backwards. Enough negative space at the top that a title could sit there later. Soft overcast light. |
| 0.2 | `itch-banner` | Optional wide header on the game page | 1920×620 | WebP | The same tower, far right of frame, walking away. The rest is jungle canopy receding into haze. Nothing in the centre. |
| 0.3 | `title` | Title/menu screen background if you add one | 2560×1440 | WebP | Interior: a lamplit deck inside the tower at night, seen from the side. Two figures at a table, small. Warm firelight, everything beyond the lamp's reach dissolving to dark wash. |
| 0.4 | `arrival` | Behind the arrival card (`Chrome.tsx`, `.arrival`) | 2048×1152 | WebP | Dawn at the coast. The tower stopped, legs folded, on a headland above pale water. First light. Nothing threatening. The end of a long walk. |
| 0.4b | `smoke-ref` | Reference only — not shipped. Pin it beside the burner-smoke work so the procedural plume has a target | 1024×1024 | WebP | A thin column of pale woodsmoke rising from a vent on a timber tower and shearing sideways in the wind. Overcast. |
| 0.5 | `elegy` | Behind the loss card (`.elegy`) | 2048×1152 | WebP | The same tower, still and dark, being taken back by vines. Overcast. No wreckage, no drama — just green closing over. |
| 0.6 | `favicon` | Browser tab | 512×512 (browser downscales) | PNG | A single bamboo leaf and a lamp, flat and simple, high contrast, readable at 16 px. |
| 0.7 | `og-image` | Link preview when the itch page is shared | 1200×630 | WebP | Crop of 0.1 or 0.3, composed for a wide letterbox. |

**Screenshots for the itch page** are not generated — take them with the harness you already
have: `cd web && npx playwright test capture` writes stills to `web/capture/`. Use those.
They are honest and they are free.

### Tier 1 — one textured quad each, big payoff

These are the ones that make the *game* look painted rather than just its menus.

| # | Asset | Purpose | Ship size | Format | Subject block |
|---|---|---|---|---|---|
| 1.1 | `paper-grain` | Multiply over the whole frame at ~8–12% opacity. This alone is most of the style | 1024×1024, **tileable** | PNG greyscale | Seamless tileable cold-press watercolour paper texture. Greyscale, even, no dark blotches, no visible seams, no subject matter. Fine tooth. |
| 1.2 | `wash-overlay` | Screen/overlay blend at ~15%, gives the sky and haze a hand-painted bleed | 2048×1152 | WebP | Abstract watercolour wash, no subject. Muted sage, teal and bone, blooming into each other with hard wet edges where they meet. Large soft shapes only. |
| 1.3 | `vignette` | Multiply, darkens the frame edges | 1024×576 | PNG greyscale + alpha | Soft radial vignette, black at the corners fading to fully transparent by 45% of the radius. No banding. |
| 1.4 | `sky-dawn` | Sky gradient strip for the dawn dayparts | 512×512 | WebP | Vertical watercolour sky gradient only, no landscape, no clouds with hard edges. Cold teal at the top through to warm bone at the horizon. |
| 1.5 | `sky-day` | As above, midday | 512×512 | WebP | As 1.4 but brighter and flatter, overcast — pale sage through to near-white at the horizon. |
| 1.6 | `sky-dusk` | As above, dusk | 512×512 | WebP | As 1.4 but warm: deep teal above, brass and rose at the horizon. |
| 1.7 | `sky-night` | As above, night | 512×512 | WebP | Near-black teal above, very faint pale wash at the horizon. A few stars as tiny dots of unpainted paper, not white dots. |

> The sky strips replace `palette.skyHigh/skyMid/skyLow`, which is currently a three-stop
> gradient. Sampling a painted strip instead is a ~10-line change and it is the single most
> visible one available.

### Tier 2 — needs a texture atlas

Scenery props, scattered by `featurePoint()` across three parallax layers. **All of these
must be generated with transparency**, side-on, and with their base at the bottom edge of
the image so they sit on the ground line.

Eight kinds, from `terrain/*.ron`'s `feature_kinds`:

| # | Asset | Appears in | Ship size | Subject block |
|---|---|---|---|---|
| 2.1 | `tree` | canopy, clearing | 512×768 | A single tall jungle tree, side-on, trunk to canopy, isolated on transparent. Silhouette-forward. |
| 2.2 | `fern` | canopy, clearing, ruin field | 512×384 | A low tree-fern cluster, side-on, isolated on transparent. |
| 2.3 | `rock` | canopy, clearing, ruin field | 512×384 | A mossy boulder, side-on, isolated on transparent. |
| 2.4 | `vine` | canopy, drowned street | 384×768 | A hanging curtain of vines, top-anchored, isolated on transparent. |
| 2.5 | `ruin` | ruin field, drowned street | 768×768 | The overgrown concrete corner of a collapsed building, one storey, vines through it. Isolated on transparent. |
| 2.6 | `wreck` | drowned street | 768×512 | A half-sunk vehicle hull, unidentifiable make, rusted and overgrown. Isolated on transparent. |
| 2.7 | `driftwood` | salt flat | 512×384 | Bleached driftwood tangle on pale ground, isolated on transparent. |
| 2.8 | `salt-pan` | salt flat | 768×256 | A cracked salt crust patch seen nearly edge-on, very pale, isolated on transparent. |

**Generate each three times** — the parallax layers tint them toward haze (far) and near-black
(near), and a prop that reads well as a crisp mid-ground tree turns to mush at 22% parallax.
Or generate once and let the existing tint do the work; try the cheap way first.

Pack all eight into **one 2048×2048 PNG atlas**. One texture bind, one draw call, no
per-prop loading.

### Tier 3 — probably don't, but here's the list if you do

20 rooms (`bombary bunk burner canopy_sails canteen cell_bank cellwright cutter_arm
dart_battery fiber_comb fitter garden heartseed mill ropery salvage_rig seed_thrower
storeroom sun_forge thornwright`), 7 creatures (`canopy_leaper feral_warden glean_crow
mire_hulk night_prowler root_borer skitter`), 12 items.

**The items are the one sub-tier worth doing on its own**, because they are currently emoji
(🎋 🔩 🪢 …) and emoji render differently on every machine — which is a real visual bug, not
a preference. Twelve 128×128 PNGs with transparency, drawn as small painted objects on a
plain ground, swapped into the DOM where the glyph is used today. No renderer work at all.

Items: `alloy bamboo charge_cells darts fiber meals mechanisms poles produce rope scrap
seed_bombs`.

---

## 4. Best practices, for someone doing this the first time

**Consistency is the whole game.** A set of twenty individually-lovely images in twenty
slightly different styles looks worse than twenty mediocre images in one. To get consistency:

1. **Never edit the style block.** Copy-paste it. If you must change it, regenerate
   *everything* in the set.
2. **Generate a set in one sitting.** Models drift between versions and sometimes between
   days.
3. **Do sheets where you can.** For the eight scenery props, ask for all eight in one image
   as a labelled row on a plain background, then cut them apart. They will match each other
   far better than eight separate generations, because they were painted by one pass.
4. **Fix one reference first.** Generate your favourite image, then attach it as a reference
   to every later prompt with "match the style, palette and linework of the attached".

**On prompting:**

- **Say what you don't want, once, at the end.** The style block already carries the negatives
  ("no text, no logos, no watermark"). Models put text in illustrations constantly; leave that
  in.
- **Be concrete about the subject and vague about the art.** "Six insectile legs mid-step" is
  useful. "Beautiful, highly detailed, 8k, masterpiece" is noise and often actively pushes
  toward the glossy digital-painting look you are trying to avoid.
- **Name the light once.** Your style block offers firelight *or* overcast. Pick one per asset
  and say which. Asking for both gets you neither.
- **Ask for less detail, not more.** "Simplified background, selective detail" is doing heavy
  lifting; reinforce it per-asset ("let the far trees dissolve into flat wash").

**On the files:**

- **Downscale in an image editor, not in CSS.** A 2048 px image squeezed into a 400 px slot
  by the browser looks softer *and* costs the full download.
- **Check the build size after each addition.** `make release` prints it; the whole game is
  ~530 KB today. Art will dominate that fast, and itch.io HTML5 games are downloaded in full
  before they start. Budget maybe 3–5 MB total; past that the first-load wait starts being
  the first thing a stranger experiences.
- **Keep the source generations.** Put the 1024/1536 originals somewhere outside `web/public/`
  (they should not ship) so you can re-derive a different size later without regenerating.
- **Record what you used.** Which model, which prompt, which date, in a text file beside the
  art. You will want to make a matching asset in six months and you will not remember.

**One project-specific note.** `DECISIONS.md` §8 is a tone guardrail: defenders not soldiers,
creatures defending territory rather than a gallery to clear. If you generate creature art,
that applies — a root-borer is an animal doing an animal thing, not a monster. And nothing in
this game gets a score, a rank or a victory pose.
