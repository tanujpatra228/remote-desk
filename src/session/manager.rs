//! Session manager for coordinating remote desktop sessions
//!
//! This module provides a central manager for creating, tracking, and
//! managing host and client sessions.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::error::{SessionError, SessionResult};
use crate::session::client::{ClientSession, ClientSessionConfig};
use crate::session::host::{HostSession, HostSessionConfig};
use crate::session::state::SessionState;
use crate::session::transport::{create_loopback_transport, SessionTransport};

/// Unique identifier for a session
pub type SessionId = String;

/// Type of session
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionType {
    /// Host session (sharing screen)
    Host,
    /// Client session (viewing screen)
    Client,
}

/// Information about a managed session
#[derive(Debug, Clone)]
pub struct SessionInfo {
    /// Session ID
    pub id: SessionId,
    /// Session type
    pub session_type: SessionType,
    /// Current state
    pub state: SessionState,
    /// Remote peer ID (if connected)
    pub remote_id: Option<String>,
}

/// Wrapper for managed sessions
pub enum ManagedSession {
    /// Host session
    Host(HostSession),
    /// Client session
    Client(ClientSession),
}

impl ManagedSession {
    /// Returns the session ID
    pub fn id(&self) -> &str {
        match self {
            ManagedSession::Host(s) => s.session_id(),
            ManagedSession::Client(s) => s.session_id(),
        }
    }

    /// Returns the session type
    pub fn session_type(&self) -> SessionType {
        match self {
            ManagedSession::Host(_) => SessionType::Host,
            ManagedSession::Client(_) => SessionType::Client,
        }
    }
}

