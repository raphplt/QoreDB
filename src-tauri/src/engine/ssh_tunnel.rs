//! SSH Tunnel
//!
//! Provides SSH tunneling for connecting to databases behind firewalls.
//! Uses the native OpenSSH client for maximum compatibility.

use std::process::Stdio;

use tokio::process::{Child, Command};

use crate::engine::error::{EngineError, EngineResult};
use crate::engine::types::{SshAuth, SshTunnelConfig};

/// Represents an active SSH tunnel using native OpenSSH
pub struct SshTunnel {
    local_port: u16,
    process: Option<Child>,
}

impl SshTunnel {
    /// Opens an SSH tunnel to the remote database using native OpenSSH
    ///
    /// This spawns an `ssh -L` process for port forwarding.
    /// Requires OpenSSH to be installed on the system.
    pub async fn open(
        config: &SshTunnelConfig,
        remote_host: &str,
        remote_port: u16,
    ) -> EngineResult<Self> {
        // Find an available local port
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| EngineError::SshError {
                message: format!("Failed to bind local port: {}", e),
            })?;

        let local_port = listener
            .local_addr()
            .map_err(|e| EngineError::SshError {
                message: format!("Failed to get local address: {}", e),
            })?
            .port();

        // Drop the listener so ssh can bind to this port
        drop(listener);

        // Build SSH command
        // ssh -N -L local_port:remote_host:remote_port user@ssh_host -p ssh_port
        let mut cmd = Command::new("ssh");

        cmd.arg("-N") // Don't execute remote command
            .arg("-o")
            .arg("StrictHostKeyChecking=no")
            .arg("-o")
            .arg("UserKnownHostsFile=/dev/null")
            .arg("-o")
            .arg("ExitOnForwardFailure=yes")
            .arg("-L")
            .arg(format!(
                "{}:{}:{}",
                local_port, remote_host, remote_port
            ))
            .arg("-p")
            .arg(config.port.to_string());

        // Add authentication
        match &config.auth {
            SshAuth::Password { .. } => {
                // Password auth requires sshpass or similar
                // For now, we'll rely on ssh-agent or key-based auth
                return Err(EngineError::SshError {
                    message: "Password authentication not supported. Use SSH keys instead.".into(),
                });
            }
            SshAuth::Key {
                private_key_path,
                passphrase: _,
            } => {
                cmd.arg("-i").arg(private_key_path);
            }
        }

        cmd.arg(format!("{}@{}", config.username, config.host));

        // Spawn the SSH process
        let process = cmd
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| EngineError::SshError {
                message: format!("Failed to spawn SSH process: {}. Is OpenSSH installed?", e),
            })?;

        // Give SSH time to establish the tunnel
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

        Ok(Self {
            local_port,
            process: Some(process),
        })
    }

    /// Returns the local port to connect to
    pub fn local_port(&self) -> u16 {
        self.local_port
    }

    /// Returns the local address to use for database connection
    pub fn local_addr(&self) -> String {
        format!("127.0.0.1:{}", self.local_port)
    }

    /// Closes the tunnel
    pub async fn close(&mut self) -> EngineResult<()> {
        if let Some(mut process) = self.process.take() {
            process.kill().await.map_err(|e| EngineError::SshError {
                message: format!("Failed to kill SSH process: {}", e),
            })?;
        }
        Ok(())
    }
}

impl Drop for SshTunnel {
    fn drop(&mut self) {
        if let Some(mut process) = self.process.take() {
            // Best effort kill on drop
            let _ = process.start_kill();
        }
    }
}
