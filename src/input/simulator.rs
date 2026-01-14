//! Input simulation functionality
//!
//! This module provides cross-platform input simulation for keyboard and mouse.

use crate::error::{RemoteDeskError, Result};
use crate::input::types::{
    InputEvent, Key, KeyboardEvent, KeyboardEventType, MouseButton, MouseEvent, MouseEventType,
};
use rdev::{simulate, Button, EventType, Key as RdevKey};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use tracing::{debug, warn};

/// Delay between simulated events in milliseconds
const DEFAULT_EVENT_DELAY_MS: u64 = 10;

/// Input simulator for keyboard and mouse events
pub struct InputSimulator {
    /// Number of events simulated
    events_simulated: Arc<AtomicU64>,
    /// Number of events failed
    events_failed: Arc<AtomicU64>,
    /// Delay between events (milliseconds)
    event_delay_ms: u64,
}

impl InputSimulator {
    /// Creates a new input simulator
    pub fn new() -> Self {
        Self {
            events_simulated: Arc::new(AtomicU64::new(0)),
            events_failed: Arc::new(AtomicU64::new(0)),
            event_delay_ms: DEFAULT_EVENT_DELAY_MS,
        }
    }

    /// Creates a new input simulator with custom event delay
    pub fn with_delay(event_delay_ms: u64) -> Self {
        Self {
            events_simulated: Arc::new(AtomicU64::new(0)),
            events_failed: Arc::new(AtomicU64::new(0)),
            event_delay_ms,
        }
    }

    /// Simulates an input event
    ///
    /// # Errors
    ///
    /// Returns error if event simulation fails
    pub fn simulate(&self, event: &InputEvent) -> Result<()> {
        match event {
            InputEvent::Keyboard(kb_event) => self.simulate_keyboard(kb_event),
            InputEvent::Mouse(mouse_event) => self.simulate_mouse(mouse_event),
        }
    }

    /// Simulates a keyboard event
    fn simulate_keyboard(&self, event: &KeyboardEvent) -> Result<()> {
        let rdev_key = self.convert_key(event.key)?;

        let event_type = match event.event_type {
            KeyboardEventType::KeyPress => EventType::KeyPress(rdev_key),
            KeyboardEventType::KeyRelease => EventType::KeyRelease(rdev_key),
        };

        self.send_event(event_type)?;

        debug!(
            "Simulated keyboard event: {:?} {:?}",
            event.event_type, event.key
        );

        Ok(())
    }

    /// Simulates a mouse event
    fn simulate_mouse(&self, event: &MouseEvent) -> Result<()> {
        match &event.event_type {
            MouseEventType::Move { x, y } => {
                let event_type = EventType::MouseMove {
                    x: *x as f64,
                    y: *y as f64,
                };
                self.send_event(event_type)?;
                debug!("Simulated mouse move to ({}, {})", x, y);
            }
            MouseEventType::ButtonPress { button } => {
                let rdev_button = self.convert_button(*button);
                let event_type = EventType::ButtonPress(rdev_button);
                self.send_event(event_type)?;
                debug!("Simulated mouse button press: {:?}", button);
            }
            MouseEventType::ButtonRelease { button } => {
                let rdev_button = self.convert_button(*button);
                let event_type = EventType::ButtonRelease(rdev_button);
                self.send_event(event_type)?;
                debug!("Simulated mouse button release: {:?}", button);
            }
            MouseEventType::Wheel { delta_x, delta_y } => {
                let event_type = EventType::Wheel {
                    delta_x: *delta_x as i64,
                    delta_y: *delta_y as i64,
                };
                self.send_event(event_type)?;
                debug!("Simulated mouse wheel: ({}, {})", delta_x, delta_y);
            }
        }

        Ok(())
    }

    /// Sends an event to the system
    fn send_event(&self, event_type: EventType) -> Result<()> {
        match simulate(&event_type) {
            Ok(()) => {
                self.events_simulated.fetch_add(1, Ordering::Relaxed);

                // Small delay between events to prevent overwhelming the system
                if self.event_delay_ms > 0 {
                    std::thread::sleep(std::time::Duration::from_millis(self.event_delay_ms));
                }

                Ok(())
            }
            Err(e) => {
                self.events_failed.fetch_add(1, Ordering::Relaxed);
                Err(RemoteDeskError::Generic(format!(
                    "Failed to simulate input event: {:?}",
                    e
                )))
            }
        }
    }

