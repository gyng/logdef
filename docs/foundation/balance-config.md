Project: SUPPLY LINE | v1 Balance Config Skeleton

> **Status:** v1 spec. All values are initial placeholders derived from telemetry targets in [telemetry-balance.md](telemetry-balance.md). Every number here WILL change during playtesting. The point is to give engineers concrete starting values so systems can be built, tested, and iterated on.
>
> **Runtime config:** these values are exposed as runtime config (TOML/JSON). No recompile to adjust. See [tech-performance.md](tech-performance.md) for the config-loading approach.

---

## 1. Enemy Stats

Design targets from [telemetry-balance.md](telemetry-balance.md) §II: grunts die in 1-3s, armored in 8-15s, climber reach-rate 20-40%, ram reach-rate <5%.

```toml
[enemies.grunt]
hp = 30
damage_per_hit = 8          # panel damage
speed = 60                   # pixels/sec approach speed
attack_rate = 1.0            # hits per second once at panel
threat_cost = 1              # encounter budget cost
bounty_gold = 3

[enemies.runner]
hp = 20
damage_per_hit = 5
speed = 120
attack_rate = 1.2
threat_cost = 2
bounty_gold = 4

[enemies.armored]
hp = 150
damage_per_hit = 15
speed = 30
attack_rate = 0.8
threat_cost = 4
bounty_gold = 10

[enemies.climber]
hp = 40
damage_per_hit = 10
climb_speed = 35             # pixels/sec vertical
attack_rate = 1.0
threat_cost = 3
bounty_gold = 6
# Targets infrastructure when reaching a position

[enemies.flyer_hoverer]
hp = 50
damage_per_hit = 12          # ranged panel damage
speed = 45
attack_rate = 0.7            # slower but consistent
attack_range = 200           # pixels, hovers at distance
threat_cost = 4
bounty_gold = 8

[enemies.catapult]
hp = 80
damage_per_hit = 25          # heavy panel damage, splash to adjacent floors
speed = 20                   # very slow approach
attack_rate = 0.3            # slow but devastating
attack_range = 400
splash_floors = 1            # hits target floor +/- 1
threat_cost = 5
bounty_gold = 12

[enemies.ram]
hp = 200
damage_per_hit = 30          # foundation damage
speed = 25
attack_rate = 0.5
threat_cost = 8
bounty_gold = 20
# Targets foundation only

[enemies.sapper]
hp = 35
damage_per_hit = 0           # doesn't damage panels — destroys infrastructure
climb_speed = 50             # faster climber
threat_cost = 3
bounty_gold = 8
# Destroys one transport/cache/rack on reaching target, then expelled

[enemies.boss_ground]
hp = 500                     # phase 1
hp_phase2 = 350              # phase 2 (triggered at 66% of phase 1 HP)
damage_per_hit = 20
speed = 35
attack_rate = 0.6
phase_transition_hp_pct = 0.66
threat_cost = 0              # bosses are standalone encounters
bounty_gold = 50

[enemies.boss_climbing]
hp = 400
hp_phase2 = 300
damage_per_hit = 15
climb_speed = 25
attack_rate = 0.8
phase_transition_hp_pct = 0.66
threat_cost = 0
bounty_gold = 50
```

---

## 2. Encounter Composition

Encounter difficulty is defined by a **threat budget**. Each enemy type has a threat_cost (above). The encounter spawner fills the budget with enemy combinations.

```toml
[encounters]
# Threat budget per difficulty tier
difficulty_1_budget = 8       # Ch1 early: 2-3 grunts + 1 runner
difficulty_2_budget = 16      # Ch1 late / Ch2 early
difficulty_3_budget = 28      # Ch2 late / Ch3 early
elite_budget_multiplier = 1.5 # elite = base budget × 1.5

# Wave structure
waves_min = 1
waves_max = 3                 # max waves per encounter (v1)
wave_delay_seconds = 5.0      # pause between waves
wave_budget_split = [0.4, 0.35, 0.25]  # % of total budget per wave (3-wave)

# Chapter difficulty ranges
[encounters.chapter_1]
min_difficulty = 1
max_difficulty = 2
elite_allowed = false
enemy_pool = ["grunt", "runner"]  # only basic types

[encounters.chapter_2]
min_difficulty = 2
max_difficulty = 3
elite_allowed = true
enemy_pool = ["grunt", "runner", "armored", "climber", "flyer_hoverer"]

[encounters.chapter_3]
min_difficulty = 2
max_difficulty = 3
elite_allowed = true
enemy_pool = ["grunt", "runner", "armored", "climber", "flyer_hoverer", "catapult", "sapper", "ram"]
```

