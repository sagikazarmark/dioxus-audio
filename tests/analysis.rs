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
fn waveform_selection_clamps_orders_and_trims_complete_frames() {
    let selection = WaveformSelection::new(0.75, 0.25);
    assert_eq!(selection, WaveformSelection::new(0.25, 0.75));
    assert_eq!(selection.start(), 0.25);
    assert_eq!(selection.end(), 0.75);
    assert_eq!(
        selection.with_start(0.9),
        WaveformSelection::new(0.75, 0.75)
    );
    assert_eq!(selection.with_end(0.1), WaveformSelection::new(0.25, 0.25));
    assert!(
        trim_interleaved_pcm(&[1_i16, 2, 3, 4], 1, WaveformSelection::new(0.3, 0.3)).is_empty()
    );

    let stereo = [1_i16, 2, 3, 4, 5, 6, 7, 8];
    assert_eq!(
        trim_interleaved_pcm(&stereo, 2, selection),
        vec![3, 4, 5, 6]
    );
}
