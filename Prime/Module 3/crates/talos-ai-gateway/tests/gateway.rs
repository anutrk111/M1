//! M03 gateway integration tests. Offline: fixture provider, wiremock on
//! loopback, and scripted in-process providers. No real network or paid APIs.

use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use talos_ai_gateway::config::{
    BreakerConfig, OperationRoute, ProviderConfig, ProviderKind, RateLimitConfig,
};
use talos_ai_gateway::*;
use talos_core::TalosError;
use talos_types::*;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn req(detection: Option<&str>) -> VisionRequest {
    VisionRequest {
        trace_id: TraceId::new("trace-1"),
        frame_id: FrameId::new("frame-1"),
        batch_id: BatchId::new("batch-1"),
        detection_id: detection.map(DetectionId::new),
        image_bytes: Some(vec![0xFF, 0xD8, 0xFF, 0xE0]),
        bytes_ref: None,
        content_type: "image/jpeg".into(),
    }
}

fn fast_cfg() -> AiConfig {
    let mut cfg = AiConfig::fixture_defaults();
    cfg.retry.base_delay_ms = 1;
    cfg.retry.max_delay_ms = 2;
    cfg.rate_limit = RateLimitConfig {
        requests_per_second: 0.0,
        burst: 1,
    };
    cfg
}

fn route(primary: &str, fallbacks: &[&str]) -> OperationRoute {
    OperationRoute {
        primary: primary.into(),
        fallbacks: fallbacks.iter().map(|s| s.to_string()).collect(),
    }
}

/// Register an enabled http_json provider whose key lives in a test-unique env var.
fn http_provider(cfg: &mut AiConfig, name: &str, uri: &str) {
    let env = format!("TALOS_TEST_KEY_{}", name.to_ascii_uppercase());
    std::env::set_var(&env, "test-key");
    cfg.providers.insert(
        name.into(),
        ProviderConfig {
            enabled: true,
            kind: ProviderKind::HttpJson,
            base_url: Some(uri.into()),
            detect_model: Some("det-model".into()),
            ocr_model: Some("ocr-model".into()),
            hsrp_model: None,
            vlm_model: Some("vlm-model".into()),
            api_key_env: Some(env),
            rate_limit: None,
        },
    );
}

fn detect_ok() -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({
        "result": {
            "plates": [{ "score": 0.93, "bbox": {"x_min":0.1,"y_min":0.4,"x_max":0.3,"y_max":0.46} }],
            "vehicles": []
        },
        "usage": { "input_tokens": 100, "output_tokens": 20, "usd_estimate": 0.002 }
    }))
}

type Script = dyn Fn(usize) -> Result<ProviderResponse, TalosError> + Send + Sync;

struct Scripted {
    name: String,
    calls: AtomicUsize,
    script: Box<Script>,
}

