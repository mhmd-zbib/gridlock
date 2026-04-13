use winit::event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{Key, NamedKey};

pub use game::input::InputState;

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
                "6" => self.state.key_6 = pressed,
                "7" => self.state.key_7 = pressed,
                "-" | "_" => self.state.key_minus = pressed,
                "=" | "+" => self.state.key_equals = pressed,
                "q" | "Q" => self.state.key_q = pressed,
                "e" | "E" => self.state.key_e = pressed,
                "g" | "G" => self.state.key_g = pressed,
                "h" | "H" => self.state.key_h = pressed,
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

    fn on_scroll(&mut self, delta: &MouseScrollDelta) {
        let amount = match delta {
            MouseScrollDelta::LineDelta(_, y) => *y,
            MouseScrollDelta::PixelDelta(pos) => (pos.y as f32) / 32.0,
        };
        self.state.mouse_wheel_y += amount;
    }

    pub fn end_frame(&mut self) {
        self.state.mouse_wheel_y = 0.0;
    }
}
