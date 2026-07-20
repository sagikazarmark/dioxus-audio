//! Microphone recording state and hooks.

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;
use std::time::Duration;

use dioxus::prelude::*;

use crate::AudioError;
use crate::analysis::AudioAnalyser;
use crate::devices::MicrophonePermission;
use crate::{AudioInputId, RecordedAudio, RecordingChunk, RecordingId};

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
mod web;

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RecorderStatus {
    Idle,
    RequestingPermission,
    Recording,
    Paused,
    Stopping,
    Failed(AudioError),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionDisposition {
    Save,
    Discard,
}

/// The terminal outcome of an accepted Recording.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RecordingOutcome {
    Completed(RecordingId),
    Discarded(RecordingId),
    Failed {
        recording_id: RecordingId,
        error: AudioError,
    },
}

impl RecordingOutcome {
    pub fn recording_id(&self) -> RecordingId {
        match self {
            Self::Completed(recording_id) | Self::Discarded(recording_id) => *recording_id,
            Self::Failed { recording_id, .. } => *recording_id,
        }
    }
}

/// The terminal failure of incremental Recording Chunk delivery.
///
/// Capture and final Recorded Audio assembly continue independently after this
/// failure, but this Recording will deliver no later chunks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordingChunkDeliveryFailure {
    recording_id: RecordingId,
    failed_sequence: u64,
    error: AudioError,
}

impl RecordingChunkDeliveryFailure {
    pub fn recording_id(&self) -> RecordingId {
        self.recording_id
    }

    pub fn failed_sequence(&self) -> u64 {
        self.failed_sequence
    }

    pub fn error(&self) -> &AudioError {
        &self.error
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecorderCommandError {
    message: &'static str,
}

impl RecorderCommandError {
    pub fn message(&self) -> &'static str {
        self.message
    }
}

impl fmt::Display for RecorderCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message)
    }
}

impl std::error::Error for RecorderCommandError {}

#[derive(Debug)]
pub struct RecorderLifecycle {
    status: RecorderStatus,
    generation: u64,
    active_recording: Option<RecordingId>,
    next_chunk_sequence: u64,
    completion: CompletionDisposition,
    finalizing: bool,
    has_unconsumed_recording: bool,
}

impl Default for RecorderLifecycle {
    fn default() -> Self {
        Self {
            status: RecorderStatus::Idle,
            generation: 0,
            active_recording: None,
            next_chunk_sequence: 0,
            completion: CompletionDisposition::Save,
            finalizing: false,
            has_unconsumed_recording: false,
        }
    }
}

impl RecorderLifecycle {
    pub fn status(&self) -> &RecorderStatus {
        &self.status
    }

    pub fn start(&mut self) -> Result<RecordingId, RecorderCommandError> {
        if self.has_unconsumed_recording {
            return Err(command_error(
                "clear the completed recording before starting another",
            ));
        }
        if !matches!(
            self.status,
            RecorderStatus::Idle | RecorderStatus::Failed(_)
        ) {
            return Err(command_error("recording is already active"));
        }

        self.generation = self.generation.wrapping_add(1);
        let recording_id = RecordingId::from_generation(self.generation);
        self.active_recording = Some(recording_id);
        self.next_chunk_sequence = 0;
        self.completion = CompletionDisposition::Save;
        self.finalizing = false;
        self.status = RecorderStatus::RequestingPermission;
        Ok(recording_id)
    }

    pub fn started(&mut self, recording_id: RecordingId) -> bool {
        if self.active_recording == Some(recording_id)
            && matches!(self.status, RecorderStatus::RequestingPermission)
        {
            self.status = RecorderStatus::Recording;
            true
        } else {
            false
        }
    }

    pub fn stop(&mut self) -> Result<(), RecorderCommandError> {
        if !matches!(
            self.status,
            RecorderStatus::Recording | RecorderStatus::Paused
        ) {
            return Err(command_error(
                "recording cannot be stopped in its current state",
            ));
        }

        self.completion = CompletionDisposition::Save;
        self.status = RecorderStatus::Stopping;
        Ok(())
    }

