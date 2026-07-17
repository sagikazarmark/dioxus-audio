//! Style customization guide.

use dioxus::prelude::*;
use dioxus_code::{Code, Language, SourceCode};

use crate::components::{InlineCode, PageHeader, snippet_theme};
use crate::examples::styles::{DaisyExample, ScopedExample, StudioExample};

const SETUP_SOURCE: &str = include_str!("../../snippets/styles_setup.rs");
const STUDIO_MODULE: &str = include_str!("../examples/styles/studio.rs");
const STUDIO_STYLESHEET: &str = include_str!("../examples/styles/studio.css");
const SCOPED_MODULE: &str = include_str!("../examples/styles/scoped.rs");
const SCOPED_STYLESHEET: &str = include_str!("../examples/styles/scoped.css");
const DAISY_MODULE: &str = include_str!("../examples/styles/daisy.rs");

#[derive(Clone, Copy)]
struct StyleToken {
    public: &'static str,
    role: &'static str,
    daisy_fallback: &'static str,
    standalone_default: &'static str,
}

const STYLE_TOKENS: [StyleToken; 10] = [
    StyleToken {
        public: "--dioxus-audio-base-100",
        role: "Primary surfaces",
        daisy_fallback: "--color-base-100",
        standalone_default: "#ffffff",
    },
    StyleToken {
        public: "--dioxus-audio-base-200",
        role: "Secondary controls and surfaces",
        daisy_fallback: "--color-base-200",
        standalone_default: "#f3f4f6",
    },
    StyleToken {
        public: "--dioxus-audio-base-300",
        role: "Borders and tracks",
        daisy_fallback: "--color-base-300",
        standalone_default: "#d1d5db",
    },
    StyleToken {
        public: "--dioxus-audio-content",
        role: "Text and neutral controls",
        daisy_fallback: "--color-base-content",
        standalone_default: "#18181b",
    },
    StyleToken {
        public: "--dioxus-audio-primary",
        role: "Active controls, Waveforms, and focus",
        daisy_fallback: "--color-primary",
        standalone_default: "#2563eb",
    },
    StyleToken {
        public: "--dioxus-audio-primary-content",
        role: "Content on primary controls",
        daisy_fallback: "--color-primary-content",
        standalone_default: "#ffffff",
    },
    StyleToken {
        public: "--dioxus-audio-warning",
        role: "Muted, paused, and caution states",
        daisy_fallback: "--color-warning",
        standalone_default: "#d97706",
    },
    StyleToken {
        public: "--dioxus-audio-error",
        role: "Denied, failed, Recording, and stop states",
        daisy_fallback: "--color-error",
        standalone_default: "#dc2626",
    },
    StyleToken {
        public: "--dioxus-audio-success",
        role: "Ready states and meters",
        daisy_fallback: "--color-success",
        standalone_default: "#16a34a",
    },
    StyleToken {
        public: "--dioxus-audio-radius",
        role: "Component corner treatment",
        daisy_fallback: "--radius-field",
        standalone_default: "0.5rem",
    },
];

