use std::cell::RefCell;
use std::f32::consts::TAU;
use std::rc::Rc;
use std::time::Duration;

use dioxus::prelude::*;
use dioxus_audio::AudioData;
use dioxus_audio::analysis::{AudioAnalyser, LiveAnalysisOptions, use_live_analysis};
use dioxus_audio::components::{
    AudioPlayer, PlaybackAudibilitySlider, PlaybackMuteButton, PlaybackPlayPauseButton,
    PlaybackRateButton, PlaybackRepeatButton, PlaybackSeekSlider, PlaybackSkipButton,
    PlaybackStatusAnnouncer, PlaybackStopButton, WaveformPreview,
};
use dioxus_audio::playback::{
    PlaybackGraphState, PlaybackLoadingPolicy, PlaybackNetworkActivity, PlaybackOptions,
    PlaybackSource, PlaybackSourceAlternative, PlaybackSourceCrossOrigin, PlaybackSourceFailure,
    PlaybackSourceFailureKind, PlaybackSourceLifecycle, PlaybackStatus, PlaybackTimeRange,
    PlaybackTransport, use_audio_player, use_audio_player_with_options,
};

/// Lazily generate a two-second WAV tone when the player asks for its bytes.
#[component]
pub fn PlaybackExample() -> Element {
    rsx! { AudioDataPlaybackExample {} }
}

#[component]
fn AudioDataPlaybackExample() -> Element {
    let mut source = use_signal(|| None::<PlaybackSource>);
    let loaded = source.read().is_some();
    let custom_controller = use_audio_player(source.into(), Duration::from_secs(2));

    rsx! {
        div { class: "grid gap-4",
            WaveformPreview {
                peaks: preview_peaks(),
                bars: 64,
                height: 56.0,
                label: "Generated tone waveform",
            }
            AudioPlayer {
                source,
                duration_secs: 2.0,
                on_request_audio: move |_| source.set(Some(sine_wave(440.0).into())),
            }
            div {
                class: "rounded-2xl border border-base-300 bg-base-100 p-4",
                role: "group",
                aria_label: "Independent playback controls",
                p { class: "mb-3 text-xs font-semibold uppercase tracking-wider text-base-content/45",
                    "Independent controls"
                }
                PlaybackStatusAnnouncer { controller: custom_controller }
                PlaybackSeekSlider {
                    controller: custom_controller,
                    label: "Custom tone position".to_string(),
                }
                div { class: "mt-2 flex flex-wrap items-center justify-center gap-3",
                    PlaybackSkipButton {
                        controller: custom_controller,
                        seconds: -0.5,
                        label: "Rewind custom tone by half a second".to_string(),
                    }
                    PlaybackStopButton {
                        controller: custom_controller,
                        label: "Stop custom tone".to_string(),
                    }
                    PlaybackPlayPauseButton {
                        controller: custom_controller,
                        play_label: "Play custom tone".to_string(),
                        pause_label: "Pause custom tone".to_string(),
                        on_request_audio: move |_| source.set(Some(sine_wave(440.0).into())),
                    }
                    PlaybackSkipButton {
                        controller: custom_controller,
                        seconds: 0.5,
                        label: "Advance custom tone by half a second".to_string(),
                    }
                    PlaybackRateButton {
                        controller: custom_controller,
                        rates: vec![0.75, 1.0, 1.25],
                        label: "Listening rate".to_string(),
                    }
                    PlaybackMuteButton {
                        controller: custom_controller,
                        label: "Mute custom tone".to_string(),
                    }
                    PlaybackAudibilitySlider {
                        controller: custom_controller,
                        label: "Custom tone audibility".to_string(),
                    }
                    PlaybackRepeatButton {
                        controller: custom_controller,
                        label: "Repeat custom tone".to_string(),
                    }
                }
            }
            div { class: "flex items-center justify-between gap-3 text-sm text-base-content/60",
                div {
                    span {
                        if loaded { "Audio bytes loaded" } else { "Audio loads on first play" }
                    }
                    p { class: "mt-1 text-xs",
                        "Audibility level uses best-effort direct media control; mute remains independent."
                    }
                }
                if loaded {
                    div { class: "flex items-center gap-1",
                        button {
                            class: "btn btn-ghost btn-xs",
                            r#type: "button",
                            onclick: move |_| source.set(Some(sine_wave(660.0).into())),
                            "Replace"
                        }
                        button {
                            class: "btn btn-ghost btn-xs",
                            r#type: "button",
                            onclick: move |_| source.set(None),
                            "Unload"
                        }
                    }
                }
            }
            a {
                class: "btn btn-outline btn-sm justify-self-start",
                href: "/playback-source",
                "Show URL Playback Source"
            }
        }
    }
}

