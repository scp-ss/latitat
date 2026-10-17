use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub player: Player,
    pub physics: Physics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub phys: PlayerPhys,
    pub utility: PlayerUtility,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerPhys {
    pub speed: f32,
    pub velocity: f32,
    pub max_speed: f32,
    pub mass: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerUtility {
    pub health: f32,
    pub surface_area: f32,
    pub height: f32,
    pub length: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Physics {
    pub gravity_on_earth: f32,
}

impl Config {
    /// Parse from JSON text. The caller decides how to get the text.
    pub fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }

    /// Convenience for normal Rust programs and tests (not for `res://`).
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let text = std::fs::read_to_string(path)?;
        Ok(Self::from_json(&text)?)
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}
