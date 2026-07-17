//! A docs-by-example gallery for `dioxus-audio`.
//!
//! Every feature page mounts a working example and shows the exact source that
//! produced it. The router and shell live in `app`, reusable presentation in
//! `components`, and the runnable examples in `examples`.

// Arborium 2.18.1 declares this C ABI symbol but drops its sysroot at the final link.
// Related upstream tracker: https://github.com/bearcove/arborium/issues/125
#[cfg(target_family = "wasm")]
#[unsafe(export_name = "stderr")]
static ARBORIUM_STDERR: usize = 0;

mod app;
mod components;
mod examples;
mod pages;

fn main() {
    dioxus::launch(app::App);
}
