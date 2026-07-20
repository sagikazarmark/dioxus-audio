use dioxus::prelude::*;
use dioxus_audio::analysis::{LiveAnalysisOptions, WaveformSelection, use_live_analysis};
use dioxus_audio::components::{
    AudioInputSelector, AudioPlayer, AudioScrubber, LevelMeter, LiveWaveform,
    MicrophoneStatusIndicator, PlaybackAnnouncementLabels, PlaybackAudibilitySlider,
    PlaybackMuteButton, PlaybackPlayPauseButton, PlaybackRateButton, PlaybackRepeatButton,
    PlaybackSeekSlider, PlaybackSkipButton, PlaybackStatusAnnouncer, PlaybackStopButton,
    RecorderAnnouncementLabels, RecorderCancelButton, RecorderClearButton, RecorderControls,
    RecorderPauseResumeButton, RecorderStartButton, RecorderStatusAnnouncer, RecorderStopButton,
    SpectrumVisualizer, Waveform, WaveformPreview, WaveformRangeSelector,
};
use dioxus_audio::devices::{MicrophonePermission, use_audio_input_devices};
use dioxus_audio::playback::{PlaybackSource, use_audio_player};
use dioxus_audio::recorder::{
    MicrophoneStatus, RecorderOptions, RecorderStatus, use_audio_recorder,
};
use dioxus_audio::waveform::{SignedEnvelope, WaveformData, WaveformLevel};
use std::time::Duration;

#[test]
fn scrubber_has_an_accessible_name_and_namespaced_styles() {
    fn app() -> Element {
        rsx! {
            AudioScrubber {
                position_secs: 5.0,
                duration_secs: 20.0,
                on_seek: move |_| {},
            }
        }
    }

    let mut vdom = VirtualDom::new(app);
    vdom.rebuild_in_place();
    let html = dioxus_ssr::render(&vdom);

    assert!(html.contains("aria-label=\"Seek audio\""));
    assert!(html.contains("aria-valuetext=\"5 seconds of 20 seconds\""));
    assert!(html.contains("dioxus-audio__scrubber"));
}

#[test]
fn playback_commands_can_be_composed_as_independent_native_controls() {
    fn app() -> Element {
        let source = use_signal(|| None::<PlaybackSource>);
        let controller = use_audio_player(source.into(), Duration::from_secs(20));

        rsx! {
            PlaybackSeekSlider {
                controller,
                label: "Episode position".to_string(),
                value_text: "Five seconds into the episode".to_string(),
            }
            PlaybackSkipButton {
                controller,
                seconds: -7.5,
                label: "Rewind seven and a half seconds".to_string(),
            }
            PlaybackPlayPauseButton {
                controller,
                play_label: "Listen".to_string(),
                pause_label: "Hold".to_string(),
                on_request_audio: move |_| {},
            }
            PlaybackSkipButton {
                controller,
                seconds: 7.5,
                label: "Advance seven and a half seconds".to_string(),
            }
            PlaybackRateButton {
                controller,
                rates: vec![0.75, 1.0, 1.25],
                label: "Listening speed".to_string(),
            }
        }
    }

    let mut vdom = VirtualDom::new(app);
    vdom.rebuild_in_place();
    let html = dioxus_ssr::render(&vdom);

    assert!(html.contains("type=\"range\""));
    assert!(html.contains("aria-label=\"Episode position\""));
    assert!(html.contains("aria-valuetext=\"Five seconds into the episode\""));
    assert!(html.contains("aria-label=\"Rewind seven and a half seconds\""));
    assert!(html.contains("aria-label=\"Listen\""));
    assert!(html.contains("aria-label=\"Advance seven and a half seconds\""));
    assert!(html.contains("aria-label=\"Listening speed: 1x\""));
    assert_eq!(html.matches(" disabled").count(), 3, "{html}");
}

#[test]
fn stop_and_repeat_are_reusable_native_controls() {
    fn app() -> Element {
        let source = use_signal(|| None::<PlaybackSource>);
        let controller = use_audio_player(source.into(), Duration::from_secs(20));

        rsx! {
            PlaybackStopButton {
                controller,
                label: "Reset episode".to_string(),
            }
            PlaybackRepeatButton {
                controller,
                label: "Repeat episode".to_string(),
            }
        }
    }

    let mut vdom = VirtualDom::new(app);
    vdom.rebuild_in_place();
    let html = dioxus_ssr::render(&vdom);

    assert_eq!(html.matches("type=\"button\"").count(), 2, "{html}");
    assert!(html.contains("aria-label=\"Reset episode\""));
    assert!(html.contains("aria-label=\"Repeat episode\""));
    assert!(html.contains("aria-pressed=\"false\""));
}

