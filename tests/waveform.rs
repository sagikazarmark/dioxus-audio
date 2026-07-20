use std::time::Duration;

use dioxus_audio::waveform::{
    AmplitudeMode, AmplitudeSlice, SignedEnvelope, WaveformData, WaveformError, WaveformLevel,
};

#[test]
fn magnitude_data_preserves_multichannel_buckets_and_snapshot_identity() {
    let data = WaveformData::from_magnitudes(
        Duration::from_secs(4),
        2,
        vec![WaveformLevel::new(
            Duration::from_secs(1),
            vec![0.1, 0.2, 0.3, 0.4, 0.8, 0.7, 0.6, 0.5],
        )],
    )
    .unwrap();

    assert_eq!(data.mode(), AmplitudeMode::Magnitude);
    assert_eq!(data.duration(), Duration::from_secs(4));
    assert_eq!(data.channel_count(), 2);
    assert_eq!(data.resolution_count(), 1);
    assert_eq!(
        data.resolution(0).unwrap().bucket_span().exact_duration(),
        Some(Duration::from_secs(1))
    );
    assert_eq!(data.resolution(0).unwrap().buckets_per_channel(), 4);
    assert_eq!(data.clone(), data);

    let independently_constructed = WaveformData::from_magnitudes(
        Duration::from_secs(4),
        2,
        vec![WaveformLevel::new(
            Duration::from_secs(1),
            vec![0.1, 0.2, 0.3, 0.4, 0.8, 0.7, 0.6, 0.5],
        )],
    )
    .unwrap();
    assert_ne!(independently_constructed, data);

    let view = data
        .select(Duration::ZERO..Duration::from_secs(4), 4)
        .unwrap();
    assert_eq!(
        view.channel(0),
        Some(AmplitudeSlice::Magnitudes(&[0.1, 0.2, 0.3, 0.4]))
    );
    assert_eq!(
        view.channel(1),
        Some(AmplitudeSlice::Magnitudes(&[0.8, 0.7, 0.6, 0.5]))
    );
    assert_eq!(view.channel(2), None);
}

#[test]
fn magnitude_construction_rejects_invalid_structure_without_repairing_it() {
    assert_eq!(
        WaveformData::from_magnitudes(
            Duration::ZERO,
            1,
            vec![WaveformLevel::new(Duration::from_secs(1), vec![0.5])],
        )
        .unwrap_err(),
        WaveformError::ZeroDuration
    );
    assert_eq!(
        WaveformData::from_magnitudes(
            Duration::from_secs(1),
            0,
            vec![WaveformLevel::new(Duration::from_secs(1), vec![0.5])],
        )
        .unwrap_err(),
        WaveformError::NoChannels
    );
    assert_eq!(
        WaveformData::from_magnitudes(Duration::from_secs(1), 1, vec![]).unwrap_err(),
        WaveformError::NoResolutions
    );
    assert_eq!(
        WaveformData::from_magnitudes(
            Duration::from_secs(1),
            1,
            vec![WaveformLevel::new(Duration::ZERO, vec![0.5])],
        )
        .unwrap_err(),
        WaveformError::ZeroBucketSpan { resolution: 0 }
    );
    assert_eq!(
        WaveformData::from_magnitudes(
            Duration::from_secs(2),
            1,
            vec![
                WaveformLevel::new(Duration::from_secs(1), vec![0.2, 0.4]),
                WaveformLevel::new(Duration::from_secs(1), vec![0.3, 0.5]),
            ],
        )
        .unwrap_err(),
        WaveformError::NonIncreasingBucketSpan { resolution: 1 }
    );
    assert_eq!(
        WaveformData::from_magnitudes(
            Duration::from_secs(2),
            1,
            vec![
                WaveformLevel::new(Duration::from_secs(2), vec![0.2]),
                WaveformLevel::new(Duration::from_secs(1), vec![0.3, 0.5]),
            ],
        )
        .unwrap_err(),
        WaveformError::NonIncreasingBucketSpan { resolution: 1 }
    );
    assert_eq!(
        WaveformData::from_magnitudes(
            Duration::from_secs(1),
            1,
            vec![WaveformLevel::new(Duration::from_secs(1), vec![])],
        )
        .unwrap_err(),
        WaveformError::NoBuckets { resolution: 0 }
    );
    assert_eq!(
        WaveformData::from_magnitudes(
            Duration::from_secs(2),
            2,
            vec![WaveformLevel::new(
                Duration::from_secs(1),
                vec![0.1, 0.2, 0.3],
            )],
        )
        .unwrap_err(),
        WaveformError::MisalignedChannelData {
            resolution: 0,
            values: 3,
            channels: 2,
        }
    );

    for (actual_buckets, values) in [(3, vec![0.1, 0.2, 0.3]), (5, vec![0.1, 0.2, 0.3, 0.4, 0.5])] {
        assert_eq!(
            WaveformData::from_magnitudes(
                Duration::from_secs(4),
                1,
                vec![WaveformLevel::new(Duration::from_secs(1), values)],
            )
            .unwrap_err(),
            WaveformError::DurationCoverage {
                resolution: 0,
                expected_buckets: 4,
                actual_buckets,
            }
        );
    }
}

