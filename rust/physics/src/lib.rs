//! Pure movement physics. No Godot in here, so you can test it with `cargo test`.
//!
//! Units: metres, kilograms, seconds, newtons.
//! Axes: Godot style. +x is right, +y is DOWN. So gravity is +y and buoyancy is -y.

pub use glam::Vec2;
use data::config::Config;

/// What the world tells the body each step.
#[derive(Debug, Clone, Copy, Default)]
pub struct Env {
    /// 0.0 = completely dry, 1.0 = completely underwater.
    pub submerged: f32,
    /// Standing on the floor (from Godot's is_on_floor()).
    pub grounded: bool,
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Advance the velocity by `dt` seconds. Returns the new velocity (m/s).
/// Godot then moves the body and resolves collisions.
pub fn step(cfg: &Config, mut vel: Vec2, input: Vec2, env: Env, dt: f32) -> Vec2 {
    let p = &cfg.player.phys;
    let u = &cfg.player.utility;
    let w = &cfg.physics;

    let m = p.mass.max(0.001);
    let g = w.gravity_on_earth;
    let f = env.submerged.clamp(0.0, 1.0);

    // 1. Normalise the input so diagonals are not faster (analog sticks below 1.0 stay as they are).
    let dir = input.clamp_length_max(1.0);

    // 2. Gravity and buoyancy.
    //    Buoyancy = water density * displaced volume * g, and displaced volume is
    //    (volume * fraction underwater). It cancels gravity exactly at the floating depth.
    let weight = m * g;
    let buoyancy = w.water_density * u.volume * f * g;
    vel.y += (weight - buoyancy) / m * dt;

    // 3. Player thrust.
    //    Horizontal: walking force on land, swim force in water, blended by how wet you are.
    //    Vertical: only works in water (you can't fly), scaled by how wet you are.
    let land_force = p.move_force * if env.grounded { 1.0 } else { p.air_control };
    let h_force = lerp(land_force, p.swim_force, f);
    let v_force = p.swim_force * f;
    let mut thrust = Vec2::new(dir.x * h_force, dir.y * v_force);

    //    Pressing the opposite way to where you're going gets a boost, per axis.
    //    The thrust simply subtracts from the velocity, so you slow down, pass through
    //    zero and start moving the other way. Not an instant flip.
    if dir.x * vel.x < 0.0 {
        thrust.x *= p.turn_boost;
    }
    if dir.y * vel.y < 0.0 {
        thrust.y *= p.turn_boost;
    }

    //    Speed cap on self-propulsion: no thrust once you already move that fast in the pressed direction.
    let max_speed = lerp(p.max_speed, p.swim_max_speed, f);
    if vel.dot(dir.normalize_or_zero()) >= max_speed {
        thrust = Vec2::ZERO;
    }
    vel += thrust / m * dt;

    // 4. Drag. Quadratic (0.5 * density * Cd * area * v^2) plus a small linear part in water.
    //    Capped so a big dt can never flip the velocity.
    let speed = vel.length();
    if speed > 1e-4 {
        let rho = lerp(w.air_density, w.water_density, f);
        let cd = lerp(w.air_drag, w.water_drag, f);
        let linear = w.water_linear_drag * f;
        let drag_force = linear * speed + 0.5 * rho * cd * u.surface_area * speed * speed;
        let dv = drag_force / m * dt;
        vel -= vel / speed * dv.min(speed);
    }

    // 5. Ground friction (Coulomb): mu * normal force, opposing horizontal motion.
    //    Normal force is your apparent weight, so you're slippery when buoyant.
    if env.grounded {
        let normal = (weight - buoyancy).max(0.0);
        let decel = p.ground_friction * normal / m * dt;
        vel.x -= vel.x.signum() * decel.min(vel.x.abs());
    }

    vel
}

/// How much of the body's box is inside one water box. Both boxes in metres.
/// Returns 0.0 to 1.0. For several water areas, add the results and clamp to 1.0.
pub fn overlap_fraction(body_center: Vec2, body_size: Vec2, water_min: Vec2, water_max: Vec2) -> f32 {
    let half = body_size * 0.5;
    let (bmin, bmax) = (body_center - half, body_center + half);
    let ox = (bmax.x.min(water_max.x) - bmin.x.max(water_min.x)).max(0.0);
    let oy = (bmax.y.min(water_max.y) - bmin.y.max(water_min.y)).max(0.0);
    let area = body_size.x * body_size.y;
    if area <= 0.0 {
        return 0.0;
    }
    (ox * oy / area).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    /// Water: x from -100..100, y from 0 (surface) down to 100.
    fn fraction_at(y: f32, cfg: &Config) -> f32 {
        let size = Vec2::new(cfg.player.utility.length, cfg.player.utility.height);
        overlap_fraction(Vec2::new(0.0, y), size, Vec2::new(-100.0, 0.0), Vec2::new(100.0, 100.0))
    }

    /// Simulate the vertical axis with a water surface at y = 0.
    fn run(cfg: &Config, y: &mut f32, vel: &mut Vec2, input: Vec2, seconds: f32) {
        for _ in 0..(seconds / DT) as usize {
            let env = Env { submerged: fraction_at(*y, cfg), grounded: false };
            *vel = step(cfg, *vel, input, env, DT);
            *y += vel.y * DT;
        }
    }

    #[test]
    fn floats_at_the_right_depth() {
        // density 700 -> should sit with 70% of its height underwater.
        let mut cfg = Config::default();
        cfg.player.utility.volume = 0.1;
        let (mut y, mut vel) = (-3.0, Vec2::ZERO);
        run(&cfg, &mut y, &mut vel, Vec2::ZERO, 40.0);
        let f = fraction_at(y, &cfg);
        assert!((f - 0.7).abs() < 0.02, "fraction {f}");
        assert!(vel.length() < 0.01);
    }

    #[test]
    fn dense_body_sinks() {
        let mut cfg = Config::default();
        cfg.player.utility.volume = 0.05; // density 1400
        let (mut y, mut vel) = (0.0, Vec2::ZERO);
        run(&cfg, &mut y, &mut vel, Vec2::ZERO, 5.0);
        assert!(y > 1.0, "y {y}");
    }

    #[test]
    fn neutral_body_stays_put() {
        let mut cfg = Config::default();
        cfg.player.utility.volume = cfg.player.phys.mass / cfg.physics.water_density;
        let (mut y, mut vel) = (5.0, Vec2::ZERO);
        run(&cfg, &mut y, &mut vel, Vec2::ZERO, 10.0);
        assert!((y - 5.0).abs() < 0.01, "y {y}");
    }

    #[test]
    fn can_swim_down_then_gets_pushed_back() {
        let cfg = Config::default(); // density ~824, floats
        let (mut y, mut vel) = (0.0, Vec2::ZERO);
        run(&cfg, &mut y, &mut vel, Vec2::new(0.0, 1.0), 3.0);
        assert!(y > 2.0, "swam down only to {y}");
        // Let go: it should rise again and settle at the floating depth.
        run(&cfg, &mut y, &mut vel, Vec2::ZERO, 30.0);
        let eq = cfg.player.phys.mass / (cfg.physics.water_density * cfg.player.utility.volume);
        let f = fraction_at(y, &cfg);
        assert!((f - eq).abs() < 0.03, "fraction {f}, expected {eq}");
    }

    #[test]
    fn return_to_surface_takes_a_few_seconds() {
        let cfg = Config::default();
        let (mut y, mut vel) = (3.0, Vec2::ZERO); // 3 m deep
        let mut t = 0.0;
        while y > 0.65 && t < 60.0 {
            run(&cfg, &mut y, &mut vel, Vec2::ZERO, DT);
            t += DT;
        }
        println!("2.4 m rise in {t:.1} s");
        assert!(t < 8.0, "took {t} s");
    }

    #[test]
    fn ground_speed_builds_slowly_and_stops_slowly() {
        let cfg = Config::default();
        let env = Env { submerged: 0.0, grounded: true };
        let mut vel = Vec2::ZERO;

        vel = step(&cfg, vel, Vec2::X, env, DT);
        assert!(vel.x > 0.0 && vel.x < 0.5, "first frame {}", vel.x);

        for _ in 0..(3.0 / DT) as usize {
            vel = step(&cfg, vel, Vec2::X, env, DT);
        }
        assert!((vel.x - cfg.player.phys.max_speed).abs() < 0.2, "top speed {}", vel.x);

        // Release: not instant...
        vel = step(&cfg, vel, Vec2::ZERO, env, DT);
        assert!(vel.x > 0.9 * cfg.player.phys.max_speed);
        // ...but it does stop.
        for _ in 0..(5.0 / DT) as usize {
            vel = step(&cfg, vel, Vec2::ZERO, env, DT);
        }
        assert_eq!(vel.x, 0.0);
    }

    #[test]
    fn reversing_slows_then_flips() {
        let cfg = Config::default();
        let env = Env { submerged: 0.0, grounded: true };
        let mut vel = Vec2::new(4.0, 0.0);
        let mut saw_slower = false;
        for _ in 0..(3.0 / DT) as usize {
            let before = vel.x;
            vel = step(&cfg, vel, -Vec2::X, env, DT);
            if vel.x < before && vel.x > 0.0 {
                saw_slower = true;
            }
        }
        assert!(saw_slower);
        assert!(vel.x < -3.0, "ended at {}", vel.x);
    }

    #[test]
    fn diagonal_is_not_faster() {
        let mut cfg = Config::default();
        cfg.physics.gravity_on_earth = 0.0;
        let env = Env { submerged: 1.0, grounded: false };
        let straight = step(&cfg, Vec2::ZERO, Vec2::X, env, DT).length();
        let diagonal = step(&cfg, Vec2::ZERO, Vec2::new(1.0, 1.0), env, DT).length();
        assert!((straight - diagonal).abs() < 1e-5, "{straight} vs {diagonal}");
    }

    #[test]
    fn no_flying_out_of_water() {
        let mut cfg = Config::default();
        cfg.physics.gravity_on_earth = 0.0;
        let env = Env { submerged: 0.0, grounded: false };
        let vel = step(&cfg, Vec2::ZERO, Vec2::new(0.0, -1.0), env, DT);
        assert_eq!(vel.y, 0.0);
    }

    #[test]
    fn overlap_math() {
        let size = Vec2::new(1.0, 2.0);
        let (wmin, wmax) = (Vec2::new(-10.0, 0.0), Vec2::new(10.0, 50.0));
        assert_eq!(overlap_fraction(Vec2::new(0.0, -5.0), size, wmin, wmax), 0.0);
        assert!((overlap_fraction(Vec2::new(0.0, 0.0), size, wmin, wmax) - 0.5).abs() < 1e-6);
        assert_eq!(overlap_fraction(Vec2::new(0.0, 5.0), size, wmin, wmax), 1.0);
    }
}