#[component]
pub fn GraphPlaybackExample() -> Element {
    let mut mounted = use_signal(|| true);
    let mut retained_analyser = use_signal(|| None::<AudioAnalyser>);
    let mut retained_available = use_signal(|| false);

    rsx! {
        section {
            class: "graph-playback-example grid gap-3 rounded-2xl border border-base-300 bg-base-100 p-4",
            role: "group",
            aria_label: "Graph-backed Playback",
            if mounted() {
                GraphPlaybackOwner {
                    on_analyser: move |analyser: AudioAnalyser| {
                        retained_available.set(analyser.is_available());
                        retained_analyser.set(Some(analyser));
                    },
                }
            } else {
                p { "Graph-backed Playback owner unmounted" }
            }
            output {
                class: "retained-analyser-state",
                "data-available": retained_available().to_string(),
                if retained_available() {
                    "Retained Analyser available"
                } else {
                    "Retained Analyser unavailable"
                }
            }
            button {
                class: "btn btn-ghost btn-xs justify-self-start",
                r#type: "button",
                disabled: retained_analyser.read().is_none(),
                onclick: move |_| {
                    retained_available.set(
                        retained_analyser
                            .read()
                            .as_ref()
                            .is_some_and(AudioAnalyser::is_available),
                    );
                },
                "Check retained Analyser"
            }
            button {
                class: "btn btn-ghost btn-xs justify-self-start",
                r#type: "button",
                disabled: !mounted(),
                onclick: move |_| mounted.set(false),
                "Unmount graph-backed Playback"
            }
        }
    }
}

