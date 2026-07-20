//! Platform-independent audio analysis helpers.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnalysisDomain {
    Waveform,
    Spectrum,
}

/// Opaque, cheap-to-clone reader for live audio analysis data.
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
#[derive(Clone, Debug, PartialEq)]
pub struct AudioAnalyser {
    node: web_sys::AnalyserNode,
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
impl AudioAnalyser {
    pub(crate) fn new(node: web_sys::AnalyserNode) -> Self {
        Self { node }
    }

    pub fn read(&self, domain: AnalysisDomain) -> Vec<f32> {
        match domain {
            AnalysisDomain::Waveform => {
                let mut samples = vec![0_u8; self.node.fft_size() as usize];
                self.node.get_byte_time_domain_data(&mut samples);
                samples
                    .into_iter()
                    .map(|sample| ((sample as f32 - 128.0) / 128.0).clamp(-1.0, 1.0))
                    .collect()
            }
            AnalysisDomain::Spectrum => {
                let samples = js_sys::Uint8Array::new_with_length(self.node.frequency_bin_count());
                self.node.get_byte_frequency_data_with_u8_array(&samples);
                let mut values = vec![0_u8; samples.length() as usize];
                samples.copy_to(&mut values);
                values
                    .into_iter()
                    .map(|sample| sample as f32 / 255.0)
                    .collect()
            }
        }
    }

    pub fn level(&self) -> f32 {
        let waveform = self.read(AnalysisDomain::Waveform);
        if waveform.is_empty() {
            return 0.0;
        }
        let mean_square =
            waveform.iter().map(|sample| sample * sample).sum::<f32>() / waveform.len() as f32;
        mean_square.sqrt().clamp(0.0, 1.0)
    }

    pub(crate) fn node(&self) -> &web_sys::AnalyserNode {
        &self.node
    }
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
#[derive(Clone, Debug, PartialEq)]
pub struct AudioAnalyser {
    _private: (),
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
impl AudioAnalyser {
    pub fn read(&self, _domain: AnalysisDomain) -> Vec<f32> {
        Vec::new()
    }

    pub fn level(&self) -> f32 {
        0.0
    }
}

/// Reduce amplitude peaks to at most `buckets` values, preserving the maximum
/// value from each source window.
pub fn downsample_peaks(peaks: &[u8], buckets: usize) -> Vec<u8> {
    let bucket_count = peaks.len().min(buckets);
    if bucket_count == 0 {
        return Vec::new();
    }

    (0..bucket_count)
        .map(|index| {
            let start = index * peaks.len() / bucket_count;
            let end = (index + 1) * peaks.len() / bucket_count;
            peaks[start..end].iter().copied().max().unwrap_or(0)
        })
        .collect()
}

/// Return the largest distance from Web Audio's unsigned silence value (128),
/// normalized to the full `u8` range.
pub fn peak_amplitude(samples: &[u8]) -> u8 {
    let distance = samples
        .iter()
        .map(|sample| (*sample as i16 - 128).unsigned_abs())
        .max()
        .unwrap_or(0);

    ((u32::from(distance) * 255 + 64) / 128).min(255) as u8
}

/// An ordered source-time interval within an audio timeline.
///
/// Boundaries are finite, non-negative seconds. They may coincide while the
/// selection is being edited, but a collapsed selection is not playable.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaveformSelection {
    start: f64,
    end: f64,
}

impl WaveformSelection {
    pub fn new(start: f64, end: f64) -> Self {
        let start = finite_non_negative(start);
        let end = finite_non_negative(end);

        Self {
            start: start.min(end),
            end: start.max(end),
        }
    }

    pub fn start(self) -> f64 {
        self.start
    }

    pub fn end(self) -> f64 {
        self.end
    }

    pub fn is_collapsed(self) -> bool {
        self.start == self.end
    }

    pub fn with_start(self, start: f64) -> Self {
        let start = if start.is_finite() {
            start.clamp(0.0, self.end)
        } else {
            self.start
        };
        Self {
            start,
            end: self.end,
        }
    }

    pub fn with_end(self, end: f64) -> Self {
        let end = if end.is_finite() {
            end.max(self.start)
        } else {
            self.end
        };
        Self {
            start: self.start,
            end,
        }
    }

    /// Clamp each boundary independently to an authoritative source duration.
    pub fn clamped_to_duration(self, duration_secs: f64) -> Self {
        let duration_secs = finite_non_negative(duration_secs);
        Self {
            start: self.start.min(duration_secs),
            end: self.end.min(duration_secs),
        }
    }

    /// Return whether this is a positive interval inside the source duration.
    pub fn is_playable_within(self, duration_secs: f64) -> bool {
        duration_secs.is_finite()
            && duration_secs > 0.0
            && !self.is_collapsed()
            && self.end <= duration_secs
    }
}

fn finite_non_negative(value: f64) -> f64 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

/// Trim a source-time interval from interleaved PCM without splitting channel frames.
pub fn trim_interleaved_pcm<T: Clone>(
    samples: &[T],
    channels: usize,
    duration_secs: f64,
    selection: WaveformSelection,
) -> Vec<T> {
    if channels == 0 || !duration_secs.is_finite() || duration_secs <= 0.0 {
        return Vec::new();
    }

    let selection = selection.clamped_to_duration(duration_secs);
    if selection.is_collapsed() {
        return Vec::new();
    }

    let frame_count = samples.len() / channels;
    let first_frame = (selection.start() / duration_secs * frame_count as f64).floor() as usize;
    let end_frame = (selection.end() / duration_secs * frame_count as f64).ceil() as usize;
    samples[first_frame.min(frame_count) * channels..end_frame.min(frame_count) * channels].to_vec()
}
