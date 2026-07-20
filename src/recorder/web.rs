use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Duration;

use dioxus::core::{Runtime as DioxusRuntime, ScopeId};
use dioxus::prelude::*;
use js_sys::{Array, Reflect, Uint8Array};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{
    AudioContext, Blob, BlobEvent, BlobPropertyBag, MediaRecorder, MediaRecorderOptions,
    MediaStream, MediaStreamAudioSourceNode, MediaStreamTrack,
};

use super::*;
use crate::analysis::{AudioAnalyser, AudioAnalyserControl, peak_amplitude};
use crate::devices::web::{audio_error_from_js, media_devices, stop_stream};
use crate::{AudioData, AudioErrorKind};

pub(super) fn use_web_audio_recorder(
    options: RecorderOptions,
    selected_input: ReadSignal<Option<AudioInputId>>,
) -> AudioRecorder {
    let mut status = use_signal(|| RecorderStatus::Idle);
    let mut completed = use_signal(|| None::<RecordedAudio>);
    let mut analyser = use_signal(|| None::<AudioAnalyser>);
    let mut elapsed = use_signal(|| Duration::ZERO);
    let mut requested_constraints = use_signal(|| None::<RecordingConstraints>);
    let mut constraint_capabilities = use_signal(|| None::<RecorderConstraintCapabilities>);
    let mut settings = use_signal(|| None::<RecordingSourceSettings>);
    let mut media_type = use_signal(|| None::<String>);
    let mut microphone = use_signal(|| MicrophoneStatus {
        permission: MicrophonePermission::Unknown,
        recorder: RecorderStatus::Idle,
        input_device: None,
        muted: false,
    });
    let runtime = use_hook(|| Rc::new(RefCell::new(Runtime::default())));
    let dioxus_runtime = DioxusRuntime::current();
    let dioxus_scope = dioxus_runtime.current_scope_id();

    use_effect(move || {
        constraint_capabilities.set(read_constraint_capabilities());
    });

    {
        let runtime = Rc::downgrade(&runtime);
        use_hook(|| Rc::new(UnmountGuard(runtime)));
    }

    let runtime_for_start = runtime.clone();
    let start: Callback<(), Result<(), RecorderCommandError>> = use_callback(move |()| {
        if let Err(error) = options.validate() {
            let accepted = runtime_for_start
                .borrow_mut()
                .lifecycle
                .configuration_failed(error.clone());
            if accepted {
                status.set(RecorderStatus::Failed(error.clone()));
                microphone.set(MicrophoneStatus {
                    permission: MicrophonePermission::Unknown,
                    recorder: RecorderStatus::Failed(error),
                    input_device: selected_input(),
                    muted: false,
                });
            }
            return Err(command_error("invalid recorder options"));
        }

        let session = runtime_for_start.borrow_mut().lifecycle.start()?;
        let input_device = selected_input();
        requested_constraints.set(Some(options.constraints.clone()));
        settings.set(None);
        media_type.set(None);
        {
            let mut runtime = runtime_for_start.borrow_mut();
            runtime.elapsed_ms = 0.0;
            runtime.segment_started_at = None;
            runtime.last_peak_at = 0.0;
            runtime.peaks.clear();
            runtime.selected_device = input_device.clone();
            runtime.muted = false;
            runtime.terminal_error = None;
        }
        analyser.set(None);
        elapsed.set(Duration::ZERO);
        publish_status(
            &runtime_for_start,
            &mut status,
            &mut microphone,
            MicrophonePermission::Prompt,
        );

        let runtime = runtime_for_start.clone();
        let options = options.clone();
        let dioxus_runtime = dioxus_runtime.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let result = start_session(
                session,
                input_device,
                options.clone(),
                &runtime,
                status,
                completed,
                analyser,
                elapsed,
                microphone,
                settings,
                media_type,
                dioxus_runtime.clone(),
                dioxus_scope,
            )
            .await;
            if !runtime.borrow().mounted {
                return;
            }
            dioxus_runtime.in_scope(dioxus_scope, || match result {
                Ok(())
                    if matches!(
                        runtime.borrow().lifecycle.status(),
                        RecorderStatus::Recording
                    ) =>
                {
                    let runtime_for_timer = runtime.clone();
                    spawn(async move {
                        run_timer(session, options.peak_interval, runtime_for_timer, elapsed).await;
                    });
                }
                Ok(()) => {}
                Err(error) => fail_start(
                    session,
                    error,
                    &runtime,
                    &mut status,
                    &mut analyser,
                    &mut microphone,
                ),
            });
        });
        Ok(())
    });

    let runtime_for_pause = runtime.clone();
    let pause: Callback<(), Result<(), RecorderCommandError>> = use_callback(move |()| {
        let recorder = runtime_for_pause
            .borrow()
            .session
            .as_ref()
            .map(|session| session.recorder.clone())
            .ok_or_else(|| command_error("no active recorder"))?;
        recorder
            .pause()
            .map_err(|_| command_error("browser rejected pause"))?;
        let mut runtime = runtime_for_pause.borrow_mut();
        runtime.lifecycle.pause()?;
        runtime.accumulate_elapsed();
        elapsed.set(duration_from_ms(runtime.elapsed_ms));
        drop(runtime);
        publish_status(
            &runtime_for_pause,
            &mut status,
            &mut microphone,
            MicrophonePermission::Granted,
        );
        Ok(())
    });

    let runtime_for_resume = runtime.clone();
    let resume: Callback<(), Result<(), RecorderCommandError>> = use_callback(move |()| {
        let recorder = runtime_for_resume
            .borrow()
            .session
            .as_ref()
            .map(|session| session.recorder.clone())
            .ok_or_else(|| command_error("no active recorder"))?;
        recorder
            .resume()
            .map_err(|_| command_error("browser rejected resume"))?;
        let mut runtime = runtime_for_resume.borrow_mut();
        runtime.lifecycle.resume()?;
        runtime.segment_started_at = Some(now_ms());
        drop(runtime);
        publish_status(
            &runtime_for_resume,
            &mut status,
            &mut microphone,
            MicrophonePermission::Granted,
        );
        Ok(())
    });

    let runtime_for_stop = runtime.clone();
    let stop: Callback<(), Result<(), RecorderCommandError>> = use_callback(move |()| {
        stop_or_cancel(
            false,
            &runtime_for_stop,
            &mut status,
            &mut analyser,
            &mut elapsed,
            &mut microphone,
        )
    });

    let runtime_for_cancel = runtime.clone();
    let cancel: Callback<(), Result<(), RecorderCommandError>> = use_callback(move |()| {
        stop_or_cancel(
            true,
            &runtime_for_cancel,
            &mut status,
            &mut analyser,
            &mut elapsed,
            &mut microphone,
        )
    });

    let runtime_for_clear = runtime.clone();
    let clear_completed = use_callback(move |()| {
        completed.set(None);
        runtime_for_clear.borrow_mut().lifecycle.clear_completed();
    });
    let runtime_for_take = runtime.clone();
    let take_completed = use_callback(move |()| {
        let value = completed.write().take();
        if value.is_some() {
            runtime_for_take.borrow_mut().lifecycle.clear_completed();
        }
        value
    });

    AudioRecorder {
        status: status.into(),
        completed: completed.into(),
        analyser: analyser.into(),
        elapsed: elapsed.into(),
        microphone: microphone.into(),
        requested_constraints: requested_constraints.into(),
        constraint_capabilities: constraint_capabilities.into(),
        settings: settings.into(),
        media_type: media_type.into(),
        start,
        pause,
        resume,
        stop,
        cancel,
        take_completed,
        clear_completed,
    }
}

