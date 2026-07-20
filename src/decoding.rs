//! Complete-file decoding into immutable planar samples.

use std::fmt;
use std::mem::size_of;
use std::sync::Arc;
use std::time::Duration;

use crate::AudioData;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
mod web;

/// Resource options for one complete-file decode operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecodeOptions {
    max_decoded_bytes: u64,
}

impl DecodeOptions {
    /// Default ceiling for the Rust-owned planar `f32` copy (128 MiB).
    pub const DEFAULT_MAX_DECODED_BYTES: u64 = 128 * 1024 * 1024;

    /// Override the ceiling for the Rust-owned planar `f32` copy.
    ///
    /// The browser may allocate the complete decoded PCM before this limit can
    /// be checked. A successful copy may transiently retain both representations.
    #[must_use]
    pub fn with_max_decoded_bytes(mut self, max_decoded_bytes: u64) -> Self {
        self.max_decoded_bytes = max_decoded_bytes;
        self
    }

    pub fn max_decoded_bytes(self) -> u64 {
        self.max_decoded_bytes
    }

    /// Check the Rust-owned planar `f32` bytes required by decoded metadata.
    ///
    /// Actual channel and frame counts are not known until the browser has
    /// decoded the complete input. This check therefore cannot prevent the
    /// browser's first PCM allocation.
    pub fn check_decoded_size(
        self,
        channel_count: u64,
        frame_count: u64,
    ) -> Result<u64, DecodeError> {
        if channel_count == 0 || frame_count == 0 {
            return Err(DecodeError::backend(
                "decoded channel and frame counts must be positive",
            ));
        }

        let required_bytes = channel_count
            .checked_mul(frame_count)
            .and_then(|samples| samples.checked_mul(size_of::<f32>() as u64))
            .ok_or_else(|| DecodeError::backend("decoded byte count overflowed"))?;
        if required_bytes > self.max_decoded_bytes {
            return Err(DecodeError::resource_limit(
                required_bytes,
                self.max_decoded_bytes,
            ));
        }

        Ok(required_bytes)
    }
}

impl Default for DecodeOptions {
    fn default() -> Self {
        Self {
            max_decoded_bytes: Self::DEFAULT_MAX_DECODED_BYTES,
        }
    }
}

/// Portable category for a complete-file decode failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DecodeErrorKind {
    UnsupportedPlatform,
    ResourceLimit,
    AllocationFailure,
    DecodeRejected,
    Backend,
}

/// Failure from a complete-file decode operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodeError {
    kind: DecodeErrorKind,
    message: String,
    required_bytes: Option<u64>,
    configured_bytes: Option<u64>,
}

impl DecodeError {
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    fn unsupported() -> Self {
        Self {
            kind: DecodeErrorKind::UnsupportedPlatform,
            message: "audio decoding requires the wasm32 web backend".into(),
            required_bytes: None,
            configured_bytes: None,
        }
    }

    fn resource_limit(required_bytes: u64, configured_bytes: u64) -> Self {
        Self {
            kind: DecodeErrorKind::ResourceLimit,
            message: format!(
                "decoded PCM requires {required_bytes} Rust-owned bytes, exceeding the configured {configured_bytes}-byte limit"
            ),
            required_bytes: Some(required_bytes),
            configured_bytes: Some(configured_bytes),
        }
    }

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    fn allocation_failure(required_bytes: u64) -> Self {
        Self {
            kind: DecodeErrorKind::AllocationFailure,
            message: format!(
                "could not allocate {required_bytes} bytes for decoded planar samples"
            ),
            required_bytes: Some(required_bytes),
            configured_bytes: None,
        }
    }

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    fn decode_rejected() -> Self {
        Self {
            kind: DecodeErrorKind::DecodeRejected,
            message: "browser rejected the complete audio data".into(),
            required_bytes: None,
            configured_bytes: None,
        }
    }

    fn backend(message: impl Into<String>) -> Self {
        Self {
            kind: DecodeErrorKind::Backend,
            message: message.into(),
            required_bytes: None,
            configured_bytes: None,
        }
    }

