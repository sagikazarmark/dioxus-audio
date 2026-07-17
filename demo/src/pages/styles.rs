//! Style customization guide.

use dioxus::prelude::*;

use crate::components::{InlineCode, PageHeader};
use crate::examples::styles::{ScopedExample, StudioExample};

const SETUP_SOURCE: &str = include_str!("../../snippets/styles_setup.rs");
const STUDIO_MODULE: &str = include_str!("../examples/styles/studio.rs");
const STUDIO_STYLESHEET: &str = include_str!("../examples/styles/studio.css");
const STUDIO_RUST_ASSET: Asset = asset!("/src/examples/styles/studio.rs");
const STUDIO_CSS_ASSET: Asset = asset!(
    "/src/examples/styles/studio.css",
    AssetOptions::css().with_minify(false)
);
const SCOPED_MODULE: &str = include_str!("../examples/styles/scoped.rs");
const SCOPED_STYLESHEET: &str = include_str!("../examples/styles/scoped.css");
const SCOPED_RUST_ASSET: Asset = asset!("/src/examples/styles/scoped.rs");
const SCOPED_CSS_ASSET: Asset = asset!(
    "/src/examples/styles/scoped.css",
    AssetOptions::css().with_minify(false)
);

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
            SourceBlock { language: "rust", source: SETUP_SOURCE }
        }

        section { aria_labelledby: "cascade-heading", class: "mt-10 rounded-2xl border border-base-300 bg-base-200/35 p-5",
            h2 { id: "cascade-heading", class: "text-lg font-semibold", "How the cascade resolves" }
            p { class: "mt-2 max-w-[75ch] text-sm leading-6 text-base-content/65",
                "Public "
                InlineCode { "--dioxus-audio-*" }
                " properties inherit normally. Set them on an app-level ancestor for app-wide branding; the nearest scoped wrapper supplies local values. Each omitted token falls back independently to its daisyUI variable, when available, and then to the standalone default. daisyUI is optional."
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
                        language: "rust",
                        source: recipe_region(STUDIO_MODULE, "studio-recipe"),
                        asset: STUDIO_RUST_ASSET,
                    }
                    SourceRecipe {
                        title: "Studio stylesheet",
                        language: "css",
                        source: STUDIO_STYLESHEET,
                        asset: STUDIO_CSS_ASSET,
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
                        language: "rust",
                        source: recipe_region(SCOPED_MODULE, "scoped-recipe"),
                        asset: SCOPED_RUST_ASSET,
                    }
                    SourceRecipe {
                        title: "Scoped-theme stylesheet",
                        language: "css",
                        source: SCOPED_STYLESHEET,
                        asset: SCOPED_CSS_ASSET,
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
fn SourceBlock(#[props(into)] language: String, #[props(into)] source: String) -> Element {
    rsx! {
        pre { class: "mt-4 overflow-x-auto rounded-2xl border border-base-300 bg-base-200/60 p-4 text-xs leading-5 text-base-content/80",
            code { "data-recipe-language": language, "{source}" }
        }
    }
}

#[component]
fn SourceRecipe(
    #[props(into)] title: String,
    #[props(into)] language: String,
    #[props(into)] source: String,
    asset: Asset,
) -> Element {
    rsx! {
        article { class: "min-w-0 rounded-2xl border border-base-300 bg-base-200/35 p-4",
            div { class: "flex items-center justify-between gap-3",
                h3 { class: "text-sm font-semibold", "{title}" }
                a {
                    href: asset,
                    class: "text-xs font-medium text-primary underline underline-offset-4",
                    target: "_blank",
                    rel: "noopener noreferrer",
                    "View production source"
                }
            }
            pre { class: "mt-3 max-h-[36rem] overflow-auto text-xs leading-5 text-base-content/80",
                code { "data-recipe-language": language, "{source}" }
            }
        }
    }
}

fn recipe_region<'a>(source: &'a str, name: &str) -> &'a str {
    let start_marker = format!("// region: {name}");
    let end_marker = format!("// endregion: {name}");
    assert_eq!(
        source.match_indices(&start_marker).count(),
        1,
        "recipe region {name:?} must have exactly one start marker"
    );
    assert_eq!(
        source.match_indices(&end_marker).count(),
        1,
        "recipe region {name:?} must have exactly one end marker"
    );

    let start = source
        .find(&start_marker)
        .expect("validated recipe start marker")
        + start_marker.len();
    let rest = &source[start..];
    let end = rest.find(&end_marker).unwrap_or_else(|| {
        panic!("recipe region {name:?} end marker must follow its start marker")
    });
    let region = rest[..end].strip_prefix('\n').unwrap_or(&rest[..end]);
    let region = region.strip_suffix('\n').unwrap_or(region);
    assert!(
        !region.trim().is_empty(),
        "recipe region {name:?} must not be empty"
    );
    region
}
