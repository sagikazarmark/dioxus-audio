use dioxus::prelude::*;
use dioxus_audio::components::{
    AudioInputSelector, AudioPlayer, AudioScrubber, MicrophoneStatusIndicator,
    PlaybackAnnouncementLabels, PlaybackPlayPauseButton, PlaybackRateButton, PlaybackSeekSlider,
    PlaybackSkipButton, PlaybackStatusAnnouncer, RecorderAnnouncementLabels, RecorderCancelButton,
    RecorderClearButton, RecorderControls, RecorderPauseResumeButton, RecorderStartButton,
    RecorderStatusAnnouncer, RecorderStopButton, SpectrumVisualizer, WaveformPreview,
};
use dioxus_audio::devices::{MicrophonePermission, use_audio_input_devices};
use dioxus_audio::playback::use_audio_player;
use dioxus_audio::recorder::{
    MicrophoneStatus, RecorderOptions, RecorderStatus, use_audio_recorder,
};
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
        let source = use_signal(|| None::<dioxus_audio::AudioData>);
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
    assert!(html.contains("data-source=\"empty\""));
    assert!(html.contains("data-transport=\"idle\""));
    assert!(html.contains("data-readiness=\"unavailable\""));
    assert!(html.contains("data-play-failure=\"none\""));
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
        let source = use_signal(|| None::<dioxus_audio::AudioData>);
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
