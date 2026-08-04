# The playtest, and what it has to capture

M5's four open exit criteria all need the same thing: **somebody playing**. This is what to
do and what to write down, so that the session is not wasted and does not have to be repeated.

It is about two and a half hours, and it closes all four.

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

**Measured already:** a scripted walker reaches the Refugia in **129–147 minutes at 1×** on
12 of 12 seeds (`make instruments`, the `journey` section). That is the *floor* — a walker
never stops, berths, reads a board or hesitates.

**What playing adds:** whether the real number lands inside 2–4 hours, and whether those
hours are worth spending. Write down the wall-clock time and whether you were bored, and
where.

### 2. The second tier changed a decision in play

Build the fiber chain at some point — a comb, a ropery — and notice whether reaching rope
felt like it opened something or like a tax. The elevator is gated behind it (`SYSTEMS.md`
§5.10). The measurable half is done; this is the half that is about wanting it.

### 3. Every constant graded `PLAYTESTED`

**All 133 rows are `MEASURED`**: an instrument confirms the effect each constant produces,
with the limit written into the row. None is `PLAYTESTED`, which this project defines as
*somebody played with it, and with neighbouring values, and this one won*.

**At the arrival screen, press "copy them".** That is the run log, JSON, every run you have
played. It is the input to the difficulty pass and the thing the criterion means by "from run
logs rather than scripted harnesses".

Three questions the harnesses cannot answer, all of them measured and none of them judged:

- A crew member spends **35.5% of their life working two-thirds as fast** (33.3% hungry,
  13.4% starving, 22.0% tired) on a tower doing everything right. Too harsh?
- Crew are **queued at a shaft 3.4% of the day** on a starting tower. Did contention read as
  a thing that was happening, or as nothing?
- A tower delivers **2.00 meals a person a day** where the design intends three, because
  crew cannot eat while asleep. Did hunger feel like a system or like a leak?

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
which needs a comb and a ropery. Berthing without the battery loses 29–94% against never
stopping; with it, it wins 0.5–66%. Nothing on screen says the rig is half a purchase. Watch
whether you fall into it, because a stranger will.
