//! Shared `/healthz` and `/readyz` handlers.

use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct Readiness {
    ready: Arc<AtomicBool>,
}

impl Readiness {
    pub fn new(initial: bool) -> Self {
        Self {
            ready: Arc::new(AtomicBool::new(initial)),
        }
    }

    pub fn set_ready(&self, ready: bool) {
        self.ready.store(ready, Ordering::SeqCst);
    }

    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::SeqCst)
    }
}

#[derive(Serialize)]
struct HealthBody {
    status: &'static str,
}

async fn healthz() -> impl IntoResponse {
    (StatusCode::OK, Json(HealthBody { status: "ok" }))
}

async fn readyz(ready: axum::Extension<Readiness>) -> impl IntoResponse {
    if ready.is_ready() {
        (StatusCode::OK, Json(HealthBody { status: "ready" })).into_response()
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(HealthBody {
                status: "not_ready",
            }),
        )
            .into_response()
    }
}

/// Router exposing M01 health contract.
pub fn routes(readiness: Readiness) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .layer(axum::Extension(readiness))
}
