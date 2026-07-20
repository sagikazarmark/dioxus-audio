//! Router and shared application shell.

use dioxus::prelude::*;
use dioxus_audio::components::AudioStyles;

use crate::components::{DemoFooter, DemoHeader, Sidebar, SidebarNavLink, SidebarNavSection};
use crate::pages::{
    analysis::Analysis, decoding::Decoding, devices::Devices, home::Home, not_found::NotFound,
    playback::Playback, recorder::Recorder, styles::Styles, visualizers::Visualizers,
    waveforms::Waveforms,
};

const STYLE: Asset = asset!("/build/style.css");

#[derive(Routable, Clone, PartialEq, Debug)]
pub enum Route {
    #[layout(DemoLayout)]
    #[route("/")]
    Home {},
    #[route("/recorder")]
    Recorder {},
    #[route("/playback")]
    Playback {},
    #[route("/devices")]
    Devices {},
    #[route("/visualizers")]
    Visualizers {},
    #[route("/waveforms")]
    Waveforms {},
    #[route("/analysis")]
    Analysis {},
    #[route("/decoding")]
    Decoding {},
    #[route("/styles")]
    Styles {},
    #[route("/:..segments")]
    NotFound { segments: Vec<String> },
}

#[component]
pub fn App() -> Element {
    rsx! {
        document::Stylesheet { href: STYLE }
        AudioStyles {}
        Router::<Route> {}
    }
}

#[component]
fn DemoLayout() -> Element {
    rsx! {
        div { class: "min-h-screen bg-base-100 text-base-content",
            DemoHeader {
                home: Route::Home {},
                mark: "da",
                name: "dioxus-audio",
                github_url: "https://github.com/sagikazarmark/dioxus-audio",
            }
            div { class: "mx-auto w-full max-w-7xl lg:flex lg:gap-8 lg:px-6",
                Sidebar {
                    SidebarNavSection { label: "Start",
                        SidebarNavLink { route: Route::Home {}, label: "Overview" }
                        SidebarNavLink { route: Route::Recorder {}, label: "Record and review" }
                        SidebarNavLink { route: Route::Playback {}, label: "Playback" }
                        SidebarNavLink { route: Route::Styles {}, label: "Style customization" }
                    }
                    SidebarNavSection { label: "Input",
                        SidebarNavLink { route: Route::Devices {}, label: "Audio devices" }
                        SidebarNavLink { route: Route::Visualizers {}, label: "Live analysis" }
                    }
                    SidebarNavSection { label: "Processing",
                        SidebarNavLink { route: Route::Decoding {}, label: "Decoded Audio" }
                        SidebarNavLink { route: Route::Waveforms {}, label: "Waveforms" }
                        SidebarNavLink { route: Route::Analysis {}, label: "Analysis helpers" }
                    }
                }
                main { id: "main-content", class: "min-w-0 flex-1 px-4 py-8 sm:px-6 lg:px-0 lg:py-12",
                    Outlet::<Route> {}
                }
            }
            DemoFooter {
                description: "A docs-by-example gallery for dioxus-audio.",
                links: rsx! {
                    a {
                        class: "hover:text-base-content",
                        href: "https://github.com/sagikazarmark/dioxus-audio",
                        target: "_blank",
                        rel: "noopener noreferrer",
                        "Repository"
                    }
                    a {
                        class: "hover:text-base-content",
                        href: "https://docs.rs/dioxus-audio",
                        target: "_blank",
                        rel: "noopener noreferrer",
                        "API docs"
                    }
                },
            }
        }
    }
}
