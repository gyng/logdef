use std::borrow::Cow;
use std::collections::BTreeMap;
#[cfg(all(feature = "hot_reload", not(target_arch = "wasm32")))]
use std::fs;
#[cfg(all(feature = "hot_reload", not(target_arch = "wasm32")))]
use std::path::{Path, PathBuf};

use include_dir::{Dir, DirEntry, include_dir};
use ron::de::from_bytes;
use serde::{Deserialize, Serialize};

use crate::state::*;
use crate::types::*;

static EMBEDDED_DATA: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../assets/data");

#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HeroId(pub String);

#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WeaponId(pub String);

#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EnemyDefId(pub String);

#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BuildingDefId(pub String);

#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TrinketId(pub String);

#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModifierId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Registry {
    pub balance: Balance,
    pub heroes: BTreeMap<HeroId, HeroDef>,
    pub weapons: BTreeMap<WeaponId, WeaponDef>,
    pub enemies: BTreeMap<EnemyDefId, EnemyDef>,
    pub buildings: BTreeMap<BuildingDefId, BuildingDef>,
    pub chapters: Vec<ChapterDef>,
    #[serde(default)]
    pub trinkets: BTreeMap<TrinketId, TrinketDef>,
    #[serde(default)]
    pub modifiers: BTreeMap<ModifierId, ModifierDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Balance {
    pub economy: EconomyBalance,
    pub construction: ConstructionBalance,
    pub production: ProductionBalance,
    pub combat: CombatBalance,
    pub hero: HeroBalance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EconomyBalance {
    pub starting_gold: u32,
    pub starting_wood: u32,
    pub starting_stone: u32,
    pub chapter_ticks: [u32; 3],
    pub unused_tick_gold: u32,
    pub encounter_completion_bonus: u32,
    pub hero_kill_multiplier: Scalar,
    pub runner_salary: u32,
    pub companion_wage: u32,
    pub leg_maintenance: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConstructionBalance {
    pub floor_tick_cost: u32,
    pub wood_floor_material_cost: u32,
    pub stone_floor_material_cost: u32,
    pub wood_panel_hp: Scalar,
    pub stone_panel_hp: Scalar,
    pub foundation_hp: Scalar,
    pub max_floors: usize,
    pub building_tick_cost: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductionBalance {
    pub fletcher_rate: Scalar,
    pub fletcher_operating_cost: u32,
    pub fletcher_buffer_max: u32,
    pub quarry_rate: Scalar,
    pub quarry_operating_cost: u32,
    pub quarry_buffer_max: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CombatBalance {
    pub battlefield_width: Scalar,
    pub wave_delay: Scalar,
    pub breach_duration: Scalar,
    pub difficulty_budgets: [u32; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeroBalance {
    pub personal_ammo: u32,
    pub ammo_rack_max: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeroDef {
    pub id: HeroId,
    pub class: HeroClass,
    pub starting_weapon_primary: WeaponId,
    pub starting_weapon_secondary: WeaponId,
    pub starting_stats: HeroStats,
    pub skill: HeroSkill,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeaponDef {
    pub id: WeaponId,
    pub base_type: WeaponBaseType,
    pub sub_type: String,
    pub damage: Scalar,
    pub fire_rate: Scalar,
    pub range: Scalar,
    pub projectile_speed: Option<Scalar>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnemyDef {
    pub id: EnemyDefId,
    pub archetype: EnemyArchetype,
    pub hp: Scalar,
    pub speed: Scalar,
    pub damage: Scalar,
    pub attack_rate: Scalar,
    pub bounty: u32,
    pub threat: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrinketDef {
    pub id: TrinketId,
    pub name: String,
    pub category: TrinketCategory,
    pub description: String,
    #[serde(default)]
    pub effect: TrinketEffect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrinketCategory {
    Combat,
    Logistics,
    Defensive,
    Wild,
}

/// Typed trinket effects. Runtime implementations consume these by matching
/// the variant; unimplemented variants are no-ops until coded.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum TrinketEffect {
    #[default]
    None,
    LastArrowBonus {
        damage_multiplier: Scalar,
    },
    RicochetOnMiss {
        chance: Scalar,
        damage_fraction: Scalar,
    },
    BonusProjectileEveryNth {
        interval: u32,
    },
    LowPanelRageBoost {
        hp_threshold: Scalar,
        fire_rate_bonus: Scalar,
        damage_bonus: Scalar,
    },
    LeechAmmoFromKills,
    EmergencyCrate {
        charges: u32,
    },
    LootMagnet {
        radius_floors: u32,
        speed_multiplier: Scalar,
    },
    PhaseShotsThroughPanels {
        shot_count: u32,
    },
    DecoyPhantom,
    FasterBreachExpel {
        seconds: Scalar,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModifierDef {
    pub id: ModifierId,
    pub name: String,
    pub category: ModifierCategory,
    pub description: String,
    #[serde(default)]
    pub effect: ModifierEffect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModifierCategory {
    Combat,
    Economy,
    Utility,
    Drawback,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum ModifierEffect {
    #[default]
    None,
    Flaming {
        damage_bonus: Scalar,
        dot_per_sec: Scalar,
        dot_duration: Scalar,
    },
    Frost {
        slow_fraction: Scalar,
        duration: Scalar,
    },
    Explosive {
        radius: Scalar,
        splash_fraction: Scalar,
    },
    Piercing {
        armor_pierce_fraction: Scalar,
    },
    Vampiric {
        heal_fraction: Scalar,
    },
    Venomous {
        dot_per_sec: Scalar,
        duration: Scalar,
    },
    Efficient {
        chance_no_ammo: Scalar,
    },
    Gilded {
        bonus_gold_per_kill: u32,
    },
    Scavenging {
        loot_speed_multiplier: Scalar,
    },
    Silent,
    Beacon {
        vuln_bonus: Scalar,
        duration: Scalar,
    },
    Magnetic {
        radius_floors: u32,
    },
    Cursed {
        damage_bonus: Scalar,
        accuracy_penalty: Scalar,
    },
    Heavy {
        damage_bonus: Scalar,
        fire_rate_penalty: Scalar,
    },
    Fragile {
        fire_rate_bonus: Scalar,
        encounters_until_break: u32,
    },
    Bloodthirsty {
        damage_per_kill: Scalar,
        ammo_penalty: u32,
    },
}

fn default_building_width_slots() -> u8 {
    2
}

/// One input requirement on a crafting building. Each production
/// cycle consumes `amount_per_craft` of `resource` from a buffer of
/// size `buffer_max`. Raw producers leave this list empty.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildingInputDef {
    pub resource: ResourceType,
    pub amount_per_craft: u32,
    pub buffer_max: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildingDef {
    pub id: BuildingDefId,
    pub building_type: BuildingType,
    pub tier: ProductionTier,
    pub output_resource: ResourceType,
    pub production_rate: Scalar,
    pub operating_cost: u32,
    pub output_buffer_max: u32,
    /// Number of slots the building occupies horizontally on its floor.
    /// T1 buildings ship at 2 slots, T2 at 3 slots.
    #[serde(default = "default_building_width_slots")]
    pub width_slots: u8,
    /// Inputs the building consumes per production cycle. Empty for
    /// raw producers (Lumberyard, Quarry); populated for crafters
    /// (Fletcher consumes wood, Forge consumes stone, etc.).
    #[serde(default)]
    pub inputs: Vec<BuildingInputDef>,
    pub build_tick_cost: u32,
    pub build_resource: ResourceType,
    pub build_resource_cost: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChapterDef {
    pub id: String,
    pub name: String,
    pub nodes: Vec<MapNode>,
    pub edges: Vec<MapEdge>,
    pub boss_node: NodeId,
    pub enemy_pool: Vec<EnemyDefId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadError {
    pub path: String,
    pub message: String,
}

pub trait DataSource {
    fn read(&self, path: &str) -> Result<Cow<'_, [u8]>, LoadError>;
    fn list(&self, prefix: &str) -> Vec<String>;
}

pub struct EmbeddedSource;

impl DataSource for EmbeddedSource {
    fn read(&self, path: &str) -> Result<Cow<'_, [u8]>, LoadError> {
        let file = EMBEDDED_DATA.get_file(path).ok_or_else(|| LoadError {
            path: path.into(),
            message: "embedded file missing".into(),
        })?;
        Ok(Cow::Borrowed(file.contents()))
    }

    fn list(&self, prefix: &str) -> Vec<String> {
        let mut files = Vec::new();
        collect_embedded_paths(&EMBEDDED_DATA, prefix, &mut files);
        files.sort();
        files
    }
}

#[cfg(all(feature = "hot_reload", not(target_arch = "wasm32")))]
pub struct DiskSource {
    root: PathBuf,
}

#[cfg(all(feature = "hot_reload", not(target_arch = "wasm32")))]
impl DiskSource {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

#[cfg(all(feature = "hot_reload", not(target_arch = "wasm32")))]
impl DataSource for DiskSource {
    fn read(&self, path: &str) -> Result<Cow<'_, [u8]>, LoadError> {
        let full_path = self.root.join(path);
        fs::read(&full_path)
            .map(Cow::Owned)
            .map_err(|err| LoadError {
                path: full_path.display().to_string(),
                message: err.to_string(),
            })
    }

    fn list(&self, prefix: &str) -> Vec<String> {
        let mut files = Vec::new();
        collect_disk_paths(&self.root.join(prefix), &self.root, &mut files);
        files.sort();
        files
    }
}

impl Registry {
    pub fn load(source: &dyn DataSource) -> Result<Self, Vec<LoadError>> {
        let mut errors = Vec::new();

        let balance = parse_one::<Balance>(source, "balance.ron", &mut errors);
        let heroes = parse_keyed_dir::<HeroId, HeroDef>(source, "entities/heroes", &mut errors);
        let weapons =
            parse_keyed_dir::<WeaponId, WeaponDef>(source, "entities/weapons", &mut errors);
        let enemies =
            parse_keyed_dir::<EnemyDefId, EnemyDef>(source, "entities/enemies", &mut errors);
        let buildings = parse_keyed_dir::<BuildingDefId, BuildingDef>(
            source,
            "entities/buildings",
            &mut errors,
        );
        let chapters = parse_vec_dir::<ChapterDef>(source, "chapters", &mut errors);
        let trinkets =
            parse_keyed_dir::<TrinketId, TrinketDef>(source, "entities/trinkets", &mut errors);
        let modifiers =
            parse_keyed_dir::<ModifierId, ModifierDef>(source, "entities/modifiers", &mut errors);

        if !errors.is_empty() {
            return Err(errors);
        }

        let registry = Self {
            balance: balance.expect("balance should exist when errors are empty"),
            heroes,
            weapons,
            enemies,
            buildings,
            chapters,
            trinkets,
            modifiers,
        };

        validate(&registry).map(|_| registry)
    }

    pub fn load_embedded() -> Result<Self, Vec<LoadError>> {
        Self::load(&EmbeddedSource)
    }

    pub fn hero_for_class(&self, class: HeroClass) -> Option<&HeroDef> {
        self.heroes.values().find(|hero| hero.class == class)
    }

    pub fn weapon(&self, id: &WeaponId) -> Option<&WeaponDef> {
        self.weapons.get(id)
    }

    pub fn weapon_by_sub_type(&self, sub_type: &str) -> Option<&WeaponDef> {
        self.weapons
            .values()
            .find(|weapon| weapon.sub_type == sub_type)
    }

    pub fn enemy_by_archetype(&self, archetype: EnemyArchetype) -> Option<&EnemyDef> {
        self.enemies
            .values()
            .find(|enemy| enemy.archetype == archetype)
    }

    pub fn building_by_type(&self, building_type: BuildingType) -> Option<&BuildingDef> {
        self.buildings
            .values()
            .find(|building| building.building_type == building_type)
    }
}

pub trait RegistryItem<K: Ord + Clone> {
    fn id(&self) -> K;
}

impl RegistryItem<HeroId> for HeroDef {
    fn id(&self) -> HeroId {
        self.id.clone()
    }
}

impl RegistryItem<WeaponId> for WeaponDef {
    fn id(&self) -> WeaponId {
        self.id.clone()
    }
}

impl RegistryItem<EnemyDefId> for EnemyDef {
    fn id(&self) -> EnemyDefId {
        self.id.clone()
    }
}

impl RegistryItem<BuildingDefId> for BuildingDef {
    fn id(&self) -> BuildingDefId {
        self.id.clone()
    }
}

impl RegistryItem<TrinketId> for TrinketDef {
    fn id(&self) -> TrinketId {
        self.id.clone()
    }
}

impl RegistryItem<ModifierId> for ModifierDef {
    fn id(&self) -> ModifierId {
        self.id.clone()
    }
}

fn collect_embedded_paths(dir: &Dir<'_>, prefix: &str, out: &mut Vec<String>) {
    for entry in dir.entries() {
        match entry {
            DirEntry::Dir(child) => collect_embedded_paths(child, prefix, out),
            DirEntry::File(file) => {
                let path = file.path().to_string_lossy().replace('\\', "/");
                if path.starts_with(prefix) && path.ends_with(".ron") {
                    out.push(path);
                }
            }
        }
    }
}

#[cfg(all(feature = "hot_reload", not(target_arch = "wasm32")))]
fn collect_disk_paths(dir: &Path, root: &Path, out: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_disk_paths(&path, root, out);
        } else if path.extension().is_some_and(|ext| ext == "ron")
            && let Ok(relative) = path.strip_prefix(root)
        {
            out.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
}

fn parse_one<T>(source: &dyn DataSource, path: &str, errors: &mut Vec<LoadError>) -> Option<T>
where
    T: for<'de> Deserialize<'de>,
{
    let bytes = match source.read(path) {
        Ok(bytes) => bytes,
        Err(err) => {
            errors.push(err);
            return None;
        }
    };

    match from_bytes::<T>(&bytes) {
        Ok(value) => Some(value),
        Err(err) => {
            errors.push(LoadError {
                path: path.into(),
                message: err.to_string(),
            });
            None
        }
    }
}

fn parse_keyed_dir<K, T>(
    source: &dyn DataSource,
    prefix: &str,
    errors: &mut Vec<LoadError>,
) -> BTreeMap<K, T>
where
    K: Ord + Clone,
    T: for<'de> Deserialize<'de> + RegistryItem<K>,
{
    let mut items = BTreeMap::new();
    for path in source.list(prefix) {
        if let Some(item) = parse_one::<T>(source, &path, errors) {
            items.insert(item.id(), item);
        }
    }
    items
}

fn parse_vec_dir<T>(source: &dyn DataSource, prefix: &str, errors: &mut Vec<LoadError>) -> Vec<T>
where
    T: for<'de> Deserialize<'de>,
{
    let mut items = Vec::new();
    for path in source.list(prefix) {
        if let Some(item) = parse_one::<T>(source, &path, errors) {
            items.push(item);
        }
    }
    items
}

fn validate(registry: &Registry) -> Result<(), Vec<LoadError>> {
    let mut errors = Vec::new();

    for hero in registry.heroes.values() {
        if !registry.weapons.contains_key(&hero.starting_weapon_primary) {
            errors.push(LoadError {
                path: format!("hero {}", hero.id.0),
                message: format!(
                    "unknown starting_weapon_primary {}",
                    hero.starting_weapon_primary.0
                ),
            });
        }
        if !registry
            .weapons
            .contains_key(&hero.starting_weapon_secondary)
        {
            errors.push(LoadError {
                path: format!("hero {}", hero.id.0),
                message: format!(
                    "unknown starting_weapon_secondary {}",
                    hero.starting_weapon_secondary.0
                ),
            });
        }
    }

    for chapter in &registry.chapters {
        for enemy_id in &chapter.enemy_pool {
            if !registry.enemies.contains_key(enemy_id) {
                errors.push(LoadError {
                    path: chapter.id.clone(),
                    message: format!("unknown enemy in pool {}", enemy_id.0),
                });
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::Registry;

    #[test]
    fn data_files_load() {
        let registry = Registry::load_embedded().expect("embedded data should load");
        assert!(!registry.heroes.is_empty());
        assert!(!registry.weapons.is_empty());
        assert!(!registry.enemies.is_empty());
        assert!(!registry.buildings.is_empty());
        assert!(!registry.chapters.is_empty());
    }
}
