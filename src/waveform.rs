//! Immutable, duration-aware Waveform Data values.

use std::fmt;
use std::ops::Range;
use std::sync::Arc;
use std::time::Duration;

/// The amplitude interpretation shared by every resolution in Waveform Data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AmplitudeMode {
    Magnitude,
    SignedEnvelope,
}

/// One minimum and maximum signed amplitude pair.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SignedEnvelope {
    /// The minimum observed signed amplitude.
    pub min: f32,
    /// The maximum observed signed amplitude.
    pub max: f32,
}

/// One source-time resolution supplied when constructing Waveform Data.
#[derive(Clone, Debug, PartialEq)]
pub struct WaveformLevel<T> {
    bucket_span: Duration,
    buckets: Vec<T>,
}

impl<T> WaveformLevel<T> {
    /// Create a level from a positive span and flat channel-major buckets.
    pub fn new(bucket_span: Duration, buckets: Vec<T>) -> Self {
        Self {
            bucket_span,
            buckets,
        }
    }
}

/// An exact positive source-time bucket span expressed as a duration ratio.
///
/// Explicit levels use a divisor of one. Peaks conversion may need a larger
/// divisor when its evenly spaced span is not an integral number of nanoseconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WaveformBucketSpan {
    numerator: Duration,
    divisor: usize,
}

impl WaveformBucketSpan {
    fn from_duration(duration: Duration) -> Self {
        Self {
            numerator: duration,
            divisor: 1,
        }
    }

    fn evenly_spaced(duration: Duration, buckets: usize) -> Self {
        Self {
            numerator: duration,
            divisor: buckets,
        }
    }

    pub fn numerator(self) -> Duration {
        self.numerator
    }

    pub fn divisor(self) -> usize {
        self.divisor
    }

    /// Return the span as a `Duration` when it is an integral number of
    /// nanoseconds.
    pub fn exact_duration(self) -> Option<Duration> {
        let divisor = self.divisor as u128;
        let nanos = self.numerator.as_nanos();
        nanos
            .is_multiple_of(divisor)
            .then(|| duration_from_nanos(nanos / divisor))
    }
}

/// A Waveform Data construction or selection error.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum WaveformError {
    ZeroDuration,
    NoChannels,
    NoResolutions,
    ZeroBucketSpan {
        resolution: usize,
    },
    NonIncreasingBucketSpan {
        resolution: usize,
    },
    NoBuckets {
        resolution: usize,
    },
    MisalignedChannelData {
        resolution: usize,
        values: usize,
        channels: usize,
    },
    BucketCountOverflow {
        resolution: usize,
    },
    DurationCoverage {
        resolution: usize,
        expected_buckets: usize,
        actual_buckets: usize,
    },
    InvalidMagnitude {
        resolution: usize,
        channel: usize,
        bucket: usize,
        value: f32,
    },
    InvalidSignedEnvelope {
        resolution: usize,
        channel: usize,
        bucket: usize,
        min: f32,
        max: f32,
    },
    ZeroBucketBudget,
    InvalidRange {
        start: Duration,
        end: Duration,
        duration: Duration,
    },
    EmptyPeaks,
}

