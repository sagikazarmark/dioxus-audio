use std::time::Duration;

use dioxus::prelude::*;
use dioxus_audio::AudioData;
use dioxus_audio::analysis::WaveformSelection;
use dioxus_audio::components::{
    InteractiveWaveform, NavigableWaveform, PlaybackStatusAnnouncer, Waveform, WaveformPreview,
    WaveformRangeSelector,
};
use dioxus_audio::playback::{
    BoundedPlaybackFailure, BoundedPlaybackMode, BoundedPlaybackPhase, PlaybackSource,
    PlaybackSourceLifecycle, PlaybackTransport, use_audio_player,
};
use dioxus_audio::waveform::{SignedEnvelope, WaveformData, WaveformLevel, use_waveform_viewport};

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
    let long_form = use_memo(four_hour_signed_stereo_data);
    let long_form = long_form();
    let long_form_controller = use_waveform_viewport(
        long_form.duration(),
        Some(Duration::from_secs(60 * 60)..Duration::from_secs(90 * 60)),
    );
    let mut selection = use_signal(|| WaveformSelection::new(2.16, 9.84));
    let selected = selection();
    let mut interactive_selection = use_signal(|| WaveformSelection::new(2.25, 9.5));
    let mut interactive_commits = use_signal(|| 0_u32);
    let interactive_selected = interactive_selection();
    let mut primary_source = use_signal(|| Some(PlaybackSource::from(generated_audio(2, 330.0))));
    let primary_controller = use_audio_player(primary_source.into(), Duration::from_secs(2));
    let primary_snapshot = primary_controller.snapshot()();
    let mut primary_bounded_error = use_signal(|| None::<String>);
    let mut short_selection = use_signal(|| WaveformSelection::new(0.5, 3.5));
    let short_selected = short_selection();
    let mut short_source = use_signal(|| Some(PlaybackSource::from(generated_audio(4, 550.0))));
    let short_controller = use_audio_player(short_source.into(), Duration::from_secs(4));
    let short_snapshot = short_controller.snapshot()();
    let mut short_bounded_error = use_signal(|| None::<String>);

    rsx! {
        div { class: "grid gap-6",
            div { class: "min-w-0 rounded-2xl border border-base-300 bg-base-100 p-4",
                p { class: "mb-2 text-xs font-semibold uppercase tracking-wider text-base-content/45", "Four-hour navigable stereo waveform" }
                NavigableWaveform {
                    data: long_form,
                    controller: long_form_controller,
                    fallback_bucket_budget: 64,
                    height: 120.0,
                    label: "Four-hour stereo waveform".to_string(),
                }
                p { class: "mt-3 text-xs text-base-content/55",
                    "Four source-time resolutions; measured width selects bounded compact path geometry after the stable fallback render."
                }
            }
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
                    div { class: "mt-3 flex flex-wrap justify-center gap-2",
                        button {
                            class: "btn btn-primary btn-sm",
                            r#type: "button",
                            onclick: move |_| {
                                primary_bounded_error.set(
                                    primary_controller
                                        .play_bounded_once(interactive_selection())
                                        .err()
                                        .map(|error| error.to_string()),
                                );
                            },
                            "Play episode selection once"
                        }
                        button {
                            class: "btn btn-ghost btn-sm",
                            r#type: "button",
                            onclick: move |_| {
                                primary_source.set(Some(PlaybackSource::from(generated_audio(2, 440.0))));
                            },
                            "Replace episode source"
                        }
                        button {
                            class: "btn btn-ghost btn-sm",
                            r#type: "button",
                            onclick: move |_| primary_source.set(None),
                            "Unload episode source"
                        }
                    }
                    output {
                        class: "episode-bounded-playback-state mt-2 block text-center text-xs text-base-content/60",
                        "data-phase": bounded_phase_name(primary_snapshot.bounded.as_ref()),
                        "data-failure": bounded_failure_name(primary_snapshot.bounded.as_ref()),
                        "data-source": source_lifecycle_name(&primary_snapshot.source),
                        "data-transport": transport_name(primary_snapshot.transport),
                        "data-position": primary_controller.position()().as_secs_f64().to_string(),
                        if let Some(error) = primary_bounded_error() {
                            span { role: "alert", "{error}" }
                        } else {
                            span { "Episode selection Playback: {bounded_phase_name(primary_snapshot.bounded.as_ref())}" }
                        }
                    }
                }
                div { role: "group", aria_label: "Short Bounded Playback",
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
                    div { class: "mt-3 flex flex-wrap justify-center gap-2",
                        button {
                            class: "btn btn-primary btn-sm",
                            r#type: "button",
                            onclick: move |_| {
                                short_bounded_error.set(
                                    short_controller
                                        .play_bounded_once(short_selection())
                                        .err()
                                        .map(|error| error.to_string()),
                                );
                            },
                            "Play short selection once"
                        }
                        button {
                            class: "btn btn-primary btn-sm",
                            r#type: "button",
                            onclick: move |_| {
                                short_bounded_error.set(
                                    short_controller
                                        .play_bounded_loop(short_selection())
                                        .err()
                                        .map(|error| error.to_string()),
                                );
                            },
                            "Loop short selection"
                        }
                        button {
                            class: "btn btn-ghost btn-sm",
                            r#type: "button",
                            onclick: move |_| {
                                short_bounded_error.set(
                                    short_controller.pause().err().map(|error| error.to_string()),
                                );
                            },
                            "Pause Bounded Playback"
                        }
                        button {
                            class: "btn btn-ghost btn-sm",
                            r#type: "button",
                            onclick: move |_| {
                                short_bounded_error.set(
                                    short_controller.play().err().map(|error| error.to_string()),
                                );
                            },
                            "Resume Bounded Playback"
                        }
                        button {
                            class: "btn btn-ghost btn-sm",
                            r#type: "button",
                            onclick: move |_| short_controller.cancel_bounded_playback(),
                            "Cancel Bounded Playback"
                        }
                        button {
                            class: "btn btn-ghost btn-sm",
                            r#type: "button",
                            onclick: move |_| {
                                short_bounded_error.set(
                                    short_controller.stop().err().map(|error| error.to_string()),
                                );
                            },
                            "Stop short Playback"
                        }
                        button {
                            class: "btn btn-ghost btn-sm",
                            r#type: "button",
                            onclick: move |_| {
                                short_bounded_error.set(
                                    short_controller.set_rate(2.0).err().map(|error| error.to_string()),
                                );
                            },
                            "Set short Playback to 2x"
                        }
                        button {
                            class: "btn btn-ghost btn-sm",
                            r#type: "button",
                            aria_pressed: short_snapshot.repeat,
                            onclick: move |_| short_controller.toggle_repeat(),
                            "Toggle short whole-source repeat"
                        }
                        button {
                            class: "btn btn-ghost btn-sm",
                            r#type: "button",
                            onclick: move |_| short_controller.seek(Duration::from_secs(1)),
                            "Seek short Playback directly"
                        }
                        button {
                            class: "btn btn-ghost btn-sm",
                            r#type: "button",
                            onclick: move |_| {
                                short_source.set(Some(PlaybackSource::from(generated_audio(4, 660.0))));
                            },
                            "Replace short source"
                        }
                        button {
                            class: "btn btn-ghost btn-sm",
                            r#type: "button",
                            onclick: move |_| short_source.set(None),
                            "Unload short source"
                        }
                    }
                    output {
                        class: "short-bounded-playback-state mt-2 block text-center text-xs text-base-content/60",
                        "data-phase": bounded_phase_name(short_snapshot.bounded.as_ref()),
                        "data-mode": bounded_mode_name(short_snapshot.bounded.as_ref()),
                        "data-failure": bounded_failure_name(short_snapshot.bounded.as_ref()),
                        "data-source": source_lifecycle_name(&short_snapshot.source),
                        "data-transport": transport_name(short_snapshot.transport),
                        "data-repeat": short_snapshot.repeat,
                        "data-rate": short_controller.rate()().to_string(),
                        "data-position": short_controller.position()().as_secs_f64().to_string(),
                        if let Some(error) = short_bounded_error() {
                            span { role: "alert", "{error}" }
                        } else {
                            span { "Bounded Playback for short selection: {bounded_phase_name(short_snapshot.bounded.as_ref())}" }
                        }
                    }
                    PlaybackStatusAnnouncer { controller: short_controller }
                }
            }
        }
    }
}