impl Scripted {
    fn new(
        name: &str,
        script: impl Fn(usize) -> Result<ProviderResponse, TalosError> + Send + Sync + 'static,
    ) -> Arc<Self> {
        Arc::new(Self {
            name: name.into(),
            calls: AtomicUsize::new(0),
            script: Box::new(script),
        })
    }
    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl Provider for Scripted {
    fn name(&self) -> &str {
        &self.name
    }
    async fn call(&self, _req: &ProviderRequest) -> Result<ProviderResponse, TalosError> {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        (self.script)(n)
    }
}

fn body(v: Value) -> Result<ProviderResponse, TalosError> {
    Ok(ProviderResponse {
        body: v,
        ..Default::default()
    })
}

fn ocr_body(text: &str) -> Result<ProviderResponse, TalosError> {
    body(json!({"hypotheses":[{"text": text, "confidence": 0.9}]}))
}

#[tokio::test]
async fn fixture_round_trip_to_m01_types() {
    let cost = Arc::new(InMemoryCostRecorder::default());
    let gw = AiGateway::from_config(AiConfig::fixture_defaults(), cost.clone()).unwrap();

    let det = gw.detect_vehicles_plates(req(None)).await.unwrap();
    assert_eq!(det.plates.len(), 1);
    assert!(
        det.plates[0].detection_id.is_none(),
        "M05 mints DetectionIds"
    );
    let encoded = serde_json::to_string(&det).unwrap();
    assert_eq!(serde_json::from_str::<Detections>(&encoded).unwrap(), det);

    let ocr = gw.run_ocr(req(Some("det-1"))).await.unwrap();
    assert_eq!(ocr[0].text, "CG04AB1234");
    assert_eq!(ocr[0].detection_id, DetectionId::new("det-1"));

    let hsrp = gw.analyze_hsrp(req(Some("det-1"))).await.unwrap();
    assert_eq!(hsrp.ind_mark, CueObservation::Observed);

    let vlm = gw
        .vlm_assist(VlmRequest {
            vision: req(None),
            prompt_context: "ambiguous band".into(),
        })
        .await
        .unwrap();
    assert_eq!(vlm.evidence.kind, EvidenceKind::VisualObservation);
    assert_eq!(vlm.evidence.source, "vlm.fixture.fixture-v1");

    let events = cost.events();
    assert_eq!(events.len(), 4);
    assert!(events
        .iter()
        .all(|e| e.status == CostStatus::Ok && e.provider == "fixture"));
}

#[tokio::test]
async fn http_429_retries_then_fails_over() {
    let primary = MockServer::start().await;
    let secondary = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/detect_vehicles_plates"))
        .respond_with(ResponseTemplate::new(429))
        .expect(3)
        .mount(&primary)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/detect_vehicles_plates"))
        .and(header("authorization", "Bearer test-key"))
        .respond_with(detect_ok())
        .expect(1)
        .mount(&secondary)
        .await;

    let mut cfg = fast_cfg();
    http_provider(&mut cfg, "p429a", &primary.uri());
    http_provider(&mut cfg, "p429b", &secondary.uri());
    cfg.operations.detect = route("p429a", &["p429b"]);
    let cost = Arc::new(InMemoryCostRecorder::default());
    let gw = AiGateway::from_config(cfg, cost.clone()).unwrap();

    let det = gw.detect_vehicles_plates(req(None)).await.unwrap();
    let p = det.plates[0].provider.as_ref().unwrap();
    assert_eq!(p.name, "p429b");
    assert_eq!(p.model.as_deref(), Some("det-model"));

    let events = cost.events();
    assert_eq!(events.len(), 4, "3 failed attempts + 1 success");
    assert_eq!(
        events
            .iter()
            .filter(|e| e.error_class.as_deref() == Some("transient"))
            .count(),
        3
    );
    let ok = events.last().unwrap();
    assert_eq!((ok.input_tokens, ok.output_tokens), (Some(100), Some(20)));
    assert_eq!(ok.usd_estimate, Some(0.002));

    let m = gw.metrics();
    assert_eq!(
        m.counter("talos_ai_fallbacks_total{operation=detect_vehicles_plates,from_provider=p429a,to_provider=p429b}"),
        1
    );
    assert_eq!(
        m.counter("talos_ai_requests_total{provider=p429a,operation=detect_vehicles_plates,status=transient}"),
        3
    );
}

#[tokio::test]
async fn transient_then_success_on_same_provider() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(detect_ok())
        .mount(&server)
        .await;
    let mut cfg = fast_cfg();
    http_provider(&mut cfg, "p503", &server.uri());
    cfg.operations.detect = route("p503", &[]);
    let gw = AiGateway::from_config(cfg, Arc::new(NoopCostRecorder)).unwrap();
    assert_eq!(
        gw.detect_vehicles_plates(req(None))
            .await
            .unwrap()
            .plates
            .len(),
        1
    );
}

#[tokio::test]
async fn timeout_is_transient() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(detect_ok().set_delay(Duration::from_millis(400)))
        .mount(&server)
        .await;
    let mut cfg = fast_cfg();
    cfg.timeouts_ms.detect = 50;
    cfg.retry.max_retries = 0;
    http_provider(&mut cfg, "pslow", &server.uri());
    cfg.operations.detect = route("pslow", &[]);
    let gw = AiGateway::from_config(cfg, Arc::new(NoopCostRecorder)).unwrap();
    let err = gw.detect_vehicles_plates(req(None)).await.unwrap_err();
    assert!(matches!(err, TalosError::Transient(_)), "{err}");
}