impl fmt::Display for WaveformError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroDuration => formatter.write_str("duration must be positive"),
            Self::NoChannels => formatter.write_str("channel count must be positive"),
            Self::NoResolutions => formatter.write_str("at least one resolution is required"),
            Self::ZeroBucketSpan { resolution } => {
                write!(formatter, "resolution {resolution} has a zero bucket span")
            }
            Self::NonIncreasingBucketSpan { resolution } => write!(
                formatter,
                "resolution {resolution} is not ordered strictly coarser than its predecessor"
            ),
            Self::NoBuckets { resolution } => {
                write!(formatter, "resolution {resolution} has no buckets")
            }
            Self::MisalignedChannelData {
                resolution,
                values,
                channels,
            } => write!(
                formatter,
                "resolution {resolution} has {values} values, which cannot form {channels} equal channels"
            ),
            Self::BucketCountOverflow { resolution } => write!(
                formatter,
                "resolution {resolution} requires more buckets than this target can address"
            ),
            Self::DurationCoverage {
                resolution,
                expected_buckets,
                actual_buckets,
            } => write!(
                formatter,
                "resolution {resolution} needs {expected_buckets} buckets per channel to cover the duration, but has {actual_buckets}"
            ),
            Self::InvalidMagnitude {
                resolution,
                channel,
                bucket,
                value,
            } => write!(
                formatter,
                "resolution {resolution}, channel {channel}, bucket {bucket} has invalid magnitude {value:?}"
            ),
            Self::InvalidSignedEnvelope {
                resolution,
                channel,
                bucket,
                min,
                max,
            } => write!(
                formatter,
                "resolution {resolution}, channel {channel}, bucket {bucket} has invalid signed envelope [{min:?}, {max:?}]"
            ),
            Self::ZeroBucketBudget => formatter.write_str("bucket budget must be positive"),
            Self::InvalidRange {
                start,
                end,
                duration,
            } => write!(
                formatter,
                "range {start:?}..{end:?} must satisfy start < end <= {duration:?}"
            ),
            Self::EmptyPeaks => formatter.write_str("Peaks must be nonempty"),
        }
    }
}

impl std::error::Error for WaveformError {}

/// An immutable Waveform Data snapshot.
///
/// Clones share storage and compare by snapshot identity. Independently
/// constructed values compare unequal even when their buckets match.
#[derive(Clone, Debug)]
pub struct WaveformData {
    inner: Arc<WaveformDataInner>,
}

impl PartialEq for WaveformData {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Eq for WaveformData {}

#[derive(Debug)]
struct WaveformDataInner {
    duration: Duration,
    channels: usize,
    ladder: AmplitudeLadder,
}

#[derive(Debug)]
enum AmplitudeLadder {
    Magnitude(Vec<Resolution<f32>>),
    SignedEnvelope(Vec<Resolution<SignedEnvelope>>),
}

#[derive(Debug)]
struct Resolution<T> {
    bucket_span: WaveformBucketSpan,
    buckets_per_channel: usize,
    buckets: Vec<T>,
}

/// Metadata for one Waveform Data resolution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResolutionInfo {
    bucket_span: WaveformBucketSpan,
    buckets_per_channel: usize,
}

impl ResolutionInfo {
    /// The exact source-time span of each full bucket.
    pub fn bucket_span(self) -> WaveformBucketSpan {
        self.bucket_span
    }

    /// The number of buckets stored for each channel.
    pub fn buckets_per_channel(self) -> usize {
        self.buckets_per_channel
    }
}

/// A borrowed amplitude slice from one channel in a selected resolution.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AmplitudeSlice<'a> {
    /// Borrowed normalized Magnitude buckets.
    Magnitudes(&'a [f32]),
    /// Borrowed Signed Envelope buckets.
    SignedEnvelopes(&'a [SignedEnvelope]),
}

/// A borrowed range from one Waveform Data resolution.
#[derive(Debug)]
pub struct WaveformView<'a> {
    resolution: usize,
    channels: usize,
    buckets: Range<usize>,
    level: ViewLevel<'a>,
}