/// Session manager for coordinating sessions
pub struct SessionManager {
    /// Active sessions
    sessions: Arc<RwLock<HashMap<SessionId, ManagedSession>>>,
    /// Local device ID
    local_id: Option<String>,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionManager {
    /// Creates a new session manager
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            local_id: None,
        }
    }

    /// Creates a session manager with local device ID
    pub fn with_local_id(local_id: String) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            local_id: Some(local_id),
        }
    }

    /// Creates a new host session
    pub async fn create_host_session(
        &self,
        config: HostSessionConfig,
        transport: SessionTransport,
    ) -> SessionResult<SessionId> {
        let session_id = config.session_id.clone();

        // Check for existing session
        {
            let sessions = self.sessions.read().await;
            if sessions.contains_key(&session_id) {
                return Err(SessionError::SessionAlreadyExists(session_id));
            }
        }

        let session = HostSession::new(config, transport);

        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id.clone(), ManagedSession::Host(session));
        }

        info!("Created host session: {}", session_id);
        Ok(session_id)
    }

    /// Creates a new client session
    pub async fn create_client_session(
        &self,
        config: ClientSessionConfig,
        transport: SessionTransport,
    ) -> SessionResult<SessionId> {
        let session_id = config.session_id.clone();

        // Check for existing session
        {
            let sessions = self.sessions.read().await;
            if sessions.contains_key(&session_id) {
                return Err(SessionError::SessionAlreadyExists(session_id));
            }
        }

        let session = ClientSession::new(config, transport);

        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id.clone(), ManagedSession::Client(session));
        }

        info!("Created client session: {}", session_id);
        Ok(session_id)
    }

    /// Creates a loopback session (host + client connected together)
    ///
    /// This is primarily for testing and demonstration.
    /// Returns (host_session_id, client_session_id)
    pub async fn create_loopback_session(
        &self,
        host_config: HostSessionConfig,
        client_config: ClientSessionConfig,
    ) -> SessionResult<(SessionId, SessionId)> {
        // Create loopback transport
        let (host_transport, client_transport) = create_loopback_transport();

        // Create host session
        let host_id = self.create_host_session(host_config, host_transport).await?;

        // Create client session
        let client_id = self
            .create_client_session(client_config, client_transport)
            .await?;

        info!(
            "Created loopback session: host={}, client={}",
            host_id, client_id
        );

        Ok((host_id, client_id))
    }

    /// Starts a session
    pub async fn start_session(&self, session_id: &str) -> SessionResult<()> {
        let mut sessions = self.sessions.write().await;

        match sessions.get_mut(session_id) {
            Some(ManagedSession::Host(session)) => {
                session.start().await?;
                info!("Started host session: {}", session_id);
            }
            Some(ManagedSession::Client(session)) => {
                session.start().await?;
                info!("Started client session: {}", session_id);
            }
            None => {
                return Err(SessionError::SessionNotFound(session_id.to_string()));
            }
        }

        Ok(())
    }

    /// Stops a session
    pub async fn stop_session(&self, session_id: &str) -> SessionResult<()> {
        let mut sessions = self.sessions.write().await;

        match sessions.get_mut(session_id) {
            Some(ManagedSession::Host(session)) => {
                session.stop().await?;
                info!("Stopped host session: {}", session_id);
            }
            Some(ManagedSession::Client(session)) => {
                session.stop().await?;
                info!("Stopped client session: {}", session_id);
            }
            None => {
                return Err(SessionError::SessionNotFound(session_id.to_string()));
            }
        }

        Ok(())
    }

    /// Removes a session
    pub async fn remove_session(&self, session_id: &str) -> SessionResult<()> {
        let mut sessions = self.sessions.write().await;

        if sessions.remove(session_id).is_some() {
            info!("Removed session: {}", session_id);
            Ok(())
        } else {
            Err(SessionError::SessionNotFound(session_id.to_string()))
        }
    }

    /// Returns information about all sessions
    pub async fn list_sessions(&self) -> Vec<SessionInfo> {
        let sessions = self.sessions.read().await;
        let mut infos = Vec::new();

        for (id, session) in sessions.iter() {
            let info = SessionInfo {
                id: id.clone(),
                session_type: session.session_type(),
                state: SessionState::Idle, // Simplified - would need async state check
                remote_id: None,
            };
            infos.push(info);
        }

        infos
    }

    /// Returns information about a specific session
    pub async fn get_session_info(&self, session_id: &str) -> Option<SessionInfo> {
        let sessions = self.sessions.read().await;

        sessions.get(session_id).map(|session| SessionInfo {
            id: session_id.to_string(),
            session_type: session.session_type(),
            state: SessionState::Idle, // Simplified
            remote_id: None,
        })
    }

    /// Returns the number of active sessions
    pub async fn session_count(&self) -> usize {
        self.sessions.read().await.len()
    }

    /// Stops all sessions
    pub async fn stop_all_sessions(&self) -> SessionResult<()> {
        let session_ids: Vec<String> = {
            let sessions = self.sessions.read().await;
            sessions.keys().cloned().collect()
        };

        for session_id in session_ids {
            if let Err(e) = self.stop_session(&session_id).await {
                warn!("Failed to stop session {}: {}", session_id, e);
            }
        }

        Ok(())
    }

    /// Gets access to a host session (for advanced operations)
    pub async fn with_host_session<F, R>(&self, session_id: &str, f: F) -> SessionResult<R>
    where
        F: FnOnce(&mut HostSession) -> R,
    {
        let mut sessions = self.sessions.write().await;

        match sessions.get_mut(session_id) {
            Some(ManagedSession::Host(session)) => Ok(f(session)),
            Some(ManagedSession::Client(_)) => Err(SessionError::InvalidStateTransition {
                from: "Client".to_string(),
                to: "Host".to_string(),
            }),
            None => Err(SessionError::SessionNotFound(session_id.to_string())),
        }
    }

    /// Gets access to a client session (for advanced operations)
    pub async fn with_client_session<F, R>(&self, session_id: &str, f: F) -> SessionResult<R>
    where
        F: FnOnce(&mut ClientSession) -> R,
    {
        let mut sessions = self.sessions.write().await;

        match sessions.get_mut(session_id) {
            Some(ManagedSession::Client(session)) => Ok(f(session)),
            Some(ManagedSession::Host(_)) => Err(SessionError::InvalidStateTransition {
                from: "Host".to_string(),
                to: "Client".to_string(),
            }),
            None => Err(SessionError::SessionNotFound(session_id.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_manager_creation() {
        let manager = SessionManager::new();
        assert_eq!(manager.session_count().await, 0);

        let manager = SessionManager::with_local_id("123456789".to_string());
        assert_eq!(manager.local_id, Some("123456789".to_string()));
    }

    #[tokio::test]
    async fn test_create_loopback_session() {
        let manager = SessionManager::new();

        let host_config = HostSessionConfig::default();
        let client_config = ClientSessionConfig::default();

        let (host_id, client_id) = manager
            .create_loopback_session(host_config, client_config)
            .await
            .unwrap();

        assert_eq!(manager.session_count().await, 2);

        let sessions = manager.list_sessions().await;
        assert_eq!(sessions.len(), 2);

        let host_info = manager.get_session_info(&host_id).await.unwrap();
        assert_eq!(host_info.session_type, SessionType::Host);

        let client_info = manager.get_session_info(&client_id).await.unwrap();
        assert_eq!(client_info.session_type, SessionType::Client);
    }

    #[tokio::test]
    async fn test_session_not_found() {
        let manager = SessionManager::new();

        let result = manager.start_session("nonexistent").await;
        assert!(matches!(result, Err(SessionError::SessionNotFound(_))));
    }

    #[tokio::test]
    async fn test_duplicate_session() {
        let manager = SessionManager::new();
        let (host_transport, _) = create_loopback_transport();

        let config1 = HostSessionConfig::default()
            .with_session_id("test-session".to_string());
        let config2 = HostSessionConfig::default()
            .with_session_id("test-session".to_string());

        manager
            .create_host_session(config1, host_transport)
            .await
            .unwrap();

        let (host_transport2, _) = create_loopback_transport();
        let result = manager.create_host_session(config2, host_transport2).await;

        assert!(matches!(result, Err(SessionError::SessionAlreadyExists(_))));
    }

    #[tokio::test]
    async fn test_remove_session() {
        let manager = SessionManager::new();
        let (host_transport, client_transport) = create_loopback_transport();

        let host_id = manager
            .create_host_session(HostSessionConfig::default(), host_transport)
            .await
            .unwrap();

        assert_eq!(manager.session_count().await, 1);

        manager.remove_session(&host_id).await.unwrap();

        assert_eq!(manager.session_count().await, 0);
    }

    #[tokio::test]
    async fn test_stop_all_sessions() {
        let manager = SessionManager::new();

        let host_config = HostSessionConfig::default();
        let client_config = ClientSessionConfig::default();

        manager
            .create_loopback_session(host_config, client_config)
            .await
            .unwrap();

        assert_eq!(manager.session_count().await, 2);

        manager.stop_all_sessions().await.unwrap();
    }
}
