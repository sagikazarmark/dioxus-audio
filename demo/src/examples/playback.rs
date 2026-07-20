use std::cell::RefCell;
use std::f32::consts::TAU;
use std::rc::Rc;
use std::time::Duration;

use dioxus::prelude::*;
use dioxus_audio::AudioData;
use dioxus_audio::components::{
    AudioPlayer, PlaybackAudibilitySlider, PlaybackMuteButton, PlaybackPlayPauseButton,
    PlaybackRateButton, PlaybackRepeatButton, PlaybackSeekSlider, PlaybackSkipButton,
    PlaybackStatusAnnouncer, PlaybackStopButton, WaveformPreview,
};
use dioxus_audio::playback::{
    PlaybackLoadingPolicy, PlaybackSource, PlaybackSourceAlternative, PlaybackSourceFailure,
    PlaybackSourceFailureKind, PlaybackSourceLifecycle, PlaybackStatus, PlaybackTransport,
    use_audio_player,
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
                "data-readiness": format!("{:?}", snapshot.readiness).to_ascii_lowercase(),
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

fn source_failure_name(failure: Option<&PlaybackSourceFailure>) -> &'static str {
    match failure {
        None => "none",
        Some(failure) => source_failure_kind_name(failure.kind()),
    }
}

fn source_failure_kind_name(kind: PlaybackSourceFailureKind) -> &'static str {
    match kind {
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
