use std::f32::consts::TAU;
use std::time::Duration;

use dioxus::prelude::*;
use dioxus_audio::AudioData;
use dioxus_audio::components::{
    AudioPlayer, PlaybackAudibilitySlider, PlaybackMuteButton, PlaybackPlayPauseButton,
    PlaybackRateButton, PlaybackRepeatButton, PlaybackSeekSlider, PlaybackSkipButton,
    PlaybackStatusAnnouncer, PlaybackStopButton, WaveformPreview,
};
use dioxus_audio::playback::use_audio_player;

/// Lazily generate a two-second WAV tone when the player asks for its bytes.
#[component]
pub fn PlaybackExample() -> Element {
    let mut source = use_signal(|| None::<AudioData>);
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
                on_request_audio: move |_| source.set(Some(sine_wave(440.0))),
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
                        on_request_audio: move |_| source.set(Some(sine_wave(440.0))),
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
                            onclick: move |_| source.set(Some(sine_wave(660.0))),
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
        }
    }
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