    pub fn pause(&mut self) -> Result<(), RecorderCommandError> {
        if !matches!(self.status, RecorderStatus::Recording) {
            return Err(command_error("recording can only be paused while active"));
        }
        self.status = RecorderStatus::Paused;
        Ok(())
    }

    pub fn resume(&mut self) -> Result<(), RecorderCommandError> {
        if !matches!(self.status, RecorderStatus::Paused) {
            return Err(command_error("recording can only be resumed while paused"));
        }
        self.status = RecorderStatus::Recording;
        Ok(())
    }

    pub fn request_chunk_boundary(&self) -> Result<(), RecorderCommandError> {
        if !matches!(
            self.status,
            RecorderStatus::Recording | RecorderStatus::Paused
        ) {
            return Err(command_error(
                "a chunk boundary can only be requested while recording or paused",
            ));
        }
        Ok(())
    }

    /// Reserve the next contiguous sequence for a non-empty Recording Chunk.
    pub fn next_chunk_sequence(&mut self, recording_id: RecordingId) -> Option<u64> {
        let accepts_chunk = self.active_recording == Some(recording_id)
            && !self.finalizing
            && (matches!(
                self.status,
                RecorderStatus::Recording | RecorderStatus::Paused
            ) || (matches!(self.status, RecorderStatus::Stopping)
                && self.completion == CompletionDisposition::Save));
        if !accepts_chunk {
            return None;
        }

        let sequence = self.next_chunk_sequence;
        self.next_chunk_sequence = self.next_chunk_sequence.checked_add(1)?;
        Some(sequence)
    }

    pub fn cancel(&mut self) -> Result<(), RecorderCommandError> {
        match self.status {
            RecorderStatus::RequestingPermission => {
                self.active_recording = None;
                self.status = RecorderStatus::Idle;
            }
            RecorderStatus::Recording | RecorderStatus::Paused => {
                self.completion = CompletionDisposition::Discard;
                self.status = RecorderStatus::Stopping;
            }
            _ => {
                return Err(command_error(
                    "recording cannot be cancelled in its current state",
                ));
            }
        }

        Ok(())
    }

    pub fn begin_finalize(&mut self, recording_id: RecordingId) -> Option<CompletionDisposition> {
        if self.active_recording != Some(recording_id) || self.finalizing {
            return None;
        }

        match self.status {
            RecorderStatus::Stopping => {}
            RecorderStatus::Recording | RecorderStatus::Paused => {
                self.completion = CompletionDisposition::Save;
                self.status = RecorderStatus::Stopping;
            }
            _ => return None,
        }

        self.finalizing = true;
        Some(self.completion)
    }

    pub fn complete_finalize(&mut self, recording_id: RecordingId) -> bool {
        if self.active_recording != Some(recording_id) || !self.finalizing {
            return false;
        }

        self.active_recording = None;
        self.finalizing = false;
        self.has_unconsumed_recording = self.completion == CompletionDisposition::Save;
        self.status = RecorderStatus::Idle;
        true
    }

    pub fn clear_completed(&mut self) {
        self.has_unconsumed_recording = false;
    }

    pub fn failed(&mut self, recording_id: RecordingId, error: AudioError) -> bool {
        if self.active_recording != Some(recording_id) {
            return false;
        }

        self.active_recording = None;
        self.finalizing = false;
        self.status = RecorderStatus::Failed(error);
        true
    }

    pub fn configuration_failed(&mut self, error: AudioError) -> bool {
        if self.active_recording.is_some()
            || !matches!(
                self.status,
                RecorderStatus::Idle | RecorderStatus::Failed(_)
            )
        {
            return false;
        }
        self.status = RecorderStatus::Failed(error);
        true
    }

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    fn abandon(&mut self) {
        self.active_recording = None;
        self.finalizing = false;
    }
}

fn command_error(message: &'static str) -> RecorderCommandError {
    RecorderCommandError { message }
}

/// A best-effort or required value requested for a Recording Source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecordingConstraint<T> {
    /// Prefer this value while allowing the browser to select another.
    Ideal(T),
    /// Require this value or reject source acquisition.
    Exact(T),
}

