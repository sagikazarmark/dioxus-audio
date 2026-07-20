use std::time::Duration;

use dioxus::prelude::*;
use dioxus_audio::playback::{
    PlaybackAudibilityCapability, PlaybackAudibilityLevel, PlaybackGraphState, PlaybackLifecycle,
    PlaybackLoadingPolicy, PlaybackNetworkActivity, PlaybackOptions, PlaybackPlayFailure,
    PlaybackReadiness, PlaybackSource, PlaybackSourceAlternative, PlaybackSourceFailure,
    PlaybackSourceLifecycle, PlaybackStatus, PlaybackTimeRange, PlaybackTransport, clamp_seek,
    use_audio_player, use_audio_player_with_options,
};
use dioxus_audio::{AudioData, AudioError, AudioErrorKind};

#[test]
fn url_playback_source_validates_and_preserves_its_descriptor() {
    let first = PlaybackSourceAlternative::new("/media/tone.mp3")
        .unwrap()
        .with_media_type("audio/mpeg")
        .unwrap();
    let second = PlaybackSourceAlternative::new("/media/tone.wav")
        .unwrap()
        .with_media_type("audio/wav")
        .unwrap();
    let untyped = PlaybackSourceAlternative::new("/media/tone").unwrap();
    assert_eq!(second.url(), "/media/tone.wav");
    assert_eq!(second.media_type(), Some("audio/wav"));

    let source =
        PlaybackSource::url_alternatives(vec![first.clone(), second.clone(), untyped.clone()])
            .unwrap()
            .with_loading_policy(PlaybackLoadingPolicy::OnPlay);
    assert_eq!(source.loading_policy(), PlaybackLoadingPolicy::OnPlay);
    assert_eq!(source.alternatives(), Some(&[first, second, untyped][..]));

    let error = PlaybackSource::url_alternatives(Vec::new()).unwrap_err();
    assert_eq!(error.kind(), AudioErrorKind::InvalidConfiguration);

    let single = PlaybackSourceAlternative::new("/media/single.wav").unwrap();
    let source = PlaybackSource::url(single.clone());
    assert_eq!(source.alternatives(), Some(&[single][..]));

    for invalid in ["", "   ", "audio\nfile.wav"] {
        let error = PlaybackSourceAlternative::new(invalid).unwrap_err();
        assert_eq!(error.kind(), AudioErrorKind::InvalidConfiguration);
    }

    let error = PlaybackSourceAlternative::new("/media/tone.wav")
        .unwrap()
        .with_media_type("  ")
        .unwrap_err();
    assert_eq!(error.kind(), AudioErrorKind::InvalidConfiguration);
}

#[test]
fn audio_data_remains_an_eager_playback_source() {
    let source = PlaybackSource::from(AudioData::new(vec![1, 2, 3], "audio/wav"));

    assert_eq!(source.loading_policy(), PlaybackLoadingPolicy::Eager);
}

#[test]
fn graph_backing_is_an_immutable_owner_level_opt_in() {
    let direct = PlaybackLifecycle::default();
    assert_eq!(direct.graph_state(), PlaybackGraphState::NotRequested);
    assert_eq!(
        direct.audibility_capability(),
        PlaybackAudibilityCapability::BestEffortMediaElement
    );

    let graph_backed = PlaybackLifecycle::new(PlaybackOptions::graph_backed());
    assert_eq!(
        graph_backed.graph_state(),
        PlaybackGraphState::AwaitingSource
    );
    assert_eq!(
        graph_backed.audibility_capability(),
        PlaybackAudibilityCapability::EffectiveGraphGain
    );
}

