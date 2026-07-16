use dioxus_audio::playback::{PlaybackLifecycle, PlaybackStatus, clamp_seek};
use dioxus_audio::{AudioError, AudioErrorKind};

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
    playback.failed(error.clone());
    assert_eq!(playback.status(), &PlaybackStatus::Failed(error));
}

#[test]
fn seeking_away_from_the_end_preserves_the_requested_position() {
    let mut playback = PlaybackLifecycle::default();
    playback.loaded();
    playback.ended();

    playback.seeked(10.0, 30.0);
    assert_eq!(playback.status(), &PlaybackStatus::Paused);

    playback.seeked(30.0, 30.0);
    assert_eq!(playback.status(), &PlaybackStatus::Ended);
}
