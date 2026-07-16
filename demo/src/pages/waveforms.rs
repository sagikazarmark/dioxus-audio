use dioxus::prelude::*;
use dioxus_code::{Code, code};

use crate::components::{ExampleSection, InlineCode, PageHeader, snippet_theme};
use crate::examples::waveforms::WaveformsExample;

#[component]
pub fn Waveforms() -> Element {
    rsx! {
        PageHeader {
            eyebrow: "Processing",
            title: "Preview and select waveform ranges",
            intro: "Render recorded peak envelopes at any size and expose an accessible normalized range selector for trimming workflows.",
        }
        ExampleSection {
            title: "Waveform components",
            intro: rsx! {
                InlineCode { "WaveformPreview" }
                " downsamples peaks to fit its bar count. "
                InlineCode { "WaveformRangeSelector" }
                " reports a normalized selection from 0.0 to 1.0."
            },
            demo: rsx! { WaveformsExample {} },
            code: rsx! {
                Code { src: code!("/src/examples/waveforms.rs"), theme: snippet_theme() }
            },
        }
    }
}
