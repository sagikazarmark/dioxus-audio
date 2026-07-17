use std::collections::BTreeSet;

const STYLE_TOKENS: [(&str, &str, &str, &str); 10] = [
    (
        "--dioxus-audio-base-100",
        "Primary surfaces",
        "--color-base-100",
        "#ffffff",
    ),
    (
        "--dioxus-audio-base-200",
        "Secondary controls and surfaces",
        "--color-base-200",
        "#f3f4f6",
    ),
    (
        "--dioxus-audio-base-300",
        "Borders and tracks",
        "--color-base-300",
        "#d1d5db",
    ),
    (
        "--dioxus-audio-content",
        "Text and neutral controls",
        "--color-base-content",
        "#18181b",
    ),
    (
        "--dioxus-audio-primary",
        "Active controls, Waveforms, and focus",
        "--color-primary",
        "#2563eb",
    ),
    (
        "--dioxus-audio-primary-content",
        "Content on primary controls",
        "--color-primary-content",
        "#ffffff",
    ),
    (
        "--dioxus-audio-warning",
        "Muted, paused, and caution states",
        "--color-warning",
        "#d97706",
    ),
    (
        "--dioxus-audio-error",
        "Denied, failed, Recording, and stop states",
        "--color-error",
        "#dc2626",
    ),
    (
        "--dioxus-audio-success",
        "Ready states and meters",
        "--color-success",
        "#16a34a",
    ),
    (
        "--dioxus-audio-radius",
        "Component corner treatment",
        "--radius-field",
        "0.5rem",
    ),
];

#[test]
fn packaged_styles_are_scoped_and_framework_independent() {
    let css = include_str!("../assets/dioxus-audio.css");

    for forbidden in ["@import", "@source", "@plugin", ":root", "--tw-", ".btn"] {
        assert!(
            !css.contains(forbidden),
            "unexpected global style: {forbidden}"
        );
    }
    assert!(css.contains(".dioxus-audio"));
}

#[test]
fn packaged_styles_expose_the_complete_public_token_contract() {
    let css = include_str!("../assets/dioxus-audio.css");
    let expected_names = STYLE_TOKENS
        .iter()
        .map(|(name, _, _, _)| *name)
        .collect::<BTreeSet<_>>();

    assert_eq!(public_style_tokens(css), expected_names);

    for (name, _, daisy_fallback, standalone_default) in STYLE_TOKENS {
        let precedence = format!("var({name}, var({daisy_fallback}, {standalone_default}))");
        assert!(
            css.contains(&precedence),
            "{name} must fall back to {daisy_fallback}, then {standalone_default}"
        );
    }

    assert_eq!(
        public_style_tokens("--dioxus-audio-primary_extra: red;"),
        BTreeSet::from(["--dioxus-audio-primary_extra"]),
        "inventory parsing must not hide an unexpected token behind a known prefix"
    );
}

#[test]
fn style_guide_publishes_the_complete_stable_contract() {
    let guide = include_str!("../demo/src/pages/styles.rs");
    let token_definitions = source_between(guide, "const STYLE_TOKENS", "#[component]");
    let reference_start = guide
        .find("id: \"style-token-reference\"")
        .expect("style token reference section");
    let responsibility_start = guide
        .find("id: \"application-author-responsibility\"")
        .expect("application-author responsibility note");

    assert!(reference_start < responsibility_start);
    assert!(guide.contains("for token in STYLE_TOKENS"));
    assert_eq!(
        public_style_tokens(token_definitions),
        STYLE_TOKENS.iter().map(|(name, _, _, _)| *name).collect()
    );

    for (name, role, daisy_fallback, standalone_default) in STYLE_TOKENS {
        let definition = source_between(token_definitions, &format!("public: \"{name}\""), "},");
        for value in [role, daisy_fallback, standalone_default] {
            assert!(
                definition.contains(value),
                "reference entry for {name} must contain {value}"
            );
        }
    }

    let reference = &guide[reference_start..responsibility_start];
    for unsupported_hook in [
        "--_dxa-*",
        "component classes",
        "DOM structure",
        "data-*",
        "ARIA attributes",
        "native state selectors",
    ] {
        assert!(
            reference.contains(unsupported_hook),
            "reference must exclude {unsupported_hook} from the stable contract"
        );
    }
    assert!(reference.contains("application-owned presentation"));

    let responsibility = &guide[responsibility_start..];
    for author_concern in [
        "contrast",
        "visible focus",
        "interaction states",
        "success",
        "warning",
        "error",
    ] {
        assert!(
            responsibility.contains(author_concern),
            "author note must cover {author_concern}"
        );
    }

    for cascade_fact in [
        "properties inherit normally",
        "nearest explicit package value",
        "independently falls through",
        "daisyUI is optional",
    ] {
        assert!(
            guide.contains(cascade_fact),
            "guide must explain that {cascade_fact}"
        );
    }
}

