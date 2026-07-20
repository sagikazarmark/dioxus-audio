use dioxus::prelude::*;
use dioxus_code::{Code, code};

use crate::components::{ExampleLayout, ExampleSection, InlineCode, PageHeader, snippet_theme};
use crate::examples::analysis::AnalysisExample;

#[component]
pub fn Analysis() -> Element {
    rsx! {
        PageHeader {
            eyebrow: "Processing",
            title: "Process audio data without a browser",
            intro: "Peak reduction, unsigned Web Audio amplitude measurement, source-time selections, and frame-safe PCM trimming are platform-independent helpers.",
        }
        ExampleSection {
            title: "Analysis helpers",
            layout: ExampleLayout::Columns,
            intro: rsx! {
                "Use " InlineCode { "downsample_peaks" }
                " for compact previews and " InlineCode { "trim_interleaved_pcm" }
                " to keep channel frames aligned while applying a source-time Waveform Selection."
            },
            demo: rsx! { AnalysisExample {} },
            code: rsx! {
                Code { src: code!("/src/examples/analysis.rs"), theme: snippet_theme() }
            },
        }
    }
}