struct Runtime {
    lifecycle: RecorderLifecycle,
    session: Option<WebSession>,
    elapsed_ms: f64,
    segment_started_at: Option<f64>,
    last_peak_at: f64,
    peaks: Vec<u8>,
    selected_device: Option<AudioInputId>,
    muted: bool,
    terminal_error: Option<AudioError>,
    mounted: bool,
}

impl Default for Runtime {
    fn default() -> Self {
        Self {
            lifecycle: RecorderLifecycle::default(),
            session: None,
            elapsed_ms: 0.0,
            segment_started_at: None,
            last_peak_at: 0.0,
            peaks: Vec::new(),
            selected_device: None,
            muted: false,
            terminal_error: None,
            mounted: true,
        }
    }
}

impl Runtime {
    fn accumulate_elapsed(&mut self) {
        if let Some(started_at) = self.segment_started_at.take() {
            self.elapsed_ms += (now_ms() - started_at).max(0.0);
        }
    }
}

struct UnmountGuard(Weak<RefCell<Runtime>>);

impl Drop for UnmountGuard {
    fn drop(&mut self) {
        if let Some(runtime) = self.0.upgrade() {
            let mut runtime = runtime.borrow_mut();
            runtime.mounted = false;
            runtime.session.take();
            runtime.lifecycle.active_session = None;
        }
    }
}

struct PendingCapture {
    stream: Option<MediaStream>,
    context: Option<AudioContext>,
}

