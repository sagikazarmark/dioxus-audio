//! Platform-independent audio analysis helpers.

use dioxus::prelude::*;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use std::cell::Cell;
use std::fmt;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::time::Duration;

/// Metadata needed to interpret one live Analysis snapshot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnalysisMetadata {
    sample_rate: f32,
    fft_size: u32,
    min_decibels: f32,
    max_decibels: f32,
    smoothing: f64,
}

impl AnalysisMetadata {
    pub fn new(
        sample_rate: f32,
        fft_size: u32,
        min_decibels: f32,
        max_decibels: f32,
        smoothing: f64,
    ) -> Self {
        Self {
            sample_rate,
            fft_size,
            min_decibels,
            max_decibels,
            smoothing,
        }
    }

    /// Effective sample rate of the audio graph, in hertz.
    pub fn sample_rate(self) -> f32 {
        self.sample_rate
    }

    /// Number of time-domain samples in each snapshot.
    pub fn fft_size(self) -> u32 {
        self.fft_size
    }

    /// Number of frequency values in each snapshot.
    pub fn frequency_bin_count(self) -> u32 {
        self.fft_size / 2
    }

    /// Width of each frequency bin, in hertz.
    pub fn frequency_bin_width(self) -> f32 {
        if self.fft_size == 0 {
            0.0
        } else {
            self.sample_rate / self.fft_size as f32
        }
    }

    /// Center frequency represented by `bin`, or `None` when it is out of range.
    pub fn frequency_for_bin(self, bin: u32) -> Option<f32> {
        (bin < self.frequency_bin_count()).then(|| bin as f32 * self.frequency_bin_width())
    }

    pub fn min_decibels(self) -> f32 {
        self.min_decibels
    }

    pub fn max_decibels(self) -> f32 {
        self.max_decibels
    }

    /// Convert a normalized byte-frequency value to its configured decibel value.
    pub fn decibels_for_frequency_value(self, value: f32) -> f32 {
        self.min_decibels + value.clamp(0.0, 1.0) * (self.max_decibels - self.min_decibels)
    }

    /// Frequency-domain smoothing time constant in the inclusive range `0.0..=1.0`.
    pub fn smoothing(self) -> f64 {
        self.smoothing
    }
}

/// Bounded scheduling options for reactive live Analysis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LiveAnalysisOptions {
    cadence: Duration,
}

impl LiveAnalysisOptions {
    pub const MIN_CADENCE: Duration = Duration::from_millis(16);
    pub const MAX_CADENCE: Duration = Duration::from_secs(1);

    /// Set the polling cadence, clamped to `16ms..=1s`.
    pub fn with_cadence(mut self, cadence: Duration) -> Self {
        self.cadence = cadence.clamp(Self::MIN_CADENCE, Self::MAX_CADENCE);
        self
    }

    pub fn cadence(self) -> Duration {
        self.cadence
    }
}

impl Default for LiveAnalysisOptions {
    fn default() -> Self {
        Self {
            cadence: Duration::from_millis(50),
        }
    }
}

/// Return normalized root mean square amplitude for one time-domain window.
pub fn rms_level(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let mean_square =
        samples.iter().map(|sample| sample * sample).sum::<f32>() / samples.len() as f32;
    mean_square.sqrt().clamp(0.0, 1.0)
}

/// Immutable values collected together from one Analyser.
///
/// Time-domain values are byte-quantized amplitudes normalized to
/// `-1.0..=1.0`. Frequency-domain values are byte-quantized magnitudes
/// normalized to `0.0..=1.0`. `level` is the normalized RMS amplitude of the
/// same time-domain window, not a peak, perceived loudness, or sound pressure
/// level measurement.
#[derive(Clone, Debug, PartialEq)]
pub struct LiveAnalysisSnapshot {
    time_domain: Arc<[f32]>,
    frequency_domain: Arc<[f32]>,
    level: f32,
    metadata: AnalysisMetadata,
}

impl LiveAnalysisSnapshot {
    pub fn time_domain(&self) -> &[f32] {
        &self.time_domain
    }

    pub fn frequency_domain(&self) -> &[f32] {
        &self.frequency_domain
    }

    pub fn level(&self) -> f32 {
        self.level
    }

