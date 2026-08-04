# Understory

A walking garden-tower striding through a solarpunk jungle: SimTower × Factorio × tower
defence. Build the supply chain that keeps it fed and lit, defend it in real-time, keep
walking. Rust/WASM deterministic core + React/TypeScript frontend.

v1 ("Supply Line," an ARPG-flavoured hero/weapons game) is archived on `main`. This is `v2`.

## Quick start

- **Whole-game plan:** `docs/v2-plan.md` — the locked plan; the only whole-game doc; sprint
  briefs and milestone schedule live here and nowhere else
- **Design:** `docs/DESIGN.md` — the pitch, the parents, the pillars, the systems, distilled
- **Built spec:** `docs/SYSTEMS.md` — grown one milestone at a time; spec before code
- **Engineering rules:** `docs/DECISIONS.md` — determinism, bridge, command pattern, tone
  guardrails; numbered so code can cite them (`DECISIONS.md §1`)
- **Balance values:** `docs/BALANCE.md` — every tuning constant, graded by provenance
- **The remaining work:** `docs/PLAYTEST.md` — M5's four open criteria all need a person at
  the keyboard; this is what to do and what to write down
- **Art:** `docs/ART.md` — the handoff: fourteen copy-pasteable prompts, sizes, formats and
  the style block. The game currently ships zero images
- **Renderer:** `docs/RENDERER.md` — what is drawn procedurally, what was tried and reverted,
  and the measured frame budget
- **Agent guidelines:** `AGENTS.md` — read this before writing any code

## Canonical override chain

`DECISIONS.md` > `SYSTEMS.md` > `v2-plan.md` > `DESIGN.md`

@AGENTS.md
