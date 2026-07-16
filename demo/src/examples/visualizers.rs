use dioxus::prelude::*;
use dioxus_audio::analysis::AudioAnalyser;
use dioxus_audio::components::{LevelMeter, LiveWaveform, SpectrumVisualizer};

/// Preview visualizer states without requesting microphone access.
#[component]
pub fn VisualizersExample() -> Element {
    let analyser = use_signal(|| None::<AudioAnalyser>);
    let mut processing = use_signal(|| true);

    rsx! {
        div { class: "grid gap-5",
            label { class: "flex cursor-pointer items-center gap-3 text-sm font-medium",
                input {
                    class: "toggle toggle-primary toggle-sm",
                    r#type: "checkbox",
                    checked: processing(),
                    onchange: move |_| processing.toggle(),
                }
                "Show processing state"
            }
            div {
                p { class: "mb-2 text-xs font-semibold uppercase tracking-wider text-base-content/45", "Waveform" }
                LiveWaveform { analyser, processing: processing(), bars: 40 }
            }
            div {
                p { class: "mb-2 text-xs font-semibold uppercase tracking-wider text-base-content/45", "Spectrum" }
                SpectrumVisualizer { analyser, processing: processing(), bars: 40 }
            }
            div {
                p { class: "mb-2 text-xs font-semibold uppercase tracking-wider text-base-content/45", "Level" }
                LevelMeter { analyser }
            }
            p { class: "text-sm leading-6 text-base-content/60",
                "Pass recorder.analyser() to display live microphone data. The processing flag supplies an animated handoff while a recording is being finalized."
            }
        }
    }
}