#[test]
fn mute_and_audibility_level_are_reusable_native_controls() {
    fn app() -> Element {
        let source = use_signal(|| None::<PlaybackSource>);
        let controller = use_audio_player(source.into(), Duration::from_secs(20));

        rsx! {
            PlaybackMuteButton {
                controller,
                label: "Silence episode".to_string(),
            }
            PlaybackAudibilitySlider {
                controller,
                label: "Episode audibility".to_string(),
            }
        }
    }

    let mut vdom = VirtualDom::new(app);
    vdom.rebuild_in_place();
    let html = dioxus_ssr::render(&vdom);

    assert!(html.contains("type=\"button\""), "{html}");
    assert!(html.contains("aria-label=\"Silence episode\""), "{html}");
    assert!(html.contains("aria-pressed=\"false\""), "{html}");
    assert!(html.contains("type=\"range\""), "{html}");
    assert!(html.contains("aria-label=\"Episode audibility\""), "{html}");
    assert!(html.contains("min=\"0\""), "{html}");
    assert!(html.contains("max=\"1\""), "{html}");
    assert!(html.contains("step=\"0.01\""), "{html}");
    assert!(html.contains("value=\"1\""), "{html}");
    assert!(html.contains("aria-valuetext=\"100 percent\""), "{html}");
}

#[test]
fn live_visualizers_are_named_for_assistive_technology() {
    fn app() -> Element {
        let analyser = use_signal(|| None);
        rsx! {
            SpectrumVisualizer {
                analyser,
                label: "Microphone spectrum".to_string(),
            }
        }
    }

    let mut vdom = VirtualDom::new(app);
    vdom.rebuild_in_place();
    let html = dioxus_ssr::render(&vdom);

    assert!(html.contains("role=\"img\""));
    assert!(html.contains("aria-label=\"Microphone spectrum\""));
}

#[test]
fn live_analysis_is_ssr_neutral_and_never_creates_a_live_region() {
    fn app() -> Element {
        let analyser = use_signal(|| None);
        let snapshot = use_live_analysis(analyser.into(), LiveAnalysisOptions::default());

        rsx! {
            p { "Analysis available: {snapshot().is_some()}" }
            LiveWaveform { analyser }
            SpectrumVisualizer { analyser }
            LevelMeter { analyser }
        }
    }

    let mut vdom = VirtualDom::new(app);
    vdom.rebuild_in_place();
    let html = dioxus_ssr::render(&vdom);

    assert!(html.contains("Analysis available: false"), "{html}");
    assert_eq!(html.matches("role=\"img\"").count(), 2, "{html}");
    assert_eq!(html.matches("role=\"meter\"").count(), 1, "{html}");
    assert!(!html.contains("aria-live"), "{html}");
    assert!(!html.contains("role=\"status\""), "{html}");
}

#[test]
fn microphone_and_device_components_expose_status_and_labels() {
    fn app() -> Element {
        let devices = use_audio_input_devices();
        let status = use_signal(|| MicrophoneStatus {
            permission: MicrophonePermission::Denied,
            recorder: RecorderStatus::Idle,
            input_device: None,
            muted: false,
        });
        rsx! {
            AudioInputSelector { devices }
            MicrophoneStatusIndicator { status }
        }
    }

    let mut vdom = VirtualDom::new(app);
    vdom.rebuild_in_place();
    let html = dioxus_ssr::render(&vdom);

    assert!(html.contains("Audio input"));
    assert!(html.contains("aria-live=\"polite\""));
    assert!(html.contains("Microphone access denied"));
}

#[test]
fn player_controls_have_explicit_accessible_names() {
    fn app() -> Element {
        let source = use_signal(|| None);
        rsx! {
            AudioPlayer {
                source,
                duration_secs: 30.0,
                on_request_audio: move |_| {},
            }
        }
    }

    let mut vdom = VirtualDom::new(app);
    vdom.rebuild_in_place();
    let html = dioxus_ssr::render(&vdom);

    assert!(html.contains("aria-label=\"Play\""));
    assert!(html.contains("aria-label=\"Skip back 15 seconds\""));
    assert!(html.contains("aria-label=\"Playback speed: 1x\""));
    assert!(html.contains("aria-label=\"Stop\""));
    assert!(html.contains("aria-label=\"Repeat\""));
    assert!(html.contains("aria-label=\"Mute\""));
    assert!(html.contains("aria-label=\"Audibility level\""));
    assert!(html.contains("aria-valuetext=\"100 percent\""));
    assert!(html.contains("aria-pressed=\"false\""));
    assert!(html.contains("data-source=\"empty\""));
    assert!(html.contains("data-transport=\"idle\""));
    assert!(html.contains("data-readiness=\"unavailable\""));
    assert!(html.contains("data-network=\"inactive\""));
    assert!(html.contains("data-buffered=\"\""));
    assert!(html.contains("data-seekable=\"\""));
    assert!(html.contains("data-source-failure=\"none\""));
    assert!(html.contains("data-play-failure=\"none\""));
    assert!(html.contains("data-repeat=\"false\""));
    assert!(html.contains("data-muted=\"false\""));
    assert!(html.contains("data-audibility-level=\"1\""));
    assert!(html.contains("data-audibility-capability=\"best-effort-media-element\""));
}

