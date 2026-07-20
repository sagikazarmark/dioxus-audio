use std::time::Duration;

use dioxus::prelude::*;
use dioxus_icons::lucide::{Pause, Play, Repeat2, RotateCcw, RotateCw, Square, Volume2, VolumeX};

use super::AudioScrubber;
use crate::AudioErrorKind;
use crate::playback::{
    AudioPlayerController, PlaybackAudibilityCapability, PlaybackNetworkActivity,
    PlaybackPlayFailure, PlaybackReadiness, PlaybackSource, PlaybackSourceFailure,
    PlaybackSourceLifecycle, PlaybackStatus, PlaybackTimeRange, PlaybackTransport,
    use_audio_player,
};

/// Localizable messages emitted by [`PlaybackStatusAnnouncer`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaybackAnnouncementLabels {
    pub empty: String,
    pub dormant: String,
    pub loading: String,
    pub ready: String,
    pub starting: String,
    pub playing: String,
    pub paused: String,
    pub ended: String,
    pub waiting: String,
    pub stalled: String,
    pub interaction_required: String,
    pub failed: String,
}

impl Default for PlaybackAnnouncementLabels {
    fn default() -> Self {
        Self {
            empty: "No audio loaded".to_string(),
            dormant: "Audio ready to load".to_string(),
            loading: "Audio loading".to_string(),
            ready: "Audio ready".to_string(),
            starting: "Playback starting".to_string(),
            playing: "Audio playing".to_string(),
            paused: "Audio paused".to_string(),
            ended: "Playback ended".to_string(),
            waiting: "Audio waiting for media".to_string(),
            stalled: "Audio loading stalled".to_string(),
            interaction_required: "Playback needs interaction".to_string(),
            failed: "Playback failed".to_string(),
        }
    }
}

/// An optional polite live region for coarse Playback lifecycle changes.
#[component]
pub fn PlaybackStatusAnnouncer(
    controller: AudioPlayerController,
    #[props(default)] labels: PlaybackAnnouncementLabels,
) -> Element {
    let status = controller.status()();
    let snapshot = controller.snapshot()();
    let message = if snapshot.source == PlaybackSourceLifecycle::Dormant {
        labels.dormant.as_str()
    } else if snapshot.source_failure.is_some() {
        labels.failed.as_str()
    } else if matches!(
        snapshot.play_failure,
        Some(PlaybackPlayFailure::InteractionRequired(_))
    ) {
        labels.interaction_required.as_str()
    } else if snapshot.network == PlaybackNetworkActivity::Stalled {
        labels.stalled.as_str()
    } else if snapshot.readiness == PlaybackReadiness::Waiting {
        labels.waiting.as_str()
    } else if snapshot.transport == PlaybackTransport::PlayPending {
        labels.starting.as_str()
    } else {
        match status {
            PlaybackStatus::Empty => labels.empty.as_str(),
            PlaybackStatus::Loading => labels.loading.as_str(),
            PlaybackStatus::Ready => labels.ready.as_str(),
            PlaybackStatus::Playing => labels.playing.as_str(),
            PlaybackStatus::Paused => labels.paused.as_str(),
            PlaybackStatus::Ended => labels.ended.as_str(),
            PlaybackStatus::Failed(_) => labels.failed.as_str(),
        }
    };

    rsx! {
        div {
            class: "dioxus-audio dioxus-audio__status-announcer",
            role: "status",
            aria_live: "polite",
            aria_atomic: "true",
            "{message}"
        }
    }
}

