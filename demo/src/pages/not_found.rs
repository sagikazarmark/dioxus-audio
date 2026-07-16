use dioxus::prelude::*;

use crate::app::Route;
use crate::components::PageHeader;

#[component]
pub fn NotFound(segments: Vec<String>) -> Element {
    let path = format!("/{}", segments.join("/"));

    rsx! {
        PageHeader {
            eyebrow: "404",
            title: "Page not found",
            intro: format!("The demo has no page at {path}."),
        }
        Link { to: Route::Home {}, class: "btn btn-primary mt-8", "Back to overview" }
    }
}
