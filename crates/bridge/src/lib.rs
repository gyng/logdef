use std::cell::RefCell;
use wasm_bindgen::prelude::*;

use supply_line_core::command::GameCommand;
use supply_line_core::engine::GameEngine;
use supply_line_core::state::HeroClass;

thread_local! {
    static ENGINE: RefCell<Option<GameEngine>> = const { RefCell::new(None) };
}

fn with_engine<R>(f: impl FnOnce(&GameEngine) -> R) -> R {
    ENGINE.with(|e| f(e.borrow().as_ref().expect("Engine not initialized")))
}

fn with_engine_mut<R>(f: impl FnOnce(&mut GameEngine) -> R) -> R {
    ENGINE.with(|e| f(e.borrow_mut().as_mut().expect("Engine not initialized")))
}

#[wasm_bindgen]
pub fn init_game(seed: u64, class: &str) -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    let hero_class = match class {
        "archer" => HeroClass::Archer,
        "engineer" => HeroClass::Engineer,
        "commander" => HeroClass::Commander,
        _ => return Err(JsValue::from_str("Unknown class")),
    };
    ENGINE.with(|e| {
        *e.borrow_mut() = Some(GameEngine::new(seed, hero_class));
    });
    Ok(())
}

#[wasm_bindgen]
pub fn send_command(json: &str) -> String {
    let cmd: GameCommand = match serde_json::from_str(json) {
        Ok(c) => c,
        Err(e) => return format!(r#"{{"error":"Invalid command: {e}"}}"#),
    };
    let result = with_engine_mut(|e| e.send_command(cmd));
    serde_json::to_string(&result).unwrap_or_else(|e| format!(r#"{{"error":"{e}"}}"#))
}

#[wasm_bindgen]
pub fn tick(real_dt: f32) -> String {
    let sounds = with_engine_mut(|e| e.tick(real_dt));
    serde_json::to_string(&sounds).unwrap_or_else(|_| "[]".into())
}

#[wasm_bindgen]
pub fn interpolation_alpha() -> f32 {
    with_engine(supply_line_core::engine::GameEngine::interpolation_alpha)
}

#[wasm_bindgen]
pub fn get_phase() -> String {
    with_engine(|e| format!("{:?}", e.get_phase()))
}

#[wasm_bindgen]
pub fn get_hud_state() -> String {
    with_engine(|e| serde_json::to_string(&e.get_hud_state()).unwrap_or_default())
}

#[wasm_bindgen]
pub fn get_tower_state() -> String {
    with_engine(|e| serde_json::to_string(&e.get_tower_state()).unwrap_or_default())
}

#[wasm_bindgen]
pub fn get_journey_state() -> String {
    with_engine(|e| serde_json::to_string(&e.get_journey_state()).unwrap_or_default())
}

#[wasm_bindgen]
pub fn get_economy_state() -> String {
    with_engine(|e| serde_json::to_string(&e.get_economy_state()).unwrap_or_default())
}

#[wasm_bindgen]
pub fn get_gold() -> u32 {
    with_engine(supply_line_core::engine::GameEngine::get_gold)
}

#[wasm_bindgen]
pub fn get_hero_state() -> String {
    with_engine(|e| serde_json::to_string(&e.get_hero_state()).unwrap_or_default())
}

#[wasm_bindgen]
pub fn get_encounter_state() -> String {
    with_engine(|e| serde_json::to_string(&e.get_encounter_state()).unwrap_or_default())
}

#[wasm_bindgen]
pub fn get_validation_warnings() -> String {
    with_engine(|e| serde_json::to_string(&e.get_validation_warnings()).unwrap_or_default())
}

#[wasm_bindgen]
pub fn get_perf_state() -> String {
    with_engine(|e| serde_json::to_string(&e.get_perf_state()).unwrap_or_default())
}

#[wasm_bindgen]
pub fn save() -> String {
    with_engine(supply_line_core::engine::GameEngine::save)
}

#[wasm_bindgen]
pub fn load(data: &str) -> Result<(), JsValue> {
    with_engine_mut(|e| e.load(data).map_err(|err| JsValue::from_str(&err)))
}
