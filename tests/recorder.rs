use dioxus_audio::recorder::{
    CompletionDisposition, RecorderConstraintCapabilities, RecorderLifecycle, RecorderOptions,
    RecorderStatus, RecordingChunkDelivery, RecordingConstraint, RecordingConstraints,
    RecordingOutcome, RecordingSourceSettings,
};
use dioxus_audio::{AudioError, AudioErrorKind, RecordingChunk};

#[test]
fn stopped_recording_completes_once_and_returns_to_idle() {
    let mut recorder = RecorderLifecycle::default();

    let recording_id = recorder.start().unwrap();
    assert_eq!(recorder.status(), &RecorderStatus::Preparing);
    assert!(recorder.started(recording_id));
    assert_eq!(recorder.status(), &RecorderStatus::Recording);

    recorder.stop().unwrap();
    assert_eq!(recorder.status(), &RecorderStatus::Stopping);
    assert_eq!(
        recorder.begin_finalize(recording_id),
        Some(CompletionDisposition::Save)
    );
    assert!(recorder.start().is_err());
    assert_eq!(recorder.status(), &RecorderStatus::Stopping);
    assert!(recorder.complete_finalize(recording_id));
    assert_eq!(recorder.status(), &RecorderStatus::Idle);
    assert_eq!(recorder.begin_finalize(recording_id), None);
    assert!(recorder.start().is_err());
    recorder.clear_completed();
    assert!(recorder.start().is_ok());
}

#[test]
fn cancelled_recording_is_discarded() {
    let mut recorder = RecorderLifecycle::default();
    let recording_id = recorder.start().unwrap();
    recorder.started(recording_id);

    recorder.cancel().unwrap();

    assert_eq!(
        recorder.begin_finalize(recording_id),
        Some(CompletionDisposition::Discard)
    );
    assert!(recorder.complete_finalize(recording_id));
}

#[test]
fn browser_initiated_stop_finalizes_the_partial_recording() {
    let mut recorder = RecorderLifecycle::default();
    let recording_id = recorder.start().unwrap();
    recorder.started(recording_id);

    assert_eq!(
        recorder.begin_finalize(recording_id),
        Some(CompletionDisposition::Save)
    );
    assert_eq!(recorder.status(), &RecorderStatus::Stopping);
    assert!(recorder.stop().is_err());
    assert!(recorder.cancel().is_err());
    assert!(recorder.complete_finalize(recording_id));
    assert_eq!(recorder.status(), &RecorderStatus::Idle);
}

#[test]
fn pause_and_resume_are_valid_only_during_recording() {
    let mut recorder = RecorderLifecycle::default();
    assert!(recorder.pause().is_err());

    let recording_id = recorder.start().unwrap();
    recorder.started(recording_id);
    recorder.pause().unwrap();
    assert_eq!(recorder.status(), &RecorderStatus::Paused);
    recorder.resume().unwrap();
    assert_eq!(recorder.status(), &RecorderStatus::Recording);
}

#[test]
fn stale_backend_events_cannot_replace_a_new_recording() {
    let mut recorder = RecorderLifecycle::default();
    let stale = recorder.start().unwrap();
    recorder.cancel().unwrap();
    let current = recorder.start().unwrap();

    assert!(!recorder.started(stale));
    assert_eq!(recorder.status(), &RecorderStatus::Preparing);
    assert!(recorder.started(current));
}

