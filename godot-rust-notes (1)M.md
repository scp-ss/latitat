# Points to note
> + This is AI generated. 
> + Thse are tips for me to write goodcode. 
> + The entire projecti s made by hadn, No code was written by AI 
 


<br><br> 

# Godot + Rust Notes (refined, Oct 2026)

Covers: Rust vs GDScript, how to organize code, the no-recompile tuning workflow, tips for godot-rust, Godot and Rust, engine callbacks vs custom API, signals, and a buoyancy + exported-params worked example.

---

## 1. Rust vs GDScript: what goes where

**One-line rule:** Rust for heavy, complex, or reusable logic. GDScript for glue, wiring, and things you change every day.

### Use Rust when

- It runs every frame over **many** things (hundreds to thousands+): enemies, particles, fluid cells, chunks, limbs
- It's an algorithm: pathfinding, procedural generation (seed_gen), physics math, noise, meshing
- It has complex state where bugs are expensive (inventory, crafting rules, injury/joint-chain model)
- It's parsing, saving, loading, or serializing data (serde)
- You want to unit-test it without launching Godot
- You want to **reuse it in your own ash engine later** (math, seed_gen, physics should never need Godot)

### Use GDScript when

- Wiring scenes and signals together: "button pressed, open the menu"
- UI logic, menus, HUD updates
- Cutscenes, triggers, one-off level scripts
- Prototyping an idea fast (no compile wait)
- Anything that will change 20 times this week and doesn't run in a hot loop

### Honest notes

- GDScript isn't slow for small things. It's slow for **big loops of work**. Typed GDScript (`var x: int`) is noticeably faster than untyped.
- Rust's win comes from doing **a lot of work per call**. A single Godot call from Rust costs about the same as from GDScript. The speedup comes from your math and loops running natively.
- Optimization order: **algorithm, then data layout, then batching, then language.** A bad algorithm in Rust loses to a good one in GDScript.
- Don't write the same system twice. Prototype in GDScript if you must, **profile**, then port only the part that's actually hot.
- Don't write 10-line scripts in Rust. The compile + reload cost isn't worth it.

### Quick decision flow

1. Does it loop over many items per frame? **Rust**
2. Is it pure logic/algorithm with no Godot types needed? **Plain Rust crate**
3. Is it complex and bug-prone? **Rust**
4. Is it mostly touching nodes, UI, and signals? **GDScript** (or a thin Rust class)
5. Will you tweak it daily? **GDScript until it settles**, then decide

### Best pattern for mixing both

Rust classes expose `#[func]`s, `#[signal]`s, and `#[export]`s. GDScript scene scripts connect to them and do the glue. Godot sees them as normal nodes either way.

---

## 2. Code organization: file, module, or crate?

Think of it as a ladder. Climb only when you have a reason.

| Step | Use it when |
|---|---|
| **Add a function to the current file** | It's small and closely related to what's already there |
| **New `.rs` file + `mod`** | File is getting past roughly 300 to 500 lines, or it's a distinct concept (e.g. `inventory.rs`) |
| **Folder module** (`player/mod.rs` or `player.rs` + `player/`) | One concept has several files (movement, injuries, input) |
| **New crate in the workspace** | See below |

### Make a new crate when

- It has **no Godot dependency** and you want to test or reuse it (math, seed_gen, physics)
- It will also be used by your **ash renderer/engine**
- You want to **enforce a dependency direction**: a pure crate physically can't import `godot`
- It's big and independent enough that separate compile caching helps rebuild times
- It's a real boundary (mod-handler, data definitions)

### Don't make a new crate when

- It's under a few hundred lines
- It's tightly coupled to something else (you'd end up with circular dependency problems)
- You'd name it `utils` or `common`. Junk-drawer crates grow forever. Name crates by purpose.

### What belongs in `lib.rs`

Keep it **tiny**:

```rust
mod player;
mod enemies;
mod world;

use godot::prelude::*;

struct MyExtension;

#[gdextension]
unsafe impl ExtensionLibrary for MyExtension {}
```

Only `mod` declarations, `pub use` re-exports, and (in the bridge crate) the extension entry. Real code lives in modules.

### Suggested workspace shape for your project

```
              godot-bridge   (cdylib, the ONLY crate that depends on `godot`)
              /    |    \     \
         player  world  mod-handler  ...   (game logic, still pure Rust)
             \    |    /
        physics  seed_gen  data
                \   |   /
                  math
        renderer (ash)  -> also depends only on math (+ data), never on godot
```

