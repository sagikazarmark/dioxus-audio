use dioxus::prelude::*;
use dioxus_audio::components::{
    AudioInputSelector, AudioStyles, LiveWaveform, RecorderControls,
};
use dioxus_audio::devices::use_audio_input_devices;
use dioxus_audio::recorder::{RecorderOptions, use_audio_recorder};

#[component]
fn App() -> Element {
    let devices = use_audio_input_devices();
    let recorder = use_audio_recorder(RecorderOptions::default(), devices.selected().into());

    rsx! {
        AudioStyles {}
        AudioInputSelector { devices }
        LiveWaveform { analyser: recorder.analyser() }
        RecorderControls { recorder }
    }
}