#[derive(Debug)]
enum ViewLevel<'a> {
    Magnitude(&'a Resolution<f32>),
    SignedEnvelope(&'a Resolution<SignedEnvelope>),
}

impl WaveformData {
    /// Construct Magnitude Waveform Data from normalized channel-major levels.
    pub fn from_magnitudes(
        duration: Duration,
        channels: usize,
        levels: Vec<WaveformLevel<f32>>,
    ) -> Result<Self, WaveformError> {
        let validated = validate_levels(duration, channels, levels, |location, value| {
            if value.is_finite() && (0.0..=1.0).contains(value) {
                Ok(())
            } else {
                Err(WaveformError::InvalidMagnitude {
                    resolution: location.resolution,
                    channel: location.channel,
                    bucket: location.bucket,
                    value: *value,
                })
            }
        })?;

        Ok(Self {
            inner: Arc::new(WaveformDataInner {
                duration,
                channels,
                ladder: AmplitudeLadder::Magnitude(validated),
            }),
        })
    }

    /// Construct Signed Envelope Waveform Data from channel-major levels.
    pub fn from_signed_envelopes(
        duration: Duration,
        channels: usize,
        levels: Vec<WaveformLevel<SignedEnvelope>>,
    ) -> Result<Self, WaveformError> {
        let validated = validate_levels(duration, channels, levels, |location, value| {
            if value.min.is_finite()
                && value.max.is_finite()
                && (-1.0..=1.0).contains(&value.min)
                && (-1.0..=1.0).contains(&value.max)
                && value.min <= value.max
            {
                Ok(())
            } else {
                Err(WaveformError::InvalidSignedEnvelope {
                    resolution: location.resolution,
                    channel: location.channel,
                    bucket: location.bucket,
                    min: value.min,
                    max: value.max,
                })
            }
        })?;

        Ok(Self {
            inner: Arc::new(WaveformDataInner {
                duration,
                channels,
                ladder: AmplitudeLadder::SignedEnvelope(validated),
            }),
        })
    }

    /// Convert Peaks into one evenly spaced mono Magnitude resolution.
    ///
    /// Peaks do not carry their original cadence, channel structure, or sign,
    /// so conversion cannot preserve those facts.
    pub fn from_peaks(duration: Duration, peaks: Vec<u8>) -> Result<Self, WaveformError> {
        if duration.is_zero() {
            return Err(WaveformError::ZeroDuration);
        }
        if peaks.is_empty() {
            return Err(WaveformError::EmptyPeaks);
        }

        let buckets_per_channel = peaks.len();
        let buckets = peaks
            .into_iter()
            .map(|peak| f32::from(peak) / 255.0)
            .collect();
        Ok(Self {
            inner: Arc::new(WaveformDataInner {
                duration,
                channels: 1,
                ladder: AmplitudeLadder::Magnitude(vec![Resolution {
                    bucket_span: WaveformBucketSpan::evenly_spaced(duration, buckets_per_channel),
                    buckets_per_channel,
                    buckets,
                }]),
            }),
        })
    }

    pub fn mode(&self) -> AmplitudeMode {
        match self.inner.ladder {
            AmplitudeLadder::Magnitude(_) => AmplitudeMode::Magnitude,
            AmplitudeLadder::SignedEnvelope(_) => AmplitudeMode::SignedEnvelope,
        }
    }

    pub fn duration(&self) -> Duration {
        self.inner.duration
    }

    pub fn channel_count(&self) -> usize {
        self.inner.channels
    }

    pub fn resolution_count(&self) -> usize {
        match &self.inner.ladder {
            AmplitudeLadder::Magnitude(levels) => levels.len(),
            AmplitudeLadder::SignedEnvelope(levels) => levels.len(),
        }
    }

    pub fn resolution(&self, index: usize) -> Option<ResolutionInfo> {
        let level = match &self.inner.ladder {
            AmplitudeLadder::Magnitude(levels) => levels
                .get(index)
                .map(|level| (level.bucket_span, level.buckets_per_channel)),
            AmplitudeLadder::SignedEnvelope(levels) => levels
                .get(index)
                .map(|level| (level.bucket_span, level.buckets_per_channel)),
        }?;
        Some(ResolutionInfo {
            bucket_span: level.0,
            buckets_per_channel: level.1,
        })
    }

    /// Borrow the finest resolution fitting a per-channel bucket budget.
    ///
    /// `range` is a nonempty half-open source-time range. The coarsest
    /// resolution is returned when no stored resolution fits the budget.
    pub fn select(
        &self,
        range: Range<Duration>,
        bucket_budget: usize,
    ) -> Result<WaveformView<'_>, WaveformError> {
        if bucket_budget == 0 {
            return Err(WaveformError::ZeroBucketBudget);
        }
        if range.start >= range.end || range.end > self.duration() {
            return Err(WaveformError::InvalidRange {
                start: range.start,
                end: range.end,
                duration: self.duration(),
            });
        }

        let mut chosen = self.resolution_count() - 1;
        let mut chosen_range = 0..0;
        for index in 0..self.resolution_count() {
            let resolution = self.resolution(index).expect("validated resolution");
            let range = intersecting_buckets(
                range.clone(),
                resolution.bucket_span,
                resolution.buckets_per_channel,
            );
            chosen = index;
            chosen_range = range.clone();
            if range.len() <= bucket_budget {
                break;
            }
        }

        let level = match &self.inner.ladder {
            AmplitudeLadder::Magnitude(levels) => ViewLevel::Magnitude(&levels[chosen]),
            AmplitudeLadder::SignedEnvelope(levels) => ViewLevel::SignedEnvelope(&levels[chosen]),
        };
        Ok(WaveformView {
            resolution: chosen,
            channels: self.channel_count(),
            buckets: chosen_range,
            level,
        })
    }
}

