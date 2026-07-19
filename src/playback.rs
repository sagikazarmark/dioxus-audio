//! Audio playback state and hooks.

use std::fmt;
use std::time::Duration;

use dioxus::prelude::*;

use crate::AudioData;
use crate::AudioError;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
mod web;

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlaybackStatus {
    Empty,
    Loading,
    Ready,
    Playing,
    Paused,
    Ended,
    Failed(AudioError),
}

/// The lifecycle of the current Playback Source, independent of transport.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlaybackSourceLifecycle {
    Empty,
    Loading,
    Playable,
    Failed(AudioError),
}

/// The requested or confirmed transport state of Playback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlaybackTransport {
    Idle,
    PlayPending,
    Playing,
    Paused,
    Ended,
}

/// How ready the current source is to advance Playback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlaybackReadiness {
    Unavailable,
    LoadingMetadata,
    Metadata,
    Playable,
    Waiting,
}

/// A play request failure that leaves the current source usable for retry.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlaybackPlayFailure {
    InteractionRequired(AudioError),
    Unknown(AudioError),
}

impl PlaybackPlayFailure {
    pub fn error(&self) -> &AudioError {
        match self {
            Self::InteractionRequired(error) | Self::Unknown(error) => error,
        }
    }
}

/// One coherent observation of Playback's independent state facets.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct PlaybackSnapshot {
    pub source: PlaybackSourceLifecycle,
    pub transport: PlaybackTransport,
    pub readiness: PlaybackReadiness,
    pub play_failure: Option<PlaybackPlayFailure>,
    /// Whole-source repeat preference, retained across source replacement and unload.
    pub repeat: bool,
}

