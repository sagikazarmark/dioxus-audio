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

pub(super) fn use_web_audio_player(
    source: ReadSignal<Option<AudioData>>,
    initial_duration: Duration,
) -> AudioPlayerController {
    let mut status = use_signal(|| PlaybackStatus::Empty);
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
            drop(runtime_mut);
            position.set(Duration::ZERO);
            duration.set(*initial_duration.peek());
            let source = source_input();
            let source = source.read();
            let Some(source) = source.as_ref() else {
                runtime.borrow_mut().lifecycle.unload();
                status.set(PlaybackStatus::Empty);
                return;
            };
            runtime.borrow_mut().lifecycle.loading();
            status.set(PlaybackStatus::Loading);
            match WebPlayer::new(
                source,
                Rc::downgrade(&runtime),
                status,
                position,
                duration,
                dioxus_runtime.clone(),
                dioxus_scope,
            ) {
                Ok(resource) => {
                    resource.element.set_playback_rate(*rate.peek());
                    runtime.borrow_mut().resource = Some(resource);
                }
                Err(error) => {
                    runtime.borrow_mut().lifecycle.failed(error.clone());
                    status.set(PlaybackStatus::Failed(error));
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
        let (element, generation, play_generation) = {
            let mut runtime = runtime_for_play.borrow_mut();
            if matches!(runtime.lifecycle.status(), PlaybackStatus::Failed(_))
                && runtime.resource.is_some()
            {
                runtime.lifecycle.loaded();
                status.set(PlaybackStatus::Ready);
            }
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
            )
        };
        if matches!(
            runtime_for_play.borrow().lifecycle.status(),
            PlaybackStatus::Ended
        ) {
            element.set_current_time(0.0);
            position.set(Duration::ZERO);
        }
        let promise = element
            .play()
            .map_err(|_| PlaybackCommandError("browser rejected playback"))?;
        let runtime = runtime_for_play.clone();
        spawn(async move {
            if let Err(value) = wasm_bindgen_futures::JsFuture::from(promise).await {
                let runtime_ref = runtime.borrow();
                if runtime_ref.generation != generation
                    || runtime_ref.play_generation != play_generation
                {
                    return;
                }
                drop(runtime_ref);
                let error = AudioError::new(
                    AudioErrorKind::PlaybackFailure,
                    value
                        .as_string()
                        .unwrap_or_else(|| "browser rejected playback".to_string()),
                );
                runtime.borrow_mut().lifecycle.failed(error.clone());
                status.set(PlaybackStatus::Failed(error));
            }
        });
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
            .map_err(|_| PlaybackCommandError("browser rejected pause"))
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
            status.set(runtime.lifecycle.status().clone());
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
            status.set(runtime.lifecycle.status().clone());
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

    AudioPlayerController {
        status: status.into(),
        position: position.into(),
        duration: duration.into(),
        rate: rate.into(),
        play,
        pause,
        seek,
        skip,
        set_rate,
    }
}

#[derive(Default)]
struct PlayerRuntime {
    lifecycle: PlaybackLifecycle,
    resource: Option<WebPlayer>,
    generation: u64,
    play_generation: u64,
}

struct UnmountGuard(Weak<RefCell<PlayerRuntime>>);

impl Drop for UnmountGuard {
    fn drop(&mut self) {
        if let Some(runtime) = self.0.upgrade() {
            runtime.borrow_mut().resource.take();
        }
    }
}

struct WebPlayer {
    element: HtmlAudioElement,
    _object_url: ObjectUrl,
    on_loaded: Closure<dyn FnMut()>,
    on_time: Closure<dyn FnMut()>,
    on_playing: Closure<dyn FnMut()>,
    on_pause: Closure<dyn FnMut()>,
    on_ended: Closure<dyn FnMut()>,
    on_error: Closure<dyn FnMut()>,
}

impl WebPlayer {
    fn new(
        source: &AudioData,
        runtime: Weak<RefCell<PlayerRuntime>>,
        mut status: Signal<PlaybackStatus>,
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
                if value.is_finite() && value > 0.0 {
                    duration.set(Duration::from_secs_f64(value));
                }
                if let Some(runtime) = runtime_for_loaded.upgrade() {
                    runtime.borrow_mut().lifecycle.loaded();
                    status.set(PlaybackStatus::Ready);
                }
            });
        }) as Box<dyn FnMut()>);
        add_listener(&element, "loadedmetadata", &on_loaded)?;

        let element_for_time = element.clone();
        let dioxus_runtime_for_time = dioxus_runtime.clone();
        let on_time = Closure::wrap(Box::new(move || {
            dioxus_runtime_for_time.in_scope(dioxus_scope, || {
                let value = element_for_time.current_time();
                if value.is_finite() && value >= 0.0 {
                    position.set(Duration::from_secs_f64(value));
                }
            });
        }) as Box<dyn FnMut()>);
        add_listener(&element, "timeupdate", &on_time)?;

        let runtime_for_playing = runtime.clone();
        let dioxus_runtime_for_playing = dioxus_runtime.clone();
        let on_playing = Closure::wrap(Box::new(move || {
            dioxus_runtime_for_playing.in_scope(dioxus_scope, || {
                if let Some(runtime) = runtime_for_playing.upgrade() {
                    runtime.borrow_mut().lifecycle.playing();
                    status.set(PlaybackStatus::Playing);
                }
            });
        }) as Box<dyn FnMut()>);
        add_listener(&element, "playing", &on_playing)?;

        let runtime_for_pause = runtime.clone();
        let dioxus_runtime_for_pause = dioxus_runtime.clone();
        let on_pause = Closure::wrap(Box::new(move || {
            dioxus_runtime_for_pause.in_scope(dioxus_scope, || {
                if let Some(runtime) = runtime_for_pause.upgrade() {
                    runtime.borrow_mut().lifecycle.paused();
                    status.set(runtime.borrow().lifecycle.status().clone());
                }
            });
        }) as Box<dyn FnMut()>);
        add_listener(&element, "pause", &on_pause)?;

        let runtime_for_ended = runtime.clone();
        let dioxus_runtime_for_ended = dioxus_runtime.clone();
        let on_ended = Closure::wrap(Box::new(move || {
            dioxus_runtime_for_ended.in_scope(dioxus_scope, || {
                if let Some(runtime) = runtime_for_ended.upgrade() {
                    runtime.borrow_mut().lifecycle.ended();
                    status.set(PlaybackStatus::Ended);
                }
            });
        }) as Box<dyn FnMut()>);
        add_listener(&element, "ended", &on_ended)?;

        let runtime_for_error = runtime;
        let on_error = Closure::wrap(Box::new(move || {
            dioxus_runtime.in_scope(dioxus_scope, || {
                if let Some(runtime) = runtime_for_error.upgrade() {
                    let error = AudioError::new(
                        AudioErrorKind::PlaybackFailure,
                        "browser could not decode or play this audio",
                    );
                    runtime.borrow_mut().lifecycle.failed(error.clone());
                    status.set(PlaybackStatus::Failed(error));
                }
            });
        }) as Box<dyn FnMut()>);
        add_listener(&element, "error", &on_error)?;

        Ok(Self {
            element,
            _object_url: object_url,
            on_loaded,
            on_time,
            on_playing,
            on_pause,
            on_ended,
            on_error,
        })
    }
}

impl Drop for WebPlayer {
    fn drop(&mut self) {
        let _ = self.element.pause();
        remove_listener(&self.element, "loadedmetadata", &self.on_loaded);
        remove_listener(&self.element, "timeupdate", &self.on_time);
        remove_listener(&self.element, "playing", &self.on_playing);
        remove_listener(&self.element, "pause", &self.on_pause);
        remove_listener(&self.element, "ended", &self.on_ended);
        remove_listener(&self.element, "error", &self.on_error);
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