fn bounded_failure_name(
    bounded: Option<&dioxus_audio::playback::BoundedPlaybackSnapshot>,
) -> &'static str {
    match bounded.map(|bounded| bounded.phase) {
        Some(BoundedPlaybackPhase::Failed(BoundedPlaybackFailure::SeekTimedOut)) => "seek-timeout",
        Some(BoundedPlaybackPhase::Failed(BoundedPlaybackFailure::ActivationRejected)) => {
            "activation-rejected"
        }
        Some(BoundedPlaybackPhase::Failed(BoundedPlaybackFailure::PauseRejected)) => {
            "pause-rejected"
        }
        _ => "none",
    }
}

fn bounded_phase_name(
    bounded: Option<&dioxus_audio::playback::BoundedPlaybackSnapshot>,
) -> &'static str {
    match bounded.map(|bounded| bounded.phase) {
        None => "none",
        Some(BoundedPlaybackPhase::Seeking) => "seeking",
        Some(BoundedPlaybackPhase::Activating) => "activating",
        Some(BoundedPlaybackPhase::Active) => "active",
        Some(BoundedPlaybackPhase::Paused) => "paused",
        Some(BoundedPlaybackPhase::Retargeting) => "retargeting",
        Some(BoundedPlaybackPhase::Wrapping) => "wrapping",
        Some(BoundedPlaybackPhase::Completed) => "completed",
        Some(BoundedPlaybackPhase::Cancelled) => "cancelled",
        Some(BoundedPlaybackPhase::Failed(_)) => "failed",
        Some(_) => "unknown",
    }
}

