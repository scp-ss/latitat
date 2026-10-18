use data::config::Config;
use data::loader::{self, Source};
use godot::classes::{
    Area2D, CharacterBody2D, CollisionShape2D, FileAccess, ICharacterBody2D, Input, Os,
    ProjectSettings, RectangleShape2D,
};
use godot::prelude::*;
use physics::{Env, Vec2 as PVec2};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Reading config.json (next to game.exe, then user://, then inside the .pck)
// ---------------------------------------------------------------------------

/// Folder next to game.exe. In the editor, the Godot project folder.
fn game_dir() -> PathBuf {
    let os = Os::singleton();
    if os.has_feature("editor") {
        PathBuf::from(
            ProjectSettings::singleton()
                .globalize_path("res://")
                .to_string(),
        )
    } else {
        PathBuf::from(os.get_executable_path().to_string())
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_default()
    }
}

fn godot_reader(rel: &str) -> Option<String> {
    if let Ok(text) = std::fs::read_to_string(game_dir().join(rel)) {
        return Some(text);
    }
    for prefix in ["user://", "res://"] {
        let full = format!("{prefix}{rel}");
        if FileAccess::file_exists(full.as_str()) {
            return Some(FileAccess::get_file_as_string(full.as_str()).to_string());
        }
    }
    None
}

// ---------------------------------------------------------------------------
// The player
// ---------------------------------------------------------------------------

#[derive(GodotClass)]
#[class(base = CharacterBody2D)]
pub struct Player {
    /// Check config.json for changes once per second and apply them live.
    #[export]
    hot_reload: bool,
    /// Print speed and how wet the player is twice a second.
    #[export]
    debug_print: bool,
    /// Tick to use the numbers below instead of config.json.
    #[export]
    use_inspector: bool,
    #[export]
    mass: f32,
    #[export]
    move_force: f32,
    #[export]
    swim_force: f32,
    #[export]
    turn_boost: f32,
    #[export]
    ground_friction: f32,
    #[export]
    volume: f32,
    #[export]
    water_density: f32,
    #[export]
    water_drag: f32,
    cfg: Config,
    reload_timer: f64,
    debug_timer: f64,
    base: Base<CharacterBody2D>,
}

#[godot_api]
impl ICharacterBody2D for Player {
    // fn init(base: Base<CharacterBody2D>) -> Self {
    //     Self {
    //         hot_reload: true,
    //         debug_print: false,
    //         // The config that was compiled in (data/default_config.json).
    //         cfg: loader::load_with(|_| None).config,
    //         reload_timer: 0.0,
    //         debug_timer: 0.0,
    //         base,
    //     }
    // }
    fn init(base: Base<CharacterBody2D>) -> Self {
        // The config that was compiled in (data/default_config.json).
        let cfg = loader::load_with(|_| None).config;
        Self {
            hot_reload: true,
            debug_print: false,
            use_inspector: false,
            mass: cfg.player.phys.mass,
            move_force: cfg.player.phys.move_force,
            swim_force: cfg.player.phys.swim_force,
            turn_boost: cfg.player.phys.turn_boost,
            ground_friction: cfg.player.phys.ground_friction,
            volume: cfg.player.utility.volume,
            water_density: cfg.physics.water_density,
            water_drag: cfg.physics.water_drag,
            cfg,
            reload_timer: 0.0,
            debug_timer: 0.0,
            base,
        }
    }
    fn ready(&mut self) {
        self.load_config(false);
    }

    fn physics_process(&mut self, delta: f64) {
        if self.hot_reload {
            self.reload_timer += delta;
            if self.reload_timer >= 1.0 {
                self.reload_timer = 0.0;
                self.load_config(true);
            }
        }

        let dt = delta as f32;
        let ppm = self.cfg.physics.pixels_per_meter.max(1.0);

        // Input: x = right, y = down (so pressing up gives y = -1). Needs these actions in the Input Map.
        let i = Input::singleton().get_vector("move_left", "move_right", "move_up", "move_down");
        let input = PVec2::new(i.x, i.y);

        let submerged = self.submerged_fraction(ppm);
        let grounded = self.base().is_on_floor();

        // Godot pixels/s -> metres/s, run the physics, back to pixels/s.
        let v = self.base().get_velocity() / ppm;
        // let new_v = physics::step(
        //     &self.cfg,
        //     PVec2::new(v.x, v.y),
        //     input,
        //     Env {
        //         submerged,
        //         grounded,
        //     },
        //     dt,
        // );
        let new_v = physics::step(
            &self.cfg,
            PVec2::new(v.x, v.y),
            input,
            Env {
                submerged,
                grounded,
            },
            dt,
        );
        let mut body = self.base_mut();
        body.set_velocity(Vector2::new(new_v.x, new_v.y) * ppm);
        body.move_and_slide();
        // move_and_slide already zeroes the velocity part that hit a wall or floor.
        drop(body);
        //
        // Rmemove the IA features and make ur own with gimpels nad shit and hwanott roboto ic arms nad nwanatot. ill code it brb
        if self.debug_print {
            self.debug_timer += delta;
            if self.debug_timer >= 0.5 {
                self.debug_timer = 0.0;
                let v = self.base().get_velocity() / ppm;
                godot_print!(
                    "speed {:.2} m/s  vel ({:.2}, {:.2})  underwater {:.0}%  grounded {}",
                    v.length(),
                    v.x,
                    v.y,
                    submerged * 100.0,
                    grounded
                );
            }
        }
    }
}

#[godot_api]
impl Player {
    /// Callable from GDScript or a button: player.reload_config()
    #[func]
    fn reload_config(&mut self) {
        self.load_config(false);
    }
    /// The config the physics really uses: the file, or the Inspector numbers if ticked.
    fn active_cfg(&self) -> Config {
        let mut c = self.cfg.clone();
        if self.use_inspector {
            c.player.phys.mass = self.mass;
            c.player.phys.move_force = self.move_force;
            c.player.phys.swim_force = self.swim_force;
            c.player.phys.turn_boost = self.turn_boost;
            c.player.phys.ground_friction = self.ground_friction;
            c.player.utility.volume = self.volume;
            c.physics.water_density = self.water_density;
            c.physics.water_drag = self.water_drag;
        }
        c
    }

    /// Shows the file's numbers in the Inspector sliders as a starting point.
    fn copy_cfg_to_inspector(&mut self) {
        let c = self.cfg.clone();
        self.mass = c.player.phys.mass;
        self.move_force = c.player.phys.move_force;
        self.swim_force = c.player.phys.swim_force;
        self.turn_boost = c.player.phys.turn_boost;
        self.ground_friction = c.player.phys.ground_friction;
        self.volume = c.player.utility.volume;
        self.water_density = c.physics.water_density;
        self.water_drag = c.physics.water_drag;
    }
}

impl Player {
    /// `automatic = true` is the once-per-second poll: it ignores half-written / broken files
    /// instead of falling back to defaults mid-game.
    fn load_config(&mut self, automatic: bool) {
        let loaded = loader::load_with(godot_reader);

        if automatic && !loaded.warnings.is_empty() {
            return;
        }
        for w in &loaded.warnings {
            godot_warn!("{w}");
        }
        if loaded.config == self.cfg {
            return;
        }
        match &loaded.source {
            Source::File(p) => godot_print!("config loaded from {p}"),
            Source::Embedded => godot_print!("config loaded from the compiled-in default"),
            Source::Defaults => godot_warn!("using hardcoded defaults"),
        }
        self.cfg = loaded.config;
        if !self.use_inspector {
            self.copy_cfg_to_inspector();
        }
        self.sync_collision_shape();
    }

    /// Keeps the collision box the same size as height/length in the config.
    /// Only touches RectangleShape2D children. Capsules you size by hand.
    fn sync_collision_shape(&mut self) {
        let ppm = self.cfg.physics.pixels_per_meter;
        let size = Vector2::new(
            self.cfg.player.utility.length,
            self.cfg.player.utility.height,
        ) * ppm;
        for child in self.base().get_children().iter_shared() {
            let Ok(cs) = child.try_cast::<CollisionShape2D>() else {
                continue;
            };
            let Some(shape) = cs.get_shape() else {
                continue;
            };
            if let Ok(mut rect) = shape.try_cast::<RectangleShape2D>() {
                rect.set_size(size);
            }
        }
    }

    /// 0.0 to 1.0: how much of the player's box is inside water.
    /// Water = any Area2D in group "water" that has a RectangleShape2D collision shape.
    fn submerged_fraction(&self, ppm: f32) -> f32 {
        // let Some(mut tree) = self.base().get_tree() else { return 0.0 };
        let mut tree = self.base().get_tree();
        let u = &self.cfg.player.utility;
        let center = self.base().get_global_position() / ppm;
        let body_center = PVec2::new(center.x, center.y);
        let body_size = PVec2::new(u.length, u.height);

        let mut total = 0.0;
        for node in tree.get_nodes_in_group("water").iter_shared() {
            let Ok(area) = node.try_cast::<Area2D>() else {
                continue;
            };
            for child in area.get_children().iter_shared() {
                let Ok(cs) = child.try_cast::<CollisionShape2D>() else {
                    continue;
                };
                let Some(shape) = cs.get_shape() else {
                    continue;
                };
                let Ok(rect) = shape.try_cast::<RectangleShape2D>() else {
                    continue;
                };

                // Box in pixels -> metres. (Rotated water is not supported.)
                let half = rect.get_size() * cs.get_global_scale().abs() * 0.5;
                let c = cs.get_global_position();
                let (min, max) = ((c - half) / ppm, (c + half) / ppm);
                total += physics::overlap_fraction(
                    body_center,
                    body_size,
                    PVec2::new(min.x, min.y),
                    PVec2::new(max.x, max.y),
                );
            }
        }
        total.min(1.0)
    }
}
