# Art manifest

Everything needed to generate the game's art: what to make, what to paste, what size to ship
it at, and where the file goes.

**If you do nothing else, do §3.** It is fourteen prompts you can copy straight into an image
generator, in the order worth making them.

---

## 1. What you are drawing

*Understory* is a walking garden-tower striding through a jungle that swallowed the old world.
You watch it from the side, in cross-section, like a doll's house — the rooms are open to you,
the crew are named people, and the whole thing walks on two heavy backward-jointed legs. It is
solarpunk and melancholy: nothing here is a war machine, the creatures are animals defending
their territory, and a run ends with an arrival rather than a victory.

Three things to keep true in every image:

- **The tower has two legs, not six.** Backward-bending knees, like a bird's. It is the
  silhouette the whole game reads by.
- **Nothing is triumphant.** No heroic poses, no explosions, no banners.
- **The jungle is winning, gently.** Everything is a little overgrown.

### The game has no art at all today

Every pixel is currently drawn procedurally — coloured shapes, no images anywhere. So this is
a manifest for *new* art, and **generating a picture is the cheap half**; the expensive half is
wiring it in. That is why §3 is ordered the way it is: the first fourteen assets need almost no
code, and everything after them needs progressively more.

| Tier | Where it goes | Code needed |
|---|---|---|
| **0** | itch.io page, title screen, arrival card | none — a CSS background |
| **1** | Paper grain, wash, sky strips, over the game itself | one textured quad |
| **2** | Scenery props — trees, ferns, ruins | a texture atlas |
| **3** | Room and creature sprites | a different renderer — **read §6 first** |

---

## 2. The style block

**Paste this at the top of every prompt, unchanged.** Never reword it between assets. The
single biggest reason a generated set looks like five different artists is somebody tidying the
style wording each time.

```
Melancholic 1980s anime watercolour magazine illustration. Watercolour wash with visible
paper grain, soft feathered edges, muted desaturated palette, subtle ink linework kept thin
and broken. Low-key chiaroscuro firelight OR soft overcast daylight — one or the other, never
both. Simplified background, selective detail: render one focal area crisply and let
everything else dissolve into wash. Solarpunk, overgrown, quiet. No text, no logos, no
watermark, no signature, no UI, no border or frame.
```

Add the palette when colour matters — backgrounds especially. These are the colours the game
actually draws with:

```
Palette anchors: deep teal sky #2b6f74, pale sage haze #b7cbb0, dark canopy green #12301f,
bark brown #3a2f26, warm brass #a8834d, lamp gold #e0c07a, bone #ecdfba.
```

---

## 3. The prompts

Each block is complete — **copy the whole thing**, style block included. Where a block says
*(style block)*, paste §2 there verbatim.

Generate at your tool's largest size (usually 1024×1024, 1536×1024 landscape, or 1024×1536
portrait), then downscale to the ship size in the heading. Never upscale.

---

### 3.1 · `itch-cover` — ship 630×500 WebP · **required by itch.io**

> *(style block)*
>
> Subject: an enormous walking tower of bamboo and salvaged timber, seen from the side and
> slightly below, striding through dense jungle. It walks on **two** heavy legs with
> backward-bending knees like a bird's — one foot planted, one lifting. Its flank is open in
> cross-section: small lit rooms stacked four high with tiny figures inside. Sail panels on the
> roof. Soft overcast light, mist between the trunks, the canopy closing over behind it.
>
> Composition: tower left of centre and small against a huge horizon. Leave the top third
> quiet — a title will sit there. Landscape 5:4.

### 3.2 · `itch-banner` — ship 1920×620 WebP

> *(style block)*
>
> Subject: the same walking tower, far right of frame and walking away from the viewer, small.
> The rest is jungle canopy receding into pale haze, layer behind layer. Overcast, early
> morning, mist in the middle distance.
>
> Composition: wide letterbox, nothing at all in the centre — text will sit there. 3:1.

### 3.3 · `title` — ship 2560×1440 WebP

