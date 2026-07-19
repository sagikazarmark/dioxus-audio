use dioxus::prelude::*;
use dioxus_icons::lucide::{Mic, Pause, Play, Square, X};

use crate::AudioErrorKind;
use crate::recorder::{AudioRecorder, RecorderStatus};

/// Localizable messages emitted by [`RecorderStatusAnnouncer`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecorderAnnouncementLabels {
    pub idle: String,
    pub requesting: String,
    pub recording: String,
    pub paused: String,
    pub stopping: String,
    pub failed: String,
}

impl Default for RecorderAnnouncementLabels {
    fn default() -> Self {
        Self {
            idle: "Recorder idle".to_string(),
            requesting: "Requesting microphone access".to_string(),
            recording: "Recording".to_string(),
            paused: "Recording paused".to_string(),
            stopping: "Finishing recording".to_string(),
            failed: "Recording failed".to_string(),
        }
    }
}

/// An optional polite live region for coarse Recorder state changes.
#[component]
pub fn RecorderStatusAnnouncer(
    recorder: AudioRecorder,
    #[props(default)] labels: RecorderAnnouncementLabels,
) -> Element {
    let status = recorder.status()();
    let message = match status {
        RecorderStatus::Idle => labels.idle.as_str(),
        RecorderStatus::RequestingPermission => labels.requesting.as_str(),
        RecorderStatus::Recording => labels.recording.as_str(),
        RecorderStatus::Paused => labels.paused.as_str(),
        RecorderStatus::Stopping => labels.stopping.as_str(),
        RecorderStatus::Failed(_) => labels.failed.as_str(),
    };

    rsx! {
        div {
            class: "dioxus-audio dioxus-audio__status-announcer",
            role: "status",
            aria_live: "polite",
            aria_atomic: "true",
            "{message}"
        }
    }
}

/// A native button that starts a Recording.
#[component]
pub fn RecorderStartButton(
    recorder: AudioRecorder,
    #[props(default = "Start recording".to_string())] label: String,
    #[props(default = "Clear recorded audio before starting".to_string())] completed_label: String,
) -> Element {
    let status = recorder.status()();
    let has_completed = recorder.completed().read().is_some();
    let disabled = has_completed || !can_start(&status);
    let aria_label = if has_completed {
        completed_label
    } else {
        label
    };

    rsx! {
        button {
            class: "dioxus-audio dioxus-audio__record-action dioxus-audio__record-action--record",
            r#type: "button",
            aria_label,
            disabled,
            onclick: move |_| { let _ = recorder.start(); },
            Mic { size: 24 }
        }
    }
}

/// A native button that cancels a pending or active Recording.
#[component]
pub fn RecorderCancelButton(
    recorder: AudioRecorder,
    #[props(default = "Cancel microphone request".to_string())] request_label: String,
    #[props(default = "Cancel recording".to_string())] recording_label: String,
    #[props(default)] on_cancelled: Option<EventHandler<()>>,
) -> Element {
    let status = recorder.status()();
    let requesting = matches!(status, RecorderStatus::RequestingPermission);
    let disabled = !can_cancel(&status);
    let aria_label = if requesting {
        request_label
    } else {
        recording_label
    };

    rsx! {
        button {
            class: "dioxus-audio dioxus-audio__record-action",
            r#type: "button",
            aria_label,
            disabled,
            onclick: move |_| {
                if recorder.cancel().is_ok()
                    && let Some(on_cancelled) = on_cancelled
                {
                    on_cancelled.call(());
                }
            },
            X { size: 20 }
        }
    }
}

/// A stable native button that pauses or resumes a Recording.
#[component]
pub fn RecorderPauseResumeButton(
    recorder: AudioRecorder,
    #[props(default = "Pause".to_string())] pause_label: String,
    #[props(default = "Resume".to_string())] resume_label: String,
) -> Element {
    let status = recorder.status()();
    let paused = matches!(status, RecorderStatus::Paused);
    let disabled = !can_control_transport(&status);
    let aria_label = if paused { resume_label } else { pause_label };

    rsx! {
        button {
            class: "dioxus-audio dioxus-audio__record-action",
            r#type: "button",
            aria_label,
            disabled,
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
    }
}

/// A native button that finishes a Recording.
#[component]
pub fn RecorderStopButton(
    recorder: AudioRecorder,
    #[props(default = "Stop recording".to_string())] stop_label: String,
    #[props(default = "Finishing recording".to_string())] stopping_label: String,
) -> Element {
    let status = recorder.status()();
    let stopping = matches!(status, RecorderStatus::Stopping);
    let disabled = !can_control_transport(&status);
    let aria_label = if stopping { stopping_label } else { stop_label };

    rsx! {
        button {
            class: "dioxus-audio dioxus-audio__record-action dioxus-audio__record-action--stop",
            r#type: "button",
            aria_label,
            aria_busy: stopping,
            disabled,
            onclick: move |_| { let _ = recorder.stop(); },
            Square { size: 22 }
        }
    }
}

/// A native button that discards the retained completed Recorded Audio.
#[component]
pub fn RecorderClearButton(
    recorder: AudioRecorder,
    #[props(default = "Clear recorded audio".to_string())] label: String,
) -> Element {
    let disabled = recorder.completed().read().is_none();

    rsx! {
        button {
            class: "dioxus-audio dioxus-audio__record-action",
            r#type: "button",
            aria_label: label,
            disabled,
            onclick: move |_| recorder.clear_completed(),
            X { size: 20 }
        }
    }
}

fn can_start(status: &RecorderStatus) -> bool {
    match status {
        RecorderStatus::Idle => true,
        RecorderStatus::Failed(error) => !matches!(
            error.kind(),
            AudioErrorKind::UnsupportedPlatform | AudioErrorKind::InvalidConfiguration
        ),
        RecorderStatus::RequestingPermission
        | RecorderStatus::Recording
        | RecorderStatus::Paused
        | RecorderStatus::Stopping => false,
    }
}

fn can_cancel(status: &RecorderStatus) -> bool {
    matches!(
        status,
        RecorderStatus::RequestingPermission | RecorderStatus::Recording | RecorderStatus::Paused
    )
}

fn can_control_transport(status: &RecorderStatus) -> bool {
    matches!(status, RecorderStatus::Recording | RecorderStatus::Paused)
}

#[component]
pub fn RecorderControls(
    recorder: AudioRecorder,
    #[props(default)] on_cancelled: Option<EventHandler<()>>,
) -> Element {
    let status = recorder.status()();
    let state_name = recorder_state_name(&status);

    rsx! {
        div {
            class: "dioxus-audio dioxus-audio__recorder-controls",
            "data-state": state_name,
            match status {
                RecorderStatus::Idle | RecorderStatus::Failed(_) => rsx! {
                    RecorderStartButton { recorder }
                },
                RecorderStatus::RequestingPermission => rsx! {
                    RecorderCancelButton { recorder, on_cancelled }
                },
                RecorderStatus::Recording | RecorderStatus::Paused => rsx! {
                    RecorderCancelButton { recorder, on_cancelled }
                    RecorderPauseResumeButton { recorder }
                    RecorderStopButton { recorder }
                },
                RecorderStatus::Stopping => rsx! {
                    RecorderStopButton { recorder }
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
