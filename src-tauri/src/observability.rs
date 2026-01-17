//! Logging and observability helpers.

use std::fs;
use std::path::PathBuf;

use tracing_appender::rolling::RollingFileAppender;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::EnvFilter;

const LOG_FILE_PREFIX: &str = "qoredb.log";

pub fn init_tracing() {
    let log_dir = log_directory();
    let _ = fs::create_dir_all(&log_dir);

    let file_appender: RollingFileAppender = tracing_appender::rolling::daily(log_dir, LOG_FILE_PREFIX);
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("qoredb=info,tauri=info"));

    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(file_appender)
        .with_ansi(false)
        .with_span_events(FmtSpan::CLOSE)
        .try_init();
}

fn log_directory() -> PathBuf {
    if cfg!(windows) {
        let appdata = std::env::var_os("APPDATA")
            .unwrap_or_else(|| std::env::var_os("USERPROFILE").unwrap_or_default());
        let mut path = PathBuf::from(appdata);
        path.push("QoreDB");
        path.push("logs");
        path
    } else {
        let home = std::env::var_os("HOME").unwrap_or_default();
        let mut path = PathBuf::from(home);
        path.push(".qoredb");
        path.push("logs");
        path
    }
}