#[component]
pub fn Styles() -> Element {
    rsx! {
        PageHeader {
            eyebrow: "Style customization",
            title: "Make audio UI belong to your application",
            intro: "Load the component stylesheet once, then follow the cascade from a complete app-wide brand to bounded overrides and automatic host-theme fallbacks.",
        }

        section { aria_labelledby: "styles-setup-heading", class: "mt-10",
            h2 { id: "styles-setup-heading", class: "text-2xl font-semibold tracking-tight", "Stylesheet setup" }
            p { class: "mt-2 max-w-[70ch] text-sm leading-6 text-base-content/65",
                "Render "
                InlineCode { "AudioStyles" }
                " once near the application root. Use "
                InlineCode { "STYLESHEET" }
                " only when the application needs the equivalent lower-level document stylesheet API."
            }
            SourceBlock { language: Language::Rust, source: SETUP_SOURCE }
        }

        section { aria_labelledby: "cascade-heading", class: "mt-10 rounded-2xl border border-base-300 bg-base-200/35 p-5",
            h2 { id: "cascade-heading", class: "text-lg font-semibold", "How the cascade resolves" }
            p { class: "mt-2 max-w-[75ch] text-sm leading-6 text-base-content/65",
                "Public "
                InlineCode { "--dioxus-audio-*" }
                " properties inherit normally. The nearest explicit package value on the component or an ancestor wins, supporting both app-wide branding and local wrapper scopes. When no inherited package value exists, each omitted token independently falls through to its daisyUI variable, when available, and then to the standalone default. daisyUI is optional."
            }
        }

        ol { class: "styles-orientation mt-8 grid gap-3 text-sm sm:grid-cols-3",
            GuideStep { number: "01", title: "Brand the application", body: "Set the complete public palette on one app ancestor." }
            GuideStep { number: "02", title: "Scope an instance", body: "Put focused overrides on an ordinary local wrapper." }
            GuideStep { number: "03", title: "Use theme fallbacks", body: "Omit package tokens when the host theme should lead." }
        }

        section { aria_labelledby: "studio-heading", class: "mt-14",
            p { class: "text-xs font-semibold uppercase tracking-[0.18em] text-primary", "01 / App-wide" }
            h2 { id: "studio-heading", class: "mt-2 text-2xl font-semibold tracking-tight", "Studio: one complete app-wide theme" }
            p { class: "mt-5 text-xs font-semibold uppercase tracking-wider text-base-content/45", "What to notice" }
            p { class: "mt-2 max-w-[75ch] text-sm leading-6 text-base-content/65",
                "Every audio component inherits the same ten values from the bounded Studio application ancestor. The surrounding guide keeps the demo theme."
            }

            p { class: "mt-6 mb-3 text-xs font-semibold uppercase tracking-wider text-base-content/45", "Live demonstration" }
            StudioExample {}

            div { class: "mt-6",
                p { class: "text-xs font-semibold uppercase tracking-wider text-base-content/45", "Exact source recipe" }
                p { class: "mt-2 max-w-[75ch] text-sm leading-6 text-base-content/65",
                    "Repeated imports and deterministic Peaks and Audio Data from "
                    InlineCode { "fixtures.rs" }
                    " are omitted. The Rust below is extracted from the compiled chapter; the CSS is the imported Studio stylesheet."
                }
                div { class: "mt-4 grid gap-4 xl:grid-cols-2",
                    SourceRecipe {
                        title: "Rust composition",
                        language: Language::Rust,
                        source: recipe_region(STUDIO_MODULE, "studio-recipe"),
                    }
                    SourceRecipe {
                        title: "Studio stylesheet",
                        language: Language::Css,
                        source: STUDIO_STYLESHEET,
                    }
                }
            }

            div { class: "mt-5 rounded-2xl border border-base-300 bg-base-100 p-5",
                h3 { class: "font-semibold", "Why it works" }
                p { class: "mt-2 max-w-[75ch] text-sm leading-6 text-base-content/65",
                    "CSS custom properties cross the component boundary through inheritance. The Studio card itself is application-owned presentation; the package consumes the semantic values without requiring component-specific styling hooks."
                }
            }
        }

        section { aria_labelledby: "scoped-heading", class: "mt-14",
            p { class: "text-xs font-semibold uppercase tracking-[0.18em] text-primary", "02 / Per-instance" }
            h2 { id: "scoped-heading", class: "mt-2 text-2xl font-semibold tracking-tight", "Citrus and Midnight: independently scoped clip editors" }
            p { class: "mt-5 text-xs font-semibold uppercase tracking-wider text-base-content/45", "What to notice" }
            p { class: "mt-2 max-w-[75ch] text-sm leading-6 text-base-content/65",
                "Both editors start from the same Audio Data, Peaks, duration, selected range, and Playback state. Only the seven values inherited from each nearest wrapper differ."
            }

            p { class: "mt-6 mb-3 text-xs font-semibold uppercase tracking-wider text-base-content/45", "Live demonstration" }
            ScopedExample {}

            div { class: "mt-6",
                p { class: "text-xs font-semibold uppercase tracking-wider text-base-content/45", "Exact source recipe" }
                p { class: "mt-2 max-w-[75ch] text-sm leading-6 text-base-content/65",
                    "Repeated imports and deterministic Peaks and Audio Data from "
                    InlineCode { "fixtures.rs" }
                    " are omitted. The Rust below is extracted from the compiled scoped chapter; the CSS is the imported scoped-theme stylesheet."
                }
                div { class: "mt-4 grid gap-4 xl:grid-cols-2",
                    SourceRecipe {
                        title: "Rust composition",
                        language: Language::Rust,
                        source: recipe_region(SCOPED_MODULE, "scoped-recipe"),
                    }
                    SourceRecipe {
                        title: "Scoped-theme stylesheet",
                        language: Language::Css,
                        source: SCOPED_STYLESHEET,
                    }
                }
            }

            div { class: "mt-5 rounded-2xl border border-base-300 bg-base-100 p-5",
                h3 { class: "font-semibold", "Why it works" }
                p { class: "mt-2 max-w-[75ch] text-sm leading-6 text-base-content/65",
                    "Each ordinary wrapper establishes a local inheritance scope without adding a component-specific API. The clip-editor surfaces are application-owned wrapper styling; the package only consumes its public semantic values."
                }
            }
        }

        section { aria_labelledby: "daisy-heading", class: "mt-14",
            p { class: "text-xs font-semibold uppercase tracking-[0.18em] text-primary", "03 / Host theme" }
            h2 { id: "daisy-heading", class: "mt-2 text-2xl font-semibold tracking-tight", "daisyUI: automatic host-theme fallback" }
            p { class: "mt-5 text-xs font-semibold uppercase tracking-wider text-base-content/45", "What to notice" }
            p { class: "mt-2 max-w-[75ch] text-sm leading-6 text-base-content/65",
                "The Waveform and Playback controls follow the demo's light and dark theme. This example deliberately declares no "
                InlineCode { "--dioxus-audio-*" }
                " properties."
            }

            p { class: "mt-6 mb-3 text-xs font-semibold uppercase tracking-wider text-base-content/45", "Live demonstration" }
            DaisyExample {}

            div { class: "mt-6",
                p { class: "text-xs font-semibold uppercase tracking-wider text-base-content/45", "Exact source recipe" }
                p { class: "mt-2 max-w-[75ch] text-sm leading-6 text-base-content/65",
                    "Repeated imports and deterministic Peaks and Audio Data from "
                    InlineCode { "fixtures.rs" }
                    " are omitted. The Rust below is extracted from the compiled chapter. There is no chapter stylesheet because the absence of a package-token declaration is the demonstrated behavior."
                }
                div { class: "mt-4",
                    SourceRecipe {
                        title: "Rust composition",
                        language: Language::Rust,
                        source: recipe_region(DAISY_MODULE, "daisy-recipe"),
                    }
                }
            }

            div { class: "mt-5 rounded-2xl border border-base-300 bg-base-100 p-5",
                h3 { class: "font-semibold", "Why it works" }
                p { class: "mt-2 max-w-[75ch] text-sm leading-6 text-base-content/65",
                    "When a public package token is absent, each component value falls back to the corresponding daisyUI theme variable and then to its standalone default. daisyUI is optional and is not a package dependency."
                }
            }
        }

        section { aria_labelledby: "style-token-reference", class: "mt-14",
            p { class: "text-xs font-semibold uppercase tracking-[0.18em] text-primary", "Reference" }
            h2 { id: "style-token-reference", class: "mt-2 text-2xl font-semibold tracking-tight", "Stable styling contract" }
            p { class: "mt-3 max-w-[75ch] text-sm leading-6 text-base-content/65",
                "These inherited custom properties are the complete stable customization surface. Each row shows its semantic role and independent daisyUI-to-standalone fallback path."
            }

            div { class: "mt-6 overflow-x-auto rounded-2xl border border-base-300",
                table { class: "w-full min-w-[48rem] border-collapse text-left text-sm",
                    thead { class: "bg-base-200/60 text-xs uppercase tracking-wider text-base-content/55",
                        tr {
                            th { class: "px-4 py-3 font-semibold", scope: "col", "Public token" }
                            th { class: "px-4 py-3 font-semibold", scope: "col", "Semantic role" }
                            th { class: "px-4 py-3 font-semibold", scope: "col", "daisyUI fallback" }
                            th { class: "px-4 py-3 font-semibold", scope: "col", "Standalone default" }
                        }
                    }
                    tbody { class: "divide-y divide-base-300",
                        for token in STYLE_TOKENS {
                            tr { class: "align-top",
                                td { class: "px-4 py-3", InlineCode { "{token.public}" } }
                                td { class: "px-4 py-3 text-base-content/70", "{token.role}" }
                                td { class: "px-4 py-3", InlineCode { "{token.daisy_fallback}" } }
                                td { class: "px-4 py-3", InlineCode { "{token.standalone_default}" } }
                            }
                        }
                    }
                }
            }

            div { class: "mt-6 rounded-2xl border border-base-300 bg-base-100 p-5",
                h3 { class: "font-semibold", "Where the stable boundary ends" }
                p { class: "mt-2 max-w-[75ch] text-sm leading-6 text-base-content/65",
                    "Private "
                    InlineCode { "--_dxa-*" }
                    " aliases, component classes, DOM structure, "
                    InlineCode { "data-*" }
                    " values, ARIA attributes, and native state selectors are implementation details, not stable customization hooks."
                }
                p { class: "mt-2 max-w-[75ch] text-sm leading-6 text-base-content/65",
                    "The themed Studio and clip-editor wrapper surfaces shown above are application-owned presentation. Only the inherited public properties are part of the package styling contract."
                }
            }
        }

        aside { aria_labelledby: "application-author-responsibility", class: "mt-8 rounded-2xl border border-primary/25 bg-primary/5 p-5",
            h2 { id: "application-author-responsibility", class: "text-lg font-semibold", "Application-author responsibility" }
            p { class: "mt-2 max-w-[75ch] text-sm leading-6 text-base-content/70",
                "The package supplies semantic tokens and fallbacks. Applications choosing custom values remain responsible for readable contrast, visible focus and interaction states, and distinguishable success, warning, and error states. Verified recipes cannot guarantee arbitrary overrides."
            }
        }
    }
}

