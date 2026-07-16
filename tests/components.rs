use dioxus::prelude::*;
use dioxus_audio::components::{
    AudioInputSelector, AudioPlayer, AudioScrubber, MicrophoneStatusIndicator, RecorderControls,
    SpectrumVisualizer, WaveformPreview,
};
use dioxus_audio::devices::{MicrophonePermission, use_audio_input_devices};
use dioxus_audio::recorder::{
    MicrophoneStatus, RecorderOptions, RecorderStatus, use_audio_recorder,
};

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
    assert!(html.contains("dioxus-audio__scrubber"));
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
