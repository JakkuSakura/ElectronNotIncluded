//! Minimal egui theme tokens: background/panel/border/text/accent/danger/positive.

use super::*;

pub(crate) const UI_BACKGROUND: egui::Color32 = egui::Color32::from_rgb(18, 20, 24);
pub(crate) const UI_PANEL: egui::Color32 = egui::Color32::from_rgba_premultiplied(24, 27, 32, 232);
pub(crate) const UI_BORDER: egui::Color32 = egui::Color32::from_rgb(70, 78, 90);
pub(crate) const UI_TEXT: egui::Color32 = egui::Color32::from_rgb(220, 225, 230);
pub(crate) const UI_MUTED: egui::Color32 = egui::Color32::from_rgb(140, 148, 158);
pub(crate) const UI_ACCENT: egui::Color32 = egui::Color32::from_rgb(70, 150, 220);
pub(crate) const UI_DANGER: egui::Color32 = egui::Color32::from_rgb(200, 80, 70);
/// Reserved for future positive/success UI states (e.g. a completed reaction).
#[allow(dead_code)]
pub(crate) const UI_POSITIVE: egui::Color32 = egui::Color32::from_rgb(80, 170, 120);

pub(super) fn apply_ui_style(context: &egui::Context) {
    let mut style = (*context.style_of(egui::Theme::Dark)).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(12.0, 7.0);
    style.spacing.window_margin = egui::Margin::same(12);
    style.visuals.dark_mode = true;
    style.visuals.override_text_color = Some(UI_TEXT);
    style.visuals.weak_text_color = Some(UI_MUTED);
    style.visuals.extreme_bg_color = UI_BACKGROUND;
    style.visuals.panel_fill = UI_PANEL;
    style.visuals.window_fill = UI_PANEL;
    style.visuals.window_stroke = egui::Stroke::new(1.0, UI_BORDER);
    style.visuals.button_frame = true;
    style.visuals.widgets.inactive.bg_fill = UI_BACKGROUND;
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, UI_BORDER);
    style.visuals.widgets.inactive.fg_stroke.color = UI_TEXT;
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, UI_ACCENT);
    style.visuals.widgets.active.bg_fill = UI_ACCENT;
    style.visuals.selection.bg_fill = UI_ACCENT;
    style.visuals.warn_fg_color = UI_ACCENT;
    style.visuals.error_fg_color = UI_DANGER;
    context.set_style_of(egui::Theme::Dark, style);
}

pub(super) fn ui_panel() -> egui::Frame {
    egui::Frame::new()
        .fill(UI_PANEL)
        .stroke(egui::Stroke::new(1.0, UI_BORDER))
        .inner_margin(egui::Margin::same(10))
}

pub(super) fn ui_section_header(ui: &mut egui::Ui, title: &str) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(title).color(UI_TEXT).size(16.0));
        ui.separator();
    });
}
