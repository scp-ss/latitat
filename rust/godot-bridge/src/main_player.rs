//##########################################################################################################
// ################################################### BRINING TO SCOPE ####################################
//##########################################################################################################
// use godot::classes::Charac;
use godot::classes::{CharacterBody2D, ICharacterBody2D, Input};
use godot::prelude::*;
//##########################################################################################################
// %%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%% END OF BOS : %%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%/
//##########################################################################################################

//

//

#[derive(GodotClass)]
#[class(base=CharacterBody2D)]
struct MainPlayer {
    speed: f32,
    angular_speed: f64,
    base: Base<CharacterBody2D>,
}

#[godot_api]
impl ICharacterBody2D for MainPlayer {
    fn init(base: Base<CharacterBody2D>) -> Self {
        Self {
            speed: 200.0,
            angular_speed: std::f64::consts::PI,
            base,
        }
    }

    fn physics_process(&mut self, delta: f64) {
        let speed = self.speed; // read your own fields first
        let dir = Input::singleton().get_vector("ui_left", "ui_right", "ui_up", "ui_down");
        let input = Input::singleton();
        let mut body = self.base_mut();
        body.set_velocity(dir * speed);
        body.move_and_slide();
    }
}
/*
*     #[export]
#[init(val = DEFAULT_JUMPS)]
max_jumps: i32,
*/
