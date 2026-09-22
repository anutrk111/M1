//! Talos worker — runs the M01 fixture pipeline spine offline.
//!
//! Usage:
//!   talos-worker run-fixture --out decision.json
//!   talos-worker serve-health --bind 127.0.0.1:8080

use std::env;
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use talos_core::fixture::fixture_pipeline;
use talos_core::health::{self, Readiness};
use talos_core::pipeline::FrameContext;
use talos_core::runtime;
use talos_core::util::{new_batch_id, new_frame_id, new_trace_id, sha256_hex};
use talos_types::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        print_usage();
        std::process::exit(2);
    }
    let cmd = args.remove(0);
    match cmd.as_str() {
        "run-fixture" => run_fixture(&args).await,
        "serve-health" => serve_health(&args).await,
        "help" | "-h" | "--help" => {
            print_usage();
            Ok(())
        }
        other => {
            eprintln!("unknown command: {other}");
            print_usage();
            std::process::exit(2);
        }
    }
}

fn print_usage() {
    eprintln!(
        "talos-worker commands:\n  run-fixture [--config DIR] [--out PATH]\n  serve-health [--config DIR] [--bind ADDR]"
    );
}

fn config_dir_from(args: &[String]) -> PathBuf {
    args.windows(2)
        .find(|w| w[0] == "--config")
        .map(|w| PathBuf::from(&w[1]))
        .unwrap_or_else(talos_config::default_config_dir)
}

fn flag_value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|w| w[0] == name)
        .map(|w| w[1].as_str())
}

async fn run_fixture(args: &[String]) -> anyhow::Result<()> {
    let config_dir = config_dir_from(args);
    let out = flag_value(args, "--out").unwrap_or("decision.json");

    // Pretty logs for CLI; still validate full config.
    let mut boot = runtime::bootstrap(&config_dir)?;
    boot.config.observability.json_logs = false;

    let pipeline = fixture_pipeline(boot.config.decision.clone())?;

    let image_bytes = b"fixture-image-bytes";
    let intake = IntakeEnvelope::new(
        new_batch_id(),
        new_frame_id(),
        SourceRef {
            kind: SourceKind::Folder,
            path_or_key: "fixtures/sample.jpg".into(),
        },
        ImageRef {
            content_type: "image/jpeg".into(),
            sha256: sha256_hex(image_bytes),
            bytes_ref: "fixture://sample.jpg".into(),
        },
    );

    let mut ctx = FrameContext::new(intake, new_trace_id());
    pipeline.run(&mut ctx).await?;
    let decision = ctx.to_decision_json()?;
    let json = serde_json::to_string_pretty(&decision)?;
    fs::write(out, &json)?;
    tracing::info!(path = out, outcome = ?decision.outcome, "wrote decision JSON");
    println!("{json}");
    Ok(())
}

async fn serve_health(args: &[String]) -> anyhow::Result<()> {
    let config_dir = config_dir_from(args);
    let bind = flag_value(args, "--bind").unwrap_or("127.0.0.1:8080");
    let boot = runtime::bootstrap(&config_dir)?;
    let readiness = Readiness::new(true);
    let app = health::routes(readiness);
    let addr: SocketAddr = bind.parse()?;
    tracing::info!(%addr, "health server listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = runtime::wait_for_shutdown(boot.shutdown).await;
        })
        .await?;
    Ok(())
}
