//! Layout and state for documented live examples.

use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum ExampleLayout {
    #[default]
    Tabbed,
    Columns,
}

#[derive(Clone, Copy, PartialEq)]
enum SectionTab {
    Demo,
    Source,
}

#[component]
pub fn ExampleSection(
    #[props(into)] title: String,
    intro: Element,
    demo: Element,
    code: Element,
    #[props(default)] layout: ExampleLayout,
) -> Element {
    let mut tab = use_signal(|| SectionTab::Demo);

    let demo_frame = rsx! {
        div { class: "rounded-2xl border border-base-300 bg-base-200/40 p-5", {demo} }
    };
    let code_frame = rsx! {
        div { class: "overflow-x-auto rounded-2xl border border-base-300 bg-base-200/60 p-4 text-sm [&_pre]:!bg-transparent",
            {code}
        }
    };

    let body = match layout {
        ExampleLayout::Tabbed => rsx! {
            div { role: "group", aria_label: "Example view", class: "mt-6 tabs tabs-border",
                button {
                    r#type: "button",
                    class: if tab() == SectionTab::Demo { "tab tab-active" } else { "tab" },
                    aria_pressed: tab() == SectionTab::Demo,
                    onclick: move |_| tab.set(SectionTab::Demo),
                    "Demo"
                }
                button {
                    r#type: "button",
                    class: if tab() == SectionTab::Source { "tab tab-active" } else { "tab" },
                    aria_pressed: tab() == SectionTab::Source,
                    onclick: move |_| tab.set(SectionTab::Source),
                    "Source"
                }
            }
            // Keep both panes mounted so source tab changes do not reset a live recording.
            div {
                role: "region",
                aria_label: "Example demo",
                class: if tab() == SectionTab::Demo { "mt-4" } else { "mt-4 hidden" },
                {demo_frame}
            }
            div {
                role: "region",
                aria_label: "Example source",
                class: if tab() == SectionTab::Source { "mt-4" } else { "mt-4 hidden" },
                {code_frame}
            }
        },
        ExampleLayout::Columns => rsx! {
            div { class: "mt-6 grid gap-6 xl:grid-cols-2",
                div { class: "min-w-0",
                    p { class: "mb-3 text-xs font-semibold uppercase tracking-wider text-base-content/45", "Demo" }
                    {demo_frame}
                }
                div { class: "min-w-0",
                    p { class: "mb-3 text-xs font-semibold uppercase tracking-wider text-base-content/45", "Source" }
                    {code_frame}
                }
            }
        },
    };

    rsx! {
        section { class: "mt-10 rounded-[2rem] border border-base-300 bg-base-100 p-6 shadow-sm sm:p-8",
            h2 { class: "text-xl font-semibold tracking-tight", "{title}" }
            p { class: "mt-2 max-w-[70ch] text-sm leading-6 text-base-content/65", {intro} }
            {body}
        }
    }
}
