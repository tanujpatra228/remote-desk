//! Input types and event structures
//!
//! This module defines types for keyboard and mouse input events.

use serde::{Deserialize, Serialize};

/// Mouse button identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum MouseButton {
    /// Left mouse button
    Left = 1,
    /// Right mouse button
    Right = 2,
    /// Middle mouse button (wheel click)
    Middle = 3,
    /// Extra button 1 (back)
    Button4 = 4,
    /// Extra button 2 (forward)
    Button5 = 5,
}

/// Mouse event type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MouseEventType {
    /// Mouse moved to absolute position
    Move {
        /// X coordinate
        x: i32,
        /// Y coordinate
        y: i32,
    },
    /// Mouse button pressed
    ButtonPress {
        /// Button that was pressed
        button: MouseButton,
    },
    /// Mouse button released
    ButtonRelease {
        /// Button that was released
        button: MouseButton,
    },
    /// Mouse wheel scrolled
    Wheel {
        /// Horizontal scroll delta
        delta_x: i32,
        /// Vertical scroll delta
        delta_y: i32,
    },
}

/// Mouse input event
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MouseEvent {
    /// Event type
    pub event_type: MouseEventType,
    /// Timestamp when event occurred (milliseconds since epoch)
    pub timestamp: u64,
}

impl MouseEvent {
    /// Creates a new mouse move event
    pub fn move_to(x: i32, y: i32) -> Self {
        Self {
            event_type: MouseEventType::Move { x, y },
            timestamp: Self::current_timestamp(),
        }
    }

    /// Creates a new mouse button press event
    pub fn button_press(button: MouseButton) -> Self {
        Self {
            event_type: MouseEventType::ButtonPress { button },
            timestamp: Self::current_timestamp(),
        }
    }

    /// Creates a new mouse button release event
    pub fn button_release(button: MouseButton) -> Self {
        Self {
            event_type: MouseEventType::ButtonRelease { button },
            timestamp: Self::current_timestamp(),
        }
    }

    /// Creates a new mouse wheel event
    pub fn wheel(delta_x: i32, delta_y: i32) -> Self {
        Self {
            event_type: MouseEventType::Wheel { delta_x, delta_y },
            timestamp: Self::current_timestamp(),
        }
    }

    /// Gets current timestamp in milliseconds
    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

/// Keyboard key identifier
///
/// Represents common keyboard keys in a platform-independent way
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u16)]
pub enum Key {
    // Letters (A-Z)
    /// A key
    A = 0x41,
    /// B key
    B = 0x42,
    /// C key
    C = 0x43,
    /// D key
    D = 0x44,
    /// E key
    E = 0x45,
    /// F key
    F = 0x46,
    /// G key
    G = 0x47,
    /// H key
    H = 0x48,
    /// I key
    I = 0x49,
    /// J key
    J = 0x4A,
    /// K key
    K = 0x4B,
    /// L key
    L = 0x4C,
    /// M key
    M = 0x4D,
    /// N key
    N = 0x4E,
    /// O key
    O = 0x4F,
    /// P key
    P = 0x50,
    /// Q key
    Q = 0x51,
    /// R key
    R = 0x52,
    /// S key
    S = 0x53,
    /// T key
    T = 0x54,
    /// U key
    U = 0x55,
    /// V key
    V = 0x56,
    /// W key
    W = 0x57,
    /// X key
    X = 0x58,
    /// Y key
    Y = 0x59,
    /// Z key
    Z = 0x5A,

    // Numbers (0-9)
    /// 0 key
    Num0 = 0x30,
    /// 1 key
    Num1 = 0x31,
    /// 2 key
    Num2 = 0x32,
    /// 3 key
    Num3 = 0x33,
    /// 4 key
    Num4 = 0x34,
    /// 5 key
    Num5 = 0x35,
    /// 6 key
    Num6 = 0x36,
    /// 7 key
    Num7 = 0x37,
    /// 8 key
    Num8 = 0x38,
    /// 9 key
    Num9 = 0x39,

    // Function keys (F1-F12)
    /// F1 key
    F1 = 0x70,
    /// F2 key
    F2 = 0x71,
    /// F3 key
    F3 = 0x72,
    /// F4 key
    F4 = 0x73,
    /// F5 key
    F5 = 0x74,
    /// F6 key
    F6 = 0x75,
    /// F7 key
    F7 = 0x76,
    /// F8 key
    F8 = 0x77,
    /// F9 key
    F9 = 0x78,
    /// F10 key
    F10 = 0x79,
    /// F11 key
    F11 = 0x7A,
    /// F12 key
    F12 = 0x7B,

    // Modifier keys
    /// Shift key
    Shift = 0x10,
    /// Control key
    Control = 0x11,
    /// Alt key
    Alt = 0x12,
    /// Meta/Windows/Command key
    Meta = 0x5B,

    // Navigation keys
    /// Up arrow
    Up = 0x26,
    /// Down arrow
    Down = 0x28,
    /// Left arrow
    Left = 0x25,
    /// Right arrow
    Right = 0x27,
    /// Home key
    Home = 0x24,
    /// End key
    End = 0x23,
    /// Page Up
    PageUp = 0x21,
    /// Page Down
    PageDown = 0x22,

