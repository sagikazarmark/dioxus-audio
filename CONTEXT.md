# dioxus-audio

This glossary defines the public audio language used by `dioxus-audio`. It
describes observable concepts without assigning them to private modules,
browser APIs, or backend implementations.

## Language

**Recorder**:
The audio-capture capability that exposes capture state, commands, elapsed
time, live **Analysis**, and completed **Recorded Audio**.
_Avoid_: Recorder component, storage service

**Recording**:
One active audio-capture lifecycle, beginning with a start request and ending
when the captured audio is completed or discarded.
_Avoid_: Recorded audio, audio data

**Recording ID**:
An opaque identity assigned to one **Recording** by its **Recorder**, used to
correlate its **Recording Chunks** with completed **Recorded Audio**, discard,
or failure. It is unique only within the assigning Recorder and is not an
application or storage identity.
_Avoid_: Recording session, upload ID

**Recording Chunk**:
An ordered encoded fragment emitted during a **Recording** before completion.
A Recording Chunk is not guaranteed to be independently playable and is not
completed **Recorded Audio**.
_Avoid_: Audio data, recorded audio, upload part

**Recording Source**:
A live source of audio captured by a **Recording**. A Recording Source may be
acquired from an **Audio Input Device** or supplied by an application under an
explicit shutdown agreement. Its availability is distinct from whether the
Recording is paused and from whether an application intentionally silences the
source.
_Avoid_: Audio input device, recorded audio

**Recorded Audio**:
The result of a completed **Recording**, containing **Audio Data**, duration,
**Peaks**, its **Recording ID**, and the selected input identity when it is
known.
_Avoid_: Recording, persisted recording

**Audio Data**:
Encoded audio bytes paired with their media type, independent of which
application stores or plays them.
_Avoid_: Recorded audio, decoded samples

**Decoded Audio**:
The complete sample representation obtained by decoding **Audio Data**, carrying
its channel structure and sample rate. Decoded Audio can be inspected through
**Analysis** without changing its source.
_Avoid_: Audio data, recorded audio, edited audio

**Playback Source**:
An input to **Playback** that identifies playable audio as **Audio Data** or as
one or more URL-addressable alternatives. A Playback Source does not transfer
ownership of an external player or live stream.
_Avoid_: Audio data, media element, media stream

**Audio Input Device**:
A discoverable device from which a **Recording Source** may be acquired,
identified independently from its browser-provided display label.
_Avoid_: Recorder, selected input

**Controller**:
An application-facing handle that exposes observable audio state and valid
commands without exposing the platform capability that performs them.
_Avoid_: Component, backend

**Playback**:
Loading and audibly presenting a **Playback Source**, including position,
seeking, pausing, and playback rate.
_Avoid_: Audio data, player component

**Audibility Level**:
A normalized **Playback** preference from silent to full. Its reported
capability distinguishes effective gain, best-effort direct control, and
unavailable control. Audibility Level is independent of mute and does not
promise perceived loudness.
_Avoid_: Volume, perceived loudness, input mute

**Analyser**:
A live read interface for time-domain, frequency-domain, and level information
from active audio.
_Avoid_: Analysis, visualizer

**Peaks**:
Ordered amplitude summaries sampled across a **Recording**. Peaks are neither
decoded audio samples nor a frequency spectrum.
_Avoid_: Waveform samples, spectrum bins

**Waveform Data**:
A duration-aware amplitude representation organized by channel and one or more
resolutions for presenting a **Waveform**. Waveform Data is independent of how
or where it was generated.
_Avoid_: Peaks, decoded audio, waveform image

**Waveform**:
A time-domain representation of amplitude across an audio duration, derived
from live samples, stored **Peaks**, or **Waveform Data**.
_Avoid_: Spectrum, audio data

**Waveform Viewport**:
A positive source-time interval within a **Waveform** duration that determines
which portion of the Waveform is currently presented. A Waveform Viewport may
pan, zoom, or follow **Playback** independently of **Waveform Selection** and
Playback position.
_Avoid_: Waveform selection, bounded playback, normalized viewport

**Waveform Selection**:
An ordered source-time interval chosen on a **Waveform** for **Analysis** or
bounded **Playback**. Its start and end may coincide while editing, but a
collapsed Waveform Selection is not a playable interval.
_Avoid_: Playback position, normalized selection

**Bounded Playback**:
**Playback** constrained for one operation to a positive source-time interval,
either once or repeatedly. It is distinct from repeating the whole
**Playback Source**.
_Avoid_: Whole-source repeat, sample-accurate scheduling

**Analysis**:
Deriving levels, **Peaks**, a **Waveform**, spectra, or range information without
changing the source audio.
_Avoid_: Playback, audio editing
