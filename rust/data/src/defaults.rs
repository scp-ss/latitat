use crate::config::*;

impl Default for Config {
    fn default() -> Self {
        Self {
            player: Player::default(),
            physics: Physics::default(),
        }
    }
}

impl Default for Player {
    fn default() -> Self {
        Self {
            phys: PlayerPhys::default(),
            utility: PlayerUtility::default(),
        }
    }
}

impl Default for PlayerPhys {
    fn default() -> Self {
        Self {
            mass: 70.0,
            max_speed: 10.0,
            swim_max_speed: 6.0,
            move_force: 500.0,
            swim_force: 900.0,
            turn_boost: 2.0,
            air_control: 0.3,
            ground_friction: 0.3,
        }
    }
}

impl Default for PlayerUtility {
    fn default() -> Self {
        Self {
            health: 100.0,
            surface_area: 0.7,
            height: 1.75,
            length: 0.5,
            volume: 0.085, // 70 kg / 0.085 m^3 = ~824 kg/m^3, so it floats
        }
    }
}

impl Default for Physics {
    fn default() -> Self {
        Self {
            pixels_per_meter: 64.0,
            gravity_on_earth: 9.81,
            air_density: 1.2,
            water_density: 1000.0,
            air_drag: 0.8,
            water_drag: 0.5,
            water_linear_drag: 40.0,
        }
    }
}
