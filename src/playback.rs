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
}

impl Default for PlaybackLifecycle {
    fn default() -> Self {
        Self {
            status: PlaybackStatus::Empty,
        }
    }
}

impl PlaybackLifecycle {
    pub fn status(&self) -> &PlaybackStatus {
        &self.status
    }

    pub fn loading(&mut self) {
        self.status = PlaybackStatus::Loading;
    }

    pub fn loaded(&mut self) {
        self.status = PlaybackStatus::Ready;
    }

    pub fn request_play(&self) -> Result<(), PlaybackCommandError> {
        if matches!(
            self.status,
            PlaybackStatus::Ready | PlaybackStatus::Paused | PlaybackStatus::Ended
        ) {
            Ok(())
        } else {
            Err(PlaybackCommandError("audio is not ready to play"))
        }
    }

    pub fn playing(&mut self) {
        self.status = PlaybackStatus::Playing;
    }

    pub fn paused(&mut self) {
        if matches!(self.status, PlaybackStatus::Playing) {
            self.status = PlaybackStatus::Paused;
        }
    }

    pub fn ended(&mut self) {
        self.status = PlaybackStatus::Ended;
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
        } else if matches!(self.status, PlaybackStatus::Ended) {
            self.status = PlaybackStatus::Paused;
        }
    }

    pub fn failed(&mut self, error: AudioError) {
        self.status = PlaybackStatus::Failed(error);
    }

    pub fn unload(&mut self) {
        self.status = PlaybackStatus::Empty;
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
    position: ReadSignal<Duration>,
    duration: ReadSignal<Duration>,
    rate: ReadSignal<f64>,
    play: Callback<(), Result<(), PlaybackCommandError>>,
    pause: Callback<(), Result<(), PlaybackCommandError>>,
    seek: Callback<Duration>,
    skip: Callback<f64>,
    set_rate: Callback<f64, Result<(), PlaybackCommandError>>,
}

impl AudioPlayerController {
    pub fn status(self) -> ReadSignal<PlaybackStatus> {
        self.status
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

    pub fn play(self) -> Result<(), PlaybackCommandError> {
        self.play.call(())
    }

    pub fn pause(self) -> Result<(), PlaybackCommandError> {
        self.pause.call(())
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
    let position = use_signal(|| Duration::ZERO);
    let mut duration = use_signal(|| initial_duration);
    let rate = use_signal(|| 1.0);
    let initial_duration = use_memo(use_reactive!(|(initial_duration,)| initial_duration));
    use_effect(move || {
        duration.set(initial_duration());
        status.set(PlaybackStatus::Failed(AudioError::unsupported()));
    });
    let unsupported: Callback<(), Result<(), PlaybackCommandError>> =
        use_callback(|()| Err(PlaybackCommandError("audio playback is unsupported")));
    let seek = use_callback(|_: Duration| {});
    let skip = use_callback(|_: f64| {});
    let set_rate: Callback<f64, Result<(), PlaybackCommandError>> =
        use_callback(|_: f64| Err(PlaybackCommandError("audio playback is unsupported")));
    AudioPlayerController {
        status: status.into(),
        position: position.into(),
        duration: duration.into(),
        rate: rate.into(),
        play: unsupported,
        pause: unsupported,
        seek,
        skip,
        set_rate,
    }
}
