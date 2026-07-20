use std::time::Duration;

use dioxus::prelude::*;
use dioxus_audio::analysis::WaveformSelection;
use dioxus_audio::components::{Waveform, WaveformPreview, WaveformRangeSelector};
use dioxus_audio::waveform::{SignedEnvelope, WaveformData, WaveformLevel};

/// Render compact Peaks and edit a source-time range over the same data.
#[component]
pub fn WaveformsExample() -> Element {
    let duration_secs = 12.0;
    let peaks = sample_peaks();
    let magnitude = WaveformData::from_peaks(Duration::from_secs_f64(duration_secs), peaks.clone())
        .expect("sample Peaks form valid Waveform Data");
    let signed_stereo = signed_stereo_data();
    let mut selection = use_signal(|| WaveformSelection::new(2.16, 9.84));
    let selected = selection();

    rsx! {
        div { class: "grid gap-6",
            div {
                p { class: "mb-2 text-xs font-semibold uppercase tracking-wider text-base-content/45", "Magnitude from Peaks" }
                Waveform {
                    data: magnitude,
                    bucket_budget: 240,
                    height: 72.0,
                    label: "Mono magnitude Waveform Data",
                }
                p { class: "mt-2 text-xs text-base-content/55", "Mono magnitude, evenly spaced across 12 seconds" }
            }
            div {
                p { class: "mb-2 text-xs font-semibold uppercase tracking-wider text-base-content/45", "Signed stereo envelope" }
                Waveform {
                    data: signed_stereo,
                    bucket_budget: 24,
                    height: 112.0,
                    label: "Stereo signed-envelope Waveform Data",
                }
                p { class: "mt-2 text-xs text-base-content/55", "Two channels, signed min/max shape, budget-selected coarse resolution" }
            }
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
                    duration_secs,
                    selection: selected,
                    on_change: move |next| selection.set(next),
                }
                p { class: "mt-3 text-center font-mono text-sm tabular-nums text-base-content/65",
                    "{selected.start():.2} s - {selected.end():.2} s"
                }
            }
        }
    }
}

fn signed_stereo_data() -> WaveformData {
    let fine = signed_stereo_buckets(48);
    let coarse = signed_stereo_buckets(12);
    WaveformData::from_signed_envelopes(
        Duration::from_secs(12),
        2,
        vec![
            WaveformLevel::new(Duration::from_millis(250), fine),
            WaveformLevel::new(Duration::from_secs(1), coarse),
        ],
    )
    .expect("sample signed envelopes form valid Waveform Data")
}

fn signed_stereo_buckets(bucket_count: usize) -> Vec<SignedEnvelope> {
    let mut buckets = Vec::with_capacity(bucket_count * 2);
    for channel in 0..2 {
        for index in 0..bucket_count {
            let phase = index as f32 / bucket_count as f32 * std::f32::consts::TAU;
            let energy = 0.18 + phase.sin().abs() * 0.72;
            buckets.push(if channel == 0 {
                SignedEnvelope {
                    min: -energy * 0.35,
                    max: energy,
                }
            } else {
                SignedEnvelope {
                    min: -energy,
                    max: energy * 0.45,
                }
            });
        }
    }
    buckets
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
