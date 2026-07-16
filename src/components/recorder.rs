use dioxus::prelude::*;
use dioxus_icons::lucide::{Mic, Pause, Play, Square, X};

use crate::recorder::{AudioRecorder, RecorderStatus};

#[component]
pub fn RecorderControls(
    recorder: AudioRecorder,
    #[props(default)] on_cancelled: Option<EventHandler<()>>,
) -> Element {
    let status = recorder.status()();
    let state_name = recorder_state_name(&status);
    let paused = matches!(status, RecorderStatus::Paused);
    let has_completed = recorder.completed().read().is_some();

    rsx! {
        div {
            class: "dioxus-audio dioxus-audio__recorder-controls",
            "data-state": state_name,
            match status {
                RecorderStatus::Idle | RecorderStatus::Failed(_) => rsx! {
                    button {
                        class: "dioxus-audio__record-action dioxus-audio__record-action--record",
                        r#type: "button",
                        aria_label: if has_completed { "Clear completed recording before starting" } else { "Start recording" },
                        disabled: has_completed,
                        onclick: move |_| { let _ = recorder.start(); },
                        Mic { size: 24 }
                    }
                },
                RecorderStatus::RequestingPermission => rsx! {
                    button {
                        class: "dioxus-audio__record-action",
                        r#type: "button",
                        aria_label: "Cancel microphone request",
                        onclick: move |_| {
                            if recorder.cancel().is_ok()
                                && let Some(on_cancelled) = on_cancelled
                            {
                                on_cancelled.call(());
                            }
                        },
                        X { size: 20 }
                    }
                },
                RecorderStatus::Recording | RecorderStatus::Paused => rsx! {
                    button {
                        class: "dioxus-audio__record-action",
                        r#type: "button",
                        aria_label: "Cancel recording",
                        onclick: move |_| {
                            if recorder.cancel().is_ok()
                                && let Some(on_cancelled) = on_cancelled
                            {
                                on_cancelled.call(());
                            }
                        },
                        X { size: 20 }
                    }
                    button {
                        class: "dioxus-audio__record-action",
                        r#type: "button",
                        aria_label: if paused { "Resume" } else { "Pause" },
                        onclick: move |_| {
                            if paused {
                                let _ = recorder.resume();
                            } else {
                                let _ = recorder.pause();
                            }
                        },
                        if paused {
                            Play { size: 20 }
                        } else {
                            Pause { size: 20 }
                        }
                    }
                    button {
                        class: "dioxus-audio__record-action dioxus-audio__record-action--stop",
                        r#type: "button",
                        aria_label: "Stop recording",
                        onclick: move |_| { let _ = recorder.stop(); },
                        Square { size: 22 }
                    }
                },
                RecorderStatus::Stopping => rsx! {
                    button {
                        class: "dioxus-audio__record-action dioxus-audio__record-action--stop",
                        r#type: "button",
                        aria_label: "Finishing recording",
                        disabled: true,
                        Square { size: 22 }
                    }
                },
            }
        }
    }
}

fn recorder_state_name(status: &RecorderStatus) -> &'static str {
    match status {
        RecorderStatus::Idle => "idle",
        RecorderStatus::RequestingPermission => "requesting",
        RecorderStatus::Recording => "recording",
        RecorderStatus::Paused => "paused",
        RecorderStatus::Stopping => "stopping",
        RecorderStatus::Failed(_) => "error",
    }
}