    /// Converts our Key enum to rdev Key
    fn convert_key(&self, key: Key) -> Result<RdevKey> {
        let rdev_key = match key {
            // Letters
            Key::A => RdevKey::KeyA,
            Key::B => RdevKey::KeyB,
            Key::C => RdevKey::KeyC,
            Key::D => RdevKey::KeyD,
            Key::E => RdevKey::KeyE,
            Key::F => RdevKey::KeyF,
            Key::G => RdevKey::KeyG,
            Key::H => RdevKey::KeyH,
            Key::I => RdevKey::KeyI,
            Key::J => RdevKey::KeyJ,
            Key::K => RdevKey::KeyK,
            Key::L => RdevKey::KeyL,
            Key::M => RdevKey::KeyM,
            Key::N => RdevKey::KeyN,
            Key::O => RdevKey::KeyO,
            Key::P => RdevKey::KeyP,
            Key::Q => RdevKey::KeyQ,
            Key::R => RdevKey::KeyR,
            Key::S => RdevKey::KeyS,
            Key::T => RdevKey::KeyT,
            Key::U => RdevKey::KeyU,
            Key::V => RdevKey::KeyV,
            Key::W => RdevKey::KeyW,
            Key::X => RdevKey::KeyX,
            Key::Y => RdevKey::KeyY,
            Key::Z => RdevKey::KeyZ,

            // Numbers
            Key::Num0 => RdevKey::Num0,
            Key::Num1 => RdevKey::Num1,
            Key::Num2 => RdevKey::Num2,
            Key::Num3 => RdevKey::Num3,
            Key::Num4 => RdevKey::Num4,
            Key::Num5 => RdevKey::Num5,
            Key::Num6 => RdevKey::Num6,
            Key::Num7 => RdevKey::Num7,
            Key::Num8 => RdevKey::Num8,
            Key::Num9 => RdevKey::Num9,

            // Function keys
            Key::F1 => RdevKey::F1,
            Key::F2 => RdevKey::F2,
            Key::F3 => RdevKey::F3,
            Key::F4 => RdevKey::F4,
            Key::F5 => RdevKey::F5,
            Key::F6 => RdevKey::F6,
            Key::F7 => RdevKey::F7,
            Key::F8 => RdevKey::F8,
            Key::F9 => RdevKey::F9,
            Key::F10 => RdevKey::F10,
            Key::F11 => RdevKey::F11,
            Key::F12 => RdevKey::F12,

            // Modifiers
            Key::Shift => RdevKey::ShiftLeft,
            Key::Control => RdevKey::ControlLeft,
            Key::Alt => RdevKey::Alt,
            Key::Meta => RdevKey::MetaLeft,

            // Navigation
            Key::Up => RdevKey::UpArrow,
            Key::Down => RdevKey::DownArrow,
            Key::Left => RdevKey::LeftArrow,
            Key::Right => RdevKey::RightArrow,
            Key::Home => RdevKey::Home,
            Key::End => RdevKey::End,
            Key::PageUp => RdevKey::PageUp,
            Key::PageDown => RdevKey::PageDown,

            // Special keys
            Key::Return => RdevKey::Return,
            Key::Escape => RdevKey::Escape,
            Key::Backspace => RdevKey::Backspace,
            Key::Tab => RdevKey::Tab,
            Key::Space => RdevKey::Space,
            Key::Delete => RdevKey::Delete,
            Key::Insert => RdevKey::Insert,
            Key::CapsLock => RdevKey::CapsLock,

            // Punctuation
            Key::Minus => RdevKey::Minus,
            Key::Equal => RdevKey::Equal,
            Key::LeftBracket => RdevKey::LeftBracket,
            Key::RightBracket => RdevKey::RightBracket,
            Key::Semicolon => RdevKey::SemiColon,
            Key::Quote => RdevKey::Quote,
            Key::Backslash => RdevKey::BackSlash,
            Key::Comma => RdevKey::Comma,
            Key::Period => RdevKey::Dot,
            Key::Slash => RdevKey::Slash,
            Key::Grave => RdevKey::BackQuote,

            Key::Unknown => {
                warn!("Attempted to simulate unknown key");
                return Err(RemoteDeskError::Generic("Unknown key".to_string()));
            }
        };

        Ok(rdev_key)
    }

    /// Converts our MouseButton to rdev Button
    fn convert_button(&self, button: MouseButton) -> Button {
        match button {
            MouseButton::Left => Button::Left,
            MouseButton::Right => Button::Right,
            MouseButton::Middle => Button::Middle,
            MouseButton::Button4 => Button::Unknown(4),
            MouseButton::Button5 => Button::Unknown(5),
        }
    }

    /// Types a text string by simulating key presses
    ///
    /// # Errors
    ///
    /// Returns error if any key simulation fails
    pub fn type_string(&self, text: &str) -> Result<()> {
        for c in text.chars() {
            let key = self.char_to_key(c)?;

            // Check if we need to hold shift
            let needs_shift = c.is_uppercase()
                || matches!(
                    c,
                    '!' | '@' | '#' | '$' | '%' | '^' | '&' | '*' | '(' | ')' | '_' | '+' | '{'
                        | '}' | '|' | ':' | '"' | '<' | '>' | '?'
                );

            if needs_shift {
                self.simulate_keyboard(&KeyboardEvent::key_press(Key::Shift))?;
            }

            self.simulate_keyboard(&KeyboardEvent::key_press(key))?;
            self.simulate_keyboard(&KeyboardEvent::key_release(key))?;

            if needs_shift {
                self.simulate_keyboard(&KeyboardEvent::key_release(Key::Shift))?;
            }
        }

        Ok(())
    }

