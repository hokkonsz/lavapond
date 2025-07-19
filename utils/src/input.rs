use glam::vec2;

pub const NUMBER_OF_KEYS: usize = 50;

#[derive(Debug)]
pub struct Inputs {
    key_last_state: [bool; NUMBER_OF_KEYS],
    key_state: [bool; NUMBER_OF_KEYS],
    mouse_last_state: MouseState,
    mouse_state: MouseState,
}

pub trait InputHandler {
    fn handle_inputs(&mut self, event: &winit::event::WindowEvent);
}

impl Default for Inputs {
    fn default() -> Self {
        Self {
            key_last_state: [false; NUMBER_OF_KEYS],
            key_state: [false; NUMBER_OF_KEYS],
            mouse_last_state: MouseState::default(),
            mouse_state: MouseState::default(),
        }
    }
}

impl Inputs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read(&mut self, event: &winit::event::WindowEvent) {
        self.key_last_state = self.key_state;
        self.mouse_last_state = self.mouse_state;

        match event {
            winit::event::WindowEvent::KeyboardInput {
                event:
                    winit::event::KeyEvent {
                        state, logical_key, ..
                    },
                ..
            } => {
                let mapped_key = match logical_key {
                    winit::keyboard::Key::Named(key_named) => match key_named {
                        winit::keyboard::NamedKey::Space => Some(Key::Space),
                        winit::keyboard::NamedKey::Enter => Some(Key::Enter),
                        winit::keyboard::NamedKey::Escape => Some(Key::Escape),
                        winit::keyboard::NamedKey::Delete => Some(Key::Delete),
                        winit::keyboard::NamedKey::CapsLock => Some(Key::CapsLock),
                        winit::keyboard::NamedKey::Tab => Some(Key::Tab),
                        winit::keyboard::NamedKey::Shift => Some(Key::Shift),
                        winit::keyboard::NamedKey::Alt => Some(Key::Alt),
                        winit::keyboard::NamedKey::Control => Some(Key::Control),
                        winit::keyboard::NamedKey::Backspace => Some(Key::Backspace),
                        winit::keyboard::NamedKey::ArrowUp => Some(Key::Up),
                        winit::keyboard::NamedKey::ArrowLeft => Some(Key::Left),
                        winit::keyboard::NamedKey::ArrowDown => Some(Key::Down),
                        winit::keyboard::NamedKey::ArrowRight => Some(Key::Right),
                        _ => None,
                    },
                    winit::keyboard::Key::Character(key) => {
                        let key = key.as_str().to_ascii_lowercase();
                        match key.as_str() {
                            "a" => Some(Key::A),
                            "b" => Some(Key::B),
                            "c" => Some(Key::C),
                            "d" => Some(Key::D),
                            "e" => Some(Key::E),
                            "f" => Some(Key::F),
                            "g" => Some(Key::G),
                            "h" => Some(Key::H),
                            "i" => Some(Key::I),
                            "j" => Some(Key::J),
                            "k" => Some(Key::K),
                            "l" => Some(Key::L),
                            "m" => Some(Key::M),
                            "n" => Some(Key::N),
                            "o" => Some(Key::O),
                            "p" => Some(Key::P),
                            "q" => Some(Key::Q),
                            "r" => Some(Key::R),
                            "s" => Some(Key::S),
                            "t" => Some(Key::T),
                            "u" => Some(Key::U),
                            "v" => Some(Key::V),
                            "w" => Some(Key::W),
                            "x" => Some(Key::X),
                            "y" => Some(Key::Y),
                            "z" => Some(Key::Z),
                            "0" => Some(Key::Num0),
                            "1" => Some(Key::Num1),
                            "2" => Some(Key::Num2),
                            "3" => Some(Key::Num3),
                            "4" => Some(Key::Num4),
                            "5" => Some(Key::Num5),
                            "6" => Some(Key::Num6),
                            "7" => Some(Key::Num7),
                            "8" => Some(Key::Num8),
                            "9" => Some(Key::Num9),
                            _ => None,
                        }
                    }
                    _ => None,
                };

                if let Some(key) = mapped_key {
                    self.key_state[key] = state.is_pressed();
                }
            }
            winit::event::WindowEvent::MouseInput { state, button, .. } => {
                match button {
                    winit::event::MouseButton::Left => self.mouse_state.lmb = state.is_pressed(),
                    winit::event::MouseButton::Right => self.mouse_state.rmb = state.is_pressed(),
                    winit::event::MouseButton::Middle => self.mouse_state.lmb = state.is_pressed(),
                    _ => (),
                    // winit::event::MouseButton::Back
                    // winit::event::MouseButton::Forward
                    // winit::event::MouseButton::Other(u16)
                }
            }
            winit::event::WindowEvent::CursorMoved { position, .. } => {
                self.mouse_state.position = Some(vec2(position.x as f32, position.y as f32));
            }
            // winit::event::WindowEvent::CursorEntered { .. } => {
            //     self.mouse_state.position = Some(vec2(0., 0.)),
            // }
            winit::event::WindowEvent::CursorLeft { .. } => {
                self.mouse_state.position = None;
            }
            _ => (),
        }
    }

    pub fn just_pressed(&self, key: Key) -> bool {
        !self.key_last_state[key] && self.key_state[key]
    }
    pub fn just_released(&self, key: Key) -> bool {
        self.key_last_state[key] && !self.key_state[key]
    }
    pub fn held_down(&self, key: Key) -> bool {
        self.key_state[key]
    }

    pub fn lmb_just_pressed(&self) -> bool {
        !self.mouse_last_state.lmb && self.mouse_state.lmb
    }

    pub fn lmb_just_released(&self) -> bool {
        self.mouse_last_state.lmb && !self.mouse_state.lmb
    }

    pub fn lmb_held_down(&self) -> bool {
        self.mouse_state.lmb
    }

    pub fn rmb_just_pressed(&self) -> bool {
        !self.mouse_last_state.rmb && self.mouse_state.rmb
    }

    pub fn rmb_just_released(&self) -> bool {
        self.mouse_last_state.rmb && !self.mouse_state.rmb
    }

    pub fn rmb_held_down(&self) -> bool {
        self.mouse_state.rmb
    }

    pub fn mmb_just_pressed(&self) -> bool {
        !self.mouse_last_state.mmb && self.mouse_state.mmb
    }

    pub fn mmb_just_released(&self) -> bool {
        self.mouse_last_state.mmb && !self.mouse_state.mmb
    }

    pub fn mmb_held_down(&self) -> bool {
        self.mouse_state.mmb
    }

    pub fn mouse_pos(&self) -> Option<glam::Vec2> {
        self.mouse_state.position
    }

    pub fn mouse_pos_delta(&self) -> Option<glam::Vec2> {
        if let (Some(last_pos), Some(pos)) =
            (self.mouse_state.position, self.mouse_last_state.position)
        {
            Some(last_pos - pos)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Key {
    Space = 0,
    Enter = 1,
    Escape = 2,
    Delete = 3,
    Tab = 4,
    CapsLock = 5,
    Backspace = 6,
    Shift = 7,
    Alt = 8,
    Control = 9,
    Up = 10,
    Left = 11,
    Down = 12,
    Right = 13,
    A = 14,
    B = 15,
    C = 16,
    D = 17,
    E = 18,
    F = 19,
    G = 20,
    H = 21,
    I = 22,
    J = 23,
    K = 24,
    L = 25,
    M = 26,
    N = 27,
    O = 28,
    P = 29,
    Q = 30,
    R = 31,
    S = 32,
    T = 33,
    U = 34,
    V = 35,
    W = 36,
    X = 37,
    Y = 38,
    Z = 39,
    Num0 = 40,
    Num1 = 41,
    Num2 = 42,
    Num3 = 43,
    Num4 = 44,
    Num5 = 45,
    Num6 = 46,
    Num7 = 47,
    Num8 = 48,
    Num9 = 49,
}

impl<T> std::ops::Index<Key> for [T] {
    type Output = T;
    fn index(&self, idx: Key) -> &Self::Output {
        &self[idx as usize]
    }
}

impl<T> std::ops::IndexMut<Key> for [T] {
    fn index_mut(&mut self, index: Key) -> &mut Self::Output {
        &mut self[index as usize]
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MouseState {
    lmb: bool,
    rmb: bool,
    mmb: bool,
    position: Option<glam::Vec2>,
}
