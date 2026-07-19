use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::time::Duration;

use dioxus::core::{Runtime as DioxusRuntime, ScopeId};
use dioxus::prelude::*;
use js_sys::Uint8Array;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{HtmlAudioElement, Url};

use super::*;
use crate::{AudioError, AudioErrorKind};

const HAVE_FUTURE_DATA: u16 = 3;

pub(super) fn use_web_audio_player(
    source: ReadSignal<Option<AudioData>>,
    initial_duration: Duration,
) -> AudioPlayerController {
    let status = use_signal(|| PlaybackStatus::Empty);
    let snapshot = use_signal(PlaybackSnapshot::default);
    let mut position = use_signal(|| Duration::ZERO);
    let mut duration = use_signal(|| initial_duration);
    let mut rate = use_signal(|| 1.0_f64);
    let runtime = use_hook(|| Rc::new(RefCell::new(PlayerRuntime::default())));
    let dioxus_runtime = DioxusRuntime::current();
    let dioxus_scope = dioxus_runtime.current_scope_id();
    let source_input = use_memo(use_reactive!(|(source,)| source));
    let initial_duration = use_memo(use_reactive!(|(initial_duration,)| initial_duration));

    {
        let runtime = Rc::downgrade(&runtime);
        use_hook(|| Rc::new(UnmountGuard(runtime)));
    }

    {
        let runtime = runtime.clone();
        use_effect(move || {
            let mut runtime_mut = runtime.borrow_mut();
            runtime_mut.generation = runtime_mut.generation.wrapping_add(1);
            runtime_mut.play_generation = runtime_mut.play_generation.wrapping_add(1);
            runtime_mut.resource.take();
            let generation = runtime_mut.generation;
            drop(runtime_mut);
            position.set(Duration::ZERO);
            duration.set(*initial_duration.peek());
            let source = source_input();
            let source = source.read();
            let Some(source) = source.as_ref() else {
                runtime.borrow_mut().lifecycle.unload();
                publish_lifecycle(&runtime.borrow().lifecycle, status, snapshot);
                return;
            };
            runtime.borrow_mut().lifecycle.loading();
            publish_lifecycle(&runtime.borrow().lifecycle, status, snapshot);
            match WebPlayer::new(
                source,
                Rc::downgrade(&runtime),
                generation,
                status,
                snapshot,
                position,
                duration,
                dioxus_runtime.clone(),
                dioxus_scope,
            ) {
                Ok(resource) => {
                    resource.element.set_playback_rate(*rate.peek());
                    resource
                        .element
                        .set_loop(runtime.borrow().lifecycle.repeat());
                    runtime.borrow_mut().resource = Some(resource);
                }
                Err(error) => {
                    runtime.borrow_mut().lifecycle.failed(error.clone());
                    publish_lifecycle(&runtime.borrow().lifecycle, status, snapshot);
                }
            }
        });
    }

    {
        let runtime = runtime.clone();
        use_effect(move || {
            let fallback = initial_duration();
            let needs_fallback = runtime
                .borrow()
                .resource
                .as_ref()
                .map(|resource| {
                    let duration = resource.element.duration();
                    !duration.is_finite() || duration <= 0.0
                })
                .unwrap_or(true);
            if needs_fallback {
                duration.set(fallback);
            }
        });
    }

    let runtime_for_play = runtime.clone();
    let play: Callback<(), Result<(), PlaybackCommandError>> = use_callback(move |()| {
        let (element, generation, play_generation, restart_ended) = {
            let mut runtime = runtime_for_play.borrow_mut();
            let restart_ended = matches!(runtime.lifecycle.status(), PlaybackStatus::Ended);
            runtime.lifecycle.request_play()?;
            runtime.play_generation = runtime.play_generation.wrapping_add(1);
            (
                runtime
                    .resource
                    .as_ref()
                    .map(|resource| resource.element.clone())
                    .ok_or(PlaybackCommandError("audio is not loaded"))?,
                runtime.generation,
                runtime.play_generation,
                restart_ended,
            )
        };
        publish_lifecycle(&runtime_for_play.borrow().lifecycle, status, snapshot);
        if restart_ended {
            element.set_current_time(0.0);
            position.set(Duration::ZERO);
        }
        let promise = match element.play() {
            Ok(promise) => promise,
            Err(value) => {
                let failure = play_failure(value);
                runtime_for_play
                    .borrow_mut()
                    .lifecycle
                    .play_rejected(failure);
                publish_lifecycle(&runtime_for_play.borrow().lifecycle, status, snapshot);
                return Err(PlaybackCommandError("browser rejected playback"));
            }
        };
        let runtime = runtime_for_play.clone();
        spawn(async move {
            let outcome = wasm_bindgen_futures::JsFuture::from(promise).await;
            let runtime_ref = runtime.borrow();
            if !runtime_ref.owner_active
                || runtime_ref.generation != generation
                || runtime_ref.play_generation != play_generation
            {
                let should_pause = runtime_ref.owner_active
                    && runtime_ref.generation == generation
                    && matches!(
                        runtime_ref.lifecycle.transport(),
                        PlaybackTransport::Idle | PlaybackTransport::Paused
                    );
                drop(runtime_ref);
                if should_pause {
                    let _ = element.pause();
                }
                return;
            }
            drop(runtime_ref);
            match outcome {
                Ok(_) => runtime.borrow_mut().lifecycle.playing(),
                Err(value) => {
                    let failure = play_failure(value);
                    runtime.borrow_mut().lifecycle.play_rejected(failure);
                }
            }
            publish_lifecycle(&runtime.borrow().lifecycle, status, snapshot);
        });
        Ok(())
    });

    let runtime_for_stop = runtime.clone();
    let stop: Callback<(), Result<(), PlaybackCommandError>> = use_callback(move |()| {
        let element = {
            let runtime = runtime_for_stop.borrow();
            if runtime.lifecycle.source() != &PlaybackSourceLifecycle::Playable {
                return Err(PlaybackCommandError("audio is not loaded"));
            }
            runtime
                .resource
                .as_ref()
                .map(|resource| resource.element.clone())
                .ok_or(PlaybackCommandError("audio is not loaded"))?
        };
        element
            .pause()
            .map_err(|_| PlaybackCommandError("browser rejected stop"))?;
        {
            let mut runtime = runtime_for_stop.borrow_mut();
            runtime.play_generation = runtime.play_generation.wrapping_add(1);
            runtime.lifecycle.stop()?;
        }
        element.set_current_time(0.0);
        position.set(Duration::ZERO);
        publish_lifecycle(&runtime_for_stop.borrow().lifecycle, status, snapshot);
        Ok(())
    });

    let runtime_for_pause = runtime.clone();
    let pause: Callback<(), Result<(), PlaybackCommandError>> = use_callback(move |()| {
        let mut runtime = runtime_for_pause.borrow_mut();
        runtime.play_generation = runtime.play_generation.wrapping_add(1);
        let element = runtime
            .resource
            .as_ref()
            .map(|resource| resource.element.clone())
            .ok_or(PlaybackCommandError("audio is not loaded"))?;
        drop(runtime);
        element
            .pause()
            .map_err(|_| PlaybackCommandError("browser rejected pause"))?;
        runtime_for_pause.borrow_mut().lifecycle.paused();
        publish_lifecycle(&runtime_for_pause.borrow().lifecycle, status, snapshot);
        Ok(())
    });

    let runtime_for_seek = runtime.clone();
    let seek = use_callback(move |requested: Duration| {
        let duration_secs = duration().as_secs_f64();
        let seconds = clamp_seek(requested.as_secs_f64(), duration_secs);
        let mut runtime = runtime_for_seek.borrow_mut();
        if let Some(resource) = runtime.resource.as_ref() {
            resource.element.set_current_time(seconds);
            position.set(Duration::from_secs_f64(seconds));
            runtime.lifecycle.seeked(seconds, duration_secs);
            publish_lifecycle(&runtime.lifecycle, status, snapshot);
        }
    });

    let runtime_for_skip = runtime.clone();
    let skip = use_callback(move |delta: f64| {
        let duration_secs = duration().as_secs_f64();
        let mut runtime = runtime_for_skip.borrow_mut();
        if let Some(resource) = runtime.resource.as_ref() {
            let seconds = clamp_seek(resource.element.current_time() + delta, duration_secs);
            resource.element.set_current_time(seconds);
            position.set(Duration::from_secs_f64(seconds));
            runtime.lifecycle.seeked(seconds, duration_secs);
            publish_lifecycle(&runtime.lifecycle, status, snapshot);
        }
    });

    let runtime_for_rate = runtime.clone();
    let set_rate: Callback<f64, Result<(), PlaybackCommandError>> =
        use_callback(move |new_rate: f64| {
            if !new_rate.is_finite() || new_rate <= 0.0 {
                return Err(PlaybackCommandError("playback rate must be positive"));
            }
            if let Some(resource) = runtime_for_rate.borrow().resource.as_ref() {
                resource.element.set_playback_rate(new_rate);
            }
            rate.set(new_rate);
            Ok(())
        });

    let runtime_for_repeat = runtime.clone();
    let set_repeat = use_callback(move |repeat: bool| {
        let mut runtime = runtime_for_repeat.borrow_mut();
        runtime.lifecycle.set_repeat(repeat);
        if let Some(resource) = runtime.resource.as_ref() {
            resource.element.set_loop(repeat);
        }
        publish_lifecycle(&runtime.lifecycle, status, snapshot);
    });

    AudioPlayerController {
        status: status.into(),
        snapshot: snapshot.into(),
        position: position.into(),
        duration: duration.into(),
        rate: rate.into(),
        play,
        pause,
        stop,
        seek,
        skip,
        set_rate,
        set_repeat,
    }
}

