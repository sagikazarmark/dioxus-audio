//! Audio playback state and hooks.

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use dioxus::prelude::*;

use crate::AudioData;
use crate::AudioError;
use crate::AudioErrorKind;
use crate::analysis::AudioAnalyser;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
mod web;

/// When Playback attaches the current source to its media resource.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlaybackLoadingPolicy {
    /// Begin acquiring the source as soon as it becomes current.
    #[default]
    Eager,
    /// Keep the source dormant until Playback is requested.
    OnPlay,
}

/// The cross-origin request policy for one URL-addressable Playback Source alternative.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlaybackSourceCrossOrigin {
    /// Fetch without credentials and require CORS authorization.
    Anonymous,
    /// Fetch with credentials and require credentialed CORS authorization.
    UseCredentials,
}

/// One validated URL-addressable Playback Source alternative.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaybackSourceAlternative {
    url: String,
    media_type: Option<String>,
    cross_origin: Option<PlaybackSourceCrossOrigin>,
}

impl PlaybackSourceAlternative {
    /// Validate a browser-resolvable URL reference.
    ///
    /// Relative references are accepted. Validation does not claim that the
    /// resource exists or that the browser can decode it.
    pub fn new(url: impl Into<String>) -> Result<Self, AudioError> {
        let url = url.into();
        if url.trim().is_empty() || url.chars().any(char::is_control) {
            return Err(AudioError::new(
                AudioErrorKind::InvalidConfiguration,
                "Playback Source URL must be non-empty and contain no control characters",
            ));
        }

        Ok(Self {
            url,
            media_type: None,
            cross_origin: None,
        })
    }

    /// Add an advisory media-type hint.
    pub fn with_media_type(mut self, media_type: impl Into<String>) -> Result<Self, AudioError> {
        let media_type = media_type.into();
        if media_type.trim().is_empty() || media_type.chars().any(char::is_control) {
            return Err(AudioError::new(
                AudioErrorKind::InvalidConfiguration,
                "Playback Source media type must be non-empty and contain no control characters",
            ));
        }
        self.media_type = Some(media_type);
        Ok(self)
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn media_type(&self) -> Option<&str> {
        self.media_type.as_deref()
    }

    /// Configure how the browser requests this cross-origin alternative.
    ///
    /// Alternatives without a policy remain direct-only. Anonymous CORS is
    /// required before a URL alternative can be attached to a Playback graph.
    pub fn with_cross_origin(mut self, cross_origin: PlaybackSourceCrossOrigin) -> Self {
        self.cross_origin = Some(cross_origin);
        self
    }

    pub fn cross_origin(&self) -> Option<PlaybackSourceCrossOrigin> {
        self.cross_origin
    }

    /// Whether this alternative declares the anonymous-CORS intent required
    /// for graph-backed Playback.
    pub fn is_graph_eligible(&self) -> bool {
        self.cross_origin == Some(PlaybackSourceCrossOrigin::Anonymous)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PlaybackSourceInput {
    AudioData(Arc<AudioData>),
    Url(Arc<[PlaybackSourceAlternative]>),
}

/// One owned Playback input and its loading policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaybackSource {
    input: PlaybackSourceInput,
    loading_policy: PlaybackLoadingPolicy,
}

impl PlaybackSource {
    pub fn audio_data(audio: AudioData) -> Self {
        Self {
            input: PlaybackSourceInput::AudioData(Arc::new(audio)),
            loading_policy: PlaybackLoadingPolicy::Eager,
        }
    }

    pub fn url(alternative: PlaybackSourceAlternative) -> Self {
        Self {
            input: PlaybackSourceInput::Url(Arc::from([alternative])),
            loading_policy: PlaybackLoadingPolicy::Eager,
        }
    }

    /// Build a URL-backed Playback Source from ordered alternatives.
    pub fn url_alternatives(
        alternatives: impl IntoIterator<Item = PlaybackSourceAlternative>,
    ) -> Result<Self, AudioError> {
        let alternatives: Arc<[PlaybackSourceAlternative]> = alternatives.into_iter().collect();
        if alternatives.is_empty() {
            return Err(AudioError::new(
                AudioErrorKind::InvalidConfiguration,
                "a URL Playback Source must contain at least one alternative",
            ));
        }

        Ok(Self {
            input: PlaybackSourceInput::Url(alternatives),
            loading_policy: PlaybackLoadingPolicy::Eager,
        })
    }

    /// Return the ordered URL alternatives, or `None` for Audio Data.
    pub fn alternatives(&self) -> Option<&[PlaybackSourceAlternative]> {
        match &self.input {
            PlaybackSourceInput::AudioData(_) => None,
            PlaybackSourceInput::Url(alternatives) => Some(alternatives),
        }
    }

    pub fn with_loading_policy(mut self, loading_policy: PlaybackLoadingPolicy) -> Self {
        self.loading_policy = loading_policy;
        self
    }

    pub fn loading_policy(&self) -> PlaybackLoadingPolicy {
        self.loading_policy
    }
}

impl From<AudioData> for PlaybackSource {
    fn from(audio: AudioData) -> Self {
        Self::audio_data(audio)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlaybackStatus {
    Empty,
    Loading,
    Ready,
    Playing,
    Paused,
    Ended,
    Failed(AudioError),
}

/// The lifecycle of the current Playback Source, independent of transport.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlaybackSourceLifecycle {
    Empty,
    Dormant,
    Loading,
    Playable,
    Failed,
}

/// The requested or confirmed transport state of Playback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlaybackTransport {
    Idle,
    PlayPending,
    Playing,
    Paused,
    Ended,
}

/// How ready the current source is to advance Playback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlaybackReadiness {
    Unavailable,
    LoadingMetadata,
    Metadata,
    Playable,
    Waiting,
}

/// Coarse network activity for the current Playback Source attempt.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlaybackNetworkActivity {
    /// No URL-addressable Playback Source is attached.
    #[default]
    Inactive,
    /// A URL-addressable Playback Source is attached but has no reported activity yet.
    Unknown,
    /// The browser reports that it is acquiring media data.
    Loading,
    /// The browser is not currently acquiring media data.
    Idle,
    /// Media acquisition has stopped unexpectedly without becoming a terminal failure.
    Stalled,
}