fn bucket_count(duration: Duration, bucket_span: Duration) -> Option<usize> {
    let count = duration.as_nanos().div_ceil(bucket_span.as_nanos());
    usize::try_from(count).ok()
}

fn intersecting_buckets(
    range: Range<Duration>,
    bucket_span: WaveformBucketSpan,
    buckets_per_channel: usize,
) -> Range<usize> {
    let numerator = bucket_span.numerator.as_nanos();
    let (start, _) = multiply_divide(range.start.as_nanos(), bucket_span.divisor, numerator)
        .unwrap_or((buckets_per_channel, false));
    let (end_floor, end_remainder) =
        multiply_divide(range.end.as_nanos(), bucket_span.divisor, numerator)
            .unwrap_or((buckets_per_channel, false));
    let end = end_floor.saturating_add(usize::from(end_remainder));
    let start = start.min(buckets_per_channel);
    let end = end.min(buckets_per_channel);
    start..end
}

fn multiply_divide(value: u128, multiplier: usize, divisor: u128) -> Option<(usize, bool)> {
    let mut multiplier = multiplier;
    let mut factor_whole = value / divisor;
    let mut factor_remainder = value % divisor;
    let mut result_whole = 0_u128;
    let mut result_remainder = 0_u128;

    while multiplier > 0 {
        if multiplier & 1 == 1 {
            add_fraction(
                &mut result_whole,
                &mut result_remainder,
                factor_whole,
                factor_remainder,
                divisor,
            )?;
        }

        multiplier >>= 1;
        if multiplier > 0 {
            let add_whole = factor_whole;
            let add_remainder = factor_remainder;
            add_fraction(
                &mut factor_whole,
                &mut factor_remainder,
                add_whole,
                add_remainder,
                divisor,
            )?;
        }
    }

    Some((usize::try_from(result_whole).ok()?, result_remainder != 0))
}

fn add_fraction(
    whole: &mut u128,
    remainder: &mut u128,
    add_whole: u128,
    add_remainder: u128,
    divisor: u128,
) -> Option<()> {
    *whole = whole.checked_add(add_whole)?;
    if *remainder >= divisor - add_remainder {
        *whole = whole.checked_add(1)?;
        *remainder -= divisor - add_remainder;
    } else {
        *remainder += add_remainder;
    }
    Some(())
}

fn duration_from_nanos(nanos: u128) -> Duration {
    let seconds = nanos / 1_000_000_000;
    let subsec_nanos = (nanos % 1_000_000_000) as u32;
    Duration::new(
        u64::try_from(seconds).expect("a divided Duration still fits Duration"),
        subsec_nanos,
    )
}