---

## 3. Production Buildings

Target from [telemetry-balance.md](telemetry-balance.md) §III: warehouse utilization 40-70%, material diversion 10-20%.

```toml
[buildings.fletcher]
tier = 1
output_resource = "arrows"
production_rate = 6.0         # crates per minute
output_buffer_max = 4         # crates before full (runner must collect)
operating_cost_gold = 5       # gold per encounter
build_cost_ticks = 2
build_cost_materials = { wood = 2 }

[buildings.forge]
tier = 1
output_resource = "bolts"
production_rate = 5.0
output_buffer_max = 4
operating_cost_gold = 6
build_cost_ticks = 2
build_cost_materials = { stone = 2 }

[buildings.quarry]
tier = 1
output_resource = "stone"
production_rate = 4.0
output_buffer_max = 3
operating_cost_gold = 4
build_cost_ticks = 2
build_cost_materials = { wood = 1 }

[buildings.lumberyard]
tier = 1
output_resource = "wood"
production_rate = 4.0
output_buffer_max = 3
operating_cost_gold = 4
build_cost_ticks = 2
build_cost_materials = { stone = 1 }

[buildings.sawmill]
tier = 2
input_resource = "wood"
input_rate = 3.0              # crates consumed per minute
output_resource = "planks"
production_rate = 2.0         # crates per minute (refining is slower)
output_buffer_max = 3
input_buffer_max = 4
operating_cost_gold = 0       # T2+ runs on inputs, not gold
build_cost_ticks = 3
build_cost_materials = { wood = 3, stone = 2 }

[buildings.alchemist]
tier = 2
input_resource = "stone"      # simplified v1: alchemist uses stone
input_rate = 2.0
output_resource = "mana_crystals"
production_rate = 1.5
output_buffer_max = 3
input_buffer_max = 3
operating_cost_gold = 0
build_cost_ticks = 3
build_cost_materials = { wood = 2, stone = 3 }

[buildings.weaponsmith]
tier = 2
input_resource = "wood"
input_rate = 2.0
output_resource = "weapon_parts"  # used for weapon crafting at merchant
production_rate = 1.0
output_buffer_max = 2
input_buffer_max = 3
operating_cost_gold = 0
build_cost_ticks = 4
build_cost_materials = { planks = 2, stone = 2 }
```

---

## 4. Transport

Target from [telemetry-balance.md](telemetry-balance.md) §IV: avg delivery time 8-15s, runner queue time <15%.

```toml
[transport.stairs]
speed = 1.0                   # base speed multiplier (reference)
direction = "both"
capacity = 1                  # one runner at a time
floor_width_cost = 0          # built-in, no width cost
build_cost_ticks = 0          # always available
build_cost_materials = {}

[transport.ladder]
speed = 0.7                   # slower than stairs
direction = "both"
capacity = 1
floors_spanned = 2            # connects 2 floors
floor_width_cost = 0.1
build_cost_ticks = 1
build_cost_materials = { wood = 1 }

[transport.dumbwaiter]
speed = 0.8
direction = "both"
capacity = 1
autonomous = true             # no runner needed
floor_width_cost = 0.15
build_cost_ticks = 2
build_cost_materials = { wood = 2, stone = 1 }

[transport.chute]
speed = 2.5                   # much faster than stairs
direction = "down_only"
capacity = 1                  # one crate at a time (2 with Engineer perk)
autonomous = true
floor_width_cost = 0.1
build_cost_ticks = 2
build_cost_materials = { planks = 1 }
unlock = "beat_ch1_boss"
```

