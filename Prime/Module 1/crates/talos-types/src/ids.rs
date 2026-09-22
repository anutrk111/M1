use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Deref;

macro_rules! id_newtype {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Deref for $name {
            type Target = str;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }
    };
}

id_newtype!(BatchId, "One intake unit (folder, ZIP, or upload set).");
id_newtype!(JobId, "Optional sub-work unit inside a batch.");
id_newtype!(FrameId, "One image / frame under processing.");
id_newtype!(
    CameraId,
    "Optional metadata from CSV/XLSX — not a live feed handle."
);
id_newtype!(TraceId, "Correlation ID for logs and spans.");
id_newtype!(
    DetectionId,
    "Talos-owned plate/vehicle detection lineage id (M05+)."
);
id_newtype!(
    ConfigRevisionId,
    "Immutable config revision fingerprint governing a decision (M12)."
);