impl<'a> WaveformView<'a> {
    /// The selected resolution's finest-to-coarsest index.
    pub fn resolution_index(&self) -> usize {
        self.resolution
    }

    /// The exact source-time span of each full bucket.
    pub fn bucket_span(&self) -> WaveformBucketSpan {
        match self.level {
            ViewLevel::Magnitude(level) => level.bucket_span,
            ViewLevel::SignedEnvelope(level) => level.bucket_span,
        }
    }

    /// The index of the first intersecting bucket in the full resolution.
    pub fn first_bucket(&self) -> usize {
        self.buckets.start
    }

    /// The number of intersecting buckets per channel.
    pub fn bucket_count(&self) -> usize {
        self.buckets.len()
    }

    /// Borrow one channel's selected buckets without copying them.
    pub fn channel(&self, channel: usize) -> Option<AmplitudeSlice<'a>> {
        if channel >= self.channels {
            return None;
        }

        match self.level {
            ViewLevel::Magnitude(level) => {
                let channel_start = channel.checked_mul(level.buckets_per_channel)?;
                let start = channel_start.checked_add(self.buckets.start)?;
                let end = channel_start.checked_add(self.buckets.end)?;
                Some(AmplitudeSlice::Magnitudes(level.buckets.get(start..end)?))
            }
            ViewLevel::SignedEnvelope(level) => {
                let channel_start = channel.checked_mul(level.buckets_per_channel)?;
                let start = channel_start.checked_add(self.buckets.start)?;
                let end = channel_start.checked_add(self.buckets.end)?;
                Some(AmplitudeSlice::SignedEnvelopes(
                    level.buckets.get(start..end)?,
                ))
            }
        }
    }
}

#[derive(Clone, Copy)]
struct BucketLocation {
    resolution: usize,
    channel: usize,
    bucket: usize,
}

fn validate_levels<T>(
    duration: Duration,
    channels: usize,
    levels: Vec<WaveformLevel<T>>,
    mut validate_bucket: impl FnMut(BucketLocation, &T) -> Result<(), WaveformError>,
) -> Result<Vec<Resolution<T>>, WaveformError> {
    if duration.is_zero() {
        return Err(WaveformError::ZeroDuration);
    }
    if channels == 0 {
        return Err(WaveformError::NoChannels);
    }
    if levels.is_empty() {
        return Err(WaveformError::NoResolutions);
    }

    let mut previous_span = Duration::ZERO;
    let mut validated = Vec::with_capacity(levels.len());
    for (resolution, level) in levels.into_iter().enumerate() {
        if level.bucket_span.is_zero() {
            return Err(WaveformError::ZeroBucketSpan { resolution });
        }
        if resolution > 0 && level.bucket_span <= previous_span {
            return Err(WaveformError::NonIncreasingBucketSpan { resolution });
        }
        if level.buckets.is_empty() {
            return Err(WaveformError::NoBuckets { resolution });
        }
        if level.buckets.len() % channels != 0 {
            return Err(WaveformError::MisalignedChannelData {
                resolution,
                values: level.buckets.len(),
                channels,
            });
        }

        let buckets_per_channel = level.buckets.len() / channels;
        let expected_buckets = bucket_count(duration, level.bucket_span)
            .ok_or(WaveformError::BucketCountOverflow { resolution })?;
        if buckets_per_channel != expected_buckets {
            return Err(WaveformError::DurationCoverage {
                resolution,
                expected_buckets,
                actual_buckets: buckets_per_channel,
            });
        }

        for (index, value) in level.buckets.iter().enumerate() {
            validate_bucket(
                BucketLocation {
                    resolution,
                    channel: index / buckets_per_channel,
                    bucket: index % buckets_per_channel,
                },
                value,
            )?;
        }

        previous_span = level.bucket_span;
        validated.push(Resolution {
            bucket_span: WaveformBucketSpan::from_duration(level.bucket_span),
            buckets_per_channel,
            buckets: level.buckets,
        });
    }

    Ok(validated)
}