---

## 5. Economy

Target from [telemetry-balance.md](telemetry-balance.md) §III: income/expense ratio 1.1-1.3x, gold at run end 50-100g, tick utilization 70-85%.

```toml
[economy]
# Tick budget per prep stop (scales by chapter)
ticks_chapter_1 = 6
ticks_chapter_2 = 8
ticks_chapter_3 = 10
ticks_per_rest_bonus = 2      # bonus ticks at rest nodes
unused_ticks_to_gold_rate = 3 # gold per unused tick

# Kill bounties
hero_kill_bonus = 1.5         # hero earns 1.5x bounty vs companion
encounter_completion_bonus = 15  # flat gold for completing encounter

# Operating costs (per encounter)
runner_salary = 3             # gold per runner per encounter
companion_wage = 2            # gold per companion per encounter
leg_maintenance = 5           # gold per encounter (chicken legs)

# Merchant
merchant_price_multiplier = 1.0  # base prices (1.0 = fair value)
merchant_sell_ratio = 0.5        # player sells at 50% of buy price
merchant_stock_size = 5          # items available per merchant visit

# Repair costs
panel_repair_cost_ticks = 1
panel_repair_cost_materials = { stone = 1 }

# Construction costs
floor_build_cost_ticks = 2
floor_build_cost_materials_wood = { wood = 3 }
floor_build_cost_materials_stone = { stone = 4 }
balcony_build_cost_ticks = 1
balcony_build_cost_materials = { planks = 1 }
office_build_cost_ticks = 1
office_build_cost_materials = { planks = 1 }
runner_quarters_build_cost_ticks = 2
runner_quarters_build_cost_materials = { wood = 2 }
runner_quarters_upgrade_cost_ticks = 1  # per additional runner slot
runner_quarters_upgrade_cost_gold = 10

# Starting resources
starting_gold = 30
starting_wood = 5
starting_stone = 3
```

---

## 6. Hero Weapon Stats

```toml
[weapons.shortbow]
base_damage = 15
fire_rate = 1.8               # shots per second (at max draw speed)
range = 250
projectile_speed = 300
min_draw_time = 0.15          # seconds
ammo_per_shot = 1
ability_cooldown = 12.0       # Rain
ability_description = "5 arrows rain on aimed zone"

[weapons.longbow]
base_damage = 35
fire_rate = 0.8
range = 400
projectile_speed = 350
min_draw_time = 0.3
ammo_per_shot = 1
ability_cooldown = 15.0       # Charged Shot
ability_description = "Piercing shot through all enemies in line"

[weapons.composite_bow]
base_damage = 22
fire_rate = 1.2
range = 300
projectile_speed = 320
min_draw_time = 0.2
ammo_per_shot = 1
ability_cooldown = 10.0       # Trick Shot
ability_description = "Ricochet to second target"

[weapons.hand_crossbow]
base_damage = 12
fire_rate = 2.0               # fast but low damage
reload_time = 0.5
range = 200
projectile_speed = 400        # flat, fast
ammo_per_shot = 1
ability_cooldown = 14.0       # Grapple Bolt
ability_description = "Pull enemy 1 floor toward hero"

[weapons.heavy_crossbow]
base_damage = 50
fire_rate = 0.5
reload_time = 1.5
range = 400
projectile_speed = 500
ammo_per_shot = 1
ability_cooldown = 30.0       # Snipe
ability_description = "Instant kill below HP threshold"
snipe_hp_threshold = 80       # kills if enemy HP <= this value

[weapons.wand]
base_damage = 10
fire_rate = 3.0               # fast channel
range = 250
projectile_speed = 500        # straight line, fast
ammo_per_shot = 1
ability_cooldown = 12.0       # Chain Lightning
ability_description = "Lightning chains to 3 nearby enemies"

[weapons.staff]
base_damage = 18
fire_rate = 1.5
range = 300
projectile_speed = 400
ammo_per_shot = 1
ability_cooldown = 18.0       # Barrier
ability_description = "Temporary shield on one panel, absorbs 50 damage"
barrier_hp = 50
barrier_duration = 10.0

[weapons.javelin]
base_damage = 30
fire_rate = 1.0
range = 300
projectile_speed = 250        # heavy arc
ammo_per_shot = 1
ability_cooldown = 14.0       # Impale
ability_description = "Pin enemy in place for 5s"
impale_duration = 5.0

[weapons.bomb]
base_damage = 20
fire_rate = 0.8
range = 250
projectile_speed = 200
splash_radius = 40            # pixels
ammo_per_shot = 1
ability_cooldown = 16.0       # Cluster Bomb
ability_description = "Splits into 4 bomblets on impact"

[weapons.dagger]
base_damage = 8
fire_rate = 3.5               # very fast
range = 30                    # melee only (tower face)
ammo_per_shot = 0
ability_cooldown = 8.0        # Flurry
ability_description = "5 rapid slashes in 1s"

[weapons.sword]
base_damage = 20
fire_rate = 1.5
range = 40                    # slightly more reach than dagger
ammo_per_shot = 0
ability_cooldown = 12.0       # Riposte
ability_description = "Next enemy hit within 2s is countered for 2x damage"
```

