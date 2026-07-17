use dioxus::prelude::*;
use dioxus_audio::components::{AudioStyles, STYLESHEET};

#[component]
fn App() -> Element {
    rsx! {
        AudioStyles {}
        Router::<Route> {}
    }
}

// Lower-level equivalent when the application manages document stylesheets itself:
#[component]
fn AppWithExplicitStylesheet() -> Element {
    rsx! {
        document::Stylesheet { href: STYLESHEET }
        Router::<Route> {}
    }
}