/// A native button that plays or pauses a Playback Controller.
///
/// Supplying `on_request_audio` also makes the button usable while Playback is
/// empty. The callback can load a source; Playback starts when that source is
/// ready.
#[component]
pub fn PlaybackPlayPauseButton(
    controller: AudioPlayerController,
    #[props(default = "Play".to_string())] play_label: String,
    #[props(default = "Pause".to_string())] pause_label: String,
    #[props(default)] on_request_audio: Option<EventHandler<()>>,
) -> Element {
    let snapshot = controller.snapshot()();
    let mut play_requested = use_signal(|| false);
    let requested = play_requested();
    let pending = snapshot.transport == PlaybackTransport::PlayPending;
    let playing = snapshot.transport == PlaybackTransport::Playing;
    let pausable = pending || playing;
    let empty = matches!(snapshot.source, PlaybackSourceLifecycle::Empty);
    let can_request_audio = on_request_audio.is_some();
    let disabled = match &snapshot.source {
        PlaybackSourceLifecycle::Empty => !can_request_audio,
        PlaybackSourceLifecycle::Dormant
        | PlaybackSourceLifecycle::Loading
        | PlaybackSourceLifecycle::Playable => false,
        PlaybackSourceLifecycle::Failed => true,
    };
    let busy = pending
        || (requested
            && matches!(
                snapshot.source,
                PlaybackSourceLifecycle::Empty | PlaybackSourceLifecycle::Loading
            ));
    let aria_label = if pausable { pause_label } else { play_label };

    use_effect(move || {
        if !play_requested() {
            return;
        }

        match controller.snapshot()().source {
            PlaybackSourceLifecycle::Dormant
            | PlaybackSourceLifecycle::Loading
            | PlaybackSourceLifecycle::Playable => {
                if controller.play().is_ok() {
                    play_requested.set(false);
                }
            }
            PlaybackSourceLifecycle::Failed => play_requested.set(false),
            PlaybackSourceLifecycle::Empty => {}
        }
    });

    rsx! {
        button {
            class: "dioxus-audio dioxus-audio__control dioxus-audio__control--primary",
            r#type: "button",
            aria_label,
            aria_busy: busy,
            aria_disabled: requested
                && matches!(snapshot.source, PlaybackSourceLifecycle::Loading),
            disabled,
            onclick: move |_| {
                if empty {
                    if let Some(on_request_audio) = on_request_audio {
                        play_requested.set(true);
                        on_request_audio.call(());
                    }
                } else if pausable {
                    let _ = controller.pause();
                } else if !busy {
                    let _ = controller.play();
                }
            },
            if pausable {
                Pause { size: 28 }
            } else {
                Play { size: 28 }
            }
        }
    }
}

/// A native button that stops Playback and resets its position.
#[component]
pub fn PlaybackStopButton(
    controller: AudioPlayerController,
    #[props(default = "Stop".to_string())] label: String,
) -> Element {
    let snapshot = controller.snapshot()();
    let position = controller.position()();
    let stopped = snapshot.transport == PlaybackTransport::Idle && position.is_zero();
    let disabled = !matches!(snapshot.source, PlaybackSourceLifecycle::Playable) || stopped;

    rsx! {
        button {
            class: "dioxus-audio dioxus-audio__control",
            r#type: "button",
            aria_label: label,
            disabled,
            onclick: move |_| { let _ = controller.stop(); },
            Square { size: 18 }
        }
    }
}

/// A native toggle button for whole-source repeat.
#[component]
pub fn PlaybackRepeatButton(
    controller: AudioPlayerController,
    #[props(default = "Repeat".to_string())] label: String,
) -> Element {
    let snapshot = controller.snapshot()();
    let repeat = snapshot.repeat;
    let unsupported = matches!(
        controller.status()(),
        PlaybackStatus::Failed(ref error) if error.kind() == AudioErrorKind::UnsupportedPlatform
    );

    rsx! {
        button {
            class: "dioxus-audio dioxus-audio__control",
            r#type: "button",
            aria_label: label,
            aria_pressed: if repeat { "true" } else { "false" },
            disabled: unsupported,
            onclick: move |_| controller.toggle_repeat(),
            Repeat2 { size: 20 }
        }
    }
}

/// A native toggle button that mutes Playback without pausing it.
#[component]
pub fn PlaybackMuteButton(
    controller: AudioPlayerController,
    #[props(default = "Mute".to_string())] label: String,
) -> Element {
    let snapshot = controller.snapshot()();
    let unsupported = matches!(
        controller.status()(),
        PlaybackStatus::Failed(ref error) if error.kind() == AudioErrorKind::UnsupportedPlatform
    );

    rsx! {
        button {
            class: "dioxus-audio dioxus-audio__control",
            r#type: "button",
            aria_label: label,
            aria_pressed: if snapshot.muted { "true" } else { "false" },
            disabled: unsupported,
            onclick: move |_| controller.toggle_muted(),
            if snapshot.muted {
                VolumeX { size: 20 }
            } else {
                Volume2 { size: 20 }
            }
        }
    }
}

/// A Controller-backed native slider for normalized Playback audibility.
///
/// The control is disabled when the Controller reports no level capability.
/// A best-effort media-element capability does not guarantee perceived loudness
/// on every browser.
#[component]
pub fn PlaybackAudibilitySlider(
    controller: AudioPlayerController,
    #[props(default = "Audibility level".to_string())] label: String,
    #[props(default)] value_text: Option<String>,
) -> Element {
    let snapshot = controller.snapshot()();
    let level = snapshot.audibility_level.value();
    let value_text = value_text.unwrap_or_else(|| format!("{} percent", (level * 100.0).round()));

    rsx! {
        input {
            class: "dioxus-audio dioxus-audio__audibility",
            r#type: "range",
            min: "0",
            max: "1",
            step: "0.01",
            value: "{level}",
            disabled: snapshot.audibility_capability == PlaybackAudibilityCapability::Unavailable,
            aria_label: label,
            aria_valuetext: value_text,
            oninput: move |event| {
                if let Ok(level) = event.value().parse::<f64>() {
                    let _ = controller.set_audibility_level(level);
                }
            },
        }
    }
}

