//! Platform-neutral audio value types.

use std::fmt;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioData {
    pub bytes: Vec<u8>,
    pub mime_type: String,
}

impl AudioData {
    pub fn new(bytes: Vec<u8>, mime_type: impl Into<String>) -> Self {
        Self {
            bytes,
            mime_type: mime_type.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AudioInputId(String);

impl AudioInputId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AudioInputId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioInputDevice {
    pub id: AudioInputId,
    pub label: String,
    pub is_default: bool,
}

impl AudioInputDevice {
    pub fn new(id: AudioInputId, label: impl Into<String>, is_default: bool) -> Self {
        Self {
            id,
            label: label.into(),
            is_default,
        }
    }

    pub fn display_label(&self, index: usize) -> String {
        if self.label.trim().is_empty() {
            format!("Microphone {}", index + 1)
        } else {
            self.label.clone()
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordedAudio {
    pub audio: AudioData,
    pub duration: Duration,
    pub peaks: Vec<u8>,
    pub input_device: Option<AudioInputId>,
}
