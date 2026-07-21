use std::fmt::Write as _;
use std::ops::Range;
use std::time::Duration;

use dioxus::prelude::*;

use crate::analysis::{WaveformSelection, downsample_peaks};
use crate::playback::{AudioPlayerController, PlaybackSourceLifecycle};
use crate::waveform::{
    AmplitudeMode, AmplitudeSlice, SignedEnvelope, WaveformData, WaveformViewportController,
};

use super::format_accessible_duration;

const MAXIMUM_NAVIGABLE_BUCKET_BUDGET: usize = 4096;
const MAXIMUM_PIXEL_DENSITY: f64 = 2.0;

/// Render immutable Waveform Data as one responsive SVG path per channel.
#[component]
pub fn Waveform(
    data: WaveformData,
    #[props(default = 512)] bucket_budget: usize,
    #[props(default = 96.0)] height: f64,
    #[props(default)] label: Option<String>,
) -> Element {
    let height = waveform_height(height);
    let visible = Duration::ZERO..data.duration();

    rsx! {
        WaveformSvg {
            data,
            visible,
            bucket_budget: bucket_budget.max(1),
            height,
            label,
        }
    }
}

#[derive(Clone, PartialEq)]
struct WaveformGeometry {
    paths: Vec<String>,
    view_start: f64,
    view_span: f64,
    amplitude_mode: &'static str,
    channel_count: usize,
    resolution: usize,
    bucket_count: usize,
}

#[component]
fn WaveformSvg(
    data: WaveformData,
    visible: Range<Duration>,
    bucket_budget: usize,
    height: f64,
    label: Option<String>,
) -> Element {
    let geometry = use_memo(use_reactive!(|(data, visible, bucket_budget, height)| {
        build_waveform_geometry(&data, visible, bucket_budget, height)
    }));
    let geometry = geometry();
    let role = label.as_ref().map(|_| "img");

    rsx! {
        svg {
            class: "dioxus-audio dioxus-audio__waveform dioxus-audio__waveform-data",
            role,
            "aria-label": label,
            "aria-hidden": role.is_none(),
            "data-amplitude-mode": geometry.amplitude_mode,
            "data-channel-count": geometry.channel_count,
            "data-resolution": geometry.resolution,
            "data-bucket-count": geometry.bucket_count,
            "data-bucket-budget": bucket_budget,
            width: "100%",
            height: "{height}",
            view_box: "{geometry.view_start} 0 {geometry.view_span} {height}",
            preserve_aspect_ratio: "none",
            for path_data in geometry.paths.iter() {
                path {
                    class: "dioxus-audio__waveform-channel",
                    d: path_data,
                }
            }
        }
    }
}

fn build_waveform_geometry(
    data: &WaveformData,
    visible: Range<Duration>,
    bucket_budget: usize,
    height: f64,
) -> WaveformGeometry {
    let view = data
        .select(visible.clone(), bucket_budget.max(1))
        .expect("Waveform Data and presentation budget are valid");
    let channel_height = height / data.channel_count() as f64;
    let bucket_span =
        view.bucket_span().numerator().as_secs_f64() / view.bucket_span().divisor() as f64;
    let axis = BucketAxis {
        first_bucket: view.first_bucket(),
        bucket_span,
        source_end: data.duration().as_secs_f64(),
    };
    let paths = (0..data.channel_count())
        .filter_map(|channel| {
            let top = channel as f64 * channel_height;
            match view.channel(channel)? {
                AmplitudeSlice::Magnitudes(values) => {
                    Some(magnitude_path(values, top, channel_height, axis))
                }
                AmplitudeSlice::SignedEnvelopes(values) => {
                    Some(signed_envelope_path(values, top, channel_height, axis))
                }
            }
        })
        .collect::<Vec<_>>();
    let amplitude_mode = match data.mode() {
        AmplitudeMode::Magnitude => "magnitude",
        AmplitudeMode::SignedEnvelope => "signed-envelope",
    };
    WaveformGeometry {
        paths,
        view_start: visible.start.as_secs_f64(),
        view_span: (visible.end - visible.start).as_secs_f64(),
        amplitude_mode,
        channel_count: data.channel_count(),
        resolution: view.resolution_index(),
        bucket_count: view.bucket_count(),
    }
}

#[derive(Clone, Copy)]
struct BucketAxis {
    first_bucket: usize,
    bucket_span: f64,
    source_end: f64,
}

impl BucketAxis {
    fn start(self, local_bucket: usize) -> f64 {
        (self.first_bucket + local_bucket) as f64 * self.bucket_span
    }

