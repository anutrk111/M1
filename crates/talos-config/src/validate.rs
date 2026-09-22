use crate::error::ConfigError;
use crate::model::TalosConfig;

pub fn validate(config: &TalosConfig) -> Result<(), ConfigError> {
    let d = &config.decision;
    for (name, value) in [
        ("auto_approve_min", d.auto_approve_min),
        ("secondary_min", d.secondary_min),
    ] {
        if !(0.0..=1.0).contains(&value) {
            return Err(ConfigError::Validation(format!(
                "{name} must be in [0.0, 1.0], got {value}"
            )));
        }
    }
    if !(d.secondary_min < d.auto_approve_min) {
        return Err(ConfigError::Validation(format!(
            "secondary_min ({}) must be < auto_approve_min ({})",
            d.secondary_min, d.auto_approve_min
        )));
    }
    if config.runtime.max_in_flight_frames == 0 {
        return Err(ConfigError::Validation(
            "max_in_flight_frames must be >= 1".into(),
        ));
    }
    if !(0.0..=1.0).contains(&config.observability.trace_sample_rate) {
        return Err(ConfigError::Validation(
            "trace_sample_rate must be in [0.0, 1.0]".into(),
        ));
    }
    Ok(())
}