/// One immutable half-open source-time observation.
///
/// Buffered and seekable ranges are UI guidance from the browser. They may
/// disappear or shrink and do not guarantee that a future seek will succeed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlaybackTimeRange {
    start: Duration,
    end: Duration,
}

impl PlaybackTimeRange {
    pub fn new(start: Duration, end: Duration) -> Result<Self, PlaybackCommandError> {
        if end <= start {
            return Err(PlaybackCommandError(
                "Playback time range end must be after its start",
            ));
        }

        Ok(Self { start, end })
    }

    pub fn start(self) -> Duration {
        self.start
    }

    pub fn end(self) -> Duration {
        self.end
    }
}

/// A portable terminal failure of the current Playback Source.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlaybackSourceFailure {
    /// The source cannot participate in an owner-requested Playback graph.
    GraphIneligible(AudioError),
    Unsupported(AudioError),
    Network(AudioError),
    Decode(AudioError),
    Unknown(AudioError),
}

impl PlaybackSourceFailure {
    pub fn error(&self) -> &AudioError {
        match self {
            Self::GraphIneligible(error)
            | Self::Unsupported(error)
            | Self::Network(error)
            | Self::Decode(error)
            | Self::Unknown(error) => error,
        }
    }

    pub fn kind(&self) -> PlaybackSourceFailureKind {
        match self {
            Self::GraphIneligible(_) => PlaybackSourceFailureKind::GraphIneligible,
            Self::Unsupported(_) => PlaybackSourceFailureKind::Unsupported,
            Self::Network(_) => PlaybackSourceFailureKind::Network,
            Self::Decode(_) => PlaybackSourceFailureKind::Decode,
            Self::Unknown(_) => PlaybackSourceFailureKind::Unknown,
        }
    }
}

/// A portable kind for one URL alternative's initial load failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlaybackSourceFailureKind {
    GraphIneligible,
    Unsupported,
    Network,
    Decode,
    Unknown,
}

/// The coarse failure of one URL alternative during initial selection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaybackAlternativeFailure {
    alternative: PlaybackSourceAlternative,
    kind: PlaybackSourceFailureKind,
}

impl PlaybackAlternativeFailure {
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    fn new(alternative: PlaybackSourceAlternative, failure: &PlaybackSourceFailure) -> Self {
        Self {
            alternative,
            kind: failure.kind(),
        }
    }

    pub fn alternative(&self) -> &PlaybackSourceAlternative {
        &self.alternative
    }

    pub fn kind(&self) -> PlaybackSourceFailureKind {
        self.kind
    }
}

/// A play request failure that leaves the current source usable for retry.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlaybackPlayFailure {
    InteractionRequired(AudioError),
    Unknown(AudioError),
}

impl PlaybackPlayFailure {
    pub fn error(&self) -> &AudioError {
        match self {
            Self::InteractionRequired(error) | Self::Unknown(error) => error,
        }
    }
}

/// How directly setting Playback's audibility level affects output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlaybackAudibilityCapability {
    /// An owned Playback graph applies effective gain.
    EffectiveGraphGain,
    /// The media element accepts the level, but some browsers may not apply it audibly.
    BestEffortMediaElement,
    /// This Playback owner cannot set an audibility level.
    Unavailable,
}