    // Special keys
    /// Enter/Return key
    Return = 0x0D,
    /// Escape key
    Escape = 0x1B,
    /// Backspace key
    Backspace = 0x08,
    /// Tab key
    Tab = 0x09,
    /// Space key
    Space = 0x20,
    /// Delete key
    Delete = 0x2E,
    /// Insert key
    Insert = 0x2D,
    /// Caps Lock
    CapsLock = 0x14,

    // Punctuation and symbols
    /// Minus/Underscore key
    Minus = 0xBD,
    /// Equal/Plus key
    Equal = 0xBB,
    /// Left bracket
    LeftBracket = 0xDB,
    /// Right bracket
    RightBracket = 0xDD,
    /// Semicolon
    Semicolon = 0xBA,
    /// Quote
    Quote = 0xDE,
    /// Backslash
    Backslash = 0xDC,
    /// Comma
    Comma = 0xBC,
    /// Period
    Period = 0xBE,
    /// Slash
    Slash = 0xBF,
    /// Backtick/Grave
    Grave = 0xC0,

    /// Unknown key
    Unknown = 0xFFFF,
}

/// Keyboard event type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyboardEventType {
    /// Key pressed
    KeyPress,
    /// Key released
    KeyRelease,
}

/// Keyboard input event
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyboardEvent {
    /// Event type (press or release)
    pub event_type: KeyboardEventType,
    /// Key that was pressed or released
    pub key: Key,
    /// Timestamp when event occurred (milliseconds since epoch)
    pub timestamp: u64,
}

impl KeyboardEvent {
    /// Creates a new key press event
    pub fn key_press(key: Key) -> Self {
        Self {
            event_type: KeyboardEventType::KeyPress,
            key,
            timestamp: Self::current_timestamp(),
        }
    }

    /// Creates a new key release event
    pub fn key_release(key: Key) -> Self {
        Self {
            event_type: KeyboardEventType::KeyRelease,
            key,
            timestamp: Self::current_timestamp(),
        }
    }

    /// Gets current timestamp in milliseconds
    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

/// Generic input event (keyboard or mouse)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InputEvent {
    /// Keyboard event
    Keyboard(KeyboardEvent),
    /// Mouse event
    Mouse(MouseEvent),
}

impl From<KeyboardEvent> for InputEvent {
    fn from(event: KeyboardEvent) -> Self {
        InputEvent::Keyboard(event)
    }
}

impl From<MouseEvent> for InputEvent {
    fn from(event: MouseEvent) -> Self {
        InputEvent::Mouse(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mouse_event_creation() {
        let event = MouseEvent::move_to(100, 200);
        assert!(matches!(event.event_type, MouseEventType::Move { x: 100, y: 200 }));

        let event = MouseEvent::button_press(MouseButton::Left);
        assert!(matches!(
            event.event_type,
            MouseEventType::ButtonPress {
                button: MouseButton::Left
            }
        ));

        let event = MouseEvent::wheel(0, -10);
        assert!(matches!(
            event.event_type,
            MouseEventType::Wheel {
                delta_x: 0,
                delta_y: -10
            }
        ));
    }

    #[test]
    fn test_keyboard_event_creation() {
        let event = KeyboardEvent::key_press(Key::A);
        assert_eq!(event.event_type, KeyboardEventType::KeyPress);
        assert_eq!(event.key, Key::A);

        let event = KeyboardEvent::key_release(Key::Control);
        assert_eq!(event.event_type, KeyboardEventType::KeyRelease);
        assert_eq!(event.key, Key::Control);
    }

    #[test]
    fn test_input_event_conversion() {
        let kb_event = KeyboardEvent::key_press(Key::A);
        let input_event: InputEvent = kb_event.into();
        assert!(matches!(input_event, InputEvent::Keyboard(_)));

        let mouse_event = MouseEvent::move_to(50, 50);
        let input_event: InputEvent = mouse_event.into();
        assert!(matches!(input_event, InputEvent::Mouse(_)));
    }

    #[test]
    fn test_serialization() {
        let event = MouseEvent::move_to(100, 200);
        let serialized = bincode::serialize(&event).unwrap();
        let deserialized: MouseEvent = bincode::deserialize(&serialized).unwrap();
        assert_eq!(event.event_type, deserialized.event_type);

        let event = KeyboardEvent::key_press(Key::A);
        let serialized = bincode::serialize(&event).unwrap();
        let deserialized: KeyboardEvent = bincode::deserialize(&serialized).unwrap();
        assert_eq!(event.key, deserialized.key);
    }

    #[test]
    fn test_key_codes() {
        assert_eq!(Key::A as u16, 0x41);
        assert_eq!(Key::Z as u16, 0x5A);
        assert_eq!(Key::Num0 as u16, 0x30);
        assert_eq!(Key::F1 as u16, 0x70);
        assert_eq!(Key::Return as u16, 0x0D);
    }
}
