//! G0 frozen-contract tests: golden JSON round-trips and invariants.

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use talos_types::*;

fn fixture(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"))
}

/// Parse golden JSON, re-serialize, and require a structurally identical document.
fn round_trip<T: Serialize + DeserializeOwned>(name: &str) -> T {
    let raw = fixture(name);
    let parsed: T = serde_json::from_str(&raw).unwrap_or_else(|e| panic!("{name}: {e}"));
    let encoded = serde_json::to_string(&parsed).unwrap();
    let golden: Value = serde_json::from_str(&raw).unwrap();
    let again: Value = serde_json::from_str(&encoded).unwrap();
    assert_eq!(golden, again, "{name} did not round-trip");
    parsed
}

#[test]
fn golden_bbox_round_trips() {
    let b: BoundingBox = round_trip("bbox.json");
    assert!(b.validate().is_ok());
}

#[test]
fn golden_hsrp_evidence_is_ternary() {
    let h: HsrpEvidence = round_trip("hsrp_evidence.json");
    assert_eq!(h.ind_mark, CueObservation::Observed);
    assert_eq!(h.hologram, CueObservation::NotObservable);
    assert_eq!(h.geometry, CueObservation::NotObserved);
    assert_eq!(h.detection_id, DetectionId::new("det-01"));
}

#[test]
fn golden_ocr_result_lineage_consistent() {
    let r: OcrResult = round_trip("ocr_result.json");
    r.validate_lineage().unwrap();
    assert!(r.grammar.is_valid());
}

#[test]
fn golden_machine_decision() {
    let d: MachineDecision = round_trip("machine_decision.json");
    assert_eq!(d.outcome, MachineOutcome::ReviewRequired);
}

#[test]
fn golden_review_event() {
    let e: ReviewEvent = round_trip("review_event.json");
    assert_eq!(e.role, Role::Clerk);
    assert_eq!(e.action, ReviewAction::Correct);
    assert!(e.corrected_plate_text.is_some());
}

#[test]
fn golden_export_row() {
    let r: ExportRow = round_trip("export_row.json");
    assert_eq!(r.final_status, FinalStatus::Corrected);
    assert_eq!(r.hsrp_status, CueObservation::NotObservable);
}

#[test]
fn golden_cost_event() {
    let c: CostEvent = round_trip("cost_event.json");
    assert_eq!(c.operation, AiOperation::DetectVehiclesPlates);
}

#[test]
fn legacy_boolean_hsrp_is_rejected() {
    let raw = fixture("legacy_hsrp_bool.json");
    assert!(serde_json::from_str::<HsrpEvidence>(&raw).is_err());
}

#[test]
fn legacy_rectified_without_lineage_is_rejected() {
    let raw = fixture("legacy_rectified_no_lineage.json");
    assert!(serde_json::from_str::<RectifiedPlate>(&raw).is_err());
}

#[test]
fn missing_detection_id_rejected_on_downstream_artifacts() {
    let ocr = r#"{"text":"CG04AB1234","confidence":0.9}"#;
    assert!(serde_json::from_str::<OcrHypothesis>(ocr).is_err());
    let grammar = r#"{"normalized_text":"X","status":"VALID","errors":[]}"#;
    assert!(serde_json::from_str::<GrammarResult>(grammar).is_err());
}

#[test]
fn bbox_rejects_invalid_values() {
    assert!(BoundingBox::new(-0.1, 0.0, 0.5, 0.5).is_err());
    assert!(BoundingBox::new(0.0, 0.0, 1.1, 0.5).is_err());
    assert!(BoundingBox::new(0.5, 0.0, 0.5, 0.5).is_err());
    assert!(BoundingBox::new(0.6, 0.0, 0.5, 0.5).is_err());
    assert!(BoundingBox::new(f32::NAN, 0.0, 0.5, 0.5).is_err());
    let out_of_range = r#"{"x_min":0.1,"y_min":0.1,"x_max":1.5,"y_max":0.5}"#;
    assert!(serde_json::from_str::<BoundingBox>(out_of_range).is_err());
    let legacy_pixels = r#"{"x":100.0,"y":200.0,"w":240.0,"h":60.0}"#;
    assert!(serde_json::from_str::<BoundingBox>(legacy_pixels).is_err());
}