Rules:

- Arrows point **downward toward pure crates**. Nothing pure ever depends on `godot-bridge`.
- Only `godot-bridge` is `crate-type = ["cdylib"]`. Everything else is a normal library crate.
- Convert between Godot types and your own types (like `Vector2` and your math types) at the **bridge**, with small `From` impls.
- Use snake_case crate names (`seed_gen`, not `seedGen`). That's the Cargo convention.
- Use `pub(crate)` for things only your crate needs. Keep the public API small.
- Use traits as boundaries: a pure crate can define `trait WorldQuery`, and the bridge implements it with Godot data.
- Rough sweet spot: 5 to 10 crates. Beyond that, the overhead starts to outweigh the benefit.

### Why not one `.gdextension` per crate?

Each cdylib gets its own copy of the godot-rust runtime, so they can't share `Gd<T>` types in Rust and would only talk through Godot. It also means more files, more export configs, and more version mismatch risk. Use **one** bridge cdylib. Split into separate extensions only for truly standalone, reusable addons.

---

## 3. The no-recompile tuning workflow

### Option A: `#[export]` for quick numbers

```rust
#[export(range = (0.0, 1000.0, 1.0))]
speed: f32,
```

- Group related fields with `#[export_group(name = "Movement")]`
- Edit live while the game runs via the **Remote** scene tree tab

### Option B: custom Resource (best for tuning by feel)

```rust
#[derive(GodotClass)]
#[class(base=Resource, init)]
struct PlayerTuning {
    #[export(range = (0.0, 1000.0, 1.0))]
    #[init(val = 200.0)]
    speed: f32,
    #[export]
    #[init(val = 400.0)]
    max_speed: f32,
    base: Base<Resource>,
}

#[derive(GodotClass)]
#[class(base=CharacterBody2D)]
struct Player {
    #[export]
    tuning: Option<Gd<PlayerTuning>>,
    base: Base<CharacterBody2D>,
}
```

Save it as a `.tres`, assign it in the inspector, and read values with `t.bind().speed`. You can share one across many entities and keep several variants (`player_fast.tres`, `player_heavy.tres`) to compare.

### Option C: JSON loaded at runtime (best for bulk data: items, recipes, events)

```rust
fn load_consts() -> Option<PlayerConstants> {
    let f = FileAccess::open("res://data/player_constants.json", ModeFlags::READ)?;
    serde_json::from_str(&f.get_as_text().to_string()).ok()
}
```

- Bind a debug key (`reload_consts` in the Input Map) that calls this again, so you edit JSON, press the key, and see the change
- `#[serde(default)]` on the struct means adding a new field won't break old files
- `#[serde(deny_unknown_fields)]` catches typos in key names
- Consider **RON** or **TOML** instead of JSON for hand-edited tuning files. They allow comments. JSON doesn't.
- For export builds, add `data/*.json` to the export preset's non-resource files filter, or the files won't ship

### The gotcha: don't mix sources for the same value

Values stored in a scene **override** what `init()` set. If you load from JSON in `init()` and also `#[export]` the same field, the scene's saved value silently wins and your JSON edits seem to do nothing. Pick **one** source per value.

### Refined version of the constants loader

Your `RConst` with `paths: Option<Vec<String>>` and "index 0 = player" works, but it's fragile and re-parses every call. Cleaner:

```rust
use serde::Deserialize;
use std::sync::LazyLock;

#[derive(Deserialize, Debug, Clone)]
pub struct PlayerConstants {
    pub init_velocity: f32,
    pub init_max_velocity: f32,
    pub init_x: f32,
}

// Parsed once, on first use, then shared everywhere.
pub static PLAYER_DEFAULTS: LazyLock<PlayerConstants> = LazyLock::new(|| {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/data/player_constants.json"
    )))
    .expect("invalid player_constants.json")
});
```

Use it anywhere with `PLAYER_DEFAULTS.init_velocity`. Named statics beat magic index numbers, and `LazyLock` (stable since Rust 1.80) parses only once.

Remember: `include_str!` bakes the file in at compile time, so use it for **defaults/fallbacks**. For tuning, load at runtime (Option C).

Also: `env!("CARGO_MANIFEST_DIR")` points to the **individual crate's** folder, not the workspace root. If the data is in the workspace root, use `"/../data/..."`.

### Cut compile-time friction

