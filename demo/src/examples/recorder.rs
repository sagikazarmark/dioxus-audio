use dioxus::prelude::*;
use dioxus_audio::components::{
    AudioInputSelector, AudioPlayer, LevelMeter, LiveWaveform, MicrophoneStatusIndicator,
    RecorderAnnouncementLabels, RecorderCancelButton, RecorderClearButton, RecorderControls,
    RecorderPauseResumeButton, RecorderStartButton, RecorderStatusAnnouncer, RecorderStopButton,
    SpectrumVisualizer, WaveformPreview,
};
use dioxus_audio::devices::{MicrophonePermission, use_audio_input_devices};
use dioxus_audio::playback::PlaybackSource;
use dioxus_audio::recorder::{
    RecorderOptions, RecorderStatus, RecordingChunkDelivery, RecordingConstraint,
    RecordingConstraints, RecordingOutcome, RecordingSource, RecordingSourceShutdown,
    is_recorder_mime_type_supported, use_audio_recorder,
};
use dioxus_audio::{RecordedAudio, RecordingChunk, RecordingId};
use wasm_bindgen::JsCast;

use crate::components::StatusChip;

struct ApplicationRecordingSource {
    recorder_source: RecordingSource,
    stream: web_sys::MediaStream,
}

impl Drop for ApplicationRecordingSource {
    fn drop(&mut self) {
        stop_application_source(self);
    }
}

/// Record from a selected input, inspect it, then play it back.
#[component]
pub fn RecorderExample() -> Element {
    let mut mounted = use_signal(|| true);
    let mut events = use_signal(Vec::<String>::new);
    let mut recorder_number = use_signal(|| 1_u64);
    let supplied_source = use_signal(|| None::<ApplicationRecordingSource>);

    rsx! {
        div { class: "grid gap-5",
            div { class: "flex flex-wrap items-center gap-2",
                button {
                    class: "btn btn-sm btn-outline",
                    r#type: "button",
                    onclick: move |_| {
                        if mounted() {
                            mounted.set(false);
                        } else {
                            recorder_number += 1;
                            mounted.set(true);
                        }
                    },
                    if mounted() { "Unmount recorder" } else { "Remount recorder" }
                }
                button {
                    class: "btn btn-sm btn-ghost",
                    r#type: "button",
                    onclick: move |_| events.write().clear(),
                    "Clear Recording lifecycle events"
                }
            }
            if mounted() {
                RecorderPanel { events, recorder_number: recorder_number(), supplied_source }
            } else {
                p { role: "status", "Recorder unmounted" }
            }
            div {
                class: "rounded-2xl border border-base-300 bg-base-100 p-4",
                role: "log",
                aria_label: "Recording lifecycle events",
                p { class: "text-xs font-semibold uppercase tracking-wider text-base-content/45",
                    "Recording lifecycle events"
                }
                ul { class: "mt-2 grid gap-1 font-mono text-xs",
                    for event in events() {
                        li { "{event}" }
                    }
                }
            }
        }
    }
}

