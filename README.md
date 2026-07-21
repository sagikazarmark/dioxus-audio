# dioxus-audio

[![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/sagikazarmark/dioxus-audio/dagger.yaml?style=flat-square)](https://github.com/sagikazarmark/dioxus-audio/actions/workflows/dagger.yaml)
[![OpenSSF Scorecard](https://api.securityscorecards.dev/projects/github.com/sagikazarmark/dioxus-audio/badge?style=flat-square)](https://securityscorecards.dev/viewer/?uri=github.com/sagikazarmark/dioxus-audio)
[![crates.io](https://img.shields.io/crates/v/dioxus-audio?style=flat-square)](https://crates.io/crates/dioxus-audio)
[![docs.rs](https://img.shields.io/docsrs/dioxus-audio?style=flat-square)](https://docs.rs/dioxus-audio)

**Audio recording, playback, analysis, and UI components for Dioxus**

## Features

- **Recording:** microphone permissions, capture lifecycle, ordered Recording
  Chunks, elapsed time, peaks, and live analysis.
- **Playback:** Audio Data and ordered URL-addressable Playback Source alternatives,
  eager or on-play loading, playback lifecycle, stop/reset, whole-source repeat,
  network and readiness observations, buffered and seekable ranges, seeking,
  skipping, playback rate, and opt-in pre-gain Analysis with effective graph gain.
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

Browser applications that already own a media stream can wrap it as an opaque
`RecordingSource` and start without another device acquisition:

```rust
use dioxus_audio::recorder::{RecordingSource, RecordingSourceShutdown};

let source = RecordingSource::from_media_stream(&application_stream);
recorder
    .start_with_source(source)
    .expect("Recorder accepted the supplied source");

let disposable_source = RecordingSource::from_media_stream(&temporary_stream)
    .with_shutdown(RecordingSourceShutdown::StopAudioTracks);
```

The application retains the raw stream if it needs to use or stop it later;
Recorder does not expose that browser resource through its Controller. An
accepted supplied source must contain exactly one live audio track. Video tracks
are ignored, and a live audio track remains valid when browser-muted or
application-disabled. Recorder creates an audio-only recording view. The default
`PreserveTracks` agreement never calls `stop()` on the supplied track.
`StopAudioTracks` explicitly authorizes Recorder to stop the accepted audio
track exactly once during terminal cleanup, including completion, discard,
failure, source end, and unmount. Do not grant that authority unless every
consumer of the shared track may be stopped.

`recorder.source_availability()` reports `Live` or `Interrupted` independently
from whether Recording is paused while a source is active. Browser mute and
unmute events update availability without pausing elapsed time. Recorder does not poll
or change the track's application-controlled `enabled` state. If the track ends
while Recording or paused, Recorder finalizes valid partial Recorded Audio;
`RecordingOutcome::Completed` identifies `SourceEnded` separately from
`Requested` and `UnexpectedEnd` completion.

Capture constraints, microphone permission requests, effective acquired-source
settings, and Audio Input Device identity apply only to `recorder.start()`.
Supplied-source startup enters the same source-neutral `Preparing` state, while
`requested_constraints()`, `settings()`, and completed input identity remain
unknown. Encoder selection, Recording Chunks, elapsed time, pause and resume,
Analysis, and completed Recorded Audio behave the same for either source.

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

Recording Chunk delivery is opt-in and supplements the final `RecordedAudio`:

```rust
use std::time::Duration;
use dioxus_audio::RecordingChunk;
use dioxus_audio::recorder::RecordingChunkDelivery;

let mut pending_chunks = use_signal(Vec::<RecordingChunk>::new);
let mut options = RecorderOptions::default();
options.chunk_delivery = Some(RecordingChunkDelivery::new(
    Duration::from_millis(250),
    move |chunk: RecordingChunk| {
        // Move the owned bytes into application-managed upload or persistence work.
        pending_chunks.write().push(chunk);
    },
));

let recorder = use_audio_recorder(options, devices.selected().into());
```

The accepted `start()` snapshots the delivery configuration and selected encoder
preferences. Cadence is approximate: browsers choose actual fragment boundaries
and may produce no data at a boundary. Every delivered chunk owns non-empty
encoded bytes and carries the effective media type, a Recorder-local
`RecordingId`, and a zero-based contiguous sequence. Chunks are delivered
serially in browser event order and are not guaranteed to be independently
playable.

While an opted-in Recording is active or paused,
`recorder.request_chunk_boundary()` asks the browser for another best-effort
boundary. The request does not promise exact timing, a non-empty chunk, or a
fragment that can be played without earlier chunks.

Callback return hands the chunk to the application; the library does not add an
upload, persistence, retry, acknowledgement, or backpressure queue. Recorder
still retains every browser fragment needed for final `RecordedAudio`, so chunk
delivery does not reduce completion memory. On success, all final chunks are
delivered before `recorder.completed()` becomes populated. Discard and unmount
suppress output that has not already been handed off.

If incremental blob conversion fails, `recorder.chunk_delivery_failure()`
identifies the Recording and failed sequence. That failure ends chunk delivery
for the Recording, but capture continues and final `RecordedAudio` assembly is
still attempted from the independently retained browser fragments. A Recorder,
encoder, or final-assembly failure instead ends the Recording, suppresses future
chunks and completed output, and is reported through `recorder.outcome()`.

`RecordedAudio::recording_id` correlates completion with its chunks.
`recorder.outcome()` exposes the same ID for completed, discarded, and failed
Recordings; configuration rejected before `start()` is accepted has no Recording
outcome.

## Live Analysis

Use `use_live_analysis` with an optional `AudioAnalyser` from any supported
source. Recorder supplies one through `recorder.analyser()` while a Recording is
active, and graph-backed Playback supplies the same source-neutral handle through
`player.analyser()`.

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

An `AudioAnalyser` is a weak owner handle: retaining it does not keep its audio
graph alive. `is_available()`, `try_read()`, and `try_level()` report
unavailability when its source is detached, its owner degrades, or its owner is
cleaned up. Convenience `read()` and `level()` retain their empty/zero fallback;
use the `try_` methods whenever valid silence must be distinguished from an
unavailable source. Reactive Analysis clears its snapshot while a stable handle
is unavailable and resumes when graph-backed Playback attaches another eligible
source to that same Analyser.

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
`network`, `decode`, `graph-ineligible`, or `unknown` failure kind.

`Eager` begins browser acquisition when the source becomes current. `OnPlay`
keeps the source `Dormant` without an attached media resource until `play()` is
requested. Pausing while an on-play source is still loading clears play intent
without cancelling acquisition. URL ownership remains with the application:
replacement, unload, and owner cleanup detach the media resource but never
revoke an application-supplied URL, including an application-owned `blob:` URL.

For custom controls, `use_audio_player` exposes a `PlaybackSnapshot` through
`controller.snapshot()`. Source lifecycle, transport, readiness, and recoverable
play failure are independent facets. `network` separately reports inactive,
unknown, loading, idle, or stalled activity, so playing transport may coexist
with waiting readiness and stalled network activity. URL selection and coarse
terminal source failure are separately available through `selected_alternative`
and `source_failure`. A play request remains `PlayPending` until the browser
confirms `Playing`, and an interaction-required rejection leaves the current
source usable for retry. That recoverable failure clears on retry, confirmed
play, source replacement, unload, or terminal source failure.

`PlaybackSnapshot::buffered` and `seekable` are immutable, sorted source-time
range snapshots for the current source attempt. Overlapping or touching ranges
are merged, but later snapshots may be empty or smaller. Treat both collections
as UI guidance: buffered time is not byte-transfer progress, and a seekable
observation does not guarantee that the same seek will remain available later.
Replacement, fallback, unload, and terminal failure clear the observations, and
late events from an older source attempt cannot republish them.

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

For pre-gain Analysis and effective gain, create an immutable graph-backed owner:

```rust
use std::time::Duration;

use dioxus::prelude::*;
use dioxus_audio::playback::{
    PlaybackOptions, PlaybackSource, use_audio_player_with_options,
};

let source = use_signal(|| None::<PlaybackSource>);
let player = use_audio_player_with_options(
    source.into(),
    Duration::ZERO,
    PlaybackOptions::graph_backed(),
);
let analyser = player.analyser();
```

The opt-in belongs to the Playback owner and cannot be switched reactively.
The graph is created lazily when the first eligible Playback Source is attached,
then its context, pre-gain Analyser, and gain node persist across replacement
among eligible Audio Data and URL-backed Playback Sources, as well as unload.
The graph state is orthogonal to source and
transport state: it reports awaiting source, preparing, suspended, running,
interaction required, or terminal unavailability. Direct Playback remains the
default.

URL-addressable alternatives are direct-only unless they explicitly request
anonymous CORS:

```rust
use dioxus_audio::playback::{
    PlaybackSource, PlaybackSourceAlternative, PlaybackSourceCrossOrigin,
};

let alternative = PlaybackSourceAlternative::new("https://media.example/episode.mp3")?
    .with_media_type("audio/mpeg")?
    .with_cross_origin(PlaybackSourceCrossOrigin::Anonymous);
assert!(alternative.is_graph_eligible());
let source = PlaybackSource::url(alternative);
# Ok::<(), dioxus_audio::AudioError>(())
```

Playback applies the declared `crossOrigin` policy before graph attachment and
before assigning `src`. The server must authorize the anonymous cross-origin
request. Rejection is an initial source-attempt failure and may fall back to the
next eligible alternative. Alternatives without a cross-origin policy and those
using `UseCredentials` remain available to ordinary direct Playback, but a
graph-backed owner skips them as `GraphIneligible` without speculative graph
attachment.

A graph-backed play request invokes context resume and media play in the same
activation turn and reports `Playing` only after both succeed. Rejection or a
later context suspension pauses the media and reports an interaction-required
failure that can be retried. Analysis observes the Playback Source before mute
and level gain, so visualizations remain meaningful at zero output gain. A
terminal setup failure invalidates Analysis, permanently changes that owner's
level capability to `BestEffortMediaElement`, and continues with direct
transport. A later failure of an already selected URL-addressable alternative is
terminal and does not silently choose another alternative.

The same Controller can drive independently arranged `PlaybackSeekSlider`,
`PlaybackSkipButton`, `PlaybackStopButton`, `PlaybackPlayPauseButton`,
`PlaybackRateButton`, `PlaybackMuteButton`, `PlaybackAudibilitySlider`, and
`PlaybackRepeatButton` components. Labels, signed skip amounts, and the rate
cycle are configurable. Mute and repeat use stable labels and native pressed
state. The seek and audibility sliders expose meaningful value text, which can
be replaced with localized `value_text`. `PlaybackStatusAnnouncer` is an optional
polite live region for coarse lifecycle, waiting, stalled, and recovery changes.
It never announces position, range snapshots, or audibility changes.

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

`InteractiveWaveform` composes Waveform Data, an `AudioPlayerController`, and
one controlled `WaveformSelection`. Playback position, selection start, and
selection end remain independently named native sliders. Arrow keys use the
configurable fine source-time step, Page Up and Page Down use the coarse step,
and Home and End honor each control's valid bounds. Selection handles may meet
but do not cross or exchange identity.

Pointer movement is shown as an internal draft. A handle drag commits one
selection update on release, while a track click seeks Playback immediately.
Waveform duration determines rendering and selection bounds; a seek is capped
by the Controller's authoritative positive Playback duration. Continuously
changing position and drafts are not live-region announcements, so native
slider feedback remains the continuous assistive signal.

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