struct PlayerRuntime {
    lifecycle: PlaybackLifecycle,
    resource: Option<WebPlayer>,
    generation: u64,
    play_generation: u64,
    owner_active: bool,
}

impl Default for PlayerRuntime {
    fn default() -> Self {
        Self {
            lifecycle: PlaybackLifecycle::default(),
            resource: None,
            generation: 0,
            play_generation: 0,
            owner_active: true,
        }
    }
}

fn publish_lifecycle(
    lifecycle: &PlaybackLifecycle,
    mut status: Signal<PlaybackStatus>,
    mut snapshot: Signal<PlaybackSnapshot>,
) {
    status.set(lifecycle.status().clone());
    snapshot.set(lifecycle.snapshot().clone());
}

fn with_current_runtime(
    runtime: &Weak<RefCell<PlayerRuntime>>,
    generation: u64,
    update: impl FnOnce(&mut PlayerRuntime),
) {
    let Some(runtime) = runtime.upgrade() else {
        return;
    };
    let mut runtime = runtime.borrow_mut();
    if !runtime.owner_active || runtime.generation != generation {
        return;
    }
    update(&mut runtime);
}

struct UnmountGuard(Weak<RefCell<PlayerRuntime>>);

impl Drop for UnmountGuard {
    fn drop(&mut self) {
        if let Some(runtime) = self.0.upgrade() {
            let resource = {
                let mut runtime = runtime.borrow_mut();
                runtime.owner_active = false;
                runtime.generation = runtime.generation.wrapping_add(1);
                runtime.play_generation = runtime.play_generation.wrapping_add(1);
                runtime.resource.take()
            };
            drop(resource);
        }
    }
}

