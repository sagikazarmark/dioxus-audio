//! Audio recording, playback, analysis, and reusable UI components for Dioxus.

pub mod analysis;
pub mod components;
pub mod decoding;
pub mod devices;
mod error;
pub mod playback;
pub mod recorder;
mod types;
pub mod waveform;

pub use error::{AudioError, AudioErrorKind};
pub use types::{
    AudioData, AudioInputDevice, AudioInputId, RecordedAudio, RecordingChunk, RecordingId,
};