#[test]
fn public_docs_lead_with_the_canonical_stylesheet_loader() {
    let readme = include_str!("../README.md");
    let styles_section = source_between(readme, "## Styles", "\n## ");

    assert!(styles_section.contains("use dioxus_audio::components::AudioStyles;"));
    assert!(styles_section.contains("AudioStyles {}"));
    assert!(!styles_section.contains("STYLESHEET"));
    assert!(public_style_tokens(styles_section).is_empty());
    for guidance in [
        "inherit",
        "app-wide",
        "per-instance",
        "daisyUI",
        "https://audio-demo.dioxus.cc/styles",
    ] {
        assert!(
            styles_section.contains(guidance),
            "README Styles section must include {guidance} guidance"
        );
    }

    let components = include_str!("../src/components.rs");
    let audio_styles_docs = rustdoc_before(components, "pub fn AudioStyles");
    let stylesheet_docs = rustdoc_before(components, "pub static STYLESHEET");

    for guidance in ["canonical", "once", "application root"] {
        assert!(
            audio_styles_docs.contains(guidance),
            "AudioStyles rustdoc must describe {guidance}"
        );
    }
    for guidance in ["lower-level", "equivalent"] {
        assert!(
            stylesheet_docs.contains(guidance),
            "STYLESHEET rustdoc must describe its {guidance} role"
        );
    }
    assert!(stylesheet_docs.contains("AudioStyles"));
    assert!(
        public_style_tokens(&format!("{audio_styles_docs}{stylesheet_docs}")).is_empty(),
        "loader rustdoc should point to the guide instead of duplicating the token reference"
    );
}

#[test]
fn every_production_chapter_has_one_well_formed_recipe_region() {
    let chapters: [(&str, &str, &str, &[&str]); 3] = [
        (
            "Studio",
            include_str!("../demo/src/examples/styles/studio.rs"),
            "studio-recipe",
            &[
                "pub fn StudioExample()",
                "AudioInputSelector",
                "WaveformPreview",
                "AudioPlayer",
            ],
        ),
        (
            "scoped",
            include_str!("../demo/src/examples/styles/scoped.rs"),
            "scoped-recipe",
            &["pub fn ScopedExample()"],
        ),
        (
            "daisyUI",
            include_str!("../demo/src/examples/styles/daisy.rs"),
            "daisy-recipe",
            &["pub fn DaisyExample()"],
        ),
    ];

    for (chapter, source, region_name, required_snippets) in chapters {
        let recipe = strict_recipe_region(source, region_name)
            .unwrap_or_else(|error| panic!("{chapter} recipe is invalid: {error}"));
        for expected in required_snippets {
            assert!(
                recipe.contains(expected),
                "{chapter} recipe must contain {expected}"
            );
        }
    }
}

#[test]
fn recipe_region_validation_rejects_malformed_sources() {
    let malformed = [
        ("missing start", "recipe\n// endregion: recipe"),
        ("missing end", "// region: recipe\nrecipe"),
        (
            "duplicate start",
            "// region: recipe\n// region: recipe\nrecipe\n// endregion: recipe",
        ),
        (
            "duplicate end",
            "// region: recipe\nrecipe\n// endregion: recipe\n// endregion: recipe",
        ),
        (
            "reversed",
            "// endregion: recipe\nrecipe\n// region: recipe",
        ),
        ("empty", "// region: recipe\n// endregion: recipe"),
        (
            "whitespace only",
            "// region: recipe\n  \n// endregion: recipe",
        ),
        (
            "malformed start marker",
            "// region: recipe trailing text\nrecipe\n// endregion: recipe",
        ),
        (
            "malformed end marker",
            "// region: recipe\nrecipe\n// endregion: recipe trailing text",
        ),
        (
            "mismatched marker",
            "// region: other\nrecipe\n// endregion: recipe",
        ),
    ];

    for (case, source) in malformed {
        assert!(
            strict_recipe_region(source, "recipe").is_err(),
            "{case} source must be rejected"
        );
    }
}

