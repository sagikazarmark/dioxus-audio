use dioxus::prelude::*;

use crate::analysis::{WaveformSelection, downsample_peaks};

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
    selection: WaveformSelection,
    on_change: EventHandler<WaveformSelection>,
    #[props(default)] label: Option<String>,
) -> Element {
    let start_percent = selection.start() * 100.0;
    let end_percent = selection.end() * 100.0;
    let selection_width = end_percent - start_percent;
    let collapsed = (selection_width).abs() < f64::EPSILON;
    let label = label.unwrap_or_else(|| "Select audio range".to_string());

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
                max: "100",
                step: "0.1",
                value: "{start_percent}",
                aria_label: "Selection start",
                oninput: move |event| {
                    if let Ok(value) = event.value().parse::<f64>() {
                        on_change.call(selection.with_start(value / 100.0));
                    }
                },
            }
            input {
                class: "dioxus-audio__range-input dioxus-audio__range-input--end",
                r#type: "range",
                min: "0",
                max: "100",
                step: "0.1",
                value: "{end_percent}",
                aria_label: "Selection end",
                oninput: move |event| {
                    if let Ok(value) = event.value().parse::<f64>() {
                        on_change.call(selection.with_end(value / 100.0));
                    }
                },
            }
        }
    }
}
