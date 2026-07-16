//! Reusable visual audio components.

use dioxus::prelude::*;

mod devices;
mod player;
mod recorder;
mod visualizer;
mod waveform;

pub use devices::{AudioInputSelector, MicrophoneStatusIndicator};
pub use player::AudioPlayer;
pub use recorder::RecorderControls;
pub use visualizer::{LevelMeter, LiveWaveform, SpectrumVisualizer};
pub use waveform::{WaveformPreview, WaveformRangeSelector};

pub static STYLESHEET: Asset = asset!("/assets/dioxus-audio.css");

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
) -> Element {
    let duration_secs = finite_non_negative(duration_secs);
    let position_secs = finite_non_negative(position_secs).min(duration_secs);
    let progress = if duration_secs > 0.0 {
        position_secs / duration_secs * 100.0
    } else {
        0.0
    };
    let aria_label = aria_label.unwrap_or_else(|| "Seek audio".to_string());

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