/// The state of an opt-in owner-lifetime Playback graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlaybackGraphState {
    /// This Playback owner uses ordinary direct media playback.
    NotRequested,
    /// The graph is waiting for an eligible Playback Source.
    AwaitingSource,
    /// The graph is being created or attached to the current source.
    Preparing,
    /// The graph exists but its audio context is suspended.
    Suspended,
    /// The graph is running with the current source attached.
    Running,
    /// Playback needs another user interaction before graph output can resume.
    InteractionRequired,
    /// Graph setup failed permanently for this owner; transport degraded to direct Playback.
    Unavailable,
}

/// Immutable configuration for one Playback owner.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlaybackOptions {
    graph_backed: bool,
}

impl PlaybackOptions {
    /// Request owner-lifetime graph-backed Playback for eligible Playback Sources.
    pub const fn graph_backed() -> Self {
        Self { graph_backed: true }
    }
}

/// A finite normalized Playback audibility preference in the inclusive range `0.0..=1.0`.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct PlaybackAudibilityLevel(f64);

impl PlaybackAudibilityLevel {
    pub const SILENT: Self = Self(0.0);
    pub const FULL: Self = Self(1.0);

    /// Validate a normalized audibility level.
    pub fn new(value: f64) -> Result<Self, PlaybackCommandError> {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(PlaybackCommandError(
                "audibility level must be finite and between 0 and 1",
            ));
        }

        Ok(Self(value))
    }

    pub fn value(self) -> f64 {
        self.0
    }
}

impl Default for PlaybackAudibilityLevel {
    fn default() -> Self {
        Self::FULL
    }
}

// Construction excludes NaN, so the value has reflexive equality.
impl Eq for PlaybackAudibilityLevel {}

/// One coherent observation of Playback's independent state facets.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct PlaybackSnapshot {
    pub source: PlaybackSourceLifecycle,
    pub transport: PlaybackTransport,
    pub readiness: PlaybackReadiness,
    /// Network activity observed independently from readiness and transport.
    pub network: PlaybackNetworkActivity,
    /// Normalized buffered observations for the current source attempt.
    pub buffered: Arc<[PlaybackTimeRange]>,
    /// Normalized seekable observations for the current source attempt.
    pub seekable: Arc<[PlaybackTimeRange]>,
    /// The selected URL alternative, if the current source is URL-addressable.
    pub selected_alternative: Option<PlaybackSourceAlternative>,
    /// A terminal failure, separate from recoverable play-policy rejection.
    pub source_failure: Option<PlaybackSourceFailure>,
    /// Ordered initial URL failures, populated when no alternative becomes playable.
    pub alternative_failures: Arc<[PlaybackAlternativeFailure]>,
    pub play_failure: Option<PlaybackPlayFailure>,
    /// Whole-source repeat preference, retained across source replacement and unload.
    pub repeat: bool,
    /// Mute preference, retained independently from transport and audibility level.
    pub muted: bool,
    /// Normalized audibility preference, retained across source replacement and unload.
    pub audibility_level: PlaybackAudibilityLevel,
    /// The effectiveness contract for setting [`Self::audibility_level`].
    pub audibility_capability: PlaybackAudibilityCapability,
    /// Owner-lifetime graph state, independent from source and transport state.
    pub graph: PlaybackGraphState,
}