impl PendingCapture {
    fn new(stream: MediaStream) -> Self {
        Self {
            stream: Some(stream),
            context: None,
        }
    }

    fn into_parts(mut self) -> (MediaStream, AudioContext) {
        (
            self.stream.take().expect("pending capture owns its stream"),
            self.context
                .take()
                .expect("pending capture owns its context"),
        )
    }
}

impl Drop for PendingCapture {
    fn drop(&mut self) {
        if let Some(stream) = self.stream.take() {
            stop_stream(&stream);
        }
        if let Some(context) = self.context.take() {
            settle_audio_promise(context.close());
        }
    }
}

struct WebSession {
    recorder: MediaRecorder,
    stream: MediaStream,
    context: AudioContext,
    _source: MediaStreamAudioSourceNode,
    analyser: AudioAnalyserControl,
    chunks: Rc<RefCell<Vec<Blob>>>,
    _on_data: Closure<dyn FnMut(BlobEvent)>,
    _on_stop: Closure<dyn FnMut()>,
    _on_error: Closure<dyn FnMut()>,
    track: MediaStreamTrack,
    on_mute: Closure<dyn FnMut()>,
    on_unmute: Closure<dyn FnMut()>,
    on_ended: Closure<dyn FnMut()>,
}

impl Drop for WebSession {
    fn drop(&mut self) {
        self.analyser.set_available(false);
        self.recorder.set_ondataavailable(None);
        self.recorder.set_onstop(None);
        self.recorder.set_onerror(None);
        let _ = self
            .track
            .remove_event_listener_with_callback("mute", self.on_mute.as_ref().unchecked_ref());
        let _ = self
            .track
            .remove_event_listener_with_callback("unmute", self.on_unmute.as_ref().unchecked_ref());
        let _ = self
            .track
            .remove_event_listener_with_callback("ended", self.on_ended.as_ref().unchecked_ref());
        stop_stream(&self.stream);
        settle_audio_promise(self.context.close());
    }
}

