use std::fmt::Write as _;

use dioxus::prelude::*;

use crate::analysis::{WaveformSelection, downsample_peaks};
use crate::playback::{AudioPlayerController, PlaybackSourceLifecycle};
use crate::waveform::{AmplitudeMode, AmplitudeSlice, SignedEnvelope, WaveformData};

use super::format_accessible_duration;

/// Render immutable Waveform Data as one responsive SVG path per channel.
#[component]
pub fn Waveform(
    data: WaveformData,
    #[props(default = 512)] bucket_budget: usize,
    #[props(default = 96.0)] height: f64,
    #[props(default)] label: Option<String>,
) -> Element {
    let height = if height.is_finite() {
        height.max(1.0)
    } else {
        96.0
    };
    let view = data
        .select(
            std::time::Duration::ZERO..data.duration(),
            bucket_budget.max(1),
        )
        .expect("Waveform Data and presentation budget are valid");
    let channel_height = height / data.channel_count() as f64;
    let paths = (0..data.channel_count())
        .filter_map(|channel| {
            let top = channel as f64 * channel_height;
            match view.channel(channel)? {
                AmplitudeSlice::Magnitudes(values) => {
                    Some(magnitude_path(values, top, channel_height))
                }
                AmplitudeSlice::SignedEnvelopes(values) => {
                    Some(signed_envelope_path(values, top, channel_height))
                }
            }
        })
        .collect::<Vec<_>>();
    let width = view.bucket_count().max(1);
    let role = label.as_ref().map(|_| "img");
    let amplitude_mode = match data.mode() {
        AmplitudeMode::Magnitude => "magnitude",
        AmplitudeMode::SignedEnvelope => "signed-envelope",
    };
    let channel_count = data.channel_count();
    let resolution = view.resolution_index();
    let bucket_count = view.bucket_count();

    rsx! {
        svg {
            class: "dioxus-audio dioxus-audio__waveform dioxus-audio__waveform-data",
            role,
            "aria-label": label,
            "aria-hidden": role.is_none(),
            "data-amplitude-mode": amplitude_mode,
            "data-channel-count": channel_count,
            "data-resolution": resolution,
            "data-bucket-count": bucket_count,
            width: "100%",
            height: "{height}",
            view_box: "0 0 {width} {height}",
            preserve_aspect_ratio: "none",
            for path_data in paths {
                path {
                    class: "dioxus-audio__waveform-channel",
                    d: path_data,
                }
            }
        }
    }
}

fn magnitude_path(values: &[f32], top: f64, height: f64) -> String {
    let baseline = top + height;
    let mut path = format!("M0 {baseline}");
    for (index, value) in values.iter().enumerate() {
        let x = index as f64;
        let next_x = x + 1.0;
        let y = baseline - f64::from(*value) * height;
        let _ = write!(path, "L{x} {y}H{next_x}");
    }
    let _ = write!(path, "L{} {baseline}Z", values.len());
    path
}

fn signed_envelope_path(values: &[SignedEnvelope], top: f64, height: f64) -> String {
    let center = top + height / 2.0;
    let amplitude_height = height / 2.0;
    let upper = |value: SignedEnvelope| center - f64::from(value.max) * amplitude_height;
    let lower = |value: SignedEnvelope| center - f64::from(value.min) * amplitude_height;

    let mut path = format!("M0 {}", upper(values[0]));
    for (index, value) in values.iter().copied().enumerate() {
        let next_x = index + 1;
        if index > 0 {
            let _ = write!(path, "V{}", upper(value));
        }
        let _ = write!(path, "H{next_x}");
    }

    let last = values[values.len() - 1];
    let _ = write!(path, "L{} {}", values.len(), lower(last));
    for index in (0..values.len()).rev() {
        if index + 1 < values.len() {
            let _ = write!(path, "V{}", lower(values[index]));
        }
        let _ = write!(path, "H{index}");
    }
    path.push('Z');
    path
}