    pub fn kind(&self) -> DecodeErrorKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    /// Bytes required for the Rust-owned planar PCM copy, when known.
    pub fn required_bytes(&self) -> Option<u64> {
        self.required_bytes
    }

    /// Configured Rust-copy ceiling for a resource-limit failure.
    pub fn configured_bytes(&self) -> Option<u64> {
        self.configured_bytes
    }
}

impl fmt::Display for DecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for DecodeError {}

/// Decode complete Audio Data into immutable planar samples.
///
/// The operation consumes its input and releases the Rust encoded allocation
/// before awaiting browser decoding. Every settled or dropped browser
/// operation requests context cleanup. Dropping suppresses its result but does
/// not promise to abort work already started by the browser.
pub async fn decode_audio_data(
    audio: AudioData,
    options: DecodeOptions,
) -> Result<DecodedAudio, DecodeError> {
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        web::decode_audio_data(audio, options).await
    }

    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    {
        let _ = (audio, options);
        Err(DecodeError::unsupported())
    }
}

/// An invalid Decoded Audio sample layout.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DecodedAudioError {
    NoChannels,
    NoFrames,
    MisalignedSamples { samples: usize, channels: usize },
    InvalidSampleRate,
}

impl fmt::Display for DecodedAudioError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoChannels => formatter.write_str("channel count must be positive"),
            Self::NoFrames => formatter.write_str("frame count must be positive"),
            Self::MisalignedSamples { samples, channels } => write!(
                formatter,
                "{samples} planar samples cannot form {channels} equal channels"
            ),
            Self::InvalidSampleRate => {
                formatter.write_str("sample rate must be positive and finite")
            }
        }
    }
}

impl std::error::Error for DecodedAudioError {}

/// Immutable Decoded Audio backed by one flat-planar sample allocation.
///
/// Clones share the same sample storage.
#[derive(Clone, Debug)]
pub struct DecodedAudio {
    inner: Arc<DecodedAudioInner>,
}

#[derive(Debug)]
struct DecodedAudioInner {
    samples: Vec<f32>,
    channels: usize,
    frames: usize,
    sample_rate: f32,
    duration: Duration,
}

impl DecodedAudio {
    /// Consume flat channel-major samples with an equal positive frame count.
    pub fn from_planar(
        samples: Vec<f32>,
        channels: usize,
        sample_rate: f32,
    ) -> Result<Self, DecodedAudioError> {
        if channels == 0 {
            return Err(DecodedAudioError::NoChannels);
        }
        if samples.is_empty() {
            return Err(DecodedAudioError::NoFrames);
        }
        if !samples.len().is_multiple_of(channels) {
            return Err(DecodedAudioError::MisalignedSamples {
                samples: samples.len(),
                channels,
            });
        }
        if !sample_rate.is_finite() || sample_rate <= 0.0 {
            return Err(DecodedAudioError::InvalidSampleRate);
        }

        let frames = samples.len() / channels;
        let duration = Duration::try_from_secs_f64(frames as f64 / f64::from(sample_rate))
            .map_err(|_| DecodedAudioError::InvalidSampleRate)?;

        Ok(Self {
            inner: Arc::new(DecodedAudioInner {
                samples,
                channels,
                frames,
                sample_rate,
                duration,
            }),
        })
    }

    pub fn channel_count(&self) -> usize {
        self.inner.channels
    }

    pub fn frame_count(&self) -> usize {
        self.inner.frames
    }

    /// Effective sample rate of the decode context, in hertz.
    pub fn sample_rate(&self) -> f32 {
        self.inner.sample_rate
    }

    /// Duration derived from frame count and effective sample rate.
    pub fn duration(&self) -> Duration {
        self.inner.duration
    }

    pub fn channel(&self, index: usize) -> Option<&[f32]> {
        let start = index.checked_mul(self.frame_count())?;
        let end = start.checked_add(self.frame_count())?;
        self.inner.samples.get(start..end)
    }

    pub fn channels(&self) -> impl ExactSizeIterator<Item = &[f32]> {
        self.inner.samples.chunks_exact(self.frame_count())
    }
}
