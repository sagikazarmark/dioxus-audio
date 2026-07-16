use dioxus::prelude::*;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use std::cell::Cell;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use std::rc::Rc;

use crate::analysis::{AnalysisDomain, AudioAnalyser};

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
    let values = use_live_values(analyser, domain, processing, bars);

    rsx! {
        div {
            class: "dioxus-audio dioxus-audio__visualizer",
            role: "img",
            aria_label: label,
            "data-domain": match domain {
                AnalysisDomain::Waveform => "waveform",
                AnalysisDomain::Spectrum => "spectrum",
            },
            for (index, value) in values().iter().enumerate() {
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
    let level = use_live_level(analyser);
    let percentage = (level() * 100.0).clamp(0.0, 100.0);

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

fn use_live_values(
    analyser: ReadSignal<Option<AudioAnalyser>>,
    domain: AnalysisDomain,
    processing: bool,
    bars: usize,
) -> ReadSignal<Vec<f32>> {
    let parameters = use_memo(use_reactive!(|(analyser, processing, bars)| (
        analyser, processing, bars
    )));
    #[allow(unused_mut)]
    let mut values = use_signal(|| vec![0.04; bars]);
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    let generation = use_hook(|| Rc::new(Cell::new(0_u64)));

    use_effect(move || {
        let (analyser, processing, bars) = parameters();
        values.set(vec![0.04; bars]);
        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        let current_generation = {
            let current = generation.get().wrapping_add(1);
            generation.set(current);
            current
        };
        let has_analyser = analyser().is_some();
        if !has_analyser && !processing {
            return;
        }
        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        {
            let generation = generation.clone();
            spawn(async move {
                let mut tick = 0.0_f32;
                loop {
                    if generation.get() != current_generation {
                        break;
                    }
                    let next = if let Some(analyser) = analyser() {
                        reduce_samples(&analyser.read(domain), bars, domain)
                    } else if processing {
                        tick += 0.18;
                        (0..bars)
                            .map(|index| {
                                let phase = index as f32 / bars as f32 * std::f32::consts::TAU;
                                (0.3 + (phase + tick).sin().abs() * 0.55).clamp(0.04, 1.0)
                            })
                            .collect()
                    } else {
                        vec![0.04; bars]
                    };
                    values.set(next);
                    gloo_timers::future::TimeoutFuture::new(50).await;
                }
            });
        }

        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        {
            let _ = (analyser, domain, processing, bars);
        }
    });

    values.into()
}

fn use_live_level(analyser: ReadSignal<Option<AudioAnalyser>>) -> ReadSignal<f32> {
    let analyser_input = use_memo(use_reactive!(|(analyser,)| analyser));
    #[allow(unused_mut)]
    let mut level = use_signal(|| 0.0_f32);
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    let generation = use_hook(|| Rc::new(Cell::new(0_u64)));

    use_effect(move || {
        let analyser = analyser_input();
        level.set(0.0);
        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        let current_generation = {
            let current = generation.get().wrapping_add(1);
            generation.set(current);
            current
        };
        if analyser().is_none() {
            return;
        }
        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        {
            let generation = generation.clone();
            spawn(async move {
                loop {
                    if generation.get() != current_generation {
                        break;
                    }
                    level.set(analyser().map(|value| value.level()).unwrap_or(0.0));
                    gloo_timers::future::TimeoutFuture::new(50).await;
                }
            });
        }

        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        {
            let _ = analyser;
        }
    });

    level.into()
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
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