#[component]
fn GraphPlaybackOwner(on_analyser: EventHandler<AudioAnalyser>) -> Element {
    let mut source = use_signal(|| None::<PlaybackSource>);
    let controller = use_audio_player_with_options(
        source.into(),
        Duration::from_secs(2),
        PlaybackOptions::graph_backed(),
    );
    let analyser_signal = controller.analyser();
    let analyser = analyser_signal();
    let analyser_available = analyser.as_ref().is_some_and(AudioAnalyser::is_available);
    let snapshot = controller.snapshot()();
    let selected_url = snapshot
        .selected_alternative
        .as_ref()
        .map(PlaybackSourceAlternative::url)
        .unwrap_or("none");
    let alternative_failures = snapshot
        .alternative_failures
        .iter()
        .map(|failure| source_failure_kind_name(failure.kind()))
        .collect::<Vec<_>>()
        .join(",");

    use_effect(move || {
        if let Some(analyser) = analyser_signal() {
            on_analyser.call(analyser);
        }
    });

    rsx! {
        div {
            class: "graph-playback-state",
            "data-graph": graph_state_name(snapshot.graph),
            "data-source": source_lifecycle_name(&snapshot.source),
            "data-transport": transport_name(snapshot.transport),
            "data-analyser": if analyser.is_some() { "present" } else { "absent" },
            "data-analyser-available": analyser_available.to_string(),
            "data-muted": snapshot.muted.to_string(),
            "data-audibility-level": snapshot.audibility_level.value().to_string(),
            "data-audibility-capability": format!("{:?}", snapshot.audibility_capability).to_ascii_lowercase(),
            "data-selected-alternative": selected_url,
            "data-source-failure": source_failure_name(snapshot.source_failure.as_ref()),
            "data-alternative-failures": alternative_failures,
        }
        if analyser.is_some() {
            GraphPlaybackAnalysis { analyser: analyser_signal }
        } else {
            output {
                class: "graph-analysis-state",
                "data-analysis": "unavailable",
                "data-analysis-level": "0",
            }
        }
        p { class: "text-sm text-base-content/60",
            "Analysis observes eligible Audio Data or anonymous-CORS URL-addressable alternatives before effective graph gain, so mute and level changes do not erase its input."
        }
        div { class: "flex flex-wrap items-center gap-2",
            button {
                class: "btn btn-ghost btn-xs",
                r#type: "button",
                onclick: move |_| source.set(Some(sine_wave(440.0).into())),
                "Load graph tone"
            }
            button {
                class: "btn btn-ghost btn-xs",
                r#type: "button",
                onclick: move |_| source.set(Some(anonymous_cors_playback_source())),
                "Load anonymous-CORS alternative"
            }
            button {
                class: "btn btn-ghost btn-xs",
                r#type: "button",
                onclick: move |_| source.set(Some(graph_ineligible_playback_source())),
                "Load graph-ineligible alternatives"
            }
            button {
                class: "btn btn-ghost btn-xs",
                r#type: "button",
                onclick: move |_| source.set(Some(mixed_playback_source())),
                "Load mixed Playback Source alternatives"
            }
            button {
                class: "btn btn-ghost btn-xs",
                r#type: "button",
                onclick: move |_| source.set(Some(selected_failure_playback_source())),
                "Load selected-failure alternatives"
            }
            PlaybackPlayPauseButton {
                controller,
                play_label: "Play graph tone".to_string(),
                pause_label: "Pause graph tone".to_string(),
            }
            PlaybackStopButton {
                controller,
                label: "Stop graph tone".to_string(),
            }
            PlaybackMuteButton {
                controller,
                label: "Mute graph tone".to_string(),
            }
            PlaybackAudibilitySlider {
                controller,
                label: "Graph tone audibility".to_string(),
            }
            button {
                class: "btn btn-ghost btn-xs",
                r#type: "button",
                disabled: source.read().is_none(),
                onclick: move |_| source.set(Some(sine_wave(660.0).into())),
                "Replace graph tone"
            }
            button {
                class: "btn btn-ghost btn-xs",
                r#type: "button",
                disabled: source.read().is_none(),
                onclick: move |_| source.set(None),
                "Unload graph tone"
            }
        }
    }
}

fn anonymous_cors_playback_source() -> PlaybackSource {
    PlaybackSource::url(anonymous_cors_alternative(
        "https://media.example/allowed.wav",
    ))
}

fn graph_ineligible_playback_source() -> PlaybackSource {
    PlaybackSource::url_alternatives(graph_ineligible_alternatives())
        .expect("the graph example supplies URL-addressable alternatives")
}

fn graph_ineligible_alternatives() -> [PlaybackSourceAlternative; 2] {
    let direct = PlaybackSourceAlternative::new("https://media.example/direct.wav")
        .expect("the direct-only alternative is valid");
    let credentialed = PlaybackSourceAlternative::new("https://media.example/private.wav")
        .expect("the credentialed alternative is valid")
        .with_cross_origin(PlaybackSourceCrossOrigin::UseCredentials);
    [direct, credentialed]
}

fn mixed_playback_source() -> PlaybackSource {
    let [direct, credentialed] = graph_ineligible_alternatives();
    let denied = anonymous_cors_alternative("https://media.example/denied.wav");
    let allowed = anonymous_cors_alternative("https://media.example/allowed.wav");
    PlaybackSource::url_alternatives([direct, credentialed, denied, allowed])
        .expect("the graph example supplies mixed URL-addressable alternatives")
}

