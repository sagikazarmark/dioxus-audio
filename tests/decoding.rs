use std::future::Future;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::time::Duration;

use dioxus_audio::AudioData;
use dioxus_audio::decoding::{
    DecodeErrorKind, DecodeOptions, DecodedAudio, DecodedAudioError, decode_audio_data,
};

#[test]
fn decoded_audio_preserves_planar_channels_and_effective_metadata() {
    let audio = DecodedAudio::from_planar(vec![-1.25, 0.0, 1.25, 0.5, -0.5, 0.25], 2, 4.0).unwrap();

    assert_eq!(audio.channel_count(), 2);
    assert_eq!(audio.frame_count(), 3);
    assert_eq!(audio.sample_rate(), 4.0);
    assert_eq!(audio.duration(), Duration::from_millis(750));
    assert_eq!(audio.channel(0), Some(&[-1.25, 0.0, 1.25][..]));
    assert_eq!(audio.channel(1), Some(&[0.5, -0.5, 0.25][..]));
    assert_eq!(audio.channel(2), None);
    assert_eq!(audio.channels().collect::<Vec<_>>().len(), 2);

    let clone = audio.clone();
    assert_eq!(clone.channel(0), audio.channel(0));
}

#[test]
fn decoded_audio_rejects_invalid_layouts_without_repairing_them() {
    assert_eq!(
        DecodedAudio::from_planar(vec![0.0], 0, 48_000.0).unwrap_err(),
        DecodedAudioError::NoChannels
    );
    assert_eq!(
        DecodedAudio::from_planar(vec![], 1, 48_000.0).unwrap_err(),
        DecodedAudioError::NoFrames
    );
    assert_eq!(
        DecodedAudio::from_planar(vec![0.0, 0.5, 1.0], 2, 48_000.0).unwrap_err(),
        DecodedAudioError::MisalignedSamples {
            samples: 3,
            channels: 2,
        }
    );

    for sample_rate in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert_eq!(
            DecodedAudio::from_planar(vec![0.0], 1, sample_rate).unwrap_err(),
            DecodedAudioError::InvalidSampleRate
        );
    }
}

#[test]
fn decode_options_apply_the_default_and_explicit_rust_copy_limits() {
    assert_eq!(
        DecodeOptions::default().max_decoded_bytes(),
        128 * 1024 * 1024
    );
    assert_eq!(
        DecodeOptions::default()
            .with_max_decoded_bytes(64 * 1024 * 1024)
            .max_decoded_bytes(),
        64 * 1024 * 1024
    );
    assert_eq!(
        DecodeOptions::default()
            .with_max_decoded_bytes(0)
            .max_decoded_bytes(),
        0
    );
}

#[test]
fn decode_options_check_size_arithmetic_and_report_exact_limit_details() {
    let options = DecodeOptions::default();
    let boundary_frames = 16_777_216;

    assert_eq!(
        options.check_decoded_size(2, boundary_frames).unwrap(),
        128 * 1024 * 1024
    );

    let error = options
        .check_decoded_size(2, boundary_frames + 1)
        .unwrap_err();
    assert_eq!(error.kind(), DecodeErrorKind::ResourceLimit);
    assert_eq!(error.required_bytes(), Some(128 * 1024 * 1024 + 8));
    assert_eq!(error.configured_bytes(), Some(128 * 1024 * 1024));

    assert_eq!(
        options
            .with_max_decoded_bytes(128 * 1024 * 1024 + 8)
            .check_decoded_size(2, boundary_frames + 1)
            .unwrap(),
        128 * 1024 * 1024 + 8
    );
    assert_eq!(
        options
            .with_max_decoded_bytes(u64::MAX)
            .check_decoded_size(u64::MAX, 2)
            .unwrap_err()
            .kind(),
        DecodeErrorKind::Backend
    );
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
#[test]
fn complete_file_decode_reports_an_unsupported_target_portably() {
    let future = decode_audio_data(
        AudioData::new(vec![0, 1, 2, 3], "audio/example"),
        DecodeOptions::default(),
    );
    let result = poll_once(future);

    let Poll::Ready(Err(error)) = result else {
        panic!("unsupported-target decoding must resolve immediately");
    };
    assert_eq!(error.kind(), DecodeErrorKind::UnsupportedPlatform);
    assert_eq!(error.required_bytes(), None);
    assert_eq!(error.configured_bytes(), None);
}

fn poll_once<F: Future>(future: F) -> Poll<F::Output> {
    struct NoopWake;

    impl Wake for NoopWake {
        fn wake(self: Arc<Self>) {}
    }

    let waker = Waker::from(Arc::new(NoopWake));
    let mut context = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    future.as_mut().poll(&mut context)
}