    /// Converts a character to a Key
    fn char_to_key(&self, c: char) -> Result<Key> {
        let key = match c.to_ascii_lowercase() {
            'a' => Key::A,
            'b' => Key::B,
            'c' => Key::C,
            'd' => Key::D,
            'e' => Key::E,
            'f' => Key::F,
            'g' => Key::G,
            'h' => Key::H,
            'i' => Key::I,
            'j' => Key::J,
            'k' => Key::K,
            'l' => Key::L,
            'm' => Key::M,
            'n' => Key::N,
            'o' => Key::O,
            'p' => Key::P,
            'q' => Key::Q,
            'r' => Key::R,
            's' => Key::S,
            't' => Key::T,
            'u' => Key::U,
            'v' => Key::V,
            'w' => Key::W,
            'x' => Key::X,
            'y' => Key::Y,
            'z' => Key::Z,
            '0' | ')' => Key::Num0,
            '1' | '!' => Key::Num1,
            '2' | '@' => Key::Num2,
            '3' | '#' => Key::Num3,
            '4' | '$' => Key::Num4,
            '5' | '%' => Key::Num5,
            '6' | '^' => Key::Num6,
            '7' | '&' => Key::Num7,
            '8' | '*' => Key::Num8,
            '9' | '(' => Key::Num9,
            ' ' => Key::Space,
            '-' | '_' => Key::Minus,
            '=' | '+' => Key::Equal,
            '[' | '{' => Key::LeftBracket,
            ']' | '}' => Key::RightBracket,
            ';' | ':' => Key::Semicolon,
            '\'' | '"' => Key::Quote,
            '\\' | '|' => Key::Backslash,
            ',' | '<' => Key::Comma,
            '.' | '>' => Key::Period,
            '/' | '?' => Key::Slash,
            '`' | '~' => Key::Grave,
            '\n' => Key::Return,
            '\t' => Key::Tab,
            _ => {
                warn!("Unsupported character for typing: '{}'", c);
                return Err(RemoteDeskError::Generic(format!(
                    "Unsupported character: '{}'",
                    c
                )));
            }
        };

        Ok(key)
    }

    /// Returns the number of events successfully simulated
    pub fn events_simulated(&self) -> u64 {
        self.events_simulated.load(Ordering::Relaxed)
    }

    /// Returns the number of events that failed
    pub fn events_failed(&self) -> u64 {
        self.events_failed.load(Ordering::Relaxed)
    }

    /// Returns the success rate (0.0 to 1.0)
    pub fn success_rate(&self) -> f64 {
        let total = self.events_simulated() + self.events_failed();
        if total == 0 {
            return 1.0;
        }
        self.events_simulated() as f64 / total as f64
    }
}

impl Default for InputSimulator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulator_creation() {
        let simulator = InputSimulator::new();
        assert_eq!(simulator.events_simulated(), 0);
        assert_eq!(simulator.events_failed(), 0);
        assert_eq!(simulator.success_rate(), 1.0);

        let simulator = InputSimulator::with_delay(5);
        assert_eq!(simulator.event_delay_ms, 5);
    }

    #[test]
    fn test_key_conversion() {
        let simulator = InputSimulator::new();

        assert!(simulator.convert_key(Key::A).is_ok());
        assert!(simulator.convert_key(Key::Return).is_ok());
        assert!(simulator.convert_key(Key::Shift).is_ok());
        assert!(simulator.convert_key(Key::Unknown).is_err());
    }

    #[test]
    fn test_button_conversion() {
        let simulator = InputSimulator::new();

        let button = simulator.convert_button(MouseButton::Left);
        assert!(matches!(button, Button::Left));

        let button = simulator.convert_button(MouseButton::Right);
        assert!(matches!(button, Button::Right));
    }

    #[test]
    fn test_char_to_key() {
        let simulator = InputSimulator::new();

        assert_eq!(simulator.char_to_key('a').unwrap(), Key::A);
        assert_eq!(simulator.char_to_key('A').unwrap(), Key::A);
        assert_eq!(simulator.char_to_key('0').unwrap(), Key::Num0);
        assert_eq!(simulator.char_to_key(' ').unwrap(), Key::Space);
        assert_eq!(simulator.char_to_key('\n').unwrap(), Key::Return);

        // Unsupported character
        assert!(simulator.char_to_key('€').is_err());
    }

    // Note: Actual input simulation tests are skipped because they would
    // actually move the mouse and press keys on the test system
    #[test]
    #[ignore]
    fn test_simulate_keyboard() {
        let simulator = InputSimulator::new();
        let event = KeyboardEvent::key_press(Key::A);
        // This would actually press the 'A' key
        // simulator.simulate_keyboard(&event).unwrap();
    }

    #[test]
    #[ignore]
    fn test_simulate_mouse() {
        let simulator = InputSimulator::new();
        let event = MouseEvent::move_to(100, 100);
        // This would actually move the mouse
        // simulator.simulate_mouse(&event).unwrap();
    }
}
