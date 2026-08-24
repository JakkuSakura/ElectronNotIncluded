//! Minimal 2D presentation layer for ElectronNotIncluded.
//!
//! Deliberately gutted compared to the wuxia project this was forked from:
//! no inventory/combat/NPC UI, no 3D over-the-shoulder camera. Tiles render
//! as flat colored squares by dominant substance; polish is out of scope.

use bevy::prelude::*;
use bevy_egui::{EguiGlobalSettings, EguiPlugin, EguiPrimaryContextPass};

mod rendering;
mod resources;
mod ui;
mod world_stream;

use resources::*;

pub use eni_domain::GameState;
pub use resources::{
    ClientControlAction, ClientControlQueue, ClientControlRequest, DebugMode,
    FramebufferCaptureQueue, FramebufferCaptureRequest, FramebufferTarget, HeadlessMode,
    HeadlessStartState,
};

pub struct ClientPlugin;

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin::default())
            .init_state::<GameState>()
            .init_resource::<EguiFontsConfigured>()
            .init_resource::<world_stream::ClientWorld>()
            .add_systems(Startup, disable_auto_egui_context)
            .add_systems(Startup, rendering::setup_camera)
            .add_systems(Startup, start_headless_gameplay)
            .add_systems(
                EguiPrimaryContextPass,
                (
                    ui::configure_egui_fonts,
                    ui::show_main_menu.run_if(in_state(GameState::Menu)),
                    ui::show_pause_menu.run_if(in_state(GameState::Playing)),
                    ui::show_game_hud.run_if(in_state(GameState::Playing)),
                )
                    .chain(),
            )
            .add_systems(OnEnter(GameState::Playing), rendering::setup_playing)
            .add_systems(OnExit(GameState::Playing), rendering::cleanup_playing)
            .add_systems(
                Update,
                ui::toggle_pause.run_if(in_state(GameState::Playing)),
            )
            .add_systems(Update, process_client_control_requests)
            .add_systems(
                Update,
                world_stream::read_input
                    .run_if(in_state(GameState::Playing))
                    .run_if(ui::not_paused),
            )
            .add_systems(
                Update,
                world_stream::apply_player_position.run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                world_stream::receive_chunks.run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                world_stream::receive_unloads.run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                rendering::render_tiles.run_if(in_state(GameState::Playing)),
            );
    }
}

fn disable_auto_egui_context(mut settings: ResMut<EguiGlobalSettings>) {
    settings.auto_create_primary_context = false;
}

fn start_headless_gameplay(
    headless: Option<Res<HeadlessMode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if headless.is_some_and(|mode| mode.start_state == HeadlessStartState::Playing) {
        tracing::info!("headless runtime starts directly in gameplay state");
        next_state.set(GameState::Playing);
    }
}

fn process_client_control_requests(
    queue: Option<Res<ClientControlQueue>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Some(queue) = queue else {
        return;
    };
    let Ok(receiver) = queue.0.lock() else {
        tracing::warn!("client control queue is poisoned");
        return;
    };
    while let Ok(request) = receiver.try_recv() {
        let result = match request.action {
            ClientControlAction::State(state) => {
                let next = match state {
                    HeadlessStartState::Menu => GameState::Menu,
                    HeadlessStartState::Playing => GameState::Playing,
                };
                next_state.set(next);
                Ok(())
            }
        };
        let _ = request.response.send(result);
    }
}
