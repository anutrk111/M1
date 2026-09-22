//! Review API — M01 health spine; VERIFY/CORRECT/REJECT/UNREADABLE in M09.

use std::net::SocketAddr;
use talos_core::health::{self, Readiness};
use talos_core::runtime;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config_dir = talos_config::default_config_dir();
    let boot = runtime::bootstrap(&config_dir)?;
    let readiness = Readiness::new(true);
    let app = health::routes(readiness);
    let addr: SocketAddr = "127.0.0.1:8082".parse()?;
    tracing::info!(%addr, "review-api listening (health only)");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = runtime::wait_for_shutdown(boot.shutdown).await;
        })
        .await?;
    Ok(())
}
