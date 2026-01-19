//! Session state machine for managing session lifecycle
//!
//! Provides a state machine that tracks session states and validates transitions.

use std::fmt;
use std::time::Instant;

use crate::error::SessionError;

/// Possible states for a remote desktop session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SessionState {
    /// Session created but not yet started
    Idle,
    /// Attempting to establish connection
    Connecting,
    /// Connection established, performing authentication
    Authenticating,
    /// Fully active session with data flowing
    Active,
    /// Session temporarily paused (e.g., minimized window)
    Paused,
    /// Graceful disconnection in progress
    Disconnecting,
    /// Session has ended
    Disconnected,
}

impl fmt::Display for SessionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionState::Idle => write!(f, "Idle"),
            SessionState::Connecting => write!(f, "Connecting"),
            SessionState::Authenticating => write!(f, "Authenticating"),
            SessionState::Active => write!(f, "Active"),
            SessionState::Paused => write!(f, "Paused"),
            SessionState::Disconnecting => write!(f, "Disconnecting"),
            SessionState::Disconnected => write!(f, "Disconnected"),
        }
    }
}

impl SessionState {
    /// Returns true if this state allows data transmission
    pub fn is_data_ready(&self) -> bool {
        matches!(self, SessionState::Active)
    }

    /// Returns true if the session can be considered "connected"
    pub fn is_connected(&self) -> bool {
        matches!(
            self,
            SessionState::Authenticating | SessionState::Active | SessionState::Paused
        )
    }

    /// Returns true if this is a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, SessionState::Disconnected)
    }

    /// Returns valid transitions from this state
    pub fn valid_transitions(&self) -> &'static [SessionState] {
        match self {
            SessionState::Idle => &[SessionState::Connecting, SessionState::Disconnected],
            SessionState::Connecting => &[
                SessionState::Authenticating,
                SessionState::Disconnecting,
                SessionState::Disconnected,
            ],
            SessionState::Authenticating => &[
                SessionState::Active,
                SessionState::Disconnecting,
                SessionState::Disconnected,
            ],
            SessionState::Active => &[
                SessionState::Paused,
                SessionState::Disconnecting,
                SessionState::Disconnected,
            ],
            SessionState::Paused => &[
                SessionState::Active,
                SessionState::Disconnecting,
                SessionState::Disconnected,
            ],
            SessionState::Disconnecting => &[SessionState::Disconnected],
            SessionState::Disconnected => &[],
        }
    }
}

/// Record of a state transition
#[derive(Debug, Clone)]
pub struct StateTransition {
    /// Previous state
    pub from: SessionState,
    /// New state
    pub to: SessionState,
    /// When the transition occurred
    pub timestamp: Instant,
}

/// State machine for managing session lifecycle
#[derive(Debug)]
pub struct SessionStateMachine {
    /// Current state
    current: SessionState,
    /// Time when current state was entered
    state_entered_at: Instant,
    /// History of state transitions
    history: Vec<StateTransition>,
    /// Maximum history size to keep
    max_history: usize,
}

impl Default for SessionStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionStateMachine {
    /// Default maximum history entries
    const DEFAULT_MAX_HISTORY: usize = 100;

    /// Creates a new state machine in Idle state
    pub fn new() -> Self {
        Self {
            current: SessionState::Idle,
            state_entered_at: Instant::now(),
            history: Vec::new(),
            max_history: Self::DEFAULT_MAX_HISTORY,
        }
    }

    /// Creates a state machine with custom max history
    pub fn with_max_history(max_history: usize) -> Self {
        Self {
            max_history,
            ..Self::new()
        }
    }

    /// Returns the current state
    pub fn current(&self) -> SessionState {
        self.current
    }

    /// Returns true if the transition is valid
    pub fn can_transition(&self, to: SessionState) -> bool {
        self.current.valid_transitions().contains(&to)
    }

    /// Attempts to transition to a new state
    ///
    /// Returns Ok(()) if successful, or Err with the reason for failure.
    pub fn transition(&mut self, to: SessionState) -> Result<(), SessionError> {
        if !self.can_transition(to) {
            return Err(SessionError::InvalidStateTransition {
                from: self.current.to_string(),
                to: to.to_string(),
            });
        }

        let transition = StateTransition {
            from: self.current,
            to,
            timestamp: Instant::now(),
        };

        self.current = to;
        self.state_entered_at = transition.timestamp;

        // Add to history, trimming if necessary
        self.history.push(transition);
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }

