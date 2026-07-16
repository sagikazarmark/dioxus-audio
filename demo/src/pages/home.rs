use dioxus::prelude::*;
use dioxus_code::{Code, code};

use crate::app::Route;
use crate::components::{DocsCallout, ExternalAction, PageHeader, snippet_theme};

struct Feature {
    title: &'static str,
    body: &'static str,
    route: Route,
    cta: &'static str,
}

fn features() -> Vec<Feature> {
    vec![
        Feature {
            title: "Record and review",
            body: "Capture from a selected microphone, display live analysis, and replay the result.",
            route: Route::Recorder {},
            cta: "Open the recorder",
        },
        Feature {
            title: "Lazy playback",
            body: "Load bytes on demand with seeking, skip controls, and playback-rate changes.",
            route: Route::Playback {},
            cta: "Try the player",
        },
        Feature {
            title: "Audio inputs",
            body: "Request permission, enumerate microphones, and keep selection reactive.",
            route: Route::Devices {},
            cta: "Inspect devices",
        },
        Feature {
            title: "Live analysis",
            body: "Render waveforms, frequency spectra, and an RMS input level in real time.",
            route: Route::Visualizers {},
            cta: "See visualizers",
        },
        Feature {
            title: "Waveform UI",
            body: "Preview recorded peaks and edit a normalized range for trimming workflows.",
            route: Route::Waveforms {},
            cta: "Explore waveforms",
        },
        Feature {
            title: "Pure helpers",
            body: "Downsample peaks and trim interleaved PCM without depending on browser APIs.",
            route: Route::Analysis {},
            cta: "Run analysis",
        },
    ]
}

#[component]
pub fn Home() -> Element {
    rsx! {
        PageHeader {
            eyebrow: "dioxus-audio",
            title: "Browser audio building blocks for Dioxus",
            intro: "Hooks, state machines, data types, analysis helpers, and accessible components for recording and playing audio in Dioxus 0.7 applications.",
        }

        div { class: "mt-8 flex flex-wrap gap-3",
            Link { to: Route::Recorder {}, class: "btn btn-primary", "Record something" }
            Link { to: Route::Playback {}, class: "btn btn-ghost", "Try playback" }
        }

        DocsCallout {
            title: "Browser-first capture",
            action: Some(ExternalAction::new("Read the crate guide", "https://docs.rs/dioxus-audio")),
            "Recording, playback, and device discovery use browser Media APIs. Serve from localhost or HTTPS so the browser can grant microphone access. Pure analysis and visual components compile for other renderers."
        }

        div { class: "mt-10 grid gap-4 sm:grid-cols-2",
            for feature in features() {
                Link {
                    to: feature.route,
                    class: "group rounded-2xl border border-base-300 bg-base-100 p-5 transition-colors hover:border-primary/50",
                    h3 { class: "font-semibold tracking-tight", "{feature.title}" }
                    p { class: "mt-1.5 text-sm leading-6 text-base-content/65", "{feature.body}" }
                    span { class: "mt-3 inline-block text-sm font-medium text-primary",
                        "{feature.cta}"
                    }
                }
            }
        }

        section { class: "mt-12",
            h2 { class: "text-xl font-semibold tracking-tight", "Quick start" }
            p { class: "mt-2 max-w-[70ch] text-sm leading-6 text-base-content/65",
                "Load the authored package stylesheet once, then compose the device and recorder hooks with the UI components you need."
            }
            div { class: "mt-4 max-w-3xl overflow-x-auto rounded-2xl border border-base-300 bg-base-200/60 p-4 text-sm [&_pre]:!bg-transparent",
                Code { src: code!("/snippets/quickstart.rs"), theme: snippet_theme() }
            }
        }
    }
}
