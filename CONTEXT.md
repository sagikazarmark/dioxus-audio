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

**Recorded Audio**:
The result of a completed **Recording**, containing **Audio Data**, duration,
**Peaks**, and the selected input identity when it is known.
_Avoid_: Recording, persisted recording

**Audio Data**:
Encoded audio bytes paired with their media type, independent of which
application stores or plays them.
_Avoid_: Recorded audio, decoded samples

**Audio Input Device**:
A discoverable source from which a **Recording** may capture audio, identified
independently from its browser-provided display label.
_Avoid_: Recorder, selected input

**Controller**:
An application-facing handle that exposes observable audio state and valid
commands without exposing the platform capability that performs them.
_Avoid_: Component, backend

**Playback**:
Loading and audibly presenting **Audio Data**, including position, seeking,
pausing, and playback rate.
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

**Waveform**:
A time-domain representation of amplitude across an audio duration, derived
from live samples or stored **Peaks**.
_Avoid_: Spectrum, audio data

**Analysis**:
Deriving levels, **Peaks**, a **Waveform**, spectra, or range information without
changing the source audio.
_Avoid_: Playback, audio editing
