use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::time::Duration;

use dioxus::core::{Runtime as DioxusRuntime, ScopeId};
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;
use js_sys::Uint8Array;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{
    AudioContext, AudioContextState, GainNode, HtmlAudioElement, MediaElementAudioSourceNode,
    TimeRanges, Url,
};

use super::*;
use crate::analysis::AudioAnalyserControl;
use crate::{AudioError, AudioErrorKind};

const HAVE_FUTURE_DATA: u16 = 3;
const NETWORK_IDLE: u16 = 1;
const NETWORK_LOADING: u16 = 2;
const BOUNDED_SEEK_TOLERANCE_SECONDS: f64 = 0.05;
const BOUNDED_SEEK_TIMEOUT_MILLIS: u32 = 2_000;

pub(super) fn use_web_audio_player(
    source: ReadSignal<Option<PlaybackSource>>,
    initial_duration: Duration,
    options: PlaybackOptions,
) -> AudioPlayerController {
    let initial_lifecycle = PlaybackLifecycle::new(options);
    let status = use_signal(|| initial_lifecycle.status().clone());
    let snapshot = use_signal(|| initial_lifecycle.snapshot().clone());
    let mut position = use_signal(|| Duration::ZERO);
    let mut duration = use_signal(|| initial_duration);
    let mut rate = use_signal(|| 1.0_f64);
    let analyser = use_signal(|| None::<crate::analysis::AudioAnalyser>);
    let runtime = use_hook(|| Rc::new(RefCell::new(PlayerRuntime::new(options, analyser))));
    let dioxus_runtime = DioxusRuntime::current();
    let dioxus_scope = dioxus_runtime.current_scope_id();
    let initial_duration = use_memo(use_reactive!(|(initial_duration,)| initial_duration));

    {
        let runtime = Rc::downgrade(&runtime);
        use_hook(|| Rc::new(UnmountGuard(runtime)));
    }

    {
        let runtime = runtime.clone();
        let dioxus_runtime = dioxus_runtime.clone();
        use_effect(move || {
            let source = source();
            let fallback_duration = *initial_duration.peek();
            let (old_resource, generation) = {
                let mut runtime_mut = runtime.borrow_mut();
                runtime_mut.generation.advance();
                runtime_mut.attempt_generation.advance();
                runtime_mut.play_generation.advance();
                runtime_mut.invalidate_bounded_operation();
                runtime_mut.authoritative_duration = None;
                let old_resource = runtime_mut.resource.take();
                if let Some(graph) = runtime_mut.graph.as_ref() {
                    graph.analyser.set_available(false);
                }
                runtime_mut.lifecycle.graph_awaiting_source();
                runtime_mut.source = source.clone();
                runtime_mut.next_alternative = 0;
                runtime_mut.clear_alternative_failures();
                runtime_mut.fallback_duration = fallback_duration;
                (old_resource, runtime_mut.generation)
            };
            drop(old_resource);
            position.set(Duration::ZERO);
            duration.set(fallback_duration);
            let Some(source) = source.as_ref() else {
                runtime.borrow_mut().lifecycle.unload();
                publish_lifecycle(&runtime.borrow().lifecycle, status, snapshot);
                return;
            };
            if source.loading_policy == PlaybackLoadingPolicy::OnPlay {
                runtime.borrow_mut().lifecycle.dormant();
                publish_lifecycle(&runtime.borrow().lifecycle, status, snapshot);
                return;
            }
            runtime.borrow_mut().lifecycle.loading();
            publish_lifecycle(&runtime.borrow().lifecycle, status, snapshot);
            let _ = attach_current_source(
                &runtime,
                generation,
                status,
                snapshot,
                position,
                duration,
                dioxus_runtime.clone(),
                dioxus_scope,
                rate,
            );
        });
    }

    {
        let runtime = runtime.clone();
        use_effect(move || {
            let fallback = initial_duration();
            runtime.borrow_mut().fallback_duration = fallback;
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
    let dioxus_runtime_for_play = dioxus_runtime.clone();
    let play: Callback<(), Result<(), PlaybackCommandError>> = use_callback(move |()| {
        let action = {
            let mut runtime = runtime_for_play.borrow_mut();
            if runtime.pending_bounded_seek.is_some()
                && runtime
                    .lifecycle
                    .bounded()
                    .is_some_and(|bounded| bounded.phase.is_enforcing())
            {
                runtime
                    .pending_bounded_seek
                    .as_mut()
                    .expect("pending seek was just checked")
                    .resume_after = true;
                runtime.lifecycle.bounded_seeking();
                ControllerPlayAction::AwaitSeek
            } else if let Some(bounded) = runtime.lifecycle.bounded().copied()
                && bounded.phase == BoundedPlaybackPhase::Paused
            {
                let resource = runtime
                    .resource
                    .as_ref()
                    .ok_or(PlaybackCommandError("audio is not loaded"))?;
                let current_time = resource.element.current_time();
                if current_time >= bounded.range.start() && current_time < bounded.range.end() {
                    runtime.lifecycle.request_play()?;
                    runtime.lifecycle.bounded_activating();
                    runtime.play_generation.advance();
                    ControllerPlayAction::Play {
                        restart_ended: false,
                    }
                } else {
                    let element = resource.element.clone();
                    let operation = runtime.queue_bounded_seek(bounded.range.start(), true)?;
                    ControllerPlayAction::Seek {
                        element,
                        operation,
                        target: bounded.range.start(),
                    }
                }
            } else {
                if runtime.lifecycle.bounded().is_some_and(|bounded| {
                    matches!(
                        bounded.phase,
                        BoundedPlaybackPhase::Completed | BoundedPlaybackPhase::Failed(_)
                    )
                }) {
                    runtime.invalidate_bounded_operation();
                }
                let restart_ended = matches!(runtime.lifecycle.status(), PlaybackStatus::Ended);
                runtime.lifecycle.request_play()?;
                runtime.play_generation.advance();
                if runtime.resource.is_none() && runtime.source.is_some() {
                    ControllerPlayAction::Attach {
                        generation: runtime.generation,
                        restart_ended,
                    }
                } else {
                    ControllerPlayAction::Play { restart_ended }
                }
            }
        };
        publish_lifecycle(&runtime_for_play.borrow().lifecycle, status, snapshot);
        match action {
            ControllerPlayAction::AwaitSeek => {}
            ControllerPlayAction::Seek {
                element,
                operation,
                target,
            } => {
                if element.pause().is_err() {
                    let mut runtime = runtime_for_play.borrow_mut();
                    if runtime.is_current_bounded(operation) {
                        runtime.pending_bounded_seek = None;
                        runtime
                            .lifecycle
                            .bounded_failed(BoundedPlaybackFailure::PauseRejected);
                        element.set_loop(runtime.lifecycle.repeat());
                        publish_lifecycle(&runtime.lifecycle, status, snapshot);
                    }
                    return Err(PlaybackCommandError("browser rejected bounded pause"));
                }
                element.set_current_time(target);
                schedule_bounded_seek_timeout(
                    Rc::downgrade(&runtime_for_play),
                    operation,
                    status,
                    snapshot,
                    dioxus_runtime_for_play.clone(),
                    dioxus_scope,
                );
            }
            ControllerPlayAction::Attach {
                generation,
                restart_ended,
            } => {
                if restart_ended {
                    position.set(Duration::ZERO);
                }
                attach_current_source(
                    &runtime_for_play,
                    generation,
                    status,
                    snapshot,
                    position,
                    duration,
                    dioxus_runtime_for_play.clone(),
                    dioxus_scope,
                    rate,
                )?;
            }
            ControllerPlayAction::Play { restart_ended } => {
                if restart_ended {
                    let element = runtime_for_play
                        .borrow()
                        .resource
                        .as_ref()
                        .map(|resource| resource.element.clone())
                        .ok_or(PlaybackCommandError("audio is not loaded"))?;
                    element.set_current_time(0.0);
                    position.set(Duration::ZERO);
                }
                play_current_resource(&runtime_for_play, status, snapshot, position)?;
            }
        }
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
            runtime.play_generation.advance();
            runtime.invalidate_bounded_operation();
            runtime.lifecycle.stop()?;
            element.set_loop(runtime.lifecycle.repeat());
        }
        element.set_current_time(0.0);
        position.set(Duration::ZERO);
        publish_lifecycle(&runtime_for_stop.borrow().lifecycle, status, snapshot);
        Ok(())
    });

    let runtime_for_pause = runtime.clone();
    let pause: Callback<(), Result<(), PlaybackCommandError>> = use_callback(move |()| {
        let mut runtime = runtime_for_pause.borrow_mut();
        runtime.play_generation.advance();
        runtime.deadline_generation.advance();
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
        {
            let mut runtime = runtime_for_pause.borrow_mut();
            if let Some(pending) = runtime.pending_bounded_seek.as_mut() {
                pending.resume_after = false;
            }
            runtime.lifecycle.bounded_paused();
        }
        publish_lifecycle(&runtime_for_pause.borrow().lifecycle, status, snapshot);
        Ok(())
    });

    let runtime_for_seek = runtime.clone();
    let seek = use_callback(move |requested: Duration| {
        let duration_secs = duration().as_secs_f64();
        let seconds = clamp_seek(requested.as_secs_f64(), duration_secs);
        seek_current_resource(
            &runtime_for_seek,
            seconds,
            duration_secs,
            position,
            status,
            snapshot,
        );
    });

    let runtime_for_skip = runtime.clone();
    let skip = use_callback(move |delta: f64| {
        let duration_secs = duration().as_secs_f64();
        let current_time = runtime_for_skip
            .borrow()
            .resource
            .as_ref()
            .map(|resource| resource.element.current_time());
        if let Some(current_time) = current_time {
            seek_current_resource(
                &runtime_for_skip,
                clamp_seek(current_time + delta, duration_secs),
                duration_secs,
                position,
                status,
                snapshot,
            );
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
            let active = {
                let runtime = runtime_for_rate.borrow();
                runtime.active_bounded_operation().and_then(|operation| {
                    runtime
                        .resource
                        .as_ref()
                        .map(|resource| (operation, resource.element.clone()))
                })
            };
            if let Some((operation, element)) = active {
                schedule_bounded_deadline(
                    Rc::downgrade(&runtime_for_rate),
                    operation,
                    element,
                    position,
                    status,
                    snapshot,
                );
            }
            Ok(())
        });

    let runtime_for_repeat = runtime.clone();
    let set_repeat = use_callback(move |repeat: bool| {
        let mut runtime = runtime_for_repeat.borrow_mut();
        runtime.lifecycle.set_repeat(repeat);
        if let Some(resource) = runtime.resource.as_ref() {
            resource.element.set_loop(
                repeat
                    && !runtime
                        .lifecycle
                        .bounded()
                        .is_some_and(|bounded| bounded.phase.is_enforcing()),
            );
        }
        publish_lifecycle(&runtime.lifecycle, status, snapshot);
    });

    let runtime_for_muted = runtime.clone();
    let set_muted = use_callback(move |muted: bool| {
        let mut runtime = runtime_for_muted.borrow_mut();
        runtime.lifecycle.set_muted(muted);
        apply_audibility(&runtime);
        publish_lifecycle(&runtime.lifecycle, status, snapshot);
    });

    let runtime_for_audibility = runtime.clone();
    let set_audibility_level: Callback<f64, Result<(), PlaybackCommandError>> =
        use_callback(move |requested: f64| {
            let level = PlaybackAudibilityLevel::new(requested)?;
            let mut runtime = runtime_for_audibility.borrow_mut();
            if runtime.lifecycle.audibility_capability()
                == PlaybackAudibilityCapability::Unavailable
            {
                return Err(PlaybackCommandError("audibility level is unavailable"));
            }
            runtime.lifecycle.set_validated_audibility_level(level);
            apply_audibility(&runtime);
            publish_lifecycle(&runtime.lifecycle, status, snapshot);
            Ok(())
        });

    let runtime_for_bounded = runtime.clone();
    let dioxus_runtime_for_bounded = dioxus_runtime.clone();
    let play_bounded_once: Callback<WaveformSelection, Result<(), PlaybackCommandError>> =
        use_callback(move |selection: WaveformSelection| {
            let (element, operation) = {
                let mut runtime = runtime_for_bounded.borrow_mut();
                let duration_secs = runtime.authoritative_duration.ok_or(PlaybackCommandError(
                    "authoritative audio duration is unavailable",
                ))?;
                runtime
                    .lifecycle
                    .start_bounded_once(selection, duration_secs)?;
                runtime.play_generation.advance();
                runtime.begin_bounded_operation();
                let operation = runtime.queue_bounded_seek(selection.start(), true)?;
                let element = runtime
                    .resource
                    .as_ref()
                    .map(|resource| resource.element.clone())
                    .ok_or(PlaybackCommandError("audio is not loaded"))?;
                (element, operation)
            };

            if element.pause().is_err() {
                let mut runtime = runtime_for_bounded.borrow_mut();
                runtime
                    .lifecycle
                    .bounded_failed(BoundedPlaybackFailure::PauseRejected);
                runtime.pending_bounded_seek = None;
                publish_lifecycle(&runtime.lifecycle, status, snapshot);
                return Err(PlaybackCommandError("browser rejected bounded pause"));
            }
            {
                let mut runtime = runtime_for_bounded.borrow_mut();
                runtime.lifecycle.paused();
                runtime.lifecycle.bounded_seeking();
            }
            element.set_loop(false);
            element.set_current_time(selection.start());
            publish_lifecycle(&runtime_for_bounded.borrow().lifecycle, status, snapshot);
            schedule_bounded_seek_timeout(
                Rc::downgrade(&runtime_for_bounded),
                operation,
                status,
                snapshot,
                dioxus_runtime_for_bounded.clone(),
                dioxus_scope,
            );
            Ok(())
        });

    let runtime_for_cancel_bounded = runtime.clone();
    let cancel_bounded = use_callback(move |()| {
        let mut runtime = runtime_for_cancel_bounded.borrow_mut();
        let cancel_pending_play = runtime.lifecycle.transport() == PlaybackTransport::PlayPending;
        let element = runtime
            .resource
            .as_ref()
            .map(|resource| resource.element.clone());
        runtime.invalidate_bounded_operation();
        if cancel_pending_play {
            runtime.play_generation.advance();
            if let Some(element) = element.as_ref() {
                let _ = element.pause();
            }
            runtime.lifecycle.paused();
        }
        if let Some(element) = element {
            element.set_loop(runtime.lifecycle.repeat());
        }
        publish_lifecycle(&runtime.lifecycle, status, snapshot);
    });

    AudioPlayerController {
        status: status.into(),
        snapshot: snapshot.into(),
        position: position.into(),
        duration: duration.into(),
        rate: rate.into(),
        analyser: analyser.into(),
        play,
        pause,
        stop,
        seek,
        skip,
        play_bounded_once,
        cancel_bounded,
        set_rate,
        set_repeat,
        set_muted,
        set_audibility_level,
    }
}

fn seek_current_resource(
    runtime: &Rc<RefCell<PlayerRuntime>>,
    seconds: f64,
    duration_secs: f64,
    mut position: Signal<Duration>,
    status: Signal<PlaybackStatus>,
    snapshot: Signal<PlaybackSnapshot>,
) {
    let mut runtime = runtime.borrow_mut();
    let Some(element) = runtime
        .resource
        .as_ref()
        .map(|resource| resource.element.clone())
    else {
        return;
    };
    let cancel_pending_play = runtime.lifecycle.transport() == PlaybackTransport::PlayPending
        && runtime.lifecycle.bounded().is_some();
    if runtime.lifecycle.bounded().is_some() {
        runtime.invalidate_bounded_operation();
        element.set_loop(runtime.lifecycle.repeat());
    }
    if cancel_pending_play {
        runtime.play_generation.advance();
        let _ = element.pause();
        runtime.lifecycle.paused();
    }
    element.set_current_time(seconds);
    position.set(Duration::from_secs_f64(seconds));
    runtime.lifecycle.seeked(seconds, duration_secs);
    publish_lifecycle(&runtime.lifecycle, status, snapshot);
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct SourceGeneration(u64);

impl SourceGeneration {
    fn advance(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct AttemptGeneration(u64);

impl AttemptGeneration {
    fn advance(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct PlayGeneration(u64);

impl PlayGeneration {
    fn advance(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct BoundedGeneration(u64);

impl BoundedGeneration {
    fn advance(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct DeadlineGeneration(u64);

impl DeadlineGeneration {
    fn advance(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SourceAttempt {
    source: SourceGeneration,
    attempt: AttemptGeneration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BoundedOperation {
    source: SourceGeneration,
    operation: BoundedGeneration,
}

#[derive(Clone, Copy, Debug)]
struct PendingBoundedSeek {
    operation: BoundedOperation,
    target: f64,
    resume_after: bool,
}

enum ControllerPlayAction {
    AwaitSeek,
    Seek {
        element: HtmlAudioElement,
        operation: BoundedOperation,
        target: f64,
    },
    Attach {
        generation: SourceGeneration,
        restart_ended: bool,
    },
    Play {
        restart_ended: bool,
    },
}

struct PlayerRuntime {
    lifecycle: PlaybackLifecycle,
    source: Option<PlaybackSource>,
    resource: Option<WebPlayer>,
    graph: Option<PlaybackGraph>,
    analyser_signal: Signal<Option<crate::analysis::AudioAnalyser>>,
    generation: SourceGeneration,
    attempt_generation: AttemptGeneration,
    play_generation: PlayGeneration,
    bounded_generation: BoundedGeneration,
    deadline_generation: DeadlineGeneration,
    bounded_operation: Option<BoundedOperation>,
    pending_bounded_seek: Option<PendingBoundedSeek>,
    authoritative_duration: Option<f64>,
    next_alternative: usize,
    last_alternative_failure: Option<PlaybackSourceFailure>,
    alternative_failures: Vec<PlaybackAlternativeFailure>,
    fallback_duration: Duration,
    owner_active: bool,
}

enum SourceErrorAction {
    Retry {
        attempt: SourceAttempt,
        fallback_duration: Duration,
    },
    Terminal,
}

enum AlternativeFailureRetention {
    Discard,
    Retain,
}

struct PlaybackGraph {
    context: AudioContext,
    analyser: AudioAnalyserControl,
    gain: GainNode,
    _on_state_change: Closure<dyn FnMut()>,
}

impl PlaybackGraph {
    fn new(
        runtime: &Rc<RefCell<PlayerRuntime>>,
        status: Signal<PlaybackStatus>,
        snapshot: Signal<PlaybackSnapshot>,
        dioxus_runtime: Rc<DioxusRuntime>,
        dioxus_scope: ScopeId,
    ) -> Result<(Self, crate::analysis::AudioAnalyser), AudioError> {
        let context = AudioContext::new().map_err(playback_error)?;
        let setup = (|| {
            let analyser_node = context.create_analyser().map_err(playback_error)?;
            let gain = context.create_gain().map_err(playback_error)?;
            analyser_node
                .connect_with_audio_node(&gain)
                .map_err(playback_error)?;
            gain.connect_with_audio_node(&context.destination())
                .map_err(playback_error)?;
            Ok::<_, AudioError>((analyser_node, gain))
        })();
        let (analyser_node, gain) = match setup {
            Ok(nodes) => nodes,
            Err(error) => {
                settle_audio_promise(context.close());
                return Err(error);
            }
        };
        let (analyser, handle) = AudioAnalyserControl::new(analyser_node, context.sample_rate());
        let runtime = Rc::downgrade(runtime);
        let dioxus_runtime_for_state = dioxus_runtime;
        let on_state_change = Closure::wrap(Box::new(move || {
            dioxus_runtime_for_state.in_scope(dioxus_scope, || {
                if let Some(runtime) = runtime.upgrade() {
                    reconcile_graph_state(&runtime, status, snapshot);
                }
            });
        }) as Box<dyn FnMut()>);
        context.set_onstatechange(Some(on_state_change.as_ref().unchecked_ref()));

        Ok((
            Self {
                context,
                analyser,
                gain,
                _on_state_change: on_state_change,
            },
            handle,
        ))
    }
}

impl Drop for PlaybackGraph {
    fn drop(&mut self) {
        self.analyser.set_available(false);
        self.context.set_onstatechange(None);
        let _ = self.analyser.node().disconnect();
        let _ = self.gain.disconnect();
        settle_audio_promise(self.context.close());
    }
}

impl PlayerRuntime {
    fn new(
        options: PlaybackOptions,
        analyser_signal: Signal<Option<crate::analysis::AudioAnalyser>>,
    ) -> Self {
        Self {
            lifecycle: PlaybackLifecycle::new(options),
            source: None,
            resource: None,
            graph: None,
            analyser_signal,
            generation: SourceGeneration::default(),
            attempt_generation: AttemptGeneration::default(),
            play_generation: PlayGeneration::default(),
            bounded_generation: BoundedGeneration::default(),
            deadline_generation: DeadlineGeneration::default(),
            bounded_operation: None,
            pending_bounded_seek: None,
            authoritative_duration: None,
            next_alternative: 0,
            last_alternative_failure: None,
            alternative_failures: Vec::new(),
            fallback_duration: Duration::ZERO,
            owner_active: true,
        }
    }

    fn is_current_source(&self, generation: SourceGeneration) -> bool {
        self.owner_active && self.generation == generation
    }

    fn graph_requested(&self) -> bool {
        !matches!(
            self.lifecycle.graph_state(),
            PlaybackGraphState::NotRequested | PlaybackGraphState::Unavailable
        )
    }

    fn source_attempt(&self) -> SourceAttempt {
        SourceAttempt {
            source: self.generation,
            attempt: self.attempt_generation,
        }
    }

    fn is_current_attempt(&self, source_attempt: SourceAttempt) -> bool {
        self.is_current_source(source_attempt.source)
            && self.attempt_generation == source_attempt.attempt
    }

    fn is_current_bounded(&self, operation: BoundedOperation) -> bool {
        self.is_current_source(operation.source)
            && self.bounded_generation == operation.operation
            && self.bounded_operation == Some(operation)
    }

    fn begin_bounded_operation(&mut self) -> BoundedOperation {
        self.bounded_generation.advance();
        self.deadline_generation.advance();
        self.pending_bounded_seek = None;
        let operation = BoundedOperation {
            source: self.generation,
            operation: self.bounded_generation,
        };
        self.bounded_operation = Some(operation);
        operation
    }

    fn queue_bounded_seek(
        &mut self,
        target: f64,
        resume_after: bool,
    ) -> Result<BoundedOperation, PlaybackCommandError> {
        let operation = self
            .bounded_operation
            .ok_or(PlaybackCommandError("Bounded Playback is not active"))?;
        self.play_generation.advance();
        self.deadline_generation.advance();
        self.pending_bounded_seek = Some(PendingBoundedSeek {
            operation,
            target,
            resume_after,
        });
        self.lifecycle.bounded_seeking();
        Ok(operation)
    }

    fn active_bounded_operation(&self) -> Option<BoundedOperation> {
        self.lifecycle
            .bounded()
            .is_some_and(|bounded| bounded.phase == BoundedPlaybackPhase::Active)
            .then_some(self.bounded_operation)
            .flatten()
    }

    fn invalidate_bounded_operation(&mut self) {
        self.bounded_generation.advance();
        self.deadline_generation.advance();
        self.bounded_operation = None;
        self.pending_bounded_seek = None;
        self.lifecycle.cancel_bounded();
    }

    fn advance_attempt(&mut self) -> AttemptGeneration {
        self.attempt_generation.advance();
        self.attempt_generation
    }

    fn next_candidate(
        &mut self,
        element: &HtmlAudioElement,
    ) -> Result<Option<WebPlayerInput>, PlaybackCommandError> {
        let source = self
            .source
            .as_ref()
            .cloned()
            .ok_or(PlaybackCommandError("audio is not loaded"))?;
        match source.input {
            PlaybackSourceInput::AudioData(audio) => Ok(Some(WebPlayerInput::AudioData(audio))),
            PlaybackSourceInput::Url(alternatives) => {
                while let Some(alternative) = alternatives.get(self.next_alternative).cloned() {
                    self.next_alternative += 1;
                    if self.graph_requested() && !alternative.is_graph_eligible() {
                        let failure = graph_ineligible_alternative_failure();
                        self.record_alternative_failure(alternative, &failure);
                        self.last_alternative_failure = Some(failure);
                        continue;
                    }
                    let definitely_unsupported = alternative
                        .media_type()
                        .is_some_and(|media_type| element.can_play_type(media_type).is_empty());
                    if definitely_unsupported {
                        let failure = unsupported_alternative_failure();
                        self.record_alternative_failure(alternative, &failure);
                        self.last_alternative_failure = Some(failure);
                        continue;
                    }
                    return Ok(Some(WebPlayerInput::Url(alternative)));
                }
                Ok(None)
            }
        }
    }

    fn record_alternative_failure(
        &mut self,
        alternative: PlaybackSourceAlternative,
        failure: &PlaybackSourceFailure,
    ) {
        self.alternative_failures
            .push(PlaybackAlternativeFailure::new(alternative, failure));
    }

    fn clear_alternative_failures(&mut self) {
        self.last_alternative_failure = None;
        self.alternative_failures.clear();
    }

    fn handle_source_error(
        &mut self,
        alternative: Option<&PlaybackSourceAlternative>,
        failure: PlaybackSourceFailure,
    ) -> SourceErrorAction {
        let tentative_url = matches!(
            self.source.as_ref().map(|source| &source.input),
            Some(PlaybackSourceInput::Url(_))
        ) && self.lifecycle.selected_alternative().is_none();
        if tentative_url {
            if let Some(graph) = self.graph.as_ref() {
                graph.analyser.set_available(false);
                self.lifecycle.graph_awaiting_source();
            }
            if let Some(alternative) = alternative {
                self.record_alternative_failure(alternative.clone(), &failure);
            }
            self.last_alternative_failure = Some(failure);
            self.authoritative_duration = None;
            self.advance_attempt();
            self.lifecycle.loading_alternative();
            SourceErrorAction::Retry {
                attempt: self.source_attempt(),
                fallback_duration: self.fallback_duration,
            }
        } else {
            self.play_generation.advance();
            self.invalidate_bounded_operation();
            self.authoritative_duration = None;
            self.clear_alternative_failures();
            if let Some(graph) = self.graph.as_ref() {
                graph.analyser.set_available(false);
                self.lifecycle.graph_awaiting_source();
            }
            self.lifecycle.source_failed(failure);
            SourceErrorAction::Terminal
        }
    }
}

fn attach_graph_source(
    runtime: &Rc<RefCell<PlayerRuntime>>,
    element: &HtmlAudioElement,
    status: Signal<PlaybackStatus>,
    snapshot: Signal<PlaybackSnapshot>,
    dioxus_runtime: Rc<DioxusRuntime>,
    dioxus_scope: ScopeId,
) -> Result<MediaElementAudioSourceNode, AudioError> {
    if runtime.borrow().graph.is_none() {
        let (graph, analyser) =
            PlaybackGraph::new(runtime, status, snapshot, dioxus_runtime, dioxus_scope)?;
        let mut analyser_signal = {
            let mut runtime = runtime.borrow_mut();
            runtime.graph = Some(graph);
            runtime.analyser_signal
        };
        analyser_signal.set(Some(analyser));
    }

    let runtime_ref = runtime.borrow();
    let graph = runtime_ref
        .graph
        .as_ref()
        .expect("Playback graph was created before source attachment");
    let source = graph
        .context
        .create_media_element_source(element)
        .map_err(playback_error)?;
    source
        .connect_with_audio_node(graph.analyser.node())
        .map_err(playback_error)?;
    Ok(source)
}

fn reconcile_graph_state(
    runtime: &Rc<RefCell<PlayerRuntime>>,
    status: Signal<PlaybackStatus>,
    snapshot: Signal<PlaybackSnapshot>,
) {
    let (state, attached, direct_routed, element) = {
        let runtime = runtime.borrow();
        let Some(graph) = runtime.graph.as_ref() else {
            return;
        };
        (
            graph.context.state(),
            runtime
                .resource
                .as_ref()
                .is_some_and(|resource| resource.graph_source.is_some()),
            runtime
                .resource
                .as_ref()
                .is_some_and(|resource| resource.graph_source.is_none()),
            runtime
                .resource
                .as_ref()
                .map(|resource| resource.element.clone()),
        )
    };

    if state == AudioContextState::Closed {
        degrade_graph(runtime, status, snapshot);
        return;
    }

    let mut runtime_ref = runtime.borrow_mut();
    let Some(graph) = runtime_ref.graph.as_ref() else {
        return;
    };
    if state == AudioContextState::Running
        && attached
        && runtime_ref.lifecycle.graph_state() != PlaybackGraphState::InteractionRequired
    {
        graph.analyser.set_available(true);
        runtime_ref.lifecycle.graph_running();
    } else {
        graph.analyser.set_available(false);
        if attached {
            if state == AudioContextState::Suspended
                && runtime_ref.lifecycle.transport() == PlaybackTransport::Playing
            {
                if let Some(element) = element {
                    let _ = element.pause();
                }
                runtime_ref.play_generation.advance();
                runtime_ref.deadline_generation.advance();
                runtime_ref
                    .lifecycle
                    .graph_interaction_required(AudioError::new(
                        AudioErrorKind::PlaybackFailure,
                        "Playback graph was interrupted and needs interaction",
                    ));
                runtime_ref.lifecycle.bounded_paused();
            } else {
                runtime_ref.lifecycle.graph_suspended();
            }
        } else {
            runtime_ref.lifecycle.graph_awaiting_source();
            if direct_routed {
                runtime_ref.lifecycle.direct_audibility();
            }
        }
    }
    publish_lifecycle(&runtime_ref.lifecycle, status, snapshot);
}

fn degrade_graph(
    runtime: &Rc<RefCell<PlayerRuntime>>,
    status: Signal<PlaybackStatus>,
    snapshot: Signal<PlaybackSnapshot>,
) {
    let (graph, mut analyser_signal) = {
        let mut runtime = runtime.borrow_mut();
        let graph = runtime.graph.take();
        runtime.lifecycle.graph_unavailable();
        (graph, runtime.analyser_signal)
    };
    analyser_signal.set(None);
    drop(graph);
    publish_lifecycle(&runtime.borrow().lifecycle, status, snapshot);
}

fn settle_audio_promise(result: Result<js_sys::Promise, JsValue>) {
    if let Ok(promise) = result {
        wasm_bindgen_futures::spawn_local(async move {
            let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn attach_current_source(
    runtime: &Rc<RefCell<PlayerRuntime>>,
    generation: SourceGeneration,
    status: Signal<PlaybackStatus>,
    snapshot: Signal<PlaybackSnapshot>,
    position: Signal<Duration>,
    duration: Signal<Duration>,
    dioxus_runtime: Rc<DioxusRuntime>,
    dioxus_scope: ScopeId,
    rate: Signal<f64>,
) -> Result<(), PlaybackCommandError> {
    let mut element = HtmlAudioElement::new().map_err(|value| {
        terminate_current_source(
            runtime,
            PlaybackSourceFailure::Unknown(playback_error(value)),
            AlternativeFailureRetention::Discard,
            status,
            snapshot,
        );
        PlaybackCommandError("browser rejected the Playback Source")
    })?;

    let candidate = {
        let mut runtime = runtime.borrow_mut();
        if !runtime.is_current_source(generation) {
            return Err(PlaybackCommandError("audio source was replaced"));
        }
        runtime.next_candidate(&element)?
    };

    let Some(candidate) = candidate else {
        let failure = runtime
            .borrow_mut()
            .last_alternative_failure
            .take()
            .unwrap_or_else(unknown_alternative_failure);
        terminate_current_source(
            runtime,
            failure,
            AlternativeFailureRetention::Retain,
            status,
            snapshot,
        );
        return Err(PlaybackCommandError(
            "no Playback Source alternative became playable",
        ));
    };

    candidate.apply_cross_origin(&element);

    let graph_source = if candidate.is_graph_eligible() && runtime.borrow().graph_requested() {
        runtime.borrow_mut().lifecycle.graph_preparing();
        publish_lifecycle(&runtime.borrow().lifecycle, status, snapshot);
        match attach_graph_source(
            runtime,
            &element,
            status,
            snapshot,
            dioxus_runtime.clone(),
            dioxus_scope,
        ) {
            Ok(source) => Some(source),
            Err(_) => {
                degrade_graph(runtime, status, snapshot);
                element = HtmlAudioElement::new().map_err(|value| {
                    terminate_current_source(
                        runtime,
                        PlaybackSourceFailure::Unknown(playback_error(value)),
                        AlternativeFailureRetention::Discard,
                        status,
                        snapshot,
                    );
                    PlaybackCommandError("browser rejected the Playback Source")
                })?;
                candidate.apply_cross_origin(&element);
                None
            }
        }
    } else {
        if !candidate.is_graph_eligible() {
            runtime.borrow_mut().lifecycle.direct_audibility();
            publish_lifecycle(&runtime.borrow().lifecycle, status, snapshot);
        }
        None
    };

    let source_attempt = {
        let mut runtime = runtime.borrow_mut();
        runtime.advance_attempt();
        runtime
            .lifecycle
            .source_attempt_started(candidate.is_url_addressable());
        runtime.source_attempt()
    };
    publish_lifecycle(&runtime.borrow().lifecycle, status, snapshot);
    let resource = WebPlayer::new(
        element,
        candidate,
        Rc::downgrade(runtime),
        source_attempt,
        status,
        snapshot,
        position,
        duration,
        dioxus_runtime,
        dioxus_scope,
        rate,
        graph_source,
    )
    .map_err(|error| {
        terminate_current_source(
            runtime,
            PlaybackSourceFailure::Unknown(error),
            AlternativeFailureRetention::Discard,
            status,
            snapshot,
        );
        PlaybackCommandError("browser rejected the Playback Source")
    })?;

    runtime.borrow_mut().resource = Some(resource);
    {
        let runtime_ref = runtime.borrow();
        configure_resource(&runtime_ref, *rate.peek());
    }
    reconcile_graph_state(runtime, status, snapshot);
    if runtime.borrow().lifecycle.transport() == PlaybackTransport::PlayPending {
        play_current_resource(runtime, status, snapshot, position)?;
    }
    Ok(())
}

fn play_current_resource(
    runtime: &Rc<RefCell<PlayerRuntime>>,
    status: Signal<PlaybackStatus>,
    snapshot: Signal<PlaybackSnapshot>,
    position: Signal<Duration>,
) -> Result<(), PlaybackCommandError> {
    let (element, source_attempt, play_generation, graph_context, bounded_operation) = {
        let runtime = runtime.borrow();
        let resource = runtime
            .resource
            .as_ref()
            .ok_or(PlaybackCommandError("audio is not loaded"))?;
        (
            resource.element.clone(),
            runtime.source_attempt(),
            runtime.play_generation,
            resource
                .graph_source
                .as_ref()
                .and_then(|_| runtime.graph.as_ref().map(|graph| graph.context.clone())),
            runtime
                .lifecycle
                .bounded()
                .is_some_and(|bounded| bounded.phase == BoundedPlaybackPhase::Activating)
                .then_some(runtime.bounded_operation)
                .flatten(),
        )
    };

    if let Some(context) = graph_context {
        let resume = context.resume();
        let media_play = element.play();
        let promise = match (resume, media_play) {
            (Ok(resume), Ok(media_play)) => {
                let promises = js_sys::Array::new();
                promises.push(&resume);
                promises.push(&media_play);
                js_sys::Promise::all(&promises)
            }
            (Err(value), _) | (_, Err(value)) => {
                reject_graph_activation(
                    runtime,
                    &element,
                    value,
                    bounded_operation,
                    status,
                    snapshot,
                );
                return Err(PlaybackCommandError(
                    "browser rejected graph-backed playback",
                ));
            }
        };
        let runtime = Rc::downgrade(runtime);
        spawn(async move {
            let outcome = wasm_bindgen_futures::JsFuture::from(promise).await;
            let Some(runtime) = runtime.upgrade() else {
                return;
            };
            let runtime_ref = runtime.borrow();
            if !runtime_ref.is_current_attempt(source_attempt)
                || runtime_ref.play_generation != play_generation
                || bounded_operation
                    .is_some_and(|operation| !runtime_ref.is_current_bounded(operation))
            {
                let should_pause = runtime_ref.is_current_attempt(source_attempt)
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

            if outcome.is_ok() && context.state() == AudioContextState::Running {
                let mut runtime_ref = runtime.borrow_mut();
                if let Some(graph) = runtime_ref.graph.as_ref() {
                    graph.analyser.set_available(true);
                }
                runtime_ref.lifecycle.graph_running();
                drop(runtime_ref);
                confirm_playback(
                    &runtime,
                    bounded_operation,
                    &element,
                    position,
                    status,
                    snapshot,
                );
            } else {
                let value = outcome.err().unwrap_or_else(|| {
                    JsValue::from_str("Playback graph did not enter the running state")
                });
                reject_graph_activation(
                    &runtime,
                    &element,
                    value,
                    bounded_operation,
                    status,
                    snapshot,
                );
            }
        });
        return Ok(());
    }

    let promise = match element.play() {
        Ok(promise) => promise,
        Err(value) => {
            reject_media_activation(
                runtime,
                &element,
                value,
                bounded_operation,
                status,
                snapshot,
            );
            return Err(PlaybackCommandError("browser rejected playback"));
        }
    };
    let runtime = Rc::downgrade(runtime);
    spawn(async move {
        let outcome = wasm_bindgen_futures::JsFuture::from(promise).await;
        let Some(runtime) = runtime.upgrade() else {
            return;
        };
        let runtime_ref = runtime.borrow();
        if !runtime_ref.is_current_attempt(source_attempt)
            || runtime_ref.play_generation != play_generation
            || bounded_operation.is_some_and(|operation| !runtime_ref.is_current_bounded(operation))
        {
            let should_pause = runtime_ref.is_current_attempt(source_attempt)
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
            Ok(_) => confirm_playback(
                &runtime,
                bounded_operation,
                &element,
                position,
                status,
                snapshot,
            ),
            Err(value) => reject_media_activation(
                &runtime,
                &element,
                value,
                bounded_operation,
                status,
                snapshot,
            ),
        }
    });
    Ok(())
}

fn confirm_playback(
    runtime: &Rc<RefCell<PlayerRuntime>>,
    bounded_operation: Option<BoundedOperation>,
    element: &HtmlAudioElement,
    position: Signal<Duration>,
    status: Signal<PlaybackStatus>,
    snapshot: Signal<PlaybackSnapshot>,
) {
    let schedule_deadline = {
        let mut runtime = runtime.borrow_mut();
        runtime.lifecycle.playing();
        if bounded_operation.is_some_and(|operation| runtime.is_current_bounded(operation)) {
            runtime.lifecycle.bounded_active();
        }
        let schedule_deadline = bounded_operation
            .is_some_and(|operation| runtime.active_bounded_operation() == Some(operation));
        publish_lifecycle(&runtime.lifecycle, status, snapshot);
        schedule_deadline
    };
    if schedule_deadline {
        schedule_bounded_deadline(
            Rc::downgrade(runtime),
            bounded_operation.expect("active bounded operation was confirmed"),
            element.clone(),
            position,
            status,
            snapshot,
        );
    }
}

fn reject_media_activation(
    runtime: &Rc<RefCell<PlayerRuntime>>,
    element: &HtmlAudioElement,
    value: JsValue,
    bounded_operation: Option<BoundedOperation>,
    status: Signal<PlaybackStatus>,
    snapshot: Signal<PlaybackSnapshot>,
) {
    let mut runtime = runtime.borrow_mut();
    runtime.lifecycle.play_rejected(play_failure(value));
    if bounded_operation.is_some_and(|operation| runtime.is_current_bounded(operation)) {
        let _ = element.pause();
        runtime.deadline_generation.advance();
        runtime
            .lifecycle
            .bounded_failed(BoundedPlaybackFailure::ActivationRejected);
        element.set_loop(runtime.lifecycle.repeat());
    }
    publish_lifecycle(&runtime.lifecycle, status, snapshot);
}

fn reject_graph_activation(
    runtime: &Rc<RefCell<PlayerRuntime>>,
    element: &HtmlAudioElement,
    value: JsValue,
    bounded_operation: Option<BoundedOperation>,
    status: Signal<PlaybackStatus>,
    snapshot: Signal<PlaybackSnapshot>,
) {
    let _ = element.pause();
    let mut runtime = runtime.borrow_mut();
    if let Some(graph) = runtime.graph.as_ref() {
        graph.analyser.set_available(false);
    }
    runtime
        .lifecycle
        .graph_interaction_required(playback_error(value));
    if bounded_operation.is_some_and(|operation| runtime.is_current_bounded(operation)) {
        runtime.deadline_generation.advance();
        runtime
            .lifecycle
            .bounded_failed(BoundedPlaybackFailure::ActivationRejected);
        element.set_loop(runtime.lifecycle.repeat());
    }
    publish_lifecycle(&runtime.lifecycle, status, snapshot);
}

fn schedule_bounded_seek_timeout(
    runtime: Weak<RefCell<PlayerRuntime>>,
    operation: BoundedOperation,
    status: Signal<PlaybackStatus>,
    snapshot: Signal<PlaybackSnapshot>,
    dioxus_runtime: Rc<DioxusRuntime>,
    dioxus_scope: ScopeId,
) {
    wasm_bindgen_futures::spawn_local(async move {
        TimeoutFuture::new(BOUNDED_SEEK_TIMEOUT_MILLIS).await;
        let Some(runtime) = runtime.upgrade() else {
            return;
        };
        dioxus_runtime.in_scope(dioxus_scope, || {
            let mut runtime = runtime.borrow_mut();
            if !runtime.is_current_bounded(operation)
                || !runtime
                    .pending_bounded_seek
                    .is_some_and(|pending| pending.operation == operation)
            {
                return;
            }
            runtime.pending_bounded_seek = None;
            runtime.play_generation.advance();
            runtime.deadline_generation.advance();
            runtime
                .lifecycle
                .bounded_failed(BoundedPlaybackFailure::SeekTimedOut);
            if let Some(resource) = runtime.resource.as_ref() {
                resource.element.set_loop(runtime.lifecycle.repeat());
            }
            publish_lifecycle(&runtime.lifecycle, status, snapshot);
        });
    });
}

fn schedule_bounded_deadline(
    runtime: Weak<RefCell<PlayerRuntime>>,
    operation: BoundedOperation,
    element: HtmlAudioElement,
    position: Signal<Duration>,
    status: Signal<PlaybackStatus>,
    snapshot: Signal<PlaybackSnapshot>,
) {
    let Some(runtime_ref) = runtime.upgrade() else {
        return;
    };
    let (deadline, delay_millis) = {
        let mut runtime = runtime_ref.borrow_mut();
        if !runtime.is_current_bounded(operation)
            || runtime.active_bounded_operation() != Some(operation)
            || runtime.lifecycle.transport() != PlaybackTransport::Playing
        {
            return;
        }
        let end = runtime
            .lifecycle
            .bounded()
            .expect("active bounded operation has a range")
            .range
            .end();
        let current = element.current_time();
        let rate = element.playback_rate();
        let delay_seconds = if current.is_finite() && rate.is_finite() && rate > 0.0 {
            ((end - current).max(0.0)) / rate
        } else {
            0.0
        };
        runtime.deadline_generation.advance();
        let delay_millis = (delay_seconds * 1_000.0).ceil().clamp(0.0, u32::MAX as f64) as u32;
        (runtime.deadline_generation, delay_millis)
    };
    drop(runtime_ref);

    wasm_bindgen_futures::spawn_local(async move {
        TimeoutFuture::new(delay_millis).await;
        let Some(runtime_ref) = runtime.upgrade() else {
            return;
        };
        let should_reconcile = {
            let runtime = runtime_ref.borrow();
            runtime.is_current_bounded(operation)
                && runtime.deadline_generation == deadline
                && runtime.active_bounded_operation() == Some(operation)
                && runtime.lifecycle.transport() == PlaybackTransport::Playing
        };
        if !should_reconcile {
            return;
        }

        let end = runtime_ref
            .borrow()
            .lifecycle
            .bounded()
            .expect("current bounded operation has a range")
            .range
            .end();
        if element.current_time() >= end {
            complete_bounded_playback(
                &runtime_ref,
                operation,
                &element,
                position,
                status,
                snapshot,
            );
        } else {
            schedule_bounded_deadline(
                Rc::downgrade(&runtime_ref),
                operation,
                element,
                position,
                status,
                snapshot,
            );
        }
    });
}

fn complete_bounded_playback(
    runtime: &Rc<RefCell<PlayerRuntime>>,
    operation: BoundedOperation,
    element: &HtmlAudioElement,
    mut position: Signal<Duration>,
    status: Signal<PlaybackStatus>,
    snapshot: Signal<PlaybackSnapshot>,
) {
    let end = {
        let runtime = runtime.borrow();
        if !runtime.is_current_bounded(operation)
            || runtime.active_bounded_operation() != Some(operation)
        {
            return;
        }
        runtime
            .lifecycle
            .bounded()
            .expect("active bounded operation has a range")
            .range
            .end()
    };

    if element.pause().is_err() {
        let mut runtime = runtime.borrow_mut();
        if runtime.is_current_bounded(operation) {
            runtime.deadline_generation.advance();
            runtime
                .lifecycle
                .bounded_failed(BoundedPlaybackFailure::PauseRejected);
            element.set_loop(runtime.lifecycle.repeat());
            publish_lifecycle(&runtime.lifecycle, status, snapshot);
        }
        return;
    }

    let mut runtime = runtime.borrow_mut();
    if !runtime.is_current_bounded(operation) {
        return;
    }
    runtime.play_generation.advance();
    runtime.deadline_generation.advance();
    runtime.pending_bounded_seek = None;
    runtime.lifecycle.bounded_completed();
    element.set_loop(runtime.lifecycle.repeat());
    position.set(Duration::from_secs_f64(end));
    publish_lifecycle(&runtime.lifecycle, status, snapshot);
}

fn terminate_current_source(
    runtime: &Rc<RefCell<PlayerRuntime>>,
    failure: PlaybackSourceFailure,
    retention: AlternativeFailureRetention,
    status: Signal<PlaybackStatus>,
    snapshot: Signal<PlaybackSnapshot>,
) {
    let mut runtime = runtime.borrow_mut();
    runtime.attempt_generation.advance();
    runtime.play_generation.advance();
    runtime.invalidate_bounded_operation();
    runtime.authoritative_duration = None;
    match retention {
        AlternativeFailureRetention::Discard => {
            runtime.clear_alternative_failures();
            runtime.lifecycle.source_failed(failure);
        }
        AlternativeFailureRetention::Retain => {
            let failures = std::mem::take(&mut runtime.alternative_failures);
            runtime.lifecycle.source_exhausted(failure, failures);
        }
    }
    publish_lifecycle(&runtime.lifecycle, status, snapshot);
}

#[allow(clippy::too_many_arguments)]
fn schedule_fallback(
    runtime: Weak<RefCell<PlayerRuntime>>,
    source_attempt: SourceAttempt,
    status: Signal<PlaybackStatus>,
    snapshot: Signal<PlaybackSnapshot>,
    position: Signal<Duration>,
    duration: Signal<Duration>,
    dioxus_runtime: Rc<DioxusRuntime>,
    dioxus_scope: ScopeId,
    rate: Signal<f64>,
) {
    wasm_bindgen_futures::spawn_local(async move {
        TimeoutFuture::new(0).await;
        let Some(runtime) = runtime.upgrade() else {
            return;
        };
        let old_resource = {
            let mut runtime = runtime.borrow_mut();
            if !runtime.is_current_attempt(source_attempt) {
                return;
            }
            runtime.resource.take()
        };
        drop(old_resource);
        dioxus_runtime.in_scope(dioxus_scope, || {
            let _ = attach_current_source(
                &runtime,
                source_attempt.source,
                status,
                snapshot,
                position,
                duration,
                dioxus_runtime.clone(),
                dioxus_scope,
                rate,
            );
        });
    });
}

fn unsupported_alternative_failure() -> PlaybackSourceFailure {
    PlaybackSourceFailure::Unsupported(AudioError::new(
        AudioErrorKind::PlaybackFailure,
        "browser definitely does not support a Playback Source alternative",
    ))
}

fn graph_ineligible_alternative_failure() -> PlaybackSourceFailure {
    PlaybackSourceFailure::GraphIneligible(AudioError::new(
        AudioErrorKind::PlaybackFailure,
        "Playback Source alternative is ineligible for graph-backed Playback",
    ))
}

fn unknown_alternative_failure() -> PlaybackSourceFailure {
    PlaybackSourceFailure::Unknown(AudioError::new(
        AudioErrorKind::PlaybackFailure,
        "browser could not load any Playback Source alternative",
    ))
}

fn publish_lifecycle(
    lifecycle: &PlaybackLifecycle,
    mut status: Signal<PlaybackStatus>,
    mut snapshot: Signal<PlaybackSnapshot>,
) {
    status.set(lifecycle.status().clone());
    snapshot.set(lifecycle.snapshot().clone());
}

fn with_current_attempt(
    runtime: &Weak<RefCell<PlayerRuntime>>,
    source_attempt: SourceAttempt,
    update: impl FnOnce(&mut PlayerRuntime),
) {
    let Some(runtime) = runtime.upgrade() else {
        return;
    };
    let mut runtime = runtime.borrow_mut();
    if !runtime.is_current_attempt(source_attempt) {
        return;
    }
    update(&mut runtime);
}

struct UnmountGuard(Weak<RefCell<PlayerRuntime>>);

impl Drop for UnmountGuard {
    fn drop(&mut self) {
        if let Some(runtime) = self.0.upgrade() {
            let (resource, graph) = {
                let mut runtime = runtime.borrow_mut();
                runtime.owner_active = false;
                runtime.generation.advance();
                runtime.attempt_generation.advance();
                runtime.play_generation.advance();
                runtime.invalidate_bounded_operation();
                runtime.authoritative_duration = None;
                runtime.source = None;
                runtime.clear_alternative_failures();
                (runtime.resource.take(), runtime.graph.take())
            };
            drop(resource);
            drop(graph);
        }
    }
}

struct WebPlayer {
    element: HtmlAudioElement,
    graph_source: Option<MediaElementAudioSourceNode>,
    _object_url: Option<ObjectUrl>,
    listeners: Vec<EventListener>,
}

#[derive(Clone)]
enum WebPlayerInput {
    AudioData(Arc<AudioData>),
    Url(PlaybackSourceAlternative),
}

impl WebPlayerInput {
    fn is_url_addressable(&self) -> bool {
        matches!(self, Self::Url(_))
    }

    fn is_graph_eligible(&self) -> bool {
        match self {
            Self::AudioData(_) => true,
            Self::Url(alternative) => alternative.is_graph_eligible(),
        }
    }

    fn apply_cross_origin(&self, element: &HtmlAudioElement) {
        let Self::Url(alternative) = self else {
            return;
        };
        match alternative.cross_origin() {
            Some(PlaybackSourceCrossOrigin::Anonymous) => {
                element.set_cross_origin(Some("anonymous"));
            }
            Some(PlaybackSourceCrossOrigin::UseCredentials) => {
                element.set_cross_origin(Some("use-credentials"));
            }
            None => {}
        }
    }
}

impl WebPlayer {
    #[allow(clippy::too_many_arguments)]
    fn new(
        element: HtmlAudioElement,
        source: WebPlayerInput,
        runtime: Weak<RefCell<PlayerRuntime>>,
        source_attempt: SourceAttempt,
        status: Signal<PlaybackStatus>,
        snapshot: Signal<PlaybackSnapshot>,
        mut position: Signal<Duration>,
        mut duration: Signal<Duration>,
        dioxus_runtime: Rc<DioxusRuntime>,
        dioxus_scope: ScopeId,
        rate: Signal<f64>,
        graph_source: Option<MediaElementAudioSourceNode>,
    ) -> Result<Self, AudioError> {
        let (source_url, object_url, selected_alternative) = match source {
            WebPlayerInput::AudioData(source) => {
                let bytes = Uint8Array::new_with_length(source.bytes.len() as u32);
                bytes.copy_from(&source.bytes);
                let parts = js_sys::Array::new();
                parts.push(&bytes);
                let properties = web_sys::BlobPropertyBag::new();
                properties.set_type(&source.mime_type);
                let blob =
                    web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &properties)
                        .map_err(playback_error)?;
                let object_url =
                    ObjectUrl(Url::create_object_url_with_blob(&blob).map_err(playback_error)?);
                (object_url.0.clone(), Some(object_url), None)
            }
            WebPlayerInput::Url(alternative) => {
                (alternative.url.clone(), None, Some(alternative.clone()))
            }
        };
        element.set_preload("auto");

        let element_for_loaded = element.clone();
        let runtime_for_loaded = runtime.clone();
        let url_source = selected_alternative.is_some();
        let dioxus_runtime_for_loaded = dioxus_runtime.clone();
        let on_loaded = Closure::wrap(Box::new(move || {
            dioxus_runtime_for_loaded.in_scope(dioxus_scope, || {
                let value = element_for_loaded.duration();
                let observations = media_observations(&element_for_loaded, url_source);
                with_current_attempt(&runtime_for_loaded, source_attempt, |runtime| {
                    if value.is_finite() && value > 0.0 {
                        runtime.authoritative_duration = Some(value);
                        duration.set(Duration::from_secs_f64(value));
                    } else {
                        runtime.authoritative_duration = None;
                    }
                    if url_source {
                        runtime.lifecycle.metadata_loaded();
                    } else {
                        runtime.lifecycle.loaded();
                    }
                    observations.apply(&mut runtime.lifecycle);
                    publish_lifecycle(&runtime.lifecycle, status, snapshot);
                });
            });
        }) as Box<dyn FnMut()>);

        let runtime_for_can_play = runtime.clone();
        let selected_for_error = selected_alternative.clone();
        let selected_for_can_play = selected_alternative;
        let dioxus_runtime_for_can_play = dioxus_runtime.clone();
        let element_for_can_play = element.clone();
        let on_can_play = Closure::wrap(Box::new(move || {
            dioxus_runtime_for_can_play.in_scope(dioxus_scope, || {
                let observations =
                    media_observations(&element_for_can_play, selected_for_can_play.is_some());
                with_current_attempt(&runtime_for_can_play, source_attempt, |runtime| {
                    if let Some(alternative) = selected_for_can_play.as_ref() {
                        runtime.lifecycle.url_playable(alternative.clone());
                        runtime.clear_alternative_failures();
                    } else {
                        runtime.lifecycle.playable();
                    }
                    observations.apply(&mut runtime.lifecycle);
                    publish_lifecycle(&runtime.lifecycle, status, snapshot);
                });
            });
        }) as Box<dyn FnMut()>);

        let element_for_time = element.clone();
        let runtime_for_time = runtime.clone();
        let dioxus_runtime_for_time = dioxus_runtime.clone();
        let on_time = Closure::wrap(Box::new(move || {
            dioxus_runtime_for_time.in_scope(dioxus_scope, || {
                let value = element_for_time.current_time();
                let Some(runtime) = runtime_for_time.upgrade() else {
                    return;
                };
                let operation_to_complete = {
                    let runtime_ref = runtime.borrow();
                    if !runtime_ref.is_current_attempt(source_attempt) {
                        return;
                    }
                    let Some(bounded) = runtime_ref.lifecycle.bounded().copied() else {
                        if runtime_ref.lifecycle.transport() != PlaybackTransport::Idle
                            && value.is_finite()
                            && value >= 0.0
                        {
                            position.set(Duration::from_secs_f64(value));
                        }
                        return;
                    };
                    if bounded.phase == BoundedPlaybackPhase::Completed {
                        position.set(Duration::from_secs_f64(bounded.range.end()));
                        return;
                    }
                    if bounded.phase == BoundedPlaybackPhase::Active
                        && value.is_finite()
                        && value >= bounded.range.end()
                    {
                        runtime_ref.active_bounded_operation()
                    } else {
                        if runtime_ref.lifecycle.transport() != PlaybackTransport::Idle
                            && value.is_finite()
                            && value >= 0.0
                        {
                            position.set(Duration::from_secs_f64(value));
                        }
                        None
                    }
                };
                if let Some(operation) = operation_to_complete {
                    complete_bounded_playback(
                        &runtime,
                        operation,
                        &element_for_time,
                        position,
                        status,
                        snapshot,
                    );
                }
            });
        }) as Box<dyn FnMut()>);

        let element_for_seeked = element.clone();
        let runtime_for_seeked = runtime.clone();
        let dioxus_runtime_for_seeked = dioxus_runtime.clone();
        let on_seeked = Closure::wrap(Box::new(move || {
            dioxus_runtime_for_seeked.in_scope(dioxus_scope, || {
                let actual = element_for_seeked.current_time();
                let Some(runtime) = runtime_for_seeked.upgrade() else {
                    return;
                };
                let should_play = {
                    let mut runtime_ref = runtime.borrow_mut();
                    if !runtime_ref.is_current_attempt(source_attempt) {
                        return;
                    }
                    let Some(pending) = runtime_ref.pending_bounded_seek else {
                        return;
                    };
                    if !runtime_ref.is_current_bounded(pending.operation)
                        || element_for_seeked.seeking()
                        || !actual.is_finite()
                        || (actual - pending.target).abs() > BOUNDED_SEEK_TOLERANCE_SECONDS
                    {
                        return;
                    }
                    runtime_ref.pending_bounded_seek = None;
                    position.set(Duration::from_secs_f64(pending.target));
                    if pending.resume_after {
                        if runtime_ref.lifecycle.request_play().is_err() {
                            runtime_ref
                                .lifecycle
                                .bounded_failed(BoundedPlaybackFailure::ActivationRejected);
                            false
                        } else {
                            runtime_ref.play_generation.advance();
                            runtime_ref.lifecycle.bounded_activating();
                            true
                        }
                    } else {
                        runtime_ref.lifecycle.bounded_paused();
                        false
                    }
                };
                publish_lifecycle(&runtime.borrow().lifecycle, status, snapshot);
                if should_play {
                    let _ = play_current_resource(&runtime, status, snapshot, position);
                }
            });
        }) as Box<dyn FnMut()>);

        let element_for_playing = element.clone();
        let runtime_for_playing = runtime.clone();
        let dioxus_runtime_for_playing = dioxus_runtime.clone();
        let on_playing = Closure::wrap(Box::new(move || {
            dioxus_runtime_for_playing.in_scope(dioxus_scope, || {
                with_current_attempt(&runtime_for_playing, source_attempt, |runtime| {
                    if matches!(
                        runtime.lifecycle.transport(),
                        PlaybackTransport::PlayPending | PlaybackTransport::Playing
                    ) && !element_for_playing.paused()
                        && !element_for_playing.ended()
                    {
                        runtime.lifecycle.playable();
                        publish_lifecycle(&runtime.lifecycle, status, snapshot);
                    }
                });
            });
        }) as Box<dyn FnMut()>);

        let element_for_waiting = element.clone();
        let runtime_for_waiting = runtime.clone();
        let dioxus_runtime_for_waiting = dioxus_runtime.clone();
        let on_waiting = Closure::wrap(Box::new(move || {
            dioxus_runtime_for_waiting.in_scope(dioxus_scope, || {
                with_current_attempt(&runtime_for_waiting, source_attempt, |runtime| {
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

        let element_for_load_start = element.clone();
        let runtime_for_load_start = runtime.clone();
        let dioxus_runtime_for_load_start = dioxus_runtime.clone();
        let on_load_start = Closure::wrap(Box::new(move || {
            dioxus_runtime_for_load_start.in_scope(dioxus_scope, || {
                let observations = MediaObservations::read(
                    &element_for_load_start,
                    url_source.then_some(PlaybackNetworkActivity::Loading),
                );
                publish_media_observations(
                    &runtime_for_load_start,
                    source_attempt,
                    observations,
                    status,
                    snapshot,
                );
            });
        }) as Box<dyn FnMut()>);

        let element_for_progress = element.clone();
        let runtime_for_progress = runtime.clone();
        let dioxus_runtime_for_progress = dioxus_runtime.clone();
        let on_progress = Closure::wrap(Box::new(move || {
            dioxus_runtime_for_progress.in_scope(dioxus_scope, || {
                let observations = media_observations(&element_for_progress, url_source);
                publish_media_observations(
                    &runtime_for_progress,
                    source_attempt,
                    observations,
                    status,
                    snapshot,
                );
            });
        }) as Box<dyn FnMut()>);

        let element_for_suspend = element.clone();
        let runtime_for_suspend = runtime.clone();
        let dioxus_runtime_for_suspend = dioxus_runtime.clone();
        let on_suspend = Closure::wrap(Box::new(move || {
            dioxus_runtime_for_suspend.in_scope(dioxus_scope, || {
                let observations = MediaObservations::read(
                    &element_for_suspend,
                    url_source.then_some(PlaybackNetworkActivity::Idle),
                );
                publish_media_observations(
                    &runtime_for_suspend,
                    source_attempt,
                    observations,
                    status,
                    snapshot,
                );
            });
        }) as Box<dyn FnMut()>);

        let element_for_stalled = element.clone();
        let runtime_for_stalled = runtime.clone();
        let dioxus_runtime_for_stalled = dioxus_runtime.clone();
        let on_stalled = Closure::wrap(Box::new(move || {
            dioxus_runtime_for_stalled.in_scope(dioxus_scope, || {
                let observations = MediaObservations::read(
                    &element_for_stalled,
                    url_source.then_some(PlaybackNetworkActivity::Stalled),
                );
                publish_media_observations(
                    &runtime_for_stalled,
                    source_attempt,
                    observations,
                    status,
                    snapshot,
                );
            });
        }) as Box<dyn FnMut()>);

        let element_for_duration_change = element.clone();
        let runtime_for_duration_change = runtime.clone();
        let dioxus_runtime_for_duration_change = dioxus_runtime.clone();
        let on_duration_change = Closure::wrap(Box::new(move || {
            dioxus_runtime_for_duration_change.in_scope(dioxus_scope, || {
                let value = element_for_duration_change.duration();
                let observations = media_observations(&element_for_duration_change, url_source);
                with_current_attempt(&runtime_for_duration_change, source_attempt, |runtime| {
                    if value.is_finite() && value > 0.0 {
                        runtime.authoritative_duration = Some(value);
                        duration.set(Duration::from_secs_f64(value));
                    } else {
                        runtime.authoritative_duration = None;
                    }
                    observations.apply(&mut runtime.lifecycle);
                    publish_lifecycle(&runtime.lifecycle, status, snapshot);
                });
            });
        }) as Box<dyn FnMut()>);

        let element_for_pause = element.clone();
        let runtime_for_pause = runtime.clone();
        let dioxus_runtime_for_pause = dioxus_runtime.clone();
        let on_pause = Closure::wrap(Box::new(move || {
            dioxus_runtime_for_pause.in_scope(dioxus_scope, || {
                with_current_attempt(&runtime_for_pause, source_attempt, |runtime| {
                    if element_for_pause.paused()
                        && runtime.lifecycle.transport() == PlaybackTransport::Playing
                    {
                        runtime.deadline_generation.advance();
                        runtime.lifecycle.paused();
                        runtime.lifecycle.bounded_paused();
                        publish_lifecycle(&runtime.lifecycle, status, snapshot);
                    }
                });
            });
        }) as Box<dyn FnMut()>);

        let element_for_ended = element.clone();
        let runtime_for_ended = runtime.clone();
        let dioxus_runtime_for_ended = dioxus_runtime.clone();
        let on_ended = Closure::wrap(Box::new(move || {
            dioxus_runtime_for_ended.in_scope(dioxus_scope, || {
                if !element_for_ended.ended() {
                    return;
                }
                let Some(runtime) = runtime_for_ended.upgrade() else {
                    return;
                };
                let operation = {
                    let runtime_ref = runtime.borrow();
                    if !runtime_ref.is_current_attempt(source_attempt) {
                        return;
                    }
                    runtime_ref.active_bounded_operation()
                };
                if let Some(operation) = operation {
                    complete_bounded_playback(
                        &runtime,
                        operation,
                        &element_for_ended,
                        position,
                        status,
                        snapshot,
                    );
                } else {
                    let mut runtime_ref = runtime.borrow_mut();
                    runtime_ref.lifecycle.ended();
                    publish_lifecycle(&runtime_ref.lifecycle, status, snapshot);
                }
            });
        }) as Box<dyn FnMut()>);

        let element_for_error = element.clone();
        let runtime_for_error = runtime;
        let on_error = Closure::wrap(Box::new(move || {
            let failure = media_source_failure(&element_for_error);
            let mut retry_attempt = None;
            dioxus_runtime.in_scope(dioxus_scope, || {
                with_current_attempt(&runtime_for_error, source_attempt, |runtime| {
                    if let SourceErrorAction::Retry {
                        attempt,
                        fallback_duration,
                    } = runtime.handle_source_error(selected_for_error.as_ref(), failure.clone())
                    {
                        position.set(Duration::ZERO);
                        duration.set(fallback_duration);
                        retry_attempt = Some(attempt);
                    }
                    publish_lifecycle(&runtime.lifecycle, status, snapshot);
                });
            });
            if let Some(retry_attempt) = retry_attempt {
                schedule_fallback(
                    runtime_for_error.clone(),
                    retry_attempt,
                    status,
                    snapshot,
                    position,
                    duration,
                    dioxus_runtime.clone(),
                    dioxus_scope,
                    rate,
                );
            }
        }) as Box<dyn FnMut()>);

        let listeners = vec![
            EventListener::new("loadstart", on_load_start),
            EventListener::new("loadedmetadata", on_loaded),
            EventListener::new("durationchange", on_duration_change),
            EventListener::new("canplay", on_can_play),
            EventListener::new("progress", on_progress),
            EventListener::new("suspend", on_suspend),
            EventListener::new("stalled", on_stalled),
            EventListener::new("seeked", on_seeked),
            EventListener::new("timeupdate", on_time),
            EventListener::new("playing", on_playing),
            EventListener::new("waiting", on_waiting),
            EventListener::new("pause", on_pause),
            EventListener::new("ended", on_ended),
            EventListener::new("error", on_error),
        ];
        for (registered, listener) in listeners.iter().enumerate() {
            if let Err(error) = add_listener(&element, listener.name, &listener.callback) {
                for listener in &listeners[..registered] {
                    remove_listener(&element, listener.name, &listener.callback);
                }
                return Err(error);
            }
        }

        let player = Self {
            element,
            graph_source,
            _object_url: object_url,
            listeners,
        };
        player.element.set_src(&source_url);
        player.element.load();
        Ok(player)
    }
}

struct MediaObservations {
    network: Option<PlaybackNetworkActivity>,
    buffered: Vec<PlaybackTimeRange>,
    seekable: Vec<PlaybackTimeRange>,
}

impl MediaObservations {
    fn read(element: &HtmlAudioElement, network: Option<PlaybackNetworkActivity>) -> Self {
        Self {
            network,
            buffered: collect_time_ranges(element.buffered()),
            seekable: collect_time_ranges(element.seekable()),
        }
    }

    fn apply(self, lifecycle: &mut PlaybackLifecycle) {
        if let Some(network) = self.network {
            lifecycle.network_observed(network);
        }
        lifecycle.ranges_changed(self.buffered, self.seekable);
    }
}

fn media_observations(element: &HtmlAudioElement, url_addressable: bool) -> MediaObservations {
    let network = if url_addressable {
        Some(match element.network_state() {
            NETWORK_IDLE => PlaybackNetworkActivity::Idle,
            NETWORK_LOADING => PlaybackNetworkActivity::Loading,
            _ => PlaybackNetworkActivity::Unknown,
        })
    } else {
        Some(PlaybackNetworkActivity::Inactive)
    };
    MediaObservations::read(element, network)
}

fn publish_media_observations(
    runtime: &Weak<RefCell<PlayerRuntime>>,
    source_attempt: SourceAttempt,
    observations: MediaObservations,
    status: Signal<PlaybackStatus>,
    snapshot: Signal<PlaybackSnapshot>,
) {
    with_current_attempt(runtime, source_attempt, |runtime| {
        observations.apply(&mut runtime.lifecycle);
        publish_lifecycle(&runtime.lifecycle, status, snapshot);
    });
}

fn collect_time_ranges(ranges: TimeRanges) -> Vec<PlaybackTimeRange> {
    (0..ranges.length())
        .filter_map(|index| {
            let start = Duration::try_from_secs_f64(ranges.start(index).ok()?).ok()?;
            let end = Duration::try_from_secs_f64(ranges.end(index).ok()?).ok()?;
            PlaybackTimeRange::new(start, end).ok()
        })
        .collect()
}

impl Drop for WebPlayer {
    fn drop(&mut self) {
        for listener in &self.listeners {
            remove_listener(&self.element, listener.name, &listener.callback);
        }
        let _ = self.element.pause();
        if let Some(source) = self.graph_source.take() {
            let _ = source.disconnect();
        }
        let _ = self.element.remove_attribute("src");
        self.element.load();
    }
}

struct EventListener {
    name: &'static str,
    callback: Closure<dyn FnMut()>,
}

impl EventListener {
    fn new(name: &'static str, callback: Closure<dyn FnMut()>) -> Self {
        Self { name, callback }
    }
}

fn configure_resource(runtime: &PlayerRuntime, rate: f64) {
    let Some(resource) = runtime.resource.as_ref() else {
        return;
    };
    resource.element.set_playback_rate(rate);
    resource.element.set_loop(runtime.lifecycle.repeat());
    apply_audibility(runtime);
}

fn apply_audibility(runtime: &PlayerRuntime) {
    let Some(resource) = runtime.resource.as_ref() else {
        return;
    };
    if resource.graph_source.is_some() {
        resource.element.set_muted(false);
        resource.element.set_volume(1.0);
        if let Some(graph) = runtime.graph.as_ref() {
            let gain = if runtime.lifecycle.muted() {
                0.0
            } else {
                runtime.lifecycle.audibility_level().value() as f32
            };
            graph.gain.gain().set_value(gain);
        }
    } else {
        resource.element.set_muted(runtime.lifecycle.muted());
        resource
            .element
            .set_volume(runtime.lifecycle.audibility_level().value());
    }
}

fn media_source_failure(element: &HtmlAudioElement) -> PlaybackSourceFailure {
    let code = element.error().map(|error| error.code());
    let error = |message| AudioError::new(AudioErrorKind::PlaybackFailure, message);
    match code {
        Some(2) => {
            PlaybackSourceFailure::Network(error("browser could not acquire the Playback Source"))
        }
        Some(3) => {
            PlaybackSourceFailure::Decode(error("browser could not decode the Playback Source"))
        }
        Some(4) => PlaybackSourceFailure::Unsupported(error(
            "browser does not support the Playback Source",
        )),
        _ => PlaybackSourceFailure::Unknown(error("browser could not load the Playback Source")),
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