- Use the **mold** or **lld** linker. In `.cargo/config.toml`:
  ```toml
  [target.x86_64-unknown-linux-gnu]
  rustflags = ["-C", "link-arg=-fuse-ld=mold"]
  ```
- `cargo watch -x build` rebuilds on save so hot reload picks it up
- Dev profile: your crates at `opt-level = 1`, dependencies at `opt-level = 3`
  ```toml
  [profile.dev]
  opt-level = 1
  [profile.dev.package."*"]
  opt-level = 3
  ```
- Use `cargo check` while writing and `cargo test` on pure crates (no Godot launch needed)
- Keep important state in exported or `#[var]` fields. Hot reload preserves those, but other Rust-side state may reset.

---

## 4. godot-rust tips

- **`real` type:** Godot can be built with single (`f32`, default) or double (`f64`) precision. `real` follows whichever the build uses, so Godot math types (`Vector2`, `Vector3`) work either way. For your own pure data, plain `f32` is fine.
- **Reentrancy panics:** if `&mut self` is borrowed and Godot calls back into the same object (e.g. a signal handler), you get an "already borrowed" panic. Emit signals at the end of a method, use `call_deferred`, or drop borrows before calling out.
- **`OnReady`:** `#[init(node = "Path/To/Node")] sprite: OnReady<Gd<Sprite2D>>` replaces manual `get_node` in `ready()`.
- **Exported enums:** derive `GodotConvert`, `Var`, `Export` with `#[godot(via = i64)]` and you get an inspector dropdown.
- **Bulk data:** `PackedVector2Array` and other Packed arrays cross the boundary in one call and can be read as slices.
- **Threads:** Godot objects aren't `Send`. Do heavy work in pure Rust threads (rayon/std) on plain data, then hand results back on the main thread.
- **Freeing:** `Node`s need `queue_free()`. `RefCounted` types free themselves. Check `is_instance_valid()` before using a stored `Gd` that something else might have freed.
- **`#[class(tool)]`:** runs your code in the editor. Powerful, but a bug there can crash the editor. Use sparingly.
- **Prefix your class names** (`GamePlayer`, not `Player`) since class names are global in Godot.
- **Pin the Godot version** with the `api-4-x` feature so updates don't surprise you.
- **Avoid string-based calls** (`call("name")`, `get("prop")`) in hot paths. Use typed methods and cache `Gd` references. Prefer `StringName` over `GString` for names.
- **Avoid `unwrap()` in per-frame code** in the bridge. gdext catches panics, but a panic every frame spams errors.
- **Debugging:** `RUST_BACKTRACE=1`, `godot_print!`, `godot_warn!`, `godot_error!`, and attach lldb/gdb for breakpoints. `cargo doc --open` gives offline API docs.

---

## 5. General Godot tips

- **Signals up, calls down.** Parents call children's methods. Children emit signals.
- Physics in `physics_process`, visuals in `process`.
- Use **collision layers and masks** instead of checking types in code. Use **groups** for "all enemies" style queries.
- Use the **Input Map**, never hardcoded keys.
- Check **Debugger > Monitors** and the profiler regularly. Turn on "Visible Collision Shapes" while testing.
- For huge counts, use `MultiMeshInstance` or the `RenderingServer` / `PhysicsServer` APIs instead of thousands of nodes.
- Keep scenes small and composable. Use Resources for data.
- Object pooling for bullets, enemies, particles.
- Use atlases, simple collision shapes, occlusion culling (3D), and baked lighting.
- Git ignore `.godot/` and `target/`.

---

## 6. Rust tips for game dev

- **Deterministic seeds:** for seed_gen, use `rand_chacha` (`ChaCha8Rng`). `StdRng` and `SmallRng` aren't guaranteed stable across crate versions, so the same seed could generate a different world after an update.
- **Indices over references:** store entities in a `Vec` and refer to them by index or generational ID (`slotmap` crate). This avoids borrow-checker fights.
- **Struct of arrays** for hot data: `positions: Vec<Vec2>`, `velocities: Vec<Vec2>` instead of `Vec<Entity>`. It's cache-friendly and easy to iterate.
- **Split borrows:** if the borrow checker complains about `&mut self`, pass individual fields to helper functions instead of the whole struct.
- **State machines as enums with data:** `enum State { Idle, Swimming { drag: f32 }, Hurt { timer: f32 } }`, matched exhaustively.
- `Vec::with_capacity` and reusing buffers across frames instead of allocating each tick.
- `[workspace.dependencies]` in the root `Cargo.toml` so all crates share dependency versions.
- Errors: `thiserror` for your own error types. `expect("clear message")` for bundled files that should never be bad. `Result` and `?` for anything loaded at runtime.
- `debug_assert!` for checks you want in dev but free in release.
- Tools: `cargo clippy` (it teaches idioms, so read its messages), `cargo fmt`, `cargo flamegraph`, `criterion` for benchmarks.
- **Profile before optimizing.** Guessing what's slow is usually wrong.