#[component]
fn RecorderPanel(
    mut events: Signal<Vec<String>>,
    recorder_number: u64,
    mut supplied_source: Signal<Option<ApplicationRecordingSource>>,
) -> Element {
    let devices = use_audio_input_devices();
    let mut next_sample_rate = use_signal(|| 48_000_u32);
    let mut require_impossible_rate = use_signal(|| false);
    let mut use_alternate_chunk_callback = use_signal(|| false);
    let mut mime_supported = use_signal(|| None::<bool>);
    let mut recording_ids = use_signal(Vec::<RecordingId>::new);
    let mut supplied_source_error = use_signal(|| None::<String>);
    let mut preparing_supplied_source = use_signal(|| false);
    let mut supplied_source_generation = use_signal(|| 0_u64);
    let mut stop_supplied_track = use_signal(|| false);
    let sample_rate = if require_impossible_rate() {
        RecordingConstraint::Exact(1)
    } else {
        RecordingConstraint::Ideal(next_sample_rate())
    };
    let mut options = RecorderOptions::default();
    options.constraints = RecordingConstraints {
        channel_count: Some(RecordingConstraint::Ideal(1)),
        sample_rate: Some(sample_rate),
        echo_cancellation: Some(RecordingConstraint::Ideal(false)),
        noise_suppression: Some(RecordingConstraint::Ideal(false)),
        latency: Some(RecordingConstraint::Ideal(
            std::time::Duration::from_millis(20),
        )),
    };
    options.mime_types.clear();
    let chunk_callback_name = if use_alternate_chunk_callback() {
        "Alternate"
    } else {
        "Primary"
    };
    options.chunk_delivery = Some(RecordingChunkDelivery::new(
        std::time::Duration::from_millis(100),
        move |chunk| {
            record_chunk_event(
                chunk_callback_name,
                chunk,
                recorder_number,
                &mut events,
                &mut recording_ids,
            );
        },
    ));
    let recorder = use_audio_recorder(options, devices.selected().into());
    let custom_recorder = use_audio_recorder(RecorderOptions::default(), devices.selected().into());
    let mut completed = use_signal(|| None::<RecordedAudio>);
    let mut source = use_signal(|| None::<PlaybackSource>);

    use_effect(move || {
        if recorder.completed().read().is_some()
            && let Some(recording) = recorder.take_completed()
        {
            let recording_label =
                recording_label(recorder_number, &mut recording_ids, recording.recording_id);
            events.write().push(format!(
                "Completed | {recording_label} | bytes {}",
                recording.audio.bytes.len(),
            ));
            source.set(Some(recording.audio.clone().into()));
            completed.set(Some(recording));
        }
    });
    use_effect(move || {
        if let Some(outcome) = recorder.outcome()() {
            match outcome {
                RecordingOutcome::Discarded(recording_id) => {
                    let recording_label =
                        recording_label(recorder_number, &mut recording_ids, recording_id);
                    events
                        .write()
                        .push(format!("Discarded | {recording_label}"));
                }
                RecordingOutcome::Failed {
                    recording_id,
                    error,
                } => {
                    let recording_label =
                        recording_label(recorder_number, &mut recording_ids, recording_id);
                    events
                        .write()
                        .push(format!("Failed | {recording_label} | {error}"));
                }
                RecordingOutcome::Completed { .. } => {}
                _ => {}
            }
        }
    });
    use_effect(move || {
        if let Some(failure) = recorder.chunk_delivery_failure()() {
            let recording_label =
                recording_label(recorder_number, &mut recording_ids, failure.recording_id());
            events.write().push(format!(
                "Chunk delivery failed | {recording_label} | sequence {} | {}",
                failure.failed_sequence(),
                failure.error(),
            ));
        }
    });
    use_effect(move || {
        mime_supported.set(Some(is_recorder_mime_type_supported(
            "audio/webm;codecs=opus",
        )));
    });

    let status = recorder.status()();
    let source_availability = recorder
        .source_availability()()
        .map(|availability| format!("{availability:?}"))
        .unwrap_or_else(|| "Unavailable".to_string());
    let completion_cause = match recorder.outcome()() {
        Some(RecordingOutcome::Completed { cause, .. }) => format!("{cause:?}"),
        _ => "Unavailable".to_string(),
    };
    let future_source_shutdown = if stop_supplied_track() {
        "StopAudioTracks"
    } else {
        "PreserveTracks"
    };
    let custom_status = custom_recorder.status()();
    let active = recorder_active(&status) || recorder_active(&custom_status);
    let permission = devices.permission()();
    let recording = completed.read().clone();
    let requested_constraints = recorder.requested_constraints()();
    let capabilities = recorder.constraint_capabilities()();
    let settings = recorder.settings()();
    let selected_media_type = recorder.media_type()();
    let mime_support = mime_supported().map(|supported| {
        if supported {
            "supported"
        } else {
            "unsupported"
        }
    });
    let elapsed = format_duration(recorder.elapsed()().as_secs_f64());
    let microphone_status = recorder.microphone()();
    let recorder_permission = format!("{:?}", microphone_status.permission);
    let recorder_input_identity = microphone_status
        .input_device
        .as_ref()
        .map(|input| input.as_str().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let rejected_constraint = match &status {
        RecorderStatus::Failed(error) => error.overconstrained_constraint().map(str::to_string),
        _ => None,
    };
    let custom_labels = RecorderAnnouncementLabels {
        idle: "Custom recorder idle".to_string(),
        preparing: "Custom recorder preparing".to_string(),
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
                    p {
                        class: "mt-1 font-mono text-2xl tabular-nums",
                        role: "timer",
                        aria_label: "Recording elapsed time",
                        "{elapsed}"
                    }
                }
                StatusChip { label: format!("{status:?}") }
            }

            AudioInputSelector { devices, disabled: active }

            div {
                class: "grid gap-3 rounded-2xl border border-base-300 bg-base-100 p-4 text-sm",
                aria_label: "Supplied Recording Source",
                div { class: "flex flex-wrap gap-2",
                    button {
                        class: "btn btn-sm btn-outline",
                        r#type: "button",
                        disabled: active || preparing_supplied_source(),
                        onclick: move |_| {
                            if preparing_supplied_source() {
                                return;
                            }
                            preparing_supplied_source.set(true);
                            supplied_source_error.set(None);
                            let shutdown = if stop_supplied_track() {
                                RecordingSourceShutdown::StopAudioTracks
                            } else {
                                RecordingSourceShutdown::PreserveTracks
                            };
                            spawn(async move {
                                match acquire_supplied_source(shutdown).await {
                                    Ok(source) => {
                                        supplied_source.write().replace(source);
                                        supplied_source_generation += 1;
                                    }
                                    Err(error) => supplied_source_error.set(Some(error)),
                                }
                                preparing_supplied_source.set(false);
                            });
                        },
                        "Prepare supplied Recording Source"
                    }
                    button {
                        class: "btn btn-sm btn-outline",
                        r#type: "button",
                        disabled: active || preparing_supplied_source() || supplied_source.read().is_none(),
                        onclick: move |_| {
                            supplied_source_error.set(None);
                            if let Some(source) = supplied_source.peek().as_ref()
                                && let Err(error) = recorder.start_with_source(source.recorder_source.clone())
                            {
                                supplied_source_error.set(Some(error.to_string()));
                            }
                        },
                        "Start supplied recording"
                    }
                }
                label { class: "flex cursor-pointer items-center gap-3",
                    input {
                        class: "toggle toggle-primary toggle-sm",
                        r#type: "checkbox",
                        checked: stop_supplied_track(),
                        disabled: active || preparing_supplied_source(),
                        onchange: move |_| stop_supplied_track.toggle(),
                    }
                    "Stop supplied audio track on Recorder cleanup"
                }
                p { "Future supplied source shutdown: {future_source_shutdown}" }
                p {
                    "data-generation": supplied_source_generation(),
                    if preparing_supplied_source() {
                        "Preparing supplied Recording Source"
                    } else if supplied_source.read().is_some() {
                        "Supplied Recording Source ready"
                    } else {
                        "No supplied Recording Source"
                    }
                }
                p { "Recording Source availability: {source_availability}" }
                p { "Recording completion cause: {completion_cause}" }
                p { "Recorder microphone permission: {recorder_permission}" }
                p { "Recorder input identity: {recorder_input_identity}" }
                if let Some(error) = supplied_source_error() {
                    p { role: "alert", "{error}" }
                }
            }

            div {
                class: "grid gap-3 rounded-2xl border border-base-300 bg-base-100 p-4 text-sm",
                aria_label: "Recorder configuration",
                div { class: "flex flex-wrap gap-2",
                    button {
                        class: "btn btn-sm btn-outline",
                        r#type: "button",
                        onclick: move |_| next_sample_rate.set(44_100),
                        "Use 44100 Hz for future recordings"
                    }
                    button {
                        class: "btn btn-sm btn-outline",
                        r#type: "button",
                        onclick: move |_| use_alternate_chunk_callback.set(true),
                        "Use alternate chunk callback for future recordings"
                    }
                    button {
                        class: "btn btn-sm btn-outline",
                        r#type: "button",
                        disabled: active,
                        onclick: move |_| require_impossible_rate.set(true),
                        "Require impossible sample rate"
                    }
                }
                p {
                    if let Some(constraints) = requested_constraints {
                        if let Some(sample_rate) = constraints.sample_rate {
                            "Requested sample rate: {format_sample_rate_constraint(&sample_rate)}"
                        } else {
                            "Requested sample rate: none"
                        }
                    } else {
                        "Requested sample rate: not started"
                    }
                }
                if let Some(capabilities) = capabilities {
                    p { "Sample rate: {recognition(capabilities.sample_rate)}" }
                    p { "Noise suppression: {recognition(capabilities.noise_suppression)}" }
                } else {
                    p { "Recorder capabilities: unknown" }
                }
                p {
                    if let Some(sample_rate) = settings.and_then(|settings| settings.sample_rate) {
                        "Effective sample rate: {sample_rate} Hz"
                    } else {
                        "Effective sample rate: unknown"
                    }
                }
                p {
                    if let Some(media_type) = selected_media_type {
                        "Selected media type: {media_type}"
                    } else {
                        "Selected media type: unknown"
                    }
                }
                if let Some(mime_support) = mime_support {
                    p { "Opus WebM MIME probe: {mime_support}" }
                }
                if let Some(constraint) = rejected_constraint {
                    p { role: "alert", "Rejected exact constraint: {constraint}" }
                }
            }

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
            button {
                class: "btn btn-sm btn-outline justify-self-center",
                r#type: "button",
                disabled: !matches!(status, RecorderStatus::Recording | RecorderStatus::Paused),
                onclick: move |_| {
                    let _ = recorder.request_chunk_boundary();
                },
                "Request chunk boundary"
            }
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
                        preparing_label: "Abort custom recording preparation".to_string(),
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
                            p { class: "text-sm text-base-content/60",
                                if recording.input_device.is_some() {
                                    "Recorded input identity: known"
                                } else {
                                    "Recorded input identity: unknown"
                                }
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

fn record_chunk_event(
    callback_name: &str,
    chunk: RecordingChunk,
    recorder_number: u64,
    events: &mut Signal<Vec<String>>,
    recording_ids: &mut Signal<Vec<RecordingId>>,
) {
    let recording_label = recording_label(recorder_number, recording_ids, chunk.recording_id);
    events.write().push(format!(
        "{callback_name} chunk | {recording_label} | sequence {} | bytes {} | {}",
        chunk.sequence,
        chunk.bytes.len(),
        chunk.media_type
    ));
}

fn recording_label(
    recorder_number: u64,
    recording_ids: &mut Signal<Vec<RecordingId>>,
    recording_id: RecordingId,
) -> String {
    let existing = recording_ids
        .peek()
        .iter()
        .position(|candidate| *candidate == recording_id);
    let index = existing.unwrap_or_else(|| {
        let mut recording_ids = recording_ids.write();
        recording_ids.push(recording_id);
        recording_ids.len() - 1
    });
    format!("Recorder {recorder_number} Recording {}", index + 1)
}

fn format_duration(seconds: f64) -> String {
    let total_seconds = seconds.max(0.0) as u64;
    format!("{}:{:02}", total_seconds / 60, total_seconds % 60)
}

fn recorder_active(status: &RecorderStatus) -> bool {
    matches!(
        status,
        RecorderStatus::Preparing
            | RecorderStatus::Recording
            | RecorderStatus::Paused
            | RecorderStatus::Stopping
    )
}

fn format_sample_rate_constraint(constraint: &RecordingConstraint<u32>) -> String {
    match constraint {
        RecordingConstraint::Ideal(value) => format!("ideal {value} Hz"),
        RecordingConstraint::Exact(value) => format!("exact {value} Hz"),
    }
}

fn recognition(recognized: bool) -> &'static str {
    if recognized {
        "recognized"
    } else {
        "unrecognized"
    }
}

async fn acquire_supplied_source(
    shutdown: RecordingSourceShutdown,
) -> Result<ApplicationRecordingSource, String> {
    let media_devices = web_sys::window()
        .ok_or_else(|| "browser window is unavailable".to_string())?
        .navigator()
        .media_devices()
        .map_err(|_| "media devices are unavailable".to_string())?;
    let constraints = web_sys::MediaStreamConstraints::new();
    constraints.set_audio(&wasm_bindgen::JsValue::TRUE);
    let value = wasm_bindgen_futures::JsFuture::from(
        media_devices
            .get_user_media_with_constraints(&constraints)
            .map_err(|_| "could not request a Recording Source".to_string())?,
    )
    .await
    .map_err(|_| "could not acquire a Recording Source".to_string())?;
    let stream = value
        .dyn_into::<web_sys::MediaStream>()
        .map_err(|_| "browser returned an invalid Recording Source".to_string())?;
    Ok(ApplicationRecordingSource {
        recorder_source: RecordingSource::from_media_stream(&stream).with_shutdown(shutdown),
        stream,
    })
}

fn stop_application_source(source: &ApplicationRecordingSource) {
    for track in source.stream.get_audio_tracks() {
        if let Ok(track) = track.dyn_into::<web_sys::MediaStreamTrack>() {
            track.stop();
        }
    }
}
