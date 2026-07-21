use dioxus::prelude::*;
use dioxus_audio::analysis::{AudioAnalyser, LiveAnalysisOptions, use_live_analysis};
use dioxus_audio::components::{RecorderCancelButton, RecorderStartButton};
use dioxus_audio::recorder::{RecorderOptions, use_audio_recorder};
use std::time::Duration;

/// Inspect reactive snapshots while exercising Analyser and consumer lifetimes.
#[component]
pub fn LiveAnalysisExample() -> Element {
    let selected_input = use_signal(|| None);
    let mut recorder_options = RecorderOptions::default();
    recorder_options.peak_interval = Duration::from_secs(60);
    let primary_recorder = use_audio_recorder(recorder_options.clone(), selected_input.into());
    let replacement_recorder = use_audio_recorder(recorder_options, selected_input.into());
    let mut use_replacement = use_signal(|| false);
    let analyser = use_memo(move || {
        if use_replacement() {
            replacement_recorder.analyser()()
        } else {
            primary_recorder.analyser()()
        }
    });
    let analyser: ReadSignal<Option<AudioAnalyser>> = analyser.into();
    let mut primary_mounted = use_signal(|| true);
    let mut secondary_mounted = use_signal(|| true);

    rsx! {
        div { class: "grid gap-5",
            div { class: "flex flex-wrap items-center gap-3",
                RecorderStartButton {
                    recorder: primary_recorder,
                    label: "Start primary Recording".to_string(),
                }
                RecorderCancelButton {
                    recorder: primary_recorder,
                    preparing_label: "Cancel primary Recording".to_string(),
                    recording_label: "Cancel primary Recording".to_string(),
                }
                RecorderStartButton {
                    recorder: replacement_recorder,
                    label: "Start replacement Recording".to_string(),
                }
                RecorderCancelButton {
                    recorder: replacement_recorder,
                    preparing_label: "Cancel replacement Recording".to_string(),
                    recording_label: "Cancel replacement Recording".to_string(),
                }
                button {
                    class: "btn btn-sm btn-outline",
                    r#type: "button",
                    onclick: move |_| use_replacement.toggle(),
                    if use_replacement() {
                        "Use primary Analyser"
                    } else {
                        "Use replacement Analyser"
                    }
                }
                button {
                    class: "btn btn-sm btn-outline",
                    r#type: "button",
                    onclick: move |_| primary_mounted.toggle(),
                    if primary_mounted() {
                        "Unmount primary Analysis consumer"
                    } else {
                        "Mount primary Analysis consumer"
                    }
                }
                button {
                    class: "btn btn-sm btn-outline",
                    r#type: "button",
                    onclick: move |_| secondary_mounted.toggle(),
                    if secondary_mounted() {
                        "Unmount secondary Analysis consumer"
                    } else {
                        "Mount secondary Analysis consumer"
                    }
                }
            }
            div { class: "grid gap-1 text-xs text-base-content/60 sm:grid-cols-2",
                p {
                    if primary_recorder.analyser().read().is_some() {
                        "Primary Analyser available"
                    } else {
                        "Primary Analyser unavailable"
                    }
                }
                p {
                    if replacement_recorder.analyser().read().is_some() {
                        "Replacement Analyser available"
                    } else {
                        "Replacement Analyser unavailable"
                    }
                }
            }
            div { class: "grid gap-4 md:grid-cols-2",
                if primary_mounted() {
                    AnalysisConsumer {
                        name: "Primary",
                        analyser,
                        cadence: Duration::from_millis(40),
                    }
                }
                if secondary_mounted() {
                    AnalysisConsumer {
                        name: "Secondary",
                        analyser,
                        cadence: Duration::from_millis(80),
                    }
                }
            }
        }
    }
}

#[component]
fn AnalysisConsumer(
    name: String,
    analyser: ReadSignal<Option<AudioAnalyser>>,
    cadence: Duration,
) -> Element {
    let snapshot = use_live_analysis(
        analyser,
        LiveAnalysisOptions::default().with_cadence(cadence),
    );
    let snapshot = snapshot();

    rsx! {
        div {
            class: "rounded-2xl border border-base-300 bg-base-100 p-4 text-sm",
            role: "group",
            aria_label: "{name} Analysis consumer",
            p { class: "font-semibold", "{name} consumer" }
            if let Some(snapshot) = snapshot {
                {
                    let metadata = snapshot.metadata();
                    let time_sample = snapshot.time_domain().first().copied().unwrap_or(0.0);
                    let frequency_sample = snapshot
                        .frequency_domain()
                        .first()
                        .copied()
                        .unwrap_or(0.0);
                    rsx! {
                        p { class: "mt-2 text-success", "Analysis available" }
                        dl { class: "mt-3 grid gap-1 font-mono text-xs tabular-nums",
                            div { "Sample rate: {metadata.sample_rate():.0} Hz" }
                            div { "FFT size: {metadata.fft_size()}" }
                            div { "Frequency bins: {metadata.frequency_bin_count()}" }
                            div { "Bin width: {metadata.frequency_bin_width():.3} Hz" }
                            div { "Decibel range: {metadata.min_decibels():.0} to {metadata.max_decibels():.0} dB" }
                            div { "Smoothing: {metadata.smoothing():.1}" }
                            div { "Time sample: {time_sample:.3}" }
                            div { "Frequency value: {frequency_sample:.3}" }
                            div { "RMS level: {snapshot.level():.3}" }
                        }
                    }
                }
            } else {
                p { class: "mt-2 text-base-content/55", "Analysis unavailable" }
            }
        }
    }
}