---

## 7. Shipping

- Build the Rust lib in **release** mode for each target platform and list each one in the `.gdextension` file.
- Keep separate debug and release paths in the `.gdextension` file so editor runs compile fast.
- Release profile:
  ```toml
  [profile.release]
  opt-level = 3
  lto = "thin"
  codegen-units = 1
  ```
- Leave `panic` as the default (`unwind`) so gdext can catch panics instead of hard-crashing.
- Always test an **exported build**, not just the editor. Missing libs and missing JSON files only show up there.
- Windows from Linux: `x86_64-pc-windows-gnu` with mingw, or `cargo-zigbuild`. Android needs the NDK. Web export is the hardest, so avoid it early.

---

## 8. Ash side notes

- Keep the renderer crate free of `godot`. Share `math` (and plain data types) with it.
- Always enable the **Vulkan validation layers** while developing.
- Learn in order: instance, device, surface, swapchain, pipeline, command buffers, synchronization, memory/buffers, vertex data, uniforms, textures, depth.
- Don't let the ash project eat the time you need to ship the Godot game. Treat them as two tracks and time-box each.

---

## 9. Engine callbacks vs your own API (the two impl blocks)

You'll see two kinds of `#[godot_api]` blocks. The difference is **which trait you're in and who calls the function**.

### `impl ISprite2D for Player` (engine virtual methods)

- A **trait impl** (note the `for Player`).
- `ISprite2D` is generated by godot-rust and lists what the engine itself calls: `init`, `ready`, `process`, `physics_process`, `input`, and so on. These are the equivalents of GDScript's `_init()`, `_ready()`, `_process()`.
- Names are fixed by the trait. Godot calls them automatically. **No `#[func]` needed.**

### `impl Player` (your own API)

- An **inherent impl** (no trait, no `for`).
- Here you write your own functions and signals. Nothing calls them automatically. You, GDScript, or a signal does.
- `#[func]` makes a function callable from GDScript and the editor (signal connections, etc.).
- `#[signal]` declares a signal.

### Side by side

| | `impl ISprite2D for Player` | `impl Player` |
|---|---|---|
| Purpose | Override engine callbacks | Add your own functions/signals |
| Names | Fixed by the trait | Anything you want |
| Called by | Godot engine | You / GDScript / signals |
| Needs `#[func]` | No | Yes, if Godot should see it |

Both blocks need `#[godot_api]`, because that macro is what connects the block to Godot. A plain `impl Player` **without** `#[godot_api]` and `#[func]` works only inside Rust. Godot can't see it.

### Example

```rust
#[godot_api]
impl ISprite2D for Player {
    fn init(base: Base<Sprite2D>) -> Self {
        Self { speed: 400.0, angular_speed: std::f64::consts::PI, base }
    }

    // Godot calls this every physics tick
    fn physics_process(&mut self, delta: f64) {
        let radians = (self.angular_speed * delta) as f32;
        self.base_mut().rotate(radians);   // base class methods go through base_mut()
    }
}

#[godot_api]
impl Player {
    // You (or GDScript) call this
    #[func]
    fn increase_speed(&mut self, amount: f64) {
        self.speed += amount;
        self.signals().speed_increased().emit();
    }

    #[signal]
    fn speed_increased();
}
```

Details worth remembering:

- Use `physics_process` for a fixed framerate and `process` otherwise.
- Never touch the `self.base` field directly. Use `self.base()` (read) and `self.base_mut()` (write), or you can't call base class methods.
- `rotate()` and many Godot methods take `f32`, while `delta` is `f64`, so cast with `as f32`.
- API attributes follow GDScript keyword names: `class`, `func`, `signal`, `export`, `var`.

---

## 10. Signals: making two nodes talk

Think of a signal as an announcement. Node A shouts "something happened!" and doesn't know or care who is listening. Node B subscribes and reacts. Someone (usually a parent) connects them.

### 1. Sender declares and emits

