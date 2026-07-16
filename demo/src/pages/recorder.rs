use dioxus::prelude::*;
use dioxus_code::{Code, code};

use crate::components::{ExampleSection, InlineCode, PageHeader, snippet_theme};
use crate::examples::recorder::RecorderExample;

#[component]
pub fn Recorder() -> Element {
    rsx! {
        PageHeader {
            eyebrow: "Recording",
            title: "Capture, inspect, and replay",
            intro: "A complete browser recording flow with input selection, permission state, live analysis, generated peaks, and playback of the finished capture.",
        }
        ExampleSection {
            title: "use_audio_recorder",
            intro: rsx! {
                "The recorder owns the MediaRecorder lifecycle. Consume "
                InlineCode { "recorder.completed()" }
                " with " InlineCode { "take_completed()" }
                " and pass its AudioData directly to the player."
            },
            demo: rsx! { RecorderExample {} },
            code: rsx! {
                Code { src: code!("/src/examples/recorder.rs"), theme: snippet_theme() }
            },
        }
    }
}
