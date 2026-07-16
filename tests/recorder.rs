use dioxus_audio::recorder::{
    CompletionDisposition, RecorderLifecycle, RecorderOptions, RecorderStatus,
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