fn bounded_mode_name(
    bounded: Option<&dioxus_audio::playback::BoundedPlaybackSnapshot>,
) -> &'static str {
    match bounded.map(|bounded| bounded.mode) {
        None => "none",
        Some(BoundedPlaybackMode::Once) => "once",
        Some(BoundedPlaybackMode::Loop) => "loop",
        Some(_) => "unknown",
    }
}

fn source_lifecycle_name(source: &PlaybackSourceLifecycle) -> &'static str {
    match source {
        PlaybackSourceLifecycle::Empty => "empty",
        PlaybackSourceLifecycle::Dormant => "dormant",
        PlaybackSourceLifecycle::Loading => "loading",
        PlaybackSourceLifecycle::Playable => "playable",
        PlaybackSourceLifecycle::Failed => "failed",
        _ => "unknown",
    }
}

fn transport_name(transport: PlaybackTransport) -> &'static str {
    match transport {
        PlaybackTransport::Idle => "idle",
        PlaybackTransport::PlayPending => "play-pending",
        PlaybackTransport::Playing => "playing",
        PlaybackTransport::Paused => "paused",
        PlaybackTransport::Ended => "ended",
        _ => "unknown",
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

fn four_hour_signed_stereo_data() -> WaveformData {
    let duration = Duration::from_secs(4 * 60 * 60);
    WaveformData::from_signed_envelopes(
        duration,
        2,
        vec![
            WaveformLevel::new(
                Duration::from_millis(250),
                long_signed_stereo_buckets(57_600),
            ),
            WaveformLevel::new(Duration::from_secs(1), long_signed_stereo_buckets(14_400)),
            WaveformLevel::new(Duration::from_secs(5), long_signed_stereo_buckets(2_880)),
            WaveformLevel::new(Duration::from_secs(30), long_signed_stereo_buckets(480)),
        ],
    )
    .expect("representative long-form envelopes form valid Waveform Data")
}

fn long_signed_stereo_buckets(bucket_count: usize) -> Vec<SignedEnvelope> {
    let mut buckets = Vec::with_capacity(bucket_count * 2);
    for channel in 0..2 {
        for index in 0..bucket_count {
            let primary = (index as f32 * 0.013 + channel as f32 * 0.7).sin();
            let detail = (index as f32 * 0.071).cos() * 0.16;
            let upper = (0.2 + primary.abs() * 0.65 + detail).clamp(0.05, 0.95);
            let lower = (0.15 + primary.abs() * 0.5 - detail).clamp(0.05, 0.9);
            buckets.push(if channel == 0 {
                SignedEnvelope {
                    min: -lower * 0.55,
                    max: upper,
                }
            } else {
                SignedEnvelope {
                    min: -lower,
                    max: upper * 0.6,
                }
            });
        }
    }
    buckets
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
