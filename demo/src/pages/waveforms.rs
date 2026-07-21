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
            intro: "Navigate measured long-form Waveform Data, render duration-aware magnitude or signed multichannel shapes, then seek Playback, edit a controlled Waveform Selection, and play or loop a valid selection.",
        }
        ExampleSection {
            title: "Waveform components",
            intro: rsx! {
                InlineCode { "WaveformPreview" }
                " downsamples Peaks to fit its bar count. "
                InlineCode { "Waveform" }
                " selects a stored resolution by bucket budget and renders each channel without discarding signed shape. "
                InlineCode { "NavigableWaveform" }
                " combines a source-time Waveform Viewport Controller, measured resolution budget, compact channel paths, native navigation buttons, and an optional overview slider. "
                InlineCode { "WaveformRangeSelector" }
                " reports an ordered selection in source seconds. "
                InlineCode { "InteractiveWaveform" }
                " adds three independently named native sliders with keyboard and pointer operation. "
                InlineCode { "play_bounded_once" }
                " and "
                InlineCode { "play_bounded_loop" }
                " pause-seek before requesting play and expose their lifecycle independently. Committed selection edits retarget an enforcing run, while a late hidden loop restarts once from its range start. Operation ordering is guaranteed; audible boundary timing is best-effort, not sample-accurate or gapless, and has no maximum overshoot promise."
            },
            demo: rsx! { WaveformsExample {} },
            code: rsx! {
                Code { src: code!("/src/examples/waveforms.rs"), theme: snippet_theme() }
            },
        }
    }
}
