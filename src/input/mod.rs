use winit::event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{Key, NamedKey};

// ---------------------------------------------------------------------------
// InputState — snapshot read by the game loop each frame
// ---------------------------------------------------------------------------

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
    pub key_b: bool, // open/cancel buy menu

    // Editor
    pub f1: bool,    // toggle editor mode
    pub f5: bool,    // save level
    pub f8: bool,    // toggle debug overlay
    pub key_1: bool, // tool: player spawn
    pub key_2: bool, // tool: enemy
    pub key_3: bool, // tool: wall
    pub key_4: bool, // tool: target dummy
    pub key_5: bool, // buy menu choice / editor tool: breakable wall
    pub key_g: bool, // toggle grid snap
    pub key_l: bool, // load level

    // Mouse
    pub mouse_x: f64,
    pub mouse_y: f64,
    pub mouse_left: bool,
    pub mouse_right: bool,
}

// ---------------------------------------------------------------------------
// InputHandler — translates winit events into InputState mutations
// ---------------------------------------------------------------------------

pub struct InputHandler {
    pub state: InputState,
}

impl InputHandler {
    pub fn new() -> Self {
        Self {
            state: InputState::default(),
        }
    }

    /// Feed a winit event in. Returns `true` if consumed.
    pub fn handle(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                self.on_key(event);
                true
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.state.mouse_x = position.x;
                self.state.mouse_y = position.y;
                true
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.on_mouse_button(*button, *state);
                true
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.on_scroll(delta);
                true
            }
            _ => false,
        }
    }

    fn on_key(&mut self, event: &KeyEvent) {
        let pressed = event.state == ElementState::Pressed;

        if let Key::Named(named) = &event.logical_key {
            match named {
                NamedKey::ArrowUp => self.state.up = pressed,
                NamedKey::ArrowDown => self.state.down = pressed,
                NamedKey::ArrowLeft => self.state.left = pressed,
                NamedKey::ArrowRight => self.state.right = pressed,
                NamedKey::Space => self.state.walk = pressed,
                NamedKey::Shift => self.state.shift = pressed,
                NamedKey::Escape => self.state.escape = pressed,
                NamedKey::Enter => self.state.enter = pressed,
                NamedKey::F1 => self.state.f1 = pressed,
                NamedKey::F5 => self.state.f5 = pressed,
                NamedKey::F8 => self.state.f8 = pressed,
                other => {
                    let _ = other;
                    return;
                }
            }
            return;
        }

        if let Key::Character(ch) = &event.logical_key {
            match ch.as_str() {
                "w" | "W" => self.state.up = pressed,
                "s" | "S" => self.state.down = pressed,
                "a" | "A" => self.state.left = pressed,
                "d" | "D" => self.state.right = pressed,
                "x" | "X" => self.state.peek = pressed,
                "r" | "R" => self.state.reload = pressed,
                "b" | "B" => self.state.key_b = pressed,
                "1" => self.state.key_1 = pressed,
                "2" => self.state.key_2 = pressed,
                "3" => self.state.key_3 = pressed,
                "4" => self.state.key_4 = pressed,
                "5" => self.state.key_5 = pressed,
                "g" | "G" => self.state.key_g = pressed,
                "l" | "L" => self.state.key_l = pressed,
                _ => {}
            }
        }
    }

    fn on_mouse_button(&mut self, button: MouseButton, state: ElementState) {
        let pressed = state == ElementState::Pressed;
        match button {
            MouseButton::Left => {
                self.state.mouse_left = pressed;
                self.state.shoot = pressed;
            }
            MouseButton::Right => self.state.mouse_right = pressed,
            _ => {}
        }
    }

    fn on_scroll(&self, _delta: &MouseScrollDelta) {}
}
