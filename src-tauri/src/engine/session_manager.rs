//! Session Manager
//!
//! Centralized management of all active database sessions.
//! This is the SINGLE SOURCE OF TRUTH for all connection state.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::engine::error::{EngineError, EngineResult};
use crate::engine::ssh_tunnel::SshTunnel;
use crate::engine::traits::DataEngine;
use crate::engine::types::{ConnectionConfig, SessionId};
use crate::engine::DriverRegistry;

/// Active session with its connection pool and optional tunnel
pub struct ActiveSession {
    pub driver_id: String,
    pub config: ConnectionConfig,
    pub display_name: String,
    pub tunnel: Option<SshTunnel>,
}

/// Manages all active database sessions
/// This is the SINGLE SOURCE OF TRUTH - pools are stored here, not in drivers.
pub struct SessionManager {
    registry: Arc<DriverRegistry>,
    sessions: RwLock<HashMap<SessionId, ActiveSession>>,
}

impl SessionManager {
    pub fn new(registry: Arc<DriverRegistry>) -> Self {
        Self {
            registry,
            sessions: RwLock::new(HashMap::new()),
        }
    }

    /// Tests a connection without persisting it
    pub async fn test_connection(&self, config: &ConnectionConfig) -> EngineResult<()> {
        let driver = self
            .registry
            .get(&config.driver)
            .ok_or_else(|| EngineError::driver_not_found(&config.driver))?;

        // If SSH tunnel is configured, we need to test through it
        let effective_config = if let Some(ref ssh_config) = config.ssh_tunnel {
            let tunnel = SshTunnel::open(ssh_config, &config.host, config.port).await?;
            let mut tunneled_config = config.clone();
            tunneled_config.host = "127.0.0.1".to_string();
            tunneled_config.port = tunnel.local_port();
            // Tunnel will be dropped after test, closing the connection
            let result = driver.test_connection(&tunneled_config).await;
            return result;
        } else {
            config.clone()
        };

        driver.test_connection(&effective_config).await
    }

    /// Establishes a new connection and returns its session ID
    pub async fn connect(&self, config: ConnectionConfig) -> EngineResult<SessionId> {
        let driver = self
            .registry
            .get(&config.driver)
            .ok_or_else(|| EngineError::driver_not_found(&config.driver))?;

        // Setup SSH tunnel if configured
        let (effective_config, tunnel) = if let Some(ref ssh_config) = config.ssh_tunnel {
            let tunnel = SshTunnel::open(ssh_config, &config.host, config.port).await?;
            let mut tunneled_config = config.clone();
            tunneled_config.host = "127.0.0.1".to_string();
            tunneled_config.port = tunnel.local_port();
            (tunneled_config, Some(tunnel))
        } else {
            (config.clone(), None)
        };

        let session_id = driver.connect(&effective_config).await?;

        let display_name = format!(
            "{}@{}:{}{}",
            config.username,
            config.host,
            config.database.as_deref().unwrap_or("default"),
            if tunnel.is_some() { " (SSH)" } else { "" }
        );

        let session = ActiveSession {
            driver_id: config.driver.clone(),
            config,
            display_name,
            tunnel,
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id, session);

        Ok(session_id)
    }

    /// Disconnects a session
    pub async fn disconnect(&self, session_id: SessionId) -> EngineResult<()> {
        let mut session = {
            let mut sessions = self.sessions.write().await;
            sessions
                .remove(&session_id)
                .ok_or_else(|| EngineError::session_not_found(session_id.0.to_string()))?
        };

        let driver = self
            .registry
            .get(&session.driver_id)
            .ok_or_else(|| EngineError::driver_not_found(&session.driver_id))?;

        // Disconnect from database
        driver.disconnect(session_id).await?;

        // Close SSH tunnel if present
        if let Some(ref mut tunnel) = session.tunnel {
            tunnel.close().await?;
        }

        Ok(())
    }

    /// Gets a driver for an existing session
    pub async fn get_driver(&self, session_id: SessionId) -> EngineResult<Arc<dyn DataEngine>> {
        let sessions = self.sessions.read().await;
        let session = sessions
            .get(&session_id)
            .ok_or_else(|| EngineError::session_not_found(session_id.0.to_string()))?;

        self.registry
            .get(&session.driver_id)
            .ok_or_else(|| EngineError::driver_not_found(&session.driver_id))
    }

    /// Lists all active sessions
    pub async fn list_sessions(&self) -> Vec<(SessionId, String)> {
        let sessions = self.sessions.read().await;
        sessions
            .iter()
            .map(|(id, session)| (*id, session.display_name.clone()))
            .collect()
    }

    /// Gets session info
    pub async fn get_session_info(&self, session_id: SessionId) -> Option<String> {
        let sessions = self.sessions.read().await;
        sessions.get(&session_id).map(|s| s.display_name.clone())
    }

    /// Checks if a session exists
    pub async fn session_exists(&self, session_id: SessionId) -> bool {
        let sessions = self.sessions.read().await;
        sessions.contains_key(&session_id)
    }
}