#[test]
fn magnitude_construction_rejects_non_finite_and_out_of_range_values() {
    for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.01, 1.01] {
        let error = WaveformData::from_magnitudes(
            Duration::from_secs(2),
            2,
            vec![WaveformLevel::new(
                Duration::from_secs(1),
                vec![0.0, 1.0, 0.5, invalid],
            )],
        )
        .unwrap_err();

        match error {
            WaveformError::InvalidMagnitude {
                resolution,
                channel,
                bucket,
                value,
            } => {
                assert_eq!((resolution, channel, bucket), (0, 1, 1));
                assert!(value.is_nan() && invalid.is_nan() || value == invalid);
            }
            other => panic!("expected invalid magnitude, got {other:?}"),
        }
    }
}

#[test]
fn levels_allow_only_the_final_bucket_to_be_shorter_than_the_span() {
    let data = WaveformData::from_magnitudes(
        Duration::from_millis(2500),
        1,
        vec![WaveformLevel::new(
            Duration::from_secs(1),
            vec![0.1, 0.2, 0.3],
        )],
    )
    .unwrap();

    let final_bucket = data
        .select(Duration::from_millis(2300)..Duration::from_millis(2500), 1)
        .unwrap();
    assert_eq!(final_bucket.first_bucket(), 2);
    assert_eq!(final_bucket.bucket_count(), 1);
    assert_eq!(
        final_bucket.channel(0),
        Some(AmplitudeSlice::Magnitudes(&[0.3]))
    );

    assert_eq!(
        WaveformData::from_magnitudes(
            Duration::MAX,
            1,
            vec![WaveformLevel::new(Duration::from_nanos(1), vec![0.1])],
        )
        .unwrap_err(),
        WaveformError::BucketCountOverflow { resolution: 0 }
    );
}

#[test]
fn signed_envelope_data_preserves_bounds_and_channel_layout() {
    let data = WaveformData::from_signed_envelopes(
        Duration::from_secs(2),
        2,
        vec![WaveformLevel::new(
            Duration::from_secs(1),
            vec![
                SignedEnvelope {
                    min: -1.0,
                    max: 0.25,
                },
                SignedEnvelope {
                    min: -0.5,
                    max: 0.75,
                },
                SignedEnvelope {
                    min: -0.2,
                    max: 0.8,
                },
                SignedEnvelope { min: 0.1, max: 0.9 },
            ],
        )],
    )
    .unwrap();

    assert_eq!(data.mode(), AmplitudeMode::SignedEnvelope);
    let view = data
        .select(Duration::ZERO..Duration::from_secs(2), 2)
        .unwrap();
    assert_eq!(
        view.channel(0),
        Some(AmplitudeSlice::SignedEnvelopes(&[
            SignedEnvelope {
                min: -1.0,
                max: 0.25,
            },
            SignedEnvelope {
                min: -0.5,
                max: 0.75,
            },
        ]))
    );
    assert_eq!(
        view.channel(1),
        Some(AmplitudeSlice::SignedEnvelopes(&[
            SignedEnvelope {
                min: -0.2,
                max: 0.8,
            },
            SignedEnvelope { min: 0.1, max: 0.9 },
        ]))
    );
}