---

## 7. Weapon Modifiers

```toml
[modifiers.combat]
flaming_damage_bonus = 0.15           # +15% damage, DoT 3 damage/sec for 3s
flaming_dot_damage = 3.0
flaming_dot_duration = 3.0
frost_slow_percent = 0.30             # -30% enemy speed for 2s
frost_slow_duration = 2.0
explosive_splash_radius = 30          # pixels, 50% damage to splash targets
explosive_splash_damage_ratio = 0.50
piercing_armor_ignore = 0.50          # ignore 50% of armor (if armor system exists)
vampiric_lifesteal_ratio = 0.10       # 10% of damage heals nearest panel
venomous_dot_damage = 5.0             # 5 damage/sec for 4s
venomous_dot_duration = 4.0

[modifiers.economy]
efficient_ammo_save_chance = 0.20     # 20% chance to not consume ammo
gilded_gold_per_kill = 2              # +2 gold per kill
scavenging_loot_speed_bonus = 0.50    # loot rolls 50% faster

[modifiers.utility]
silent_aggro_reduction = true         # enemies don't prioritize this position
beacon_mark_on_hit = true             # hit enemies take +10% from all for 2s
beacon_mark_bonus = 0.10
beacon_mark_duration = 2.0
magnetic_loot_range_floors = 2        # loot within 2 floors rolls toward hero

[modifiers.drawback]
cursed_damage_bonus = 0.25            # +25% damage, but -20% accuracy
cursed_accuracy_penalty = -0.20
heavy_damage_bonus = 0.20             # +20% damage, -30% fire rate
heavy_fire_rate_penalty = -0.30
fragile_fire_rate_bonus = 0.25        # +25% fire rate, weapon breaks after 3 encounters (repairable)
fragile_durability_encounters = 3
bloodthirsty_damage_per_kill = 0.05   # +5% damage per kill this encounter, -1 ammo per rack restock
bloodthirsty_ammo_penalty = 1
```

---

## 8. Rarity Multipliers

```toml
[rarity]
common_stat_multiplier = 1.0
uncommon_stat_multiplier = 1.15       # +15% base stats
rare_stat_multiplier = 1.30           # +30% base stats
legendary_stat_multiplier = 1.50      # +50% base stats
legendary_unique_names_v1 = 15        # number of unique legendary names in v1

# Drop rates (per loot roll)
common_weight = 60
uncommon_weight = 25
rare_weight = 12
legendary_weight = 3
```

---

## 9. Tower & Foundation

```toml
[tower]
max_floors_chicken_legs = 4
starting_floors = 2

[wall_panel]
base_hp_wood_floor = 80
base_hp_stone_floor = 120
breach_raider_duration = 15.0         # seconds (see implementation-decisions §12)
breach_raider_infrastructure_damage_rate = 1  # destroys 1 transport/cache per 5s inside
breach_repair_auto = false            # breaches persist until player repairs

[foundation]
chicken_legs_hp = 300
chicken_legs_speed = 1.0              # base speed multiplier
chicken_legs_mountain_penalty = 0.5   # 50% speed on mountains
```

