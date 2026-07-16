use std::time::Duration;

use dioxus::prelude::*;
use dioxus_icons::lucide::{Pause, Play, RotateCcw, RotateCw};

use super::AudioScrubber;
use crate::AudioData;
use crate::playback::{PlaybackStatus, use_audio_player};

const PLAYBACK_RATES: [f64; 3] = [1.0, 1.5, 2.0];

#[component]
pub fn AudioPlayer(
    source: ReadSignal<Option<AudioData>>,
    on_request_audio: EventHandler<()>,
    #[props(default = 0.0)] duration_secs: f64,
) -> Element {
    let mut play_requested = use_signal(|| false);
    let source_input = use_memo(use_reactive!(|(source,)| source));
    let controller = use_audio_player(
        source,
        Duration::from_secs_f64(finite_non_negative(duration_secs)),
    );
    let status = controller.status()();
    let position = controller.position()().as_secs_f64();
    let duration = controller.duration()().as_secs_f64();
    let rate = controller.rate()();
    let rate_label = format!("Playback speed: {rate}x");
    let has_source = source.read().is_some();
    let playing = matches!(status, PlaybackStatus::Playing);
    let remaining = (duration - position).max(0.0);

    use_effect(move || {
        if play_requested()
            && source_input().read().is_some()
            && matches!(controller.status()(), PlaybackStatus::Ready)
            && controller.play().is_ok()
        {
            play_requested.set(false);
        }
    });

    rsx! {
        div {
            class: "dioxus-audio dioxus-audio__player",
            "data-state": playback_state_name(&status),
            AudioScrubber {
                position_secs: position,
                duration_secs: duration,
                disabled: matches!(status, PlaybackStatus::Empty | PlaybackStatus::Loading),
                on_seek: move |seconds| controller.seek(Duration::from_secs_f64(seconds)),
            }
            div { class: "dioxus-audio__player-times",
                span { "{format_time(position)}" }
                span { "-{format_time(remaining)}" }
            }
            div { class: "dioxus-audio__player-controls",
                button {
                    class: "dioxus-audio__control",
                    r#type: "button",
                    aria_label: "Skip back 15 seconds",
                    disabled: !has_source,
                    onclick: move |_| controller.skip(-15.0),
                    RotateCcw { size: 20 }
                }
                button {
                    class: "dioxus-audio__control dioxus-audio__control--primary",
                    r#type: "button",
                    aria_label: if playing { "Pause" } else { "Play" },
                    disabled: has_source && matches!(status, PlaybackStatus::Empty | PlaybackStatus::Loading),
                    onclick: move |_| {
                        if !has_source {
                            play_requested.set(true);
                            on_request_audio.call(());
                        } else if playing {
                            let _ = controller.pause();
                        } else {
                            let _ = controller.play();
                        }
                    },
                    if playing {
                        Pause { size: 28 }
                    } else {
                        Play { size: 28 }
                    }
                }
                button {
                    class: "dioxus-audio__control",
                    r#type: "button",
                    aria_label: "Skip forward 15 seconds",
                    disabled: !has_source,
                    onclick: move |_| controller.skip(15.0),
                    RotateCw { size: 20 }
                }
                button {
                    class: "dioxus-audio__rate",
                    r#type: "button",
                    aria_label: rate_label,
                    onclick: move |_| {
                        let current = PLAYBACK_RATES
                            .iter()
                            .position(|candidate| (*candidate - rate).abs() < f64::EPSILON)
                            .unwrap_or(0);
                        let _ = controller.set_rate(PLAYBACK_RATES[(current + 1) % PLAYBACK_RATES.len()]);
                    },
                    "{rate}x"
                }
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