#[tokio::test]
async fn http_400_is_validation_without_failover() {
    let primary = MockServer::start().await;
    let secondary = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(400).set_body_string("plate CG04AB1234 bad"))
        .expect(1)
        .mount(&primary)
        .await;
    Mock::given(method("POST"))
        .respond_with(detect_ok())
        .expect(0)
        .mount(&secondary)
        .await;
    let mut cfg = fast_cfg();
    http_provider(&mut cfg, "p400a", &primary.uri());
    http_provider(&mut cfg, "p400b", &secondary.uri());
    cfg.operations.detect = route("p400a", &["p400b"]);
    let gw = AiGateway::from_config(cfg, Arc::new(NoopCostRecorder)).unwrap();
    let err = gw.detect_vehicles_plates(req(None)).await.unwrap_err();
    assert!(matches!(err, TalosError::Validation(_)));
    assert!(
        !err.to_string().contains("CG04AB1234"),
        "body must not leak"
    );
}

#[tokio::test]
async fn http_401_is_config_not_retried_not_billed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(401))
        .expect(1)
        .mount(&server)
        .await;
    let mut cfg = fast_cfg();
    http_provider(&mut cfg, "p401", &server.uri());
    cfg.operations.detect = route("p401", &[]);
    let cost = Arc::new(InMemoryCostRecorder::default());
    let gw = AiGateway::from_config(cfg, cost.clone()).unwrap();
    let err = gw.detect_vehicles_plates(req(None)).await.unwrap_err();
    assert!(matches!(err, TalosError::Config(_)));
    assert!(cost.events().is_empty());
}

#[tokio::test]
async fn malformed_provider_geometry_is_permanent() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "result": {"plates":[{"score":0.9,"bbox":{"x_min":0.5,"y_min":0.1,"x_max":0.2,"y_max":0.3}}]}
        })))
        .mount(&server)
        .await;
    let mut cfg = fast_cfg();
    http_provider(&mut cfg, "pgeom", &server.uri());
    cfg.operations.detect = route("pgeom", &[]);
    let gw = AiGateway::from_config(cfg, Arc::new(NoopCostRecorder)).unwrap();
    let err = gw.detect_vehicles_plates(req(None)).await.unwrap_err();
    assert!(matches!(err, TalosError::Permanent(_)));
}

#[tokio::test]
async fn vlm_non_visual_evidence_rejected_over_http() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/vlm_assist"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "result": {"confidence":0.99,"evidence_kind":"external_database_verification"}
        })))
        .mount(&server)
        .await;
    let mut cfg = fast_cfg();
    http_provider(&mut cfg, "pvlm", &server.uri());
    cfg.operations.vlm = route("pvlm", &[]);
    let gw = AiGateway::from_config(cfg, Arc::new(NoopCostRecorder)).unwrap();
    let err = gw
        .vlm_assist(VlmRequest {
            vision: req(None),
            prompt_context: "x".into(),
        })
        .await
        .unwrap_err();
    assert!(err.to_string().contains("non-VisualObservation"));
}

#[tokio::test]
async fn vlm_disabled_is_not_implemented() {
    let mut cfg = fast_cfg();
    cfg.vlm.enabled = false;
    let gw = AiGateway::from_config(cfg, Arc::new(NoopCostRecorder)).unwrap();
    let err = gw
        .vlm_assist(VlmRequest {
            vision: req(None),
            prompt_context: "x".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, TalosError::NotImplemented(_)));
}

#[test]
fn enabled_provider_without_key_fails_closed() {
    let mut cfg = fast_cfg();
    cfg.providers.insert(
        "nokey".into(),
        ProviderConfig {
            enabled: true,
            kind: ProviderKind::HttpJson,
            base_url: Some("https://api.example.invalid".into()),
            detect_model: None,
            ocr_model: None,
            hsrp_model: None,
            vlm_model: None,
            api_key_env: Some("TALOS_TEST_KEY_DEFINITELY_UNSET".into()),
            rate_limit: None,
        },
    );
    let err = AiGateway::from_config(cfg, Arc::new(NoopCostRecorder))
        .err()
        .expect("must fail closed");
    assert!(matches!(err, TalosError::Config(_)));
    assert!(err.to_string().contains("TALOS_TEST_KEY_DEFINITELY_UNSET"));
}

