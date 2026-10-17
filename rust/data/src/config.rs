use serde::{Deserialize, Serialize};

/// Everything the game reads from config.json.
/// `#[serde(default)]` = any field missing from the JSON falls back to defaults.rs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub player: Player,
    pub physics: Physics,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Player {
    pub phys: PlayerPhys,
    pub utility: PlayerUtility,
}

/// How the player moves. Forces are in newtons, speeds in m/s, mass in kg.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlayerPhys {
    pub mass: f32,
    /// Self-propulsion speed cap on land / in air (m/s). Gravity can still push you faster.
    pub max_speed: f32,
    /// Same cap while fully underwater (m/s).
    pub swim_max_speed: f32,
    /// Push force when walking on the ground (N). Higher = faster speed-up.
    pub move_force: f32,
    /// Push force while swimming, both axes (N). Must beat (buoyancy - weight) to swim down.
    pub swim_force: f32,
    /// Multiplier on the push force when you press the OPPOSITE way to your velocity.
    /// 1.0 = no extra help, 2.0 = reversing is twice as strong.
    pub turn_boost: f32,
    /// Fraction of move_force you keep while in the air (not grounded, not in water).
    pub air_control: f32,
    /// Ground friction coefficient (mu). Stopping force = mu * normal force.
    pub ground_friction: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlayerUtility {
    pub health: f32,
    /// Cross-section facing the flow, used for drag (m^2).
    pub surface_area: f32,
    /// Body height (m). Also the height of the collision shape / water overlap box.
    pub height: f32,
    /// Body width (m). Spelled "length" to match your old config.
    pub length: f32,
    /// Total body volume (m^3). density = mass / volume. Water is 1000 kg/m^3:
    /// density < 1000 floats, density > 1000 sinks, density == 1000 is neutral.
    pub volume: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Physics {
    /// Metres -> pixels. All physics runs in metres; only the Godot bridge converts.
    pub pixels_per_meter: f32,
    pub gravity_on_earth: f32,
    pub air_density: f32,
    /// kg/m^3. Real water is about 1000 (sea water 1025).
    pub water_density: f32,
    /// Drag coefficient (Cd) in air.
    pub air_drag: f32,
    /// Drag coefficient (Cd) in water.
    pub water_drag: f32,
    /// Extra slow-speed damping in water (N per m/s). Helps things settle instead of bobbing forever.
    pub water_linear_drag: f32,
}

impl Config {
    /// Parse from JSON text. The caller decides how to get the text.
    pub fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}
