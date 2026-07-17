use dioxus::prelude::*;
use dioxus_audio::AudioData;
use dioxus_audio::components::{AudioPlayer, WaveformPreview};

use super::fixtures::{generated_audio, peaks};

// region: daisy-recipe
#[component]
pub fn DaisyExample() -> Element {
    let mut source = use_signal(|| None::<AudioData>);

    rsx! {
        article { class: "rounded-2xl border border-base-300 bg-base-100 p-5 shadow-sm sm:p-6",
            header { class: "flex flex-col gap-1 border-b border-base-300 pb-4 sm:flex-row sm:items-end sm:justify-between",
                div {
                    p { class: "text-xs font-semibold uppercase tracking-[0.16em] text-base-content/55", "Host-themed playback" }
                    h3 { class: "mt-1 text-xl font-semibold tracking-tight", "Generated tone" }
                }
                span { class: "text-xs text-base-content/55", "2 second Audio Data" }
            }

            div { class: "mt-5 grid min-w-0 gap-5",
                WaveformPreview {
                    peaks: peaks(),
                    bars: 72,
                    height: 68.0,
                    label: "Host theme waveform",
                }
                AudioPlayer {
                    source,
                    duration_secs: 2.0,
                    on_request_audio: move |_| source.set(Some(generated_audio())),
                }
            }
        }
    }
}
// endregion: daisy-recipe