fn selected_failure_playback_source() -> PlaybackSource {
    PlaybackSource::url_alternatives([
        anonymous_cors_alternative("https://media.example/allowed.wav"),
        anonymous_cors_alternative("https://media.example/backup.wav"),
    ])
    .expect("the graph example supplies fallback URL-addressable alternatives")
}

fn anonymous_cors_alternative(url: &str) -> PlaybackSourceAlternative {
    PlaybackSourceAlternative::new(url)
        .expect("the URL-addressable alternative is valid")
        .with_media_type("audio/wav")
        .expect("the graph media type is valid")
        .with_cross_origin(PlaybackSourceCrossOrigin::Anonymous)
}

#[component]
fn GraphPlaybackAnalysis(analyser: ReadSignal<Option<AudioAnalyser>>) -> Element {
    let analysis = use_live_analysis(analyser, LiveAnalysisOptions::default());
    let level = analysis().map(|analysis| analysis.level());

    rsx! {
        output {
            class: "graph-analysis-state",
            "data-analysis": if level.is_some() { "available" } else { "unavailable" },
            "data-analysis-level": level.unwrap_or_default().to_string(),
        }
    }
}

#[component]
pub fn UrlPlaybackExample() -> Element {
    let application_urls = use_hook(|| Rc::new(ApplicationMediaUrls::default()));
    let mut source = use_signal(|| None::<PlaybackSource>);
    let controller = use_audio_player(source.into(), Duration::from_secs(2));
    let snapshot = controller.snapshot()();
    let eager_urls = application_urls.clone();
    let on_play_urls = application_urls.clone();
    let alternative_urls = application_urls.clone();
    let replacement_urls = application_urls.clone();
    let selected_url = snapshot
        .selected_alternative
        .as_ref()
        .map(PlaybackSourceAlternative::url)
        .unwrap_or("none");
    let selected_media_type = snapshot
        .selected_alternative
        .as_ref()
        .and_then(PlaybackSourceAlternative::media_type)
        .unwrap_or("none");
    let alternative_failures = snapshot
        .alternative_failures
        .iter()
        .map(|failure| source_failure_kind_name(failure.kind()))
        .collect::<Vec<_>>()
        .join(",");

    rsx! {
        section {
            class: "url-playback-example grid gap-3 rounded-2xl border border-base-300 bg-base-100 p-4",
            role: "group",
            aria_label: "URL Playback Source",
            h3 { class: "font-semibold", "Application-owned Playback Source" }
            p { class: "text-sm text-base-content/60",
                "Load one URL or let Playback select the first playable URL from ordered typed alternatives."
            }
            PlaybackStatusAnnouncer { controller }
            div {
                class: "url-playback-state",
                "data-source": source_lifecycle_name(&snapshot.source),
                "data-transport": transport_name(snapshot.transport),
                "data-graph": graph_state_name(snapshot.graph),
                "data-readiness": format!("{:?}", snapshot.readiness).to_ascii_lowercase(),
                "data-network": network_activity_name(snapshot.network),
                "data-buffered": format_time_ranges(&snapshot.buffered),
                "data-seekable": format_time_ranges(&snapshot.seekable),
                "data-selected-alternative": selected_url,
                "data-selected-media-type": selected_media_type,
                "data-source-failure": source_failure_name(snapshot.source_failure.as_ref()),
                "data-alternative-failures": alternative_failures,
                "data-play-failure": if snapshot.play_failure.is_some() { "present" } else { "none" },
                "data-position": controller.position()().as_secs_f64().to_string(),
                "data-duration": controller.duration()().as_secs_f64().to_string(),
            }
            div { class: "flex flex-wrap items-center gap-2",
                PlaybackPlayPauseButton {
                    controller,
                    play_label: "Play URL Playback Source".to_string(),
                    pause_label: "Pause URL Playback Source".to_string(),
                }
                PlaybackStopButton {
                    controller,
                    label: "Stop URL Playback Source".to_string(),
                }
                button {
                    class: "btn btn-ghost btn-xs",
                    r#type: "button",
                    onclick: move |_| {
                        let url = eager_urls.create(sine_wave(330.0));
                        source.set(Some(playback_url_source(url, PlaybackLoadingPolicy::Eager)));
                    },
                    "Load eager URL"
                }
                button {
                    class: "btn btn-ghost btn-xs",
                    r#type: "button",
                    onclick: move |_| {
                        let url = on_play_urls.create(sine_wave(330.0));
                        source.set(Some(playback_url_source(url, PlaybackLoadingPolicy::OnPlay)));
                    },
                    "Load on-play URL"
                }
                button {
                    class: "btn btn-ghost btn-xs",
                    r#type: "button",
                    onclick: move |_| {
                        let playable_url = alternative_urls.create(sine_wave(440.0));
                        source.set(Some(playback_url_alternatives(playable_url)));
                    },
                    "Load URL alternatives"
                }
                button {
                    class: "btn btn-ghost btn-xs",
                    r#type: "button",
                    onclick: move |_| {
                        let url = replacement_urls.create(sine_wave(550.0));
                        source.set(Some(playback_url_source(url, PlaybackLoadingPolicy::Eager)));
                    },
                    "Replace URL Playback Source"
                }
                button {
                    class: "btn btn-ghost btn-xs",
                    r#type: "button",
                    onclick: move |_| source.set(None),
                    "Unload URL Playback Source"
                }
            }
            if let PlaybackStatus::Failed(error) = controller.status()() {
                div { class: "url-playback-error", role: "alert", "{error}" }
            }
        }
    }
}

