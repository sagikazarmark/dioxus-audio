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
            intro: "Render duration-aware magnitude or signed multichannel Waveform Data alongside compact Peaks and an accessible range selector.",
        }
        ExampleSection {
            title: "Waveform components",
            intro: rsx! {
                InlineCode { "WaveformPreview" }
                " downsamples Peaks to fit its bar count. "
                InlineCode { "Waveform" }
                " selects a stored resolution by bucket budget and renders each channel without discarding signed shape. "
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