impl Default for PlaybackSnapshot {
    fn default() -> Self {
        Self {
            source: PlaybackSourceLifecycle::Empty,
            transport: PlaybackTransport::Idle,
            readiness: PlaybackReadiness::Unavailable,
            network: PlaybackNetworkActivity::Inactive,
            buffered: Arc::from([]),
            seekable: Arc::from([]),
            selected_alternative: None,
            source_failure: None,
            alternative_failures: Arc::from([]),
            play_failure: None,
            repeat: false,
            muted: false,
            audibility_level: PlaybackAudibilityLevel::FULL,
            audibility_capability: PlaybackAudibilityCapability::BestEffortMediaElement,
            graph: PlaybackGraphState::NotRequested,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaybackCommandError(&'static str);

impl fmt::Display for PlaybackCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for PlaybackCommandError {}

#[derive(Debug)]
pub struct PlaybackLifecycle {
    status: PlaybackStatus,
    snapshot: PlaybackSnapshot,
    graph_backed: bool,
}

impl Default for PlaybackLifecycle {
    fn default() -> Self {
        Self::new(PlaybackOptions::default())
    }
}

impl PlaybackLifecycle {
    pub fn new(options: PlaybackOptions) -> Self {
        let mut snapshot = PlaybackSnapshot::default();
        if options.graph_backed {
            snapshot.graph = PlaybackGraphState::AwaitingSource;
            snapshot.audibility_capability = PlaybackAudibilityCapability::EffectiveGraphGain;
        }
        Self {
            status: PlaybackStatus::Empty,
            snapshot,
            graph_backed: options.graph_backed,
        }
    }

    pub fn status(&self) -> &PlaybackStatus {
        &self.status
    }

    pub fn snapshot(&self) -> &PlaybackSnapshot {
        &self.snapshot
    }

    pub fn source(&self) -> &PlaybackSourceLifecycle {
        &self.snapshot.source
    }

    pub fn transport(&self) -> PlaybackTransport {
        self.snapshot.transport
    }

    pub fn readiness(&self) -> PlaybackReadiness {
        self.snapshot.readiness
    }

    pub fn network_activity(&self) -> PlaybackNetworkActivity {
        self.snapshot.network
    }

    pub fn play_failure(&self) -> Option<&PlaybackPlayFailure> {
        self.snapshot.play_failure.as_ref()
    }

    pub fn selected_alternative(&self) -> Option<&PlaybackSourceAlternative> {
        self.snapshot.selected_alternative.as_ref()
    }

    pub fn source_failure(&self) -> Option<&PlaybackSourceFailure> {
        self.snapshot.source_failure.as_ref()
    }

    pub fn repeat(&self) -> bool {
        self.snapshot.repeat
    }

    pub fn set_repeat(&mut self, repeat: bool) {
        self.snapshot.repeat = repeat;
    }

    pub fn toggle_repeat(&mut self) {
        self.snapshot.repeat = !self.snapshot.repeat;
    }

    pub fn muted(&self) -> bool {
        self.snapshot.muted
    }

    pub fn set_muted(&mut self, muted: bool) {
        self.snapshot.muted = muted;
    }

    pub fn toggle_muted(&mut self) {
        self.snapshot.muted = !self.snapshot.muted;
    }

    pub fn audibility_level(&self) -> PlaybackAudibilityLevel {
        self.snapshot.audibility_level
    }

    pub fn set_audibility_level(&mut self, level: f64) -> Result<(), PlaybackCommandError> {
        self.set_validated_audibility_level(PlaybackAudibilityLevel::new(level)?);
        Ok(())
    }

    fn set_validated_audibility_level(&mut self, level: PlaybackAudibilityLevel) {
        self.snapshot.audibility_level = level;
    }

    pub fn audibility_capability(&self) -> PlaybackAudibilityCapability {
        self.snapshot.audibility_capability
    }

    pub fn graph_state(&self) -> PlaybackGraphState {
        self.snapshot.graph
    }

    pub fn graph_preparing(&mut self) {
        if self.graph_backed && self.snapshot.graph != PlaybackGraphState::Unavailable {
            self.snapshot.graph = PlaybackGraphState::Preparing;
            self.snapshot.audibility_capability = PlaybackAudibilityCapability::EffectiveGraphGain;
        }
    }

    pub fn graph_awaiting_source(&mut self) {
        if self.graph_backed && self.snapshot.graph != PlaybackGraphState::Unavailable {
            self.snapshot.graph = PlaybackGraphState::AwaitingSource;
            self.snapshot.audibility_capability = PlaybackAudibilityCapability::EffectiveGraphGain;
        }
    }

    pub fn direct_audibility(&mut self) {
        if self.snapshot.graph != PlaybackGraphState::Unavailable {
            self.snapshot.audibility_capability =
                PlaybackAudibilityCapability::BestEffortMediaElement;
        }
    }

    pub fn graph_suspended(&mut self) {
        if self.graph_backed
            && !matches!(
                self.snapshot.graph,
                PlaybackGraphState::InteractionRequired | PlaybackGraphState::Unavailable
            )
        {
            self.snapshot.graph = PlaybackGraphState::Suspended;
        }
    }

    pub fn graph_running(&mut self) {
        if self.graph_backed
            && !matches!(
                self.snapshot.graph,
                PlaybackGraphState::InteractionRequired | PlaybackGraphState::Unavailable
            )
        {
            self.snapshot.graph = PlaybackGraphState::Running;
        }
    }

    pub fn graph_interaction_required(&mut self, error: AudioError) {
        if !self.graph_backed
            || self.snapshot.graph == PlaybackGraphState::Unavailable
            || !matches!(
                self.snapshot.source,
                PlaybackSourceLifecycle::Loading | PlaybackSourceLifecycle::Playable
            )
            || !matches!(
                self.snapshot.transport,
                PlaybackTransport::PlayPending | PlaybackTransport::Playing
            )
        {
            return;
        }

        self.status = PlaybackStatus::Failed(error.clone());
        self.snapshot.transport = if self.snapshot.source == PlaybackSourceLifecycle::Loading {
            PlaybackTransport::Idle
        } else {
            PlaybackTransport::Paused
        };
        if self.snapshot.readiness == PlaybackReadiness::Waiting {
            self.snapshot.readiness = PlaybackReadiness::Metadata;
        }
        self.snapshot.play_failure = Some(PlaybackPlayFailure::InteractionRequired(error));
        self.snapshot.graph = PlaybackGraphState::InteractionRequired;
    }

    pub fn graph_unavailable(&mut self) {
        if self.graph_backed {
            self.snapshot.graph = PlaybackGraphState::Unavailable;
            self.snapshot.audibility_capability =
                PlaybackAudibilityCapability::BestEffortMediaElement;
        }
    }

    fn clear_source_failures(&mut self) {
        self.snapshot.source_failure = None;
        self.snapshot.alternative_failures = Arc::from([]);
    }

    fn clear_source_observations(&mut self) {
        self.snapshot.selected_alternative = None;
        self.clear_source_failures();
        self.snapshot.play_failure = None;
        self.clear_attempt_observations();
    }

    fn clear_attempt_observations(&mut self) {
        self.snapshot.network = PlaybackNetworkActivity::Inactive;
        self.snapshot.buffered = Arc::from([]);
        self.snapshot.seekable = Arc::from([]);
    }

    pub fn loading(&mut self) {
        self.status = PlaybackStatus::Loading;
        self.snapshot.source = PlaybackSourceLifecycle::Loading;
        self.snapshot.transport = PlaybackTransport::Idle;
        self.snapshot.readiness = PlaybackReadiness::LoadingMetadata;
        self.clear_source_observations();
    }

    pub fn dormant(&mut self) {
        self.status = PlaybackStatus::Ready;
        self.snapshot.source = PlaybackSourceLifecycle::Dormant;
        self.snapshot.transport = PlaybackTransport::Idle;
        self.snapshot.readiness = PlaybackReadiness::Unavailable;
        self.clear_source_observations();
    }

    pub fn loaded(&mut self) {
        self.status = PlaybackStatus::Ready;
        self.snapshot.source = PlaybackSourceLifecycle::Playable;
        self.snapshot.readiness = PlaybackReadiness::Metadata;
        self.clear_source_failures();
    }

    pub fn metadata_loaded(&mut self) {
        if self.snapshot.source == PlaybackSourceLifecycle::Loading {
            self.snapshot.readiness = PlaybackReadiness::Metadata;
        }
    }

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    fn loading_alternative(&mut self) {
        if self.snapshot.source == PlaybackSourceLifecycle::Loading {
            self.status = PlaybackStatus::Loading;
            self.snapshot.readiness = PlaybackReadiness::LoadingMetadata;
            self.clear_source_failures();
            self.clear_attempt_observations();
        }
    }

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    fn source_attempt_started(&mut self, url_addressable: bool) {
        if self.snapshot.source == PlaybackSourceLifecycle::Loading {
            self.snapshot.network = if url_addressable {
                PlaybackNetworkActivity::Unknown
            } else {
                PlaybackNetworkActivity::Inactive
            };
            self.snapshot.buffered = Arc::from([]);
            self.snapshot.seekable = Arc::from([]);
        }
    }

    pub fn url_playable(&mut self, alternative: PlaybackSourceAlternative) {
        if self.snapshot.source == PlaybackSourceLifecycle::Loading {
            self.status = PlaybackStatus::Ready;
            self.snapshot.source = PlaybackSourceLifecycle::Playable;
            self.snapshot.readiness = PlaybackReadiness::Playable;
            self.snapshot.selected_alternative = Some(alternative);
            self.clear_source_failures();
        }
    }

    pub fn request_play(&mut self) -> Result<(), PlaybackCommandError> {
        if !matches!(
            self.snapshot.transport,
            PlaybackTransport::Idle | PlaybackTransport::Paused | PlaybackTransport::Ended
        ) {
            return Err(PlaybackCommandError("audio is not ready to play"));
        }

        match self.snapshot.source {
            PlaybackSourceLifecycle::Dormant => {
                self.status = PlaybackStatus::Loading;
                self.snapshot.source = PlaybackSourceLifecycle::Loading;
                self.snapshot.readiness = PlaybackReadiness::LoadingMetadata;
            }
            PlaybackSourceLifecycle::Loading => {
                self.status = PlaybackStatus::Loading;
            }
            PlaybackSourceLifecycle::Playable => {
                if self.snapshot.play_failure.is_some() {
                    self.status = PlaybackStatus::Ready;
                }
            }
            PlaybackSourceLifecycle::Empty | PlaybackSourceLifecycle::Failed => {
                return Err(PlaybackCommandError("audio is not ready to play"));
            }
        }
        self.snapshot.play_failure = None;
        if self.snapshot.graph == PlaybackGraphState::InteractionRequired {
            self.snapshot.graph = PlaybackGraphState::Suspended;
        }
        self.snapshot.transport = PlaybackTransport::PlayPending;
        Ok(())
    }

    pub fn playing(&mut self) {
        if !matches!(self.snapshot.source, PlaybackSourceLifecycle::Playable)
            || !matches!(
                self.snapshot.transport,
                PlaybackTransport::PlayPending | PlaybackTransport::Playing
            )
        {
            return;
        }
        self.status = PlaybackStatus::Playing;
        self.snapshot.transport = PlaybackTransport::Playing;
        self.snapshot.readiness = PlaybackReadiness::Playable;
        self.snapshot.play_failure = None;
    }

    pub fn waiting(&mut self) {
        if matches!(self.snapshot.source, PlaybackSourceLifecycle::Playable)
            && matches!(
                self.snapshot.transport,
                PlaybackTransport::PlayPending | PlaybackTransport::Playing
            )
        {
            self.snapshot.readiness = PlaybackReadiness::Waiting;
        }
    }

    pub fn playable(&mut self) {
        if matches!(self.snapshot.source, PlaybackSourceLifecycle::Playable) {
            self.snapshot.readiness = PlaybackReadiness::Playable;
        }
    }

    pub fn network_observed(&mut self, activity: PlaybackNetworkActivity) {
        if matches!(
            self.snapshot.source,
            PlaybackSourceLifecycle::Loading | PlaybackSourceLifecycle::Playable
        ) {
            self.snapshot.network = activity;
        }
    }

    /// Replace buffered and seekable observations for the current source attempt.
    ///
    /// Ranges are sorted and overlapping or touching ranges are merged. A later
    /// observation replaces the whole snapshot, so either collection may shrink.
    pub fn ranges_changed(
        &mut self,
        buffered: impl IntoIterator<Item = PlaybackTimeRange>,
        seekable: impl IntoIterator<Item = PlaybackTimeRange>,
    ) {
        if matches!(
            self.snapshot.source,
            PlaybackSourceLifecycle::Loading | PlaybackSourceLifecycle::Playable
        ) {
            self.snapshot.buffered = normalize_time_ranges(buffered);
            self.snapshot.seekable = normalize_time_ranges(seekable);
        }
    }

    pub fn paused(&mut self) {
        if matches!(
            self.snapshot.transport,
            PlaybackTransport::PlayPending | PlaybackTransport::Playing
        ) {
            if self.snapshot.source == PlaybackSourceLifecycle::Loading {
                self.snapshot.transport = PlaybackTransport::Idle;
                self.status = PlaybackStatus::Loading;
            } else {
                self.snapshot.transport = PlaybackTransport::Paused;
                self.status = PlaybackStatus::Paused;
            }
        }
    }

    /// Return a loaded source to its ready, idle state.
    pub fn stop(&mut self) -> Result<(), PlaybackCommandError> {
        if !matches!(self.snapshot.source, PlaybackSourceLifecycle::Playable) {
            return Err(PlaybackCommandError("audio is not loaded"));
        }

        self.status = PlaybackStatus::Ready;
        self.snapshot.transport = PlaybackTransport::Idle;
        self.snapshot.play_failure = None;
        Ok(())
    }

    pub fn ended(&mut self) {
        if self.snapshot.transport != PlaybackTransport::Playing {
            return;
        }
        self.status = PlaybackStatus::Ended;
        self.snapshot.transport = PlaybackTransport::Ended;
        self.snapshot.readiness = PlaybackReadiness::Playable;
    }

    pub fn play_rejected(&mut self, failure: PlaybackPlayFailure) {
        if !matches!(
            self.snapshot.source,
            PlaybackSourceLifecycle::Loading | PlaybackSourceLifecycle::Playable
        ) || !matches!(self.snapshot.transport, PlaybackTransport::PlayPending)
        {
            return;
        }

        self.status = PlaybackStatus::Failed(failure.error().clone());
        self.snapshot.transport = if self.snapshot.source == PlaybackSourceLifecycle::Loading {
            PlaybackTransport::Idle
        } else {
            PlaybackTransport::Paused
        };
        if self.snapshot.readiness == PlaybackReadiness::Waiting {
            self.snapshot.readiness = PlaybackReadiness::Metadata;
        }
        self.snapshot.play_failure = Some(failure);
    }

    pub fn seeked(&mut self, position: f64, duration: f64) {
        if matches!(
            self.status,
            PlaybackStatus::Empty | PlaybackStatus::Loading | PlaybackStatus::Failed(_)
        ) {
            return;
        }
        if duration.is_finite() && duration > 0.0 && position >= duration {
            self.status = PlaybackStatus::Ended;
            self.snapshot.transport = PlaybackTransport::Ended;
        } else if matches!(self.status, PlaybackStatus::Ended) {
            self.status = PlaybackStatus::Paused;
            self.snapshot.transport = PlaybackTransport::Paused;
        }
    }

    pub fn failed(&mut self, error: AudioError) {
        self.source_failed(PlaybackSourceFailure::Unknown(error));
    }

    pub fn source_failed(&mut self, failure: PlaybackSourceFailure) {
        let error = failure.error().clone();
        self.status = PlaybackStatus::Failed(error.clone());
        self.snapshot.source = PlaybackSourceLifecycle::Failed;
        self.snapshot.transport = PlaybackTransport::Idle;
        self.snapshot.readiness = PlaybackReadiness::Unavailable;
        self.snapshot.source_failure = Some(failure);
        self.snapshot.alternative_failures = Arc::from([]);
        self.snapshot.play_failure = None;
        self.clear_attempt_observations();
    }

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    fn source_exhausted(
        &mut self,
        failure: PlaybackSourceFailure,
        alternative_failures: Vec<PlaybackAlternativeFailure>,
    ) {
        self.source_failed(failure);
        self.snapshot.alternative_failures = alternative_failures.into();
    }

    pub fn unload(&mut self) {
        self.status = PlaybackStatus::Empty;
        self.snapshot.source = PlaybackSourceLifecycle::Empty;
        self.snapshot.transport = PlaybackTransport::Idle;
        self.snapshot.readiness = PlaybackReadiness::Unavailable;
        self.clear_source_observations();
        if self.graph_backed && self.snapshot.graph != PlaybackGraphState::Unavailable {
            self.snapshot.graph = PlaybackGraphState::AwaitingSource;
        }
    }
}

fn normalize_time_ranges(
    ranges: impl IntoIterator<Item = PlaybackTimeRange>,
) -> Arc<[PlaybackTimeRange]> {
    let mut ranges: Vec<_> = ranges.into_iter().collect();
    ranges.sort_unstable_by_key(|range| range.start);

    let mut normalized: Vec<PlaybackTimeRange> = Vec::with_capacity(ranges.len());
    for range in ranges {
        if let Some(last) = normalized.last_mut()
            && range.start <= last.end
        {
            last.end = last.end.max(range.end);
        } else {
            normalized.push(range);
        }
    }
    normalized.into()
}

/// Clamp a requested playback position to a usable finite timeline.
pub fn clamp_seek(position: f64, duration: f64) -> f64 {
    if !position.is_finite() || !duration.is_finite() || duration <= 0.0 {
        return 0.0;
    }
    position.clamp(0.0, duration)
}

#[derive(Clone, Copy, PartialEq)]
pub struct AudioPlayerController {
    status: ReadSignal<PlaybackStatus>,
    snapshot: ReadSignal<PlaybackSnapshot>,
    position: ReadSignal<Duration>,
    duration: ReadSignal<Duration>,
    rate: ReadSignal<f64>,
    analyser: ReadSignal<Option<AudioAnalyser>>,
    play: Callback<(), Result<(), PlaybackCommandError>>,
    pause: Callback<(), Result<(), PlaybackCommandError>>,
    stop: Callback<(), Result<(), PlaybackCommandError>>,
    seek: Callback<Duration>,
    skip: Callback<f64>,
    set_rate: Callback<f64, Result<(), PlaybackCommandError>>,
    set_repeat: Callback<bool>,
    set_muted: Callback<bool>,
    set_audibility_level: Callback<f64, Result<(), PlaybackCommandError>>,
}

impl AudioPlayerController {
    pub fn status(self) -> ReadSignal<PlaybackStatus> {
        self.status
    }

    pub fn snapshot(self) -> ReadSignal<PlaybackSnapshot> {
        self.snapshot
    }

    pub fn position(self) -> ReadSignal<Duration> {
        self.position
    }

    pub fn duration(self) -> ReadSignal<Duration> {
        self.duration
    }

    pub fn rate(self) -> ReadSignal<f64> {
        self.rate
    }

    /// The stable source-neutral Analyser for this graph-backed owner, once created.
    pub fn analyser(self) -> ReadSignal<Option<AudioAnalyser>> {
        self.analyser
    }

    pub fn repeat(self) -> bool {
        self.snapshot.read().repeat
    }

    pub fn muted(self) -> bool {
        self.snapshot.read().muted
    }

    pub fn audibility_level(self) -> PlaybackAudibilityLevel {
        self.snapshot.read().audibility_level
    }

    pub fn audibility_capability(self) -> PlaybackAudibilityCapability {
        self.snapshot.read().audibility_capability
    }

    pub fn play(self) -> Result<(), PlaybackCommandError> {
        self.play.call(())
    }

    pub fn pause(self) -> Result<(), PlaybackCommandError> {
        self.pause.call(())
    }

    /// Stop Playback atomically and reset its observable position.
    pub fn stop(self) -> Result<(), PlaybackCommandError> {
        self.stop.call(())
    }

    pub fn seek(self, position: Duration) {
        self.seek.call(position);
    }

    pub fn skip(self, seconds: f64) {
        self.skip.call(seconds);
    }

    pub fn set_rate(self, rate: f64) -> Result<(), PlaybackCommandError> {
        self.set_rate.call(rate)
    }

    /// Set the whole-source repeat preference.
    pub fn set_repeat(self, repeat: bool) {
        self.set_repeat.call(repeat);
    }

    /// Toggle the whole-source repeat preference.
    pub fn toggle_repeat(self) {
        self.set_repeat(!self.repeat());
    }

    /// Set mute without changing transport, position, or the audibility level preference.
    pub fn set_muted(self, muted: bool) {
        self.set_muted.call(muted);
    }

    /// Toggle mute without changing transport, position, or the audibility level preference.
    pub fn toggle_muted(self) {
        self.set_muted(!self.muted());
    }

    /// Set the normalized audibility preference in the inclusive range `0.0..=1.0`.
    ///
    /// Consult [`Self::audibility_capability`] before presenting this value as effective
    /// output gain. Direct media-element control is best effort on some browsers.
    pub fn set_audibility_level(self, level: f64) -> Result<(), PlaybackCommandError> {
        self.set_audibility_level.call(level)
    }
}

pub fn use_audio_player(
    source: ReadSignal<Option<PlaybackSource>>,
    initial_duration: Duration,
) -> AudioPlayerController {
    use_audio_player_with_options(source, initial_duration, PlaybackOptions::default())
}

/// Create a Playback owner with immutable owner-level options.
pub fn use_audio_player_with_options(
    source: ReadSignal<Option<PlaybackSource>>,
    initial_duration: Duration,
    options: PlaybackOptions,
) -> AudioPlayerController {
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        web::use_web_audio_player(source, initial_duration, options)
    }

    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    {
        let _ = source;
        use_unsupported_audio_player(initial_duration, options)
    }
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
fn use_unsupported_audio_player(
    initial_duration: Duration,
    options: PlaybackOptions,
) -> AudioPlayerController {
    let initial_lifecycle = PlaybackLifecycle::new(options);
    let mut status = use_signal(|| PlaybackStatus::Empty);
    let mut snapshot = use_signal(|| initial_lifecycle.snapshot().clone());
    let position = use_signal(|| Duration::ZERO);
    let mut duration = use_signal(|| initial_duration);
    let rate = use_signal(|| 1.0);
    let analyser = use_signal(|| None::<AudioAnalyser>);
    let initial_duration = use_memo(use_reactive!(|(initial_duration,)| initial_duration));
    use_effect(move || {
        duration.set(initial_duration());
        let error = AudioError::unsupported();
        let preferences = snapshot.peek().clone();
        status.set(PlaybackStatus::Failed(error.clone()));
        snapshot.set(PlaybackSnapshot {
            source: PlaybackSourceLifecycle::Failed,
            transport: PlaybackTransport::Idle,
            readiness: PlaybackReadiness::Unavailable,
            network: PlaybackNetworkActivity::Inactive,
            buffered: Arc::from([]),
            seekable: Arc::from([]),
            selected_alternative: None,
            source_failure: Some(PlaybackSourceFailure::Unsupported(error)),
            alternative_failures: Arc::from([]),
            play_failure: None,
            repeat: preferences.repeat,
            muted: preferences.muted,
            audibility_level: preferences.audibility_level,
            audibility_capability: PlaybackAudibilityCapability::Unavailable,
            graph: if preferences.graph == PlaybackGraphState::NotRequested {
                PlaybackGraphState::NotRequested
            } else {
                PlaybackGraphState::Unavailable
            },
        });
    });
    let unsupported: Callback<(), Result<(), PlaybackCommandError>> =
        use_callback(|()| Err(PlaybackCommandError("audio playback is unsupported")));
    let seek = use_callback(|_: Duration| {});
    let skip = use_callback(|_: f64| {});
    let set_rate: Callback<f64, Result<(), PlaybackCommandError>> =
        use_callback(|_: f64| Err(PlaybackCommandError("audio playback is unsupported")));
    let mut snapshot_for_repeat = snapshot;
    let set_repeat = use_callback(move |repeat: bool| {
        snapshot_for_repeat.write().repeat = repeat;
    });
    let mut snapshot_for_muted = snapshot;
    let set_muted = use_callback(move |muted: bool| {
        snapshot_for_muted.write().muted = muted;
    });
    let set_audibility_level: Callback<f64, Result<(), PlaybackCommandError>> =
        use_callback(|_: f64| Err(PlaybackCommandError("audio playback is unsupported")));
    AudioPlayerController {
        status: status.into(),
        snapshot: snapshot.into(),
        position: position.into(),
        duration: duration.into(),
        rate: rate.into(),
        analyser: analyser.into(),
        play: unsupported,
        pause: unsupported,
        stop: unsupported,
        seek,
        skip,
        set_rate,
        set_repeat,
        set_muted,
        set_audibility_level,
    }
}
