/// Snapshot of player input read by the game loop each frame.
/// No external dependencies — usable on both client and server.
#[derive(Default, Clone)]
pub struct InputState {
    // Movement
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,

    // Actions
    pub shoot: bool, // left mouse button
    pub walk: bool,  // space — slow/sneak movement
    pub shift: bool, // shift — run
    pub peek: bool,  // hold X — peek modifier
    pub reload: bool,
    pub escape: bool,
    pub enter: bool,
    pub key_b: bool, // reserved

    // Editor
    pub f1: bool,    // toggle editor mode
    pub f5: bool,    // save level
    pub f8: bool,    // toggle debug overlay
    pub key_1: bool, // tool: player spawn
    pub key_2: bool, // tool: enemy
    pub key_3: bool, // tool: wall
    pub key_4: bool, // tool: target dummy
    pub key_5: bool, // editor tool: breakable wall
    pub key_6: bool, // editor tool: prop
    pub key_7: bool, // editor tool: base map bounds
    pub key_q: bool, // editor: previous prop id
    pub key_e: bool, // editor: next prop id
    pub key_g: bool, // toggle grid snap
    pub key_h: bool, // toggle inner grid visibility
    pub key_l: bool, // load level
    pub key_minus: bool,
    pub key_equals: bool,

    // Mouse
    pub mouse_x: f64,
    pub mouse_y: f64,
    pub mouse_left: bool,
    pub mouse_right: bool,
    pub mouse_wheel_y: f32, // transient, reset each frame
}