#[allow(clippy::too_many_arguments)]
async fn start_session(
    session_id: RecordingSessionId,
    input_device: Option<AudioInputId>,
    options: RecorderOptions,
    runtime: &Rc<RefCell<Runtime>>,
    mut status: Signal<RecorderStatus>,
    mut completed: Signal<Option<RecordedAudio>>,
    mut analyser_signal: Signal<Option<AudioAnalyser>>,
    mut elapsed: Signal<Duration>,
    mut microphone: Signal<MicrophoneStatus>,
    mut settings_signal: Signal<Option<RecordingSourceSettings>>,
    mut media_type_signal: Signal<Option<String>>,
    dioxus_runtime: Rc<DioxusRuntime>,
    dioxus_scope: ScopeId,
) -> Result<(), AudioError> {
    let stream = acquire_stream(input_device.as_ref(), &options.constraints).await?;
    let mut pending = PendingCapture::new(stream);
    if !runtime.borrow().mounted || runtime.borrow().lifecycle.active_session != Some(session_id) {
        return Ok(());
    }

    let context = AudioContext::new().map_err(audio_error_from_js)?;
    pending.context = Some(context.clone());
    // MediaRecorder does not depend on the AudioContext. Request a running
    // analyser without blocking capture on browser autoplay policy.
    settle_audio_promise(context.resume());
    let analyser_node = context.create_analyser().map_err(audio_error_from_js)?;
    analyser_node.set_fft_size(options.fft_size);
    analyser_node.set_smoothing_time_constant(options.smoothing);
    let source = context
        .create_media_stream_source(
            pending
                .stream
                .as_ref()
                .expect("pending capture owns its stream"),
        )
        .map_err(audio_error_from_js)?;
    source
        .connect_with_audio_node(&analyser_node)
        .map_err(audio_error_from_js)?;

    let recorder_options = MediaRecorderOptions::new();
    if let Some(mime_type) = options
        .mime_types
        .iter()
        .find(|mime_type| MediaRecorder::is_type_supported(mime_type))
    {
        recorder_options.set_mime_type(mime_type);
    }
    if let Some(bits_per_second) = options.audio_bits_per_second {
        recorder_options.set_audio_bits_per_second(bits_per_second);
    }
    let recorder = MediaRecorder::new_with_media_stream_and_media_recorder_options(
        pending
            .stream
            .as_ref()
            .expect("pending capture owns its stream"),
        &recorder_options,
    )
    .map_err(audio_error_from_js)?;
    let track = pending
        .stream
        .as_ref()
        .expect("pending capture owns its stream")
        .get_audio_tracks()
        .get(0)
        .dyn_into::<MediaStreamTrack>()
        .map_err(|_| {
            AudioError::new(
                AudioErrorKind::DeviceNotFound,
                "microphone stream has no audio track",
            )
        })?;
    let settings = read_settings(&track);
    let chunks = Rc::new(RefCell::new(Vec::<Blob>::new()));
    let start_resolver = Rc::new(RefCell::new(None::<js_sys::Function>));
    let resolver_for_promise = start_resolver.clone();
    let start_promise = js_sys::Promise::new(&mut move |resolve, _| {
        resolver_for_promise.borrow_mut().replace(resolve);
    });
    let start_succeeded = Rc::new(Cell::new(false));
    let resolver_for_start = start_resolver.clone();
    let succeeded_for_start = start_succeeded.clone();
    let on_start = Closure::wrap(Box::new(move || {
        succeeded_for_start.set(true);
        settle_start(&resolver_for_start);
    }) as Box<dyn FnMut()>);
    recorder.set_onstart(Some(on_start.as_ref().unchecked_ref()));

    let chunks_for_data = chunks.clone();
    let on_data = Closure::wrap(Box::new(move |event: BlobEvent| {
        if let Some(blob) = event.data()
            && blob.size() > 0.0
        {
            chunks_for_data.borrow_mut().push(blob);
        }
    }) as Box<dyn FnMut(BlobEvent)>);
    recorder.set_ondataavailable(Some(on_data.as_ref().unchecked_ref()));

    let runtime_for_stop = Rc::downgrade(runtime);
    let recorder_for_stop = recorder.clone();
    let dioxus_runtime_for_stop = dioxus_runtime.clone();
    let resolver_for_stop = start_resolver.clone();
    let on_stop = Closure::wrap(Box::new(move || {
        settle_start(&resolver_for_stop);
        dioxus_runtime_for_stop.in_scope(dioxus_scope, || {
            let Some(runtime) = runtime_for_stop.upgrade() else {
                return;
            };
            let (disposition, terminal_error, chunks, duration, peaks, selected_device, mime_type) = {
                let mut runtime = runtime.borrow_mut();
                if matches!(
                    runtime.lifecycle.status(),
                    RecorderStatus::Recording | RecorderStatus::Paused
                ) {
                    runtime.accumulate_elapsed();
                }
                let disposition = runtime.lifecycle.begin_finalize(session_id);
                (
                    disposition,
                    runtime.terminal_error.take(),
                    runtime
                        .session
                        .as_ref()
                        .map(|session| session.chunks.borrow().clone())
                        .unwrap_or_default(),
                    duration_from_ms(runtime.elapsed_ms),
                    runtime.peaks.clone(),
                    runtime.selected_device.clone(),
                    recorder_for_stop.mime_type(),
                )
            };
            let Some(disposition) = disposition else {
                return;
            };
            analyser_signal.set(None);
            publish_status(
                &runtime,
                &mut status,
                &mut microphone,
                MicrophonePermission::Granted,
            );

            spawn(async move {
                gloo_timers::future::TimeoutFuture::new(0).await;
                runtime.borrow_mut().session.take();

                if let Some(error) = terminal_error {
                    let mut runtime = runtime.borrow_mut();
                    runtime.lifecycle.failed(session_id, error.clone());
                    runtime.muted = false;
                    drop(runtime);
                    status.set(RecorderStatus::Failed(error.clone()));
                    microphone.set(MicrophoneStatus {
                        permission: MicrophonePermission::Granted,
                        recorder: RecorderStatus::Failed(error),
                        input_device: selected_device,
                        muted: false,
                    });
                    return;
                }

                if disposition == CompletionDisposition::Discard {
                    let mut runtime = runtime.borrow_mut();
                    runtime.lifecycle.complete_finalize(session_id);
                    runtime.muted = false;
                    drop(runtime);
                    status.set(RecorderStatus::Idle);
                    microphone.set(MicrophoneStatus {
                        permission: MicrophonePermission::Granted,
                        recorder: RecorderStatus::Idle,
                        input_device: selected_device,
                        muted: false,
                    });
                    return;
                }

                match collect_audio(chunks, mime_type).await {
                    Ok(audio) => {
                        let mut runtime = runtime.borrow_mut();
                        runtime.lifecycle.complete_finalize(session_id);
                        runtime.muted = false;
                        drop(runtime);
                        completed.set(Some(RecordedAudio {
                            audio,
                            duration,
                            peaks,
                            input_device: selected_device.clone(),
                        }));
                        status.set(RecorderStatus::Idle);
                        microphone.set(MicrophoneStatus {
                            permission: MicrophonePermission::Granted,
                            recorder: RecorderStatus::Idle,
                            input_device: selected_device,
                            muted: false,
                        });
                    }
                    Err(error) => {
                        let mut runtime = runtime.borrow_mut();
                        runtime.lifecycle.failed(session_id, error.clone());
                        runtime.muted = false;
                        drop(runtime);
                        status.set(RecorderStatus::Failed(error.clone()));
                        microphone.set(MicrophoneStatus {
                            permission: MicrophonePermission::Granted,
                            recorder: RecorderStatus::Failed(error),
                            input_device: selected_device,
                            muted: false,
                        });
                    }
                }
            });
        });
    }) as Box<dyn FnMut()>);
    recorder.set_onstop(Some(on_stop.as_ref().unchecked_ref()));

    let runtime_for_error = Rc::downgrade(runtime);
    let dioxus_runtime_for_error = dioxus_runtime.clone();
    let resolver_for_error = start_resolver.clone();
    let on_error = Closure::wrap(Box::new(move || {
        settle_start(&resolver_for_error);
        dioxus_runtime_for_error.in_scope(dioxus_scope, || {
            let Some(runtime) = runtime_for_error.upgrade() else {
                return;
            };
            let should_finalize = {
                let mut runtime = runtime.borrow_mut();
                let active = runtime.lifecycle.active_session == Some(session_id);
                let status = runtime.lifecycle.status().clone();
                if active
                    && (matches!(status, RecorderStatus::Recording | RecorderStatus::Paused)
                        || (matches!(status, RecorderStatus::Stopping)
                            && runtime.lifecycle.completion == CompletionDisposition::Save))
                {
                    runtime.terminal_error = Some(AudioError::new(
                        AudioErrorKind::RecorderFailure,
                        "media recorder failed",
                    ));
                    if matches!(status, RecorderStatus::Recording | RecorderStatus::Paused) {
                        runtime.accumulate_elapsed();
                        runtime.lifecycle.stop().is_ok()
                    } else {
                        false
                    }
                } else {
                    false
                }
            };
            if should_finalize {
                analyser_signal.set(None);
                publish_status(
                    &runtime,
                    &mut status,
                    &mut microphone,
                    MicrophonePermission::Granted,
                );
            }
        });
    }) as Box<dyn FnMut()>);
    recorder.set_onerror(Some(on_error.as_ref().unchecked_ref()));

    let runtime_for_mute = Rc::downgrade(runtime);
    let dioxus_runtime_for_mute = dioxus_runtime.clone();
    let on_mute = Closure::wrap(Box::new(move || {
        dioxus_runtime_for_mute.in_scope(dioxus_scope, || {
            if let Some(runtime) = runtime_for_mute.upgrade() {
                runtime.borrow_mut().muted = true;
                publish_status(
                    &runtime,
                    &mut status,
                    &mut microphone,
                    MicrophonePermission::Granted,
                );
            }
        });
    }) as Box<dyn FnMut()>);
    let _ = track.add_event_listener_with_callback("mute", on_mute.as_ref().unchecked_ref());

    let runtime_for_unmute = Rc::downgrade(runtime);
    let dioxus_runtime_for_unmute = dioxus_runtime.clone();
    let on_unmute = Closure::wrap(Box::new(move || {
        dioxus_runtime_for_unmute.in_scope(dioxus_scope, || {
            if let Some(runtime) = runtime_for_unmute.upgrade() {
                runtime.borrow_mut().muted = false;
                publish_status(
                    &runtime,
                    &mut status,
                    &mut microphone,
                    MicrophonePermission::Granted,
                );
            }
        });
    }) as Box<dyn FnMut()>);
    let _ = track.add_event_listener_with_callback("unmute", on_unmute.as_ref().unchecked_ref());

    let runtime_for_ended = Rc::downgrade(runtime);
    let dioxus_runtime_for_ended = dioxus_runtime.clone();
    let recorder_for_ended = recorder.clone();
    let on_ended = Closure::wrap(Box::new(move || {
        dioxus_runtime_for_ended.in_scope(dioxus_scope, || {
            if let Some(runtime) = runtime_for_ended.upgrade() {
                let should_stop = {
                    let mut runtime = runtime.borrow_mut();
                    runtime.muted = true;
                    if matches!(
                        runtime.lifecycle.status(),
                        RecorderStatus::Recording | RecorderStatus::Paused
                    ) {
                        runtime.accumulate_elapsed();
                        runtime.lifecycle.stop().is_ok()
                    } else {
                        false
                    }
                };
                publish_status(
                    &runtime,
                    &mut status,
                    &mut microphone,
                    MicrophonePermission::Granted,
                );
                if should_stop {
                    let _ = recorder_for_ended.stop();
                }
            }
        });
    }) as Box<dyn FnMut()>);
    let _ = track.add_event_listener_with_callback("ended", on_ended.as_ref().unchecked_ref());

    let initially_muted = track.muted();
    if let Err(error) = recorder.start() {
        recorder.set_onstart(None);
        return Err(audio_error_from_js(error));
    }
    let _ = wasm_bindgen_futures::JsFuture::from(start_promise).await;
    recorder.set_onstart(None);
    drop(on_start);
    if !start_succeeded.get() {
        return Err(AudioError::new(
            AudioErrorKind::RecorderFailure,
            "media recorder failed before starting",
        ));
    }
    let media_type = recorder.mime_type();
    let (stream, context) = pending.into_parts();
    let (analyser_control, analyser) =
        AudioAnalyserControl::new(analyser_node, context.sample_rate());
    analyser_control.set_available(true);
    let session = WebSession {
        recorder,
        stream,
        context,
        _source: source,
        analyser: analyser_control,
        chunks,
        _on_data: on_data,
        _on_stop: on_stop,
        _on_error: on_error,
        track,
        on_mute,
        on_unmute,
        on_ended,
    };

    let mut runtime_mut = runtime.borrow_mut();
    if !runtime_mut.lifecycle.started(session_id) || !runtime_mut.mounted {
        drop(runtime_mut);
        drop(session);
        return Ok(());
    }
    runtime_mut.segment_started_at = Some(now_ms());
    runtime_mut.last_peak_at = now_ms();
    runtime_mut.muted = initially_muted;
    runtime_mut.session = Some(session);
    drop(runtime_mut);

    dioxus_runtime.in_scope(dioxus_scope, || {
        analyser_signal.set(Some(analyser));
        elapsed.set(Duration::ZERO);
        settings_signal.set(Some(settings));
        media_type_signal.set(Some(media_type));
        publish_status(
            runtime,
            &mut status,
            &mut microphone,
            MicrophonePermission::Granted,
        );
    });
    Ok(())
}