fn playback_url_source(url: String, loading_policy: PlaybackLoadingPolicy) -> PlaybackSource {
    let alternative = PlaybackSourceAlternative::new(url)
        .and_then(|alternative| alternative.with_media_type("audio/wav"))
        .expect("the local demo URL and media type are valid");
    PlaybackSource::url(alternative).with_loading_policy(loading_policy)
}

fn playback_url_alternatives(playable_url: String) -> PlaybackSource {
    let unsupported = PlaybackSourceAlternative::new("/media/unsupported.alternative")
        .and_then(|alternative| {
            alternative.with_media_type("audio/x-dioxus-audio-definitely-unsupported")
        })
        .expect("the unsupported demo descriptor is valid");
    let unavailable = PlaybackSourceAlternative::new("/media/unavailable-alternative.wav")
        .expect("the untyped unavailable demo descriptor is valid");
    let playable = PlaybackSourceAlternative::new(playable_url)
        .and_then(|alternative| alternative.with_media_type("audio/wav"))
        .expect("the local demo URL and media type are valid");

    PlaybackSource::url_alternatives([unsupported, unavailable, playable])
        .expect("the demo supplies a non-empty ordered alternative set")
}

fn source_lifecycle_name(source: &PlaybackSourceLifecycle) -> &'static str {
    match source {
        PlaybackSourceLifecycle::Empty => "empty",
        PlaybackSourceLifecycle::Dormant => "dormant",
        PlaybackSourceLifecycle::Loading => "loading",
        PlaybackSourceLifecycle::Playable => "playable",
        PlaybackSourceLifecycle::Failed => "failed",
        _ => "unknown",
    }
}

fn transport_name(transport: PlaybackTransport) -> &'static str {
    match transport {
        PlaybackTransport::Idle => "idle",
        PlaybackTransport::PlayPending => "play-pending",
        PlaybackTransport::Playing => "playing",
        PlaybackTransport::Paused => "paused",
        PlaybackTransport::Ended => "ended",
        _ => "unknown",
    }
}

fn network_activity_name(activity: PlaybackNetworkActivity) -> &'static str {
    match activity {
        PlaybackNetworkActivity::Inactive => "inactive",
        PlaybackNetworkActivity::Unknown => "unknown",
        PlaybackNetworkActivity::Loading => "loading",
        PlaybackNetworkActivity::Idle => "idle",
        PlaybackNetworkActivity::Stalled => "stalled",
        _ => "unknown",
    }
}

