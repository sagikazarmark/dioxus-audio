use dioxus::prelude::*;
use dioxus_audio::analysis::WaveformSelection;
use dioxus_audio::components::{AudioPlayer, WaveformRangeSelector};

use super::fixtures::{generated_audio, peaks};

// region: scoped-recipe
#[component]
pub fn ScopedExample() -> Element {
    rsx! {
        div { class: "grid gap-4 xl:grid-cols-2",
            ClipEditor { theme: "Citrus", wrapper_class: "clip-editor citrus" }
            ClipEditor { theme: "Midnight", wrapper_class: "clip-editor midnight" }
        }
    }
}

#[component]
fn ClipEditor(#[props(into)] theme: String, #[props(into)] wrapper_class: String) -> Element {
    let mut selection = use_signal(|| WaveformSelection::new(0.36, 1.64));
    let mut source = use_signal(|| Some(generated_audio()));
    let selected = selection();

    rsx! {
        article { class: wrapper_class,
            header { class: "clip-editor__header",
                div {
                    p { class: "clip-editor__eyebrow", "Scoped clip editor" }
                    h3 { class: "clip-editor__title", "{theme}" }
                }
                span { class: "clip-editor__scope", "ordinary wrapper" }
            }

            ul { class: "clip-editor__facts", aria_label: "Clip details",
                li { "Generated WAV Audio Data" }
                li { "240 fixed Peaks" }
                li { "2 second duration" }
            }

            div { class: "clip-editor__range",
                p { class: "clip-editor__label", "Selected range" }
                WaveformRangeSelector {
                    peaks: peaks(),
                    duration_secs: 2.0,
                    selection: selected,
                    on_change: move |next| selection.set(next),
                    label: "Select clip range",
                }
                output { class: "clip-editor__selection",
                    "{selected.start():.2} s - {selected.end():.2} s"
                }
            }

            div { class: "clip-editor__playback",
                p { class: "clip-editor__label", "Playback" }
                AudioPlayer {
                    source,
                    duration_secs: 2.0,
                    on_request_audio: move |_| source.set(Some(generated_audio())),
                }
            }
        }
    }
}
// endregion: scoped-recipe
