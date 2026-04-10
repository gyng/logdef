Project: SUPPLY LINE | Audio Direction

> **Status:** v1 and post-v1 vision. Check [v1-scope.md](v1-scope.md) before implementing any feature described here.

## I. Core Principle

The tower is a living machine. Its sounds ARE the soundtrack. Every building, every transport link, every runner contributes to a layered soundscape that shifts between warm workshop ambience (prep) and tense mechanical urgency (combat). Music exists as ambient support — it sets the emotional floor, but the tower's components and combat feedback carry the audio experience.

**The emotional arc:** Warm and cozy when safe → tense and driving when fighting → quiet and thin when damaged → warm again when rebuilt. The player should know their tower's state by ear alone.

**Essential pillars vs. full target mix:** This document describes both the non-negotiable emotional promises and the aspirational production target. The pillars that must ship in any version: (1) tower components produce audible loops that go silent when destroyed, (2) combat sounds come from the right, tower sounds from the left, (3) silence is meaningful — you hear what's missing. Everything else — adaptive music stems, terrain variants, vertical spatial cues, per-transport loops — is the full target mix. It makes the game richer but is not load-bearing for the emotional contract. When scoping, protect the pillars first.

---

## II. The Two Modes

### Prep / Travel — "Home"

The tower is resting. You're inside the workshop. Everything is warm, rhythmic, productive.

**Tone:** Cozy, contemplative, safe. The sound of a well-maintained home that also happens to be a factory. Think: a blacksmith's shop on a quiet afternoon, a clockmaker's workshop, a ship's cabin in calm waters.

**Characteristics:**
- Gentle, organic rhythms from buildings (not metronomic — slightly irregular, alive)
- Warm tonal palette: wood creaks, fire crackle, liquid bubbles, soft metal rings
- Ambient room tone: wind outside, distant landscape sounds
- The walking sound of the tower's legs (rhythmic, grounding, the heartbeat of the game)
- Companion chatter as occasional punctuation (not constant — a comment every 30-60 seconds)

### Combat / Encounter — "Under Pressure"

Enemies are approaching. The tower has stopped. Everything tightens.

**Tone:** Tense, percussive, escalating. The workshop sounds don't stop — they're overwhelmed by combat noise. The contrast between the warm tower interior still producing and the harsh exterior fight is the emotional design. The feeling is a home under threat, not a battlefield — protective tension, not war.

**Characteristics:**
- Sharp, impactful combat sounds (arrow release, bolt snap, hit thud)
- Enemy sounds approach from the RIGHT (spatial audio)
- Tower sounds continue on the LEFT but are muted slightly during heavy combat
- Intensity scales with threat level: quiet approach → steady combat → intense multi-wave → boss crescendo
- Breaches are alarming: crash of panel, discord, interior destruction sounds from the LEFT
- Victory: combat sounds fade, tower sounds return to prominence, relief

---

## III. Tower Soundscape — The Living Factory

Every component of the tower contributes a sound layer. When the tower is healthy and running, these layer into a rich ambient hum. When components are destroyed, their sounds cut out — you hear the gap.

### Production buildings (each has a distinct loop)

| Building | Sound | Character |
|----------|-------|-----------|
| Fletcher | rhythmic scraping, wood shaving, string pluck (bow stringing) | light, woody, quick |
| Forge | hammer on anvil (ring-clang), bellows (whoosh-huff), fire crackle | heavy, metallic, warm |
| Quarry | stone grinding, chisel tap, rock crumble | deep, gritty, slow |
| Lumberyard | saw rasp, wood crack, thud of logs | raw, organic |
| Sawmill | circular saw whine (low fantasy version), plank drop | industrial, whiny |
| Alchemist | liquid bubble, glass clink, hiss of vapor, occasional pop | delicate, mysterious |
| Weaponsmith | precise hammering (lighter than forge), grinding wheel | refined, metallic |
| Enchanter | low hum, crystal resonance, magical chime | ethereal, tonal |
| Siege Works | heavy hammering, wood creak of large structures | massive, ponderous |
| Artificer | clicking, spring tension, small explosions (contained) | precise, dangerous |

**Building states affect sound:**
- **Producing (active):** full sound loop, normal volume
- **Full buffer (idle):** loop slows down, quieter — the worker is waiting, tapping idle sounds
- **Starved (no inputs):** loop stops. Silence from that floor. Maybe a frustrated sigh or idle fidget sound every few seconds
- **Destroyed:** silence. Complete. You notice the gap where the forge clang used to be

**Layering:** A 3-floor tower has a simple soundscape (fletcher + forge + quarry = scraping + clanging + grinding). An 8-floor tower is a rich symphony. The sounds layer additively — each new building adds its voice. The tower literally sounds bigger as it grows.

