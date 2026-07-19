use std::time::Duration;

use dioxus::prelude::*;
use dioxus_audio::playback::{
    PlaybackLifecycle, PlaybackPlayFailure, PlaybackReadiness, PlaybackSourceLifecycle,
    PlaybackStatus, PlaybackTransport, clamp_seek, use_audio_player,
};
use dioxus_audio::{AudioData, AudioError, AudioErrorKind};

#[test]
fn seeking_is_clamped_to_a_finite_timeline() {
    assert_eq!(clamp_seek(-2.0, 30.0), 0.0);
    assert_eq!(clamp_seek(40.0, 30.0), 30.0);
    assert_eq!(clamp_seek(f64::NAN, 30.0), 0.0);
    assert_eq!(clamp_seek(12.5, f64::NAN), 0.0);
}

#[test]
fn playback_waits_for_browser_events_and_surfaces_rejection() {
    let mut playback = PlaybackLifecycle::default();
    playback.loaded();
    playback.request_play().unwrap();
    assert_eq!(playback.status(), &PlaybackStatus::Ready);

    playback.playing();
    assert_eq!(playback.status(), &PlaybackStatus::Playing);
    playback.paused();
    assert_eq!(playback.status(), &PlaybackStatus::Paused);

    let error = AudioError::new(AudioErrorKind::PlaybackFailure, "autoplay rejected");
    playback.request_play().unwrap();
    playback.play_rejected(PlaybackPlayFailure::Unknown(error.clone()));
    assert_eq!(playback.status(), &PlaybackStatus::Failed(error));
}

#[test]
fn playback_reports_loading_and_pending_play_as_orthogonal_state() {
    let mut playback = PlaybackLifecycle::default();
    assert_eq!(playback.source(), &PlaybackSourceLifecycle::Empty);
    assert_eq!(playback.transport(), PlaybackTransport::Idle);
    assert_eq!(playback.readiness(), PlaybackReadiness::Unavailable);

    playback.loading();
    assert_eq!(playback.source(), &PlaybackSourceLifecycle::Loading);
    assert_eq!(playback.transport(), PlaybackTransport::Idle);
    assert_eq!(playback.readiness(), PlaybackReadiness::LoadingMetadata);

    playback.loaded();
    playback.request_play().unwrap();
    assert_eq!(playback.source(), &PlaybackSourceLifecycle::Playable);
    assert_eq!(playback.transport(), PlaybackTransport::PlayPending);
    assert_eq!(playback.readiness(), PlaybackReadiness::Metadata);
    assert_eq!(playback.status(), &PlaybackStatus::Ready);

    playback.playing();
    assert_eq!(playback.transport(), PlaybackTransport::Playing);
    assert_eq!(playback.readiness(), PlaybackReadiness::Playable);
    assert_eq!(playback.status(), &PlaybackStatus::Playing);
}

#[test]
fn rejected_play_is_recoverable_without_failing_the_source() {
    let mut playback = PlaybackLifecycle::default();
    playback.loaded();
    playback.request_play().unwrap();
    playback.waiting();

    let error = AudioError::new(AudioErrorKind::PlaybackFailure, "interaction required");
    let failure = PlaybackPlayFailure::InteractionRequired(error.clone());
    playback.play_rejected(failure.clone());

    assert_eq!(playback.source(), &PlaybackSourceLifecycle::Playable);
    assert_eq!(playback.transport(), PlaybackTransport::Paused);
    assert_eq!(playback.readiness(), PlaybackReadiness::Metadata);
    assert_eq!(playback.play_failure(), Some(&failure));
    assert_eq!(playback.status(), &PlaybackStatus::Failed(error));

    playback.request_play().unwrap();
    assert_eq!(playback.transport(), PlaybackTransport::PlayPending);
    assert_eq!(playback.play_failure(), None);
    assert_eq!(playback.status(), &PlaybackStatus::Ready);
}

#[test]
fn waiting_and_terminal_source_failure_do_not_contradict_transport() {
    let mut playback = PlaybackLifecycle::default();
    playback.loaded();
    playback.request_play().unwrap();
    playback.playing();

    playback.waiting();
    assert_eq!(playback.transport(), PlaybackTransport::Playing);
    assert_eq!(playback.readiness(), PlaybackReadiness::Waiting);
    assert_eq!(playback.status(), &PlaybackStatus::Playing);

    playback.playable();
    assert_eq!(playback.transport(), PlaybackTransport::Playing);
    assert_eq!(playback.readiness(), PlaybackReadiness::Playable);

    let error = AudioError::new(AudioErrorKind::PlaybackFailure, "source failed");
    playback.failed(error.clone());
    assert_eq!(
        playback.source(),
        &PlaybackSourceLifecycle::Failed(error.clone())
    );
    assert_eq!(playback.transport(), PlaybackTransport::Idle);
    assert_eq!(playback.readiness(), PlaybackReadiness::Unavailable);
    assert_eq!(playback.play_failure(), None);
    assert_eq!(playback.status(), &PlaybackStatus::Failed(error));

    playback.unload();
    assert_eq!(playback.snapshot(), &Default::default());
    assert_eq!(playback.status(), &PlaybackStatus::Empty);
}

