use dioxus::prelude::*;
use dioxus_audio::analysis::WaveformSelection;
use dioxus_audio::components::{WaveformPreview, WaveformRangeSelector};

/// Render compact peaks and edit a normalized range over the same data.
#[component]
pub fn WaveformsExample() -> Element {
    let peaks = sample_peaks();
    let mut selection = use_signal(|| WaveformSelection::new(0.18, 0.82));
    let selected = selection();

    rsx! {
        div { class: "grid gap-6",
            div {
                p { class: "mb-2 text-xs font-semibold uppercase tracking-wider text-base-content/45", "Compact preview" }
                WaveformPreview {
                    peaks: peaks.clone(),
                    bars: 72,
                    height: 64.0,
                    label: "Sample waveform",
                }
            }
            div {
                p { class: "mb-2 text-xs font-semibold uppercase tracking-wider text-base-content/45", "Range selector" }
                WaveformRangeSelector {
                    peaks,
                    selection: selected,
                    on_change: move |next| selection.set(next),
                }
                p { class: "mt-3 text-center font-mono text-sm tabular-nums text-base-content/65",
                    "{selected.start() * 100.0:.1}% - {selected.end() * 100.0:.1}%"
                }
            }
        }
    }
}

fn sample_peaks() -> Vec<u8> {
    (0..240)
        .map(|index| {
            let primary = (index as f32 * 0.17).sin().abs();
            let detail = (index as f32 * 0.61).sin().abs() * 0.28;
            ((primary + detail).min(1.0) * 230.0) as u8 + 12
        })
        .collect()
}