#[component]
fn GuideStep(
    #[props(into)] number: String,
    #[props(into)] title: String,
    #[props(into)] body: String,
) -> Element {
    rsx! {
        li { class: "rounded-2xl border border-base-300 bg-base-100 p-4",
            span { class: "font-mono text-xs text-primary", "{number}" }
            p { class: "mt-2 font-semibold", "{title}" }
            p { class: "mt-1 text-base-content/60", "{body}" }
        }
    }
}

#[component]
fn SourceBlock(language: Language, #[props(into)] source: String) -> Element {
    rsx! {
        div { class: "mt-4 overflow-hidden rounded-2xl border border-base-300 bg-base-200/60 text-base-content/80 [&_pre]:!bg-transparent [&_pre]:!text-xs [&_pre]:!leading-5",
            Code { src: SourceCode::new(language, source), theme: snippet_theme() }
        }
    }
}

#[component]
fn SourceRecipe(
    #[props(into)] title: String,
    language: Language,
    #[props(into)] source: String,
) -> Element {
    rsx! {
        article { class: "min-w-0 rounded-2xl border border-base-300 bg-base-200/35 p-4",
            h3 { class: "text-sm font-semibold", "{title}" }
            div { class: "mt-3 text-base-content/80 [&_pre]:max-h-[36rem] [&_pre]:!bg-transparent [&_pre]:!text-xs [&_pre]:!leading-5",
                Code { src: SourceCode::new(language, source), theme: snippet_theme() }
            }
        }
    }
}

fn recipe_region<'a>(source: &'a str, name: &str) -> &'a str {
    let start_marker = format!("// region: {name}");
    let end_marker = format!("// endregion: {name}");
    let starts = exact_line_ranges(source, &start_marker);
    let ends = exact_line_ranges(source, &end_marker);

    assert_eq!(
        starts.len(),
        1,
        "recipe region {name:?} must have exactly one start marker"
    );
    assert_eq!(
        ends.len(),
        1,
        "recipe region {name:?} must have exactly one end marker"
    );

    let body_start = starts[0].1;
    let body_end = ends[0].0;
    assert!(
        body_start <= body_end,
        "recipe region {name:?} end marker must follow its start marker"
    );
    let region = source[body_start..body_end].trim_end_matches(['\r', '\n']);
    assert!(
        !region.trim().is_empty(),
        "recipe region {name:?} must not be empty"
    );
    region
}

fn exact_line_ranges(source: &str, marker: &str) -> Vec<(usize, usize)> {
    let mut offset = 0;

    source
        .split_inclusive('\n')
        .filter_map(|line| {
            let start = offset;
            offset += line.len();
            let content = line.trim_end_matches(['\r', '\n']);
            (content == marker).then_some((start, offset))
        })
        .collect()
}
