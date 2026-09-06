// SPDX-License-Identifier: Apache-2.0

//! Gate between the vault and the headless agent surfaces (MCP server, CLI).
//! A connection reaches an agent only when the user opted in, and always in
//! read-only mode.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use qore_core::{ConnectionConfig, SessionId};

use crate::vault::credentials::{SavedConnection, StoredCredentials};

pub const IDLE_SESSION_TIMEOUT: Duration = Duration::from_secs(10 * 60);

pub fn exposed_connections(all: Vec<SavedConnection>) -> Vec<SavedConnection> {
    all.into_iter().filter(|c| c.expose_to_agents).collect()
}

pub fn require_exposed(connection: &SavedConnection) -> Result<(), String> {
    if connection.expose_to_agents {
        return Ok(());
    }
    Err(format!(
        "Connection '{}' is not exposed to AI agents. Enable it in QoreDB under \
         Settings > AI agents to allow access.",
        connection.id
    ))
}

/// Builds the config an agent may connect with: exposure is checked and the
/// session is forced read-only whatever the saved flag says.
pub fn agent_connection_config(
    connection: &SavedConnection,
    creds: &StoredCredentials,
) -> Result<ConnectionConfig, String> {
    require_exposed(connection)?;
    let mut config = connection
        .to_connection_config(creds)
        .map_err(|e| e.sanitized_message())?;
    config.read_only = true;
    Ok(config)
}

pub fn connection_summary(connection: &SavedConnection) -> serde_json::Value {
    serde_json::json!({
        "id": connection.id,
        "name": connection.name,
        "driver": connection.driver,
        "host": connection.host,
        "database": connection.database,
        "environment": connection.environment.as_str(),
        "read_only": true,
    })
}

/// Sessions opened on behalf of an agent, keyed by connection id. Sessions
/// idle for longer than [`IDLE_SESSION_TIMEOUT`] are handed back by
/// [`AgentSessions::take_idle`] so the caller can close them; there is no
/// background task.
#[derive(Default)]
pub struct AgentSessions {
    entries: HashMap<String, (SessionId, Instant)>,
}

impl AgentSessions {
    pub fn get(&mut self, connection_id: &str) -> Option<SessionId> {
        let (session, last_used) = self.entries.get_mut(connection_id)?;
        *last_used = Instant::now();
        Some(*session)
    }

    pub fn insert(&mut self, connection_id: String, session: SessionId) {
        self.entries
            .insert(connection_id, (session, Instant::now()));
    }

    pub fn take_idle(&mut self) -> Vec<SessionId> {
        self.take_idle_at(Instant::now())
    }

    fn take_idle_at(&mut self, now: Instant) -> Vec<SessionId> {
        let idle: Vec<String> = self
            .entries
            .iter()
            .filter(|(_, (_, last))| now.duration_since(*last) >= IDLE_SESSION_TIMEOUT)
            .map(|(id, _)| id.clone())
            .collect();
        idle.iter()
            .filter_map(|id| self.entries.remove(id).map(|(session, _)| session))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensitive::Sensitive;
    use crate::vault::credentials::Environment;

    fn connection(id: &str, expose: bool) -> SavedConnection {
        SavedConnection {
            options: Default::default(),
            id: id.to_string(),
            name: id.to_string(),
            driver: "postgres".to_string(),
            environment: Environment::Production,
            read_only: false,
            expose_to_agents: expose,
            host: "localhost".to_string(),
            port: 5432,
            username: "qoredb".to_string(),
            database: None,
            ssl: false,
            ssl_mode: None,
            pool_max_connections: None,
            pool_min_connections: None,
            pool_acquire_timeout_secs: None,
            ssh_tunnel: None,
            proxy: None,
            mssql_auth: None,
            clickhouse_cluster: None,
            search_auth_mode: None,
            ssl_ca_cert: None,
            project_id: "default".to_string(),
        }
    }

    fn creds() -> StoredCredentials {
        StoredCredentials {
            db_password: Sensitive::new("pw".to_string()),
            ssh_password: None,
            ssh_key_passphrase: None,
            proxy_password: None,
        }
    }

    #[test]
    fn only_exposed_connections_are_listed() {
        let listed = exposed_connections(vec![
            connection("hidden", false),
            connection("visible", true),
        ]);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "visible");
    }

    #[test]
    fn legacy_connections_are_hidden_by_default() {
        let legacy = r#"{
            "id":"c1","name":"legacy","driver":"postgres",
            "environment":"development","read_only":false,
            "host":"localhost","port":5432,"username":"u","database":null,
            "ssl":false,"ssh_tunnel":null,"project_id":"default"
        }"#;
        let parsed: SavedConnection = serde_json::from_str(legacy).unwrap();
        assert!(!parsed.expose_to_agents);
        assert!(require_exposed(&parsed).is_err());
    }

    #[test]
    fn unexposed_connection_is_refused_even_by_id() {
        let err = agent_connection_config(&connection("secret", false), &creds()).unwrap_err();
        assert!(err.contains("not exposed"), "{err}");
        assert!(err.contains("secret"));
    }

    #[test]
    fn exposed_connection_is_forced_read_only() {
        let config = agent_connection_config(&connection("prod", true), &creds()).unwrap();
        assert!(config.read_only);
        assert_eq!(config.environment, "production");
    }

    #[test]
    fn idle_sessions_are_evicted_after_timeout() {
        let mut sessions = AgentSessions::default();
        let fresh = SessionId(uuid::Uuid::new_v4());
        let stale = SessionId(uuid::Uuid::new_v4());
        sessions.insert("fresh".to_string(), fresh);
        sessions.insert("stale".to_string(), stale);
        sessions.entries.get_mut("stale").unwrap().1 =
            Instant::now() - IDLE_SESSION_TIMEOUT - Duration::from_secs(1);

        let evicted = sessions.take_idle_at(Instant::now());
        assert_eq!(evicted, vec![stale]);
        assert_eq!(sessions.get("fresh"), Some(fresh));
        assert_eq!(sessions.get("stale"), None);
    }

    #[test]
    fn touching_a_session_keeps_it_alive() {
        let mut sessions = AgentSessions::default();
        let id = SessionId(uuid::Uuid::new_v4());
        sessions.insert("c".to_string(), id);
        sessions.entries.get_mut("c").unwrap().1 = Instant::now() - IDLE_SESSION_TIMEOUT;
        assert_eq!(sessions.get("c"), Some(id));
        assert!(sessions.take_idle().is_empty());
    }
}
