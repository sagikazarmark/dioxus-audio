//! Audio errors shared by platform adapters.

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AudioErrorKind {
    UnsupportedPlatform,
    PermissionDenied,
    DeviceNotFound,
    DeviceUnavailable,
    InvalidConfiguration,
    RecorderFailure,
    PlaybackFailure,
    Backend,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioError {
    kind: AudioErrorKind,
    message: String,
}

impl AudioError {
    pub fn new(kind: AudioErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
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
}

impl fmt::Display for AudioError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for AudioError {}