async fn acquire_stream(
    input_device: Option<&AudioInputId>,
    requested: &RecordingConstraints,
) -> Result<MediaStream, AudioError> {
    let constraints = web_sys::MediaStreamConstraints::new();
    let audio = web_sys::MediaTrackConstraints::new();
    if let Some(input_device) = input_device {
        let exact = web_sys::ConstrainDomStringParameters::new();
        exact.set_exact_str(input_device.as_str());
        audio.set_device_id_constrain_dom_string_parameters(&exact);
    }
    set_constraint(
        &audio,
        "channelCount",
        requested.channel_count.as_ref(),
        |value| JsValue::from_f64(*value as f64),
    )?;
    set_constraint(
        &audio,
        "sampleRate",
        requested.sample_rate.as_ref(),
        |value| JsValue::from_f64(*value as f64),
    )?;
    set_constraint(
        &audio,
        "echoCancellation",
        requested.echo_cancellation.as_ref(),
        |value| JsValue::from_bool(*value),
    )?;
    set_constraint(
        &audio,
        "noiseSuppression",
        requested.noise_suppression.as_ref(),
        |value| JsValue::from_bool(*value),
    )?;
    set_constraint(&audio, "latency", requested.latency.as_ref(), |value| {
        JsValue::from_f64(value.as_secs_f64())
    })?;
    constraints.set_audio_media_track_constraints(&audio);
    let value = wasm_bindgen_futures::JsFuture::from(
        media_devices()?
            .get_user_media_with_constraints(&constraints)
            .map_err(audio_error_from_js)?,
    )
    .await
    .map_err(audio_error_from_js)?;
    value.dyn_into::<MediaStream>().map_err(audio_error_from_js)
}

