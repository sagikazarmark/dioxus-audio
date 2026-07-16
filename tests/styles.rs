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
    assert!(css.contains("var(--color-primary"));
}
