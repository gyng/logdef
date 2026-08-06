# The playtest, and what it has to capture

M5's four open exit criteria all need the same thing: **somebody playing**. This is what to
do and what to write down, so that the session is not wasted and does not have to be repeated.

It is about two and a half hours, and it closes all four.

> **Re-derived 2026-08-06, after M6 cut the shift rota (`SYSTEMS.md` §6.32).** Every figure
> below was re-measured against the shipped pack on that date, and **most of them had moved a
> long way** — one by a factor of four and one to zero. The stale set is kept beside the new
> one wherever the change is the interesting part, because a playtester who knows the game
> used to starve its crew will read a fed one differently. If a number here surprises you,
> re-run the instrument before believing either of us.
>
> **And the crew figures are now ranges over eight seeds, not one.** They were published
> from a single seed on 2026-08-06 and that seed turned out to be the *worst* of eight —
> right to within a percentage point, and still not the figure. `needs.rs` sweeps and
> prints ranges now.

---

## Before you start

```bash
make release          # cuts understory-web.zip; on Windows use Compress-Archive -Path web/dist/*
```

Play the **built bundle**, not `make dev`. Three of the four criteria are about what a real
player meets, and the dev server is not that.

Do not read `SYSTEMS.md` first. Two of the criteria are about whether the game explains
itself, and you cannot un-know the answer.

---

## What each criterion needs

### 1. A full run to the Refugia in 2–4 hours, played rather than scripted

**Measured already:** a scripted walker reaches the Refugia in **31–36 minutes at 1×** on 12
of 12 seeds (`make instruments`, the `journey` section). That is the *floor* — a walker never
stops, berths, reads a board or hesitates.

> **This line said 129–147 minutes until 2026-08-06 and had said it for a long time.** The
> journey layer was rescaled twice (§6.19: 37–44 minutes, then 31–36) and this document was
> not. Anybody planning a session off it would have set aside an afternoon for a game whose
> floor is half an hour. It is the largest single thing that was wrong here, and it is worth
> knowing that the instrument was right the whole time — `journey.rs` prints "0.5–0.6 hours"
> and names the 2–4 hour criterion in the same breath.

**What playing adds:** whether the real number lands inside 2–4 hours, and whether those
hours are worth spending. Write down the wall-clock time and whether you were bored, and
where.

**Be aware of the size of the gap before you start.** The criterion wants 2–4 hours and the
floor is 31–36 minutes, so playing has to add between one and a half and three and a half
hours of stopping, reading and deciding. That is a *lot* to add to a 32-minute walk, and if it
does not happen the finding is not "the player was fast" — it is that the run is too short for
the criterion, which is §6.19's open question and the one this session is most likely to
settle.

### 2. The second tier changed a decision in play

Build the fiber chain at some point — a comb, a ropery — and notice whether reaching rope
felt like it opened something or like a tax. The elevator is gated behind it (`SYSTEMS.md`
§5.10). The measurable half is done; this is the half that is about wanting it.

### 3. Every constant graded `PLAYTESTED`

**All 140 rows are `MEASURED`**: an instrument confirms the effect each constant produces,
with the limit written into the row. None is `PLAYTESTED`, which this project defines as
*somebody played with it, and with neighbouring values, and this one won*.

**At the arrival screen, press "copy them".** That is the run log, JSON, every run you have
played. It is the input to the difficulty pass and the thing the criterion means by "from run
logs rather than scripted harnesses".

Three questions the harnesses cannot answer, all of them measured and none of them judged.
**All three moved when the rota was cut, and two of them reversed direction:**

- A crew member spends **3.6% of their life working two-thirds as fast** (13.9% hungry,
  **0.0% starving on every one of eight seeds**, 3.6% tired) on a tower doing everything
  right — against **35.5%**
  (33.3% / 13.4% / 22.0%) under the rota. Nobody starves at all any more. The question used
  to be "too harsh?" and is now **"too gentle?"**: needs may have stopped being a pressure.
  Judge whether you ever felt the kitchen mattered.
- Crew are **queued at a shaft 5.5% of a person's day** (8 seeds, range 4.3-6.2) on a
  four-floor tower, and **11% of ticks** on a stairs-only eight-floor one, falling to 1% once
  an elevator is up. The old figure here was 3.4%. Did contention read as a thing that was
  happening?
- A tower delivers **2.60 meals a person a day** where the design intends three, against
  2.00 before. The shortfall is structural and always will be — hunger rises while you sleep
  and eating does not — but it is now a shortfall rather than a deficit. Did hunger feel like
  a system or like a leak?

To promote a row to `PLAYTESTED`, change the value, play with it, and prefer one. Anything
less is what `MEASURED` is for.

### 4. An itch build a stranger can open and play without being told anything

Hand somebody the zip. **Say nothing.** Watch.

The failure this catches is not a bug — it is the tower being unreadable. Write down the
first thing they tried, the first thing they got wrong, and how long before they walked.

There is deliberately no tutorial. The plan never asked for one, and `DECISIONS.md` §8 wants
the cross-section to explain itself rather than a panel to explain the cross-section. If a
stranger cannot get in, that is a finding about the *diegetic* layer, and the fix belongs
there rather than in a help screen.

---

## Two things worth knowing before you trust anything

**This project's instruments have lied more often than its game has.** One session found
three of them dead or lying — one hiding that M2's defence comparison had never once run —
plus a lighting figure 70% out, a meals figure a third out, and four build costs naming the
wrong currency. `AGENTS.md` carries the three rules that came out of it. If a number here
surprises you, suspect the instrument first.

**And the berth is a trap by construction.** A salvage rig is 8 poles and affordable in the
first minutes; the dart battery that makes using one survivable costs 6 poles **and 2 rope**,
which needs a comb and a ropery. Nothing on screen says the rig is half a purchase. Watch
whether you fall into it, because a stranger will.

The numbers under that have widened since it was written. `journey.rs` now reports berthing
recklessly paying on **7 seeds of 12** and carefully on **5**, with the spread running
−97.5% to +186.1% — so it is not "berthing is bad, berthing with a battery is good" but
"both answers occur and the tower decides which". The instrument says it in as many words.
The trap is unchanged; the verdict is less tidy than this paragraph used to make it.

**One more thing to watch, added by the same change.** Crew now sleep when they are tired
rather than on a rota, so they drift apart and the tower is never wholly asleep. That is what
the change was for. What it costs is that "everybody is in bed, come back later" is no longer
a readable state — see whether a mostly-asleep tower reads as resting or as broken.
