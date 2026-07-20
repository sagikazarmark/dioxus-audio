use dioxus::prelude::*;
use dioxus_audio::AudioData;
use dioxus_audio::decoding::{
    DecodeError, DecodeErrorKind, DecodeOptions, DecodedAudio, decode_audio_data,
};

const ENCODED_SAMPLE_RATE: u32 = 8_000;
const SOURCE_FRAMES: u32 = 800;

#[derive(Clone, Debug)]
enum DecodeOutcome {
    Idle,
    Running,
    Ready(DecodedAudio),
    Failed(DecodeError),
}

/// Decode generated and malformed complete Audio Data through the public API.
#[component]
pub fn DecodingExample() -> Element {
    let mut controls_mounted = use_signal(|| true);
    let outcome = use_signal(|| DecodeOutcome::Idle);

    rsx! {
        div { class: "grid gap-5",
            div { class: "flex flex-wrap gap-3",
                button {
                    class: "btn btn-sm btn-outline",
                    r#type: "button",
                    onclick: move |_| controls_mounted.toggle(),
                    if controls_mounted() {
                        "Unmount decode controls"
                    } else {
                        "Mount decode controls"
                    }
                }
                if controls_mounted() {
                    DecodeControls { outcome }
                }
            }
            {render_decode_result(outcome())}
        }
    }
}

#[component]
fn DecodeControls(mut outcome: Signal<DecodeOutcome>) -> Element {
    rsx! {
        button {
            class: "btn btn-sm btn-primary",
            r#type: "button",
            onclick: move |_| {
                start_decode(outcome, generated_stereo_wav(), DecodeOptions::default());
            },
            "Decode generated stereo WAV"
        }
        button {
            class: "btn btn-sm btn-outline",
            r#type: "button",
            onclick: move |_| {
                start_decode(
                    outcome,
                    AudioData::new(vec![0, 1, 2, 3, 4, 5], "audio/not-a-codec"),
                    DecodeOptions::default(),
                );
            },
            "Decode malformed Audio Data"
        }
        button {
            class: "btn btn-sm btn-outline",
            r#type: "button",
            onclick: move |_| {
                start_decode(
                    outcome,
                    generated_stereo_wav(),
                    DecodeOptions::default().with_max_decoded_bytes(0),
                );
            },
            "Decode with zero-byte limit"
        }
    }
}

fn start_decode(mut outcome: Signal<DecodeOutcome>, audio: AudioData, options: DecodeOptions) {
    outcome.set(DecodeOutcome::Running);
    spawn(async move {
        let next = match decode_audio_data(audio, options).await {
            Ok(audio) => DecodeOutcome::Ready(audio),
            Err(error) => DecodeOutcome::Failed(error),
        };
        outcome.set(next);
    });
}

fn render_decode_result(outcome: DecodeOutcome) -> Element {
    rsx! {
        div {
            class: "rounded-2xl border border-base-300 bg-base-100 p-5",
            role: "status",
            aria_label: "Decoded Audio result",
            match outcome {
                DecodeOutcome::Idle => rsx! {
                    p { class: "text-base-content/60", "No decode requested" }
                },
                DecodeOutcome::Running => rsx! {
                    p { class: "font-medium", "Decoding complete Audio Data" }
                },
                DecodeOutcome::Ready(audio) => {
                    let channel_views = audio.channels().len();
                    let channel_peaks = audio
                        .channels()
                        .map(|channel| {
                            channel
                                .iter()
                                .map(|sample| sample.abs())
                                .fold(0.0_f32, f32::max)
                        })
                        .collect::<Vec<_>>();
                    rsx! {
                        p { class: "font-semibold text-success", "Decoded Audio ready" }
                        dl { class: "mt-3 grid gap-2 font-mono text-sm tabular-nums sm:grid-cols-2",
                            div { "Channels: {audio.channel_count()}" }
                            div { "Frames per channel: {audio.frame_count()}" }
                            div {
                                "data-testid": "effective-sample-rate",
                                "Effective sample rate: {audio.sample_rate():.0} Hz"
                            }
                            div { "Duration: {audio.duration().as_secs_f64():.6} s" }
                            div { "Channel views: {channel_views}" }
                            div { "Encoded source rate: {ENCODED_SAMPLE_RATE} Hz" }
                            for (channel, peak) in channel_peaks.iter().enumerate() {
                                div {
                                    "data-testid": "channel-{channel}-peak",
                                    "Channel {channel} peak: {peak:.3}"
                                }
                            }
                        }
                    }
                }
                DecodeOutcome::Failed(error) => {
                    let category = error_category(error.kind());
                    rsx! {
                        p { class: "font-semibold text-error", "Decode failed: {category}" }
                        p { class: "mt-2 text-sm text-base-content/65", "{error.message()}" }
                        if let (Some(required), Some(configured)) =
                            (error.required_bytes(), error.configured_bytes())
                        {
                            p {
                                class: "mt-2 font-mono text-sm tabular-nums",
                                "Required bytes: {required}; configured bytes: {configured}"
                            }
                        }
                    }
                }
            }
        }
    }
}

fn error_category(kind: DecodeErrorKind) -> &'static str {
    match kind {
        DecodeErrorKind::UnsupportedPlatform => "unsupported platform",
        DecodeErrorKind::ResourceLimit => "resource limit",
        DecodeErrorKind::AllocationFailure => "allocation failure",
        DecodeErrorKind::DecodeRejected => "decode rejected",
        DecodeErrorKind::Backend => "backend failure",
        _ => "unknown failure",
    }
}

fn generated_stereo_wav() -> AudioData {
    let channels = 2_u16;
    let bits_per_sample = 16_u16;
    let bytes_per_sample = u32::from(bits_per_sample / 8);
    let data_len = SOURCE_FRAMES * u32::from(channels) * bytes_per_sample;
    let mut bytes = Vec::with_capacity(44 + data_len as usize);

    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&ENCODED_SAMPLE_RATE.to_le_bytes());
    bytes.extend_from_slice(
        &(ENCODED_SAMPLE_RATE * u32::from(channels) * bytes_per_sample).to_le_bytes(),
    );
    bytes.extend_from_slice(&(channels * (bits_per_sample / 8)).to_le_bytes());
    bytes.extend_from_slice(&bits_per_sample.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());

    for frame in 0..SOURCE_FRAMES {
        let time = frame as f32 / ENCODED_SAMPLE_RATE as f32;
        for (frequency, amplitude) in [(220.0_f32, 0.2_f32), (440.0, 0.7)] {
            let sample =
                (time * frequency * std::f32::consts::TAU).sin() * amplitude;
            let encoded = (sample * i16::MAX as f32) as i16;
            bytes.extend_from_slice(&encoded.to_le_bytes());
        }
    }

    AudioData::new(bytes, "audio/wav")
}