fn read_constraint_capabilities() -> Option<RecorderConstraintCapabilities> {
    let supported = media_devices().ok()?.get_supported_constraints();
    Some(RecorderConstraintCapabilities {
        channel_count: supported.get_channel_count().unwrap_or(false),
        sample_rate: supported.get_sample_rate().unwrap_or(false),
        echo_cancellation: supported.get_echo_cancellation().unwrap_or(false),
        noise_suppression: supported.get_noise_suppression().unwrap_or(false),
        latency: supported.get_latency().unwrap_or(false),
    })
}

fn read_settings(track: &MediaStreamTrack) -> RecordingSourceSettings {
    let settings = track.get_settings();
    let settings = settings.as_ref();
    RecordingSourceSettings {
        channel_count: read_u32(settings, "channelCount"),
        sample_rate: read_u32(settings, "sampleRate"),
        echo_cancellation: read_bool(settings, "echoCancellation"),
        noise_suppression: read_bool(settings, "noiseSuppression"),
        latency: read_number(settings, "latency").and_then(|seconds| {
            if seconds.is_finite() && seconds >= 0.0 {
                Duration::try_from_secs_f64(seconds).ok()
            } else {
                None
            }
        }),
    }
}

fn read_u32(value: &JsValue, field: &str) -> Option<u32> {
    let number = read_number(value, field)?;
    if number.is_finite() && number >= 0.0 && number <= u32::MAX as f64 && number.fract() == 0.0 {
        Some(number as u32)
    } else {
        None
    }
}

