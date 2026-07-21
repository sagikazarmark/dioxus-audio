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
            intro: "Render duration-aware magnitude or signed multichannel Waveform Data, then seek Playback and edit a controlled Waveform Selection through an interactive Waveform.",
        }
        ExampleSection {
            title: "Waveform components",
            intro: rsx! {
                InlineCode { "WaveformPreview" }
                " downsamples Peaks to fit its bar count. "
                InlineCode { "Waveform" }
                " selects a stored resolution by bucket budget and renders each channel without discarding signed shape. "
                InlineCode { "WaveformRangeSelector" }
                " reports an ordered selection in source seconds. "
                InlineCode { "InteractiveWaveform" }
                " adds three independently named native sliders with keyboard and pointer operation."
            },
            demo: rsx! { WaveformsExample {} },
            code: rsx! {
                Code { src: code!("/src/examples/waveforms.rs"), theme: snippet_theme() }
            },
        }
    }
}
