// Data Engine Module
// Universal abstraction layer for all database engines

pub mod drivers;
pub mod error;
pub mod registry;
pub mod session_manager;
pub mod ssh_tunnel;
pub mod traits;
pub mod types;

pub use error::EngineError;
pub use registry::DriverRegistry;
pub use session_manager::SessionManager;
pub use traits::DataEngine;
pub use types::*;

