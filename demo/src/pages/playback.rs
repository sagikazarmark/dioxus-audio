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
            intro: "Choose eager acquisition or keep a URL-addressable Playback Source genuinely dormant until Playback is requested, while retaining authoritative replace and unload behavior.",
        }
        ExampleSection {
            title: "URL Playback Source",
            layout: ExampleLayout::Columns,
            intro: rsx! {
                "The application owns the URL while the Playback Controller owns its attached media resource and stale-outcome protection."
            },
            demo: rsx! { UrlPlaybackExample {} },
            code: rsx! {
                Code { src: code!("/src/examples/playback.rs"), theme: snippet_theme() }
            },
        }
    }
}