    pub fn metadata(&self) -> AnalysisMetadata {
        self.metadata
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnalysisDomain {
    Waveform,
    Spectrum,
}

/// An Analyser has no active source its owner can safely expose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnalysisUnavailable;

impl fmt::Display for AnalysisUnavailable {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Analysis is unavailable")
    }
}

impl std::error::Error for AnalysisUnavailable {}

/// Opaque, cheap-to-clone reader for live audio analysis data.
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
#[derive(Clone)]
pub struct AudioAnalyser {
    inner: Weak<AudioAnalyserInner>,
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
struct AudioAnalyserInner {
    node: web_sys::AnalyserNode,
    sample_rate: f32,
    available: Cell<bool>,
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub(crate) struct AudioAnalyserControl {
    inner: Rc<AudioAnalyserInner>,
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
impl fmt::Debug for AudioAnalyser {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AudioAnalyser")
            .field("available", &self.is_available())
            .finish_non_exhaustive()
    }
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
impl PartialEq for AudioAnalyser {
    fn eq(&self, other: &Self) -> bool {
        self.inner.ptr_eq(&other.inner)
    }
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
impl AudioAnalyserControl {
    pub(crate) fn new(node: web_sys::AnalyserNode, sample_rate: f32) -> (Self, AudioAnalyser) {
        let inner = Rc::new(AudioAnalyserInner {
            node,
            sample_rate,
            available: Cell::new(false),
        });
        let analyser = AudioAnalyser {
            inner: Rc::downgrade(&inner),
        };
        (Self { inner }, analyser)
    }

    pub(crate) fn set_available(&self, available: bool) {
        self.inner.available.set(available);
    }

    pub(crate) fn node(&self) -> &web_sys::AnalyserNode {
        &self.inner.node
    }
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
impl AudioAnalyser {
    fn available_inner(&self) -> Result<Rc<AudioAnalyserInner>, AnalysisUnavailable> {
        self.inner
            .upgrade()
            .filter(|inner| inner.available.get())
            .ok_or(AnalysisUnavailable)
    }

    pub fn is_available(&self) -> bool {
        self.available_inner().is_ok()
    }

    /// Read current normalized values, or report that the owning source is unavailable.
    pub fn try_read(&self, domain: AnalysisDomain) -> Result<Vec<f32>, AnalysisUnavailable> {
        let inner = self.available_inner()?;
        Ok(read_node(&inner.node, domain))
    }

    /// Read current normalized values, returning no samples while unavailable.
    ///
    /// Use [`Self::try_read`] when source availability must be distinguished from
    /// a valid empty result.
    pub fn read(&self, domain: AnalysisDomain) -> Vec<f32> {
        self.try_read(domain).unwrap_or_default()
    }

    pub fn try_level(&self) -> Result<f32, AnalysisUnavailable> {
        Ok(rms_level(&self.try_read(AnalysisDomain::Waveform)?))
    }

    pub fn level(&self) -> f32 {
        self.try_level().unwrap_or(0.0)
    }

    fn snapshot(&self) -> Option<LiveAnalysisSnapshot> {
        let inner = self.available_inner().ok()?;
        let time_domain: Arc<[f32]> = read_node(&inner.node, AnalysisDomain::Waveform).into();
        let frequency_domain = read_node(&inner.node, AnalysisDomain::Spectrum).into();
        Some(LiveAnalysisSnapshot {
            level: rms_level(&time_domain),
            time_domain,
            frequency_domain,
            metadata: AnalysisMetadata::new(
                inner.sample_rate,
                inner.node.fft_size(),
                inner.node.min_decibels() as f32,
                inner.node.max_decibels() as f32,
                inner.node.smoothing_time_constant(),
            ),
        })
    }
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
fn read_node(node: &web_sys::AnalyserNode, domain: AnalysisDomain) -> Vec<f32> {
    match domain {
        AnalysisDomain::Waveform => {
            let mut samples = vec![0_u8; node.fft_size() as usize];
            node.get_byte_time_domain_data(&mut samples);
            samples
                .into_iter()
                .map(|sample| ((sample as f32 - 128.0) / 128.0).clamp(-1.0, 1.0))
                .collect()
        }
        AnalysisDomain::Spectrum => {
            let samples = js_sys::Uint8Array::new_with_length(node.frequency_bin_count());
            node.get_byte_frequency_data_with_u8_array(&samples);
            let mut values = vec![0_u8; samples.length() as usize];
            samples.copy_to(&mut values);
            values
                .into_iter()
                .map(|sample| sample as f32 / 255.0)
                .collect()
        }
    }
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
#[derive(Clone, Debug, PartialEq)]
pub struct AudioAnalyser {
    _private: (),
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
impl AudioAnalyser {
    pub fn is_available(&self) -> bool {
        false
    }

    pub fn try_read(&self, _domain: AnalysisDomain) -> Result<Vec<f32>, AnalysisUnavailable> {
        Err(AnalysisUnavailable)
    }

    pub fn read(&self, _domain: AnalysisDomain) -> Vec<f32> {
        Vec::new()
    }

    pub fn try_level(&self) -> Result<f32, AnalysisUnavailable> {
        Err(AnalysisUnavailable)
    }

    pub fn level(&self) -> f32 {
        0.0
    }

    fn snapshot(&self) -> Option<LiveAnalysisSnapshot> {
        None
    }
}

/// Reactively collect complete, interpretable snapshots from an optional Analyser.
///
/// The output is `None` until an Analyser is available and is cleared when the
/// Analyser is lost or replaced. Polling is suspended while the document is
/// hidden and ends when this hook is unmounted. Every hook invocation owns an
/// independent schedule.
pub fn use_live_analysis(
    analyser: ReadSignal<Option<AudioAnalyser>>,
    options: LiveAnalysisOptions,
) -> ReadSignal<Option<LiveAnalysisSnapshot>> {
    use_scheduled_analysis(analyser, options, AudioAnalyser::snapshot)
}

pub(crate) fn use_live_analysis_domain(
    analyser: ReadSignal<Option<AudioAnalyser>>,
    domain: AnalysisDomain,
) -> ReadSignal<Option<Arc<[f32]>>> {
    match domain {
        AnalysisDomain::Waveform => {
            use_scheduled_analysis(analyser, LiveAnalysisOptions::default(), read_waveform)
        }
        AnalysisDomain::Spectrum => {
            use_scheduled_analysis(analyser, LiveAnalysisOptions::default(), read_spectrum)
        }
    }
}

pub(crate) fn use_live_analysis_level(
    analyser: ReadSignal<Option<AudioAnalyser>>,
) -> ReadSignal<Option<f32>> {
    use_scheduled_analysis(analyser, LiveAnalysisOptions::default(), |analyser| {
        analyser.try_level().ok()
    })
}

fn read_waveform(analyser: &AudioAnalyser) -> Option<Arc<[f32]>> {
    Some(analyser.try_read(AnalysisDomain::Waveform).ok()?.into())
}

fn read_spectrum(analyser: &AudioAnalyser) -> Option<Arc<[f32]>> {
    Some(analyser.try_read(AnalysisDomain::Spectrum).ok()?.into())
}

fn use_scheduled_analysis<T: 'static>(
    analyser: ReadSignal<Option<AudioAnalyser>>,
    options: LiveAnalysisOptions,
    collect: fn(&AudioAnalyser) -> Option<T>,
) -> ReadSignal<Option<T>> {
    let parameters = use_memo(use_reactive!(|(analyser, options)| (analyser, options)));
    #[allow(unused_mut)]
    let mut value = use_signal(|| None::<T>);
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    let scheduler = use_live_analysis_scheduler();

    use_effect(move || {
        let (analyser, options) = parameters();
        value.set(None);

        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        {
            let generation = scheduler.next_generation();
            let Some(source_analyser) = analyser() else {
                return;
            };
            let scheduler = scheduler.clone();
            let scheduler_for_publish = scheduler.clone();
            gloo_timers::callback::Timeout::new(0, move || {
                wasm_bindgen_futures::spawn_local(run_live_analysis_schedule(
                    scheduler,
                    generation,
                    options.cadence(),
                    move || {
                        if analyser.peek().as_ref() != Some(&source_analyser) {
                            return false;
                        }
                        let next = collect(&source_analyser);
                        if !scheduler_for_publish.is_current(generation)
                            || analyser.peek().as_ref() != Some(&source_analyser)
                        {
                            return false;
                        }
                        if let Some(next) = next {
                            value.set(Some(next));
                        } else if value.peek().is_some() {
                            value.set(None);
                        }
                        true
                    },
                ));
            })
            .forget();
        }

        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        {
            let _ = (analyser, options, collect);
        }
    });

    value.into()
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub(crate) struct LiveAnalysisScheduler {
    generation: Cell<u64>,
    mounted: Cell<bool>,
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
impl Default for LiveAnalysisScheduler {
    fn default() -> Self {
        Self {
            generation: Cell::new(0),
            mounted: Cell::new(true),
        }
    }
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
impl LiveAnalysisScheduler {
    pub(crate) fn next_generation(&self) -> u64 {
        let generation = self.generation.get().wrapping_add(1);
        self.generation.set(generation);
        generation
    }

    fn is_current(&self, generation: u64) -> bool {
        self.mounted.get() && self.generation.get() == generation
    }

    fn unmount(&self) {
        self.mounted.set(false);
        self.next_generation();
    }
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
struct AnalysisUnmountGuard(Weak<LiveAnalysisScheduler>);

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
impl Drop for AnalysisUnmountGuard {
    fn drop(&mut self) {
        if let Some(scheduler) = self.0.upgrade() {
            scheduler.unmount();
        }
    }
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub(crate) fn use_live_analysis_scheduler() -> Rc<LiveAnalysisScheduler> {
    let scheduler = use_hook(|| Rc::new(LiveAnalysisScheduler::default()));
    let scheduler_for_guard = Rc::downgrade(&scheduler);
    use_hook(|| Rc::new(AnalysisUnmountGuard(scheduler_for_guard)));
    scheduler
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub(crate) async fn run_live_analysis_schedule(
    scheduler: Rc<LiveAnalysisScheduler>,
    generation: u64,
    cadence: Duration,
    mut publish: impl FnMut() -> bool + 'static,
) {
    let cadence_millis = cadence.as_millis() as u32;
    while scheduler.is_current(generation) {
        if document_hidden() {
            gloo_timers::future::TimeoutFuture::new(cadence_millis.max(250)).await;
            continue;
        }
        if !publish() {
            break;
        }
        gloo_timers::future::TimeoutFuture::new(cadence_millis).await;
    }
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
fn document_hidden() -> bool {
    web_sys::window()
        .and_then(|window| window.document())
        .is_some_and(|document| document.hidden())
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

// Construction and mutation exclude NaN, so Waveform Selection has reflexive equality.
impl Eq for WaveformSelection {}

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