    fn end(self, local_bucket: usize) -> f64 {
        (self.start(local_bucket) + self.bucket_span).min(self.source_end)
    }
}

fn magnitude_path(values: &[f32], top: f64, height: f64, axis: BucketAxis) -> String {
    let baseline = top + height;
    let first_x = axis.start(0);
    let mut path = format!("M{first_x} {baseline}");
    for (index, value) in values.iter().enumerate() {
        let x = axis.start(index);
        let next_x = axis.end(index);
        let y = baseline - f64::from(*value) * height;
        let _ = write!(path, "L{x} {y}H{next_x}");
    }
    let final_x = axis.end(values.len() - 1);
    let _ = write!(path, "L{final_x} {baseline}Z");
    path
}

fn signed_envelope_path(
    values: &[SignedEnvelope],
    top: f64,
    height: f64,
    axis: BucketAxis,
) -> String {
    let center = top + height / 2.0;
    let amplitude_height = height / 2.0;
    let upper = |value: SignedEnvelope| center - f64::from(value.max) * amplitude_height;
    let lower = |value: SignedEnvelope| center - f64::from(value.min) * amplitude_height;
    let first_x = axis.start(0);

    let mut path = format!("M{first_x} {}", upper(values[0]));
    for (index, value) in values.iter().copied().enumerate() {
        let next_x = axis.end(index);
        if index > 0 {
            let _ = write!(path, "V{}", upper(value));
        }
        let _ = write!(path, "H{next_x}");
    }

    let last = values[values.len() - 1];
    let final_x = axis.end(values.len() - 1);
    let _ = write!(path, "L{final_x} {}", lower(last));
    for index in (0..values.len()).rev() {
        if index + 1 < values.len() {
            let _ = write!(path, "V{}", lower(values[index]));
        }
        let x = axis.start(index);
        let _ = write!(path, "H{x}");
    }
    path.push('Z');
    path
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct WaveformMeasurement {
    bucket_budget: usize,
    measured: bool,
}

/// Present and navigate one measured, source-time Waveform Viewport.
///
/// The server and first client render use `fallback_bucket_budget`. Once
/// mounted, the component derives a per-channel budget from its observed CSS
/// width and browser pixel density, capped at 2x density and 4,096 buckets.
#[component]
pub fn NavigableWaveform(
    data: WaveformData,
    controller: WaveformViewportController,
    #[props(default = 512)] fallback_bucket_budget: usize,
    #[props(default = 96.0)] height: f64,
    #[props(default = true)] show_overview: bool,
    #[props(default = "Waveform viewport".to_string())] label: String,
) -> Element {
    assert_eq!(
        data.duration(),
        controller.total_duration(),
        "Waveform Data and Viewport Controller durations must match"
    );
    let height = waveform_height(height);
    let fallback_bucket_budget = fallback_bucket_budget.clamp(1, MAXIMUM_NAVIGABLE_BUCKET_BUDGET);
    let mut measurement = use_signal(move || WaveformMeasurement {
        bucket_budget: fallback_bucket_budget,
        measured: false,
    });
    let current_measurement = measurement();
    let visible = controller.visible_range();
    let span = visible.end - visible.start;
    let center = visible.start + span / 2;
    let total_duration = controller.total_duration();
    let visible_text = format_visible_range(&visible);
    let at_start = visible.start.is_zero();
    let at_end = visible.end == total_duration;
    let fully_zoomed_out = span == total_duration;
    let fully_zoomed_in = span <= Duration::from_nanos(1);
    let overview_minimum = span.as_secs_f64() / 2.0;
    let overview_maximum = total_duration.as_secs_f64() - overview_minimum;
    let overview_step = (span.as_secs_f64() / 100.0).max(0.001);
    let budget_source = if current_measurement.measured {
        "measured"
    } else {
        "fallback"
    };
    let render_data = data.clone();

    rsx! {
        div {
            class: "dioxus-audio dioxus-audio__waveform-viewport",
            role: "group",
            aria_label: label,
            "data-budget-source": budget_source,
            onresize: move |event| {
                let Ok(size) = event.data().get_content_box_size() else {
                    return;
                };
                let Some(bucket_budget) = measured_bucket_budget(size.width) else {
                    return;
                };
                let next = WaveformMeasurement {
                    bucket_budget,
                    measured: true,
                };
                if *measurement.peek() != next {
                    measurement.set(next);
                }
            },
            WaveformSvg {
                data: render_data,
                visible: visible.clone(),
                bucket_budget: current_measurement.bucket_budget,
                height,
                label: None,
            }
            output {
                class: "dioxus-audio__viewport-range",
                "{visible_text}"
            }
            nav {
                class: "dioxus-audio__viewport-controls",
                aria_label: "Waveform viewport controls",
                button {
                    class: "dioxus-audio__viewport-control",
                    r#type: "button",
                    aria_label: "Pan backward",
                    disabled: at_start,
                    onclick: move |_| controller.pan(-0.5),
                    "Pan back"
                }
                button {
                    class: "dioxus-audio__viewport-control",
                    r#type: "button",
                    aria_label: "Zoom out",
                    disabled: fully_zoomed_out,
                    onclick: move |_| controller.zoom(0.5, center),
                    "Zoom out"
                }
                button {
                    class: "dioxus-audio__viewport-control",
                    r#type: "button",
                    aria_label: "Reset view",
                    disabled: fully_zoomed_out,
                    onclick: move |_| controller.reset(),
                    "Reset"
                }
                button {
                    class: "dioxus-audio__viewport-control",
                    r#type: "button",
                    aria_label: "Zoom in",
                    disabled: fully_zoomed_in,
                    onclick: move |_| controller.zoom(2.0, center),
                    "Zoom in"
                }
                button {
                    class: "dioxus-audio__viewport-control",
                    r#type: "button",
                    aria_label: "Pan forward",
                    disabled: at_end,
                    onclick: move |_| controller.pan(0.5),
                    "Pan forward"
                }
            }
            if show_overview {
                label { class: "dioxus-audio__viewport-overview",
                    span { "Overview" }
                    input {
                        r#type: "range",
                        min: overview_minimum,
                        max: overview_maximum.max(overview_minimum),
                        step: overview_step,
                        value: center.as_secs_f64(),
                        disabled: fully_zoomed_out,
                        aria_label: "Overview position",
                        aria_valuetext: visible_text,
                        oninput: move |event| {
                            let Ok(requested_center) = event.value().parse::<f64>() else {
                                return;
                            };
                            if !requested_center.is_finite() {
                                return;
                            }
                            let span_seconds = span.as_secs_f64();
                            let maximum_start = total_duration.as_secs_f64() - span_seconds;
                            let start = (requested_center - span_seconds / 2.0)
                                .clamp(0.0, maximum_start);
                            controller.show_range(
                                Duration::from_secs_f64(start)
                                    ..Duration::from_secs_f64(start + span_seconds),
                            );
                        },
                    }
                }
            }
        }
    }
}

fn waveform_height(height: f64) -> f64 {
    if height.is_finite() {
        height.max(1.0)
    } else {
        96.0
    }
}

fn measured_bucket_budget(width: f64) -> Option<usize> {
    if !width.is_finite() || width <= 0.0 {
        return None;
    }
    let density = browser_pixel_density().clamp(1.0, MAXIMUM_PIXEL_DENSITY);
    Some(((width * density).round() as usize).clamp(1, MAXIMUM_NAVIGABLE_BUCKET_BUDGET))
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
fn browser_pixel_density() -> f64 {
    web_sys::window()
        .map(|window| window.device_pixel_ratio())
        .filter(|density| density.is_finite() && *density > 0.0)
        .unwrap_or(1.0)
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
fn browser_pixel_density() -> f64 {
    1.0
}

fn format_visible_range(visible: &Range<Duration>) -> String {
    let span = visible.end - visible.start;
    let fractional_digits = if span >= Duration::from_millis(10) {
        2
    } else if span >= Duration::from_millis(1) {
        3
    } else if span >= Duration::from_micros(1) {
        6
    } else {
        9
    };
    format!(
        "Visible {} to {}",
        format_source_time(visible.start, fractional_digits),
        format_source_time(visible.end, fractional_digits)
    )
}

fn format_source_time(duration: Duration, fractional_digits: usize) -> String {
    let total_seconds = duration.as_secs();
    let hours = total_seconds / 3_600;
    let minutes = total_seconds / 60 % 60;
    let seconds = total_seconds % 60;
    let divisor = 10_u32.pow(9 - fractional_digits as u32);
    let fraction_value = duration.subsec_nanos() / divisor;
    let fraction = if fraction_value == 0 {
        String::new()
    } else {
        let fraction = format!("{fraction_value:0fractional_digits$}");
        format!(".{}", fraction.trim_end_matches('0'))
    };

    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}{fraction}")
    } else {
        format!("{minutes}:{seconds:02}{fraction}")
    }
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
