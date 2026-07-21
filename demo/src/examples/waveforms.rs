use std::time::Duration;

use dioxus::prelude::*;
use dioxus_audio::AudioData;
use dioxus_audio::analysis::WaveformSelection;
use dioxus_audio::components::{
    InteractiveWaveform, Waveform, WaveformPreview, WaveformRangeSelector,
};
use dioxus_audio::playback::{PlaybackSource, use_audio_player};
use dioxus_audio::waveform::{SignedEnvelope, WaveformData, WaveformLevel};

/// Render compact Peaks and edit a source-time range over the same data.
#[component]
pub fn WaveformsExample() -> Element {
    let duration_secs = 12.0;
    let peaks = sample_peaks();
    let magnitude = WaveformData::from_peaks(Duration::from_secs_f64(duration_secs), peaks.clone())
        .expect("sample Peaks form valid Waveform Data");
    let short_waveform = WaveformData::from_peaks(Duration::from_secs(4), peaks.clone())
        .expect("sample Peaks form valid short Waveform Data");
    let signed_stereo = signed_stereo_data();
    let mut selection = use_signal(|| WaveformSelection::new(2.16, 9.84));
    let selected = selection();
    let mut interactive_selection = use_signal(|| WaveformSelection::new(2.25, 9.5));
    let mut interactive_commits = use_signal(|| 0_u32);
    let interactive_selected = interactive_selection();
    let primary_source = use_signal(|| Some(PlaybackSource::from(generated_audio(2, 330.0))));
    let primary_controller = use_audio_player(primary_source.into(), Duration::from_secs(2));
    let mut short_selection = use_signal(|| WaveformSelection::new(0.5, 3.5));
    let short_selected = short_selection();
    let short_source = use_signal(|| Some(PlaybackSource::from(generated_audio(4, 550.0))));
    let short_controller = use_audio_player(short_source.into(), Duration::from_secs(4));

    rsx! {
        div { class: "grid gap-6",
            div {
                p { class: "mb-2 text-xs font-semibold uppercase tracking-wider text-base-content/45", "Magnitude from Peaks" }
                Waveform {
                    data: magnitude.clone(),
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
            div { class: "grid gap-5 rounded-2xl border border-base-300 bg-base-100 p-4",
                div {
                    p { class: "mb-2 text-xs font-semibold uppercase tracking-wider text-base-content/45", "Interactive timeline" }
                    InteractiveWaveform {
                        data: magnitude,
                        controller: primary_controller,
                        selection: interactive_selected,
                        on_selection_change: move |next| {
                            interactive_selection.set(next);
                            interactive_commits += 1;
                        },
                        fine_step_secs: 0.25,
                        coarse_step_secs: 2.0,
                        height: 88.0,
                        label: "Interactive episode waveform".to_string(),
                        playback_label: "Episode playback position".to_string(),
                        selection_start_label: "Episode selection start".to_string(),
                        selection_end_label: "Episode selection end".to_string(),
                    }
                    p { class: "interactive-selection-state mt-3 text-center font-mono text-sm tabular-nums text-base-content/65",
                        "Committed selection: {interactive_selected.start():.2} s to {interactive_selected.end():.2} s"
                    }
                    p { class: "interactive-selection-commits mt-1 text-center text-xs text-base-content/50",
                        "Selection commits: {interactive_commits}"
                    }
                    p { class: "mt-1 text-center text-xs text-base-content/50",
                        "12-second Waveform; authoritative Playback duration: 2 seconds"
                    }
                }
                div {
                    InteractiveWaveform {
                        data: short_waveform,
                        controller: short_controller,
                        selection: short_selected,
                        on_selection_change: move |next| short_selection.set(next),
                        fine_step_secs: 0.5,
                        coarse_step_secs: 1.0,
                        height: 56.0,
                        label: "Independent short waveform".to_string(),
                        playback_label: "Short playback position".to_string(),
                        selection_start_label: "Short selection start".to_string(),
                        selection_end_label: "Short selection end".to_string(),
                    }
                    p { class: "mt-2 text-center text-xs text-base-content/50",
                        "Independent selection: {short_selected.start():.2} s to {short_selected.end():.2} s"
                    }
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

fn generated_audio(seconds: u32, frequency: f32) -> AudioData {
    const SAMPLE_RATE: u32 = 8_000;
    const BITS_PER_SAMPLE: u16 = 16;

    let sample_count = SAMPLE_RATE * seconds;
    let data_size = sample_count * u32::from(BITS_PER_SAMPLE / 8);
    let mut bytes = Vec::with_capacity(44 + data_size as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_size).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    bytes.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    bytes.extend_from_slice(&2_u16.to_le_bytes());
    bytes.extend_from_slice(&BITS_PER_SAMPLE.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_size.to_le_bytes());

    for index in 0..sample_count {
        let time = index as f32 / SAMPLE_RATE as f32;
        let sample = (frequency * time * std::f32::consts::TAU).sin() * 0.12;
        bytes.extend_from_slice(&((sample * i16::MAX as f32) as i16).to_le_bytes());
    }

    AudioData::new(bytes, "audio/wav")
}