> *(style block)*
>
> Subject: **interior**. One lamplit deck inside the walking tower at night, seen from the side
> in cross-section. Two crew sitting at a low table, small in frame, sharing a meal. Bamboo
> racks, hanging tools, a cooking pot. Warm firelight from a single lamp; everything beyond its
> reach dissolves into dark wash. Through a gap in the timber, black jungle.
>
> Composition: lamp low-left, the dark two-thirds of the frame doing the work. 16:9.

### 3.4 · `arrival` — ship 2048×1152 WebP · behind the arrival card

> *(style block)*
>
> Subject: dawn at the coast. The walking tower stopped on a headland above pale flat water,
> legs folded under it, still. Salt grass. First light on its flank. Nothing threatening,
> nothing ruined — the end of a very long walk.
>
> Composition: tower right of centre, low horizon, most of the frame given to sky. 16:9.

### 3.5 · `elegy` — ship 2048×1152 WebP · behind the loss card

> *(style block)*
>
> Subject: the same walking tower, dark and motionless, being taken back by the jungle. Vines
> through its open flank, saplings on its roof, one leg buckled. Flat overcast light. No
> wreckage, no fire, no drama — just green closing over something that stopped.
>
> Composition: tower centred and small, canopy crowding in from every edge. 16:9.

### 3.6 · `favicon` — ship 512×512 PNG

> *(style block)*
>
> Subject: a single bamboo leaf crossed with a small lit lamp. Flat, simple, high contrast,
> almost a woodcut. Very few shapes.
>
> Composition: centred, square, generous margin. **Must read at 16 pixels.**

### 3.7 · `og-image` — ship 1200×630 WebP · link previews

Crop 3.1 or 3.3 to a wide letterbox. No new generation needed.

---

### 3.8 · `paper-grain` — ship 1024×1024 PNG greyscale · **tileable**

This single asset does most of the work of making the game look painted.

> Seamless tileable cold-press watercolour paper texture. Greyscale only, no subject matter, no
> objects, no composition. Even fine tooth across the whole square, no dark blotches, no
> visible seam at any edge, no vignette, no lighting. A flat scan of blank paper.

### 3.9 · `wash-overlay` — ship 2048×1152 WebP

> Abstract watercolour wash, no subject matter and no recognisable objects. Muted sage green,
> deep teal and pale bone blooming into one another with hard wet edges where the colours meet.
> Large soft shapes only, no detail, no linework. Like the inside of a paint-water jar settling.

### 3.10 · `vignette` — ship 1024×576 PNG with alpha

> Soft radial vignette. Pure black at the four corners fading to fully transparent by 45% of the
> radius from centre. Perfectly smooth, no banding, no colour, no subject matter.

### 3.11–3.14 · sky strips — ship 512×512 WebP each

Four vertical gradients. **No landscape, no hard-edged clouds, no horizon line** — sky only.

- **`sky-dawn`** — *(style block)* Vertical watercolour sky gradient only. Cold deep teal at the
  top, warming through pale sage to bone at the bottom. Soft wet-into-wet blending, paper grain
  visible. No clouds, no landscape, no horizon.
- **`sky-day`** — *(style block)* Vertical watercolour sky gradient only. Flat pale overcast:
  soft grey-sage at the top, near-white at the bottom. Very low contrast. No clouds, no
  landscape, no horizon.
- **`sky-dusk`** — *(style block)* Vertical watercolour sky gradient only. Deep teal at the top
  through warm brass to dusty rose at the bottom. No clouds, no landscape, no horizon.
- **`sky-night`** — *(style block)* Vertical watercolour sky gradient only. Near-black blue-teal
  at the top, a very faint pale wash at the bottom. A few stars as tiny points of *unpainted
  paper*, not white dots. No moon, no clouds, no landscape.

---

### Later tiers

**Tier 2 — scenery props.** Eight kinds: `tree` `fern` `rock` `vine` `ruin` `wreck`
`driftwood` `salt-pan`. All side-on, isolated on transparency, base at the bottom edge of the
image. **Generate all eight in one image as a labelled row and cut them apart** — one painting
pass matches itself far better than eight separate ones. Pack into a 2048×2048 atlas.

