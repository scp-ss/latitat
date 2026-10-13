// fn main() {
//     println!("Hello, world!");
// }

use godot::prelude::*;

struct MyExtension;

#[gdextension]
unsafe impl ExtensionLibrary for MyExtension {}
