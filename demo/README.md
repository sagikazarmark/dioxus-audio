# dioxus-audio demo

A docs-by-example gallery for `dioxus-audio`.

## Run locally

Build the Tailwind and daisyUI stylesheet once, then run the CSS watcher and
Dioxus dev server in separate terminals. From the repository root:

```sh
npm install
npm run build
dx serve
```

Open the URL printed by `dx`. Microphone access requires a secure browser
context; `localhost` and `127.0.0.1` are accepted by browsers for local
development.

## Project layout

- `src/examples/` contains the small runnable components shown in the gallery.
- `src/pages/` adds explanation and quotes each example's exact source.
- `src/components/` contains the responsive shell and documentation UI.
- `snippets/` contains non-compiled quick-start source shown on the overview.

The gallery covers the end-to-end recorder and player flow, input discovery,
live visualizers, waveform components, and platform-independent analysis
helpers.