/// Portable startup constraints applied when the Recorder acquires a source.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecordingConstraints {
    pub channel_count: Option<RecordingConstraint<u32>>,
    pub sample_rate: Option<RecordingConstraint<u32>>,
    pub echo_cancellation: Option<RecordingConstraint<bool>>,
    pub noise_suppression: Option<RecordingConstraint<bool>>,
    pub latency: Option<RecordingConstraint<Duration>>,
}

/// Constraint fields that the browser reports recognizing.
///
/// Recognition does not prove that any particular value is available.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecorderConstraintCapabilities {
    pub channel_count: bool,
    pub sample_rate: bool,
    pub echo_cancellation: bool,
    pub noise_suppression: bool,
    pub latency: bool,
}

/// Effective settings reported by an acquired Recording Source.
///
/// Every field is optional because browser reporting varies.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecordingSourceSettings {
    pub channel_count: Option<u32>,
    pub sample_rate: Option<u32>,
    pub echo_cancellation: Option<bool>,
    pub noise_suppression: Option<bool>,
    pub latency: Option<Duration>,
}

type RecordingChunkHandler = Rc<RefCell<Box<dyn FnMut(RecordingChunk)>>>;

/// Opt-in cadence and callback for ordered Recording Chunk delivery.
#[derive(Clone)]
pub struct RecordingChunkDelivery {
    /// Approximate interval at which the browser should create chunk boundaries.
    ///
    /// Boundaries may be empty, late, or dependent on earlier fragments.
    pub cadence: Duration,
    on_chunk: RecordingChunkHandler,
}

impl fmt::Debug for RecordingChunkDelivery {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RecordingChunkDelivery")
            .field("cadence", &self.cadence)
            .finish_non_exhaustive()
    }
}

impl PartialEq for RecordingChunkDelivery {
    fn eq(&self, other: &Self) -> bool {
        self.cadence == other.cadence && Rc::ptr_eq(&self.on_chunk, &other.on_chunk)
    }
}

