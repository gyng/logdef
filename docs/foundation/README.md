# Supply Line — Foundation Docs

## For Engineers: Start Here

Read these three files first. They override everything else.

1. **[implementation-decisions.md](implementation-decisions.md)** — Canonical answers to every architectural question (tick rate, bridge model, audio ownership, save model, UI rules). When any other doc conflicts, this file wins.

2. **[v1-scope.md](v1-scope.md)** — Every feature tagged `v1` or `post-v1`. Only build v1 features. If a feature appears in another doc but isn't tagged v1 here, don't build it.

3. **[registries.md](registries.md)** — Single source of truth for all content: companion names/passives, weapon sub-types/abilities, trinkets, resources, map nodes, unlock milestones. When other docs disagree on a name or stat, registries wins.

## Document Map

| Document | What it covers | Status |
|----------|---------------|--------|
| [game-design.md](game-design.md) | Full game vision — pitch, loop, all systems | Full vision (v1 + post-v1 annotated) |
| [art-direction.md](art-direction.md) | Visual style, color language, tone guardrails | Vision |
| [design-system.md](design-system.md) | UI tokens, atoms, molecules, organisms, pages | v1 spec |
| [software-architecture.md](software-architecture.md) | Rust/React stack, GameState, commands, systems, bridge | Vision (check impl-decisions) |
| [ui-ux.md](ui-ux.md) | Combat HUD, prep flow, map, merchant, inventory | v1 spec |
| [audio-direction.md](audio-direction.md) | Soundscape, music, spatial audio, Web Audio API | Vision |
| [animation.md](animation.md) | Tween/bone/particle systems, entity animation specs | Vision |
| [controls.md](controls.md) | Weapon-dependent input models, input abstraction | Vision |
| [equipment.md](equipment.md) | Weapons, modifiers, trinkets, rarity, melee | Vision |
| [class-design.md](class-design.md) | Hero classes, companion passives/synergies, relationships | Vision |
| [narrative.md](narrative.md) | World, tower spirit, companions, destinations | Version-independent |
| [procedural-generation.md](procedural-generation.md) | Map gen, encounters, loot, merchants, seeds | Vision |
| [metagame.md](metagame.md) | Unlocks, destinations, ascension, run structure | Vision |
| [telemetry-balance.md](telemetry-balance.md) | Metrics, suggestions, pacing adaptation | Vision |
| [tech-performance.md](tech-performance.md) | Frame budgets, tick rates, profiling, dev workflow | v1 spec |
| [balance-config.md](balance-config.md) | All v1 balance values: enemy stats, production rates, economy, weapons | v1 spec (placeholder values, iterate in playtesting) |

## Implementation Checklist

Before writing code for any feature:

- [ ] Check [v1-scope.md](v1-scope.md) — is this feature tagged `v1`?
- [ ] Check [implementation-decisions.md](implementation-decisions.md) — does this touch a resolved architectural question?
- [ ] Check [registries.md](registries.md) — are the names, stats, and content correct?
- [ ] Read the relevant detail doc for the full spec
- [ ] If the detail doc says something different from the three canonical files, the canonical files win
