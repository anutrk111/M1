//! Talos normalized wire format → frozen M01 types.
//!
//! Malformed provider output (bad geometry, out-of-range scores, boolean HSRP,
//! non-visual VLM evidence) is a provider defect: `TalosError::Permanent`, no
//! failover (ADR-0007), never silently repaired.

use serde::Deserialize;
use talos_core::TalosError;
use talos_types::{
    BoundingBox, CueObservation, Detection, DetectionId, Detections, Evidence, EvidenceKind,
    HsrpEvidence, OcrHypothesis, ProviderRef,
};

fn malformed(provider: &ProviderRef, what: impl std::fmt::Display) -> TalosError {
    TalosError::Permanent(format!("provider {} {what}", provider.name))
}

fn check_unit(provider: &ProviderRef, field: &str, v: f32) -> Result<(), TalosError> {
    if v.is_finite() && (0.0..=1.0).contains(&v) {
        Ok(())
    } else {
        Err(malformed(provider, format!("{field}={v} outside [0,1]")))
    }
}

fn parse<T: for<'de> Deserialize<'de>>(
    provider: &ProviderRef,
    body: serde_json::Value,
    what: &str,
) -> Result<T, TalosError> {
    serde_json::from_value(body).map_err(|e| malformed(provider, format!("invalid {what}: {e}")))
}

#[derive(Deserialize)]
struct DetectWire {
    #[serde(default)]
    vehicles: Vec<DetWire>,
    #[serde(default)]
    plates: Vec<DetWire>,
}

#[derive(Deserialize)]
struct DetWire {
    #[serde(default)]
    label: Option<String>,
    score: f32,
    bbox: BoundingBox,
}

/// Raw detections carry no `DetectionId`: M05 mints Talos ids (ADR-0011).
pub fn detections(
    provider: &ProviderRef,
    body: serde_json::Value,
) -> Result<Detections, TalosError> {
    let wire: DetectWire = parse(provider, body, "detection payload")?;
    let convert = |d: DetWire, default_label: &str| -> Result<Detection, TalosError> {
        check_unit(provider, "score", d.score)?;
        Ok(Detection {
            label: d.label.unwrap_or_else(|| default_label.to_owned()),
            score: d.score,
            bbox: d.bbox,
            detection_id: None,
            provider: Some(provider.clone()),
        })
    };
    Ok(Detections {
        vehicles: wire
            .vehicles
            .into_iter()
            .map(|d| convert(d, "vehicle"))
            .collect::<Result<_, _>>()?,
        plates: wire
            .plates
            .into_iter()
            .map(|d| convert(d, "plate"))
            .collect::<Result<_, _>>()?,
        summary: None,
    })
}

#[derive(Deserialize)]
struct OcrWire {
    hypotheses: Vec<HypWire>,
}

#[derive(Deserialize)]
struct HypWire {
    text: String,
    confidence: f32,
    #[serde(default)]
    char_confidences: Vec<f32>,
}

pub fn ocr(
    provider: &ProviderRef,
    detection_id: &DetectionId,
    body: serde_json::Value,
) -> Result<Vec<OcrHypothesis>, TalosError> {
    let wire: OcrWire = parse(provider, body, "ocr payload")?;
    wire.hypotheses
        .into_iter()
        .map(|h| {
            check_unit(provider, "confidence", h.confidence)?;
            for c in &h.char_confidences {
                check_unit(provider, "char_confidence", *c)?;
            }
            if h.text.trim().is_empty() {
                return Err(malformed(provider, "returned an empty OCR hypothesis"));
            }
            Ok(OcrHypothesis {
                detection_id: detection_id.clone(),
                text: h.text,
                confidence: h.confidence,
                char_confidences: h.char_confidences,
                provider: Some(provider.clone()),
            })
        })
        .collect()
}

#[derive(Deserialize)]
struct HsrpWire {
    score: f32,
    ind_mark: CueObservation,
    hologram: CueObservation,
    geometry: CueObservation,
    #[serde(default)]
    notes: Vec<String>,
}

pub fn hsrp(
    provider: &ProviderRef,
    detection_id: &DetectionId,
    body: serde_json::Value,
) -> Result<HsrpEvidence, TalosError> {
    let w: HsrpWire = parse(provider, body, "hsrp payload")?;
    check_unit(provider, "score", w.score)?;
    Ok(HsrpEvidence {
        detection_id: detection_id.clone(),
        score: w.score,
        ind_mark: w.ind_mark,
        hologram: w.hologram,
        geometry: w.geometry,
        notes: w.notes,
    })
}

