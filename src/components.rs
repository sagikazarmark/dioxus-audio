//! Reusable visual audio components.

use dioxus::prelude::*;

mod devices;
mod player;
mod recorder;
mod visualizer;
mod waveform;

pub use devices::{AudioInputSelector, MicrophoneStatusIndicator};
pub use player::{
    AudioPlayer, PlaybackAnnouncementLabels, PlaybackPlayPauseButton, PlaybackRateButton,
    PlaybackRepeatButton, PlaybackSeekSlider, PlaybackSkipButton, PlaybackStatusAnnouncer,
    PlaybackStopButton,
};
pub use recorder::{
    RecorderAnnouncementLabels, RecorderCancelButton, RecorderClearButton, RecorderControls,
    RecorderPauseResumeButton, RecorderStartButton, RecorderStatusAnnouncer, RecorderStopButton,
};
pub use visualizer::{LevelMeter, LiveWaveform, SpectrumVisualizer};
pub use waveform::{WaveformPreview, WaveformRangeSelector};

/// The supported lower-level equivalent to rendering [`AudioStyles`].
///
/// Prefer [`AudioStyles`] unless the application needs to construct its own
/// document stylesheet element.
pub static STYLESHEET: Asset = asset!("/assets/dioxus-audio.css");

/// The canonical component stylesheet loader.
///
/// Render this once near the application root so all audio components can use
/// the packaged styles.
#[component]
pub fn AudioStyles() -> Element {
    rsx! { document::Stylesheet { href: STYLESHEET } }
}

#[component]
pub fn AudioScrubber(
    position_secs: f64,
    duration_secs: f64,
    on_seek: EventHandler<f64>,
    #[props(default = false)] disabled: bool,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] value_text: Option<String>,
) -> Element {
    let duration_secs = finite_non_negative(duration_secs);
    let position_secs = finite_non_negative(position_secs).min(duration_secs);
    let progress = if duration_secs > 0.0 {
        position_secs / duration_secs * 100.0
    } else {
        0.0
    };
    let aria_label = aria_label.unwrap_or_else(|| "Seek audio".to_string());
    let value_text = value_text.unwrap_or_else(|| {
        format!(
            "{} of {}",
            format_accessible_duration(position_secs),
            format_accessible_duration(duration_secs)
        )
    });

    rsx! {
        div { class: "dioxus-audio dioxus-audio__scrubber",
            div { class: "dioxus-audio__scrubber-track",
                div {
                    class: "dioxus-audio__scrubber-fill",
                    style: "width: {progress}%",
                }
            }
            input {
                class: "dioxus-audio__scrubber-input",
                r#type: "range",
                min: "0",
                max: "{duration_secs}",
                step: "0.01",
                value: "{position_secs}",
                disabled,
                aria_label,
                aria_valuetext: value_text,
                oninput: move |event| {
                    if let Ok(value) = event.value().parse::<f64>() {
                        on_seek.call(value);
                    }
                },
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

fn format_accessible_duration(seconds: f64) -> String {
    let minutes = (seconds / 60.0).floor() as u64;
    let seconds = seconds - minutes as f64 * 60.0;
    let seconds = format_number(seconds);

    if minutes == 0 {
        return format!(
            "{seconds} {}",
            if seconds == "1" { "second" } else { "seconds" }
        );
    }

    format!(
        "{minutes} {}, {seconds} {}",
        if minutes == 1 { "minute" } else { "minutes" },
        if seconds == "1" { "second" } else { "seconds" }
    )
}

fn format_number(value: f64) -> String {
    let rounded = (value * 100.0).round() / 100.0;
    rounded.to_string()
}