struct WebPlayer {
    element: HtmlAudioElement,
    _object_url: ObjectUrl,
    on_loaded: Closure<dyn FnMut()>,
    on_can_play: Closure<dyn FnMut()>,
    on_time: Closure<dyn FnMut()>,
    on_playing: Closure<dyn FnMut()>,
    on_waiting: Closure<dyn FnMut()>,
    on_pause: Closure<dyn FnMut()>,
    on_ended: Closure<dyn FnMut()>,
    on_error: Closure<dyn FnMut()>,
}

impl WebPlayer {
    fn new(
        source: &AudioData,
        runtime: Weak<RefCell<PlayerRuntime>>,
        generation: u64,
        status: Signal<PlaybackStatus>,
        snapshot: Signal<PlaybackSnapshot>,
        mut position: Signal<Duration>,
        mut duration: Signal<Duration>,
        dioxus_runtime: Rc<DioxusRuntime>,
        dioxus_scope: ScopeId,
    ) -> Result<Self, AudioError> {
        let bytes = Uint8Array::new_with_length(source.bytes.len() as u32);
        bytes.copy_from(&source.bytes);
        let parts = js_sys::Array::new();
        parts.push(&bytes);
        let properties = web_sys::BlobPropertyBag::new();
        properties.set_type(&source.mime_type);
        let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &properties)
            .map_err(playback_error)?;
        let object_url =
            ObjectUrl(Url::create_object_url_with_blob(&blob).map_err(playback_error)?);
        let element = HtmlAudioElement::new_with_src(&object_url.0).map_err(playback_error)?;
        element.set_preload("metadata");