#[test]
fn graph_state_is_orthogonal_and_persists_across_eligible_sources() {
    let mut playback = PlaybackLifecycle::new(PlaybackOptions::graph_backed());
    playback.set_muted(true);
    playback.set_audibility_level(0.35).unwrap();

    playback.loading();
    playback.graph_preparing();
    assert_eq!(playback.graph_state(), PlaybackGraphState::Preparing);
    assert_eq!(playback.transport(), PlaybackTransport::Idle);

    playback.loaded();
    playback.graph_suspended();
    assert_eq!(playback.graph_state(), PlaybackGraphState::Suspended);
    playback.request_play().unwrap();
    playback.graph_running();
    playback.playing();
    assert_eq!(playback.graph_state(), PlaybackGraphState::Running);
    assert_eq!(playback.transport(), PlaybackTransport::Playing);

    playback.unload();
    assert_eq!(playback.graph_state(), PlaybackGraphState::AwaitingSource);
    assert!(playback.muted());
    assert_eq!(playback.audibility_level().value(), 0.35);
    assert_eq!(
        playback.audibility_capability(),
        PlaybackAudibilityCapability::EffectiveGraphGain
    );

    playback.loading();
    playback.graph_preparing();
    playback.loaded();
    playback.graph_running();
    assert_eq!(playback.graph_state(), PlaybackGraphState::Running);
}

#[test]
fn graph_activation_and_later_interruption_are_recoverable() {
    let mut playback = PlaybackLifecycle::new(PlaybackOptions::graph_backed());
    playback.loading();
    playback.graph_preparing();
    playback.loaded();
    playback.graph_suspended();
    playback.request_play().unwrap();

    let error = AudioError::new(AudioErrorKind::PlaybackFailure, "interaction required");
    playback.graph_interaction_required(error.clone());
    assert_eq!(
        playback.graph_state(),
        PlaybackGraphState::InteractionRequired
    );
    assert_eq!(playback.transport(), PlaybackTransport::Paused);
    assert_eq!(
        playback.play_failure(),
        Some(&PlaybackPlayFailure::InteractionRequired(error.clone()))
    );
    playback.graph_running();
    assert_eq!(
        playback.graph_state(),
        PlaybackGraphState::InteractionRequired
    );
    playback.graph_suspended();
    assert_eq!(
        playback.graph_state(),
        PlaybackGraphState::InteractionRequired
    );

    playback.request_play().unwrap();
    assert_eq!(playback.graph_state(), PlaybackGraphState::Suspended);
    assert_eq!(playback.play_failure(), None);
    playback.graph_running();
    playback.playing();

    playback.graph_interaction_required(error.clone());
    assert_eq!(playback.transport(), PlaybackTransport::Paused);
    assert_eq!(
        playback.graph_state(),
        PlaybackGraphState::InteractionRequired
    );
    assert_eq!(
        playback.play_failure(),
        Some(&PlaybackPlayFailure::InteractionRequired(error))
    );
}

#[test]
fn terminal_graph_failure_degrades_only_gain_and_analysis() {
    let mut playback = PlaybackLifecycle::new(PlaybackOptions::graph_backed());
    playback.set_muted(true);
    playback.set_audibility_level(0.35).unwrap();
    playback.loading();
    playback.graph_preparing();
    playback.loaded();

    playback.graph_unavailable();
    assert_eq!(playback.graph_state(), PlaybackGraphState::Unavailable);
    assert_eq!(playback.source(), &PlaybackSourceLifecycle::Playable);
    assert_eq!(
        playback.audibility_capability(),
        PlaybackAudibilityCapability::BestEffortMediaElement
    );
    assert!(playback.muted());
    assert_eq!(playback.audibility_level().value(), 0.35);

    playback.request_play().unwrap();
    playback.playing();
    assert_eq!(playback.transport(), PlaybackTransport::Playing);
    playback.unload();
    assert_eq!(playback.graph_state(), PlaybackGraphState::Unavailable);

    playback.loading();
    playback.graph_preparing();
    assert_eq!(playback.graph_state(), PlaybackGraphState::Unavailable);
}

#[test]
fn graph_gain_capability_is_qualified_for_the_current_route() {
    let mut playback = PlaybackLifecycle::new(PlaybackOptions::graph_backed());

    playback.direct_audibility();
    assert_eq!(
        playback.audibility_capability(),
        PlaybackAudibilityCapability::BestEffortMediaElement
    );

    playback.graph_preparing();
    assert_eq!(
        playback.audibility_capability(),
        PlaybackAudibilityCapability::EffectiveGraphGain
    );

    playback.graph_unavailable();
    playback.graph_preparing();
    assert_eq!(
        playback.audibility_capability(),
        PlaybackAudibilityCapability::BestEffortMediaElement
    );
}