#[test]
fn studio_recipe_uses_the_exact_authored_theme() {
    let guide = include_str!("../demo/src/pages/styles.rs");
    let studio_css = include_str!("../demo/src/examples/styles/studio.css");
    let expected_tokens = [
        ("--dioxus-audio-base-100", "#fffaf2"),
        ("--dioxus-audio-base-200", "#f2e9dc"),
        ("--dioxus-audio-base-300", "#d7c8b7"),
        ("--dioxus-audio-content", "#241c2f"),
        ("--dioxus-audio-primary", "#7446e8"),
        ("--dioxus-audio-primary-content", "#ffffff"),
        ("--dioxus-audio-warning", "#b86813"),
        ("--dioxus-audio-error", "#c83d61"),
        ("--dioxus-audio-success", "#2f8464"),
        ("--dioxus-audio-radius", "1.15rem"),
    ];

    assert_eq!(
        public_style_tokens(studio_css),
        expected_tokens.iter().map(|(name, _)| *name).collect()
    );
    for (name, value) in expected_tokens {
        assert!(
            studio_css.contains(&format!("{name}: {value};")),
            "Studio must declare {name} as {value}"
        );
    }

    for source_wiring in [
        "include_str!(\"../examples/styles/studio.rs\")",
        "include_str!(\"../examples/styles/studio.css\")",
        "source: recipe_region(STUDIO_MODULE, \"studio-recipe\")",
        "source: STUDIO_STYLESHEET",
    ] {
        assert!(
            guide.contains(source_wiring),
            "Studio recipe must use production source via {source_wiring}"
        );
    }
}

#[test]
fn guide_uses_the_exact_scoped_stylesheet_and_token_free_daisy_recipe() {
    let guide = include_str!("../demo/src/pages/styles.rs");
    let demo_styles = include_str!("../demo/src/style.css");
    let scoped_css = include_str!("../demo/src/examples/styles/scoped.css");
    let daisy = include_str!("../demo/src/examples/styles/daisy.rs");
    let expected_themes = [
        (
            ".citrus {",
            [
                ("--dioxus-audio-base-100", "#fff8e8"),
                ("--dioxus-audio-base-200", "#f7e7c4"),
                ("--dioxus-audio-base-300", "#d9b979"),
                ("--dioxus-audio-content", "#422716"),
                ("--dioxus-audio-primary", "#c4561f"),
                ("--dioxus-audio-primary-content", "#fff8e8"),
                ("--dioxus-audio-radius", "1.25rem"),
            ],
        ),
        (
            ".midnight {",
            [
                ("--dioxus-audio-base-100", "#091524"),
                ("--dioxus-audio-base-200", "#10243a"),
                ("--dioxus-audio-base-300", "#27425f"),
                ("--dioxus-audio-content", "#e6f4ff"),
                ("--dioxus-audio-primary", "#28c7d9"),
                ("--dioxus-audio-primary-content", "#06202a"),
                ("--dioxus-audio-radius", "0.35rem"),
            ],
        ),
    ];

    assert!(demo_styles.contains("@import \"./examples/styles/studio.css\";"));
    assert!(demo_styles.contains("@import \"./examples/styles/scoped.css\";"));
    assert_eq!(public_style_tokens(scoped_css).len(), 7);

    for (selector, tokens) in expected_themes {
        let rule = source_between(scoped_css, selector, "  }");
        assert_eq!(
            public_style_tokens(rule),
            tokens.iter().map(|(name, _)| *name).collect(),
            "{selector} must declare exactly its seven scoped tokens"
        );
        for (name, value) in tokens {
            assert!(
                rule.contains(&format!("{name}: {value};")),
                "{selector} must declare {name} as {value}"
            );
        }
    }

    for source_wiring in [
        "include_str!(\"../examples/styles/scoped.rs\")",
        "include_str!(\"../examples/styles/scoped.css\")",
        "source: recipe_region(SCOPED_MODULE, \"scoped-recipe\")",
        "source: SCOPED_STYLESHEET",
        "include_str!(\"../examples/styles/daisy.rs\")",
        "source: recipe_region(DAISY_MODULE, \"daisy-recipe\")",
    ] {
        assert!(
            guide.contains(source_wiring),
            "guide recipe must use production source via {source_wiring}"
        );
    }

    assert!(public_style_tokens(daisy).is_empty());
    assert!(
        !guide.contains("DAISY_STYLESHEET"),
        "the daisyUI chapter must not introduce a package-token stylesheet"
    );
}