#[test]
fn signed_envelope_construction_rejects_invalid_bounds() {
    for invalid in [
        SignedEnvelope {
            min: f32::NAN,
            max: 0.5,
        },
        SignedEnvelope {
            min: -0.5,
            max: f32::INFINITY,
        },
        SignedEnvelope {
            min: -1.01,
            max: 0.5,
        },
        SignedEnvelope {
            min: -0.5,
            max: 1.01,
        },
        SignedEnvelope { min: 0.5, max: 0.4 },
    ] {
        let error = WaveformData::from_signed_envelopes(
            Duration::from_secs(2),
            2,
            vec![WaveformLevel::new(
                Duration::from_secs(1),
                vec![
                    SignedEnvelope {
                        min: -1.0,
                        max: 1.0,
                    },
                    SignedEnvelope { min: 0.0, max: 0.0 },
                    SignedEnvelope {
                        min: -0.5,
                        max: 0.5,
                    },
                    invalid,
                ],
            )],
        )
        .unwrap_err();

        match error {
            WaveformError::InvalidSignedEnvelope {
                resolution,
                channel,
                bucket,
                min,
                max,
            } => {
                assert_eq!((resolution, channel, bucket), (0, 1, 1));
                assert!(min.is_nan() && invalid.min.is_nan() || min == invalid.min);
                assert!(max.is_nan() && invalid.max.is_nan() || max == invalid.max);
            }
            other => panic!("expected invalid signed envelope, got {other:?}"),
        }
    }
}

#[test]
fn range_and_budget_select_the_finest_fitting_resolution() {
    let data = WaveformData::from_magnitudes(
        Duration::from_secs(8),
        1,
        vec![
            WaveformLevel::new(
                Duration::from_secs(1),
                vec![0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7],
            ),
            WaveformLevel::new(Duration::from_secs(2), vec![0.1, 0.3, 0.5, 0.7]),
            WaveformLevel::new(Duration::from_secs(4), vec![0.3, 0.7]),
        ],
    )
    .unwrap();

    let finest = data
        .select(Duration::from_secs(2)..Duration::from_secs(6), 4)
        .unwrap();
    assert_eq!(finest.resolution_index(), 0);
    assert_eq!(finest.first_bucket(), 2);
    assert_eq!(finest.bucket_count(), 4);
    assert_eq!(
        finest.channel(0),
        Some(AmplitudeSlice::Magnitudes(&[0.2, 0.3, 0.4, 0.5]))
    );

    let middle = data
        .select(Duration::from_secs(2)..Duration::from_secs(6), 3)
        .unwrap();
    assert_eq!(middle.resolution_index(), 1);
    assert_eq!(middle.first_bucket(), 1);
    assert_eq!(middle.bucket_count(), 2);
    assert_eq!(
        middle.bucket_span().exact_duration(),
        Some(Duration::from_secs(2))
    );

    let fallback = data
        .select(Duration::from_secs(2)..Duration::from_secs(6), 1)
        .unwrap();
    assert_eq!(fallback.resolution_index(), 2);
    assert_eq!(fallback.first_bucket(), 0);
    assert_eq!(fallback.bucket_count(), 2);
}

