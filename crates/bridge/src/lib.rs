use wasm_bindgen::prelude::*;

use supply_line_core::command::GameCommand;
use supply_line_core::engine::GameEngine;
use supply_line_core::state::HeroClass;

/// WASM bridge — the single entry point between JS/React and Rust.
/// React calls these functions; Rust owns all game state.
#[wasm_bindgen]
pub struct Bridge {
    engine: GameEngine,
}

#[wasm_bindgen]
impl Bridge {
    #[wasm_bindgen(constructor)]
    pub fn new(seed: u64, class: &str) -> Result<Bridge, JsValue> {
        let hero_class = match class {
            "archer" => HeroClass::Archer,
            "engineer" => HeroClass::Engineer,
            "commander" => HeroClass::Commander,
            _ => return Err(JsValue::from_str("Unknown class")),
        };
        Ok(Bridge {
            engine: GameEngine::new(seed, hero_class),
        })
    }

    /// Send a JSON-encoded GameCommand. Returns JSON result.
    pub fn send_command(&mut self, json: &str) -> String {
        let cmd: GameCommand = match serde_json::from_str(json) {
            Ok(c) => c,
            Err(e) => return format!(r#"{{"error":"Invalid command: {e}"}}"#),
        };
        let result = self.engine.send_command(cmd);
        serde_json::to_string(&result).unwrap_or_else(|e| format!(r#"{{"error":"{e}"}}"#))
    }

    /// Advance simulation by real_dt seconds. Returns JSON array of SoundEvents.
    pub fn tick(&mut self, real_dt: f32) -> String {
        let sounds = self.engine.tick(real_dt);
        serde_json::to_string(&sounds).unwrap_or_else(|_| "[]".into())
    }

    /// Interpolation alpha for smooth rendering between sim ticks.
    pub fn interpolation_alpha(&self) -> f32 {
        self.engine.interpolation_alpha()
    }

    /// Compact HUD state for the combat overlay (call once per sim tick).
    pub fn get_hud_state(&self) -> String {
        serde_json::to_string(&self.engine.get_hud_state()).unwrap_or_default()
    }

    /// Tower layout for the prep UI.
    pub fn get_tower_state(&self) -> String {
        serde_json::to_string(&self.engine.get_tower_state()).unwrap_or_default()
    }

    /// Journey/map state.
    pub fn get_journey_state(&self) -> String {
        serde_json::to_string(&self.engine.get_journey_state()).unwrap_or_default()
    }

    /// Hero stats/equipment.
    pub fn get_hero_state(&self) -> String {
        serde_json::to_string(&self.engine.get_hero_state()).unwrap_or_default()
    }

    /// Pre-march validation warnings.
    pub fn get_validation_warnings(&self) -> String {
        serde_json::to_string(&self.engine.get_validation_warnings()).unwrap_or_default()
    }

    /// Serialize full GameState for save.
    pub fn save(&self) -> String {
        self.engine.save()
    }

    /// Load a previously saved GameState.
    pub fn load(&mut self, data: &str) -> Result<(), JsValue> {
        self.engine.load(data).map_err(|e| JsValue::from_str(&e))
    }
}
