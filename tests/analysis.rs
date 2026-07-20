use dioxus_audio::analysis::{
    AnalysisMetadata, LiveAnalysisOptions, WaveformSelection, downsample_peaks, peak_amplitude,
    rms_level, trim_interleaved_pcm,
};
use std::time::Duration;

#[test]
fn analysis_metadata_maps_frequency_bins_and_decibels() {
    let metadata = AnalysisMetadata::new(48_000.0, 1_024, -100.0, -30.0, 0.8);

    assert_eq!(metadata.sample_rate(), 48_000.0);
    assert_eq!(metadata.fft_size(), 1_024);
    assert_eq!(metadata.frequency_bin_count(), 512);
    assert_eq!(metadata.frequency_bin_width(), 46.875);
    assert_eq!(metadata.frequency_for_bin(0), Some(0.0));
    assert_eq!(metadata.frequency_for_bin(511), Some(23_953.125));
    assert_eq!(metadata.frequency_for_bin(512), None);
    assert_eq!(metadata.decibels_for_frequency_value(0.0), -100.0);
    assert_eq!(metadata.decibels_for_frequency_value(0.5), -65.0);
    assert_eq!(metadata.decibels_for_frequency_value(1.0), -30.0);
    assert_eq!(metadata.smoothing(), 0.8);
}

#[test]
fn live_analysis_cadence_is_clamped_to_documented_bounds() {
    assert_eq!(
        LiveAnalysisOptions::default().cadence(),
        Duration::from_millis(50)
    );
    assert_eq!(
        LiveAnalysisOptions::default()
            .with_cadence(Duration::ZERO)
            .cadence(),
        Duration::from_millis(16)
    );
    assert_eq!(
        LiveAnalysisOptions::default()
            .with_cadence(Duration::from_secs(5))
            .cadence(),
        Duration::from_secs(1)
    );
}

#[test]
fn analysis_level_is_normalized_root_mean_square_amplitude() {
    assert_eq!(rms_level(&[]), 0.0);
    assert_eq!(rms_level(&[0.5, -0.5]), 0.5);
    assert!((rms_level(&[1.0, 0.0]) - std::f32::consts::FRAC_1_SQRT_2).abs() < f32::EPSILON);
    assert_eq!(rms_level(&[2.0, -2.0]), 1.0);
}

#[test]
fn short_waveforms_do_not_gain_empty_buckets() {
    assert_eq!(downsample_peaks(&[12, 90], 32), vec![12, 90]);
}

#[test]
fn long_waveforms_keep_the_peak_from_each_window() {
    assert_eq!(downsample_peaks(&[10, 80, 30, 60], 2), vec![80, 60]);
}

#[test]
fn time_domain_amplitude_uses_the_full_byte_range() {
    assert_eq!(peak_amplitude(&[128, 128]), 0);
    assert_eq!(peak_amplitude(&[128, 64]), 128);
    assert_eq!(peak_amplitude(&[0, 128, 255]), 255);
}

#[test]
fn waveform_selection_is_an_ordered_source_time_interval() {
    let selection = WaveformSelection::new(7.5, 2.5);

    assert_eq!(selection, WaveformSelection::new(2.5, 7.5));
    assert_eq!(selection.start(), 2.5);
    assert_eq!(selection.end(), 7.5);
    assert_eq!(selection.with_start(9.0), WaveformSelection::new(7.5, 7.5));
    assert_eq!(selection.with_end(1.0), WaveformSelection::new(2.5, 2.5));
    assert!(selection.with_start(9.0).is_collapsed());
}

#[test]
fn waveform_selection_keeps_only_finite_non_negative_source_times() {
    assert_eq!(
        WaveformSelection::new(f64::NAN, f64::INFINITY),
        WaveformSelection::new(0.0, 0.0)
    );
    assert_eq!(
        WaveformSelection::new(f64::NEG_INFINITY, -2.0),
        WaveformSelection::new(0.0, 0.0)
    );

    let selection = WaveformSelection::new(2.5, 7.5);
    assert_eq!(selection.with_start(f64::NAN), selection);
    assert_eq!(selection.with_end(f64::INFINITY), selection);
}

#[test]
fn waveform_selection_clamps_to_duration_without_swapping_boundaries() {
    assert_eq!(
        WaveformSelection::new(2.5, 7.5).clamped_to_duration(5.0),
        WaveformSelection::new(2.5, 5.0)
    );
    assert_eq!(
        WaveformSelection::new(7.5, 9.0).clamped_to_duration(5.0),
        WaveformSelection::new(5.0, 5.0)
    );

    assert!(WaveformSelection::new(2.5, 5.0).is_playable_within(5.0));
    assert!(!WaveformSelection::new(2.5, 2.5).is_playable_within(5.0));
    assert!(!WaveformSelection::new(2.5, 7.5).is_playable_within(5.0));
    assert!(!WaveformSelection::new(0.0, 1.0).is_playable_within(f64::NAN));
}

#[test]
fn source_time_pcm_trimming_resolves_complete_channel_frames() {
    let stereo_with_incomplete_frame = [1_i16, 2, 3, 4, 5, 6, 7, 8, 9];

    assert_eq!(
        trim_interleaved_pcm(
            &stereo_with_incomplete_frame,
            2,
            4.0,
            WaveformSelection::new(1.25, 2.25),
        ),
        vec![3, 4, 5, 6]
    );
    assert_eq!(
        trim_interleaved_pcm(
            &stereo_with_incomplete_frame,
            2,
            4.0,
            WaveformSelection::new(3.0, 7.0),
        ),
        vec![7, 8]
    );
    assert!(
        trim_interleaved_pcm(
            &stereo_with_incomplete_frame,
            2,
            4.0,
            WaveformSelection::new(2.0, 2.0),
        )
        .is_empty()
    );
    assert!(
        trim_interleaved_pcm(
            &stereo_with_incomplete_frame,
            0,
            4.0,
            WaveformSelection::new(1.0, 2.0),
        )
        .is_empty()
    );
    assert!(
        trim_interleaved_pcm(
            &stereo_with_incomplete_frame,
            2,
            f64::NAN,
            WaveformSelection::new(1.0, 2.0),
        )
        .is_empty()
    );
}