#[derive(Deserialize)]
struct VlmWire {
    confidence: f32,
    #[serde(default)]
    observation: serde_json::Value,
    #[serde(default)]
    hints: serde_json::Value,
    #[serde(default)]
    evidence_kind: Option<EvidenceKind>,
}

/// ADR-0006: VLM output is additive `VisualObservation` only. A provider that
/// claims any other evidence kind is rejected outright.
pub fn vlm(
    provider: &ProviderRef,
    body: serde_json::Value,
) -> Result<(Evidence, serde_json::Value), TalosError> {
    let w: VlmWire = parse(provider, body, "vlm payload")?;
    check_unit(provider, "confidence", w.confidence)?;
    if let Some(kind) = w.evidence_kind {
        if kind != EvidenceKind::VisualObservation {
            return Err(malformed(
                provider,
                "attempted non-VisualObservation VLM evidence",
            ));
        }
    }
    let source = format!(
        "vlm.{}.{}",
        provider.name,
        provider.model.as_deref().unwrap_or("unknown")
    );
    let evidence = Evidence::visual(source, w.observation, w.confidence);
    ensure_visual(&evidence)?;
    Ok((evidence, w.hints))
}

pub fn ensure_visual(evidence: &Evidence) -> Result<(), TalosError> {
    if evidence.kind != EvidenceKind::VisualObservation {
        return Err(TalosError::Validation(
            "gateway may only emit VisualObservation evidence".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn p() -> ProviderRef {
        ProviderRef {
            name: "p".into(),
            model: Some("m".into()),
        }
    }

    #[test]
    fn detections_reject_out_of_range_bbox_and_score() {
        let bad_box = json!({"plates":[{"score":0.9,"bbox":{"x_min":0.1,"y_min":0.1,"x_max":1.4,"y_max":0.2}}]});
        assert_eq!(
            detections(&p(), bad_box).unwrap_err().class_label(),
            "permanent"
        );
        let bad_score = json!({"plates":[{"score":1.9,"bbox":{"x_min":0.1,"y_min":0.1,"x_max":0.4,"y_max":0.2}}]});
        assert_eq!(
            detections(&p(), bad_score).unwrap_err().class_label(),
            "permanent"
        );
        let pixels = json!({"plates":[{"score":0.9,"bbox":{"x":10,"y":10,"w":40,"h":20}}]});
        assert!(detections(&p(), pixels).is_err());
    }

    #[test]
    fn detections_have_no_minted_ids() {
        let ok = json!({"plates":[{"score":0.9,"bbox":{"x_min":0.1,"y_min":0.1,"x_max":0.4,"y_max":0.2}}]});
        let d = detections(&p(), ok).unwrap();
        assert_eq!(d.plates.len(), 1);
        assert!(d.plates[0].detection_id.is_none());
        assert_eq!(d.plates[0].provider.as_ref().unwrap().name, "p");
    }

    #[test]
    fn hsrp_rejects_booleans() {
        let body = json!({"score":0.8,"ind_mark":true,"hologram":"OBSERVED","geometry":"OBSERVED"});
        assert!(hsrp(&p(), &DetectionId::new("d"), body).is_err());
    }

    #[test]
    fn vlm_rejects_external_database_evidence() {
        let body = json!({"confidence":0.9,"evidence_kind":"external_database_verification"});
        let err = vlm(&p(), body).unwrap_err();
        assert!(err.to_string().contains("non-VisualObservation"));
        let ok = vlm(
            &p(),
            json!({"confidence":0.4,"evidence_kind":"visual_observation"}),
        )
        .unwrap();
        assert_eq!(ok.0.kind, EvidenceKind::VisualObservation);
        assert_eq!(ok.0.source, "vlm.p.m");
    }

    #[test]
    fn ocr_binds_detection_id_and_rejects_empty_text() {
        let d = DetectionId::new("det-9");
        let h = ocr(
            &p(),
            &d,
            json!({"hypotheses":[{"text":"CG04AB1234","confidence":0.9}]}),
        )
        .unwrap();
        assert_eq!(h[0].detection_id, d);
        assert!(ocr(
            &p(),
            &d,
            json!({"hypotheses":[{"text":" ","confidence":0.9}]})
        )
        .is_err());
    }
}
