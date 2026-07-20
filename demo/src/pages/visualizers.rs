use dioxus::prelude::*;
use dioxus_code::{Code, code};

use crate::components::{ExampleLayout, ExampleSection, InlineCode, PageHeader, snippet_theme};
use crate::examples::live_analysis::LiveAnalysisExample;
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
        ExampleSection {
            title: "Reactive Analysis snapshots",
            intro: rsx! {
                "Use " InlineCode { "use_live_analysis" }
                " with any optional Analyser. Each consumer owns a bounded schedule, clears snapshots when its Analyser is lost or replaced, pauses reads while the document is hidden, and stops on unmount."
            },
            demo: rsx! { LiveAnalysisExample {} },
            code: rsx! {
                Code { src: code!("/src/examples/live_analysis.rs"), theme: snippet_theme() }
            },
        }
    }
}
