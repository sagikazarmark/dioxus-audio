use dioxus::prelude::*;
use dioxus_code::{Code, code};

use crate::components::{ExampleLayout, ExampleSection, InlineCode, PageHeader, snippet_theme};
use crate::examples::visualizers::VisualizersExample;

#[component]
pub fn Visualizers() -> Element {
    rsx! {
        PageHeader {
            eyebrow: "Input",
            title: "Render live audio analysis",
            intro: "Waveform, frequency spectrum, and RMS level components read from the recorder's shared Web Audio analyser.",
        }
        ExampleSection {
            title: "Live visualizers",
            layout: ExampleLayout::Columns,
            intro: rsx! {
                "Connect each component to " InlineCode { "recorder.analyser()" }
                ". This preview uses the processing state, so it needs no microphone permission."
            },
            demo: rsx! { VisualizersExample {} },
            code: rsx! {
                Code { src: code!("/src/examples/visualizers.rs"), theme: snippet_theme() }
            },
        }
    }
}
