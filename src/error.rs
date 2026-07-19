//! Audio errors shared by platform adapters.

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AudioErrorKind {
    UnsupportedPlatform,
    PermissionDenied,
    DeviceNotFound,
    DeviceUnavailable,
    Overconstrained,
    InvalidConfiguration,
    RecorderFailure,
    PlaybackFailure,
    Backend,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioError {
    kind: AudioErrorKind,
    message: String,
    overconstrained_constraint: Option<String>,
}

impl AudioError {
    pub fn new(kind: AudioErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            overconstrained_constraint: None,
        }
    }

    /// Create an exact-constraint acquisition failure.
    ///
    /// An empty browser constraint name is treated as unavailable detail.
    pub fn overconstrained(constraint: impl Into<String>, message: impl Into<String>) -> Self {
        let constraint = constraint.into();
        Self {
            kind: AudioErrorKind::Overconstrained,
            message: message.into(),
            overconstrained_constraint: (!constraint.is_empty()).then_some(constraint),
        }
    }

    pub fn unsupported() -> Self {
        Self::new(
            AudioErrorKind::UnsupportedPlatform,
            "audio capture and playback require the wasm32 web backend",
        )
    }

    pub fn kind(&self) -> AudioErrorKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    /// Browser constraint name associated with an overconstraint failure.
    pub fn overconstrained_constraint(&self) -> Option<&str> {
        self.overconstrained_constraint.as_deref()
    }
}

impl fmt::Display for AudioError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for AudioError {}
