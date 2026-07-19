use dioxus::prelude::*;
use dioxus_audio::components::{
    AudioInputSelector, AudioPlayer, LevelMeter, LiveWaveform, MicrophoneStatusIndicator,
    RecorderAnnouncementLabels, RecorderCancelButton, RecorderClearButton, RecorderControls,
    RecorderPauseResumeButton, RecorderStartButton, RecorderStatusAnnouncer, RecorderStopButton,
    SpectrumVisualizer, WaveformPreview,
};
use dioxus_audio::devices::{MicrophonePermission, use_audio_input_devices};
use dioxus_audio::recorder::{RecorderOptions, RecorderStatus, use_audio_recorder};
use dioxus_audio::{AudioData, RecordedAudio};

use crate::components::StatusChip;

/// Record from a selected input, inspect it, then play it back.
#[component]
pub fn RecorderExample() -> Element {
    let devices = use_audio_input_devices();
    let recorder = use_audio_recorder(RecorderOptions::default(), devices.selected().into());
    let custom_recorder = use_audio_recorder(RecorderOptions::default(), devices.selected().into());
    let mut completed = use_signal(|| None::<RecordedAudio>);
    let mut source = use_signal(|| None::<AudioData>);

    use_effect(move || {
        if recorder.completed().read().is_some()
            && let Some(recording) = recorder.take_completed()
        {
            source.set(Some(recording.audio.clone()));
            completed.set(Some(recording));
        }
    });

    let status = recorder.status()();
    let custom_status = custom_recorder.status()();
    let active = recorder_active(&status) || recorder_active(&custom_status);
    let permission = devices.permission()();
    let recording = completed.read().clone();
    let elapsed = format_duration(recorder.elapsed()().as_secs_f64());
    let custom_labels = RecorderAnnouncementLabels {
        idle: "Custom recorder idle".to_string(),
        requesting: "Custom recorder requesting microphone access".to_string(),
        recording: "Custom recording active".to_string(),
        paused: "Custom recording on hold".to_string(),
        stopping: "Custom recording finishing".to_string(),
        failed: "Custom recording failed".to_string(),
    };

    rsx! {
        div { class: "grid gap-5",
            div { class: "flex flex-wrap items-center justify-between gap-3",
                div {
                    p { class: "text-xs font-semibold uppercase tracking-wider text-base-content/45", "Capture status" }
                    p { class: "mt-1 font-mono text-2xl tabular-nums", "{elapsed}" }
                }
                StatusChip { label: format!("{status:?}") }
            }

            AudioInputSelector { devices, disabled: active }

            if permission != MicrophonePermission::Granted && !active {
                button {
                    class: "btn btn-sm btn-outline justify-self-start",
                    r#type: "button",
                    disabled: permission == MicrophonePermission::Prompt,
                    onclick: move |_| devices.request_permission(),
                    if permission == MicrophonePermission::Prompt {
                        "Requesting access..."
                    } else {
                        "Allow microphone access"
                    }
                }
            }

            MicrophoneStatusIndicator {
                status: recorder.microphone(),
                on_retry: move |_| {
                    let _ = recorder.start();
                },
            }

            div { class: "grid gap-4 md:grid-cols-2",
                div {
                    p { class: "mb-2 text-xs font-semibold uppercase tracking-wider text-base-content/45", "Waveform" }
                    LiveWaveform {
                        analyser: recorder.analyser(),
                        processing: matches!(status, RecorderStatus::Stopping),
                    }
                }
                div {
                    p { class: "mb-2 text-xs font-semibold uppercase tracking-wider text-base-content/45", "Spectrum" }
                    SpectrumVisualizer {
                        analyser: recorder.analyser(),
                        processing: matches!(status, RecorderStatus::Stopping),
                    }
                }
            }
            LevelMeter { analyser: recorder.analyser() }
            RecorderControls { recorder }
            div {
                class: "rounded-2xl border border-base-300 bg-base-100 p-4",
                role: "group",
                aria_label: "Independent recorder controls",
                p { class: "mb-3 text-xs font-semibold uppercase tracking-wider text-base-content/45",
                    "Independent controls"
                }
                RecorderStatusAnnouncer { recorder: custom_recorder, labels: custom_labels }
                div { class: "flex flex-wrap items-center justify-center gap-3",
                    RecorderStartButton {
                        recorder: custom_recorder,
                        label: "Begin custom recording".to_string(),
                        completed_label: "Clear custom recorded audio first".to_string(),
                    }
                    RecorderCancelButton {
                        recorder: custom_recorder,
                        request_label: "Abort custom microphone request".to_string(),
                        recording_label: "Discard custom recording".to_string(),
                    }
                    RecorderPauseResumeButton {
                        recorder: custom_recorder,
                        pause_label: "Hold custom recording".to_string(),
                        resume_label: "Continue custom recording".to_string(),
                    }
                    RecorderStopButton {
                        recorder: custom_recorder,
                        stop_label: "Finish custom recording".to_string(),
                        stopping_label: "Custom recording is finishing".to_string(),
                    }
                    RecorderClearButton {
                        recorder: custom_recorder,
                        label: "Clear custom recorded audio".to_string(),
                    }
                }
            }

            if let Some(recording) = recording {
                div {
                    class: "rounded-2xl border border-success/30 bg-success/5 p-4",
                    div { class: "flex flex-wrap items-center justify-between gap-2",
                        div { role: "status", aria_live: "polite",
                            p { class: "font-semibold", "Recording ready" }
                            p { class: "text-sm text-base-content/60",
                                "{format_duration(recording.duration.as_secs_f64())} | {recording.audio.mime_type} | {recording.audio.bytes.len()} bytes"
                            }
                        }
                        button {
                            class: "btn btn-ghost btn-sm",
                            r#type: "button",
                            onclick: move |_| {
                                completed.set(None);
                                source.set(None);
                            },
                            "Clear"
                        }
                    }
                    div { class: "mt-4",
                        WaveformPreview {
                            peaks: recording.peaks,
                            bars: 64,
                            height: 48.0,
                            label: "Recorded waveform",
                        }
                    }
                    AudioPlayer {
                        source,
                        duration_secs: recording.duration.as_secs_f64(),
                        on_request_audio: move |_| {},
                    }
                }
            }
        }
    }
}

fn format_duration(seconds: f64) -> String {
    let total_seconds = seconds.max(0.0) as u64;
    format!("{}:{:02}", total_seconds / 60, total_seconds % 60)
}

fn recorder_active(status: &RecorderStatus) -> bool {
    matches!(
        status,
        RecorderStatus::RequestingPermission
            | RecorderStatus::Recording
            | RecorderStatus::Paused
            | RecorderStatus::Stopping
    )
}