#[test]
fn live_mode_rejects_fixture_routes_and_unknown_providers() {
    let mut cfg = fast_cfg();
    cfg.default_mode = GatewayMode::Live;
    assert!(matches!(
        AiGateway::from_config(cfg.clone(), Arc::new(NoopCostRecorder)).err(),
        Some(TalosError::Config(_))
    ));
    cfg.default_mode = GatewayMode::Fixture;
    cfg.operations.ocr = route("fixture", &["ghost"]);
    let err = AiGateway::from_config(cfg, Arc::new(NoopCostRecorder))
        .err()
        .unwrap();
    assert!(err.to_string().contains("ghost"));
}

#[tokio::test]
async fn ocr_and_hsrp_require_lineage_before_any_call() {
    let p = Scripted::new("lineage", |_| ocr_body("X"));
    let mut cfg = fast_cfg();
    cfg.operations.ocr = route("lineage", &[]);
    cfg.operations.hsrp = route("lineage", &[]);
    let gw = AiGateway::builder(cfg)
        .with_provider(p.clone())
        .build()
        .unwrap();
    assert!(matches!(
        gw.run_ocr(req(None)).await,
        Err(TalosError::Validation(_))
    ));
    assert!(matches!(
        gw.analyze_hsrp(req(None)).await,
        Err(TalosError::Validation(_))
    ));
    assert_eq!(p.calls(), 0);
}

#[tokio::test]
async fn request_validation_rejects_bad_inputs() {
    let gw = AiGateway::from_config(fast_cfg(), Arc::new(NoopCostRecorder)).unwrap();
    let mut both = req(None);
    both.bytes_ref = Some("file:///x.jpg".into());
    assert!(matches!(
        gw.detect_vehicles_plates(both).await,
        Err(TalosError::Validation(_))
    ));
    let mut gif = req(None);
    gif.content_type = "image/gif".into();
    assert!(matches!(
        gw.detect_vehicles_plates(gif).await,
        Err(TalosError::Validation(_))
    ));
    let mut s3 = req(None);
    s3.image_bytes = None;
    s3.bytes_ref = Some("s3://bucket/key.jpg".into());
    assert!(matches!(
        gw.detect_vehicles_plates(s3).await,
        Err(TalosError::NotImplemented(_))
    ));
}

fn file_req(path: impl AsRef<std::path::Path>) -> VisionRequest {
    let mut r = req(None);
    r.image_bytes = None;
    r.bytes_ref = Some(format!("file://{}", path.as_ref().display()));
    r
}

/// Gateway authorized to read only `root`, with a cost recorder to prove no egress.
fn rooted_gateway(root: &std::path::Path) -> (AiGateway, Arc<InMemoryCostRecorder>) {
    let mut cfg = fast_cfg();
    cfg.local_artifact_roots = vec![root.to_path_buf()];
    let cost = Arc::new(InMemoryCostRecorder::default());
    (AiGateway::from_config(cfg, cost.clone()).unwrap(), cost)
}

#[tokio::test]
async fn file_bytes_ref_inside_authorized_root_is_resolved() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("staging");
    std::fs::create_dir_all(root.join("batch")).unwrap();
    let path = root.join("batch/frame 1.jpg");
    std::fs::write(&path, [0xFF, 0xD8, 0xFF]).unwrap();
    let (gw, _) = rooted_gateway(&root);
    let det = gw.detect_vehicles_plates(file_req(&path)).await.unwrap();
    assert_eq!(det.plates.len(), 1);
}

#[tokio::test]
async fn file_bytes_ref_refused_without_configured_roots() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("frame.jpg");
    std::fs::write(&path, [0xFF, 0xD8, 0xFF]).unwrap();
    let gw = AiGateway::from_config(fast_cfg(), Arc::new(NoopCostRecorder)).unwrap();
    assert!(matches!(
        gw.detect_vehicles_plates(file_req(&path)).await,
        Err(TalosError::Config(_))
    ));
}