        Ok(())
    }

    /// Force transition to a state (bypasses validation)
    ///
    /// Use sparingly, mainly for error recovery scenarios.
    pub fn force_transition(&mut self, to: SessionState) {
        let transition = StateTransition {
            from: self.current,
            to,
            timestamp: Instant::now(),
        };

        self.current = to;
        self.state_entered_at = transition.timestamp;
        self.history.push(transition);

        if self.history.len() > self.max_history {
            self.history.remove(0);
        }
    }

    /// Returns how long we've been in the current state
    pub fn time_in_state(&self) -> std::time::Duration {
        self.state_entered_at.elapsed()
    }

    /// Returns the instant when the current state was entered
    pub fn state_entered_at(&self) -> Instant {
        self.state_entered_at
    }

    /// Returns the transition history
    pub fn history(&self) -> &[StateTransition] {
        &self.history
    }

    /// Returns the last transition, if any
    pub fn last_transition(&self) -> Option<&StateTransition> {
        self.history.last()
    }

    /// Resets the state machine to Idle
    pub fn reset(&mut self) {
        self.current = SessionState::Idle;
        self.state_entered_at = Instant::now();
        self.history.clear();
    }

    /// Returns true if the session is in an active data-transmitting state
    pub fn is_active(&self) -> bool {
        self.current.is_data_ready()
    }

    /// Returns true if the session has reached a terminal state
    pub fn is_terminated(&self) -> bool {
        self.current.is_terminal()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let sm = SessionStateMachine::new();
        assert_eq!(sm.current(), SessionState::Idle);
        assert!(!sm.is_active());
        assert!(!sm.is_terminated());
    }

    #[test]
    fn test_valid_transition() {
        let mut sm = SessionStateMachine::new();

        assert!(sm.transition(SessionState::Connecting).is_ok());
        assert_eq!(sm.current(), SessionState::Connecting);

        assert!(sm.transition(SessionState::Authenticating).is_ok());
        assert_eq!(sm.current(), SessionState::Authenticating);

        assert!(sm.transition(SessionState::Active).is_ok());
        assert_eq!(sm.current(), SessionState::Active);
        assert!(sm.is_active());
    }

    #[test]
    fn test_invalid_transition() {
        let mut sm = SessionStateMachine::new();

        // Cannot go directly from Idle to Active
        let result = sm.transition(SessionState::Active);
        assert!(result.is_err());
        assert_eq!(sm.current(), SessionState::Idle);
    }

    #[test]
    fn test_pause_resume() {
        let mut sm = SessionStateMachine::new();

        sm.transition(SessionState::Connecting).unwrap();
        sm.transition(SessionState::Authenticating).unwrap();
        sm.transition(SessionState::Active).unwrap();

        // Pause
        assert!(sm.transition(SessionState::Paused).is_ok());
        assert!(!sm.is_active());

        // Resume
        assert!(sm.transition(SessionState::Active).is_ok());
        assert!(sm.is_active());
    }

    #[test]
    fn test_disconnection_from_any_connected_state() {
        for start_state in [
            SessionState::Active,
            SessionState::Paused,
            SessionState::Authenticating,
        ] {
            let mut sm = SessionStateMachine::new();
            sm.force_transition(start_state);

            assert!(sm.transition(SessionState::Disconnecting).is_ok());
            assert!(sm.transition(SessionState::Disconnected).is_ok());
            assert!(sm.is_terminated());
        }
    }

    #[test]
    fn test_history_tracking() {
        let mut sm = SessionStateMachine::new();

        sm.transition(SessionState::Connecting).unwrap();
        sm.transition(SessionState::Authenticating).unwrap();
        sm.transition(SessionState::Active).unwrap();

        assert_eq!(sm.history().len(), 3);

        let last = sm.last_transition().unwrap();
        assert_eq!(last.from, SessionState::Authenticating);
        assert_eq!(last.to, SessionState::Active);
    }

    #[test]
    fn test_history_trimming() {
        let mut sm = SessionStateMachine::with_max_history(2);

        sm.transition(SessionState::Connecting).unwrap();
        sm.transition(SessionState::Authenticating).unwrap();
        sm.transition(SessionState::Active).unwrap();

        // Only last 2 transitions should be kept
        assert_eq!(sm.history().len(), 2);
    }

    #[test]
    fn test_time_in_state() {
        let sm = SessionStateMachine::new();
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(sm.time_in_state().as_millis() >= 10);
    }

    #[test]
    fn test_state_display() {
        assert_eq!(SessionState::Active.to_string(), "Active");
        assert_eq!(SessionState::Disconnected.to_string(), "Disconnected");
    }

    #[test]
    fn test_reset() {
        let mut sm = SessionStateMachine::new();
        sm.transition(SessionState::Connecting).unwrap();
        sm.transition(SessionState::Authenticating).unwrap();

        sm.reset();

        assert_eq!(sm.current(), SessionState::Idle);
        assert!(sm.history().is_empty());
    }
}
