use dioxus_audio::analysis::{
    WaveformSelection, downsample_peaks, peak_amplitude, trim_interleaved_pcm,
};

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
