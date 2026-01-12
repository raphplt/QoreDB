//! Session Manager
//!
//! Centralized management of all active database sessions.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::engine::error::{EngineError, EngineResult};
use crate::engine::traits::DataEngine;
use crate::engine::types::{ConnectionConfig, SessionId};
use crate::engine::DriverRegistry;

/// Information about an active session
#[derive(Debug)]
pub struct SessionInfo {
    pub driver_id: String,
    pub config: ConnectionConfig,
    pub display_name: String,
}

/// Manages all active database sessions
pub struct SessionManager {
    registry: Arc<DriverRegistry>,
    sessions: RwLock<HashMap<SessionId, SessionInfo>>,
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

        driver.test_connection(config).await
    }

    /// Establishes a new connection and returns its session ID
    pub async fn connect(&self, config: ConnectionConfig) -> EngineResult<SessionId> {
        let driver = self
            .registry
            .get(&config.driver)
            .ok_or_else(|| EngineError::driver_not_found(&config.driver))?;

        let session_id = driver.connect(&config).await?;

        let display_name = format!(
            "{}@{}:{}",
            config.username,
            config.host,
            config.database.as_deref().unwrap_or("default")
        );

        let info = SessionInfo {
            driver_id: config.driver.clone(),
            config,
            display_name,
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id, info);

        Ok(session_id)
    }

    /// Disconnects a session
    pub async fn disconnect(&self, session_id: SessionId) -> EngineResult<()> {
        let info = {
            let mut sessions = self.sessions.write().await;
            sessions
                .remove(&session_id)
                .ok_or_else(|| EngineError::session_not_found(session_id.0.to_string()))?
        };

        let driver = self
            .registry
            .get(&info.driver_id)
            .ok_or_else(|| EngineError::driver_not_found(&info.driver_id))?;

        driver.disconnect(session_id).await
    }

    /// Gets a driver for an existing session
    pub async fn get_driver(&self, session_id: SessionId) -> EngineResult<Arc<dyn DataEngine>> {
        let sessions = self.sessions.read().await;
        let info = sessions
            .get(&session_id)
            .ok_or_else(|| EngineError::session_not_found(session_id.0.to_string()))?;

        self.registry
            .get(&info.driver_id)
            .ok_or_else(|| EngineError::driver_not_found(&info.driver_id))
    }

    /// Lists all active sessions
    pub async fn list_sessions(&self) -> Vec<(SessionId, String)> {
        let sessions = self.sessions.read().await;
        sessions
            .iter()
            .map(|(id, info)| (*id, info.display_name.clone()))
            .collect()
    }

    /// Gets session info
    pub async fn get_session_info(&self, session_id: SessionId) -> Option<String> {
        let sessions = self.sessions.read().await;
        sessions.get(&session_id).map(|i| i.display_name.clone())
    }
}
