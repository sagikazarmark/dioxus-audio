//! Reusable documentation callouts.

use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
pub struct ExternalAction {
    label: String,
    href: String,
}

impl ExternalAction {
    pub fn new(label: impl Into<String>, href: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            href: href.into(),
        }
    }
}

#[component]
pub fn DocsCallout(
    #[props(into)] title: String,
    #[props(default)] action: Option<ExternalAction>,
    children: Element,
) -> Element {
    rsx! {
        div { class: "mt-8 rounded-2xl border border-info/40 bg-info/5 p-5",
            div { class: "flex items-center gap-2",
                span { class: "text-sm font-semibold uppercase tracking-wider text-info", "Docs" }
                p { class: "font-semibold text-base-content", "{title}" }
            }
            div { class: "mt-2 max-w-[70ch] text-sm leading-6 text-base-content/70", {children} }
            if let Some(action) = action {
                div { class: "mt-4",
                    a {
                        class: "btn btn-sm btn-outline btn-info",
                        href: "{action.href}",
                        target: "_blank",
                        rel: "noopener noreferrer",
                        "{action.label}"
                    }
                }
            }
        }
    }
}

#[component]
pub fn StatusChip(#[props(into)] label: String) -> Element {
    rsx! {
        span { class: "badge badge-ghost badge-sm font-mono", "{label}" }
    }
}