        let element_for_loaded = element.clone();
        let runtime_for_loaded = runtime.clone();
        let dioxus_runtime_for_loaded = dioxus_runtime.clone();
        let on_loaded = Closure::wrap(Box::new(move || {
            dioxus_runtime_for_loaded.in_scope(dioxus_scope, || {
                let value = element_for_loaded.duration();
                with_current_runtime(&runtime_for_loaded, generation, |runtime| {
                    if value.is_finite() && value > 0.0 {
                        duration.set(Duration::from_secs_f64(value));
                    }
                    runtime.lifecycle.loaded();
                    publish_lifecycle(&runtime.lifecycle, status, snapshot);
                });
            });
        }) as Box<dyn FnMut()>);
        add_listener(&element, "loadedmetadata", &on_loaded)?;

        let runtime_for_can_play = runtime.clone();
        let dioxus_runtime_for_can_play = dioxus_runtime.clone();
        let on_can_play = Closure::wrap(Box::new(move || {
            dioxus_runtime_for_can_play.in_scope(dioxus_scope, || {
                with_current_runtime(&runtime_for_can_play, generation, |runtime| {
                    runtime.lifecycle.playable();
                    publish_lifecycle(&runtime.lifecycle, status, snapshot);
                });
            });
        }) as Box<dyn FnMut()>);
        add_listener(&element, "canplay", &on_can_play)?;

        let element_for_time = element.clone();
        let runtime_for_time = runtime.clone();
        let dioxus_runtime_for_time = dioxus_runtime.clone();
        let on_time = Closure::wrap(Box::new(move || {
            dioxus_runtime_for_time.in_scope(dioxus_scope, || {
                let value = element_for_time.current_time();
                with_current_runtime(&runtime_for_time, generation, |runtime| {
                    if runtime.lifecycle.transport() != PlaybackTransport::Idle
                        && value.is_finite()
                        && value >= 0.0
                    {
                        position.set(Duration::from_secs_f64(value));
                    }
                });
            });
        }) as Box<dyn FnMut()>);
        add_listener(&element, "timeupdate", &on_time)?;

        let element_for_playing = element.clone();
        let runtime_for_playing = runtime.clone();
        let dioxus_runtime_for_playing = dioxus_runtime.clone();
        let on_playing = Closure::wrap(Box::new(move || {
            dioxus_runtime_for_playing.in_scope(dioxus_scope, || {
                with_current_runtime(&runtime_for_playing, generation, |runtime| {
                    if runtime.lifecycle.transport() == PlaybackTransport::Playing
                        && !element_for_playing.paused()
                        && !element_for_playing.ended()
                    {
                        runtime.lifecycle.playable();
                        publish_lifecycle(&runtime.lifecycle, status, snapshot);
                    }
                });
            });
        }) as Box<dyn FnMut()>);
        add_listener(&element, "playing", &on_playing)?;

        let element_for_waiting = element.clone();
        let runtime_for_waiting = runtime.clone();
        let dioxus_runtime_for_waiting = dioxus_runtime.clone();
        let on_waiting = Closure::wrap(Box::new(move || {
            dioxus_runtime_for_waiting.in_scope(dioxus_scope, || {
                with_current_runtime(&runtime_for_waiting, generation, |runtime| {
                    if !element_for_waiting.paused()
                        && !element_for_waiting.ended()
                        && element_for_waiting.ready_state() < HAVE_FUTURE_DATA
                    {
                        runtime.lifecycle.waiting();
                        publish_lifecycle(&runtime.lifecycle, status, snapshot);
                    }
                });
            });
        }) as Box<dyn FnMut()>);
        add_listener(&element, "waiting", &on_waiting)?;