#[test]
fn recording_chunks_keep_identity_and_sequence_across_pause_resume_and_stop() {
    let mut recorder = RecorderLifecycle::default();
    let recording_id = recorder.start().unwrap();
    recorder.started(recording_id);

    let first = RecordingChunk {
        recording_id,
        sequence: recorder.next_chunk_sequence(recording_id).unwrap(),
        bytes: vec![1, 2, 3],
        media_type: "audio/webm;codecs=opus".to_string(),
    };
    recorder.pause().unwrap();
    recorder.resume().unwrap();
    let second = RecordingChunk {
        recording_id,
        sequence: recorder.next_chunk_sequence(recording_id).unwrap(),
        bytes: vec![4, 5],
        media_type: "audio/webm;codecs=opus".to_string(),
    };
    recorder.stop().unwrap();
    let final_chunk = RecordingChunk {
        recording_id,
        sequence: recorder.next_chunk_sequence(recording_id).unwrap(),
        bytes: vec![6],
        media_type: "audio/webm;codecs=opus".to_string(),
    };

    assert_eq!(first.sequence, 0);
    assert_eq!(second.sequence, 1);
    assert_eq!(final_chunk.sequence, 2);
    assert_eq!(first.recording_id, second.recording_id);
    assert_eq!(second.recording_id, final_chunk.recording_id);
    assert_eq!(first.bytes, vec![1, 2, 3]);
    assert_eq!(first.media_type, "audio/webm;codecs=opus");

    assert_eq!(
        recorder.begin_finalize(recording_id),
        Some(CompletionDisposition::Save)
    );
    assert!(recorder.complete_finalize(recording_id));
    assert_eq!(recorder.next_chunk_sequence(recording_id), None);
}

#[test]
fn best_effort_chunk_boundaries_preserve_identity_and_sequence_while_active_or_paused() {
    let mut recorder = RecorderLifecycle::default();
    assert!(recorder.request_chunk_boundary().is_err());

    let recording_id = recorder.start().unwrap();
    recorder.started(recording_id);

    recorder.request_chunk_boundary().unwrap();
    assert_eq!(recorder.next_chunk_sequence(recording_id), Some(0));

    recorder.pause().unwrap();
    recorder.request_chunk_boundary().unwrap();
    assert_eq!(recorder.next_chunk_sequence(recording_id), Some(1));

    recorder.resume().unwrap();
    recorder.stop().unwrap();
    assert!(recorder.request_chunk_boundary().is_err());
}

#[test]
fn discard_suppresses_chunks_and_a_restart_gets_new_identity_and_sequence() {
    let mut recorder = RecorderLifecycle::default();
    let discarded_id = recorder.start().unwrap();
    recorder.started(discarded_id);
    assert_eq!(recorder.next_chunk_sequence(discarded_id), Some(0));

    recorder.cancel().unwrap();

    assert_eq!(recorder.next_chunk_sequence(discarded_id), None);
    assert_eq!(
        recorder.begin_finalize(discarded_id),
        Some(CompletionDisposition::Discard)
    );
    assert!(recorder.complete_finalize(discarded_id));

    let restarted_id = recorder.start().unwrap();
    assert_ne!(restarted_id, discarded_id);
    recorder.started(restarted_id);
    assert_eq!(recorder.next_chunk_sequence(restarted_id), Some(0));
    assert_eq!(recorder.next_chunk_sequence(discarded_id), None);
}

#[test]
fn recording_outcomes_expose_the_recording_identity() {
    let mut recorder = RecorderLifecycle::default();
    let recording_id = recorder.start().unwrap();
    let error = AudioError::new(AudioErrorKind::RecorderFailure, "encoder stopped");

    let completed = RecordingOutcome::Completed(recording_id);
    let discarded = RecordingOutcome::Discarded(recording_id);
    let failed = RecordingOutcome::Failed {
        recording_id,
        error: error.clone(),
    };

    assert_eq!(completed.recording_id(), recording_id);
    assert_eq!(discarded.recording_id(), recording_id);
    assert_eq!(failed.recording_id(), recording_id);
    assert_eq!(
        failed,
        RecordingOutcome::Failed {
            recording_id,
            error,
        }
    );
}

#[test]
fn backend_failures_are_observable_and_restartable() {
    let mut recorder = RecorderLifecycle::default();
    let recording_id = recorder.start().unwrap();
    let error = AudioError::new(AudioErrorKind::PermissionDenied, "microphone denied");

    assert!(recorder.failed(recording_id, error.clone()));
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

    let mut options = RecorderOptions::default();
    options.chunk_delivery = Some(RecordingChunkDelivery::new(
        std::time::Duration::ZERO,
        |_| {},
    ));
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