impl Default for PlaybackSnapshot {
    fn default() -> Self {
        Self {
            source: PlaybackSourceLifecycle::Empty,
            transport: PlaybackTransport::Idle,
            readiness: PlaybackReadiness::Unavailable,
            play_failure: None,
            repeat: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaybackCommandError(&'static str);

impl fmt::Display for PlaybackCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for PlaybackCommandError {}

#[derive(Debug)]
pub struct PlaybackLifecycle {
    status: PlaybackStatus,
    snapshot: PlaybackSnapshot,
}

impl Default for PlaybackLifecycle {
    fn default() -> Self {
        Self {
            status: PlaybackStatus::Empty,
            snapshot: PlaybackSnapshot::default(),
        }
    }
}

impl PlaybackLifecycle {
    pub fn status(&self) -> &PlaybackStatus {
        &self.status
    }

    pub fn snapshot(&self) -> &PlaybackSnapshot {
        &self.snapshot
    }

    pub fn source(&self) -> &PlaybackSourceLifecycle {
        &self.snapshot.source
    }

    pub fn transport(&self) -> PlaybackTransport {
        self.snapshot.transport
    }

    pub fn readiness(&self) -> PlaybackReadiness {
        self.snapshot.readiness
    }

    pub fn play_failure(&self) -> Option<&PlaybackPlayFailure> {
        self.snapshot.play_failure.as_ref()
    }

    pub fn repeat(&self) -> bool {
        self.snapshot.repeat
    }

    pub fn set_repeat(&mut self, repeat: bool) {
        self.snapshot.repeat = repeat;
    }

    pub fn toggle_repeat(&mut self) {
        self.snapshot.repeat = !self.snapshot.repeat;
    }

    pub fn loading(&mut self) {
        let repeat = self.snapshot.repeat;
        self.status = PlaybackStatus::Loading;
        self.snapshot = PlaybackSnapshot {
            source: PlaybackSourceLifecycle::Loading,
            transport: PlaybackTransport::Idle,
            readiness: PlaybackReadiness::LoadingMetadata,
            play_failure: None,
            repeat,
        };
    }

    pub fn loaded(&mut self) {
        self.status = PlaybackStatus::Ready;
        self.snapshot.source = PlaybackSourceLifecycle::Playable;
        self.snapshot.readiness = PlaybackReadiness::Metadata;
    }

    pub fn request_play(&mut self) -> Result<(), PlaybackCommandError> {
        if matches!(self.snapshot.source, PlaybackSourceLifecycle::Playable)
            && matches!(
                self.snapshot.transport,
                PlaybackTransport::Idle | PlaybackTransport::Paused | PlaybackTransport::Ended
            )
        {
            if self.snapshot.play_failure.take().is_some() {
                self.status = PlaybackStatus::Ready;
            }
            self.snapshot.transport = PlaybackTransport::PlayPending;
            Ok(())
        } else {
            Err(PlaybackCommandError("audio is not ready to play"))
        }
    }

    pub fn playing(&mut self) {
        if !matches!(self.snapshot.source, PlaybackSourceLifecycle::Playable)
            || !matches!(
                self.snapshot.transport,
                PlaybackTransport::PlayPending | PlaybackTransport::Playing
            )
        {
            return;
        }
        self.status = PlaybackStatus::Playing;
        self.snapshot.transport = PlaybackTransport::Playing;
        self.snapshot.readiness = PlaybackReadiness::Playable;
        self.snapshot.play_failure = None;
    }

    pub fn waiting(&mut self) {
        if matches!(self.snapshot.source, PlaybackSourceLifecycle::Playable)
            && matches!(
                self.snapshot.transport,
                PlaybackTransport::PlayPending | PlaybackTransport::Playing
            )
        {
            self.snapshot.readiness = PlaybackReadiness::Waiting;
        }
    }

    pub fn playable(&mut self) {
        if matches!(self.snapshot.source, PlaybackSourceLifecycle::Playable) {
            self.snapshot.readiness = PlaybackReadiness::Playable;
        }
    }

    pub fn paused(&mut self) {
        if matches!(
            self.snapshot.transport,
            PlaybackTransport::PlayPending | PlaybackTransport::Playing
        ) {
            self.snapshot.transport = PlaybackTransport::Paused;
        }
        if matches!(self.status, PlaybackStatus::Playing) {
            self.status = PlaybackStatus::Paused;
        }
    }

    /// Return a loaded source to its ready, idle state.
    pub fn stop(&mut self) -> Result<(), PlaybackCommandError> {
        if !matches!(self.snapshot.source, PlaybackSourceLifecycle::Playable) {
            return Err(PlaybackCommandError("audio is not loaded"));
        }

        self.status = PlaybackStatus::Ready;
        self.snapshot.transport = PlaybackTransport::Idle;
        self.snapshot.play_failure = None;
        Ok(())
    }

    pub fn ended(&mut self) {
        if self.snapshot.transport != PlaybackTransport::Playing {
            return;
        }
        self.status = PlaybackStatus::Ended;
        self.snapshot.transport = PlaybackTransport::Ended;
        self.snapshot.readiness = PlaybackReadiness::Playable;
    }

    pub fn play_rejected(&mut self, failure: PlaybackPlayFailure) {
        if !matches!(self.snapshot.source, PlaybackSourceLifecycle::Playable)
            || !matches!(self.snapshot.transport, PlaybackTransport::PlayPending)
        {
            return;
        }

        self.status = PlaybackStatus::Failed(failure.error().clone());
        self.snapshot.transport = PlaybackTransport::Paused;
        if self.snapshot.readiness == PlaybackReadiness::Waiting {
            self.snapshot.readiness = PlaybackReadiness::Metadata;
        }
        self.snapshot.play_failure = Some(failure);
    }

    pub fn seeked(&mut self, position: f64, duration: f64) {
        if matches!(
            self.status,
            PlaybackStatus::Empty | PlaybackStatus::Loading | PlaybackStatus::Failed(_)
        ) {
            return;
        }
        if duration.is_finite() && duration > 0.0 && position >= duration {
            self.status = PlaybackStatus::Ended;
            self.snapshot.transport = PlaybackTransport::Ended;
        } else if matches!(self.status, PlaybackStatus::Ended) {
            self.status = PlaybackStatus::Paused;
            self.snapshot.transport = PlaybackTransport::Paused;
        }
    }

    pub fn failed(&mut self, error: AudioError) {
        let repeat = self.snapshot.repeat;
        self.status = PlaybackStatus::Failed(error.clone());
        self.snapshot = PlaybackSnapshot {
            source: PlaybackSourceLifecycle::Failed(error),
            transport: PlaybackTransport::Idle,
            readiness: PlaybackReadiness::Unavailable,
            play_failure: None,
            repeat,
        };
    }

    pub fn unload(&mut self) {
        let repeat = self.snapshot.repeat;
        self.status = PlaybackStatus::Empty;
        self.snapshot = PlaybackSnapshot {
            repeat,
            ..Default::default()
        };
    }
}

/// Clamp a requested playback position to a usable finite timeline.
pub fn clamp_seek(position: f64, duration: f64) -> f64 {
    if !position.is_finite() || !duration.is_finite() || duration <= 0.0 {
        return 0.0;
    }
    position.clamp(0.0, duration)
}

#[derive(Clone, Copy, PartialEq)]
pub struct AudioPlayerController {
    status: ReadSignal<PlaybackStatus>,
    snapshot: ReadSignal<PlaybackSnapshot>,
    position: ReadSignal<Duration>,
    duration: ReadSignal<Duration>,
    rate: ReadSignal<f64>,
    play: Callback<(), Result<(), PlaybackCommandError>>,
    pause: Callback<(), Result<(), PlaybackCommandError>>,
    stop: Callback<(), Result<(), PlaybackCommandError>>,
    seek: Callback<Duration>,
    skip: Callback<f64>,
    set_rate: Callback<f64, Result<(), PlaybackCommandError>>,
    set_repeat: Callback<bool>,
}

impl AudioPlayerController {
    pub fn status(self) -> ReadSignal<PlaybackStatus> {
        self.status
    }

    pub fn snapshot(self) -> ReadSignal<PlaybackSnapshot> {
        self.snapshot
    }

    pub fn position(self) -> ReadSignal<Duration> {
        self.position
    }

    pub fn duration(self) -> ReadSignal<Duration> {
        self.duration
    }

    pub fn rate(self) -> ReadSignal<f64> {
        self.rate
    }

    pub fn repeat(self) -> bool {
        self.snapshot.read().repeat
    }

    pub fn play(self) -> Result<(), PlaybackCommandError> {
        self.play.call(())
    }

    pub fn pause(self) -> Result<(), PlaybackCommandError> {
        self.pause.call(())
    }

    /// Stop Playback atomically and reset its observable position.
    pub fn stop(self) -> Result<(), PlaybackCommandError> {
        self.stop.call(())
    }

    pub fn seek(self, position: Duration) {
        self.seek.call(position);
    }

    pub fn skip(self, seconds: f64) {
        self.skip.call(seconds);
    }

    pub fn set_rate(self, rate: f64) -> Result<(), PlaybackCommandError> {
        self.set_rate.call(rate)
    }

    /// Set the whole-source repeat preference.
    pub fn set_repeat(self, repeat: bool) {
        self.set_repeat.call(repeat);
    }

    /// Toggle the whole-source repeat preference.
    pub fn toggle_repeat(self) {
        self.set_repeat(!self.repeat());
    }
}

pub fn use_audio_player(
    source: ReadSignal<Option<AudioData>>,
    initial_duration: Duration,
) -> AudioPlayerController {
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        web::use_web_audio_player(source, initial_duration)
    }

    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    {
        let _ = source;
        use_unsupported_audio_player(initial_duration)
    }
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
fn use_unsupported_audio_player(initial_duration: Duration) -> AudioPlayerController {
    let mut status = use_signal(|| PlaybackStatus::Empty);
    let mut snapshot = use_signal(PlaybackSnapshot::default);
    let position = use_signal(|| Duration::ZERO);
    let mut duration = use_signal(|| initial_duration);
    let rate = use_signal(|| 1.0);
    let initial_duration = use_memo(use_reactive!(|(initial_duration,)| initial_duration));
    use_effect(move || {
        duration.set(initial_duration());
        let error = AudioError::unsupported();
        let repeat = snapshot.peek().repeat;
        status.set(PlaybackStatus::Failed(error.clone()));
        snapshot.set(PlaybackSnapshot {
            source: PlaybackSourceLifecycle::Failed(error),
            transport: PlaybackTransport::Idle,
            readiness: PlaybackReadiness::Unavailable,
            play_failure: None,
            repeat,
        });
    });
    let unsupported: Callback<(), Result<(), PlaybackCommandError>> =
        use_callback(|()| Err(PlaybackCommandError("audio playback is unsupported")));
    let seek = use_callback(|_: Duration| {});
    let skip = use_callback(|_: f64| {});
    let set_rate: Callback<f64, Result<(), PlaybackCommandError>> =
        use_callback(|_: f64| Err(PlaybackCommandError("audio playback is unsupported")));
    let mut snapshot_for_repeat = snapshot;
    let set_repeat = use_callback(move |repeat: bool| {
        snapshot_for_repeat.write().repeat = repeat;
    });
    AudioPlayerController {
        status: status.into(),
        snapshot: snapshot.into(),
        position: position.into(),
        duration: duration.into(),
        rate: rate.into(),
        play: unsupported,
        pause: unsupported,
        stop: unsupported,
        seek,
        skip,
        set_rate,
        set_repeat,
    }
}
