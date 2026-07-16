use dioxus::prelude::*;

use crate::AudioInputId;
use crate::devices::{AudioInputDevices, DeviceListStatus, MicrophonePermission};
use crate::recorder::{MicrophoneStatus, RecorderStatus};

#[component]
pub fn AudioInputSelector(
    devices: AudioInputDevices,
    #[props(default = false)] disabled: bool,
    #[props(default)] label: Option<String>,
) -> Element {
    let available = devices.devices()();
    let selected = devices.selected()();
    let status = devices.status()();
    let unavailable = matches!(
        status,
        DeviceListStatus::Unsupported | DeviceListStatus::Loading
    );
    let label = label.unwrap_or_else(|| "Audio input".to_string());

    rsx! {
        label { class: "dioxus-audio dioxus-audio__device",
            span { class: "dioxus-audio__device-label",
                {label.clone()}
            }
            select {
                class: "dioxus-audio__device-select",
                value: selected.as_ref().map(AudioInputId::as_str).unwrap_or(""),
                disabled: disabled || unavailable,
                aria_label: label,
                onchange: move |event| {
                    let value = event.value();
                    devices.select(if value.is_empty() {
                        None
                    } else {
                        Some(AudioInputId::new(value))
                    });
                },
                option { value: "", "System default" }
                for (index, device) in available.iter().enumerate() {
                    option {
                        key: "{device.id}",
                        value: "{device.id}",
                        selected: selected.as_ref() == Some(&device.id),
                        {device.display_label(index)}
                    }
                }
            }
            match status {
                DeviceListStatus::Loading => rsx! {
                    span { class: "dioxus-audio__device-help", role: "status", "Finding microphones..." }
                },
                DeviceListStatus::Unsupported => rsx! {
                    span { class: "dioxus-audio__device-help", "Audio inputs are unavailable" }
                },
                DeviceListStatus::Failed(ref error) => rsx! {
                    span { class: "dioxus-audio__device-help dioxus-audio__device-help--error",
                        role: "alert",
                        "{error}"
                    }
                },
                DeviceListStatus::Ready => rsx! {},
            }
        }
    }
}

#[component]
pub fn MicrophoneStatusIndicator(
    status: ReadSignal<MicrophoneStatus>,
    #[props(default)] on_retry: Option<EventHandler<()>>,
) -> Element {
    let status = status();
    let (state, message) = microphone_message(&status);

    rsx! {
        div {
            class: "dioxus-audio dioxus-audio__microphone-status",
            role: "status",
            aria_live: "polite",
            "data-state": state,
            span { class: "dioxus-audio__microphone-dot" }
            span { class: "dioxus-audio__microphone-message", "{message}" }
            if matches!(status.permission, MicrophonePermission::Denied)
                || matches!(status.recorder, RecorderStatus::Failed(_))
            {
                if let Some(on_retry) = on_retry {
                    button {
                        class: "dioxus-audio__retry",
                        r#type: "button",
                        onclick: move |_| on_retry.call(()),
                        "Try again"
                    }
                }
            }
        }
    }
}

fn microphone_message(status: &MicrophoneStatus) -> (&'static str, String) {
    if status.permission == MicrophonePermission::Unsupported {
        return ("unsupported", "Microphone unavailable".to_string());
    }
    if status.permission == MicrophonePermission::Denied {
        return ("error", "Microphone access denied".to_string());
    }
    if status.muted {
        return ("warning", "Microphone muted by the device".to_string());
    }

    match &status.recorder {
        RecorderStatus::Idle if status.permission == MicrophonePermission::Granted => {
            ("ready", "Microphone ready".to_string())
        }
        RecorderStatus::Idle => ("idle", "Microphone not requested".to_string()),
        RecorderStatus::RequestingPermission => {
            ("pending", "Requesting microphone access".to_string())
        }
        RecorderStatus::Recording => ("recording", "Recording".to_string()),
        RecorderStatus::Paused => ("paused", "Recording paused".to_string()),
        RecorderStatus::Stopping => ("pending", "Finishing recording".to_string()),
        RecorderStatus::Failed(error) => ("error", error.to_string()),
    }
}