#[test]
fn recorder_controls_name_every_action() {
    fn app() -> Element {
        let selected = use_signal(|| None);
        let recorder = use_audio_recorder(RecorderOptions::default(), selected.into());
        rsx! { RecorderControls { recorder } }
    }

    let mut vdom = VirtualDom::new(app);
    vdom.rebuild_in_place();
    let html = dioxus_ssr::render(&vdom);

    assert!(html.contains("aria-label=\"Start recording\""));
}

#[test]
fn recorder_commands_can_be_composed_as_independent_native_controls() {
    fn app() -> Element {
        let selected = use_signal(|| None);
        let recorder = use_audio_recorder(RecorderOptions::default(), selected.into());

        rsx! {
            RecorderStartButton {
                recorder,
                label: "Begin capture".to_string(),
                completed_label: "Clear the saved capture first".to_string(),
            }
            RecorderCancelButton {
                recorder,
                request_label: "Abort microphone access".to_string(),
                recording_label: "Discard capture".to_string(),
            }
            RecorderPauseResumeButton {
                recorder,
                pause_label: "Hold capture".to_string(),
                resume_label: "Continue capture".to_string(),
            }
            RecorderStopButton {
                recorder,
                stop_label: "Finish capture".to_string(),
                stopping_label: "Saving capture".to_string(),
            }
            RecorderClearButton {
                recorder,
                label: "Clear saved capture".to_string(),
            }
        }
    }

    let mut vdom = VirtualDom::new(app);
    vdom.rebuild_in_place();
    let html = dioxus_ssr::render(&vdom);

    assert!(html.contains("aria-label=\"Begin capture\""));
    assert!(html.contains("aria-label=\"Discard capture\""));
    assert!(html.contains("aria-label=\"Hold capture\""));
    assert!(html.contains("aria-label=\"Finish capture\""));
    assert!(html.contains("aria-label=\"Clear saved capture\""));
    assert_eq!(html.matches(" disabled").count(), 4, "{html}");
}

#[test]
fn optional_status_announcers_expose_only_coarse_localizable_state() {
    fn app() -> Element {
        let source = use_signal(|| None::<PlaybackSource>);
        let player = use_audio_player(source.into(), Duration::from_secs(20));
        let selected = use_signal(|| None);
        let recorder = use_audio_recorder(RecorderOptions::default(), selected.into());
        let playback_labels = PlaybackAnnouncementLabels {
            empty: "Nothing queued".to_string(),
            ..Default::default()
        };
        let recorder_labels = RecorderAnnouncementLabels {
            idle: "Capture ready".to_string(),
            ..Default::default()
        };

        rsx! {
            PlaybackStatusAnnouncer { controller: player, labels: playback_labels }
            RecorderStatusAnnouncer { recorder, labels: recorder_labels }
        }
    }

    let mut vdom = VirtualDom::new(app);
    vdom.rebuild_in_place();
    let html = dioxus_ssr::render(&vdom);

    assert_eq!(html.matches("role=\"status\"").count(), 2);
    assert_eq!(html.matches("aria-live=\"polite\"").count(), 2);
    assert_eq!(html.matches("aria-atomic=\"true\"").count(), 2);
    assert!(html.contains("Nothing queued"));
    assert!(html.contains("Capture ready"));
    assert!(!html.contains("20 seconds"));
}

#[test]
fn waveform_can_expose_an_accessible_description() {
    fn app() -> Element {
        rsx! {
            WaveformPreview {
                peaks: vec![0, 64, 255, 64],
                label: "Recorded waveform".to_string(),
            }
        }
    }

    let mut vdom = VirtualDom::new(app);
    vdom.rebuild_in_place();
    let html = dioxus_ssr::render(&vdom);

    assert!(html.contains("role=\"img\""));
    assert!(html.contains("aria-label=\"Recorded waveform\""));
}

