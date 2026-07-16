//! Code presentation for the docs-by-example pages.

use dioxus::prelude::*;
use dioxus_code::{CodeTheme, Theme};

pub fn snippet_theme() -> CodeTheme {
    CodeTheme::system(Theme::GITHUB_LIGHT, Theme::TOKYO_NIGHT)
}

#[component]
pub fn InlineCode(children: Element) -> Element {
    rsx! {
        code { class: "rounded bg-base-200 px-1.5 py-0.5 font-mono text-[0.85em] text-base-content/80",
            {children}
        }
    }
}
