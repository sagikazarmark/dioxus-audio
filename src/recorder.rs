//! Microphone recording state and hooks.

use std::fmt;
use std::time::Duration;

use dioxus::prelude::*;

use crate::AudioError;
use crate::analysis::AudioAnalyser;
use crate::devices::MicrophonePermission;
use crate::{AudioInputId, RecordedAudio};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecordingSessionId(u64);

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
    active_session: Option<RecordingSessionId>,
    completion: CompletionDisposition,
    finalizing: bool,
    has_unconsumed_recording: bool,
}

impl Default for RecorderLifecycle {
    fn default() -> Self {
        Self {
            status: RecorderStatus::Idle,
            generation: 0,
            active_session: None,
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

    pub fn start(&mut self) -> Result<RecordingSessionId, RecorderCommandError> {
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
        let session = RecordingSessionId(self.generation);
        self.active_session = Some(session);
        self.completion = CompletionDisposition::Save;
        self.finalizing = false;
        self.status = RecorderStatus::RequestingPermission;
        Ok(session)
    }

    pub fn started(&mut self, session: RecordingSessionId) -> bool {
        if self.active_session == Some(session)
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

    pub fn cancel(&mut self) -> Result<(), RecorderCommandError> {
        match self.status {
            RecorderStatus::RequestingPermission => {
                self.active_session = None;
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

    pub fn begin_finalize(&mut self, session: RecordingSessionId) -> Option<CompletionDisposition> {
        if self.active_session != Some(session) || self.finalizing {
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

    pub fn complete_finalize(&mut self, session: RecordingSessionId) -> bool {
        if self.active_session != Some(session) || !self.finalizing {
            return false;
        }

        self.active_session = None;
        self.finalizing = false;
        self.has_unconsumed_recording = self.completion == CompletionDisposition::Save;
        self.status = RecorderStatus::Idle;
        true
    }

    pub fn clear_completed(&mut self) {
        self.has_unconsumed_recording = false;
    }

    pub fn failed(&mut self, session: RecordingSessionId, error: AudioError) -> bool {
        if self.active_session != Some(session) {
            return false;
        }

        self.active_session = None;
        self.finalizing = false;
        self.status = RecorderStatus::Failed(error);
        true
    }

    pub fn configuration_failed(&mut self, error: AudioError) -> bool {
        if self.active_session.is_some()
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
}

fn command_error(message: &'static str) -> RecorderCommandError {
    RecorderCommandError { message }
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct RecorderOptions {
    pub fft_size: u32,
    pub smoothing: f64,
    pub peak_interval: Duration,
    pub mime_types: Vec<String>,
    pub audio_bits_per_second: Option<u32>,
}

impl Default for RecorderOptions {
    fn default() -> Self {
        Self {
            fft_size: 256,
            smoothing: 0.8,
            peak_interval: Duration::from_millis(100),
            mime_types: vec![
                "audio/webm;codecs=opus".to_string(),
                "audio/webm".to_string(),
                "audio/mp4".to_string(),
            ],
            audio_bits_per_second: None,
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
    start: Callback<(), Result<(), RecorderCommandError>>,
    pause: Callback<(), Result<(), RecorderCommandError>>,
    resume: Callback<(), Result<(), RecorderCommandError>>,
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

    pub fn start(self) -> Result<(), RecorderCommandError> {
        self.start.call(())
    }

    pub fn pause(self) -> Result<(), RecorderCommandError> {
        self.pause.call(())
    }

    pub fn resume(self) -> Result<(), RecorderCommandError> {
        self.resume.call(())
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
        start: unsupported,
        pause: unsupported,
        resume: unsupported,
        stop: unsupported,
        cancel: unsupported,
        take_completed,
        clear_completed,
    }
}