#[test]
fn waveform_range_selectors_expose_independent_source_time_values() {
    fn app() -> Element {
        rsx! {
            WaveformRangeSelector {
                peaks: vec![0, 128, 255],
                duration_secs: 5.0,
                selection: WaveformSelection::new(1.256, 3.5),
                on_change: move |_| {},
                label: "Primary clip range",
            }
            WaveformRangeSelector {
                peaks: vec![255, 128, 0],
                duration_secs: 2.0,
                selection: WaveformSelection::new(0.5, 1.5),
                on_change: move |_| {},
                label: "Secondary clip range",
            }
        }
    }

    let mut vdom = VirtualDom::new(app);
    vdom.rebuild_in_place();
    let html = dioxus_ssr::render(&vdom);

    assert_eq!(html.matches("type=\"range\"").count(), 4, "{html}");
    assert_eq!(html.matches("step=\"any\"").count(), 4, "{html}");
    assert!(html.contains("aria-label=\"Primary clip range\""), "{html}");
    assert!(
        html.contains("aria-label=\"Secondary clip range\""),
        "{html}"
    );
    assert!(html.contains("max=\"5\""), "{html}");
    assert!(html.contains("value=\"1.256\""), "{html}");
    assert!(html.contains("aria-valuetext=\"1.26 seconds\""), "{html}");
    assert!(html.contains("value=\"3.5\""), "{html}");
    assert!(html.contains("aria-valuetext=\"3.5 seconds\""), "{html}");
    assert!(html.contains("max=\"2\""), "{html}");
    assert!(html.contains("value=\"0.5\""), "{html}");
    assert!(html.contains("aria-valuetext=\"0.5 seconds\""), "{html}");
    assert!(html.contains("value=\"1.5\""), "{html}");
    assert!(html.contains("aria-valuetext=\"1.5 seconds\""), "{html}");
}

#[test]
fn short_quiet_waveforms_fill_the_preview_and_keep_visible_contrast() {
    let html = dioxus_ssr::render_element(rsx! {
        WaveformPreview {
            peaks: vec![4, 8],
            bars: 8,
            height: 32.0,
        }
    });

    assert_eq!(html.matches("<rect").count(), 8);
    assert!(html.contains("height=\"8\""));
}

#[test]
fn waveform_data_renders_each_channel_responsively() {
    let data = WaveformData::from_magnitudes(
        Duration::from_secs(2),
        2,
        vec![WaveformLevel::new(
            Duration::from_secs(1),
            vec![0.25, 1.0, 0.75, 0.5],
        )],
    )
    .unwrap();

    let html = dioxus_ssr::render_element(rsx! {
        Waveform {
            data,
            height: 80.0,
            label: "Stereo magnitude waveform".to_string(),
        }
    });

    assert!(html.contains("role=\"img\""), "{html}");
    assert!(
        html.contains("aria-label=\"Stereo magnitude waveform\""),
        "{html}"
    );
    assert!(html.contains("width=\"100%\""), "{html}");
    assert_eq!(html.matches("<path").count(), 2, "{html}");
    assert!(html.contains("data-amplitude-mode=\"magnitude\""), "{html}");
    assert!(html.contains("data-channel-count=2"), "{html}");
    assert!(html.contains("d=\"M0 40L0 30H1L1 0H2L2 40Z\""), "{html}");
    assert!(html.contains("d=\"M0 80L0 50H1L1 60H2L2 80Z\""), "{html}");
}

#[test]
fn waveform_data_keeps_signed_envelopes_visually_distinct_from_magnitudes() {
    let magnitude = WaveformData::from_magnitudes(
        Duration::from_secs(1),
        1,
        vec![WaveformLevel::new(Duration::from_secs(1), vec![0.5])],
    )
    .unwrap();
    let signed = WaveformData::from_signed_envelopes(
        Duration::from_secs(1),
        1,
        vec![WaveformLevel::new(
            Duration::from_secs(1),
            vec![SignedEnvelope {
                min: -1.0,
                max: 0.5,
            }],
        )],
    )
    .unwrap();

    let magnitude_html = dioxus_ssr::render_element(rsx! {
        Waveform { data: magnitude, height: 100.0 }
    });
    let signed_html = dioxus_ssr::render_element(rsx! {
        Waveform { data: signed, height: 100.0 }
    });

    assert!(
        magnitude_html.contains("data-amplitude-mode=\"magnitude\""),
        "{magnitude_html}"
    );
    assert!(
        signed_html.contains("data-amplitude-mode=\"signed-envelope\""),
        "{signed_html}"
    );
    assert_ne!(magnitude_html, signed_html);
    assert!(
        magnitude_html.contains("d=\"M0 100L0 50H1L1 100Z\""),
        "{magnitude_html}"
    );
    assert!(
        signed_html.contains("d=\"M0 25H1L1 100H0Z\""),
        "{signed_html}"
    );
    assert!(magnitude_html.contains("aria-hidden=true"));
    assert!(signed_html.contains("aria-hidden=true"));
}
