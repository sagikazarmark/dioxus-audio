use dioxus::prelude::*;
use dioxus_code::{Code, code};

use crate::components::{ExampleLayout, ExampleSection, InlineCode, PageHeader, snippet_theme};
use crate::examples::devices::DevicesExample;

#[component]
pub fn Devices() -> Element {
    rsx! {
        PageHeader {
            eyebrow: "Input",
            title: "Discover and select microphones",
            intro: "Enumerate browser audio inputs, request access explicitly, and react to devices being connected or removed.",
        }
        ExampleSection {
            title: "use_audio_input_devices",
            layout: ExampleLayout::Columns,
            intro: rsx! {
                "Device labels usually become available only after "
                InlineCode { "request_permission()" }
                ". The selected input is validated whenever the list refreshes."
            },
            demo: rsx! { DevicesExample {} },
            code: rsx! {
                Code { src: code!("/src/examples/devices.rs"), theme: snippet_theme() }
            },
        }
    }
}