### Transport infrastructure

| Transport | Sound | When |
|-----------|-------|------|
| Built-in stairs | wood creak, footsteps | when runners use them |
| Ladder | faster creaking, slightly strained | when runners climb |
| Dumbwaiter | pulley squeak, rope creak, gentle thud at each end | continuous when active |
| Chute | whoosh of sliding goods, clatter at the bottom | each crate sent |
| Cargo lift | chain rattle, mechanical grinding, platform thud at stops | when in motion |
| Express lift | faster chain rattle, wind of speed, sharp stop | when in motion |
| Conveyor | continuous low rumble, click of belt segments | when active |
| Pneumatic tube | hiss, whoosh, pop at delivery end | each item sent |

**Congestion sounds:** When runners queue on stairs, you hear footsteps bunching — tap-tap-tap becoming a cluster of impatient shuffling. When a lift is overloaded (multiple runners waiting), you hear the queue — runners shifting weight, crates being set down and picked up.

**Transport breakdown:** When a lift breaks, the chain grinding stops with a loud CLUNK. Silence where the lift sound was. Runners scramble — hurried footsteps on stairs that weren't being used a moment ago.

### Warehouse / depot

| Event | Sound |
|-------|-------|
| Crate deposited by runner | solid thud, wood-on-wood |
| Crate picked up from cache | lighter thud, rustle |
| Cache refilled | satisfying click (crate sliding into slot) |
| Rack restocked | thud on exterior, arrows/bolts rattling in crate |
| Warehouse full | subtle chime — "we're good" |
| Warehouse critically low | low alarm hum (not jarring, just present) |

### Tower walking (by leg type)

| Legs | Sound | Character |
|------|-------|-----------|
| Chicken legs | organic thump-thump, slight squawk, wobbly creaking | charming, irregular, alive |
| Spider legs | precise clicking, mechanical articulation, steady rhythm | clockwork, even, unsettling |
| Mechanical treads | continuous grinding rumble, metallic clatter, exhaust chug | heavy, industrial, powerful |
| Magical hover | deep harmonic hum, magical shimmer, air displacement | ethereal, otherworldly |

The walking sound is the game's heartbeat. It plays during travel between nodes. It stops during combat (the tower plants itself). The silence of the stopped tower when combat begins is itself a transition cue.

---

## IV. Combat Audio

### Weapon sounds

Each weapon type has a distinct fire sound AND hit sound. The fire-hit pair should feel connected — you hear the release, then the impact, with the delay proportional to distance.

| Weapon type | Fire sound | Hit sound |
|-------------|-----------|-----------|
| Bow | string twang, arrow whoosh | meaty thud (flesh), thwack (armor) |
| Crossbow | mechanical snap, bolt whistle | sharp crack, piercing thud |
| Staff/wand | magical charge-up, release hum | crystalline shatter, energy burst |
| Thrown (javelin) | grunt of effort, air whoosh | heavy impact, clang (if armored) |
| Thrown (bomb) | fuse sizzle, whoosh | explosion (medium, contained) |
| Gun (pistol) | sharp crack, smoke pop | impact thud |
| Gun (musket) | deep boom, echo | devastating crunch |
| Whip | crack (sharp, instant) | lash sound (skin) |
| Melee (sword) | slash through air | blade-on-flesh, clang on armor |
| Melee (hammer) | heavy swing whoosh | crunch, bone crack |
| Shield (block) | metallic clang, impact absorption | reflected projectile whistle |
| Instrument (drum) | deep boom, vibration | no separate hit — the boom IS the damage |
| Instrument (horn) | brassy blast | no separate hit |
| Instrument (bell) | massive toll, reverb | no separate hit |

**Weapon abilities** should have a distinct "charge up" or "activation" sound that's louder and more dramatic than normal fire. The player should FEEL the ability activate.

### Miss sounds
- Arrow misses: whistle past, thud into ground (quiet, mildly shameful)
- Bolt misses: whistle, distant impact on terrain
- Spell misses: fizzle, dissipation

Misses should be audibly distinct from hits. You know you missed by sound alone.

### Critical hits
All critical hits get a bonus sound layer: a sharp, bright accent on top of the normal hit sound. Like a bell strike or a glass crack. Universal across weapon types — you always know a crit happened regardless of weapon.

### Enemy sounds

