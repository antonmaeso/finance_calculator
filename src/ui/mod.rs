//! Shared UI helpers: number formatting, colours and input widgets.

pub mod inputs;
pub mod tabs;

use eframe::egui::{self, Color32, RichText, SliderClamping, Ui};
use egui_plot::{GridMark, Legend, Plot};

// Categorical slots (dark-surface steps). Colour follows the entity:
// Sell is always slot 1, Keep is always slot 2.
pub const SELL: Color32 = Color32::from_rgb(0x39, 0x87, 0xe5);
pub const KEEP: Color32 = Color32::from_rgb(0xd9, 0x59, 0x26);
pub const SLOT3: Color32 = Color32::from_rgb(0x19, 0x9e, 0x70);
pub const SLOT4: Color32 = Color32::from_rgb(0xc9, 0x85, 0x00);
pub const SLOT5: Color32 = Color32::from_rgb(0xd5, 0x51, 0x81);
pub const SLOT7: Color32 = Color32::from_rgb(0x90, 0x85, 0xe9);

// Status colours – always paired with a text label.
pub const GOOD: Color32 = Color32::from_rgb(0x0c, 0xa3, 0x0c);
pub const CRITICAL: Color32 = Color32::from_rgb(0xd0, 0x3b, 0x3b);
pub const WARNING: Color32 = Color32::from_rgb(0xfa, 0xb2, 0x19);
pub const NEUTRAL_MID: Color32 = Color32::from_rgb(0x38, 0x38, 0x35);
pub const MUTED: Color32 = Color32::from_rgb(0x9a, 0x99, 0x92);

/// £1,234,567
pub fn money(v: f64) -> String {
    if !v.is_finite() {
        return "–".into();
    }
    let neg = v < 0.0;
    let s = format!("{:.0}", v.abs());
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    format!("{}£{}", if neg { "−" } else { "" }, out)
}

/// £1.2m / £340k / £950
pub fn money_short(v: f64) -> String {
    let a = v.abs();
    let sign = if v < 0.0 { "−" } else { "" };
    if a >= 1_000_000.0 {
        format!("{sign}£{:.1}m", a / 1_000_000.0)
    } else if a >= 10_000.0 {
        format!("{sign}£{:.0}k", a / 1_000.0)
    } else if a >= 1_000.0 {
        format!("{sign}£{:.1}k", a / 1_000.0)
    } else {
        format!("{sign}£{:.0}", a)
    }
}

pub fn signed_money(v: f64) -> String {
    if v >= 0.0 {
        format!("+{}", money(v))
    } else {
        money(v)
    }
}

/// A plot with money on the y axis, years on the x axis and a hover readout.
pub fn money_plot<'a>(id: &'a str, x_label: &'a str) -> Plot<'a> {
    money_plot_bare(id, x_label).legend(Legend::default().position(egui_plot::Corner::LeftTop))
}

/// Same as [`money_plot`] without a legend (for charts whose axis names the marks).
pub fn money_plot_bare<'a>(id: &'a str, x_label: &'a str) -> Plot<'a> {
    let x_label_owned = x_label.to_string();
    Plot::new(id)
        .y_axis_formatter(|m: GridMark, _| money_short(m.value))
        .x_axis_label(x_label)
        .label_formatter(move |hp: &egui_plot::HoverPosition<'_>| {
            let (name, p) = match hp {
                egui_plot::HoverPosition::NearDataPoint { plot_name, position, .. } => (*plot_name, *position),
                egui_plot::HoverPosition::Elsewhere { position } => ("", *position),
            };
            let head = if name.is_empty() { String::new() } else { format!("{name}\n") };
            Some(format!("{head}{} {:.1}\n{}", x_label_owned, p.x, money(p.y)))
        })
        .allow_scroll(false)
        .height(300.0)
}

pub fn section_title(ui: &mut Ui, text: &str) {
    ui.add_space(6.0);
    ui.label(RichText::new(text).strong().size(16.0));
}

pub fn help(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).small().color(MUTED));
}

/// Money slider that also lets you type a value outside the range.
pub fn money_input(ui: &mut Ui, label: &str, v: &mut f64, max: f64, tip: &str) -> bool {
    let step = if max > 200_000.0 {
        1_000.0
    } else if max > 20_000.0 {
        500.0
    } else if max > 2_000.0 {
        50.0
    } else {
        10.0
    };
    let r = ui
        .add(
            egui::Slider::new(v, 0.0..=max)
                .text(label)
                .prefix("£")
                .step_by(step)
                .clamping(SliderClamping::Never)
                .custom_formatter(|n, _| money(n).trim_start_matches('£').to_string()),
        )
        .on_hover_text(tip);
    r.changed()
}

pub fn pct_input(ui: &mut Ui, label: &str, v: &mut f64, lo: f64, hi: f64, tip: &str) -> bool {
    ui.add(
        egui::Slider::new(v, lo..=hi)
            .text(label)
            .suffix("%")
            .step_by(0.05)
            .fixed_decimals(2)
            .clamping(SliderClamping::Never),
    )
    .on_hover_text(tip)
    .changed()
}

pub fn years_input(ui: &mut Ui, label: &str, v: &mut f64, hi: f64, tip: &str) -> bool {
    ui.add(egui::Slider::new(v, 0.0..=hi).text(label).suffix(" yrs").step_by(0.25).fixed_decimals(2))
        .on_hover_text(tip)
        .changed()
}

/// A KPI tile waiting to be laid out by [`kpi_row`].
pub struct Tile {
    title: String,
    value: String,
    sub: String,
    accent: Option<Color32>,
}

pub fn kpi(t: &mut Vec<Tile>, title: &str, value: &str, sub: &str, accent: Option<Color32>) {
    t.push(Tile { title: title.into(), value: value.into(), sub: sub.into(), accent });
}

/// Lay KPI tiles out in as many equal columns as fit the width.
pub fn kpi_row(ui: &mut Ui, build: impl FnOnce(&mut Vec<Tile>)) {
    let mut tiles = Vec::new();
    build(&mut tiles);
    let per_row = ((ui.available_width() / 185.0).floor() as usize).clamp(1, tiles.len().max(1));
    for chunk in tiles.chunks(per_row) {
        ui.columns(per_row, |cols| {
            for (col, t) in cols.iter_mut().zip(chunk) {
                egui::Frame::group(col.style()).inner_margin(8.0).show(col, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new(&t.title).small().color(MUTED));
                    ui.horizontal(|ui| {
                        if let Some(c) = t.accent {
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(4.0, 22.0), egui::Sense::hover());
                            ui.painter().rect_filled(rect, 2.0, c);
                        }
                        ui.label(RichText::new(&t.value).size(20.0).strong());
                    });
                    ui.label(RichText::new(if t.sub.is_empty() { " " } else { &t.sub }).small());
                });
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_formats() {
        assert_eq!(money(1234567.4), "£1,234,567");
        assert_eq!(money(-950.0), "−£950");
        assert_eq!(money_short(340_000.0), "£340k");
        assert_eq!(money_short(1_250_000.0), "£1.2m");
    }
}
