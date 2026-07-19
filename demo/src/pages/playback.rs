use dioxus::prelude::*;
use dioxus_code::{Code, code};

use crate::components::{ExampleLayout, ExampleSection, InlineCode, PageHeader, snippet_theme};
use crate::examples::playback::PlaybackExample;

#[component]
pub fn Playback() -> Element {
    rsx! {
        PageHeader {
            eyebrow: "Playback",
            title: "Load audio only when it is needed",
            intro: "Use the composed Playback control or arrange native transport, mute, best-effort audibility, repeat, seek, skip, and rate controls around the same Controller.",
        }
        ExampleSection {
            title: "AudioPlayer",
            layout: ExampleLayout::Columns,
            intro: rsx! {
                InlineCode { "on_request_audio" }
                " lets storage-backed applications fetch bytes on the first play instead of eagerly loading every recording."
            },
            demo: rsx! { PlaybackExample {} },
            code: rsx! {
                Code { src: code!("/src/examples/playback.rs"), theme: snippet_theme() }
            },
        }
    }
}
