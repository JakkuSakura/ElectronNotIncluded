//! Minimal egui HUD: main menu, pause menu, and a status readout.

mod theme;

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use eni_domain::{GameClock, GamePaused, GameState};

use crate::resources::EguiFontsConfigured;
use theme::*;

pub(crate) fn configure_egui_fonts(
    mut contexts: EguiContexts,
    mut configured: ResMut<EguiFontsConfigured>,
) {
    if configured.0 {
        return;
    }
    let Ok(context) = contexts.ctx_mut() else {
        return;
    };
    apply_ui_style(context);
    configured.0 = true;
}

pub(crate) fn not_paused(paused: Res<GamePaused>) -> bool {
    !paused.0
}

pub(crate) fn toggle_pause(keys: Res<ButtonInput<KeyCode>>, mut paused: ResMut<GamePaused>) {
    if keys.just_pressed(KeyCode::Escape) {
        paused.0 = !paused.0;
    }
}

pub(crate) fn show_main_menu(
    mut contexts: EguiContexts,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Ok(context) = contexts.ctx_mut() else {
        return;
    };
    egui::Window::new("ElectronNotIncluded")
        .id(egui::Id::new("main_menu"))
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .frame(ui_panel())
        .show(context, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("ElectronNotIncluded")
                        .color(UI_TEXT)
                        .size(32.0),
                );
                ui.label(egui::RichText::new("a tile chemistry sandbox").color(UI_MUTED));
                ui.add_space(16.0);
                if ui.button("Start").clicked() {
                    next_state.set(GameState::Playing);
                }
            });
        });
}

pub(crate) fn show_pause_menu(
    mut contexts: EguiContexts,
    paused: Res<GamePaused>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if !paused.0 {
        return;
    }
    let Ok(context) = contexts.ctx_mut() else {
        return;
    };
    egui::Window::new("Paused")
        .collapsible(false)
        .resizable(false)
        .frame(ui_panel())
        .show(context, |ui| {
            if ui.button("Resume").clicked() {
                // handled by toggle_pause on next Escape press; nothing else to do here
            }
            if ui.button("Quit to menu").clicked() {
                next_state.set(GameState::Menu);
            }
        });
}

pub(crate) fn show_game_hud(mut contexts: EguiContexts, clock: Res<GameClock>) {
    let Ok(context) = contexts.ctx_mut() else {
        return;
    };
    let date_time = clock.date_time();
    egui::Window::new("hud")
        .id(egui::Id::new("hud"))
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .anchor(egui::Align2::LEFT_TOP, egui::vec2(8.0, 8.0))
        .frame(ui_panel())
        .show(context, |ui| {
            ui_section_header(ui, "ElectronNotIncluded");
            ui.label(format!(
                "Year {} / Day {} — {:02}:{:02}:{:02}",
                date_time.year, date_time.day, date_time.hour, date_time.minute, date_time.second
            ));
        });
}
