use dioxus::prelude::*;

use crate::analysis::{
    AnalysisDomain, AudioAnalyser, use_live_analysis_domain, use_live_analysis_level,
};
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use crate::analysis::{
    LiveAnalysisOptions, run_live_analysis_schedule, use_live_analysis_scheduler,
};

#[component]
pub fn LiveWaveform(
    analyser: ReadSignal<Option<AudioAnalyser>>,
    #[props(default = false)] processing: bool,
    #[props(default = 32)] bars: usize,
    #[props(default)] label: Option<String>,
) -> Element {
    LiveVisualizer(
        analyser,
        AnalysisDomain::Waveform,
        processing,
        bars,
        label.unwrap_or_else(|| "Live audio waveform".to_string()),
    )
}

#[component]
pub fn SpectrumVisualizer(
    analyser: ReadSignal<Option<AudioAnalyser>>,
    #[props(default = false)] processing: bool,
    #[props(default = 32)] bars: usize,
    #[props(default)] label: Option<String>,
) -> Element {
    LiveVisualizer(
        analyser,
        AnalysisDomain::Spectrum,
        processing,
        bars,
        label.unwrap_or_else(|| "Live audio spectrum".to_string()),
    )
}

#[allow(non_snake_case)]
fn LiveVisualizer(
    analyser: ReadSignal<Option<AudioAnalyser>>,
    domain: AnalysisDomain,
    processing: bool,
    bars: usize,
    label: String,
) -> Element {
    let bars = bars.clamp(1, 128);
    let samples = use_live_analysis_domain(analyser, domain);
    let has_samples = samples.read().is_some();
    let processing_values = use_processing_values(processing && !has_samples, bars);
    let values = samples()
        .map(|samples| reduce_samples(&samples, bars, domain))
        .unwrap_or_else(|| {
            if processing {
                processing_values()
            } else {
                vec![0.04; bars]
            }
        });

    rsx! {
        div {
            class: "dioxus-audio dioxus-audio__visualizer",
            role: "img",
            aria_label: label,
            "data-domain": match domain {
                AnalysisDomain::Waveform => "waveform",
                AnalysisDomain::Spectrum => "spectrum",
            },
            for (index, value) in values.iter().enumerate() {
                {
                    let bar_height = (value * 100.0).clamp(4.0, 100.0);
                    rsx! {
                        div {
                            key: "{index}",
                            class: "dioxus-audio__visualizer-bar",
                            style: "height: {bar_height}%",
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn LevelMeter(
    analyser: ReadSignal<Option<AudioAnalyser>>,
    #[props(default)] label: Option<String>,
) -> Element {
    let level = use_live_analysis_level(analyser);
    let percentage = (level().unwrap_or(0.0) * 100.0).clamp(0.0, 100.0);

    rsx! {
        div {
            class: "dioxus-audio dioxus-audio__meter",
            role: "meter",
            aria_label: label.unwrap_or_else(|| "Microphone level".to_string()),
            aria_valuemin: "0",
            aria_valuemax: "100",
            aria_valuenow: "{percentage:.0}",
            div {
                class: "dioxus-audio__meter-fill",
                style: "width: {percentage}%",
            }
        }
    }
}

fn use_processing_values(processing: bool, bars: usize) -> ReadSignal<Vec<f32>> {
    let parameters = use_memo(use_reactive!(|(processing, bars)| (processing, bars)));
    #[allow(unused_mut)]
    let mut values = use_signal(|| vec![0.04; bars]);
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    let scheduler = use_live_analysis_scheduler();

    use_effect(move || {
        let (processing, bars) = parameters();
        values.set(vec![0.04; bars]);
        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        let current_generation = scheduler.next_generation();
        if !processing {
            return;
        }
        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        {
            let scheduler = scheduler.clone();
            let mut tick = 0.0_f32;
            spawn(run_live_analysis_schedule(
                scheduler,
                current_generation,
                LiveAnalysisOptions::default().cadence(),
                move || {
                    tick += 0.18;
                    let next = (0..bars)
                        .map(|index| {
                            let phase = index as f32 / bars as f32 * std::f32::consts::TAU;
                            (0.3 + (phase + tick).sin().abs() * 0.55).clamp(0.04, 1.0)
                        })
                        .collect();
                    values.set(next);
                    true
                },
            ));
        }

        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        {
            let _ = (processing, bars);
        }
    });

    values.into()
}

fn reduce_samples(samples: &[f32], bars: usize, domain: AnalysisDomain) -> Vec<f32> {
    if samples.is_empty() {
        return vec![0.04; bars];
    }
    let bucket_count = samples.len().min(bars);
    let mut result: Vec<f32> = (0..bucket_count)
        .map(|index| {
            let start = index * samples.len() / bucket_count;
            let end = (index + 1) * samples.len() / bucket_count;
            samples[start..end]
                .iter()
                .map(|sample| match domain {
                    AnalysisDomain::Waveform => sample.abs(),
                    AnalysisDomain::Spectrum => *sample,
                })
                .fold(0.0_f32, f32::max)
                .clamp(0.04, 1.0)
        })
        .collect();
    result.resize(bars, 0.04);
    result
}