#[test]
fn production_guide_contains_no_prototype_routes_or_selectors() {
    let production_sources = [
        include_str!("../demo/src/app.rs"),
        include_str!("../demo/src/pages.rs"),
        include_str!("../demo/src/pages/styles.rs"),
        include_str!("../demo/src/examples.rs"),
        include_str!("../demo/src/examples/styles/mod.rs"),
        include_str!("../demo/src/examples/styles/studio.rs"),
        include_str!("../demo/src/examples/styles/studio.css"),
        include_str!("../demo/src/examples/styles/scoped.rs"),
        include_str!("../demo/src/examples/styles/scoped.css"),
        include_str!("../demo/src/examples/styles/daisy.rs"),
        include_str!("../demo/src/style.css"),
    ];

    for forbidden in [
        "/styles-prototype",
        "styles_prototype",
        "StylesPrototype",
        "WorkbenchVariant",
        "ReferenceVariant",
        "style-prototype-",
        "?variant=",
        "Style guide prototype variants",
    ] {
        assert!(
            production_sources
                .iter()
                .all(|source| !source.contains(forbidden)),
            "production guide must not contain prototype artifact {forbidden}"
        );
    }

    assert!(production_sources[0].contains("#[route(\"/styles\")]"));
    assert!(production_sources[0].contains("label: \"Style customization\""));
}

fn strict_recipe_region<'a>(source: &'a str, name: &str) -> Result<&'a str, String> {
    let start_marker = format!("// region: {name}");
    let end_marker = format!("// endregion: {name}");
    let starts = exact_line_ranges(source, &start_marker);
    let ends = exact_line_ranges(source, &end_marker);

    if starts.len() != 1 {
        return Err(format!(
            "recipe region {name:?} must have exactly one start marker, found {}",
            starts.len()
        ));
    }
    if ends.len() != 1 {
        return Err(format!(
            "recipe region {name:?} must have exactly one end marker, found {}",
            ends.len()
        ));
    }

    let body_start = starts[0].1;
    let body_end = ends[0].0;
    if body_start > body_end {
        return Err(format!(
            "recipe region {name:?} end marker must follow its start marker"
        ));
    }

    let region = source[body_start..body_end].trim_end_matches(['\r', '\n']);
    if region.trim().is_empty() {
        return Err(format!("recipe region {name:?} must not be empty"));
    }

    Ok(region)
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

fn public_style_tokens(source: &str) -> BTreeSet<&str> {
    const PREFIX: &str = "--dioxus-audio-";

    source
        .match_indices(PREFIX)
        .map(|(start, _)| {
            let candidate = &source[start..];
            let end = candidate
                .find(|character: char| {
                    character.is_ascii_whitespace()
                        || matches!(
                            character,
                            ':' | ',' | ')' | ';' | '{' | '}' | '"' | '\'' | '`'
                        )
                })
                .unwrap_or(candidate.len());
            &candidate[..end]
        })
        .collect()
}

fn source_between<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    let start = source
        .find(start)
        .unwrap_or_else(|| panic!("missing {start:?}"));
    let source = &source[start..];
    let end = source
        .find(end)
        .unwrap_or_else(|| panic!("missing {end:?} after {start}"));
    &source[..end]
}

fn rustdoc_before(source: &str, declaration: &str) -> String {
    let declaration = source
        .find(declaration)
        .unwrap_or_else(|| panic!("missing declaration {declaration:?}"));
    let mut lines = source[..declaration].lines().rev().peekable();

    while lines
        .peek()
        .is_some_and(|line| line.trim_start().starts_with("#["))
    {
        lines.next();
    }

    let mut docs = lines
        .take_while(|line| line.trim_start().starts_with("///"))
        .map(|line| line.trim_start().trim_start_matches("///").trim())
        .collect::<Vec<_>>();
    docs.reverse();
    docs.join(" ")
}
