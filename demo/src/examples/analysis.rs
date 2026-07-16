use dioxus::prelude::*;
use dioxus_audio::analysis::{
    WaveformSelection, downsample_peaks, peak_amplitude, trim_interleaved_pcm,
};
use dioxus_audio::components::WaveformPreview;

/// Exercise the platform-independent helpers with reactive bucket sizing.
#[component]
pub fn AnalysisExample() -> Element {
    let source = source_peaks();
    let mut buckets = use_signal(|| 16_usize);
    let reduced = downsample_peaks(&source, buckets());
    let amplitude = peak_amplitude(&[128, 142, 96, 205, 81, 128]);
    let stereo = vec![0_i16, 10, 1, 11, 2, 12, 3, 13, 4, 14, 5, 15];
    let trimmed = trim_interleaved_pcm(&stereo, 2, WaveformSelection::new(0.25, 0.75));

    rsx! {
        div { class: "grid gap-5",
            label { class: "grid gap-2",
                span { class: "flex items-center justify-between text-sm font-medium",
                    "Peak buckets"
                    span { class: "font-mono text-base-content/60", "{buckets}" }
                }
                input {
                    class: "range range-primary range-sm",
                    r#type: "range",
                    min: "4",
                    max: "48",
                    value: buckets(),
                    oninput: move |event| {
                        if let Ok(value) = event.value().parse() {
                            buckets.set(value);
                        }
                    },
                }
            }
            WaveformPreview {
                peaks: reduced.clone(),
                bars: buckets(),
                height: 56.0,
                label: "Downsampled peaks",
            }
            div { class: "grid gap-3 text-sm sm:grid-cols-3",
                Metric { label: "Source peaks", value: source.len().to_string() }
                Metric { label: "Reduced peaks", value: reduced.len().to_string() }
                Metric { label: "Peak amplitude", value: amplitude.to_string() }
            }
            div { class: "rounded-xl border border-base-300 bg-base-100 p-4",
                p { class: "text-xs font-semibold uppercase tracking-wider text-base-content/45", "Frame-safe PCM trim" }
                p { class: "mt-2 font-mono text-sm", "{trimmed:?}" }
                p { class: "mt-1 text-sm text-base-content/60", "Four interleaved stereo frames remain aligned." }
            }
        }
    }
}

#[component]
fn Metric(label: &'static str, value: String) -> Element {
    rsx! {
        div { class: "rounded-xl bg-base-100 p-3",
            p { class: "text-xs text-base-content/50", "{label}" }
            p { class: "mt-1 font-mono text-lg", "{value}" }
        }
    }
}

fn source_peaks() -> Vec<u8> {
    (0..96)
        .map(|index| ((index as f32 * 0.31).sin().abs() * 245.0) as u8)
        .collect()
}
