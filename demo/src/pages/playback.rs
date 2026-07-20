use dioxus::prelude::*;
use dioxus_code::{Code, code};

use crate::components::{ExampleLayout, ExampleSection, InlineCode, PageHeader, snippet_theme};
use crate::examples::playback::{PlaybackExample, UrlPlaybackExample};

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

#[component]
pub fn PlaybackSource() -> Element {
    rsx! {
        PageHeader {
            eyebrow: "Playback Source",
            title: "Load local and remote media by URL",
            intro: "Provide one URL or ordered typed alternatives, choose eager acquisition or genuinely dormant on-play loading, and observe waiting, stalled, buffered, and seekable state without treating media time as byte progress.",
        }
        ExampleSection {
            title: "URL Playback Source",
            layout: ExampleLayout::Columns,
            intro: rsx! {
                "The application owns every URL while Playback skips definitely unsupported media types, selects the first playable alternative, and scopes network and range observations to that source attempt."
            },
            demo: rsx! { UrlPlaybackExample {} },
            code: rsx! {
                Code { src: code!("/src/examples/playback.rs"), theme: snippet_theme() }
            },
        }
    }
}
