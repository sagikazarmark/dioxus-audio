use std::mem::size_of;

use js_sys::Uint8Array;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{AudioBuffer, AudioContext};

use super::{DecodeError, DecodeOptions, DecodedAudio};
use crate::AudioData;

pub(super) async fn decode_audio_data(
    audio: AudioData,
    options: DecodeOptions,
) -> Result<DecodedAudio, DecodeError> {
    let context = AudioContext::new()
        .map_err(|error| DecodeError::backend(js_message("create AudioContext", &error)))?;
    let context = AudioContextGuard::new(context);
    let effective_sample_rate = context.sample_rate();

    let AudioData { bytes, mime_type } = audio;
    let encoded = Uint8Array::from(bytes.as_slice());
    let array_buffer = encoded.buffer();
    let decode = context
        .decode_audio_data(&array_buffer)
        .map_err(|error| DecodeError::backend(js_message("start browser decoding", &error)))?;

    drop(array_buffer);
    drop(encoded);
    drop(bytes);
    drop(mime_type);

    let decoded = JsFuture::from(decode)
        .await
        .map_err(|_| DecodeError::decode_rejected())?
        .dyn_into::<AudioBuffer>()
        .map_err(|error| DecodeError::backend(js_message("read decoded AudioBuffer", &error)))?;

    let channels = decoded.number_of_channels();
    let frames = decoded.length();
    if !effective_sample_rate.is_finite() || effective_sample_rate <= 0.0 {
        return Err(DecodeError::backend(
            "browser returned an invalid effective sample rate",
        ));
    }

    let required_bytes = options.check_decoded_size(u64::from(channels), u64::from(frames))?;
    let total_samples = required_bytes / size_of::<f32>() as u64;
    let total_samples = usize::try_from(total_samples)
        .map_err(|_| DecodeError::allocation_failure(required_bytes))?;
    let frames =
        usize::try_from(frames).map_err(|_| DecodeError::allocation_failure(required_bytes))?;
    let channels =
        usize::try_from(channels).map_err(|_| DecodeError::allocation_failure(required_bytes))?;
    let mut samples = Vec::new();
    samples
        .try_reserve_exact(total_samples)
        .map_err(|_| DecodeError::allocation_failure(required_bytes))?;
    samples.resize(total_samples, 0.0);

    for channel in 0..channels {
        let start = channel * frames;
        decoded
            .copy_from_channel(&mut samples[start..start + frames], channel as i32)
            .map_err(|error| {
                DecodeError::backend(js_message("copy decoded audio channel", &error))
            })?;
    }

    DecodedAudio::from_planar(samples, channels, effective_sample_rate)
        .map_err(|error| DecodeError::backend(format!("invalid decoded audio: {error}")))
}

struct AudioContextGuard(Option<AudioContext>);

impl AudioContextGuard {
    fn new(context: AudioContext) -> Self {
        Self(Some(context))
    }
}

impl std::ops::Deref for AudioContextGuard {
    type Target = AudioContext;

    fn deref(&self) -> &Self::Target {
        self.0
            .as_ref()
            .expect("decode context remains available until guard drop")
    }
}

impl Drop for AudioContextGuard {
    fn drop(&mut self) {
        let Some(context) = self.0.take() else {
            return;
        };
        if let Ok(close) = context.close() {
            spawn_local(async move {
                let _ = JsFuture::from(close).await;
            });
        }
    }
}

fn js_message(action: &str, value: &wasm_bindgen::JsValue) -> String {
    value
        .as_string()
        .map(|detail| format!("failed to {action}: {detail}"))
        .unwrap_or_else(|| format!("failed to {action}"))
}
