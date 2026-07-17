use std::f32::consts::TAU;

use dioxus_audio::AudioData;

pub(super) fn peaks() -> Vec<u8> {
    (0..240)
        .map(|index| {
            let primary = (index as f32 * 0.17).sin().abs();
            let detail = (index as f32 * 0.61).sin().abs() * 0.28;
            ((primary + detail).min(1.0) * 230.0) as u8 + 12
        })
        .collect()
}

pub(super) fn generated_audio() -> AudioData {
    const SAMPLE_RATE: u32 = 44_100;
    const SECONDS: u32 = 2;
    const BITS_PER_SAMPLE: u16 = 16;

    let sample_count = SAMPLE_RATE * SECONDS;
    let data_size = sample_count * u32::from(BITS_PER_SAMPLE / 8);
    let mut bytes = Vec::with_capacity(44 + data_size as usize);

    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_size).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    bytes.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    bytes.extend_from_slice(&2_u16.to_le_bytes());
    bytes.extend_from_slice(&BITS_PER_SAMPLE.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_size.to_le_bytes());

    for index in 0..sample_count {
        let time = index as f32 / SAMPLE_RATE as f32;
        let fade_envelope = (time / 0.04).min(1.0) * ((SECONDS as f32 - time) / 0.08).min(1.0);
        let sample = (440.0 * time * TAU).sin() * fade_envelope * 0.18;
        bytes.extend_from_slice(&((sample * i16::MAX as f32) as i16).to_le_bytes());
    }

    AudioData::new(bytes, "audio/wav")
}