#[test]
fn bbox_pixel_conversion_round_trip() {
    let b = BoundingBox::from_pixels(100, 200, 240, 60, 1000, 500).unwrap();
    let px = b.to_pixel_rect(1000, 500).unwrap();
    assert_eq!(
        px,
        PixelRect {
            x: 100,
            y: 200,
            w: 240,
            h: 60
        }
    );
    // Resolution independence: same bbox on a 2x frame scales pixels.
    let px2 = b.to_pixel_rect(2000, 1000).unwrap();
    assert_eq!((px2.x, px2.y, px2.w, px2.h), (200, 400, 480, 120));
    assert!(BoundingBox::from_pixels(900, 0, 200, 10, 1000, 500).is_err());
    assert!(b.to_pixel_rect(0, 500).is_err());
}

#[test]
fn bbox_pixel_rect_is_at_least_one_pixel_and_clamped() {
    let tiny = BoundingBox::new(0.5, 0.5, 0.5001, 0.5001).unwrap();
    let px = tiny.to_pixel_rect(10, 10).unwrap();
    assert!(px.w >= 1 && px.h >= 1);
    let full = BoundingBox::new(0.0, 0.0, 1.0, 1.0).unwrap();
    assert_eq!(
        full.to_pixel_rect(640, 480).unwrap(),
        PixelRect {
            x: 0,
            y: 0,
            w: 640,
            h: 480
        }
    );
}

#[test]
fn ternary_serde_names_are_frozen() {
    for (v, s) in [
        (CueObservation::Observed, "\"OBSERVED\""),
        (CueObservation::NotObserved, "\"NOT_OBSERVED\""),
        (CueObservation::NotObservable, "\"NOT_OBSERVABLE\""),
    ] {
        assert_eq!(serde_json::to_string(&v).unwrap(), s);
    }
    assert!(serde_json::from_str::<CueObservation>("false").is_err());
}

#[test]
fn ocr_result_lineage_mismatch_detected() {
    let mut r: OcrResult = serde_json::from_str(&fixture("ocr_result.json")).unwrap();
    r.grammar.detection_id = DetectionId::new("other");
    assert!(r.validate_lineage().is_err());
}

#[test]
fn machine_outcome_mapping_is_conservative() {
    assert_eq!(
        MachineOutcome::from(&DecisionOutcome::AutoApproved),
        MachineOutcome::Accept
    );
    assert_eq!(
        MachineOutcome::from(&DecisionOutcome::SecondaryVerification),
        MachineOutcome::ReviewRequired
    );
    assert_eq!(
        MachineOutcome::from(&DecisionOutcome::ReviewRequired),
        MachineOutcome::ReviewRequired
    );
}

#[test]
fn super_admin_has_no_review_authority() {
    let perms = Role::SuperAdmin.default_permissions();
    assert!(!perms.contains(&Permission::ReviewAct));
    assert!(!perms.contains(&Permission::ReviewOverride));
    assert!(Role::ReviewingOfficer
        .default_permissions()
        .contains(&Permission::ReviewOverride));
    assert!(!Role::Clerk
        .default_permissions()
        .contains(&Permission::ReviewOverride));
    assert!(Role::Clerk
        .default_permissions()
        .contains(&Permission::IntakeSubmit));
}

#[test]
fn schema_versions() {
    assert_eq!(INTAKE_SCHEMA_VERSION, "2.0");
    assert_eq!(DECISION_SCHEMA_VERSION, "2.1");
    assert_eq!(SCHEMA_VERSION, INTAKE_SCHEMA_VERSION);
}