impl RecordingChunkDelivery {
    /// Configure ordered push delivery at an approximate cadence.
    pub fn new(cadence: Duration, on_chunk: impl FnMut(RecordingChunk) + 'static) -> Self {
        Self {
            cadence,
            on_chunk: Rc::new(RefCell::new(Box::new(on_chunk))),
        }
    }

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    fn call(&self, chunk: RecordingChunk) {
        (self.on_chunk.borrow_mut())(chunk);
    }

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    fn time_slice_millis(&self) -> i32 {
        i32::try_from(self.cadence.as_millis())
            .expect("validated Recording Chunk cadence fits in an i32")
    }
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct RecorderOptions {
    pub fft_size: u32,
    pub smoothing: f64,
    pub peak_interval: Duration,
    pub constraints: RecordingConstraints,
    pub mime_types: Vec<String>,
    pub audio_bits_per_second: Option<u32>,
    pub chunk_delivery: Option<RecordingChunkDelivery>,
}

impl Default for RecorderOptions {
    fn default() -> Self {
        Self {
            fft_size: 256,
            smoothing: 0.8,
            peak_interval: Duration::from_millis(100),
            constraints: RecordingConstraints::default(),
            mime_types: vec![
                "audio/webm;codecs=opus".to_string(),
                "audio/webm".to_string(),
                "audio/mp4".to_string(),
            ],
            audio_bits_per_second: None,
            chunk_delivery: None,
        }
    }
}

impl RecorderOptions {
    pub fn validate(&self) -> Result<(), AudioError> {
        if !(32..=32768).contains(&self.fft_size) || !self.fft_size.is_power_of_two() {
            return Err(AudioError::new(
                crate::AudioErrorKind::InvalidConfiguration,
                "fft_size must be a power of two between 32 and 32768",
            ));
        }
        if !(0.0..=1.0).contains(&self.smoothing) {
            return Err(AudioError::new(
                crate::AudioErrorKind::InvalidConfiguration,
                "smoothing must be between 0 and 1",
            ));
        }
        if self.peak_interval.is_zero() {
            return Err(AudioError::new(
                crate::AudioErrorKind::InvalidConfiguration,
                "peak_interval must be greater than zero",
            ));
        }
        if let Some(delivery) = &self.chunk_delivery
            && (delivery.cadence.as_millis() == 0
                || delivery.cadence.as_millis() > i32::MAX as u128)
        {
            return Err(AudioError::new(
                crate::AudioErrorKind::InvalidConfiguration,
                "Recording Chunk cadence must be between 1 millisecond and 2147483647 milliseconds",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MicrophoneStatus {
    pub permission: MicrophonePermission,
    pub recorder: RecorderStatus,
    pub input_device: Option<AudioInputId>,
    pub muted: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub struct AudioRecorder {
    status: ReadSignal<RecorderStatus>,
    completed: ReadSignal<Option<RecordedAudio>>,
    analyser: ReadSignal<Option<AudioAnalyser>>,
    elapsed: ReadSignal<Duration>,
    microphone: ReadSignal<MicrophoneStatus>,
    requested_constraints: ReadSignal<Option<RecordingConstraints>>,
    constraint_capabilities: ReadSignal<Option<RecorderConstraintCapabilities>>,
    settings: ReadSignal<Option<RecordingSourceSettings>>,
    media_type: ReadSignal<Option<String>>,
    outcome: ReadSignal<Option<RecordingOutcome>>,
    chunk_delivery_failure: ReadSignal<Option<RecordingChunkDeliveryFailure>>,
    start: Callback<(), Result<(), RecorderCommandError>>,
    pause: Callback<(), Result<(), RecorderCommandError>>,
    resume: Callback<(), Result<(), RecorderCommandError>>,
    request_chunk_boundary: Callback<(), Result<(), RecorderCommandError>>,
    stop: Callback<(), Result<(), RecorderCommandError>>,
    cancel: Callback<(), Result<(), RecorderCommandError>>,
    take_completed: Callback<(), Option<RecordedAudio>>,
    clear_completed: Callback,
}

impl AudioRecorder {
    pub fn status(self) -> ReadSignal<RecorderStatus> {
        self.status
    }

    pub fn completed(self) -> ReadSignal<Option<RecordedAudio>> {
        self.completed
    }

    pub fn analyser(self) -> ReadSignal<Option<AudioAnalyser>> {
        self.analyser
    }

    pub fn elapsed(self) -> ReadSignal<Duration> {
        self.elapsed
    }

    pub fn microphone(self) -> ReadSignal<MicrophoneStatus> {
        self.microphone
    }

    /// Constraints snapshotted by the most recently accepted start request.
    pub fn requested_constraints(self) -> ReadSignal<Option<RecordingConstraints>> {
        self.requested_constraints
    }

    /// Constraint fields that the browser reports recognizing.
    ///
    /// Recognition does not imply that a particular value can be acquired.
    pub fn constraint_capabilities(self) -> ReadSignal<Option<RecorderConstraintCapabilities>> {
        self.constraint_capabilities
    }

    /// Effective settings reported by the acquired Recording Source.
    pub fn settings(self) -> ReadSignal<Option<RecordingSourceSettings>> {
        self.settings
    }

    /// Encoder media type selected for the current or most recent Recording.
    pub fn media_type(self) -> ReadSignal<Option<String>> {
        self.media_type
    }

    /// Terminal outcome of the most recently accepted Recording, if any.
    pub fn outcome(self) -> ReadSignal<Option<RecordingOutcome>> {
        self.outcome
    }

    /// Terminal incremental delivery failure for the current or most recent Recording.
    pub fn chunk_delivery_failure(self) -> ReadSignal<Option<RecordingChunkDeliveryFailure>> {
        self.chunk_delivery_failure
    }

    pub fn start(self) -> Result<(), RecorderCommandError> {
        self.start.call(())
    }

    pub fn pause(self) -> Result<(), RecorderCommandError> {
        self.pause.call(())
    }

    pub fn resume(self) -> Result<(), RecorderCommandError> {
        self.resume.call(())
    }

    /// Ask the browser to create a best-effort Recording Chunk boundary.
    ///
    /// The request is available only to an opted-in Recording while active or
    /// paused. It does not promise exact timing, non-empty output, or an
    /// independently playable chunk.
    pub fn request_chunk_boundary(self) -> Result<(), RecorderCommandError> {
        self.request_chunk_boundary.call(())
    }

    pub fn stop(self) -> Result<(), RecorderCommandError> {
        self.stop.call(())
    }

    pub fn cancel(self) -> Result<(), RecorderCommandError> {
        self.cancel.call(())
    }

    pub fn clear_completed(self) {
        self.clear_completed.call(());
    }

    /// Move the completed recording out without cloning its audio buffer.
    pub fn take_completed(self) -> Option<RecordedAudio> {
        self.take_completed.call(())
    }
}

/// Probe whether the browser recognizes a Recorder media type.
///
/// A positive result does not guarantee that source acquisition, Recorder
/// construction, or a complete Recording will succeed.
pub fn is_recorder_mime_type_supported(mime_type: &str) -> bool {
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        web_sys::MediaRecorder::is_type_supported(mime_type)
    }

    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    {
        let _ = mime_type;
        false
    }
}

/// Create a recorder controller. The selected input is snapshotted by `start`.
pub fn use_audio_recorder(
    options: RecorderOptions,
    selected_input: ReadSignal<Option<AudioInputId>>,
) -> AudioRecorder {
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        web::use_web_audio_recorder(options, selected_input)
    }

    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    {
        let _ = (options, selected_input);
        use_unsupported_audio_recorder()
    }
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
fn use_unsupported_audio_recorder() -> AudioRecorder {
    let error = AudioError::unsupported();
    let mut status = use_signal(|| RecorderStatus::Idle);
    let completed = use_signal(|| None::<RecordedAudio>);
    let analyser = use_signal(|| None::<AudioAnalyser>);
    let elapsed = use_signal(|| Duration::ZERO);
    let requested_constraints = use_signal(|| None::<RecordingConstraints>);
    let constraint_capabilities = use_signal(|| None::<RecorderConstraintCapabilities>);
    let settings = use_signal(|| None::<RecordingSourceSettings>);
    let media_type = use_signal(|| None::<String>);
    let outcome = use_signal(|| None::<RecordingOutcome>);
    let chunk_delivery_failure = use_signal(|| None::<RecordingChunkDeliveryFailure>);
    let mut microphone = use_signal(|| MicrophoneStatus {
        permission: MicrophonePermission::Unknown,
        recorder: RecorderStatus::Idle,
        input_device: None,
        muted: false,
    });
    use_effect(move || {
        let status_error = RecorderStatus::Failed(error.clone());
        status.set(status_error.clone());
        microphone.set(MicrophoneStatus {
            permission: MicrophonePermission::Unsupported,
            recorder: status_error,
            input_device: None,
            muted: false,
        });
    });
    let unsupported: Callback<(), Result<(), RecorderCommandError>> = use_callback(|()| {
        Err(command_error(
            "audio recording is unsupported on this platform",
        ))
    });
    let mut completed_to_clear = completed;
    let clear_completed = use_callback(move |()| completed_to_clear.set(None));
    let mut completed_to_take = completed;
    let take_completed = use_callback(move |()| completed_to_take.write().take());

    AudioRecorder {
        status: status.into(),
        completed: completed.into(),
        analyser: analyser.into(),
        elapsed: elapsed.into(),
        microphone: microphone.into(),
        requested_constraints: requested_constraints.into(),
        constraint_capabilities: constraint_capabilities.into(),
        settings: settings.into(),
        media_type: media_type.into(),
        outcome: outcome.into(),
        chunk_delivery_failure: chunk_delivery_failure.into(),
        start: unsupported,
        pause: unsupported,
        resume: unsupported,
        request_chunk_boundary: unsupported,
        stop: unsupported,
        cancel: unsupported,
        take_completed,
        clear_completed,
    }
}
