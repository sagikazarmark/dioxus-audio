use dioxus::prelude::*;
use dioxus_audio::components::{
    AudioInputSelector, AudioPlayer, MicrophoneStatusIndicator, WaveformPreview,
};
use dioxus_audio::devices::{MicrophonePermission, use_audio_input_devices};
use dioxus_audio::playback::PlaybackSource;
use dioxus_audio::recorder::{MicrophoneStatus, RecorderStatus};

use super::fixtures::{generated_audio, peaks};

// region: studio-recipe
#[component]
pub fn StudioExample() -> Element {
    let devices = use_audio_input_devices();
    let mut microphone_status = use_signal(ready_status);
    let mut source = use_signal(|| None::<PlaybackSource>);
    let current_status = microphone_status();

    rsx! {
        div { class: "studio-app",
            header { class: "studio-app__header",
                div {
                    p { class: "studio-app__eyebrow", "Studio / voice note 04" }
                    h3 { class: "studio-app__title", "Morning field notes" }
                }
                span { class: "studio-app__duration", "00:02 generated sample" }
            }

            div { class: "studio-app__workspace",
                div { class: "studio-app__input-panel",
                    AudioInputSelector { devices, label: "Recording input" }
                    fieldset { class: "studio-app__preview-controls",
                        legend { "Preview microphone state" }
                        div { class: "studio-app__preview-buttons",
                            PreviewButton {
                                label: "Ready",
                                active: is_ready(&current_status),
                                onclick: move |_| microphone_status.set(ready_status()),
                            }
                            PreviewButton {
                                label: "Recording",
                                active: current_status.recorder == RecorderStatus::Recording,
                                onclick: move |_| microphone_status.set(recording_status()),
                            }
                            PreviewButton {
                                label: "Muted",
                                active: current_status.muted,
                                onclick: move |_| microphone_status.set(muted_status()),
                            }
                            PreviewButton {
                                label: "Denied",
                                active: current_status.permission == MicrophonePermission::Denied,
                                onclick: move |_| microphone_status.set(denied_status()),
                            }
                        }
                    }
                    MicrophoneStatusIndicator { status: microphone_status }
                }

                div { class: "studio-app__audio-panel",
                    div {
                        p { class: "studio-app__panel-label", "Voice note peaks" }
                        WaveformPreview {
                            peaks: peaks(),
                            bars: 72,
                            height: 68.0,
                            label: "Morning field notes waveform",
                        }
                    }
                    AudioPlayer {
                        source,
                        duration_secs: 2.0,
                        on_request_audio: move |_| source.set(Some(generated_audio().into())),
                    }
                }
            }
        }
    }
}

#[component]
fn PreviewButton(
    #[props(into)] label: String,
    active: bool,
    onclick: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        button {
            r#type: "button",
            class: if active { "btn btn-xs btn-primary" } else { "btn btn-xs btn-ghost" },
            aria_pressed: active,
            onclick: move |event| onclick.call(event),
            "{label}"
        }
    }
}

fn ready_status() -> MicrophoneStatus {
    MicrophoneStatus {
        permission: MicrophonePermission::Granted,
        recorder: RecorderStatus::Idle,
        input_device: None,
        muted: false,
    }
}

fn recording_status() -> MicrophoneStatus {
    MicrophoneStatus {
        recorder: RecorderStatus::Recording,
        ..ready_status()
    }
}

fn muted_status() -> MicrophoneStatus {
    MicrophoneStatus {
        muted: true,
        ..ready_status()
    }
}

fn denied_status() -> MicrophoneStatus {
    MicrophoneStatus {
        permission: MicrophonePermission::Denied,
        ..ready_status()
    }
}

fn is_ready(status: &MicrophoneStatus) -> bool {
    status.permission == MicrophonePermission::Granted
        && status.recorder == RecorderStatus::Idle
        && !status.muted
}
// endregion: studio-recipe