**Tier 3 — rooms and creatures.** Read §6 first; this is where the cost jumps. The one part
worth doing on its own is the **twelve item icons** — 128×128 PNG with alpha, small painted
objects on transparency — because those are emoji today and emoji render differently on every
machine, which is a real bug rather than a preference. Items: `alloy bamboo charge_cells darts
fiber meals mechanisms poles produce rope scrap seed_bombs`.

---

## 4. Sizes, formats, and where files go

**Generate large, ship small. Never upscale.** Downscaling hides the softness AI images have at
100% and tightens the watercolour edges.

| Use | Format | Why |
|---|---|---|
| Backgrounds, title, itch page | **WebP**, quality 82–88 | Half the bytes of PNG at the same look |
| Anything with transparency | **PNG** | Simpler to debug than WebP alpha |
| Paper grain, vignette | **PNG** greyscale | Gets multiplied over colour |
| Atlas sheets | **PNG**, one sheet | One texture bind beats twenty |

**Transparency:** if your generator can do a transparent background, ask for it. If not,
generate on **flat magenta `#FF00FF`** and key it out. Do *not* generate on white — watercolour
edges are semi-transparent and white fringing will follow you forever.

**Where files go:** `web/public/art/`. Vite serves that folder as-is, so
`web/public/art/title.webp` is reachable at `/art/title.webp` with no import and no config.
Name files after the content id they belong to: `room.mill` → `art/rooms/mill.png`.

**Budget: 3–5 MB of art, total.** The whole game is ~530 KB today, and an itch.io HTML5 game
downloads *in full* before it starts. Past 5 MB the first thing a stranger experiences is a
wait. `make release` prints the size.

**Screenshots for the itch page are free** — `cd web && npx playwright test capture` writes real
stills to `web/capture/`. Use those rather than generating fake ones.

---

## 5. If this is your first time

**Consistency beats quality.** Twenty individually-lovely images in twenty slightly different
styles look worse than twenty mediocre ones that match. To get it:

1. **Never edit the style block.** Copy-paste it every time.
2. **Generate a set in one sitting.** Models drift between versions, sometimes between days.
3. **Do sheets.** Several related things in one image, then cut them apart — painted by one
   pass, so they match.
4. **Fix a reference early.** Generate your favourite, then attach it to later prompts with
   "match the style, palette and linework of the attached image".

**On the prompts:**

- **Be concrete about the subject, vague about the art.** "Two backward-bending legs mid-stride"
  is useful. "Beautiful, highly detailed, 8k, masterpiece" is noise, and actively pushes toward
  the glossy digital-painting look you are trying to avoid.
- **Pick one light per image and say which.** The style block offers firelight *or* overcast.
  Asking for both gets you neither.
- **Ask for less detail, not more.** Reinforce "let the far trees dissolve into flat wash".
- The negatives are already in the style block — leave them in. Models put text and signatures
  into illustrations constantly.

**Practical:**

- **Downscale in an image editor, not in CSS.** A 2048px image squeezed into a 400px slot by the
  browser looks softer *and* costs the full download.
- **Keep the originals outside `web/public/`** so you can re-derive a different size later
  without regenerating.
- **Write down what you used** — model, prompt, date — beside the art. You will want a matching
  asset in six months and will not remember.

**One project-specific rule.** `DECISIONS.md` §8 is a tone guardrail: defenders not soldiers,
creatures defending territory rather than a gallery to clear. If you generate creature art, a
root-borer is an animal doing an animal thing, not a monster. And nothing in this game gets a
score, a rank, or a victory pose.

---

## 6. Before you commission tier 3

`DECISIONS.md` §10 chose a procedural renderer deliberately, and the cross-section's
readability comes from flat colour and silhouette. Replacing it with painted sprites is not a
reskin — it is a different renderer, and it would fight a documented decision.

If you want the watercolour look *inside* the tower, **tier 1 gets you most of the way**: a
paper grain and a wash multiplied over the existing shapes reads as painted and costs one quad.
Try that before commissioning twenty room sprites.

`docs/RENDERER.md` lists what the renderer already draws and animates by itself — smoke, water,
sails filling, legs planting, loads coloured by what they are — which is more than most people
expect, and worth reading before deciding a picture is needed.
