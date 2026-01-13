//! Vault Module
//!
//! Secure credential storage using OS-native keychain.

pub mod credentials;
pub mod lock;
pub mod storage;

pub use credentials::SavedConnection;
pub use lock::VaultLock;
pub use storage::VaultStorage;