/// A native button that seeks Playback by a signed number of seconds.
#[component]
pub fn PlaybackSkipButton(
    controller: AudioPlayerController,
    #[props(default = 15.0)] seconds: f64,
    #[props(default)] label: Option<String>,
) -> Element {
    let playable = matches!(
        controller.snapshot()().source,
        PlaybackSourceLifecycle::Playable
    );
    let valid = seconds.is_finite() && seconds != 0.0;
    let aria_label = label.unwrap_or_else(|| skip_label(seconds));

    rsx! {
        button {
            class: "dioxus-audio dioxus-audio__control",
            r#type: "button",
            aria_label,
            disabled: !playable || !valid,
            onclick: move |_| controller.skip(seconds),
            if seconds < 0.0 {
                RotateCcw { size: 20 }
            } else {
                RotateCw { size: 20 }
            }
        }
    }
}

/// A native button that cycles through configurable Playback rates.
#[component]
pub fn PlaybackRateButton(
    controller: AudioPlayerController,
    #[props(default = vec![1.0, 1.5, 2.0])] rates: Vec<f64>,
    #[props(default = "Playback speed".to_string())] label: String,
) -> Element {
    let rate = controller.rate()();
    let rates: Vec<_> = rates
        .into_iter()
        .filter(|rate| rate.is_finite() && *rate > 0.0)
        .collect();
    let next_rate = if rates.is_empty() {
        None
    } else if let Some(current) = rates
        .iter()
        .position(|candidate| (*candidate - rate).abs() < f64::EPSILON)
    {
        Some(rates[(current + 1) % rates.len()])
    } else {
        rates.first().copied()
    };
    let unsupported = matches!(
        controller.status()(),
        PlaybackStatus::Failed(ref error) if error.kind() == AudioErrorKind::UnsupportedPlatform
    );
    let rate_text = rate.to_string();
    let aria_label = format!("{label}: {rate_text}x");

    rsx! {
        button {
            class: "dioxus-audio dioxus-audio__rate",
            r#type: "button",
            aria_label,
            disabled: unsupported || next_rate.is_none(),
            onclick: move |_| {
                if let Some(next_rate) = next_rate {
                    let _ = controller.set_rate(next_rate);
                }
            },
            "{rate_text}x"
        }
    }
}

/// A Controller-backed native Playback position slider.
#[component]
pub fn PlaybackSeekSlider(
    controller: AudioPlayerController,
    #[props(default = "Seek audio".to_string())] label: String,
    #[props(default)] value_text: Option<String>,
) -> Element {
    let snapshot = controller.snapshot()();
    let position = controller.position()().as_secs_f64();
    let duration = controller.duration()().as_secs_f64();

    rsx! {
        AudioScrubber {
            position_secs: position,
            duration_secs: duration,
            disabled: !matches!(snapshot.source, PlaybackSourceLifecycle::Playable),
            aria_label: label,
            value_text,
            on_seek: move |seconds| controller.seek(Duration::from_secs_f64(seconds)),
        }
    }
}

#[component]
pub fn AudioPlayer(
    source: ReadSignal<Option<PlaybackSource>>,
    on_request_audio: EventHandler<()>,
    #[props(default = 0.0)] duration_secs: f64,
) -> Element {
    let controller = use_audio_player(
        source,
        Duration::from_secs_f64(finite_non_negative(duration_secs)),
    );
    let status = controller.status()();
    let snapshot = controller.snapshot()();
    let position = controller.position()().as_secs_f64();
    let duration = controller.duration()().as_secs_f64();
    let remaining = (duration - position).max(0.0);

    rsx! {
        div {
            class: "dioxus-audio dioxus-audio__player",
            "data-state": playback_state_name(&status),
            "data-source": source_lifecycle_name(&snapshot.source),
            "data-transport": transport_state_name(snapshot.transport),
            "data-readiness": readiness_state_name(snapshot.readiness),
            "data-network": network_activity_name(snapshot.network),
            "data-buffered": format_time_ranges(&snapshot.buffered),
            "data-seekable": format_time_ranges(&snapshot.seekable),
            "data-source-failure": source_failure_name(snapshot.source_failure.as_ref()),
            "data-play-failure": play_failure_name(snapshot.play_failure.as_ref()),
            "data-repeat": if snapshot.repeat { "true" } else { "false" },
            "data-muted": if snapshot.muted { "true" } else { "false" },
            "data-audibility-level": snapshot.audibility_level.value().to_string(),
            "data-audibility-capability": audibility_capability_name(snapshot.audibility_capability),
            PlaybackSeekSlider { controller }
            div { class: "dioxus-audio__player-times",
                span { "{format_time(position)}" }
                span { "-{format_time(remaining)}" }
            }
            div { class: "dioxus-audio__player-controls",
                PlaybackSkipButton { controller, seconds: -15.0 }
                PlaybackStopButton { controller }
                PlaybackPlayPauseButton { controller, on_request_audio }
                PlaybackSkipButton { controller, seconds: 15.0 }
                PlaybackRateButton { controller }
                PlaybackMuteButton { controller }
                PlaybackAudibilitySlider { controller }
                PlaybackRepeatButton { controller }
            }
            if let PlaybackStatus::Failed(ref error) = status {
                div {
                    class: "dioxus-audio__player-error",
                    role: "alert",
                    "{error}"
                }
            }
        }
    }
}