```rust
#[derive(GodotClass)]
#[class(base=Sprite2D, init)]
struct Talker {
    base: Base<Sprite2D>,
}

#[godot_api]
impl Talker {
    #[signal]
    fn said_hello(message: GString);

    #[func]
    fn say_hello(&mut self) {
        self.signals().said_hello().emit(GString::from("Hi there!"));
    }
}
```

### 2. Receiver has a matching function

```rust
#[godot_api]
impl Listener {
    #[func]
    fn on_said_hello(&mut self, message: GString) {
        godot_print!("Listener heard: {message}");
    }
}
```

The receiver's parameters must match the signal's parameters.

### 3. Connect them (usually from a parent/Main node)

```rust
// inside Main's ready()
let talker = self.base().get_node_as::<Talker>("Talker");
let listener = self.base().get_node_as::<Listener>("Listener");

talker
    .signals()
    .said_hello()
    .connect_other(&listener, Listener::on_said_hello);
```

### Why it's good design

- The Talker never holds a reference to the Listener. You can add 5 more listeners, or none, without touching the Talker.
- **Up the tree (child tells parent): signal. Down the tree (parent tells child): call the function directly.**
- No-code alternative: in the editor, select the Talker, open Node > Signals, double-click the signal, and pick the receiver. This works because the receiver has `#[func]`, which is exactly why Godot needs that attribute.
- Version note: the typed-signal API (`signals()`, `connect_other`) is newer and names have shifted between godot-rust versions. If it doesn't compile, check the Signals chapter of the book for your version.

---

## 11. Worked example: "is it in water, and how much?"