#[test]
fn on_play_loading_can_be_paused_without_cancelling_the_source() {
    let alternative = PlaybackSourceAlternative::new("/media/tone.wav").unwrap();
    let mut playback = PlaybackLifecycle::default();

    playback.dormant();
    assert_eq!(playback.source(), &PlaybackSourceLifecycle::Dormant);
    assert_eq!(playback.transport(), PlaybackTransport::Idle);
    assert_eq!(playback.readiness(), PlaybackReadiness::Unavailable);

    playback.request_play().unwrap();
    assert_eq!(playback.source(), &PlaybackSourceLifecycle::Loading);
    assert_eq!(playback.transport(), PlaybackTransport::PlayPending);
    assert_eq!(playback.readiness(), PlaybackReadiness::LoadingMetadata);

    playback.paused();
    assert_eq!(playback.source(), &PlaybackSourceLifecycle::Loading);
    assert_eq!(playback.transport(), PlaybackTransport::Idle);

    playback.metadata_loaded();
    assert_eq!(playback.selected_alternative(), None);
    playback.url_playable(alternative.clone());
    assert_eq!(playback.source(), &PlaybackSourceLifecycle::Playable);
    assert_eq!(playback.transport(), PlaybackTransport::Idle);
    assert_eq!(playback.readiness(), PlaybackReadiness::Playable);
    assert_eq!(playback.selected_alternative(), Some(&alternative));

    let failure = PlaybackSourceFailure::Decode(AudioError::new(
        AudioErrorKind::PlaybackFailure,
        "browser could not decode the source",
    ));
    playback.source_failed(failure.clone());
    assert_eq!(playback.source(), &PlaybackSourceLifecycle::Failed);
    assert_eq!(playback.source_failure(), Some(&failure));
    assert_eq!(playback.selected_alternative(), Some(&alternative));

    playback.unload();
    assert_eq!(playback.source(), &PlaybackSourceLifecycle::Empty);
    assert_eq!(playback.source_failure(), None);
    assert_eq!(playback.selected_alternative(), None);
}

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
    assert_eq!(playback.source(), &PlaybackSourceLifecycle::Failed);
    assert_eq!(
        playback.source_failure(),
        Some(&PlaybackSourceFailure::Unknown(error.clone()))
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
fn network_and_range_observations_are_orthogonal_and_may_shrink() {
    let mut playback = PlaybackLifecycle::default();
    playback.loaded();
    playback.request_play().unwrap();
    playback.playing();
    playback.waiting();

    playback.network_observed(PlaybackNetworkActivity::Stalled);
    playback.ranges_changed(
        [
            PlaybackTimeRange::new(Duration::from_secs(5), Duration::from_secs(10)).unwrap(),
            PlaybackTimeRange::new(Duration::ZERO, Duration::from_secs(6)).unwrap(),
            PlaybackTimeRange::new(Duration::from_secs(10), Duration::from_secs(12)).unwrap(),
            PlaybackTimeRange::new(Duration::from_secs(15), Duration::from_secs(20)).unwrap(),
        ],
        [PlaybackTimeRange::new(Duration::ZERO, Duration::from_secs(30)).unwrap()],
    );

    assert_eq!(playback.transport(), PlaybackTransport::Playing);
    assert_eq!(playback.readiness(), PlaybackReadiness::Waiting);
    assert_eq!(
        playback.network_activity(),
        PlaybackNetworkActivity::Stalled
    );
    assert_eq!(
        &*playback.snapshot().buffered,
        &[
            PlaybackTimeRange::new(Duration::ZERO, Duration::from_secs(12)).unwrap(),
            PlaybackTimeRange::new(Duration::from_secs(15), Duration::from_secs(20)).unwrap(),
        ]
    );
    assert_eq!(
        &*playback.snapshot().seekable,
        &[PlaybackTimeRange::new(Duration::ZERO, Duration::from_secs(30)).unwrap()]
    );

    playback.ranges_changed(
        [PlaybackTimeRange::new(Duration::from_secs(2), Duration::from_secs(3)).unwrap()],
        [],
    );

    assert_eq!(
        &*playback.snapshot().buffered,
        &[PlaybackTimeRange::new(Duration::from_secs(2), Duration::from_secs(3)).unwrap()]
    );
    assert!(playback.snapshot().seekable.is_empty());

    assert!(PlaybackTimeRange::new(Duration::ZERO, Duration::ZERO).is_err());
    assert!(PlaybackTimeRange::new(Duration::from_secs(2), Duration::from_secs(1)).is_err());
}

#[test]
fn network_and_range_observations_are_scoped_to_the_current_source() {
    let range = PlaybackTimeRange::new(Duration::ZERO, Duration::from_secs(10)).unwrap();
    let mut playback = PlaybackLifecycle::default();
    playback.loading();
    playback.network_observed(PlaybackNetworkActivity::Loading);
    playback.ranges_changed([range], [range]);

    playback.loading();
    assert_eq!(
        playback.network_activity(),
        PlaybackNetworkActivity::Inactive
    );
    assert!(playback.snapshot().buffered.is_empty());
    assert!(playback.snapshot().seekable.is_empty());

    playback.network_observed(PlaybackNetworkActivity::Stalled);
    playback.ranges_changed([range], [range]);
    playback.source_failed(PlaybackSourceFailure::Network(AudioError::new(
        AudioErrorKind::PlaybackFailure,
        "source failed",
    )));
    assert_eq!(
        playback.network_activity(),
        PlaybackNetworkActivity::Inactive
    );
    assert!(playback.snapshot().buffered.is_empty());
    assert!(playback.snapshot().seekable.is_empty());
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
    assert_eq!(playback.status(), &PlaybackStatus::Paused);
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
fn mute_changes_audibility_without_changing_transport() {
    let mut playback = PlaybackLifecycle::default();
    assert!(!playback.muted());
    assert_eq!(playback.audibility_level(), PlaybackAudibilityLevel::FULL);
    assert_eq!(
        playback.audibility_capability(),
        PlaybackAudibilityCapability::BestEffortMediaElement
    );

    playback.loaded();
    playback.request_play().unwrap();
    playback.playing();
    let transport = playback.transport();

    playback.set_muted(true);

    assert!(playback.muted());
    assert_eq!(playback.transport(), transport);
    assert_eq!(playback.status(), &PlaybackStatus::Playing);
    assert_eq!(playback.audibility_level(), PlaybackAudibilityLevel::FULL);

    playback.toggle_muted();
    assert!(!playback.muted());
}

#[test]
fn audibility_level_rejects_invalid_values_and_survives_source_lifecycle() {
    let mut playback = PlaybackLifecycle::default();
    playback.set_audibility_level(0.35).unwrap();
    playback.set_muted(true);

    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.01, 1.01] {
        assert!(playback.set_audibility_level(invalid).is_err());
        assert_eq!(playback.audibility_level().value(), 0.35);
    }

    playback.loading();
    playback.loaded();
    playback.failed(AudioError::new(
        AudioErrorKind::PlaybackFailure,
        "source failed",
    ));
    playback.unload();

    assert!(playback.muted());
    assert_eq!(playback.audibility_level().value(), 0.35);

    playback.set_muted(false);
    playback.set_audibility_level(0.0).unwrap();
    assert!(!playback.muted(), "level zero is not the mute preference");
    playback.set_audibility_level(1.0).unwrap();
    assert_eq!(playback.audibility_level(), PlaybackAudibilityLevel::FULL);
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
        let source = use_signal(|| None::<PlaybackSource>);
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

#[test]
fn graph_backed_playback_is_hydration_neutral_while_awaiting_a_source() {
    fn app() -> Element {
        let source = use_signal(|| None::<PlaybackSource>);
        let player = use_audio_player_with_options(
            source.into(),
            Duration::from_secs(30),
            PlaybackOptions::graph_backed(),
        );
        let snapshot = player.snapshot()();

        rsx! {
            output { "{snapshot.source:?}/{snapshot.transport:?}/{snapshot.graph:?}" }
        }
    }

    let mut vdom = VirtualDom::new(app);
    vdom.rebuild_in_place();
    let html = dioxus_ssr::render(&vdom);

    assert!(html.contains("Empty/Idle/AwaitingSource"));
}
