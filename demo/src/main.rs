//! A docs-by-example gallery for `dioxus-audio`.
//!
//! Every feature page mounts a working example and shows the exact source that
//! produced it. The router and shell live in `app`, reusable presentation in
//! `components`, and the runnable examples in `examples`.

mod app;
mod components;
mod examples;
mod pages;

fn main() {
    dioxus::launch(app::App);
}
