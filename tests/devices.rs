use dioxus_audio::devices::reconcile_selected_device;
use dioxus_audio::{AudioInputDevice, AudioInputId};

fn device(id: &str, label: &str, is_default: bool) -> AudioInputDevice {
    AudioInputDevice::new(AudioInputId::new(id), label, is_default)
}

#[test]
fn selected_device_is_cleared_when_it_disappears() {
    let selected = Some(AudioInputId::new("external"));
    let devices = vec![device("default", "", true)];

    assert_eq!(reconcile_selected_device(selected, &devices), None);
}

#[test]
fn devices_without_permission_have_useful_fallback_labels() {
    let microphone = device("default", "", true);

    assert_eq!(microphone.display_label(1), "Microphone 2");
}
