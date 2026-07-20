use dioxus::prelude::*;
use dioxus_code::{Code, code};

use crate::components::{DocsCallout, ExampleSection, InlineCode, PageHeader, snippet_theme};
use crate::examples::decoding::DecodingExample;

#[component]
pub fn Decoding() -> Element {
    rsx! {
        PageHeader {
            eyebrow: "Processing",
            title: "Decode complete audio into planar samples",
            intro: "Consume Audio Data and inspect immutable Decoded Audio with channel-preserving samples and browser-effective metadata.",
        }
        DocsCallout {
            title: "The Rust-copy limit applies after browser decode",
            "The default 128 MiB ceiling bounds the Rust-owned planar copy. The browser may already hold decoded PCM before the library can check that limit, and successful materialization may briefly retain both representations."
        }
        ExampleSection {
            title: "Complete-file decoding",
            intro: rsx! {
                "The generated stereo WAV declares an 8 kHz source rate. "
                InlineCode { "DecodedAudio::sample_rate" }
                " reports the operation context's effective rate instead. Decode rejection intentionally combines malformed input, unsupported codecs, and browser decoder refusal."
            },
            demo: rsx! { DecodingExample {} },
            code: rsx! {
                Code { src: code!("/src/examples/decoding.rs"), theme: snippet_theme() }
            },
        }
    }
}