#[tokio::test]
async fn file_bytes_ref_outside_root_rejected_without_egress() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("staging");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("ok.jpg"), [0xFF, 0xD8, 0xFF]).unwrap();
    let secret = dir.path().join("secret.jpg");
    std::fs::write(&secret, [0xFF, 0xD8, 0xFF]).unwrap();
    // Sibling that shares the root's string prefix.
    let sibling = dir.path().join("staging-evil");
    std::fs::create_dir_all(&sibling).unwrap();
    std::fs::write(sibling.join("x.jpg"), [0xFF, 0xD8, 0xFF]).unwrap();
    let (gw, cost) = rooted_gateway(&root);

    for (label, path) in [
        ("absolute outside", secret.clone()),
        ("dot-dot escape", root.join("../secret.jpg")),
        ("prefix sibling", sibling.join("x.jpg")),
        ("root itself", root.clone()),
    ] {
        let err = gw
            .detect_vehicles_plates(file_req(&path))
            .await
            .unwrap_err();
        assert!(
            matches!(err, TalosError::Validation(_)),
            "{label}: expected Validation, got {err:?}"
        );
    }
    for path in ["/etc/passwd", "relative/frame.jpg"] {
        let err = gw.detect_vehicles_plates(file_req(path)).await.unwrap_err();
        assert!(
            matches!(err, TalosError::Validation(_) | TalosError::Permanent(_)),
            "{path}: {err:?}"
        );
    }
    assert!(
        cost.events().is_empty(),
        "no provider call for refused refs"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn file_bytes_ref_symlink_escape_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("staging");
    std::fs::create_dir_all(&root).unwrap();
    let outside = dir.path().join("outside.jpg");
    std::fs::write(&outside, [0xFF, 0xD8, 0xFF]).unwrap();
    std::os::unix::fs::symlink(&outside, root.join("link.jpg")).unwrap();
    std::os::unix::fs::symlink(dir.path(), root.join("dirlink")).unwrap();
    let (gw, cost) = rooted_gateway(&root);

    for path in [root.join("link.jpg"), root.join("dirlink/outside.jpg")] {
        let err = gw
            .detect_vehicles_plates(file_req(&path))
            .await
            .unwrap_err();
        assert!(matches!(err, TalosError::Validation(_)), "{err:?}");
    }
    assert!(cost.events().is_empty());
}

#[tokio::test]
async fn file_bytes_ref_over_size_cap_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("big.jpg");
    std::fs::write(&path, vec![0xFF; 64]).unwrap();
    let mut cfg = fast_cfg();
    cfg.local_artifact_roots = vec![dir.path().to_path_buf()];
    cfg.max_request_image_bytes = 63;
    let gw = AiGateway::from_config(cfg, Arc::new(NoopCostRecorder)).unwrap();
    assert!(matches!(
        gw.detect_vehicles_plates(file_req(&path)).await,
        Err(TalosError::Validation(_))
    ));
}

#[tokio::test(start_paused = true)]
async fn circuit_breaker_opens_then_half_open_probe_closes() {
    let p = Scripted::new("flaky", |n| {
        if n < 2 {
            Err(TalosError::Transient("down".into()))
        } else {
            body(json!({"plates":[],"vehicles":[]}))
        }
    });
    let mut cfg = fast_cfg();
    cfg.retry.max_retries = 0;
    cfg.circuit_breaker = BreakerConfig {
        failure_threshold: 2,
        open_ms: 10_000,
        half_open_max_calls: 1,
    };
    cfg.operations.detect = route("flaky", &[]);
    let gw = AiGateway::builder(cfg)
        .with_provider(p.clone())
        .build()
        .unwrap();

    assert!(gw.detect_vehicles_plates(req(None)).await.is_err());
    assert!(gw.detect_vehicles_plates(req(None)).await.is_err());
    let health = gw.health().into_iter().find(|h| h.name == "flaky").unwrap();
    assert_eq!(health.breaker, BreakerState::Open);
    assert_eq!(health.error_total, 2);

    let err = gw.detect_vehicles_plates(req(None)).await.unwrap_err();
    assert!(err.to_string().contains("circuit open"));
    assert_eq!(p.calls(), 2, "open circuit must not call the provider");

    tokio::time::advance(Duration::from_millis(10_001)).await;
    assert!(gw.detect_vehicles_plates(req(None)).await.is_ok());
    let health = gw.health().into_iter().find(|h| h.name == "flaky").unwrap();
    assert_eq!(health.breaker, BreakerState::Closed);
}