fn read_number(value: &JsValue, field: &str) -> Option<f64> {
    Reflect::get(value, &JsValue::from_str(field))
        .ok()?
        .as_f64()
}

fn read_bool(value: &JsValue, field: &str) -> Option<bool> {
    Reflect::get(value, &JsValue::from_str(field))
        .ok()?
        .as_bool()
}

fn set_constraint<T>(
    target: &web_sys::MediaTrackConstraints,
    field: &str,
    constraint: Option<&RecordingConstraint<T>>,
    into_js: impl FnOnce(&T) -> JsValue,
) -> Result<(), AudioError> {
    if let Some(constraint) = constraint {
        let (kind, value) = match constraint {
            RecordingConstraint::Ideal(value) => ("ideal", value),
            RecordingConstraint::Exact(value) => ("exact", value),
        };
        set_js_constraint(target, field, kind, into_js(value))?;
    }
    Ok(())
}

fn set_js_constraint(
    target: &web_sys::MediaTrackConstraints,
    field: &str,
    kind: &str,
    value: JsValue,
) -> Result<(), AudioError> {
    let constraint = js_sys::Object::new();
    Reflect::set(&constraint, &JsValue::from_str(kind), &value).map_err(audio_error_from_js)?;
    Reflect::set(
        target.as_ref(),
        &JsValue::from_str(field),
        constraint.as_ref(),
    )
    .map_err(audio_error_from_js)?;
    Ok(())
}

fn settle_start(resolver: &Rc<RefCell<Option<js_sys::Function>>>) {
    if let Some(resolve) = resolver.borrow_mut().take() {
        let _ = resolve.call0(&JsValue::UNDEFINED);
    }
}

async fn run_timer(
    session_id: RecordingSessionId,
    peak_interval: Duration,
    runtime: Rc<RefCell<Runtime>>,
    mut elapsed: Signal<Duration>,
) {
    loop {
        gloo_timers::future::TimeoutFuture::new(30).await;
        let mut runtime = runtime.borrow_mut();
        if runtime.lifecycle.active_session != Some(session_id) {
            break;
        }
        if !matches!(runtime.lifecycle.status(), RecorderStatus::Recording) {
            continue;
        }

        let now = now_ms();
        let current_ms = runtime.elapsed_ms
            + runtime
                .segment_started_at
                .map(|start| (now - start).max(0.0))
                .unwrap_or(0.0);
        elapsed.set(duration_from_ms(current_ms));

        if now - runtime.last_peak_at >= peak_interval.as_secs_f64() * 1000.0 {
            runtime.last_peak_at = now;
            if let Some(session) = runtime.session.as_ref() {
                let mut samples = vec![0_u8; session.analyser.node().fft_size() as usize];
                session
                    .analyser
                    .node()
                    .get_byte_time_domain_data(&mut samples);
                let peak = peak_amplitude(&samples);
                runtime.peaks.push(peak);
            }
        }
    }
}

