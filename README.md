# dioxus-audio

[![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/sagikazarmark/dioxus-audio/dagger.yaml?style=flat-square)](https://github.com/sagikazarmark/dioxus-audio/actions/workflows/dagger.yaml)
[![OpenSSF Scorecard](https://api.securityscorecards.dev/projects/github.com/sagikazarmark/dioxus-audio/badge?style=flat-square)](https://securityscorecards.dev/viewer/?uri=github.com/sagikazarmark/dioxus-audio)
[![crates.io](https://img.shields.io/crates/v/dioxus-audio?style=flat-square)](https://crates.io/crates/dioxus-audio)
[![docs.rs](https://img.shields.io/docsrs/dioxus-audio?style=flat-square)](https://docs.rs/dioxus-audio)

**Audio recording, playback, analysis, and UI components for Dioxus**

## Features

- **Recording:** microphone permissions, capture lifecycle, elapsed time, peaks, and live analysis.
- **Playback:** loading, playback lifecycle, seeking, skipping, and playback rate.
- **Audio input devices:** enumeration, selection, permission requests, and device-change handling.
- **Analysis:** peak reduction, waveform and spectrum data, levels, and PCM range trimming.
- **Dioxus components:** player and recorder controls, scrubber, input
  selector, microphone status, waveform views, spectrum, and level meter.
- **Scoped styles:** authored CSS with a `dioxus-audio` namespace and stable
  package variables, without a Tailwind or daisyUI build dependency.

## Quick Start

```toml
[dependencies]
dioxus-audio = { version = "0.x" }
```

## Styles

The crate ships authored, namespace-scoped CSS.
Load it once near the application root:

```rust
use dioxus::prelude::*;
use dioxus_audio::components::AudioStyles;

fn App() -> Element {
    rsx! {
        AudioStyles {}
        // application UI
    }
}
```

Stable package custom properties inherit, so applications can set them on an
ancestor for app-wide styling or on a local wrapper for per-instance styling.
When values are omitted, components can follow an installed daisyUI theme and
otherwise use standalone defaults; daisyUI is optional.

See the [Style Customization Guide](https://audio-demo.dioxus.cc/styles) for
complete recipes and the public token reference.

`WaveformPreview` normalizes each non-empty waveform against its own loudest
peak (with a quiet-signal floor), so compact previews remain legible. It is a
shape preview, not an absolute loudness comparison between recordings.

## Recording

```rust
use dioxus::prelude::*;
use dioxus_audio::components::{
    AudioInputSelector, LiveWaveform, MicrophoneStatusIndicator, RecorderControls,
};
use dioxus_audio::devices::use_audio_input_devices;
use dioxus_audio::recorder::{use_audio_recorder, RecorderOptions};

#[component]
fn Recorder() -> Element {
    let devices = use_audio_input_devices();
    let recorder = use_audio_recorder(RecorderOptions::default(), devices.selected().into());

    rsx! {
        AudioInputSelector { devices }
        MicrophoneStatusIndicator { status: recorder.microphone() }
        LiveWaveform { analyser: recorder.analyser() }
        RecorderControls { recorder }
    }
}
```

Call `recorder.completed()` to react to a finished `RecordedAudio`, then call
`recorder.clear_completed()` only after consuming it successfully. Keeping the
value until persistence succeeds lets an application retry without losing the
captured bytes. Consumers that maintain their own retry queue can use
`recorder.take_completed()` to move the recording out without cloning its
audio buffer.

Microphone capture requires a secure browser context. HTTPS, `localhost`, and
`127.0.0.1` are normally accepted by browsers.

Input selection is snapshotted by `recorder.start()`. Let users choose a device
before starting capture; changing `devices.selected()` does not switch an
active recording.

## Playback

```rust
use dioxus::prelude::*;
use dioxus_audio::components::AudioPlayer;
use dioxus_audio::AudioData;

#[component]
fn Player() -> Element {
    let mut audio = use_signal(|| None::<AudioData>);

    rsx! {
        AudioPlayer {
            source: audio,
            duration_secs: 42.0,
            on_request_audio: move |_| {
                // Load bytes from your store, then call audio.set(Some(data)).
            },
        }
    }
}
```

The player creates and revokes browser object URLs as its source changes.
Playback rate changes do not reload the source or reset its position.

For custom controls, `use_audio_player` exposes a `PlaybackSnapshot` through
`controller.snapshot()`. Source lifecycle, transport, readiness, and recoverable
play failure are independent facets: a play request remains `PlayPending` until
the browser confirms `Playing`, and an interaction-required rejection leaves the
current source usable for retry.

## Platform Support

Browser recording, playback, and device hooks require the
`wasm32-unknown-unknown` browser target with the relevant Web APIs. On other
targets, pure analysis and visual components remain available while controllers
report `AudioErrorKind::UnsupportedPlatform`.
Permission prompts, available MIME types, device labels, and background-tab
capture behavior remain browser and operating-system policies.

For hydration-safe fullstack rendering, unsupported server targets use the
same neutral first-render states as the browser and transition to unsupported
inside the first client effect. Commands still return unsupported errors.

## Domain Language

The public audio terms used by this project are defined in
[`CONTEXT.md`](CONTEXT.md). The glossary describes the domain contract without
documenting private module or backend layout.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