---

## 10. Companion Combat Stats

```toml
[companion_combat]
base_accuracy = 0.40                  # new companions (0 encounters)
veteran_accuracy = 0.55               # after 10 encounters
elite_accuracy = 0.70                 # after 20 encounters (post-v1)
fire_rate_multiplier = 0.7            # companions fire at 70% of weapon base rate
ammo_per_shot = 1

# Companion-specific overrides
[companion_combat.kael]
fires_weapon = false                  # Kael doesn't shoot — blocks climbers
block_climber_stun_duration = 2.0     # seconds stunned when blocked

[companion_combat.yuki]
panel_heal_per_hit = 3.0              # HP restored to panel per shot
fires_at_enemies = false              # shots target panels, not enemies

[companion_combat.rust]
weapon = "pistol"
base_damage = 14
fire_rate = 1.8
ammo_type = "gunpowder"

[companion_combat.stone]
weapon = "hammer"
base_damage = 25
fire_rate = 0.8
range = 40                            # melee
ammo_type = "none"
```

---

## 11. Runner AI

Target from [telemetry-balance.md](telemetry-balance.md) §IV: runner utilization 50-70%, queue time <15%, delivery time 8-15s.

```toml
[runners]
base_speed = 50                       # pixels/sec
carry_capacity = 1                    # crates per trip
max_runners_per_quarters = 4
starting_runners = 2

# Priority logic
demand_check_interval = 1.0           # seconds between routing decisions
priority_flag_weight = 3.0            # multiplier for player-flagged resource
empty_rack_weight = 2.0               # high priority for empty racks
full_output_weight = 1.5              # collect from full production buffers
cache_restock_weight = 1.0            # normal priority

# Pathfinding
prefer_chute_over_stairs = true
prefer_dumbwaiter_over_stairs = true
congestion_threshold = 2              # runners at same transport = congested
congestion_reroute = true             # try alternate path when congested
```

---

## 12. Map Generation Parameters

```toml
[map]
# v1: 3 chapters to Harbor
chapters_v1 = 3

[map.chapter_1]
columns = 3
nodes_per_column_min = 1
nodes_per_column_max = 2
total_nodes_min = 4
total_nodes_max = 5
terrain_pool = ["plains", "forest"]

[map.chapter_2]
columns = 4
nodes_per_column_min = 2
nodes_per_column_max = 3
total_nodes_min = 6
total_nodes_max = 8
terrain_pool = ["plains", "forest", "mountain"]

[map.chapter_3]
columns = 5
nodes_per_column_min = 2
nodes_per_column_max = 3
total_nodes_min = 8
total_nodes_max = 10
terrain_pool = ["forest", "mountain", "coast"]
```

---

## 13. Loot Generation

```toml
[loot]
# Drops per encounter
min_drops_difficulty_1 = 0
max_drops_difficulty_1 = 1
min_drops_difficulty_2 = 1
max_drops_difficulty_2 = 2
min_drops_difficulty_3 = 1
max_drops_difficulty_3 = 3
elite_guaranteed_rare = true          # elite encounters drop rare+ minimum
boss_guaranteed_legendary = true

# Trinket drop rate
trinket_drop_chance = 0.15            # 15% chance a drop is a trinket instead of weapon
```

---

## 14. Misc Timing

```toml
[timing]
prep_phase_timeout = 0                # 0 = unlimited (turn-based, no time pressure)
march_transition_duration = 2.0       # seconds of tower walking animation before encounter
post_combat_summary_min_display = 3.0 # seconds before player can dismiss
autosave_trigger = "encounter_boundary"  # save after encounter end + after march
```

---

## Change Log

This config will evolve rapidly during development. When changing a value:
1. Note what changed and why in the commit message
2. Reference the telemetry metric that motivated the change (e.g., "grunt HP 30→25: grunt survival was 4s, target is 1-3s")
3. Test at least 3 encounters before committing the change
