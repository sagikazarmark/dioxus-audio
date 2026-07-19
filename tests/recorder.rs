use dioxus_audio::recorder::{
    CompletionDisposition, RecorderConstraintCapabilities, RecorderLifecycle, RecorderOptions,
    RecorderStatus, RecordingConstraint, RecordingConstraints, RecordingSourceSettings,
};
use dioxus_audio::{AudioError, AudioErrorKind};

#[test]
fn stopped_recording_completes_once_and_returns_to_idle() {
    let mut recorder = RecorderLifecycle::default();

    let session = recorder.start().unwrap();
    assert_eq!(recorder.status(), &RecorderStatus::RequestingPermission);
    assert!(recorder.started(session));
    assert_eq!(recorder.status(), &RecorderStatus::Recording);

    recorder.stop().unwrap();
    assert_eq!(recorder.status(), &RecorderStatus::Stopping);
    assert_eq!(
        recorder.begin_finalize(session),
        Some(CompletionDisposition::Save)
    );
    assert!(recorder.start().is_err());
    assert_eq!(recorder.status(), &RecorderStatus::Stopping);
    assert!(recorder.complete_finalize(session));
    assert_eq!(recorder.status(), &RecorderStatus::Idle);
    assert_eq!(recorder.begin_finalize(session), None);
    assert!(recorder.start().is_err());
    recorder.clear_completed();
    assert!(recorder.start().is_ok());
}

#[test]
fn cancelled_recording_is_discarded() {
    let mut recorder = RecorderLifecycle::default();
    let session = recorder.start().unwrap();
    recorder.started(session);

    recorder.cancel().unwrap();

    assert_eq!(
        recorder.begin_finalize(session),
        Some(CompletionDisposition::Discard)
    );
    assert!(recorder.complete_finalize(session));
}

#[test]
fn browser_initiated_stop_finalizes_the_partial_recording() {
    let mut recorder = RecorderLifecycle::default();
    let session = recorder.start().unwrap();
    recorder.started(session);

    assert_eq!(
        recorder.begin_finalize(session),
        Some(CompletionDisposition::Save)
    );
    assert_eq!(recorder.status(), &RecorderStatus::Stopping);
    assert!(recorder.stop().is_err());
    assert!(recorder.cancel().is_err());
    assert!(recorder.complete_finalize(session));
    assert_eq!(recorder.status(), &RecorderStatus::Idle);
}

#[test]
fn pause_and_resume_are_valid_only_during_recording() {
    let mut recorder = RecorderLifecycle::default();
    assert!(recorder.pause().is_err());

    let session = recorder.start().unwrap();
    recorder.started(session);
    recorder.pause().unwrap();
    assert_eq!(recorder.status(), &RecorderStatus::Paused);
    recorder.resume().unwrap();
    assert_eq!(recorder.status(), &RecorderStatus::Recording);
}

#[test]
fn stale_backend_events_cannot_replace_a_new_session() {
    let mut recorder = RecorderLifecycle::default();
    let stale = recorder.start().unwrap();
    recorder.cancel().unwrap();
    let current = recorder.start().unwrap();

    assert!(!recorder.started(stale));
    assert_eq!(recorder.status(), &RecorderStatus::RequestingPermission);
    assert!(recorder.started(current));
}

#[test]
fn backend_failures_are_observable_and_restartable() {
    let mut recorder = RecorderLifecycle::default();
    let session = recorder.start().unwrap();
    let error = AudioError::new(AudioErrorKind::PermissionDenied, "microphone denied");

    assert!(recorder.failed(session, error.clone()));
    assert_eq!(recorder.status(), &RecorderStatus::Failed(error));
    assert!(recorder.start().is_ok());
}

#[test]
fn recorder_options_reject_invalid_analysis_configuration() {
    let mut options = RecorderOptions::default();
    options.fft_size = 100;
    assert!(options.validate().is_err());

    let mut options = RecorderOptions::default();
    options.smoothing = 1.5;
    assert!(options.validate().is_err());

    let mut options = RecorderOptions::default();
    options.peak_interval = std::time::Duration::ZERO;
    assert!(options.validate().is_err());

    let mut recorder = RecorderLifecycle::default();
    let error = options.validate().unwrap_err();
    assert!(recorder.configuration_failed(error.clone()));
    assert_eq!(recorder.status(), &RecorderStatus::Failed(error));
}

#[test]
fn recorder_constraints_express_the_portable_startup_subset() {
    let constraints = RecordingConstraints {
        channel_count: Some(RecordingConstraint::Exact(1)),
        sample_rate: Some(RecordingConstraint::Ideal(48_000)),
        echo_cancellation: Some(RecordingConstraint::Ideal(false)),
        noise_suppression: Some(RecordingConstraint::Exact(false)),
        latency: Some(RecordingConstraint::Ideal(
            std::time::Duration::from_millis(20),
        )),
    };

    let snapshot = constraints.clone();
    let mut changed = constraints;
    changed.sample_rate = Some(RecordingConstraint::Exact(44_100));

    assert_eq!(
        snapshot.sample_rate,
        Some(RecordingConstraint::Ideal(48_000))
    );
    assert_eq!(
        snapshot.latency,
        Some(RecordingConstraint::Ideal(
            std::time::Duration::from_millis(20)
        ))
    );
    assert_ne!(snapshot, changed);
}

#[test]
fn recorder_capabilities_and_effective_settings_are_distinct_values() {
    let capabilities = RecorderConstraintCapabilities {
        channel_count: true,
        sample_rate: true,
        echo_cancellation: true,
        noise_suppression: false,
        latency: true,
    };
    let settings = RecordingSourceSettings {
        channel_count: Some(1),
        sample_rate: Some(48_000),
        echo_cancellation: Some(false),
        noise_suppression: None,
        latency: Some(std::time::Duration::from_millis(10)),
    };

    assert!(!capabilities.noise_suppression);
    assert_eq!(settings.noise_suppression, None);
    assert_eq!(settings.sample_rate, Some(48_000));
}

#[test]
fn exact_constraint_failures_preserve_the_rejected_constraint() {
    let error = AudioError::overconstrained("sampleRate", "sample rate is unavailable");

    assert_eq!(error.kind(), AudioErrorKind::Overconstrained);
    assert_eq!(error.overconstrained_constraint(), Some("sampleRate"));
    assert_eq!(error.message(), "sample rate is unavailable");
}

#[test]
fn overconstraint_failure_does_not_invent_missing_detail() {
    let error = AudioError::overconstrained("", "constraints are unavailable");

    assert_eq!(error.kind(), AudioErrorKind::Overconstrained);
    assert_eq!(error.overconstrained_constraint(), None);
}
