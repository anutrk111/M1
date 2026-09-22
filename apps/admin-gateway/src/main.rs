//! Admin gateway — M01 health spine; dashboard/WebSocket in M12.

use std::net::SocketAddr;
use talos_core::health::{self, Readiness};
use talos_core::runtime;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config_dir = talos_config::default_config_dir();
    let boot = runtime::bootstrap(&config_dir)?;
    let readiness = Readiness::new(true);
    let app = health::routes(readiness);
    let addr: SocketAddr = "127.0.0.1:8083".parse()?;
    tracing::info!(%addr, "admin-gateway listening (health only)");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = runtime::wait_for_shutdown(boot.shutdown).await;
        })
        .await?;
    Ok(())
}
