//! Audio input discovery and selection.

use dioxus::prelude::*;

use crate::AudioError;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use crate::AudioErrorKind;
use crate::{AudioInputDevice, AudioInputId};

/// Keep a selected input only while it remains present in the latest device
/// enumeration. `None` continues to mean the browser's default input.
pub fn reconcile_selected_device(
    selected: Option<AudioInputId>,
    devices: &[AudioInputDevice],
) -> Option<AudioInputId> {
    selected.filter(|selected_id| devices.iter().any(|device| device.id == *selected_id))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum MicrophonePermission {
    Unsupported,
    Unknown,
    Prompt,
    Granted,
    Denied,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeviceListStatus {
    Unsupported,
    Loading,
    Ready,
    Failed(AudioError),
}

#[derive(Clone, Copy, PartialEq)]
pub struct AudioInputDevices {
    devices: ReadSignal<Vec<AudioInputDevice>>,
    selected: Signal<Option<AudioInputId>>,
    status: ReadSignal<DeviceListStatus>,
    permission: ReadSignal<MicrophonePermission>,
    refresh: Callback,
    request_permission: Callback,
}

impl AudioInputDevices {
    pub fn devices(self) -> ReadSignal<Vec<AudioInputDevice>> {
        self.devices
    }

    pub fn selected(self) -> Signal<Option<AudioInputId>> {
        self.selected
    }

    pub fn status(self) -> ReadSignal<DeviceListStatus> {
        self.status
    }

    pub fn permission(self) -> ReadSignal<MicrophonePermission> {
        self.permission
    }

    pub fn select(mut self, id: Option<AudioInputId>) {
        self.selected.set(id);
    }

    pub fn refresh(self) {
        self.refresh.call(());
    }

    pub fn request_permission(self) {
        self.request_permission.call(());
    }
}

pub fn use_audio_input_devices() -> AudioInputDevices {
    let devices = use_signal(Vec::<AudioInputDevice>::new);
    let selected = use_signal(|| None::<AudioInputId>);

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    let mut status = use_signal(|| DeviceListStatus::Loading);
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    let mut status = use_signal(|| DeviceListStatus::Loading);

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    let mut permission = use_signal(|| MicrophonePermission::Unknown);
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    let mut permission = use_signal(|| MicrophonePermission::Unknown);

    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    use_effect(move || {
        status.set(DeviceListStatus::Unsupported);
        permission.set(MicrophonePermission::Unsupported);
    });

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    let refresh_generation = use_hook(|| std::rc::Rc::new(std::cell::Cell::new(0_u64)));
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    let permission_runtime = use_hook(|| std::rc::Rc::new(PermissionRuntime::default()));
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        let permission_runtime = permission_runtime.clone();
        use_hook(|| std::rc::Rc::new(PermissionUnmountGuard(permission_runtime)));
    }

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    let refresh = use_callback(move |()| {
        let request = refresh_generation.get().wrapping_add(1);
        refresh_generation.set(request);
        status.set(DeviceListStatus::Loading);
        let refresh_generation = refresh_generation.clone();
        spawn(async move {
            let result = web::enumerate_inputs().await;
            if refresh_generation.get() != request {
                return;
            }
            match result {
                Ok(latest) => {
                    if latest.iter().any(|device| !device.label.is_empty()) {
                        permission.set(MicrophonePermission::Granted);
                    }
                    let reconciled = reconcile_selected_device(selected(), &latest);
                    if reconciled != selected() {
                        let mut selected = selected;
                        selected.set(reconciled);
                    }
                    let mut devices = devices;
                    devices.set(latest);
                    status.set(DeviceListStatus::Ready);
                }
                Err(error) => status.set(DeviceListStatus::Failed(error)),
            }
        });
    });

    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    let refresh = use_callback(|()| {});

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    let request_permission = use_callback(move |()| {
        let request = permission_runtime.generation.get().wrapping_add(1);
        permission_runtime.generation.set(request);
        permission.set(MicrophonePermission::Prompt);
        let permission_runtime = permission_runtime.clone();
        let dioxus_runtime = dioxus::core::Runtime::current();
        let dioxus_scope = dioxus_runtime.current_scope_id();
        wasm_bindgen_futures::spawn_local(async move {
            let result = web::request_microphone_permission().await;
            if !permission_runtime.mounted.get() || permission_runtime.generation.get() != request {
                return;
            }
            dioxus_runtime.in_scope(dioxus_scope, || match result {
                Ok(()) => {
                    permission.set(MicrophonePermission::Granted);
                    refresh.call(());
                }
                Err(error) => {
                    permission.set(if error.kind() == AudioErrorKind::PermissionDenied {
                        MicrophonePermission::Denied
                    } else {
                        MicrophonePermission::Unknown
                    });
                    status.set(DeviceListStatus::Failed(error));
                }
            });
        });
    });

    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    let request_permission = use_callback(|()| {});

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        use_hook(move || {
            refresh.call(());
            std::rc::Rc::new(web::DeviceChangeListener::new(refresh))
        });
    }

    AudioInputDevices {
        devices: devices.into(),
        selected,
        status: status.into(),
        permission: permission.into(),
        refresh,
        request_permission,
    }
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
struct PermissionRuntime {
    generation: std::cell::Cell<u64>,
    mounted: std::cell::Cell<bool>,
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
impl Default for PermissionRuntime {
    fn default() -> Self {
        Self {
            generation: std::cell::Cell::new(0),
            mounted: std::cell::Cell::new(true),
        }
    }
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
struct PermissionUnmountGuard(std::rc::Rc<PermissionRuntime>);

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
impl Drop for PermissionUnmountGuard {
    fn drop(&mut self) {
        self.0.mounted.set(false);
        self.0
            .generation
            .set(self.0.generation.get().wrapping_add(1));
    }
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub(crate) mod web {
    use js_sys::Array;
    use wasm_bindgen::JsCast;
    use wasm_bindgen::prelude::*;
    use web_sys::{MediaDeviceInfo, MediaDeviceKind, MediaDevices, MediaStream};

    use super::*;

    pub async fn enumerate_inputs() -> Result<Vec<AudioInputDevice>, AudioError> {
        let media_devices = media_devices()?;
        let value = wasm_bindgen_futures::JsFuture::from(
            media_devices
                .enumerate_devices()
                .map_err(audio_error_from_js)?,
        )
        .await
        .map_err(audio_error_from_js)?;
        let entries = Array::from(&value);

        Ok(entries
            .iter()
            .filter_map(|entry| entry.dyn_into::<MediaDeviceInfo>().ok())
            .filter(|device| device.kind() == MediaDeviceKind::Audioinput)
            .map(|device| {
                let id = device.device_id();
                AudioInputDevice::new(AudioInputId::new(&id), device.label(), id == "default")
            })
            .collect())
    }

    pub async fn request_microphone_permission() -> Result<(), AudioError> {
        let constraints = web_sys::MediaStreamConstraints::new();
        constraints.set_audio(&JsValue::TRUE);
        let value = wasm_bindgen_futures::JsFuture::from(
            media_devices()?
                .get_user_media_with_constraints(&constraints)
                .map_err(audio_error_from_js)?,
        )
        .await
        .map_err(audio_error_from_js)?;
        let stream = value
            .dyn_into::<MediaStream>()
            .map_err(audio_error_from_js)?;
        stop_stream(&stream);
        Ok(())
    }

    pub struct DeviceChangeListener {
        media_devices: Option<MediaDevices>,
        callback: Option<Closure<dyn FnMut()>>,
    }

    impl DeviceChangeListener {
        pub fn new(refresh: Callback) -> Self {
            let Ok(media_devices) = media_devices() else {
                return Self {
                    media_devices: None,
                    callback: None,
                };
            };
            let runtime = dioxus::core::Runtime::current();
            let scope = runtime.current_scope_id();
            let callback =
                Closure::wrap(
                    Box::new(move || runtime.in_scope(scope, || refresh.call(())))
                        as Box<dyn FnMut()>,
                );
            let _ = media_devices.add_event_listener_with_callback(
                "devicechange",
                callback.as_ref().unchecked_ref(),
            );
            Self {
                media_devices: Some(media_devices),
                callback: Some(callback),
            }
        }
    }

    impl Drop for DeviceChangeListener {
        fn drop(&mut self) {
            if let (Some(media_devices), Some(callback)) = (&self.media_devices, &self.callback) {
                let _ = media_devices.remove_event_listener_with_callback(
                    "devicechange",
                    callback.as_ref().unchecked_ref(),
                );
            }
        }
    }

    pub(crate) fn media_devices() -> Result<MediaDevices, AudioError> {
        web_sys::window()
            .ok_or_else(AudioError::unsupported)?
            .navigator()
            .media_devices()
            .map_err(audio_error_from_js)
    }

    pub(crate) fn stop_stream(stream: &MediaStream) {
        for track in stream.get_tracks().iter() {
            if let Ok(track) = track.dyn_into::<web_sys::MediaStreamTrack>() {
                track.stop();
            }
        }
    }

    pub(crate) fn audio_error_from_js(value: JsValue) -> AudioError {
        let name = value
            .dyn_ref::<web_sys::DomException>()
            .map(web_sys::DomException::name)
            .unwrap_or_default();
        let kind = match name.as_str() {
            "NotAllowedError" | "SecurityError" => AudioErrorKind::PermissionDenied,
            "NotFoundError" | "OverconstrainedError" => AudioErrorKind::DeviceNotFound,
            "NotReadableError" | "AbortError" => AudioErrorKind::DeviceUnavailable,
            _ => AudioErrorKind::Backend,
        };
        let message = value
            .dyn_ref::<web_sys::DomException>()
            .map(web_sys::DomException::message)
            .or_else(|| value.as_string())
            .filter(|message| !message.is_empty())
            .unwrap_or_else(|| "browser audio operation failed".to_string());
        AudioError::new(kind, message)
    }
}