fn skip_label(seconds: f64) -> String {
    let amount = seconds.abs().to_string();
    let unit = if amount == "1" { "second" } else { "seconds" };
    if seconds < 0.0 {
        format!("Skip back {amount} {unit}")
    } else {
        format!("Skip forward {amount} {unit}")
    }
}

fn finite_non_negative(value: f64) -> f64 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn format_time(seconds: f64) -> String {
    let seconds = finite_non_negative(seconds) as u64;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

fn playback_state_name(status: &PlaybackStatus) -> &'static str {
    match status {
        PlaybackStatus::Empty => "empty",
        PlaybackStatus::Loading => "loading",
        PlaybackStatus::Ready => "ready",
        PlaybackStatus::Playing => "playing",
        PlaybackStatus::Paused => "paused",
        PlaybackStatus::Ended => "ended",
        PlaybackStatus::Failed(_) => "error",
    }
}

fn source_lifecycle_name(source: &PlaybackSourceLifecycle) -> &'static str {
    match source {
        PlaybackSourceLifecycle::Empty => "empty",
        PlaybackSourceLifecycle::Dormant => "dormant",
        PlaybackSourceLifecycle::Loading => "loading",
        PlaybackSourceLifecycle::Playable => "playable",
        PlaybackSourceLifecycle::Failed => "failed",
    }
}

fn transport_state_name(transport: PlaybackTransport) -> &'static str {
    match transport {
        PlaybackTransport::Idle => "idle",
        PlaybackTransport::PlayPending => "play-pending",
        PlaybackTransport::Playing => "playing",
        PlaybackTransport::Paused => "paused",
        PlaybackTransport::Ended => "ended",
    }
}

fn readiness_state_name(readiness: PlaybackReadiness) -> &'static str {
    match readiness {
        PlaybackReadiness::Unavailable => "unavailable",
        PlaybackReadiness::LoadingMetadata => "loading-metadata",
        PlaybackReadiness::Metadata => "metadata",
        PlaybackReadiness::Playable => "playable",
        PlaybackReadiness::Waiting => "waiting",
    }
}

fn play_failure_name(failure: Option<&PlaybackPlayFailure>) -> &'static str {
    match failure {
        None => "none",
        Some(PlaybackPlayFailure::InteractionRequired(_)) => "interaction-required",
        Some(PlaybackPlayFailure::Unknown(_)) => "unknown",
    }
}

fn network_activity_name(activity: PlaybackNetworkActivity) -> &'static str {
    match activity {
        PlaybackNetworkActivity::Inactive => "inactive",
        PlaybackNetworkActivity::Unknown => "unknown",
        PlaybackNetworkActivity::Loading => "loading",
        PlaybackNetworkActivity::Idle => "idle",
        PlaybackNetworkActivity::Stalled => "stalled",
    }
}

fn format_time_ranges(ranges: &[PlaybackTimeRange]) -> String {
    ranges
        .iter()
        .map(|range| {
            format!(
                "{}-{}",
                range.start().as_secs_f64(),
                range.end().as_secs_f64()
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn source_failure_name(failure: Option<&PlaybackSourceFailure>) -> &'static str {
    match failure {
        None => "none",
        Some(PlaybackSourceFailure::Unsupported(_)) => "unsupported",
        Some(PlaybackSourceFailure::Network(_)) => "network",
        Some(PlaybackSourceFailure::Decode(_)) => "decode",
        Some(PlaybackSourceFailure::Unknown(_)) => "unknown",
    }
}

fn audibility_capability_name(capability: PlaybackAudibilityCapability) -> &'static str {
    match capability {
        PlaybackAudibilityCapability::EffectiveGraphGain => "effective-graph-gain",
        PlaybackAudibilityCapability::BestEffortMediaElement => "best-effort-media-element",
        PlaybackAudibilityCapability::Unavailable => "unavailable",
    }
}