Two questions, one small **pure Rust** function (no Godot types, so it's unit-testable).

### The idea

Treat the object and the water as rectangles. The submerged fraction is how much of the object's box overlaps the water's box. In Godot, **y points down**, so water sits at larger y than the surface.

```rust
// buoyancy.rs (plain Rust, no Godot types)
pub struct BoxArea {
    pub left: f32,
    pub right: f32,
    pub top: f32,      // smaller y
    pub bottom: f32,   // larger y
}

/// 0.0 = fully out of water, 1.0 = fully underwater
pub fn submerged_fraction(obj: &BoxArea, water: &BoxArea) -> f32 {
    let w = (obj.right.min(water.right) - obj.left.max(water.left)).max(0.0);
    let h = (obj.bottom.min(water.bottom) - obj.top.max(water.top)).max(0.0);
    let overlap = w * h;
    let total = (obj.right - obj.left) * (obj.bottom - obj.top);
    if total <= 0.0 { 0.0 } else { (overlap / total).clamp(0.0, 1.0) }
}
```

Example: an object 40 px tall with 10 px below the surface gives 0.25. For a wide pool, just make the water box very wide and deep.

### Using it in the force

Buoyancy only pushes while in water, but weight always pulls. So scale the water-related terms by `submerged`:

```rust
pub fn vertical_force(depth: f32, vel_up: f32, air: f32, submerged: f32, p: &BuoyParams) -> f32 {
    let weight = -p.mass * p.gravity;                        // always pulls down
    let lift = p.water_density * p.volume * p.gravity * submerged;
    let tank = (air - 0.5) * 2.0 * p.tank_volume * p.water_density * p.gravity * submerged;

    let too_shallow = (p.band_top - depth).max(0.0);
    let too_deep = (depth - p.band_bottom).max(0.0);
    let walls = (too_deep - too_shallow) * p.zone_stiffness * submerged;

    let drag = -vel_up * p.drag * submerged;                 // water drag only in water
    weight + lift + tank + walls + drag
}
```

- `submerged = 0` (in the air): only weight acts, so it falls.
- `submerged = 1`: full underwater behavior.
- In between it blends, which is how floating at the surface works.

### Why `(air - 0.5) * 2.0`?

It's a remap. `air` runs 0 to 1, but the tank force needs to run -1 to +1:

| `air` | `(air - 0.5) * 2.0` | Effect |
|---|---|---|
| 0.0 | -1 | Full sink push |
| 0.5 | 0 | Neutral |
| 1.0 | +1 | Full lift |

Multiplied by `tank_volume * water_density * gravity`, that's the tank's maximum force either way. It's a design choice. If you'd rather `air` mean "how much air is in the tank" (0 = empty, 1 = full), drop the remap and let mass and volume set the baseline.

### Building the boxes in Player

```rust
let pos = self.base().get_global_position();
let obj = BoxArea {
    left: pos.x - self.width / 2.0,
    right: pos.x + self.width / 2.0,
    top: pos.y - self.height / 2.0,
    bottom: pos.y + self.height / 2.0,
};
let water = BoxArea { left: -10_000.0, right: 10_000.0, top: self.surface_y, bottom: 10_000.0 };
let submerged = submerged_fraction(&obj, &water);
let depth = pos.y - self.surface_y;
```

This assumes a centered `Sprite2D`. For a `CharacterBody2D`, use the collision shape's size instead of the texture size.

### Extensions

- **Several water zones:** make each an `Area2D` with a rectangle `CollisionShape2D`, build a `BoxArea` from each, and take the larger fraction.
- **Waves:** make the surface height depend on x and time, and sample it at the object's center x:
  ```rust
  pub fn surface_at(x: f32, time: f32, base_y: f32, amp: f32, freq: f32, speed: f32) -> f32 {
      base_y + amp * (freq * x + speed * time).sin()
  }
  ```
- **Irregular shapes:** define 4 to 8 sample points on the body, count how many are below the surface, and use `count_under / total` as the fraction. Cheap and smooth.

### Quick checks

- Fraction jumps between 0 and 1 with nothing in between? The object may be 0 px tall in the box, or the water box is on the wrong side (larger y is lower).
- Sinks in the air and doesn't float? Make sure `submerged` is actually passed into the force function.
- Unit test without Godot: a 40 px box with 10 px overlap should return about 0.25.

---

## 12. Exporting params to find the best values

Put tunable values on the Godot class as `#[export]` fields, and **rebuild the params struct from them each physics frame**. Then Inspector changes (even in the Remote tab while the game runs) take effect immediately.

```rust
#[derive(GodotClass)]
#[class(base=Sprite2D)]
struct Player {
    // object
    #[export] mass: f32,
    #[export] width: f32,
    #[export] height: f32,

    // water and world
    #[export] gravity: f32,
    #[export] water_density: f32,
    #[export] surface_y: f32,

    // tank
    #[export] tank_volume: f32,
    #[export] air_rate: f32,

    // depth band
    #[export] band_top: f32,
    #[export] band_bottom: f32,
    #[export] zone_stiffness: f32,
    #[export] drag: f32,

    // runtime state (not exported)
    air: f32,
    vel_up: f32,
    velocity: Vector2,
    speed: f32,

    base: Base<Sprite2D>,
}

impl Player {
    fn params(&self) -> BuoyParams {
        BuoyParams {
            gravity: self.gravity,
            water_density: self.water_density,
            volume: self.width * self.height * consts::VOLUME_SCALE,
            mass: self.mass,
            tank_volume: self.tank_volume,
            band_top: self.band_top,
            band_bottom: self.band_bottom,
            zone_stiffness: self.zone_stiffness,
            drag: self.drag,
        }
    }
}
```

In `physics_process`, call `let p = self.params();` at the top, then use `p` in the force function. In `init`, set defaults from `consts.rs`. Add `pub const VOLUME_SCALE: f32 = 0.001;` (a 40 x 40 box then has volume 1.6).

### Why this design

- **Volume isn't a separate export.** It's computed from `width * height`, so resizing automatically changes buoyancy.
- **Neutral mass = `water_density * volume`.** With a 40 x 40 box and density 1.0 that's 1.6. Set mass a bit above to sink, below to float. This is the fastest way to compare weights.
- **`BuoyParams` is not a field.** Building it fresh each frame is nearly free and is what makes live edits work.

### How to test values

- Run the game, open the **Remote** tab in the Scene dock, select the Player, and change numbers while it runs.
- **Remote edits aren't saved.** When you find good values, copy them into the Inspector on the normal scene (or into `consts.rs`).
- Change **one value at a time**: mass near neutral first, then `zone_stiffness`, then `drag`.
- Add a debug print:
  ```rust
  godot_print!("depth {depth:.0} sub {submerged:.2} vel_up {:.1} force {force:.1} air {:.2}", self.vel_up, self.air);
  ```
- Add `#[export(range = (0.0, 50.0, 0.1))]` for a slider, and `#[export_group(name = "Tank")]` above a set of fields to group them. Exact attribute forms can differ between godot-rust versions.
- **Gotcha:** a value saved in the scene overrides the default in `init()`. If you edit a const and nothing changes, the Inspector probably holds an older saved number. Use the revert arrow next to that field.