        let element_for_pause = element.clone();
        let runtime_for_pause = runtime.clone();
        let dioxus_runtime_for_pause = dioxus_runtime.clone();
        let on_pause = Closure::wrap(Box::new(move || {
            dioxus_runtime_for_pause.in_scope(dioxus_scope, || {
                with_current_runtime(&runtime_for_pause, generation, |runtime| {
                    if element_for_pause.paused()
                        && runtime.lifecycle.transport() == PlaybackTransport::Playing
                    {
                        runtime.lifecycle.paused();
                        publish_lifecycle(&runtime.lifecycle, status, snapshot);
                    }
                });
            });
        }) as Box<dyn FnMut()>);
        add_listener(&element, "pause", &on_pause)?;

        let element_for_ended = element.clone();
        let runtime_for_ended = runtime.clone();
        let dioxus_runtime_for_ended = dioxus_runtime.clone();
        let on_ended = Closure::wrap(Box::new(move || {
            dioxus_runtime_for_ended.in_scope(dioxus_scope, || {
                with_current_runtime(&runtime_for_ended, generation, |runtime| {
                    if element_for_ended.ended() {
                        runtime.lifecycle.ended();
                        publish_lifecycle(&runtime.lifecycle, status, snapshot);
                    }
                });
            });
        }) as Box<dyn FnMut()>);
        add_listener(&element, "ended", &on_ended)?;

        let runtime_for_error = runtime;
        let on_error = Closure::wrap(Box::new(move || {
            dioxus_runtime.in_scope(dioxus_scope, || {
                with_current_runtime(&runtime_for_error, generation, |runtime| {
                    let error = AudioError::new(
                        AudioErrorKind::PlaybackFailure,
                        "browser could not decode or play this audio",
                    );
                    runtime.lifecycle.failed(error);
                    publish_lifecycle(&runtime.lifecycle, status, snapshot);
                });
            });
        }) as Box<dyn FnMut()>);
        add_listener(&element, "error", &on_error)?;

        Ok(Self {
            element,
            _object_url: object_url,
            on_loaded,
            on_can_play,
            on_time,
            on_playing,
            on_waiting,
            on_pause,
            on_ended,
            on_error,
        })
    }
}

impl Drop for WebPlayer {
    fn drop(&mut self) {
        remove_listener(&self.element, "loadedmetadata", &self.on_loaded);
        remove_listener(&self.element, "canplay", &self.on_can_play);
        remove_listener(&self.element, "timeupdate", &self.on_time);
        remove_listener(&self.element, "playing", &self.on_playing);
        remove_listener(&self.element, "waiting", &self.on_waiting);
        remove_listener(&self.element, "pause", &self.on_pause);
        remove_listener(&self.element, "ended", &self.on_ended);
        remove_listener(&self.element, "error", &self.on_error);
        let _ = self.element.pause();
        self.element.set_src("");
    }
}

struct ObjectUrl(String);

impl Drop for ObjectUrl {
    fn drop(&mut self) {
        let _ = Url::revoke_object_url(&self.0);
    }
}

fn add_listener(
    element: &HtmlAudioElement,
    name: &str,
    callback: &Closure<dyn FnMut()>,
) -> Result<(), AudioError> {
    element
        .add_event_listener_with_callback(name, callback.as_ref().unchecked_ref())
        .map_err(playback_error)
}

fn remove_listener(element: &HtmlAudioElement, name: &str, callback: &Closure<dyn FnMut()>) {
    let _ = element.remove_event_listener_with_callback(name, callback.as_ref().unchecked_ref());
}

fn playback_error(value: JsValue) -> AudioError {
    AudioError::new(
        AudioErrorKind::PlaybackFailure,
        value
            .as_string()
            .unwrap_or_else(|| "browser audio playback operation failed".to_string()),
    )
}

fn play_failure(value: JsValue) -> PlaybackPlayFailure {
    let interaction_required = value
        .dyn_ref::<web_sys::DomException>()
        .is_some_and(|error| error.name() == "NotAllowedError");
    let error = AudioError::new(
        AudioErrorKind::PlaybackFailure,
        value
            .as_string()
            .unwrap_or_else(|| "browser rejected playback".to_string()),
    );

    if interaction_required {
        PlaybackPlayFailure::InteractionRequired(error)
    } else {
        PlaybackPlayFailure::Unknown(error)
    }
}