fn graph_state_name(state: PlaybackGraphState) -> &'static str {
    match state {
        PlaybackGraphState::NotRequested => "not-requested",
        PlaybackGraphState::AwaitingSource => "awaiting-source",
        PlaybackGraphState::Preparing => "preparing",
        PlaybackGraphState::Suspended => "suspended",
        PlaybackGraphState::Running => "running",
        PlaybackGraphState::InteractionRequired => "interaction-required",
        PlaybackGraphState::Unavailable => "unavailable",
        _ => "unknown",
    }
}

fn format_time_ranges(ranges: &[PlaybackTimeRange]) -> String {
    ranges
        .iter()
        .map(|range| {
            format!(
                "{}-{}",
                range.start().as_secs_f64(),
                range.end().as_secs_f64()
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn source_failure_name(failure: Option<&PlaybackSourceFailure>) -> &'static str {
    match failure {
        None => "none",
        Some(failure) => source_failure_kind_name(failure.kind()),
    }
}

fn source_failure_kind_name(kind: PlaybackSourceFailureKind) -> &'static str {
    match kind {
        PlaybackSourceFailureKind::GraphIneligible => "graph-ineligible",
        PlaybackSourceFailureKind::Unsupported => "unsupported",
        PlaybackSourceFailureKind::Network => "network",
        PlaybackSourceFailureKind::Decode => "decode",
        PlaybackSourceFailureKind::Unknown => "unknown",
        _ => "unknown",
    }
}

#[derive(Default)]
struct ApplicationMediaUrls(RefCell<Vec<String>>);

impl ApplicationMediaUrls {
    fn create(&self, audio: AudioData) -> String {
        let url = application_object_url(audio)
            .expect("the browser should create a local demo media URL");
        self.0.borrow_mut().push(url.clone());
        url
    }
}

impl Drop for ApplicationMediaUrls {
    fn drop(&mut self) {
        for url in self.0.get_mut() {
            let _ = web_sys::Url::revoke_object_url(url);
        }
    }
}

fn application_object_url(audio: AudioData) -> Result<String, String> {
    let bytes = js_sys::Uint8Array::new_with_length(audio.bytes.len() as u32);
    bytes.copy_from(&audio.bytes);
    let parts = js_sys::Array::new();
    parts.push(&bytes);
    let properties = web_sys::BlobPropertyBag::new();
    properties.set_type(&audio.mime_type);
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &properties)
        .map_err(|_| "could not create local demo media".to_string())?;
    web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|_| "could not address local demo media".to_string())
}

fn preview_peaks() -> Vec<u8> {
    (0..128)
        .map(|index| {
            let envelope = (index.min(127 - index) as f32 / 32.0).min(1.0);
            ((index as f32 * 0.42).sin().abs() * envelope * 220.0) as u8 + 20
        })
        .collect()
}

fn sine_wave(frequency: f32) -> AudioData {
    const SAMPLE_RATE: u32 = 44_100;
    const SECONDS: u32 = 2;
    const CHANNELS: u16 = 1;
    const BITS_PER_SAMPLE: u16 = 16;

    let sample_count = SAMPLE_RATE * SECONDS;
    let data_size = sample_count * u32::from(BITS_PER_SAMPLE / 8);
    let mut bytes = Vec::with_capacity(44 + data_size as usize);

    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_size).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&CHANNELS.to_le_bytes());
    bytes.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    bytes.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    bytes.extend_from_slice(&2_u16.to_le_bytes());
    bytes.extend_from_slice(&BITS_PER_SAMPLE.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_size.to_le_bytes());

    for index in 0..sample_count {
        let time = index as f32 / SAMPLE_RATE as f32;
        let edge = (time / 0.04).min(1.0) * ((SECONDS as f32 - time) / 0.08).min(1.0);
        let sample = (frequency * time * TAU).sin() * edge * 0.18;
        bytes.extend_from_slice(&((sample * i16::MAX as f32) as i16).to_le_bytes());
    }

    AudioData::new(bytes, "audio/wav")
}