#[test]
fn peaks_conversion_is_evenly_spaced_mono_magnitude_data() {
    let data =
        WaveformData::from_peaks(Duration::from_nanos(10), vec![0, 64, 128, 192, 255, 32]).unwrap();

    assert_eq!(data.mode(), AmplitudeMode::Magnitude);
    assert_eq!(data.duration(), Duration::from_nanos(10));
    assert_eq!(data.channel_count(), 1);
    assert_eq!(data.resolution_count(), 1);

    let resolution = data.resolution(0).unwrap();
    assert_eq!(resolution.buckets_per_channel(), 6);
    assert_eq!(
        resolution.bucket_span().numerator(),
        Duration::from_nanos(10)
    );
    assert_eq!(resolution.bucket_span().divisor(), 6);
    assert_eq!(resolution.bucket_span().exact_duration(), None);

    let view = data
        .select(Duration::ZERO..Duration::from_nanos(10), 6)
        .unwrap();
    let Some(AmplitudeSlice::Magnitudes(values)) = view.channel(0) else {
        panic!("expected mono magnitude data");
    };
    assert_eq!(values.len(), 6);
    assert_eq!(values[0], 0.0);
    assert_eq!(values[2], 128.0 / 255.0);
    assert_eq!(values[4], 1.0);

    let latter_half = data
        .select(Duration::from_nanos(5)..Duration::from_nanos(10), 3)
        .unwrap();
    assert_eq!(latter_half.first_bucket(), 3);
    assert_eq!(latter_half.bucket_count(), 3);
    assert_eq!(
        latter_half.channel(0),
        Some(AmplitudeSlice::Magnitudes(&[
            192.0 / 255.0,
            1.0,
            32.0 / 255.0,
        ]))
    );

    let non_boundary_range = data
        .select(Duration::from_nanos(4)..Duration::from_nanos(6), 2)
        .unwrap();
    assert_eq!(non_boundary_range.first_bucket(), 2);
    assert_eq!(non_boundary_range.bucket_count(), 2);
}

#[test]
fn peaks_conversion_rejects_missing_duration_or_peaks() {
    assert_eq!(
        WaveformData::from_peaks(Duration::ZERO, vec![1]).unwrap_err(),
        WaveformError::ZeroDuration
    );
    assert_eq!(
        WaveformData::from_peaks(Duration::from_secs(1), vec![]).unwrap_err(),
        WaveformError::EmptyPeaks
    );
}

#[test]
fn selection_uses_half_open_bucket_boundaries_and_rejects_invalid_inputs() {
    let data = WaveformData::from_magnitudes(
        Duration::from_secs(4),
        1,
        vec![WaveformLevel::new(
            Duration::from_secs(1),
            vec![0.1, 0.2, 0.3, 0.4],
        )],
    )
    .unwrap();

    let exact_bucket = data
        .select(Duration::from_secs(1)..Duration::from_secs(2), 1)
        .unwrap();
    assert_eq!(exact_bucket.first_bucket(), 1);
    assert_eq!(exact_bucket.bucket_count(), 1);

    let partial_final_bucket = data
        .select(Duration::from_millis(1500)..Duration::from_secs(2), 1)
        .unwrap();
    assert_eq!(partial_final_bucket.first_bucket(), 1);
    assert_eq!(partial_final_bucket.bucket_count(), 1);

    let partial_end = data
        .select(Duration::from_secs(1)..Duration::from_millis(1500), 1)
        .unwrap();
    assert_eq!(partial_end.first_bucket(), 1);
    assert_eq!(partial_end.bucket_count(), 1);

    assert_eq!(
        data.select(Duration::ZERO..Duration::from_secs(1), 0)
            .unwrap_err(),
        WaveformError::ZeroBucketBudget
    );
    for range in [
        Duration::from_secs(1)..Duration::from_secs(1),
        Duration::from_secs(3)..Duration::from_secs(2),
        Duration::ZERO..Duration::from_secs(5),
    ] {
        assert_eq!(
            data.select(range.clone(), 1).unwrap_err(),
            WaveformError::InvalidRange {
                start: range.start,
                end: range.end,
                duration: Duration::from_secs(4),
            }
        );
    }
}
