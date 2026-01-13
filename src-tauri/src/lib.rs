// QoreDB - Modern local-first database client
// Core library

pub mod commands;
pub mod engine;

use std::sync::Arc;
use tokio::sync::Mutex;

use engine::drivers::mongodb::MongoDriver;
use engine::drivers::mysql::MySqlDriver;
use engine::drivers::postgres::PostgresDriver;
use engine::{DriverRegistry, SessionManager};

/// Shared application state type
pub type SharedState = Arc<Mutex<AppState>>;

/// Application state shared across Tauri commands
pub struct AppState {
    pub registry: Arc<DriverRegistry>,
    pub session_manager: SessionManager,
}

impl AppState {
    pub fn new() -> Self {
        let mut registry = DriverRegistry::new();

        // Register all built-in drivers
        registry.register(Arc::new(PostgresDriver::new()));
        registry.register(Arc::new(MySqlDriver::new()));
        registry.register(Arc::new(MongoDriver::new()));

        let registry = Arc::new(registry);
        let session_manager = SessionManager::new(Arc::clone(&registry));

        Self {
            registry,
            session_manager,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state: SharedState = Arc::new(Mutex::new(AppState::new()));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            // Connection commands
            commands::connection::test_connection,
            commands::connection::connect,
            commands::connection::disconnect,
            commands::connection::list_sessions,
            // Query commands
            commands::query::execute_query,
            commands::query::cancel_query,
            commands::query::list_namespaces,
            commands::query::list_collections,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
