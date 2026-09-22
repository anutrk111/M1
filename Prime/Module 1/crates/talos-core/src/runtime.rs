//! Runtime bootstrap: config load, tracing, shutdown signal.

use crate::error::TalosError;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use talos_config::{self, ObservabilityConfig, TalosConfig};
use tracing_subscriber::{fmt, EnvFilter};

/// Process-wide accept-work gate flipped false on shutdown.
#[derive(Clone, Debug, Default)]
pub struct ShutdownGate {
    accepting: Arc<AtomicBool>,
}

impl ShutdownGate {
    pub fn new() -> Self {
        Self {
            accepting: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn is_accepting(&self) -> bool {
        self.accepting.load(Ordering::SeqCst)
    }

    pub fn begin_shutdown(&self) {
        self.accepting.store(false, Ordering::SeqCst);
    }
}

pub struct Bootstrap {
    pub config: TalosConfig,
    pub shutdown: ShutdownGate,
}

/// Load config from `config_dir`, initialize tracing, return bootstrap handle.
pub fn bootstrap(config_dir: impl AsRef<Path>) -> Result<Bootstrap, TalosError> {
    let config = talos_config::load(config_dir)?;
    init_tracing(&config.observability)?;
    Ok(Bootstrap {
        config,
        shutdown: ShutdownGate::new(),
    })
}

pub fn init_tracing(obs: &ObservabilityConfig) -> Result<(), TalosError> {
    let filter = EnvFilter::try_new(&obs.log_level)
        .or_else(|_| EnvFilter::try_new("info"))
        .map_err(|e| TalosError::Config(format!("invalid log filter: {e}")))?;

    let subscriber = fmt()
        .with_env_filter(filter)
        .with_target(true);

    if obs.json_logs {
        subscriber.json().try_init().ok();
    } else {
        subscriber.try_init().ok();
    }
    Ok(())
}

/// Wait for SIGINT / SIGTERM then flip the shutdown gate.
pub async fn wait_for_shutdown(gate: ShutdownGate) -> Result<(), TalosError> {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = signal(SignalKind::terminate())
            .map_err(|e| TalosError::Internal(format!("signal hook: {e}")))?;
        let mut sigint = signal(SignalKind::interrupt())
            .map_err(|e| TalosError::Internal(format!("signal hook: {e}")))?;
        tokio::select! {
            _ = sigterm.recv() => {}
            _ = sigint.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c()
            .await
            .map_err(|e| TalosError::Internal(format!("ctrl_c: {e}")))?;
    }
    gate.begin_shutdown();
    tracing::info!("shutdown signal received; stopped accepting work");
    Ok(())
}
