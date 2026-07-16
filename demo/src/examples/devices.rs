use dioxus::prelude::*;
use dioxus_audio::components::AudioInputSelector;
use dioxus_audio::devices::{MicrophonePermission, use_audio_input_devices};

use crate::components::StatusChip;

/// Request permission, enumerate microphones, and keep a reactive selection.
#[component]
pub fn DevicesExample() -> Element {
    let devices = use_audio_input_devices();
    let permission = devices.permission()();
    let status = devices.status()();
    let available = devices.devices()();
    let selected = devices.selected()()
        .map(|id| id.to_string())
        .unwrap_or_else(|| "system default".to_string());

    rsx! {
        div { class: "grid gap-4",
            div { class: "flex flex-wrap gap-2",
                StatusChip { label: format!("permission: {permission:?}") }
                StatusChip { label: format!("devices: {status:?}") }
            }
            AudioInputSelector { devices, label: "Microphone" }
            div { class: "flex flex-wrap gap-2",
                button {
                    class: "btn btn-primary btn-sm",
                    r#type: "button",
                    disabled: permission == MicrophonePermission::Prompt,
                    onclick: move |_| devices.request_permission(),
                    "Request access"
                }
                button {
                    class: "btn btn-ghost btn-sm",
                    r#type: "button",
                    onclick: move |_| devices.refresh(),
                    "Refresh"
                }
            }
            div { class: "rounded-xl border border-base-300 bg-base-100 p-4 text-sm",
                p { class: "font-medium", "Selected: {selected}" }
                p { class: "mt-1 text-base-content/60", "{available.len()} audio input(s) found" }
                if !available.is_empty() {
                    ul { class: "mt-3 space-y-1 font-mono text-xs text-base-content/70",
                        for (index, device) in available.iter().enumerate() {
                            li { key: "{device.id}", "{device.display_label(index)}" }
                        }
                    }
                }
            }
        }
    }
}
