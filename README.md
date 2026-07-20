# dioxus-audio

[![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/sagikazarmark/dioxus-audio/dagger.yaml?style=flat-square)](https://github.com/sagikazarmark/dioxus-audio/actions/workflows/dagger.yaml)
[![OpenSSF Scorecard](https://api.securityscorecards.dev/projects/github.com/sagikazarmark/dioxus-audio/badge?style=flat-square)](https://securityscorecards.dev/viewer/?uri=github.com/sagikazarmark/dioxus-audio)
[![crates.io](https://img.shields.io/crates/v/dioxus-audio?style=flat-square)](https://crates.io/crates/dioxus-audio)
[![docs.rs](https://img.shields.io/docsrs/dioxus-audio?style=flat-square)](https://docs.rs/dioxus-audio)

**Audio recording, playback, analysis, and UI components for Dioxus**

## Features

- **Recording:** microphone permissions, capture lifecycle, elapsed time, peaks, and live analysis.
- **Playback:** Audio Data and ordered URL-addressable Playback Source alternatives,
  eager or on-play loading, playback lifecycle, stop/reset, whole-source repeat,
  seeking, skipping, and playback rate.
- **Audio input devices:** enumeration, selection, permission requests, and device-change handling.
- **Analysis:** bounded reactive snapshots, interpretable waveform and spectrum
  data, RMS levels, peak reduction, and PCM range trimming.
- **Decoding:** complete Audio Data to immutable channel-preserving planar samples
  with an explicit Rust-copy memory ceiling.
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

For custom layouts, pass the same Recorder to `RecorderStartButton`,
`RecorderCancelButton`, `RecorderPauseResumeButton`, `RecorderStopButton`, and
`RecorderClearButton`. Each native button exposes command validity through its
disabled state and accepts application-specific labels. Mount
`RecorderStatusAnnouncer` when the application needs polite, coarse lifecycle
announcements; it does not announce elapsed time or Analysis updates.

Microphone capture requires a secure browser context. HTTPS, `localhost`, and
`127.0.0.1` are normally accepted by browsers.

Input selection is snapshotted by `recorder.start()`. Let users choose a device
before starting capture; changing `devices.selected()` does not switch an
active recording.

`RecorderOptions::constraints` accepts portable `Ideal` or `Exact` requests for
channel count, sample rate, echo cancellation, noise suppression, and latency.
The Recorder snapshots those constraints when it accepts `start()`, so changing
options during a Recording only affects a future Recording.

Use `recorder.requested_constraints()` for that snapshot,
`recorder.constraint_capabilities()` for the constraint fields the browser
recognizes, and `recorder.settings()` for the effective settings reported by
the acquired Recording Source. Effective fields are optional because browsers
do not report all settings consistently. An exact-constraint failure has
`AudioErrorKind::Overconstrained`; `overconstrained_constraint()` identifies the
rejected browser constraint when supplied by the browser.

`recorder.media_type()` exposes the selected encoder format once the Recording
starts. `is_recorder_mime_type_supported()` can be used before starting to probe
a candidate format, but a positive probe only means the browser recognizes the
type. It does not guarantee source acquisition, Recorder construction, or a
successful Recording.

## Live Analysis

Use `use_live_analysis` with an optional `AudioAnalyser` from any supported
source. Recorder supplies one through `recorder.analyser()` while a Recording is
active; future sources can provide the same source-neutral handle.

```rust
use dioxus_audio::analysis::{LiveAnalysisOptions, use_live_analysis};

let analysis = use_live_analysis(
    recorder.analyser(),
    LiveAnalysisOptions::default(),
);

if let Some(snapshot) = analysis() {
    let metadata = snapshot.metadata();
    let first_bin_hz = metadata.frequency_for_bin(1);
    let rms_level = snapshot.level();
    let waveform = snapshot.time_domain();
    let spectrum = snapshot.frequency_domain();
}
```

The default cadence is 50ms. `with_cadence` clamps requested values to
`16ms..=1s`, preventing unbounded polling and rendering rates. Each hook call
has an independent schedule. Its snapshot becomes `None` when the Analyser is
removed or replaced, stale work cannot publish into the replacement, polling
stops on unmount, and analyser reads are suspended while the document is
hidden.

Time-domain values are byte-quantized amplitudes normalized to `-1.0..=1.0`.
Frequency-domain values are byte-quantized magnitudes normalized to
`0.0..=1.0`; `AnalysisMetadata` supplies the effective graph sample rate, FFT
size, bin count and frequency mapping, decibel range, and smoothing constant
needed to interpret them. `snapshot.level()` is normalized RMS amplitude over
that snapshot's FFT-sized time-domain window. It is not peak amplitude,
perceived loudness, sound pressure level, or Playback audibility.

`LiveWaveform`, `SpectrumVisualizer`, and `LevelMeter` use the same bounded
scheduling behavior while collecting only the values each presentation needs.
Their changing Analysis values are not live-region announcements.

## Decoded Audio

Use `decode_audio_data` to consume complete `AudioData` and copy the browser's
decoded channels into one immutable flat-planar Rust allocation:

```rust
use dioxus_audio::AudioData;
use dioxus_audio::decoding::{DecodeOptions, decode_audio_data};

async fn inspect(audio: AudioData) -> Result<(), Box<dyn std::error::Error>> {
    let decoded = decode_audio_data(audio, DecodeOptions::default()).await?;

    println!(
        "{} channels, {} frames at {} Hz ({:?})",
        decoded.channel_count(),
        decoded.frame_count(),
        decoded.sample_rate(),
        decoded.duration(),
    );
    for channel in decoded.channels() {
        // Each item is one borrowed contiguous channel slice.
        analyze(channel);
    }

    Ok(())
}

fn analyze(_samples: &[f32]) {}
```

The reported sample rate is the browser decode context's effective rate, not a
claim about the encoded source's original rate. The media type is retained as
part of `AudioData` but browser `decodeAudioData` does not consume it; media-type
support probes therefore cannot prove that a particular payload will decode.
Unsupported codecs, malformed or truncated input, and decoder refusal all map
to the portable `DecodeErrorKind::DecodeRejected` outcome.

`DecodeOptions` defaults to a 128 MiB ceiling for the Rust-owned planar `f32`
copy and allows an explicit `with_max_decoded_bytes` override. The size uses
checked channel, frame, and sample-width arithmetic, and a resource-limit error
reports both required and configured bytes. This gate runs only after the
browser has decoded the complete file, so it cannot prevent the browser's first
PCM allocation. Successful materialization may briefly retain roughly two PCM
representations, excluding encoded data and opaque decoder internals.

Each operation owns an internal `AudioContext` and requests context cleanup when
it settles or its future is dropped. Dropping the future suppresses result
publication, but does not promise to abort decoding work already started by the
browser. Decoded Audio is not a streaming decoder, mutable sample buffer,
resampler, transformed output, or Playback source.

## Playback

```rust
use dioxus::prelude::*;
use dioxus_audio::components::AudioPlayer;
use dioxus_audio::playback::PlaybackSource;

#[component]
fn Player() -> Element {
    let mut source = use_signal(|| None::<PlaybackSource>);

    rsx! {
        AudioPlayer {
            source,
            duration_secs: 42.0,
            on_request_audio: move |_| {
                // Load Audio Data from your store, then convert it into a Playback Source.
                // source.set(Some(data.into()));
            },
        }
    }
}
```

The player creates and revokes browser object URLs for Audio Data as its source
changes. Playback rate changes do not reload the source or reset its position.

For URL-addressable media, construct one alternative or a non-empty ordered set.
Each alternative is validated and can carry an optional media-type hint. Relative
URLs are accepted; validation does not claim that a resource exists or that the
browser can decode it.

```rust
use dioxus_audio::playback::{
    PlaybackLoadingPolicy, PlaybackSource, PlaybackSourceAlternative,
};

let alternative = PlaybackSourceAlternative::new("/media/episode.mp3")?
    .with_media_type("audio/mpeg")?;
let source = PlaybackSource::url(alternative)
    .with_loading_policy(PlaybackLoadingPolicy::OnPlay);
# Ok::<(), dioxus_audio::AudioError>(())
```

Use `PlaybackSource::url_alternatives` when an application can offer multiple
alternatives:

```rust
use dioxus_audio::playback::{PlaybackSource, PlaybackSourceAlternative};

let source = PlaybackSource::url_alternatives([
    PlaybackSourceAlternative::new("/media/episode.webm")?
        .with_media_type("audio/webm; codecs=opus")?,
    PlaybackSourceAlternative::new("/media/episode.mp3")?
        .with_media_type("audio/mpeg")?,
    PlaybackSourceAlternative::new("/media/episode")?,
])?;
# Ok::<(), dioxus_audio::AudioError>(())
```

Playback skips only media-type hints the browser reports as definitely
unsupported. Untyped, `maybe`, and `probably` alternatives receive real load
attempts in order. Metadata remains tentative; `canplay` selects and exposes the
playable alternative. Initial failures continue to the next alternative, while a
failure after selection is terminal and never switches media implicitly.
If no alternative becomes playable, `PlaybackSnapshot::alternative_failures`
reports each attempted or skipped alternative in order with an `unsupported`,
`network`, `decode`, or `unknown` failure kind.

`Eager` begins browser acquisition when the source becomes current. `OnPlay`
keeps the source `Dormant` without an attached media resource until `play()` is
requested. Pausing while an on-play source is still loading clears play intent
without cancelling acquisition. URL ownership remains with the application:
replacement, unload, and owner cleanup detach the media resource but never
revoke an application-supplied URL, including an application-owned `blob:` URL.

For custom controls, `use_audio_player` exposes a `PlaybackSnapshot` through
`controller.snapshot()`. Source lifecycle, transport, readiness, and recoverable
play failure are independent facets. URL selection and coarse terminal source
failure are separately available through `selected_alternative` and
`source_failure`. A play request remains `PlayPending` until the browser confirms
`Playing`, and an interaction-required rejection leaves the current source
usable for retry.

Calling `controller.stop()` atomically pauses Playback, resets position to zero,
invalidates an outstanding play request, and returns a loaded source to
`Ready`/`Idle`. Whole-source repeat is available through `repeat()`,
`set_repeat()`, and `toggle_repeat()`; it remains enabled or disabled when the
source is replaced or unloaded and applies to the next loaded source. It is
separate from Bounded Playback over a Waveform Selection.

Mute is observable through `muted()` and can be changed with `set_muted()` or
`toggle_muted()` without pausing Playback, seeking, or changing the retained
audibility level. `set_audibility_level()` accepts a finite normalized value in
the inclusive range `0.0..=1.0`; invalid values are rejected without changing
public state. Mute and level preferences survive source replacement and unload
and apply to the next source.

Always inspect `audibility_capability()` before describing the level as
effective output gain. Current direct Playback reports
`PlaybackAudibilityCapability::BestEffortMediaElement`: the browser media
element receives the value, but some platforms, notably iOS, may not apply it
to perceived loudness. `EffectiveGraphGain` is reserved for graph-backed
Playback; direct control does not claim that guarantee. Mute and all transport
commands remain independent from level capability.

The same Controller can drive independently arranged `PlaybackSeekSlider`,
`PlaybackSkipButton`, `PlaybackStopButton`, `PlaybackPlayPauseButton`,
`PlaybackRateButton`, `PlaybackMuteButton`, `PlaybackAudibilitySlider`, and
`PlaybackRepeatButton` components. Labels, signed skip amounts, and the rate
cycle are configurable. Mute and repeat use stable labels and native pressed
state. The seek and audibility sliders expose meaningful value text, which can
be replaced with localized `value_text`. `PlaybackStatusAnnouncer` is an optional
polite live region for coarse lifecycle changes and never announces position or
audibility changes.

## Waveform Data

`WaveformData` is an immutable, cheap-to-clone snapshot for duration-aware
Waveforms. A snapshot has one amplitude mode, one channel count, and one or more
resolutions ordered from finest to coarsest. Construction consumes flat
channel-major buffers and rejects invalid spans, coverage, channel alignment,
and amplitudes rather than repairing them.

```rust
use std::time::Duration;

use dioxus::prelude::*;
use dioxus_audio::components::Waveform;
use dioxus_audio::waveform::WaveformData;

#[component]
fn RecordedWaveform() -> Element {
    let data = WaveformData::from_peaks(
        Duration::from_secs(4),
        vec![12, 48, 180, 255, 160, 52, 24, 8],
    )
    .expect("positive duration and nonempty Peaks");

    rsx! {
        Waveform {
            data,
            bucket_budget: 256,
            label: "Recorded waveform",
        }
    }
}
```

`WaveformData::select` accepts a half-open source-time range and a per-channel
bucket budget. It returns the finest fitting stored resolution, or the coarsest
resolution when none fit, as borrowed channel slices without copying buckets.
Clone and equality use shared snapshot identity, so independently reconstructed
data intentionally counts as changed.

Use `from_magnitudes` for normalized values in `0.0..=1.0` and
`from_signed_envelopes` for ordered minimum/maximum pairs in `-1.0..=1.0`.
`from_peaks` creates one evenly spaced mono Magnitude resolution; this conversion
necessarily loses Peaks cadence, channel structure, and sign information.

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