/// Present Playback position and one controlled Waveform Selection over Waveform Data.
///
/// The three interactions remain native sliders. Pointer movement is presented as an
/// internal draft and is committed through the Controller or `on_selection_change`
/// when the native interaction completes.
#[component]
pub fn InteractiveWaveform(
    data: WaveformData,
    controller: AudioPlayerController,
    selection: WaveformSelection,
    on_selection_change: EventHandler<WaveformSelection>,
    #[props(default = 512)] bucket_budget: usize,
    #[props(default = 96.0)] height: f64,
    #[props(default = 0.01)] fine_step_secs: f64,
    #[props(default = 1.0)] coarse_step_secs: f64,
    #[props(default = "Interactive waveform".to_string())] label: String,
    #[props(default = "Playback position".to_string())] playback_label: String,
    #[props(default = "Selection start".to_string())] selection_start_label: String,
    #[props(default = "Selection end".to_string())] selection_end_label: String,
) -> Element {
    let waveform_duration = data.duration().as_secs_f64();
    let playback_duration = controller.duration()().as_secs_f64();
    let playback_position = controller.position()().as_secs_f64().min(waveform_duration);
    let playback_source = controller.snapshot()().source;
    let has_playback_duration = playback_duration.is_finite() && playback_duration > 0.0;
    let can_seek =
        has_playback_duration && matches!(playback_source, PlaybackSourceLifecycle::Playable);
    let seek_end = if has_playback_duration {
        playback_duration.min(waveform_duration)
    } else {
        0.0
    };
    let fine_step_secs = positive_step(fine_step_secs, 0.01);
    let coarse_step_secs = positive_step(coarse_step_secs, 1.0);
    let controlled_selection = selection.clamped_to_duration(waveform_duration);
    let mut draft_seek = use_signal(|| None::<f64>);
    let mut draft_selection = use_signal(|| None::<WaveformSelection>);
    use_effect(use_reactive!(|(
        controlled_selection,
        waveform_duration,
        seek_end,
        can_seek,
    )| {
        let _ = (controlled_selection, waveform_duration, seek_end, can_seek);
        draft_selection.set(None);
        draft_seek.set(None);
    }));
    let presented_position = draft_seek().unwrap_or(playback_position);
    let presented_selection = draft_selection().unwrap_or(controlled_selection);
    let start_percent = presented_selection.start() / waveform_duration * 100.0;
    let end_percent = presented_selection.end() / waveform_duration * 100.0;
    let selection_width = end_percent - start_percent;
    let position_percent = presented_position / waveform_duration * 100.0;
    let collapsed = presented_selection.is_collapsed();
    let position_value_text = format_accessible_duration(presented_position);
    let start_value_text = format_accessible_duration(presented_selection.start());
    let end_value_text = format_accessible_duration(presented_selection.end());
    let (collapse_edge, start_hit_offset, end_hit_offset) = if !collapsed {
        ("none", "0rem", "0rem")
    } else if presented_selection.start() == 0.0 {
        ("start", "0rem", "1.5rem")
    } else if presented_selection.end() == waveform_duration {
        ("end", "-1.5rem", "0rem")
    } else {
        ("middle", "-0.75rem", "0.75rem")
    };

    rsx! {
        div {
            class: "dioxus-audio dioxus-audio__interactive-waveform",
            role: "group",
            aria_label: label,
            "data-collapsed": collapsed,
            "data-collapse-edge": collapse_edge,
            Waveform { data, bucket_budget, height, label: None }
            div {
                class: "dioxus-audio__interactive-position",
                style: "left: {position_percent}%",
            }
            div {
                class: "dioxus-audio__interactive-selection",
                style: "left: {start_percent}%; width: {selection_width}%",
            }
            input {
                class: "dioxus-audio__interactive-input dioxus-audio__interactive-input--playback",
                r#type: "range",
                min: "0",
                max: "{waveform_duration}",
                step: "{fine_step_secs}",
                value: "{presented_position}",
                disabled: !can_seek,
                aria_label: playback_label,
                aria_valuemin: 0.0,
                aria_valuemax: seek_end,
                aria_valuetext: position_value_text,
                oninput: move |event| {
                    if let Some(value) = source_time(&event.value(), waveform_duration) {
                        draft_seek.set(Some(value.min(seek_end)));
                    }
                },
                onchange: move |event| {
                    if let Some(value) = source_time(&event.value(), waveform_duration) {
                        draft_seek.set(None);
                        if can_seek {
                            controller.seek(std::time::Duration::from_secs_f64(
                                value.min(playback_duration),
                            ));
                        }
                    }
                },
                onkeydown: move |event| {
                    if let Some(value) = keyboard_source_time(
                        event.key(),
                        presented_position,
                        0.0,
                        seek_end,
                        fine_step_secs,
                        coarse_step_secs,
                    ) {
                        event.prevent_default();
                        draft_seek.set(None);
                        controller.seek(std::time::Duration::from_secs_f64(value));
                    }
                },
                onpointercancel: move |_| draft_seek.set(None),
            }
            input {
                class: "dioxus-audio__interactive-input dioxus-audio__interactive-input--selection dioxus-audio__interactive-input--start",
                style: "--_dxa-position: {start_percent}%; --_dxa-hit-offset: {start_hit_offset}",
                r#type: "range",
                min: "0",
                max: "{waveform_duration}",
                step: "{fine_step_secs}",
                value: "{presented_selection.start()}",
                aria_label: selection_start_label,
                aria_valuemin: 0.0,
                aria_valuemax: presented_selection.end(),
                aria_valuetext: start_value_text,
                oninput: move |event| {
                    if let Some(value) = source_time(&event.value(), waveform_duration) {
                        let current = *draft_selection.peek();
                        let current = current.unwrap_or(controlled_selection);
                        draft_selection.set(Some(current.with_start(value)));
                    }
                },
                onchange: move |event| {
                    if let Some(value) = source_time(&event.value(), waveform_duration) {
                        let current = *draft_selection.peek();
                        let next = current
                            .unwrap_or(controlled_selection)
                            .with_start(value)
                            .clamped_to_duration(waveform_duration);
                        draft_selection.set(None);
                        commit_selection_change(
                            controller,
                            controlled_selection,
                            next,
                            on_selection_change,
                        );
                    }
                },
                onkeydown: move |event| {
                    if let Some(value) = keyboard_source_time(
                        event.key(),
                        presented_selection.start(),
                        0.0,
                        presented_selection.end(),
                        fine_step_secs,
                        coarse_step_secs,
                    ) {
                        event.prevent_default();
                        draft_selection.set(None);
                        commit_selection_change(
                            controller,
                            controlled_selection,
                            controlled_selection.with_start(value),
                            on_selection_change,
                        );
                    }
                },
                onpointercancel: move |_| draft_selection.set(None),
            }
            input {
                class: "dioxus-audio__interactive-input dioxus-audio__interactive-input--selection dioxus-audio__interactive-input--end",
                style: "--_dxa-position: {end_percent}%; --_dxa-hit-offset: {end_hit_offset}",
                r#type: "range",
                min: "0",
                max: "{waveform_duration}",
                step: "{fine_step_secs}",
                value: "{presented_selection.end()}",
                aria_label: selection_end_label,
                aria_valuemin: presented_selection.start(),
                aria_valuemax: waveform_duration,
                aria_valuetext: end_value_text,
                oninput: move |event| {
                    if let Some(value) = source_time(&event.value(), waveform_duration) {
                        let current = *draft_selection.peek();
                        let current = current.unwrap_or(controlled_selection);
                        draft_selection.set(Some(current.with_end(value).clamped_to_duration(waveform_duration)));
                    }
                },
                onchange: move |event| {
                    if let Some(value) = source_time(&event.value(), waveform_duration) {
                        let current = *draft_selection.peek();
                        let next = current
                            .unwrap_or(controlled_selection)
                            .with_end(value)
                            .clamped_to_duration(waveform_duration);
                        draft_selection.set(None);
                        commit_selection_change(
                            controller,
                            controlled_selection,
                            next,
                            on_selection_change,
                        );
                    }
                },
                onkeydown: move |event| {
                    if let Some(value) = keyboard_source_time(
                        event.key(),
                        presented_selection.end(),
                        presented_selection.start(),
                        waveform_duration,
                        fine_step_secs,
                        coarse_step_secs,
                    ) {
                        event.prevent_default();
                        draft_selection.set(None);
                        commit_selection_change(
                            controller,
                            controlled_selection,
                            controlled_selection.with_end(value),
                            on_selection_change,
                        );
                    }
                },
                onpointercancel: move |_| draft_selection.set(None),
            }
        }
    }
}