#[tokio::test(start_paused = true)]
async fn open_circuit_fails_over_to_next_provider() {
    let down = Scripted::new("down", |_| Err(TalosError::Transient("x".into())));
    let up = Scripted::new("up", |_| body(json!({"plates":[],"vehicles":[]})));
    let mut cfg = fast_cfg();
    cfg.retry.max_retries = 0;
    cfg.circuit_breaker.failure_threshold = 1;
    cfg.operations.detect = route("down", &["up"]);
    let gw = AiGateway::builder(cfg)
        .with_provider(down.clone())
        .with_provider(up.clone())
        .build()
        .unwrap();
    gw.detect_vehicles_plates(req(None)).await.unwrap();
    gw.detect_vehicles_plates(req(None)).await.unwrap();
    assert_eq!(down.calls(), 1);
    assert_eq!(up.calls(), 2);
}

#[tokio::test(start_paused = true)]
async fn rate_limit_spaces_calls() {
    let p = Scripted::new("limited", |_| body(json!({"plates":[],"vehicles":[]})));
    let mut cfg = fast_cfg();
    cfg.rate_limit = RateLimitConfig {
        requests_per_second: 1.0,
        burst: 1,
    };
    cfg.operations.detect = route("limited", &[]);
    let gw = AiGateway::builder(cfg).with_provider(p).build().unwrap();
    let start = tokio::time::Instant::now();
    for _ in 0..3 {
        gw.detect_vehicles_plates(req(None)).await.unwrap();
    }
    assert!(start.elapsed() >= Duration::from_millis(1_990));
}

#[tokio::test]
async fn tie_breaker_consults_arbiter_only_on_disagreement() {
    let a = Scripted::new("prov-a", |_| ocr_body("CG04AB1234"));
    let b = Scripted::new("prov-b", |_| ocr_body("CG04AB1284"));
    let emperor = Scripted::new("emperor", |_| ocr_body("CG04AB1234"));
    let mut cfg = fast_cfg();
    cfg.operations.ocr = route("prov-a", &[]);
    cfg.tie_breaker.enabled = true;
    cfg.tie_breaker.secondary = Some("prov-b".into());
    cfg.tie_breaker.arbiter = Some("emperor".into());
    let gw = AiGateway::builder(cfg.clone())
        .with_provider(a.clone())
        .with_provider(b.clone())
        .with_provider(emperor.clone())
        .build()
        .unwrap();
    let c = gw.run_ocr_consensus(req(Some("det-1"))).await.unwrap();
    assert_eq!(c.agreed, Some(false));
    assert_eq!(c.arbiter.as_ref().unwrap()[0].text, "CG04AB1234");
    assert_eq!(c.all().len(), 3);
    assert!(c
        .all()
        .iter()
        .all(|h| h.detection_id == DetectionId::new("det-1")));
    assert_eq!(
        c.arbiter.as_ref().unwrap()[0]
            .provider
            .as_ref()
            .unwrap()
            .name,
        "emperor"
    );

    let b_agree = Scripted::new("prov-b", |_| ocr_body("cg-04-ab-1234"));
    let emperor2 = Scripted::new("emperor", |_| ocr_body("ZZ"));
    let gw = AiGateway::builder(cfg)
        .with_provider(a)
        .with_provider(b_agree)
        .with_provider(emperor2.clone())
        .build()
        .unwrap();
    let c = gw.run_ocr_consensus(req(Some("det-1"))).await.unwrap();
    assert_eq!(c.agreed, Some(true));
    assert!(c.arbiter.is_none());
    assert_eq!(emperor2.calls(), 0);
}

#[test]
fn tie_breaker_requires_registered_providers() {
    let mut cfg = fast_cfg();
    cfg.tie_breaker.enabled = true;
    cfg.tie_breaker.secondary = Some("fixture".into());
    assert!(AiGateway::from_config(cfg, Arc::new(NoopCostRecorder)).is_err());
}

#[tokio::test]
async fn gateway_is_usable_as_m01_port_trait_object() {
    let gw: Arc<dyn VisionGateway> =
        Arc::new(AiGateway::from_config(fast_cfg(), Arc::new(NoopCostRecorder)).unwrap());
    let c = gw.run_ocr_consensus(req(Some("d"))).await.unwrap();
    assert_eq!(c.primary.len(), 1);
    assert!(c.secondary.is_none());
}