fn stop_or_cancel(
    cancel: bool,
    runtime: &Rc<RefCell<Runtime>>,
    status: &mut Signal<RecorderStatus>,
    analyser: &mut Signal<Option<AudioAnalyser>>,
    elapsed: &mut Signal<Duration>,
    microphone: &mut Signal<MicrophoneStatus>,
) -> Result<(), RecorderCommandError> {
    let mut runtime_mut = runtime.borrow_mut();
    if cancel {
        runtime_mut.lifecycle.cancel()?;
    } else {
        runtime_mut.lifecycle.stop()?;
    }

    if matches!(runtime_mut.lifecycle.status(), RecorderStatus::Idle) {
        status.set(RecorderStatus::Idle);
        microphone.set(MicrophoneStatus {
            permission: MicrophonePermission::Unknown,
            recorder: RecorderStatus::Idle,
            input_device: runtime_mut.selected_device.clone(),
            muted: false,
        });
        return Ok(());
    }

    runtime_mut.accumulate_elapsed();
    elapsed.set(duration_from_ms(runtime_mut.elapsed_ms));
    let recorder = runtime_mut
        .session
        .as_ref()
        .map(|session| session.recorder.clone())
        .ok_or_else(|| command_error("no active recorder"))?;
    drop(runtime_mut);
    publish_status(runtime, status, microphone, MicrophonePermission::Granted);
    if recorder.stop().is_err() {
        let error = AudioError::new(
            AudioErrorKind::RecorderFailure,
            "browser rejected recording stop",
        );
        let mut runtime = runtime.borrow_mut();
        if let Some(session) = runtime.lifecycle.active_session {
            runtime.lifecycle.failed(session, error.clone());
        }
        runtime.session.take();
        let selected_device = runtime.selected_device.clone();
        drop(runtime);
        analyser.set(None);
        status.set(RecorderStatus::Failed(error.clone()));
        microphone.set(MicrophoneStatus {
            permission: MicrophonePermission::Granted,
            recorder: RecorderStatus::Failed(error),
            input_device: selected_device,
            muted: false,
        });
        return Err(command_error("browser rejected stop"));
    }
    Ok(())
}

fn fail_start(
    session: RecordingSessionId,
    error: AudioError,
    runtime: &Rc<RefCell<Runtime>>,
    status: &mut Signal<RecorderStatus>,
    analyser: &mut Signal<Option<AudioAnalyser>>,
    microphone: &mut Signal<MicrophoneStatus>,
) {
    if runtime
        .borrow_mut()
        .lifecycle
        .failed(session, error.clone())
    {
        runtime.borrow_mut().session.take();
        analyser.set(None);
        status.set(RecorderStatus::Failed(error.clone()));
        microphone.set(MicrophoneStatus {
            permission: if error.kind() == AudioErrorKind::PermissionDenied {
                MicrophonePermission::Denied
            } else {
                MicrophonePermission::Unknown
            },
            recorder: RecorderStatus::Failed(error),
            input_device: runtime.borrow().selected_device.clone(),
            muted: false,
        });
    }
}

fn publish_status(
    runtime: &Rc<RefCell<Runtime>>,
    status: &mut Signal<RecorderStatus>,
    microphone: &mut Signal<MicrophoneStatus>,
    permission: MicrophonePermission,
) {
    let runtime = runtime.borrow();
    let recorder = runtime.lifecycle.status().clone();
    status.set(recorder.clone());
    microphone.set(MicrophoneStatus {
        permission,
        recorder,
        input_device: runtime.selected_device.clone(),
        muted: runtime.muted,
    });
}

async fn collect_audio(chunks: Vec<Blob>, mime_type: String) -> Result<AudioData, AudioError> {
    if chunks.is_empty() {
        return Err(AudioError::new(
            AudioErrorKind::RecorderFailure,
            "recording produced no audio data",
        ));
    }
    let parts = Array::new();
    for chunk in chunks {
        parts.push(&chunk);
    }
    let properties = BlobPropertyBag::new();
    properties.set_type(&mime_type);
    let blob = Blob::new_with_blob_sequence_and_options(&parts, &properties)
        .map_err(audio_error_from_js)?;
    let buffer = wasm_bindgen_futures::JsFuture::from(blob.array_buffer())
        .await
        .map_err(audio_error_from_js)?;
    let bytes = Uint8Array::new(&buffer).to_vec();
    if bytes.is_empty() {
        return Err(AudioError::new(
            AudioErrorKind::RecorderFailure,
            "recording produced empty audio data",
        ));
    }
    Ok(AudioData::new(bytes, mime_type))
}

fn now_ms() -> f64 {
    web_sys::window()
        .and_then(|window| window.performance())
        .map(|performance| performance.now())
        .unwrap_or_else(js_sys::Date::now)
}

fn duration_from_ms(milliseconds: f64) -> Duration {
    Duration::from_secs_f64((milliseconds / 1000.0).max(0.0))
}

fn settle_audio_promise(promise: Result<js_sys::Promise, JsValue>) {
    if let Ok(promise) = promise {
        wasm_bindgen_futures::spawn_local(async move {
            let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
        });
    }
}