fn commit_selection_change(
    controller: AudioPlayerController,
    current: WaveformSelection,
    next: WaveformSelection,
    on_selection_change: EventHandler<WaveformSelection>,
) {
    if next != current {
        controller.retarget_bounded_after_selection_commit(next);
    }
    on_selection_change.call(next);
}

fn positive_step(value: f64, fallback: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback
    }
}

fn source_time(value: &str, duration: f64) -> Option<f64> {
    value
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(0.0, duration))
}

fn keyboard_source_time(
    key: Key,
    current: f64,
    start: f64,
    end: f64,
    fine_step: f64,
    coarse_step: f64,
) -> Option<f64> {
    let value = match key {
        Key::ArrowDown | Key::ArrowLeft => current - fine_step,
        Key::ArrowUp | Key::ArrowRight => current + fine_step,
        Key::Home => start,
        Key::End => end,
        Key::PageDown => current - coarse_step,
        Key::PageUp => current + coarse_step,
        _ => return None,
    };
    Some(value.clamp(start, end))
}

#[component]
pub fn WaveformPreview(
    peaks: Vec<u8>,
    #[props(default = 32)] bars: usize,
    #[props(default = 32.0)] height: f64,
    #[props(default)] label: Option<String>,
) -> Element {
    let bars = bars.max(1);
    let reduced = downsample_peaks(&peaks, bars);
    let values = if reduced.is_empty() || reduced.len() == bars {
        reduced
    } else {
        (0..bars)
            .map(|index| reduced[index * reduced.len() / bars])
            .collect()
    };
    let amplitude_scale = values.iter().copied().max().unwrap_or(255).max(32) as f64;
    let bar_width = 2.0;
    let gap = 1.0;
    let height = if height.is_finite() {
        height.max(1.0)
    } else {
        32.0
    };
    let width = (values.len().max(1) as f64 * (bar_width + gap)).max(1.0);
    let role = label.as_ref().map(|_| "img");

    rsx! {
        svg {
            class: "dioxus-audio dioxus-audio__waveform",
            role,
            "aria-label": label,
            "aria-hidden": role.is_none(),
            width: "100%",
            height: "{height}",
            view_box: "0 0 {width} {height}",
            preserve_aspect_ratio: "none",
            if values.is_empty() {
                line {
                    x1: "0",
                    x2: "{width}",
                    y1: "{height / 2.0}",
                    y2: "{height / 2.0}",
                    class: "dioxus-audio__waveform-idle",
                }
            } else {
                for (index, value) in values.iter().enumerate() {
                    {
                        let bar_height = ((*value as f64 / amplitude_scale) * height).max(1.0);
                        let x = index as f64 * (bar_width + gap);
                        let y = (height - bar_height) / 2.0;
                        rsx! {
                            rect {
                                x: "{x}",
                                y: "{y}",
                                width: "{bar_width}",
                                height: "{bar_height}",
                                rx: "1",
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn WaveformRangeSelector(
    peaks: Vec<u8>,
    duration_secs: f64,
    selection: WaveformSelection,
    on_change: EventHandler<WaveformSelection>,
    #[props(default)] label: Option<String>,
) -> Element {
    let duration_secs = if duration_secs.is_finite() && duration_secs > 0.0 {
        duration_secs
    } else {
        0.0
    };
    let selection = selection.clamped_to_duration(duration_secs);
    let start_percent = if duration_secs > 0.0 {
        selection.start() / duration_secs * 100.0
    } else {
        0.0
    };
    let end_percent = if duration_secs > 0.0 {
        selection.end() / duration_secs * 100.0
    } else {
        0.0
    };
    let selection_width = end_percent - start_percent;
    let collapsed = selection.is_collapsed();
    let label = label.unwrap_or_else(|| "Select audio range".to_string());
    let start_value_text = format_accessible_duration(selection.start());
    let end_value_text = format_accessible_duration(selection.end());
    let disabled = duration_secs == 0.0;

    rsx! {
        div {
            class: "dioxus-audio dioxus-audio__range",
            role: "group",
            aria_label: label,
            "data-collapsed": collapsed,
            WaveformPreview { peaks, label: None }
            div {
                class: "dioxus-audio__range-selection",
                style: "left: {start_percent}%; width: {selection_width}%",
            }
            input {
                class: "dioxus-audio__range-input dioxus-audio__range-input--start",
                r#type: "range",
                min: "0",
                max: "{duration_secs}",
                step: "any",
                value: "{selection.start()}",
                disabled,
                aria_label: "Selection start",
                aria_valuetext: start_value_text,
                oninput: move |event| {
                    if let Ok(value) = event.value().parse::<f64>() {
                        on_change.call(selection.with_start(value).clamped_to_duration(duration_secs));
                    }
                },
            }
            input {
                class: "dioxus-audio__range-input dioxus-audio__range-input--end",
                r#type: "range",
                min: "0",
                max: "{duration_secs}",
                step: "any",
                value: "{selection.end()}",
                disabled,
                aria_label: "Selection end",
                aria_valuetext: end_value_text,
                oninput: move |event| {
                    if let Ok(value) = event.value().parse::<f64>() {
                        on_change.call(selection.with_end(value).clamped_to_duration(duration_secs));
                    }
                },
            }
        }
    }
}