| Enemy type | Approach | At tower | Death |
|------------|----------|----------|-------|
| Grunt | distant footsteps, murmuring | banging on panel (thud-thud) | short cry, body fall |
| Runner | rapid footsteps, panting | same but faster | short cry |
| Armored | heavy stomping, metal clank | heavy panel hammering | metallic crash |
| Climber | scraping on stone/wood, grunting | same, reaching balcony level | cry + falling sound |
| Flyer (hoverer) | wing beats, distant | projectile launch (whoosh-crack) | squawk, falling + impact |
| Flyer (dive bomber) | rising whistle (dive incoming) | heavy impact on panel | explosion-like crash |
| Catapult | distant creak, rope snap, boulder whoosh | impact BOOM (loud, screen-shaking) | wooden collapse, crew scatter |
| Ram | heavy footsteps, horn blare | deep THOOM against foundation | massive crash |
| Sapper | quiet footsteps (they're sneaky) | tool sounds (wrench, hammering on infrastructure) | small cry, tools clatter |
| Siege tower | massive rolling creak, wood grinding | docking THUD, climbers pouring out | wood splintering, collapse |

**Enemy approach is spatial.** Enemies come from the RIGHT. Their sounds should pan from right to center as they approach the tower. Climbers move from center (base) upward — their sounds rise in vertical pan.

**When the mix gets crowded, gameplay-critical priority beats spatial realism.** A breach alarm on floor 3 must be audible even if the literal pan position would place it behind a wall of combat noise on the same side. The spatial model is a default — the priority mix (Section IX) overrides it when sounds compete.

**Enemy intensity scaling.** A few grunts approaching are barely audible above the tower ambience. A full wave creates a rising pressure — footsteps become a rumble, calls overlap, the tower's internal sounds are pushed to the background. The audio itself tells you how much pressure you're under.

### Damage and breach

| Event | Sound | Intensity |
|-------|-------|-----------|
| Panel taking light damage | crack, small stone chip | subtle, background |
| Panel taking heavy damage | louder crack, splintering, dust rattle | noticeable, alarming |
| Panel at critical HP | continuous low creak, groaning stress | persistent warning |
| Panel breach | CRASH — wall crumbling, stones falling, dust cloud | dramatic, unmissable |
| Interior raiders entering | scrambling sounds from LEFT (inside the tower) | alarming |
| Interior raiders wrecking infrastructure | smashing wood, breaking metal, tool sounds | painful to hear |
| Interior raiders expelled | magical hum crescendo, whoosh, enchantment chime | relief |
| Foundation hit | deep THOOM, ground shake, entire tower rattles | the most alarming sound |
| Foundation critical | persistent low rumble, groaning, things falling off shelves | dread |

**The breach sound design is critical.** When a panel breaches, THREE things happen simultaneously:
1. The crash of the wall breaking (exterior)
2. The sudden appearance of enemy sounds from the LEFT (interior — they're inside)
3. The SILENCE of whatever building was on that floor (its production loop stops)

This three-part audio event tells the player everything: what happened, where, and what was lost. Without looking.

### Wave pacing audio

- **Wave start:** distant horn or drum beat from the RIGHT. "They're coming." (Not a military war horn — more like a warning call, a watchman's alert.)
- **Wave active:** layered enemy approach + combat sounds
- **Lull (between waves):** combat sounds fade. Tower ambience returns briefly. Runner sounds become prominent (they're restocking). Brief relief.
- **Next wave:** another horn/drum, slightly more urgent than the last. "More coming."
- **Final wave of encounter:** distinct horn call — deeper, longer. "This is the last push."
- **Victory:** last enemy dies. Beat of silence. Then a completion chime/sting. Tower walking sounds begin again. Warmth returns.

---

## V. Music

Music is ambient support, not the star. It sets the emotional floor that the SFX play on top of.

### Approach: layered stems

Each music track is composed of multiple stems (melodic, rhythmic, pad, etc.) that can be brought in and out dynamically based on game state. Not fully adaptive like AAA games — more like 3-4 layers that fade in/out.

### Prep / Travel music

**Instrument palette:** acoustic guitar, wooden flute, gentle strings, light percussion (hand drums, wood blocks). Folk-adjacent, warm, contemplative. Not medieval cliché — more like a campfire in a Ghibli film.

**Layers:**
1. **Base pad:** warm drone, barely there. Always playing.
2. **Melodic:** gentle theme, plays during early prep. Fades if player is idle for too long (the music respects your thinking).
3. **Rhythmic:** light percussion, fades in when you're spending ticks (activity = gentle energy).

**Variations by chapter/terrain:**
- Plains: open, airy, pastoral
- Forest: deeper, more reverb, bird calls in the mix
- Mountain: sparser, wind-carried melody, echo
- Swamp: slightly darker, wet percussion, lower register
- Storm: muted, distant thunder in the bass, melody barely audible

### Combat music

**Instrument palette:** deeper percussion (frame drums, floor toms — rhythmic urgency, not military), tense strings, low brass hints. The warmth of prep music is gone — replaced by tension and weight. The feeling should be a storm approaching a house, not an army marching.

**Layers:**
1. **Tension pad:** low drone, begins when enemies appear at range. Barely noticeable but shifts the emotional key.
2. **Rhythmic:** percussion starts with first enemy reaching the tower. Tempo matches wave intensity — slow for attrition, fast for burst.
3. **Intensity:** strings/brass swell when multiple threats are active (climbers + flyers + siege simultaneously). Fades when pressure drops.
4. **Boss layer:** distinct theme or motif that plays only during boss encounters. Heavier, more melodic, more memorable. Each boss type could have a motif variation.

**Critical moment stings (brief, triggered):**
- Breach: discordant hit, low brass
- Ram approaching: deep drum, builds (tension, not fanfare)
- Hero rack empty: subtle, thinning of music (instruments drop out, leaving just the drone)
- Victory: resolution chord, return to warmth

### Post-combat

Music returns to prep warmth, but muted. The "we survived" moment. If damage was heavy, the warm theme is played with fewer instruments (reflecting the thinned tower soundscape). As you repair during prep, instruments return.

### Map music

Its own piece — contemplative, slightly adventurous. The melody of the journey. Plays only during the map screen. When you close the map, it crossfades back to prep music. This theme is the one players will associate with "planning the route" — it should feel like looking at a map by firelight.

---

## VI. UI Audio

Every interaction should have a sound. But UI sounds should be QUIET — they're confirmations, not events.

| Action | Sound | Volume |
|--------|-------|--------|
| Click button | soft click, tactile | quiet |
| Hover button | very soft tick | barely there |
| Spend tick | clock tick, mechanical click | noticeable (spending ticks matters) |
| Build floor/building | construction thud (heavier than button) | medium |
| Equip weapon | metallic slide, weapon-specific clink | medium |
| Equip trinket | light chime | quiet |
| Configure orders | paper rustle, stamp | quiet |
| Open panel (slide in) | soft whoosh | quiet |
| Close panel | reverse whoosh | quiet |
| Validation warning | gentle alert ding | noticeable |
| March button | decisive drum hit | medium-loud (this is a commitment) |
| Buy from merchant | coin clink, register "ka-ching" | medium |
| Sell to merchant | coin slide, lower "ka-ching" | medium |
| Loot reveal (card flip) | card flip swoosh + rarity-dependent chime | medium |
| Perk selection | deep, resonant chime (permanent choice, weight) | medium-loud |
| Stat allocation | tick + small ascending tone | quiet-medium |
| Error (can't afford) | dull thud or buzz | quiet but distinct |

**Rarity chimes (for loot reveals):**
- Common: simple click (barely a sound)
- Uncommon: gentle chime
- Rare: brighter two-note chime
- Legendary: dramatic three-note rising chime with shimmer

---

## VII. Spatial Audio

The game is side-on. Left = tower interior. Right = exterior/battlefield. Spatial panning reinforces this:

- **Tower production/transport sounds:** center-left. You hear the factory on your left.
- **Combat sounds:** center-right. Enemies approach from the right.
- **Breach sounds:** sharp shift to LEFT. Interior destruction from the wrong side. Alarming because it's in the "safe" zone.
- **Loot rolling to base:** right to center (loot rolls from battlefield toward tower base).
- **Companion shouts:** positioned vertically matching their floor height. A companion on floor 6 shouts from "high" in the mix, one on floor 2 from "low." You can tell WHO is in trouble by spatial position.

### Height in audio

Vertical audio is harder to implement than horizontal but worth approximating:
- Higher-positioned sounds have slightly more reverb (more "air")
- Lower-positioned sounds are drier, more grounded
- This gives a subtle sense of floor height to building sounds and companion positions

---

## VIII. Silence as Design

Silence is the most powerful audio tool. Use it deliberately.

**Protecting silence in a busy mix:** The tower soundscape can have 10+ simultaneous loops, combat layers, music, and UI. For silence to register as feedback, the mix must actively make room. During lulls, combat sounds and music stems must fully duck — not just reduce. When a building is destroyed, its frequency band should stay empty for at least 2-3 seconds before other sounds fill the gap. The volume priority list (Section IX) exists partly to serve this: lower-priority layers yield so that the absence of a higher-priority layer is audible.

- **Between waves (lull):** combat sounds fade. Tower ambience returns. The ABSENCE of combat is the relief. Don't fill it with music or UI sounds. Let the factory breathe.
- **Destroyed building:** the building's sound loop cuts out. The gap in the soundscape IS the damage feedback. A forge that was clanging for 20 encounters is suddenly silent. You FEEL the loss.
- **Broken transport:** the lift's chain rattle stops. The staircase is suddenly busy (runners rerouting — footstep sounds increase on stairs). The soundscape shifts to tell you the logistics changed.
- **Before a boss:** brief total silence. All sounds dip for 1 second. Then the boss appears with its own sound. The silence is the "oh no" moment.
- **Empty rack:** when your rack runs out, there's a subtle audio change — the background seems to thin, the combat sounds feel more exposed. You're not hearing a new sound; you're hearing the absence of your weapon's rhythm. The gap where your bow twangs used to be.

---

## IX. Implementation Notes

### Architecture: Web Audio API everywhere

One audio implementation. The game always runs in a web context — browser natively, desktop via Tauri/Electron. Web Audio API is the only audio backend.

```
Rust Simulation → SoundEvent list + AudioState (in RenderSnapshot)
    │
    ▼
JS AudioManager (Web Audio API) → audio output
    │
    ├── Browser: native Web Audio
    └── Desktop: Web Audio via Tauri/Electron webview
```

**Why one backend:**
- Web Audio API works identically in browser and Electron/Tauri. Same code, same behavior, same capabilities.
- No Rust audio dependency (no kira, no rodio, no cpal). Fewer crates, simpler build.
- Web Audio has everything we need: spatial panning, effects, mixing, dynamic routing, low latency.
- The React layer already exists — audio lives there alongside the UI.

### Audio manager (JavaScript/TypeScript)

```typescript
class BrowserAudioManager {
    private ctx: AudioContext;
    // Spatial panner for left/right positioning
    private masterPanner: StereoPannerNode;
    // Group gain nodes for volume mixing
    private groups: Record<SoundGroup, GainNode>;
    // Active ambient loops
    private buildingLoops: Map<number, AudioBufferSourceNode>;
    // Pre-decoded audio buffers
    private buffers: Map<string, AudioBuffer>;
    // Music stem sources
    private musicStems: AudioBufferSourceNode[];

    processEvents(events: SoundEvent[]) {
        for (const event of events) {
            switch (event.type) {
                case 'WeaponFired':
                    this.playOneShot(
                        `weapon_${event.weaponType}_fire`,
                        this.groups.combat,
                        { pan: this.positionToPan(event.position) }
                    );
                    break;
                case 'PanelBreach':
                    this.playOneShot('panel_crash', this.groups.damage, {
                        pan: this.floorToPan(event.floor),
                    });
                    // Stop building loop (silence = damage)
                    this.stopBuildingLoop(event.floor, { fadeOut: 0.1 });
                    break;
                // ... etc
            }
        }
    }

    private playOneShot(
        soundId: string,
        group: GainNode,
        opts: { pan?: number; volume?: number } = {}
    ) {
        const buffer = this.buffers.get(soundId);
        if (!buffer) return;
        const source = this.ctx.createBufferSource();
        source.buffer = buffer;
        const panner = this.ctx.createStereoPanner();
        panner.pan.value = opts.pan ?? 0;
        source.connect(panner).connect(group).connect(this.ctx.destination);
        source.start();
    }
}
```

**React integration:**

```typescript
// Hook that connects simulation events to browser audio
function useAudioManager(bridge: GameBridge) {
    const audioRef = useRef<BrowserAudioManager>();

    useEffect(() => {
        audioRef.current = new BrowserAudioManager();
        return () => audioRef.current?.dispose();
    }, []);

    // Called every frame during combat, every action during prep.
    // Production note: real implementation must handle browser audio unlock
    // (user gesture required before AudioContext starts) and burst limiting
    // (cap sound_events processed per frame to avoid audio pile-up).
    const processFrame = useCallback((snapshot: RenderSnapshot) => {
        if (audioRef.current && snapshot.sound_events.length > 0) {
            audioRef.current.processEvents(snapshot.sound_events);
        }
        // Sync ambient state (building loops, music stems)
        audioRef.current?.syncAmbient(snapshot.audio_state);
    }, []);

    return { processFrame };
}
```

### Audio asset loading

All audio assets are pre-decoded into `AudioBuffer` on load. Assets bundled as OGG files, fetched and decoded during the loading screen. Works identically in browser and Tauri/Electron desktop.

```typescript
async function loadAudioAssets(ctx: AudioContext): Promise<Map<string, AudioBuffer>> {
    const manifest = await fetch('/assets/audio/manifest.json').then(r => r.json());
    const buffers = new Map();
    await Promise.all(
        manifest.map(async (entry: { id: string; path: string }) => {
            const response = await fetch(entry.path);
            const arrayBuffer = await response.arrayBuffer();
            const audioBuffer = await ctx.decodeAudioData(arrayBuffer);
            buffers.set(entry.id, audioBuffer);
        })
    );
    return buffers;
}
```

### Shared SoundEvent enum

The simulation produces platform-agnostic sound events. The audio manager consumes them identically in browser and desktop (same Web Audio API backend, per Section IX).

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum SoundEvent {
    // Combat
    WeaponFired { weapon_type: WeaponBaseType, position: Vec2 },
    ProjectileHit { weapon_type: WeaponBaseType, position: Vec2, is_crit: bool },
    ProjectileMiss { weapon_type: WeaponBaseType, position: Vec2 },
    EnemyDeath { archetype: EnemyArchetype, position: Vec2 },
    EnemyApproach { archetype: EnemyArchetype, position: Vec2, count: u32 },
    AbilityActivated { ability: AbilityId },
    CompanionFired { companion_id: CompanionId, weapon_type: WeaponBaseType },

    // Damage
    PanelDamage { floor: usize, severity: DamageSeverity },
    PanelBreach { floor: usize },
    FoundationHit,
    RaiderEnter { floor: usize },
    RaiderExpelled { floor: usize },
    InfrastructureDestroyed { floor: usize, infra_type: InfraType },

    // Logistics
    CrateDeposited { floor: usize },
    CratePickedUp { floor: usize },
    RackRestocked { floor: usize },
    RackEmpty { floor: usize },
    BufferFull { floor: usize, building_type: BuildingType },
    BufferStarved { floor: usize, building_type: BuildingType },
    TransportBreakdown { transport_type: TransportType, floor: usize },
    LiftArrived { floor: usize },
    ChuteDelivery { from_floor: usize, to_floor: usize },
    RunnerQueueForming { transport_id: TransportId, queue_length: u32 },

    // Flow
    WaveStart { wave_number: u32, is_final: bool },
    LullStart,
    EncounterStart,
    EncounterEnd,
    BossAppear { boss_type: BossType },

    // UI (generated by React/JS, not simulation)
    TickSpent,
    BuildComplete { building_type: BuildingType },
    LootReveal { rarity: Rarity },
    PerkSelected { perk: Perk },
    MarchBegin { leg_type: LegType },
}
```

### AudioState — ambient sync

Beyond one-shot events, the audio manager must continuously sync ambient state (which buildings are active, which transport is running, what music phase we're in). This is a separate struct in the RenderSnapshot:

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AudioState {
    pub phase: GamePhase,
    pub active_buildings: Vec<(FloorIndex, BuildingType, BuildingStatus)>,
    pub active_transport: Vec<(TransportId, TransportType, TransportStatus)>,
    pub walking: Option<LegType>,  // None if stopped
    pub combat_intensity: f32,     // 0.0 (calm) to 1.0 (maximum pressure)
    pub music_phase: MusicPhase,   // Prep | Combat | Boss | Lull | PostCombat | Map
    pub terrain: Option<TerrainType>,  // for terrain-specific ambient
}
```

The audio manager reads this every frame and adjusts ambient loops/music accordingly — starting/stopping building loops, adjusting music stem volumes, changing the walking sound.

### Volume mix priorities

When multiple sounds compete, priorities determine what's loudest:

1. **Damage alerts** (panel breach, foundation hit) — always audible, never masked
2. **Hero weapon** (your fire + hit sounds) — always prominent
3. **Combat (enemy approach, companion fire)** — scales with intensity
4. **Tower logistics** (runners, transport, production) — background, can be partially masked by combat
5. **Music** — lowest priority, ducks under everything else
6. **UI sounds** — very quiet, never competes with game audio

### Asset list (priority order)

**Phase 0 (prototype):**
- Bow fire + hit sounds
- Grunt approach + death sounds
- Basic panel damage sound
- Simple tower ambient loop (forge-like)
- UI click

**Phase 1 (logistics):**
- All production building ambient loops (10 buildings)
- Runner footsteps (walking, climbing, queuing)
- Crate deposit/pickup
- Staircase creak
- Rack restock

**Phase 2 (combat + silence):**
- All weapon fire + hit sounds (9 base types)
- All enemy type sounds (approach, climb, attack, death — 10+ types)
- Critical hit accent
- Miss sounds
- Panel damage progression (light → heavy → critical → breach)
- Breach three-part audio event (crash + interior enemy sounds + building silence) — this IS the pillar, not polish
- Silence design for destroyed buildings and empty racks — absence feedback must be testable as soon as buildings have loops
- Wave start/end stings
- Ability activation sounds
- Victory/defeat stings

**Phase 3 (journey):**
- All leg type walking sounds (4 types)
- All transport sounds (8 types)
- Map music
- Merchant interaction sounds
- Companion voice lines (text-triggered, not full VO — just combat barks and brief reactions)

**Phase 4 (polish):**
- Adaptive music stems (prep + combat + boss)
- Spatial audio refinement
- Terrain ambient variations (6-7 terrains)
- Rarity chimes
- Foundation damage rumble

**Phase 5 (content):**
- Boss-specific themes/motifs
- Chapter-specific ambient variations
- Weather sounds (rain, storm, wind)
- More companion barks
- More UI polish sounds

---

## XI. AI Audio Generation Spec

This section exists to make AI-generated audio assets consistent with each other and with the game's sound identity. The descriptions in earlier sections are written for human understanding. This section translates them into generation constraints.

### Global consistency anchors

All generated assets must be post-processed to share these properties. Raw AI output is never the final asset.

| Property | Target | Why |
|----------|--------|-----|
| Loudness | -16 LUFS integrated (loops), -12 LUFS peak (one-shots) | Consistent perceived volume before in-engine mixing |
| Sample rate | 44.1kHz / 16-bit | Web Audio standard, smaller files than 24-bit, no audible difference in-game |
| Noise floor | Below -60dB | AI tools often leave artifacts; gate or denoise every asset |
| Room character | Small wooden room, ~0.3s RT60, warm early reflections | The tower interior is wood/stone, not a cathedral. Apply a shared convolution reverb IR to ALL interior sounds so they feel co-located |
| Stereo width | Mono for one-shots, stereo for ambient loops | One-shots are panned in-engine; baking in stereo fights the spatial system |
| Dynamic range | Max 12dB crest factor for loops, 18dB for impacts | Loops must sit steady in the mix; impacts can spike |

**Shared reverb IR:** generate or source ONE impulse response that represents "inside a 4m × 4m wooden room with stone walls." Apply this to all building loops, transport sounds, and runner footsteps. This single constraint does more for cohesion than any amount of prompt engineering.

### Asset types and durations

| Type | Duration | Format | Loop? | Notes |
|------|----------|--------|-------|-------|
| Building ambient loop | 8-12s | OGG | Yes, seamless | Must crossfade cleanly at loop point. Generate 15s, trim to clean loop |
| Transport loop | 4-8s | OGG | Yes, seamless | Shorter than buildings — mechanical rhythms |
| Walking loop (per leg type) | 4-6s | OGG | Yes, seamless | Tempo-locked: chicken ~90 BPM, spider ~120 BPM, treads continuous, hover continuous |
| Weapon fire (one-shot) | 0.1-0.5s | OGG | No | Must start instantly — no fade-in, no pre-delay |
| Weapon hit (one-shot) | 0.1-0.4s | OGG | No | Tighter than fire. The hit confirms the fire |
| Enemy approach (loop) | 4-6s | OGG | Yes | Layered per-enemy, so keep sparse |
| Enemy death (one-shot) | 0.3-0.8s | OGG | No | Short. Not dramatic — these die often |
| Panel damage (one-shot) | 0.3-1.0s | OGG | No | Escalating: light=0.3s, heavy=0.6s, breach=1.0s |
| UI click/chime (one-shot) | 0.05-0.3s | OGG | No | Extremely short. Quiet |
| Music stem | 30-60s | OGG | Yes, seamless | All stems for a track must share tempo and key |
| Rarity chime (one-shot) | 0.3-1.5s | OGG | No | Common=shortest, Legendary=longest with shimmer tail |

### Frequency band assignments

Each building loop should occupy a primary frequency band so they layer without mud. This is the most important constraint for a 6-8 floor tower sounding clear.

| Building | Primary band | Character | Generation hint |
|----------|-------------|-----------|-----------------|
| Fletcher | 2kHz-6kHz | scraping, plucking — bright, airy | High-shelf emphasis, roll off below 1kHz |
| Forge | 200Hz-800Hz | hammer ring, bellows — warm, heavy | The bass anchor of the tower |
| Quarry | 100Hz-400Hz | grinding, rumble — deep, subby | Below forge, felt more than heard |
| Lumberyard | 500Hz-2kHz | sawing, cracking — mid, organic | Midrange body |
| Sawmill | 1kHz-4kHz | whine, buzz — industrial, cutting | Shares range with fletcher but different texture (sustained vs. rhythmic) |
| Alchemist | 3kHz-8kHz | bubbling, glass, hiss — delicate, sparkly | Highest of all buildings. Sits on top |
| Weaponsmith | 800Hz-2kHz | precise hammering, grinding — refined | Between forge and lumberyard |
| Enchanter | 300Hz-1kHz (fundamental) + 4kHz-8kHz (harmonics) | hum + shimmer — split-band | Two-band character: low drone + high sparkle |
| Siege Works | 80Hz-300Hz | heavy hammering, wood stress — massive | Lowest, shares with quarry but slower rhythm |
| Artificer | 1.5kHz-5kHz | clicking, springs, tiny pops — precise | Shares upper range but very different rhythm (staccato) |

**How to use these bands:** When prompting an AI tool, describe the sound AND specify "emphasize frequencies between X and Y Hz" or "roll off below X Hz." Then EQ the result to reinforce. The goal is not surgical separation — it's giving each building enough spectral space that 6+ loops layer into a rich texture instead of a wall of mid.

### Prompt template

When generating with AI audio tools, use this structure for consistency:

```
[SOUND TYPE]: [one-shot / loop, duration]
[DESCRIPTION]: [physical description — what object, what action, what material]
[CHARACTER]: [2-3 adjectives from the art direction: warm, mechanical, organic, etc.]
[FREQUENCY]: [primary band from table above]
[ROOM]: [small wooden interior / open exterior / none for UI]
[REFERENCE]: [closest real-world sound analogy]
[AVOID]: [what this should NOT sound like]
```

**Example — Forge loop:**
```
SOUND TYPE: loop, 10 seconds, seamless
DESCRIPTION: blacksmith hammer striking anvil every 2-3 seconds, leather bellows
  breathing in and out slowly, fire crackling underneath. Irregular rhythm — alive,
  not metronomic. The hammer rings vary slightly in pitch and timing.
CHARACTER: warm, heavy, metallic, organic
FREQUENCY: emphasize 200-800Hz. The hammer ring can peak at 1.5kHz but should
  decay quickly. Bellows are sub-300Hz.
ROOM: small stone-and-wood workshop, close-mic, short reverb (apply shared IR in post)
REFERENCE: a real blacksmith shop recorded from 2 meters, not a cinematic sound effect
AVOID: epic/cinematic anvil hits, reverb tails longer than 0.5s, any synthesized
  or electronic character, metronomic timing
```

**Example — Bow fire one-shot:**
```
SOUND TYPE: one-shot, 0.3 seconds max
DESCRIPTION: wooden bow string released, arrow leaves — string twang followed
  immediately by a brief air whoosh as the arrow passes the mic.
CHARACTER: light, woody, quick, tactile
FREQUENCY: string twang at 1-3kHz, whoosh is broadband but quiet
ROOM: none (apply shared IR in post, or leave dry for exterior positioning)
REFERENCE: real recurve bow release, not a cinematic "power shot"
AVOID: reverb, delay, any sustained resonance, synthetic bow sounds, slow attack
```

### Post-processing chain

Apply to every AI-generated asset before importing into the game:

1. **Denoise** — gate or spectral denoise to reach -60dB noise floor. AI tools leave artifacts.
2. **Trim** — remove silence at head and tail. One-shots should start at the transient.
3. **EQ** — reinforce the building's assigned frequency band. Gentle cuts (3-6dB) outside the primary band, not surgical notches. The goal is emphasis, not isolation.
4. **Normalize** — to target LUFS per the table above.
5. **Shared reverb** — apply the shared room IR to all interior sounds. Skip for exterior combat sounds and UI.
6. **Loop check** (loops only) — crossfade the last 0.5-1s into the first 0.5-1s. Verify seamless playback at 2× loop count minimum.
7. **Layering test** — play the new asset simultaneously with 3-4 other building loops. If it muds up or masks another building, go back to step 3.

### Batch generation strategy

Don't generate one sound at a time. Generate in families:

1. **All building loops in one session.** Generate all 10, then layer-test them together. Adjust EQ until the stack is clear. This is more efficient than perfecting one loop in isolation and discovering it clashes later.
2. **All weapon fire sounds in one session.** They need to be distinct from each other but share a dynamic range and presence level.
3. **All enemy approach sounds in one session.** They layer in combat — a grunt + armored + climber stack must be readable.
4. **Music stems per track.** All stems for one music track must be generated together — same key, same tempo, same tonal palette. Generate the base pad first, then generate other stems with the pad as a reference.

### Iteration and replacement

AI-generated assets are placeholders until they're not. Tag every asset with:

```
{id}_v{version}_ai_{tool}_{date}.ogg
```

Example: `forge_loop_v3_ai_elevenlabs_20260410.ogg`

This lets you batch-replace all assets from one tool if a better tool appears, track which version is in-game, and A/B test alternatives without losing prior work.