#[test]
fn paused_playback_ignores_a_superseded_play_confirmation() {
    let mut playback = PlaybackLifecycle::default();
    playback.loaded();
    playback.request_play().unwrap();
    playback.paused();

    playback.playing();
    playback.ended();

    assert_eq!(playback.transport(), PlaybackTransport::Paused);
    assert_eq!(playback.status(), &PlaybackStatus::Ready);
}

#[test]
fn play_cannot_be_requested_while_pending_or_playing() {
    let mut playback = PlaybackLifecycle::default();
    playback.loaded();

    playback.request_play().unwrap();
    assert!(playback.request_play().is_err());
    assert_eq!(playback.transport(), PlaybackTransport::PlayPending);

    playback.playing();
    assert!(playback.request_play().is_err());
    assert_eq!(playback.transport(), PlaybackTransport::Playing);
}

#[test]
fn stop_resets_every_non_idle_playback_state() {
    for initial_transport in [
        PlaybackTransport::PlayPending,
        PlaybackTransport::Playing,
        PlaybackTransport::Paused,
        PlaybackTransport::Ended,
    ] {
        let mut playback = PlaybackLifecycle::default();
        playback.loaded();
        playback.request_play().unwrap();
        if initial_transport != PlaybackTransport::PlayPending {
            playback.playing();
        }
        if initial_transport == PlaybackTransport::Paused {
            playback.paused();
        } else if initial_transport == PlaybackTransport::Ended {
            playback.ended();
        }
        let readiness = playback.readiness();

        playback.stop().unwrap();

        assert_eq!(
            playback.status(),
            &PlaybackStatus::Ready,
            "status after stopping {initial_transport:?}"
        );
        assert_eq!(playback.source(), &PlaybackSourceLifecycle::Playable);
        assert_eq!(playback.transport(), PlaybackTransport::Idle);
        assert_eq!(playback.readiness(), readiness);
        assert_eq!(playback.play_failure(), None);
    }
}

#[test]
fn stopped_playback_ignores_superseded_transport_outcomes() {
    let mut playback = PlaybackLifecycle::default();
    playback.loaded();
    playback.request_play().unwrap();
    playback.stop().unwrap();
    let stopped = playback.snapshot().clone();

    playback.playing();
    playback.paused();
    playback.ended();
    playback.play_rejected(PlaybackPlayFailure::Unknown(AudioError::new(
        AudioErrorKind::PlaybackFailure,
        "late rejection",
    )));

    assert_eq!(playback.snapshot(), &stopped);
    assert_eq!(playback.status(), &PlaybackStatus::Ready);
}

#[test]
fn repeat_preference_survives_source_replacement_and_unload() {
    let mut playback = PlaybackLifecycle::default();
    assert!(!playback.repeat());

    playback.set_repeat(true);
    playback.loading();
    playback.loaded();
    playback.stop().unwrap();
    assert!(playback.repeat());

    playback.unload();
    assert!(playback.repeat());

    playback.toggle_repeat();
    assert!(!playback.repeat());
}

#[test]
fn seeking_away_from_the_end_preserves_the_requested_position() {
    let mut playback = PlaybackLifecycle::default();
    playback.loaded();
    playback.request_play().unwrap();
    playback.playing();
    playback.ended();

    playback.seeked(10.0, 30.0);
    assert_eq!(playback.status(), &PlaybackStatus::Paused);

    playback.seeked(30.0, 30.0);
    assert_eq!(playback.status(), &PlaybackStatus::Ended);
}

#[test]
fn unsupported_playback_snapshot_is_neutral_for_server_rendering() {
    fn app() -> Element {
        let source = use_signal(|| None::<AudioData>);
        let player = use_audio_player(source.into(), Duration::from_secs(30));
        let snapshot = player.snapshot()();

        rsx! {
            output {
                "{snapshot.source:?}/{snapshot.transport:?}/{snapshot.readiness:?}"
            }
        }
    }

    let mut vdom = VirtualDom::new(app);
    vdom.rebuild_in_place();
    let html = dioxus_ssr::render(&vdom);

    assert!(html.contains("Empty/Idle/Unavailable"));
}
